//! Names the printer writes verbatim are checked first: an IR that came from anywhere
//! (JSON, a log, another tool) may carry an output name like `x = 1\nseries y`, which
//! would inject statements. Such a program is refused with the JSON pointer of the bad
//! value, never renamed: output and parameter names are part of the program's meaning
//! (series ids, bindings). Node ids need no check: unprintable ones get `v<k>`.

use std::collections::BTreeSet;
use std::fmt;

use qp_schema::program::{ConstValue, IntArg, NodeOp, Num, OutputKind, Program, ProgramReject};

use crate::names::text_name;

/// A value the canonical text cannot spell safely.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrintError {
    /// JSON pointer of the value (relative to the program root).
    pub at: String,
    /// The value.
    pub value: String,
    /// What it must look like.
    pub expected: &'static str,
}

impl fmt::Display for PrintError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {:?} is not {}", self.at, self.value, self.expected)
    }
}

impl std::error::Error for PrintError {}

const IDENT: &str = "an identifier ([A-Za-z_][A-Za-z0-9_]{0,63})";
const NAME: &str = "a text name ([A-Za-z][A-Za-z0-9_]{0,63}, not reserved)";
const TF: &str = "a timeframe (digits then letters, e.g. 4h)";

/// `[A-Za-z_][A-Za-z0-9_]{0,63}`: what the lexer reads as one plain name.
fn ident(s: &str) -> bool {
    let mut cs = s.chars();
    cs.next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && cs.all(|c| c.is_ascii_alphanumeric() || c == '_')
        && s.len() <= 64
}

/// Digits then letters or digits, at most 16 characters (`4h`, `15m`, `1M`).
fn interval(s: &str) -> bool {
    let digits = s.chars().take_while(char::is_ascii_digit).count();
    let rest = &s[digits..];
    digits > 0
        && rest.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
        && rest.chars().all(|c| c.is_ascii_alphanumeric())
        && s.len() <= 16
}

fn need(ok: bool, at: String, value: &str, expected: &'static str) -> Result<(), PrintError> {
    if ok {
        Ok(())
    } else {
        Err(PrintError {
            at,
            value: value.to_owned(),
            expected,
        })
    }
}

fn param(at: String, key: &str) -> Result<(), PrintError> {
    need(text_name(key), at, key, NAME)
}

fn int(at: String, a: &IntArg) -> Result<(), PrintError> {
    match a {
        IntArg::Param(p) => param(at, &p.param),
        IntArg::Lit(_) => Ok(()),
    }
}

fn num(at: String, n: &Num) -> Result<(), PrintError> {
    match n {
        Num::Param(p) => param(at, &p.param),
        Num::Lit(_) => Ok(()),
    }
}

/// A reference: a node id (printed by its text name) or a plain text name, then an
/// optional `.output` identifier.
fn reference(ids: &BTreeSet<&str>, at: String, r: &str) -> Result<(), PrintError> {
    let (b, out) = match r.split_once('.') {
        Some((b, o)) => (b, Some(o)),
        None => (r, None),
    };
    let known = ids.contains(b);
    need(known || text_name(b), at.clone(), r, NAME)?;
    match out {
        Some(o) => need(o.split('.').all(ident), at, r, IDENT),
        None => Ok(()),
    }
}

/// Checks every verbatim-printed name of a program.
pub(super) fn check_program(p: &Program) -> Result<(), PrintError> {
    let ids: BTreeSet<&str> = p.nodes.iter().map(|n| n.id.as_str()).collect();
    for (i, d) in p.params.iter().enumerate() {
        param(format!("/params/{i}/key"), &d.key)?;
    }
    for (i, n) in p.nodes.iter().enumerate() {
        let at = format!("/nodes/{i}");
        for r in n.op.refs() {
            reference(&ids, at.clone(), r)?;
        }
        node_fields(&at, &n.op)?;
    }
    for (i, o) in p.outputs.iter().enumerate() {
        let at = format!("/outputs/{i}");
        need(ident(&o.name), format!("{at}/name"), &o.name, IDENT)?;
        match &o.kind {
            OutputKind::Series { node } | OutputKind::Event { node } => {
                reference(&ids, format!("{at}/node"), node)?;
            }
            OutputKind::Signal { node, rule, .. } => {
                reference(&ids, format!("{at}/node"), node)?;
                if let Some(t) = &rule.target_pct {
                    num(format!("{at}/rule/target_pct"), t)?;
                }
                if let Some(t) = &rule.stop_pct {
                    num(format!("{at}/rule/stop_pct"), t)?;
                }
                int(format!("{at}/rule/max_bars"), &rule.max_bars)?;
            }
            OutputKind::State { dims } => {
                for (j, d) in dims.iter().enumerate() {
                    need(
                        ident(&d.name),
                        format!("{at}/dims/{j}/name"),
                        &d.name,
                        IDENT,
                    )?;
                    reference(&ids, format!("{at}/dims/{j}/node"), &d.node)?;
                }
            }
        }
    }
    Ok(())
}

fn node_fields(at: &str, op: &NodeOp) -> Result<(), PrintError> {
    let tf = |t: &Option<String>| match t {
        Some(t) => need(interval(t), format!("{at}/tf"), t, TF),
        None => Ok(()),
    };
    match op {
        NodeOp::Input(i) => tf(&i.tf),
        NodeOp::Atr(a) => {
            tf(&a.tf)?;
            int(format!("{at}/period"), &a.period)
        }
        NodeOp::Const(c) => match &c.value {
            ConstValue::Param(p) => param(format!("{at}/value"), &p.param),
            _ => Ok(()),
        },
        NodeOp::Sma(w)
        | NodeOp::Ema(w)
        | NodeOp::Rsi(w)
        | NodeOp::Highest(w)
        | NodeOp::Lowest(w)
        | NodeOp::Stdev(w)
        | NodeOp::Percentrank(w) => int(format!("{at}/period"), &w.period),
        NodeOp::Macd(m) => {
            int(format!("{at}/fast"), &m.fast)?;
            int(format!("{at}/slow"), &m.slow)?;
            int(format!("{at}/signal"), &m.signal)
        }
        NodeOp::Bollinger(b) => {
            int(format!("{at}/period"), &b.period)?;
            num(format!("{at}/k"), &b.k)
        }
        NodeOp::Lag(l) => int(format!("{at}/n"), &l.n),
        NodeOp::Cooldown(c) => int(format!("{at}/bars"), &c.bars),
        NodeOp::Line(l) => {
            int(format!("{at}/a/t"), &l.a.t)?;
            num(format!("{at}/a/price"), &l.a.price)?;
            int(format!("{at}/b/t"), &l.b.t)?;
            num(format!("{at}/b/price"), &l.b.price)
        }
        NodeOp::Bucket(b) => b
            .edges
            .iter()
            .enumerate()
            .try_for_each(|(j, e)| num(format!("{at}/edges/{j}"), e)),
        _ => Ok(()),
    }
}

/// Checks a refusal's code.
pub(super) fn check_reject(r: &ProgramReject) -> Result<(), PrintError> {
    need(ident(&r.code), "/reject/code".into(), &r.code, IDENT)
}
