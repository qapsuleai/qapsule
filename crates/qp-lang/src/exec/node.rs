//! One node's step on one bar of its clock (PR §4, RX4 §4.3–§4.9), split as `eval` on top
//! of the committed state and `commit` folding the bar in — the `qp-compute` `Core` pattern
//! for every node, so a live bar is a pure evaluation and a closed bar is evaluation plus
//! commit. Each value carries its provenance `lo` (PR §9; RX4 §8): the first base bar it
//! depends on.
//!
//! Values are `Option<f64>`: `bool` is `1.0`/`0.0`, `cat` is the bin index.

use std::collections::VecDeque;

use qp_compute::{Atr, ComputeError};
use qp_schema::program::BarField;
use qp_schema::Bar;

use super::kernel::{Kernel, Outs};
use crate::checked::{Anchor, BinaryKind, CondKind, Literal, Op, UnaryKind};

/// `lo` of a clockless value: no base bar (combined with `min`, published as `min(lo, i)`).
pub(super) const LO_FREE: u64 = u64::MAX;

/// The bar a node steps on.
pub(super) struct Ctx<'a> {
    /// Index of the bar on the node's clock (base bar `i` or higher bar `k`).
    pub(super) i: u64,
    /// That bar (aggregated for a higher timeframe).
    pub(super) bar: &'a Bar,
    /// Its first base bar (provenance of a bar field).
    pub(super) lo: u64,
}

/// A node's committed state.
#[derive(Debug, Clone)]
pub(super) enum State {
    /// Stateless.
    None,
    /// A window kernel; `lows` holds the `lo`s of the last `back` committed defined inputs
    /// (`None`: recursive kernel, `lo` = 0).
    Kernel {
        k: Box<Kernel>,
        back: Option<usize>,
        lows: VecDeque<u64>,
    },
    /// `atr` (recursive).
    Atr(Box<Atr>),
    /// `lag`: the last `n` committed `(value, lo)`.
    Lag(VecDeque<(Option<f64>, u64)>),
    /// `cross_*`: the previous bar's `(a, b, lo)`.
    Cross(Option<(Option<f64>, Option<f64>, u64)>),
    /// `streak`: the current count.
    Streak(u64),
    /// `bars_since`: the last bar where the condition was true.
    BarsSince(Option<u64>),
    /// `once`: fired already.
    Once(bool),
    /// `rearm`: armed.
    Rearm(bool),
    /// `cooldown`: the last firing bar.
    Cooldown(Option<u64>),
    /// `line`: the anchors' bars so far.
    Line(Option<u64>, Option<u64>),
}

impl State {
    /// The empty state of `op`.
    pub(super) fn new(op: &Op) -> Result<State, ComputeError> {
        Ok(match op {
            Op::Window { kind, period, .. } => State::Kernel {
                k: Box::new(Kernel::window(*kind, *period)?),
                back: Kernel::lookback(*kind, *period),
                lows: VecDeque::new(),
            },
            Op::Macd {
                fast, slow, signal, ..
            } => State::Kernel {
                k: Box::new(Kernel::Macd(qp_compute::Macd::new(*fast, *slow, *signal)?)),
                back: None,
                lows: VecDeque::new(),
            },
            Op::Bollinger { period, k, .. } => State::Kernel {
                k: Box::new(Kernel::Bollinger(qp_compute::Bollinger::new(*period, *k)?)),
                back: Some(period - 1),
                lows: VecDeque::new(),
            },
            Op::Atr { period } => State::Atr(Box::new(Atr::new(*period)?)),
            Op::Lag { .. } => State::Lag(VecDeque::new()),
            Op::Binary {
                kind: BinaryKind::CrossAbove | BinaryKind::CrossBelow,
                ..
            } => State::Cross(None),
            Op::Cond { kind, .. } => match kind {
                CondKind::Streak => State::Streak(0),
                CondKind::BarsSince => State::BarsSince(None),
                CondKind::Once => State::Once(false),
            },
            Op::Rearm { .. } => State::Rearm(true),
            Op::Cooldown { .. } => State::Cooldown(None),
            Op::Line { .. } => State::Line(None, None),
            _ => State::None,
        })
    }
}

/// A `bool` value.
#[inline]
fn b(v: bool) -> Option<f64> {
    Some(if v { 1.0 } else { 0.0 })
}

/// Whether a `bool` value is true (`None` is false for state nodes, PR §4.4).
#[inline]
fn truthy(v: Option<f64>) -> bool {
    v.is_some_and(|x| x != 0.0)
}

/// A finite number or nothing (PR §4.3: a non-finite result is undefined).
#[inline]
fn finite(x: f64) -> Option<f64> {
    x.is_finite().then_some(x)
}

/// A bar field; a non-finite price is undefined.
#[inline]
fn field(bar: &Bar, f: BarField) -> Option<f64> {
    finite(match f {
        BarField::Open => bar.o,
        BarField::High => bar.h,
        BarField::Low => bar.l,
        BarField::Close => bar.c,
        BarField::Volume => bar.v,
    })
}

