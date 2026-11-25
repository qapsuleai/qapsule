//! One node (PR §4): id, op, fields — read strictly, every field checked even after an
//! error — then typed and clocked (`types.rs`). A node with any error is broken; nodes
//! reading it stay silent (PR §11.1: no noise).

use qp_schema::program::BarField;
use serde_json::Value;

use super::args::{const_value, int_arg, num_arg, num_value, ref_arg, IntRole, NumRole, MAX_EDGES};
use super::clock::tf_clock;
use super::cost::weight;
use super::fields::Fields;
use super::types::infer;
use super::{Cx, NodeEntry, Typed};
use crate::checked::{Anchor, BinaryKind, Clock, CondKind, Op, Slot, UnaryKind, WindowKind};
use crate::diag::{join_i, Diags};
use crate::message::{Msg, Space};
use crate::names::{did_you_mean, is_node_id, suggest, BAR_FIELDS, OP_NAMES};
use crate::reason::Reason;

/// A node's fields, read.
pub(super) struct Parsed {
    pub(super) op: Op,
    /// Reference fields in field order.
    pub(super) ins: Vec<(&'static str, Slot)>,
    /// The fixed clock of a source (`input`, `atr`), if any.
    pub(super) clock: Option<Clock>,
}

/// Reader state of one node.
struct Rd<'c, 'a, 'v> {
    cx: &'c mut Cx<'a>,
    f: Fields<'v>,
    here: usize,
    ins: Vec<(&'static str, Slot)>,
}

impl Rd<'_, '_, '_> {
    fn r(&mut self, name: &'static str) -> Option<Slot> {
        let s = ref_arg(self.cx, &mut self.f, name, self.here)?;
        self.ins.push((name, s));
        Some(s)
    }

    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    fn int(&mut self, name: &'static str, role: IntRole) -> Option<usize> {
        int_arg(self.cx, &mut self.f, name, role).map(|i| i as usize)
    }

    fn num(&mut self, name: &'static str, role: NumRole) -> Option<f64> {
        num_arg(self.cx, &mut self.f, name, role)
    }

    fn tf(&mut self) -> Option<Clock> {
        let at = self.f.ptr("tf");
        let s = self.f.opt_str("tf", &mut self.cx.d);
        if self.f.has("tf") && s.is_none() {
            return None;
        }
        tf_clock(self.cx, s, &at)
    }
}

/// Checks node `i` and returns what later nodes see of it.
pub(super) fn node(cx: &mut Cx<'_>, v: &Value, i: usize) -> NodeEntry {
    let at = join_i("/nodes", i);
    let broken = NodeEntry {
        op: None,
        typed: None,
    };
    let Some(mut f) = Fields::new(v, &at, "nodes[]", &mut cx.d) else {
        return broken;
    };
    let before = cx.d.count();
    if let Some(id) = f.req_str("id", &mut cx.d) {
        if !is_node_id(id) {
            cx.d.at(Reason::BadParam, &f.ptr("id"), &Msg::BadName(id.to_owned()))
                .expected(["^[A-Za-z_][A-Za-z0-9_]{0,63}$"]);
        } else if cx.ids.get(id).is_some_and(|&j| j != i) {
            cx.d.at(
                Reason::DuplicateName,
                &f.ptr("id"),
                &Msg::Duplicate(Space::Node, id.to_owned()),
            );
        }
    }
    let Some(name) = f.req_str("op", &mut cx.d) else {
        return broken;
    };
    let Some(op) = OP_NAMES.iter().copied().find(|o| *o == name) else {
        cx.d.at(
            Reason::UnknownOp,
            &f.ptr("op"),
            &Msg::UnknownOp(name.to_owned()),
        )
        .expected(OP_NAMES)
        .dym(suggest(name, &OP_NAMES));
        return broken;
    };
    let mut rd = Rd {
        cx,
        f,
        here: i,
        ins: Vec::new(),
    };
    let parsed = parse(&mut rd, op);
    let Rd { cx, f, ins, .. } = rd;
    f.finish(&mut cx.d, &["partial"]);
    let mut entry = NodeEntry {
        op: Some(op),
        typed: None,
    };
    if cx.d.count() != before {
        return entry;
    }
    let Some((op_, clock)) = parsed else {
        return entry;
    };
    let parsed = Parsed {
        op: op_,
        ins,
        clock,
    };
    if let Some((outputs, clock)) = infer(cx, &parsed, &at) {
        let w = weight(&parsed.op);
        entry.typed = Some(Typed {
            op: parsed.op,
            outputs,
            clock,
            w,
        });
    }
    entry
}

