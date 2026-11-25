//! Expressions in canonical text: single spaces around operators, parentheses only where
//! precedence or the IR's tree shape needs them. Every rendering also carries the two
//! depths the parser will measure on it (tree height and recursion depth), so that the
//! planner (`plan.rs`) can cut long inlined chains before the parser would refuse them.

use std::collections::BTreeMap;

use qp_schema::program::{BarField, BinaryNode, ConstValue, NodeOp, Output, OutputKind, Program};

use super::lit::{fmt_num, int_arg};
use super::text::num;

const OR: u8 = 1;
const AND: u8 = 2;
const NOT: u8 = 3;
const CMP: u8 = 4;
const ADD: u8 = 5;
const MUL: u8 = 6;
const NEG: u8 = 7;
const POST: u8 = 8;
const ATOM: u8 = 9;

/// What the printer knows about a program.
pub(super) struct Printer<'a> {
    pub(super) p: &'a Program,
    pub(super) index: BTreeMap<&'a str, usize>,
    pub(super) inline: Vec<bool>,
    pub(super) names: Vec<String>,
}

/// A rendered expression: text, precedence, and what the parser will measure on it —
/// `h`, the syntax-tree height (`Expr::new`), and `rd`, the parser recursion levels it
/// enters below the statement's own expression (parentheses, call arguments, list items,
/// `[n]`, each `not`, each non-literal `-`). The parser accepts `h <= max_depth` and
/// `rd < max_depth`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct R {
    pub(super) s: String,
    prec: u8,
    pub(super) h: usize,
    pub(super) rd: usize,
}

impl R {
    fn leaf(s: String, prec: u8) -> R {
        R {
            s,
            prec,
            h: 1,
            rd: 0,
        }
    }

    /// Whether the parser accepts it under `max`.
    pub(super) fn fits(&self, max: usize) -> bool {
        self.h <= max && self.rd < max
    }
}

/// The node id of a reference (`id` of `id.output`).
pub(super) fn base(r: &str) -> (&str, Option<&str>) {
    match r.split_once('.') {
        Some((b, out)) => (b, Some(out)),
        None => (r, None),
    }
}

pub(super) fn output_refs(o: &Output) -> Vec<&str> {
    match &o.kind {
        OutputKind::Series { node } | OutputKind::Event { node } => vec![node.as_str()],
        OutputKind::Signal { node, .. } => vec![node.as_str()],
        OutputKind::State { dims } => dims.iter().map(|d| d.node.as_str()).collect(),
    }
}

