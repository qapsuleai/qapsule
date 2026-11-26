//! Types and clocks of a node from its inputs (PR §3, §4): `num`, `bool`, `cat` (only
//! into `defined`, `align` and state outputs), and one clock per node (`align` the only way
//! down).

use super::clock::merge;
use super::nodes::Parsed;
use super::Cx;
use crate::checked::{BinaryKind, Clock, CondKind, Literal, Op, Ty, UnaryKind};
use crate::diag::join;
use crate::message::Msg;
use crate::reason::Reason;

/// What an input accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Want {
    Num,
    Bool,
    NumOrBool,
    Any,
}

impl Want {
    const fn accepts(self, t: Ty) -> bool {
        match self {
            Want::Num => matches!(t, Ty::Num),
            Want::Bool => matches!(t, Ty::Bool),
            Want::NumOrBool => !matches!(t, Ty::Cat),
            Want::Any => true,
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Want::Num => "num",
            Want::Bool => "bool",
            Want::NumOrBool => "num or bool",
            Want::Any => "num, bool or cat",
        }
    }
}

/// What the node yields from its inputs.
enum Out {
    /// Fixed output types.
    Fixed(Vec<Ty>),
    /// The first input's type.
    Same,
}

/// `(input want, output, whether the node keeps history)`.
fn signature(op: &Op) -> (Want, Out, bool) {
    use BinaryKind::*;
    let num = |n: usize| Out::Fixed(vec![Ty::Num; n]);
    match op {
        Op::Window { .. } => (Want::Num, num(1), true),
        Op::Macd { .. } => (Want::Num, num(3), true),
        Op::Bollinger { .. } => (Want::Num, num(4), true),
        Op::Binary { kind, .. } => match kind {
            Add | Sub | Mul | Div | Min | Max => (Want::Num, num(1), false),
            Gt | Ge | Lt | Le | Eq | Ne => (Want::Num, Out::Fixed(vec![Ty::Bool]), false),
            And | Or => (Want::Bool, Out::Fixed(vec![Ty::Bool]), false),
            CrossAbove | CrossBelow => (Want::Num, Out::Fixed(vec![Ty::Bool]), true),
        },
        Op::Unary { kind, .. } => match kind {
            UnaryKind::Neg | UnaryKind::Abs => (Want::Num, num(1), false),
            UnaryKind::Not => (Want::Bool, Out::Fixed(vec![Ty::Bool]), false),
            UnaryKind::Defined => (Want::Any, Out::Fixed(vec![Ty::Bool]), false),
        },
        Op::Lag { .. } => (Want::NumOrBool, Out::Same, true),
        Op::Cond { kind, .. } => match kind {
            CondKind::Streak | CondKind::BarsSince => (Want::Bool, num(1), true),
            CondKind::Once => (Want::Bool, Out::Fixed(vec![Ty::Bool]), true),
        },
        Op::Rearm { .. } | Op::Cooldown { .. } => (Want::Bool, Out::Fixed(vec![Ty::Bool]), true),
        // PR §4.6: the result has `x`'s type, so a higher-timeframe `cat` comes down too.
        Op::Align { .. } => (Want::Any, Out::Same, false),
        Op::Bucket { .. } => (Want::Num, Out::Fixed(vec![Ty::Cat]), false),
        Op::Const(Literal::Bool(_)) => (Want::Any, Out::Fixed(vec![Ty::Bool]), false),
        Op::Input(_)
        | Op::Const(Literal::Num(_))
        | Op::BarIndex
        | Op::Time
        | Op::Atr { .. }
        | Op::Line { .. } => (Want::Any, num(1), false),
    }
}

/// Output types and clock of a parsed node at `at`; `None` after an error.
pub(super) fn infer(cx: &mut Cx<'_>, p: &Parsed, at: &str) -> Option<(Vec<Ty>, Clock)> {
    let (want, out, history) = signature(&p.op);
    let mut ins = Vec::with_capacity(p.ins.len());
    let mut ok = true;
    for (field, slot) in &p.ins {
        let typed = cx.nodes[slot.node].typed.as_ref()?;
        let (ty, clock) = (typed.outputs[slot.out], typed.clock.clone());
        if !want.accepts(ty) {
            let m = Msg::TypeMismatch((*field).to_owned(), want.name(), ty.name());
            cx.d.at(Reason::Type, &join(at, field), &m)
                .expected([want.name()]);
            ok = false;
        }
        ins.push((*field, ty, clock));
    }
    let tys = match out {
        Out::Fixed(t) => t,
        Out::Same => vec![ins.first()?.1],
    };
    let clock = node_clock(cx, p, &ins, history, at)?;
    ok.then_some((tys, clock))
}

fn node_clock(
    cx: &mut Cx<'_>,
    p: &Parsed,
    ins: &[(&'static str, Ty, Clock)],
    history: bool,
    at: &str,
) -> Option<Clock> {
    if let Some(c) = &p.clock {
        return Some(c.clone());
    }
    match &p.op {
        Op::Const(_) => return Some(Clock::Free),
        Op::BarIndex | Op::Time | Op::Line { .. } => return Some(Clock::Base),
        Op::Align { .. } => {
            let (field, _, c) = ins.first()?;
            if matches!(c, Clock::Tf(_)) {
                return Some(Clock::Base);
            }
            let m = Msg::AlignNeedsHigher(c.name().to_owned());
            cx.d.at(Reason::ClockMismatch, &join(at, field), &m)
                .expected(["4h", "1d", "1w", "1M"]);
            return None;
        }
        _ => {}
    }
    let clocks: Vec<&Clock> = ins.iter().map(|(_, _, c)| c).collect();
    match merge(&clocks, history) {
        Ok(c) => Some(c),
        Err((i, a, b)) => {
            let m = Msg::ClockMismatch(a.clone(), b);
            cx.d.at(Reason::ClockMismatch, &join(at, ins[i].0), &m)
                .expected([a]);
            None
        }
    }
}