fn parse(rd: &mut Rd<'_, '_, '_>, op: &'static str) -> Option<(Op, Option<Clock>)> {
    if let Some(kind) = window_kind(op) {
        let x = rd.r("x");
        let period = rd.int("period", IntRole::Window);
        return Some((
            Op::Window {
                kind,
                x: x?,
                period: period?,
            },
            None,
        ));
    }
    if let Some(kind) = binary_kind(op) {
        let (a, b) = (rd.r("a"), rd.r("b"));
        return Some((Op::Binary { kind, a: a?, b: b? }, None));
    }
    if let Some(kind) = unary_kind(op) {
        return Some((
            Op::Unary {
                kind,
                x: rd.r("x")?,
            },
            None,
        ));
    }
    if let Some(kind) = cond_kind(op) {
        return Some((
            Op::Cond {
                kind,
                c: rd.r("c")?,
            },
            None,
        ));
    }
    parse_other(rd, op)
}

fn parse_other(rd: &mut Rd<'_, '_, '_>, op: &'static str) -> Option<(Op, Option<Clock>)> {
    let op = match op {
        "input" => return parse_input(rd),
        "atr" => {
            let period = rd.int("period", IntRole::Window);
            let clock = rd.tf();
            return Some((Op::Atr { period: period? }, Some(clock?)));
        }
        "const" => {
            let at = rd.f.ptr("value");
            let v = rd.f.req("value", &mut rd.cx.d)?;
            Op::Const(const_value(rd.cx, v, &at)?)
        }
        "bar_index" => Op::BarIndex,
        "time" => Op::Time,
        "macd" => return parse_macd(rd),
        "bollinger" => {
            let x = rd.r("x");
            let period = rd.int("period", IntRole::Window);
            let k = rd.num("k", NumRole::Positive);
            Op::Bollinger {
                x: x?,
                period: period?,
                k: k?,
            }
        }
        "lag" => {
            let x = rd.r("x");
            let n = rd.int("n", IntRole::Lag);
            Op::Lag { x: x?, n: n? }
        }
        "rearm" => {
            let (trigger, reset) = (rd.r("trigger"), rd.r("reset"));
            Op::Rearm {
                trigger: trigger?,
                reset: reset?,
            }
        }
        "cooldown" => {
            let trigger = rd.r("trigger");
            let bars = rd.int("bars", IntRole::Bars);
            Op::Cooldown {
                trigger: trigger?,
                bars: bars?,
            }
        }
        "align" => Op::Align { x: rd.r("x")? },
        "line" => return parse_line(rd).map(|op| (op, None)),
        _ => return parse_bucket(rd).map(|op| (op, None)),
    };
    Some((op, None))
}

fn parse_input(rd: &mut Rd<'_, '_, '_>) -> Option<(Op, Option<Clock>)> {
    if let Some(p) = rd.f.opt("partial") {
        let at = rd.f.ptr("partial");
        if p == &Value::Bool(true) {
            rd.cx
                .d
                .at(Reason::Lookahead, &at, &Msg::Partial)
                .hint(&Msg::HintLookahead);
        } else {
            rd.cx.d.at(
                Reason::UnknownField,
                &at,
                &Msg::UnknownField("partial".to_owned()),
            );
        }
    }
    let at = rd.f.ptr("field");
    let field =
        rd.f.req_str("field", &mut rd.cx.d)
            .and_then(|s| bar_field(&mut rd.cx.d, s, &at));
    let clock = rd.tf();
    Some((Op::Input(field?), Some(clock?)))
}

fn bar_field(d: &mut Diags, s: &str, at: &str) -> Option<BarField> {
    Some(match s {
        "open" => BarField::Open,
        "high" => BarField::High,
        "low" => BarField::Low,
        "close" => BarField::Close,
        "volume" => BarField::Volume,
        _ => {
            d.at(
                Reason::UnknownField,
                at,
                &Msg::UnknownBarField(s.to_owned()),
            )
            .expected(BAR_FIELDS)
            .dym(did_you_mean(s, BAR_FIELDS));
            return None;
        }
    })
}

