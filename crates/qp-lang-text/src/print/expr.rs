//! Expressions in canonical text: single spaces around operators, parentheses only where
//! precedence or the IR's tree shape needs them.

use std::collections::BTreeMap;

use qp_schema::program::{BarField, BinaryNode, ConstValue, NodeOp, Output, OutputKind, Program};

use super::lit::{fmt_num, int_arg};
use super::text::{names, num};

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
    p: &'a Program,
    index: BTreeMap<&'a str, usize>,
    inline: Vec<bool>,
    names: Vec<String>,
}

/// The node id of a reference (`id` of `id.output`).
fn base(r: &str) -> (&str, Option<&str>) {
    match r.split_once('.') {
        Some((b, out)) => (b, Some(out)),
        None => (r, None),
    }
}

fn output_refs(o: &Output) -> Vec<&str> {
    match &o.kind {
        OutputKind::Series { node } | OutputKind::Event { node } => vec![node.as_str()],
        OutputKind::Signal { node, .. } => vec![node.as_str()],
        OutputKind::State { dims } => dims.iter().map(|d| d.node.as_str()).collect(),
    }
}

impl<'a> Printer<'a> {
    pub(super) fn new(p: &'a Program) -> Printer<'a> {
        let mut index = BTreeMap::new();
        for (i, n) in p.nodes.iter().enumerate() {
            index.entry(n.id.as_str()).or_insert(i);
        }
        let mut uses = vec![0usize; p.nodes.len()];
        let refs = p
            .nodes
            .iter()
            .flat_map(|n| n.op.refs())
            .chain(p.outputs.iter().flat_map(output_refs));
        for r in refs {
            if let Some(&i) = index.get(base(r).0) {
                uses[i] += 1;
            }
        }
        let inline: Vec<bool> = p
            .nodes
            .iter()
            .zip(&uses)
            .map(|(n, u)| n.id.starts_with('_') && *u == 1)
            .collect();
        let names = names(p, &inline);
        Printer {
            p,
            index,
            inline,
            names,
        }
    }

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
        self.reference(r).0
    }

    /// A node's operation at the top of its line.
    pub(super) fn op(&self, op: &NodeOp) -> String {
        self.render(op).0
    }

    fn reference(&self, r: &str) -> (String, u8) {
        let (b, out) = base(r);
        let (s, prec) = match self.index.get(b) {
            Some(&i) if self.inline[i] => self.render(&self.p.nodes[i].op),
            Some(&i) => (self.names[i].clone(), ATOM),
            None => (b.to_owned(), ATOM),
        };
        match out {
            Some(out) => (format!("{}.{out}", wrap(s, prec, POST)), POST),
            None => (s, prec),
        }
    }

    /// A reference as an operand needing at least precedence `min`.
    fn operand(&self, r: &str, min: u8) -> String {
        let (s, p) = self.reference(r);
        wrap(s, p, min)
    }

    fn call(&self, name: &str, args: &[String]) -> (String, u8) {
        (format!("{name}({})", args.join(", ")), ATOM)
    }

    fn arg(&self, r: &str) -> String {
        self.reference(r).0
    }

    fn bin(&self, b: &BinaryNode, sym: &str, prec: u8, left_eq: bool) -> (String, u8) {
        let lmin = if left_eq { prec } else { prec + 1 };
        let l = self.operand(&b.a, lmin);
        let r = self.operand(&b.b, prec + 1);
        (format!("{l} {sym} {r}"), prec)
    }

    fn render(&self, op: &NodeOp) -> (String, u8) {
        match op {
            NodeOp::Input(i) => {
                let f = field_name(i.field).to_owned();
                match &i.tf {
                    Some(tf) => (format!("{f}@{tf}"), ATOM),
                    None => (f, ATOM),
                }
            }
            NodeOp::Const(c) => match &c.value {
                ConstValue::Bool(b) => (b.to_string(), ATOM),
                ConstValue::Num(f) => {
                    let s = fmt_num(*f);
                    let p = if s.starts_with('-') { NEG } else { ATOM };
                    (s, p)
                }
                ConstValue::Param(p) => (p.param.clone(), ATOM),
            },
            NodeOp::BarIndex {} => ("bar_index".into(), ATOM),
            NodeOp::Time {} => ("time".into(), ATOM),
            NodeOp::Sma(w)
            | NodeOp::Ema(w)
            | NodeOp::Rsi(w)
            | NodeOp::Highest(w)
            | NodeOp::Lowest(w)
            | NodeOp::Stdev(w)
            | NodeOp::Percentrank(w) => self.call(op.name(), &[self.arg(&w.x), int_arg(&w.period)]),
            NodeOp::Macd(m) => self.call(
                "macd",
                &[
                    self.arg(&m.x),
                    int_arg(&m.fast),
                    int_arg(&m.slow),
                    int_arg(&m.signal),
                ],
            ),
            NodeOp::Bollinger(b) => self.call(
                "bollinger",
                &[self.arg(&b.x), int_arg(&b.period), num(&b.k)],
            ),
            NodeOp::Atr(a) => {
                let mut args = vec![int_arg(&a.period)];
                args.extend(a.tf.clone());
                self.call("atr", &args)
            }
            _ => self.render_rest(op),
        }
    }

    fn render_rest(&self, op: &NodeOp) -> (String, u8) {
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
                self.call(op.name(), &[self.arg(&b.a), self.arg(&b.b)])
            }
            NodeOp::Neg(u) => {
                let (s, p) = self.reference(&u.x);
                // `-2` would read back as a negative literal, `--x` is unclear.
                let literal = s.chars().all(|c| c.is_ascii_digit() || c == '.');
                let s = if literal || s.starts_with('-') {
                    format!("({s})")
                } else {
                    wrap(s, p, NEG)
                };
                (format!("-{s}"), NEG)
            }
            NodeOp::Not(u) => (format!("not {}", self.operand(&u.x, NOT)), NOT),
            NodeOp::Lag(l) => (
                format!("{}[{}]", self.operand(&l.x, POST), int_arg(&l.n)),
                POST,
            ),
            NodeOp::Abs(u) | NodeOp::Defined(u) | NodeOp::Align(u) => {
                self.call(op.name(), &[self.arg(&u.x)])
            }
            NodeOp::Streak(c) | NodeOp::BarsSince(c) | NodeOp::Once(c) => {
                self.call(op.name(), &[self.arg(&c.c)])
            }
            NodeOp::Rearm(r) => self.call("rearm", &[self.arg(&r.trigger), self.arg(&r.reset)]),
            NodeOp::Cooldown(c) => self.call("cooldown", &[self.arg(&c.trigger), int_arg(&c.bars)]),
            NodeOp::Line(l) => self.call(
                "line",
                &[
                    int_arg(&l.a.t),
                    num(&l.a.price),
                    int_arg(&l.b.t),
                    num(&l.b.price),
                ],
            ),
            NodeOp::Bucket(b) => {
                let edges: Vec<String> = b.edges.iter().map(num).collect();
                let list = format!("[{}]", edges.join(", "));
                self.call("bucket", &[self.arg(&b.x), list])
            }
            // Every other operation is rendered by `render`.
            _ => (op.name().to_owned(), ATOM),
        }
    }
}

fn wrap(s: String, prec: u8, min: u8) -> String {
    if prec < min {
        format!("({s})")
    } else {
        s
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