impl Printer<'_> {
    pub(super) fn inlined(&self, i: usize) -> bool {
        self.inline[i]
    }

    pub(super) fn name(&self, i: usize) -> &str {
        &self.names[i]
    }

    /// Where an output line goes among the node lines: at its own inlined root, or after
    /// the last node it reads.
    pub(super) fn output_pos(&self, o: &Output) -> usize {
        output_refs(o)
            .into_iter()
            .filter_map(|r| self.index.get(base(r).0).copied())
            .max()
            .unwrap_or(0)
    }

    /// A reference at the top of a statement.
    pub(super) fn top(&self, r: &str) -> String {
        self.reference(r).s
    }

    /// A node's operation at the top of its line.
    pub(super) fn op(&self, op: &NodeOp) -> String {
        self.render(op).s
    }

    pub(super) fn reference(&self, r: &str) -> R {
        let (b, out) = base(r);
        let x = match self.index.get(b) {
            Some(&i) if self.inline[i] => self.render(&self.p.nodes[i].op),
            Some(&i) => R::leaf(self.names[i].clone(), ATOM),
            None => R::leaf(b.to_owned(), ATOM),
        };
        match out {
            Some(out) => {
                let w = wrap(x, POST);
                R {
                    s: format!("{}.{out}", w.s),
                    prec: POST,
                    h: w.h + out.split('.').count(),
                    rd: w.rd,
                }
            }
            None => x,
        }
    }

    fn operand(&self, r: &str, min: u8) -> R {
        wrap(self.reference(r), min)
    }

    fn call(&self, name: &str, args: Vec<R>) -> R {
        let h = 1 + args.iter().map(|a| a.h).max().unwrap_or(0);
        let rd = args.iter().map(|a| a.rd + 1).max().unwrap_or(0);
        let ss: Vec<String> = args.into_iter().map(|a| a.s).collect();
        R {
            s: format!("{name}({})", ss.join(", ")),
            prec: ATOM,
            h,
            rd,
        }
    }

    fn arg(&self, r: &str) -> R {
        self.reference(r)
    }

    fn bin(&self, b: &BinaryNode, sym: &str, prec: u8, left_eq: bool) -> R {
        let lmin = if left_eq { prec } else { prec + 1 };
        let l = self.operand(&b.a, lmin);
        let r = self.operand(&b.b, prec + 1);
        R {
            s: format!("{} {sym} {}", l.s, r.s),
            prec,
            h: 1 + l.h.max(r.h),
            rd: l.rd.max(r.rd),
        }
    }

    pub(super) fn render(&self, op: &NodeOp) -> R {
        match op {
            NodeOp::Input(i) => {
                let f = field_name(i.field);
                match &i.tf {
                    Some(tf) => R::leaf(format!("{f}@{tf}"), ATOM),
                    None => R::leaf(f.to_owned(), ATOM),
                }
            }
            NodeOp::Const(c) => match &c.value {
                ConstValue::Bool(b) => R::leaf(b.to_string(), ATOM),
                ConstValue::Num(f) => {
                    let s = fmt_num(*f);
                    let p = if s.starts_with('-') { NEG } else { ATOM };
                    R::leaf(s, p)
                }
                ConstValue::Param(p) => R::leaf(p.param.clone(), ATOM),
            },
            NodeOp::BarIndex {} => R::leaf("bar_index".into(), ATOM),
            NodeOp::Time {} => R::leaf("time".into(), ATOM),
            NodeOp::Sma(w)
            | NodeOp::Ema(w)
            | NodeOp::Rsi(w)
            | NodeOp::Highest(w)
            | NodeOp::Lowest(w)
            | NodeOp::Stdev(w)
            | NodeOp::Percentrank(w) => self.call(op.name(), vec![self.arg(&w.x), int(&w.period)]),
            NodeOp::Macd(m) => self.call(
                "macd",
                vec![self.arg(&m.x), int(&m.fast), int(&m.slow), int(&m.signal)],
            ),
            NodeOp::Bollinger(b) => self.call(
                "bollinger",
                vec![self.arg(&b.x), int(&b.period), lit(num(&b.k))],
            ),
            NodeOp::Atr(a) => {
                let mut args = vec![int(&a.period)];
                args.extend(a.tf.clone().map(lit));
                self.call("atr", args)
            }
            _ => self.render_rest(op),
        }
    }

    fn render_rest(&self, op: &NodeOp) -> R {
        match op {
            NodeOp::Add(b) => self.bin(b, "+", ADD, true),
            NodeOp::Sub(b) => self.bin(b, "-", ADD, true),
            NodeOp::Mul(b) => self.bin(b, "*", MUL, true),
            NodeOp::Div(b) => self.bin(b, "/", MUL, true),
            NodeOp::Gt(b) => self.bin(b, ">", CMP, false),
            NodeOp::Ge(b) => self.bin(b, ">=", CMP, false),
            NodeOp::Lt(b) => self.bin(b, "<", CMP, false),
            NodeOp::Le(b) => self.bin(b, "<=", CMP, false),
            NodeOp::Eq(b) => self.bin(b, "==", CMP, false),
            NodeOp::Ne(b) => self.bin(b, "!=", CMP, false),
            NodeOp::And(b) => self.bin(b, "and", AND, true),
            NodeOp::Or(b) => self.bin(b, "or", OR, true),
            NodeOp::Min(b) | NodeOp::Max(b) | NodeOp::CrossAbove(b) | NodeOp::CrossBelow(b) => {
                self.call(op.name(), vec![self.arg(&b.a), self.arg(&b.b)])
            }
            NodeOp::Neg(u) => neg(self.reference(&u.x)),
            NodeOp::Not(u) => {
                let w = self.operand(&u.x, NOT);
                R {
                    s: format!("not {}", w.s),
                    prec: NOT,
                    h: w.h + 1,
                    rd: w.rd + 1,
                }
            }
            NodeOp::Lag(l) => {
                let w = self.operand(&l.x, POST);
                R {
                    s: format!("{}[{}]", w.s, int_arg(&l.n)),
                    prec: POST,
                    h: 1 + w.h.max(1),
                    rd: w.rd.max(1),
                }
            }
            NodeOp::Abs(u) | NodeOp::Defined(u) | NodeOp::Align(u) => {
                self.call(op.name(), vec![self.arg(&u.x)])
            }
            NodeOp::Streak(c) | NodeOp::BarsSince(c) | NodeOp::Once(c) => {
                self.call(op.name(), vec![self.arg(&c.c)])
            }
            NodeOp::Rearm(r) => self.call("rearm", vec![self.arg(&r.trigger), self.arg(&r.reset)]),
            NodeOp::Cooldown(c) => self.call("cooldown", vec![self.arg(&c.trigger), int(&c.bars)]),
            NodeOp::Line(l) => self.call(
                "line",
                vec![
                    int(&l.a.t),
                    lit(num(&l.a.price)),
                    int(&l.b.t),
                    lit(num(&l.b.price)),
                ],
            ),
            NodeOp::Bucket(b) => {
                let edges: Vec<String> = b.edges.iter().map(num).collect();
                let list = R {
                    h: if edges.is_empty() { 1 } else { 2 },
                    rd: usize::from(!edges.is_empty()),
                    s: format!("[{}]", edges.join(", ")),
                    prec: ATOM,
                };
                self.call("bucket", vec![self.arg(&b.x), list])
            }
            // Every other operation is rendered by `render`.
            _ => R::leaf(op.name().to_owned(), ATOM),
        }
    }
}

/// `-x`. A plain number operand is parenthesized (`-2` would read back as a negative
/// literal); an operand that starts with `-` is separated by a space (`- -x`), which the
/// parser reads at the same depth as the tree.
fn neg(x: R) -> R {
    let plain = !x.s.is_empty() && x.s.chars().all(|c| c.is_ascii_digit() || c == '.');
    let w = if plain { force_wrap(x) } else { wrap(x, NEG) };
    let sep = if w.s.starts_with('-') { " " } else { "" };
    R {
        s: format!("-{sep}{}", w.s),
        prec: NEG,
        h: w.h + 1,
        rd: w.rd + 1,
    }
}

/// A number field or other literal argument (always a leaf).
fn lit(s: String) -> R {
    R::leaf(s, ATOM)
}

fn int(a: &qp_schema::program::IntArg) -> R {
    lit(int_arg(a))
}

fn force_wrap(r: R) -> R {
    R {
        s: format!("({})", r.s),
        prec: ATOM,
        h: r.h,
        rd: r.rd + 1,
    }
}

fn wrap(r: R, min: u8) -> R {
    if r.prec < min {
        force_wrap(r)
    } else {
        r
    }
}

/// A bar field as written.
const fn field_name(f: BarField) -> &'static str {
    match f {
        BarField::Open => "open",
        BarField::High => "high",
        BarField::Low => "low",
        BarField::Close => "close",
        BarField::Volume => "volume",
    }
}