/// The inputs a step reads: values and `lo`s, in the op's field order.
pub(super) struct In<'a> {
    pub(super) v: &'a [Option<f64>],
    pub(super) lo: &'a [u64],
}

/// One step: writes the outputs and their `lo`s; commits when `commit`.
pub(super) fn step(
    op: &Op,
    st: &mut State,
    x: &In<'_>,
    cx: &Ctx<'_>,
    out: &mut [Option<f64>],
    out_lo: &mut [u64],
    commit: bool,
) {
    match (op, st) {
        (
            Op::Window { .. } | Op::Macd { .. } | Op::Bollinger { .. },
            State::Kernel { k, back, lows },
        ) => {
            kernel(k, *back, lows, x, out, out_lo, commit);
        }
        (Op::Atr { .. }, State::Atr(a)) => {
            out[0] = a.peek_hl(cx.bar.h, cx.bar.l);
            out_lo[0] = 0;
            if commit {
                a.commit_hlc(cx.bar.h, cx.bar.l, cx.bar.c);
            }
        }
        (Op::Lag { n, .. }, State::Lag(ring)) => {
            let (v, lo) = if *n == 0 {
                (x.v[0], x.lo[0])
            } else if ring.len() == *n {
                ring.front().copied().unwrap_or((None, x.lo[0]))
            } else {
                (None, x.lo[0])
            };
            out[0] = v;
            out_lo[0] = lo;
            if commit && *n > 0 {
                if ring.len() == *n {
                    ring.pop_front();
                }
                ring.push_back((x.v[0], x.lo[0]));
            }
        }
        (Op::Binary { kind, .. }, State::Cross(prev)) => {
            cross(*kind, prev, x, out, out_lo, commit);
        }
        (Op::Binary { kind, .. }, _) => {
            out[0] = binary(*kind, x.v[0], x.v[1]);
            out_lo[0] = x.lo[0].min(x.lo[1]);
        }
        (Op::Unary { kind, .. }, _) => {
            out[0] = unary(*kind, x.v[0]);
            out_lo[0] = x.lo[0];
        }
        (Op::Line { a, b: bb }, State::Line(ia, ib)) => {
            line(a, bb, ia, ib, cx, out, commit);
            out_lo[0] = cx.lo;
        }
        (_, st) => {
            out_lo[0] = 0;
            if !state_step(op, st, x, cx, out, commit) {
                source(op, x, cx, out, out_lo);
            }
        }
    }
}

/// Inputs, constants, `align` and `bucket` (stateless, not arithmetic).
fn source(op: &Op, x: &In<'_>, cx: &Ctx<'_>, out: &mut [Option<f64>], out_lo: &mut [u64]) {
    let (v, lo) = match op {
        Op::Input(f) => (field(cx.bar, *f), cx.lo),
        Op::Const(Literal::Num(v)) => (finite(*v), LO_FREE),
        Op::Const(Literal::Bool(v)) => (b(*v), LO_FREE),
        #[allow(clippy::cast_precision_loss)]
        Op::BarIndex => (Some(cx.i as f64), cx.lo),
        #[allow(clippy::cast_precision_loss)]
        Op::Time => (Some(cx.bar.t as f64), cx.lo),
        Op::Align { .. } => (x.v[0], x.lo[0]),
        Op::Bucket { edges, .. } => (bucket(edges, x.v[0]), x.lo[0]),
        _ => (None, cx.lo),
    };
    out[0] = v;
    out_lo[0] = lo;
}

/// The `Chain` rule around a kernel: undefined input → undefined outputs, no commit.
fn kernel(
    k: &mut Kernel,
    back: Option<usize>,
    lows: &mut VecDeque<u64>,
    x: &In<'_>,
    out: &mut [Option<f64>],
    out_lo: &mut [u64],
    commit: bool,
) {
    let Some(v) = x.v[0] else {
        out.iter_mut().for_each(|o| *o = None);
        out_lo.iter_mut().for_each(|l| *l = x.lo[0]);
        return;
    };
    let o: Outs = k.peek(v);
    let lo = match back {
        None => 0,
        Some(_) => lows.front().copied().unwrap_or(x.lo[0]),
    };
    for (slot, val) in out.iter_mut().zip(o) {
        *slot = val;
    }
    out_lo.iter_mut().for_each(|l| *l = lo);
    if commit {
        k.commit(v);
        if let Some(m) = back.filter(|m| *m > 0) {
            if lows.len() == m {
                lows.pop_front();
            }
            lows.push_back(x.lo[0]);
        }
    }
}

/// `cross_above`, `cross_below` (RX4 §4.5): undefined now → undefined; no previous
/// defined pair → false.
fn cross(
    kind: BinaryKind,
    prev: &mut Option<(Option<f64>, Option<f64>, u64)>,
    x: &In<'_>,
    out: &mut [Option<f64>],
    out_lo: &mut [u64],
    commit: bool,
) {
    let (a, b_) = (x.v[0], x.v[1]);
    let lo_now = x.lo[0].min(x.lo[1]);
    out[0] = match (a, b_, *prev) {
        (Some(a), Some(bv), Some((Some(pa), Some(pb), _))) => {
            b(if kind == BinaryKind::CrossAbove {
                pa < pb && a >= bv
            } else {
                pa > pb && a <= bv
            })
        }
        (Some(_), Some(_), _) => b(false),
        _ => None,
    };
    out_lo[0] = prev.map_or(lo_now, |(_, _, l)| l.min(lo_now));
    if commit {
        *prev = Some((a, b_, lo_now));
    }
}

