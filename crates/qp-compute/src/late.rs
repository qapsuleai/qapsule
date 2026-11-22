//! Certified late start ("sandwich", RP1/E8): the outputs of a contracting recursive study
//! (EMA, RSI) for bars `[from, n)` without running the whole history, **bit-identical** to
//! a run from bar 0.
//!
//! The steady update of these studies is monotone non-decreasing in its state even with
//! rounding (`fl(s·c)` for `c ≥ 0` and `fl(k + s)` are monotone, so is their composition).
//! The true state at bar `s − 1` is unknown but bounded (EMA: within the range of the inputs
//! so far, widened for rounding; RSI: averages in `[0, largest gain/loss so far]`). Two runs
//! start from the lower and the upper bound; by induction `L ≤ T ≤ U` on every bar. When `L`
//! and `U` have the same bits, the true run `T` has them too, and from then on all three
//! perform the same operations. Every output after that bar is therefore exactly the full
//! run's output. If the bounds have not met before `from`, the start moves back (doubling
//! the distance), and finally the whole history runs. The result is always exact; only the
//! cost depends on the data.

use crate::studies::{Ema, Rsi};

/// How a tail was computed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Certificate {
    /// First bar that was computed (0 = the whole history ran).
    pub start: usize,
    /// Bar at which the bounds met (`None`: the whole history ran).
    pub met: Option<usize>,
    /// Bars stepped in total, both bound runs counted (work measure).
    pub steps: usize,
}

/// Relative widening of the input range used as state bounds. The rounding drift of an
/// average of values in `[lo, hi]` is below `~ 2·u·period` relative (`u = 2^-53`); 1e-6
/// covers every allowed period (≤ [`crate::MAX_PERIOD`]).
const WIDEN: f64 = 1e-6;

fn widen_lo(v: f64) -> f64 {
    v - v.abs() * WIDEN - f64::MIN_POSITIVE
}

fn widen_hi(v: f64) -> f64 {
    v + v.abs() * WIDEN + f64::MIN_POSITIVE
}

/// Bars per period the bounds typically need to meet (measured on random walks, E8):
/// EMA ≈ 18·p, RSI ≈ 37·p. The first late start looks back a bit more than that.
pub const EMA_LOOKBACK_PER_PERIOD: usize = 24;
/// See [`EMA_LOOKBACK_PER_PERIOD`].
pub const RSI_LOOKBACK_PER_PERIOD: usize = 48;
const MIN_LOOKBACK: usize = 64;

/// State bounds valid for every late start before `from`, computed once per input and
/// shared by all periods (looser bounds only cost a few more bars).
///
/// Preconditions of the guarantee: the bounds must be built from the **same** series that
/// is later passed to `*_tail_with` (they are not checked against it), and the period must
/// be at most ~1e8 (the 1e-6 widening covers the seed mean's rounding drift up to there;
/// periods are validated up to [`crate::MAX_PERIOD`] = 1e5 elsewhere).
#[derive(Debug, Clone, Copy)]
pub struct Bounds {
    from: usize,
    finite: bool,
    lo: f64,
    hi: f64,
    gain: f64,
    loss: f64,
}

impl Bounds {
    /// Bounds over `x[..from]`.
    #[must_use]
    pub fn new(x: &[f64], from: usize) -> Self {
        let h = &x[..from.min(x.len())];
        let finite = h.iter().all(|v| v.is_finite());
        let (mut lo, mut hi, mut gain, mut loss) =
            (f64::INFINITY, f64::NEG_INFINITY, 0.0_f64, 0.0_f64);
        for (i, &v) in h.iter().enumerate() {
            lo = lo.min(v);
            hi = hi.max(v);
            if i > 0 {
                let c = v - h[i - 1];
                gain = gain.max(c);
                loss = loss.max(-c);
            }
        }
        Self {
            from,
            finite,
            lo: widen_lo(lo),
            hi: widen_hi(hi),
            gain: widen_hi(gain),
            loss: widen_hi(loss),
        }
    }
}

/// EMA(`period`) outputs for bars `from..x.len()` into `out` (`NaN` in warmup), identical
/// to [`crate::run`] over the whole series.
///
/// # Panics
/// `period` of 0 or above [`crate::MAX_PERIOD`], `from > x.len()` or `out.len() != x.len() - from`.
pub fn ema_tail(x: &[f64], period: usize, from: usize, out: &mut [f64]) -> Certificate {
    ema_tail_with(x, period, &Bounds::new(x, from), out)
}