fn parse_macd(rd: &mut Rd<'_, '_, '_>) -> Option<(Op, Option<Clock>)> {
    let x = rd.r("x");
    let fast = rd.int("fast", IntRole::Window);
    let slow = rd.int("slow", IntRole::Window);
    let signal = rd.int("signal", IntRole::Window);
    let (fast, slow) = (fast?, slow?);
    if fast >= slow {
        let at = rd.f.ptr("slow");
        #[allow(clippy::cast_possible_wrap)]
        let m = Msg::MacdOrder(fast as i64, slow as i64);
        rd.cx.d.at(Reason::BadParam, &at, &m);
        return None;
    }
    Some((
        Op::Macd {
            x: x?,
            fast,
            slow,
            signal: signal?,
        },
        None,
    ))
}

fn anchor(rd: &mut Rd<'_, '_, '_>, name: &'static str) -> Option<Anchor> {
    let at = rd.f.ptr(name);
    let v = rd.f.req(name, &mut rd.cx.d)?;
    let mut f = Fields::new(v, &at, name, &mut rd.cx.d)?;
    let t = int_arg(rd.cx, &mut f, "t", IntRole::Time);
    let price = num_arg(rd.cx, &mut f, "price", NumRole::Positive);
    f.finish(&mut rd.cx.d, &[]);
    Some(Anchor {
        t: t?,
        price: price?,
    })
}

fn parse_line(rd: &mut Rd<'_, '_, '_>) -> Option<Op> {
    let (a, b) = (anchor(rd, "a"), anchor(rd, "b"));
    let (a, b) = (a?, b?);
    if a.t >= b.t {
        let at = format!("{}/t", rd.f.ptr("b"));
        rd.cx.d.at(Reason::BadParam, &at, &Msg::LineOrder);
        return None;
    }
    Some(Op::Line { a, b })
}

fn parse_bucket(rd: &mut Rd<'_, '_, '_>) -> Option<Op> {
    let x = rd.r("x");
    let at = rd.f.ptr("edges");
    let v = rd.f.req("edges", &mut rd.cx.d)?;
    let Value::Array(items) = v else {
        rd.cx
            .d
            .at(Reason::Type, &at, &Msg::NotArray("edges".to_owned()));
        return None;
    };
    if items.is_empty() || items.len() > MAX_EDGES {
        rd.cx
            .d
            .at(Reason::BadParam, &at, &Msg::EdgesCount(items.len()))
            .expected([format!("1..={MAX_EDGES}")]);
        return None;
    }
    let edges: Vec<Option<f64>> = items
        .iter()
        .enumerate()
        .map(|(k, e)| num_value(rd.cx, e, &join_i(&at, k), "edges", NumRole::Edge))
        .collect();
    let edges: Vec<f64> = edges.into_iter().collect::<Option<_>>()?;
    if edges.windows(2).any(|w| w[0] >= w[1]) {
        rd.cx.d.at(Reason::BadParam, &at, &Msg::EdgesOrder);
        return None;
    }
    Some(Op::Bucket { x: x?, edges })
}

fn window_kind(op: &str) -> Option<WindowKind> {
    Some(match op {
        "sma" => WindowKind::Sma,
        "ema" => WindowKind::Ema,
        "rsi" => WindowKind::Rsi,
        "highest" => WindowKind::Highest,
        "lowest" => WindowKind::Lowest,
        "stdev" => WindowKind::Stdev,
        "percentrank" => WindowKind::Percentrank,
        _ => return None,
    })
}

fn binary_kind(op: &str) -> Option<BinaryKind> {
    use BinaryKind::*;
    Some(match op {
        "add" => Add,
        "sub" => Sub,
        "mul" => Mul,
        "div" => Div,
        "min" => Min,
        "max" => Max,
        "gt" => Gt,
        "ge" => Ge,
        "lt" => Lt,
        "le" => Le,
        "eq" => Eq,
        "ne" => Ne,
        "and" => And,
        "or" => Or,
        "cross_above" => CrossAbove,
        "cross_below" => CrossBelow,
        _ => return None,
    })
}

fn unary_kind(op: &str) -> Option<UnaryKind> {
    Some(match op {
        "neg" => UnaryKind::Neg,
        "abs" => UnaryKind::Abs,
        "not" => UnaryKind::Not,
        "defined" => UnaryKind::Defined,
        _ => return None,
    })
}

fn cond_kind(op: &str) -> Option<CondKind> {
    Some(match op {
        "streak" => CondKind::Streak,
        "bars_since" => CondKind::BarsSince,
        "once" => CondKind::Once,
        _ => return None,
    })
}