/// Arithmetic, comparison and logic (RX4 §4.3, §4.5): undefined spreads.
fn binary(kind: BinaryKind, a: Option<f64>, bv: Option<f64>) -> Option<f64> {
    let (a, bv) = (a?, bv?);
    match kind {
        BinaryKind::Add => finite(a + bv),
        BinaryKind::Sub => finite(a - bv),
        BinaryKind::Mul => finite(a * bv),
        BinaryKind::Div => finite(a / bv),
        BinaryKind::Min => Some(if bv < a { bv } else { a }),
        BinaryKind::Max => Some(if bv > a { bv } else { a }),
        BinaryKind::Gt => b(a > bv),
        BinaryKind::Ge => b(a >= bv),
        BinaryKind::Lt => b(a < bv),
        BinaryKind::Le => b(a <= bv),
        BinaryKind::Eq => b(a == bv),
        BinaryKind::Ne => b(a != bv),
        BinaryKind::And => b(a != 0.0 && bv != 0.0),
        BinaryKind::Or => b(a != 0.0 || bv != 0.0),
        BinaryKind::CrossAbove | BinaryKind::CrossBelow => None,
    }
}

fn unary(kind: UnaryKind, x: Option<f64>) -> Option<f64> {
    match kind {
        UnaryKind::Defined => b(x.is_some()),
        UnaryKind::Neg => x.map(|v| -v),
        UnaryKind::Abs => x.map(f64::abs),
        UnaryKind::Not => x.map(|v| if v == 0.0 { 1.0 } else { 0.0 }),
    }
}

/// `bucket` (RX4 §4.9): the number of edges at or below `x`.
fn bucket(edges: &[f64], x: Option<f64>) -> Option<f64> {
    let x = x?;
    #[allow(clippy::cast_precision_loss)]
    Some(edges.iter().filter(|e| **e <= x).count() as f64)
}

/// `line` (RX4 §4.8): `L(i) = a.price + (b.price − a.price) × (i − ia) / (ib − ia)`,
/// defined once both anchors resolve to different bars and `t_i ≥ b.t`.
#[allow(clippy::cast_precision_loss)]
fn line(
    a: &Anchor,
    bb: &Anchor,
    ia: &mut Option<u64>,
    ib: &mut Option<u64>,
    cx: &Ctx<'_>,
    out: &mut [Option<f64>],
    commit: bool,
) {
    let t = cx.bar.t;
    let na = if t <= a.t { Some(cx.i) } else { *ia };
    let nb = if t <= bb.t { Some(cx.i) } else { *ib };
    out[0] = match (na, nb) {
        (Some(x), Some(y)) if x != y && t >= bb.t => finite(
            a.price + (bb.price - a.price) * (cx.i as f64 - x as f64) / (y as f64 - x as f64),
        ),
        _ => None,
    };
    if commit {
        *ia = na;
        *ib = nb;
    }
}

/// The state nodes (RX4 §4.6); `false` when `op` is not one.
fn state_step(
    op: &Op,
    st: &mut State,
    x: &In<'_>,
    cx: &Ctx<'_>,
    out: &mut [Option<f64>],
    commit: bool,
) -> bool {
    let c = truthy(x.v[0]);
    match (op, st) {
        (Op::Cond { .. }, State::Streak(n)) => {
            let v = if c { *n + 1 } else { 0 };
            #[allow(clippy::cast_precision_loss)]
            {
                out[0] = Some(v as f64);
            }
            if commit {
                *n = v;
            }
        }
        (Op::Cond { .. }, State::BarsSince(last)) => {
            let l = if c { Some(cx.i) } else { *last };
            #[allow(clippy::cast_precision_loss)]
            {
                out[0] = l.map(|l| (cx.i - l) as f64);
            }
            if commit {
                *last = l;
            }
        }
        (Op::Cond { .. }, State::Once(done)) => {
            out[0] = b(c && !*done);
            if commit && c {
                *done = true;
            }
        }
        (Op::Rearm { .. }, State::Rearm(armed)) => {
            let reset = truthy(x.v[1]);
            let fire = *armed && c;
            out[0] = b(fire);
            if commit {
                if fire {
                    *armed = false;
                } else if !*armed && reset {
                    *armed = true;
                }
            }
        }
        (Op::Cooldown { bars, .. }, State::Cooldown(last)) => {
            let fire = c && last.is_none_or(|l| cx.i - l > *bars as u64);
            out[0] = b(fire);
            if commit && fire {
                *last = Some(cx.i);
            }
        }
        _ => return false,
    }
    true
}