#[allow(clippy::expect_used)]
/// [`ema_tail`] with precomputed bounds (`bounds.from` is the first output bar).
///
/// # Panics
/// See [`ema_tail`].
pub fn ema_tail_with(x: &[f64], period: usize, bounds: &Bounds, out: &mut [f64]) -> Certificate {
    let from = bounds.from;
    let n = x.len();
    assert!(from <= n && out.len() == n - from);
    assert!(
        (1..=crate::MAX_PERIOD).contains(&period),
        "period out of range"
    );
    let pf = period as f64;
    let a = 2.0 / (pf + 1.0);
    let step = |s: f64, xi: f64| xi * a + s * (1.0 - a);
    let mut look = (EMA_LOOKBACK_PER_PERIOD * period).max(MIN_LOOKBACK);
    let mut steps = 0;
    let mut last = !bounds.finite;
    loop {
        // The bounds replace a steady state: bar s - 1 must be at or after the seed bar.
        let earliest = period.max(1);
        if last || from <= earliest {
            break;
        }
        let s = from.saturating_sub(look).max(earliest);
        // The last try starts at the earliest valid bar; after it, the whole history.
        last = s == earliest;
        let (mut l, mut u) = (bounds.lo, bounds.hi);
        let mut met = None;
        for (i, &xi) in x.iter().enumerate().take(from).skip(s) {
            l = step(l, xi);
            u = step(u, xi);
            steps += 2;
            if l.to_bits() == u.to_bits() {
                met = Some(i);
                break;
            }
        }
        if let Some(t) = met {
            let mut v = l;
            for i in t + 1..n {
                v = step(v, x[i]);
                steps += 1;
                if i >= from {
                    out[i - from] = v;
                }
            }
            return Certificate {
                start: s,
                met,
                steps,
            };
        }
        look = look.saturating_mul(2);
    }
    // Whole history.
    let mut e = Ema::new(period).expect("period >= 1");
    for (i, &xi) in x.iter().enumerate() {
        let v = e.peek1(xi);
        e.commit1(xi);
        if i >= from {
            out[i - from] = v.unwrap_or(f64::NAN);
        }
    }
    Certificate {
        start: 0,
        met: None,
        steps: steps + n,
    }
}

/// RSI(`period`) outputs for bars `from..x.len()` into `out`, identical to a full run.
///
/// # Panics
/// `period` of 0 or above [`crate::MAX_PERIOD`], `from > x.len()` or `out.len() != x.len() - from`.
pub fn rsi_tail(x: &[f64], period: usize, from: usize, out: &mut [f64]) -> Certificate {
    rsi_tail_with(x, period, &Bounds::new(x, from), out)
}

#[allow(clippy::expect_used)]
/// [`rsi_tail`] with precomputed bounds.
///
/// # Panics
/// See [`rsi_tail`].
pub fn rsi_tail_with(x: &[f64], period: usize, bounds: &Bounds, out: &mut [f64]) -> Certificate {
    let from = bounds.from;
    let n = x.len();
    assert!(from <= n && out.len() == n - from);
    assert!(
        (1..=crate::MAX_PERIOD).contains(&period),
        "period out of range"
    );
    let pf = period as f64;
    let pm1 = pf - 1.0;
    let split = |c: f64| {
        let up = c > 0.0;
        (if up { c } else { 0.0 }, if up { 0.0 } else { c.abs() })
    };
    let avg = |s: f64, g: f64| (s * pm1 + g) / pf;
    let mut look = (RSI_LOOKBACK_PER_PERIOD * period).max(MIN_LOOKBACK);
    let mut steps = 0;
    let mut last = !bounds.finite;
    loop {
        // Bar s - 1 must be after the initializing bar `period`.
        let earliest = period + 2;
        if last || from <= earliest {
            break;
        }
        let s = from.saturating_sub(look).max(earliest);
        last = s == earliest;
        // Averages are >= 0 exactly (sums and quotients of non-negative values).
        let (mut gl, mut gu, mut ll, mut lu) = (0.0, bounds.gain, 0.0, bounds.loss);
        let mut met = None;
        for i in s..from {
            let (g, l) = split(x[i] - x[i - 1]);
            gl = avg(gl, g);
            gu = avg(gu, g);
            ll = avg(ll, l);
            lu = avg(lu, l);
            steps += 2;
            if gl.to_bits() == gu.to_bits() && ll.to_bits() == lu.to_bits() {
                met = Some(i);
                break;
            }
        }
        if let Some(t) = met {
            let (mut ag, mut al) = (gl, ll);
            for i in t + 1..n {
                let (g, l) = split(x[i] - x[i - 1]);
                ag = avg(ag, g);
                al = avg(al, l);
                steps += 1;
                if i >= from {
                    out[i - from] = crate::studies::rsi_from_avg(ag, al);
                }
            }
            return Certificate {
                start: s,
                met,
                steps,
            };
        }
        look = look.saturating_mul(2);
    }
    let mut r = Rsi::new(period).expect("period >= 1");
    for (i, &xi) in x.iter().enumerate() {
        let v = r.peek1(xi);
        r.commit1(xi);
        if i >= from {
            out[i - from] = v.unwrap_or(f64::NAN);
        }
    }
    Certificate {
        start: 0,
        met: None,
        steps: steps + n,
    }
}
