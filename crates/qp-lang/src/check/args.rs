//! Node fields (PR §4): references, integer fields, float fields and constants, each a
//! literal or a parameter (`{"param": key}`), checked against the field's range — the
//! static half of the two-level range check (PR §2.1) — and resolved to bound values.

use serde_json::Value;

use super::fields::Fields;
use super::params::as_num;
use super::{Cx, ParamInfo};
use crate::checked::{Literal, Slot};
use crate::diag::join;
use crate::limits::Limit;
use crate::message::Msg;
use crate::names::did_you_mean;
use crate::reason::Reason;
use qp_schema::program::{ProgramParamKind, ProgramParamValue};

/// Largest line anchor time (2^53 ms, PR §4.6).
const MAX_T: i64 = 1 << 53;

/// What an integer field means, which sets its range and codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum IntRole {
    /// `period`, `fast`, `slow`, `signal`: `1 … max_window`.
    Window,
    /// `lag.n`: `0 … max_window`; negative is `E_LOOKAHEAD`.
    Lag,
    /// `cooldown.bars`: `0 … max_window`.
    Bars,
    /// `rule.max_bars`: `1 … max_horizon_bars` (`E_BAD_RULE`).
    MaxBars,
    /// A line anchor's `t`: `0 … 2^53`.
    Time,
}

/// What a float field means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NumRole {
    /// `k`, a line anchor's `price`: `> 0` (`E_BAD_PARAM`).
    Positive,
    /// `target_pct`, `stop_pct`: `> 0` (`E_BAD_RULE`).
    Pct,
    /// A bucket edge: any number.
    Edge,
}

impl IntRole {
    #[allow(clippy::cast_possible_wrap)]
    fn range(self, cx: &Cx<'_>) -> (i64, i64) {
        let w = cx.limit(Limit::MaxWindow) as i64;
        match self {
            IntRole::Window => (1, w),
            IntRole::Lag | IntRole::Bars => (0, w),
            IntRole::MaxBars => (1, cx.limit(Limit::SignalMaxHorizonBars) as i64),
            IntRole::Time => (0, MAX_T),
        }
    }

    const fn bad(self) -> Reason {
        match self {
            IntRole::MaxBars => Reason::BadRule,
            _ => Reason::BadParam,
        }
    }
}

fn range_text<T: std::fmt::Display>(lo: T, hi: T) -> String {
    format!("{lo}..={hi}")
}

/// A required integer field.
pub(super) fn int_arg(
    cx: &mut Cx<'_>,
    f: &mut Fields<'_>,
    name: &'static str,
    role: IntRole,
) -> Option<i64> {
    let at = f.ptr(name);
    let v = if role == IntRole::MaxBars {
        let v = f.opt(name);
        if v.is_none() {
            cx.d.at(Reason::NoOutcomeRule, &f.at, &Msg::NoRule)
                .expected([name]);
        }
        v?
    } else {
        f.req(name, &mut cx.d)?
    };
    match v {
        Value::Number(n) => match n.as_i64() {
            Some(i) => literal_int(cx, &at, name, i, role),
            None => {
                cx.d.at(role.bad(), &at, &Msg::IntExpected(name.to_owned()));
                None
            }
        },
        Value::Object(_) => param_int(cx, v, &at, name, role),
        _ => {
            cx.d.at(Reason::Type, &at, &Msg::IntExpected(name.to_owned()))
                .expected(["integer", "{\"param\": key}"]);
            None
        }
    }
}

fn literal_int(cx: &mut Cx<'_>, at: &str, name: &str, i: i64, role: IntRole) -> Option<i64> {
    let (lo, hi) = role.range(cx);
    if i < lo {
        if role == IntRole::Lag {
            cx.d.at(Reason::Lookahead, at, &Msg::NegativeLag(i))
                .hint(&Msg::HintLookahead);
        } else {
            let m = Msg::OutOfRange(name.to_owned(), i.to_string(), range_text(lo, hi));
            cx.d.at(role.bad(), at, &m).expected([range_text(lo, hi)]);
        }
        return None;
    }
    if i > hi {
        if matches!(role, IntRole::Window | IntRole::Lag | IntRole::Bars) {
            let lim = cx.limit(Limit::MaxWindow);
            cx.d.at(
                Reason::WindowLimit,
                at,
                &Msg::WindowLimit(name.to_owned(), i, lim),
            )
            .expected([lim]);
        } else {
            let m = Msg::OutOfRange(name.to_owned(), i.to_string(), range_text(lo, hi));
            cx.d.at(role.bad(), at, &m).expected([range_text(lo, hi)]);
        }
        return None;
    }
    Some(i)
}

/// Reads `{"param": key}` at `at`; `None` after an error or for a broken definition.
fn param_ref(cx: &mut Cx<'_>, v: &Value, at: &str) -> Option<(String, ParamInfo)> {
    let mut f = Fields::new(v, at, "{\"param\": key}", &mut cx.d)?;
    let key = f.req_str("param", &mut cx.d).map(str::to_owned);
    f.finish(&mut cx.d, &[]);
    let key = key?;
    let Some(info) = cx.params.get(&key).cloned() else {
        let dym = did_you_mean(&key, cx.params.keys().map(String::as_str));
        cx.d.at(
            Reason::UnknownParam,
            &join(at, "param"),
            &Msg::UnknownParam(key),
        )
        .expected(cx.params.keys().cloned().collect::<Vec<_>>())
        .dym(dym);
        return None;
    };
    info.kind?;
    Some((key, info))
}

#[allow(clippy::cast_precision_loss)]
fn param_int(cx: &mut Cx<'_>, v: &Value, at: &str, name: &str, role: IntRole) -> Option<i64> {
    let (key, info) = param_ref(cx, v, at)?;
    if info.kind != Some(ProgramParamKind::Int) {
        cx.d.at(
            Reason::Type,
            at,
            &Msg::ParamKind(key, name.to_owned(), "int"),
        )
        .expected(["int"]);
        return None;
    }
    let (lo, hi) = role.range(cx);
    if let Some((min, max)) = info.range {
        if min < lo as f64 || max > hi as f64 {
            let m = Msg::ParamFieldRange(key, name.to_owned(), range_text(lo, hi));
            cx.d.at(Reason::ParamRange, at, &m)
                .expected([range_text(lo, hi)]);
            return None;
        }
    }
    match info.value {
        Some(ProgramParamValue::Int(i)) => Some(i),
        _ => None,
    }
}

/// A float field's value (`v` already present).
pub(super) fn num_value(
    cx: &mut Cx<'_>,
    v: &Value,
    at: &str,
    name: &str,
    role: NumRole,
) -> Option<f64> {
    let bad = if role == NumRole::Pct {
        Reason::BadRule
    } else {
        Reason::BadParam
    };
    match v {
        Value::Number(n) => {
            let x = n.as_f64()?;
            // JSON numbers are finite, so `<= 0` is the whole complement of `> 0`.
            if role != NumRole::Edge && x <= 0.0 {
                let m = Msg::OutOfRange(name.to_owned(), n.to_string(), "> 0".to_owned());
                cx.d.at(bad, at, &m).expected(["> 0"]);
                return None;
            }
            Some(x)
        }
        Value::Object(_) => {
            let (key, info) = param_ref(cx, v, at)?;
            if info.kind != Some(ProgramParamKind::Float) {
                cx.d.at(
                    Reason::Type,
                    at,
                    &Msg::ParamKind(key, name.to_owned(), "float"),
                )
                .expected(["float"]);
                return None;
            }
            if role != NumRole::Edge && info.range.is_some_and(|(min, _)| min <= 0.0) {
                let m = Msg::ParamFieldRange(key, name.to_owned(), "> 0".to_owned());
                cx.d.at(Reason::ParamRange, at, &m).expected(["> 0"]);
                return None;
            }
            info.value.and_then(as_num)
        }
        _ => {
            cx.d.at(Reason::Type, at, &Msg::NumberExpected(name.to_owned()))
                .expected(["number", "{\"param\": key}"]);
            None
        }
    }
}

/// A required float field.
pub(super) fn num_arg(
    cx: &mut Cx<'_>,
    f: &mut Fields<'_>,
    name: &'static str,
    role: NumRole,
) -> Option<f64> {
    let at = f.ptr(name);
    let v = f.req(name, &mut cx.d)?;
    num_value(cx, v, &at, name, role)
}

/// A `const` value: a number, a boolean or a parameter of any kind.
pub(super) fn const_value(cx: &mut Cx<'_>, v: &Value, at: &str) -> Option<Literal> {
    match v {
        Value::Bool(b) => Some(Literal::Bool(*b)),
        Value::Number(n) => n.as_f64().map(Literal::Num),
        Value::Object(_) => {
            let (_, info) = param_ref(cx, v, at)?;
            match info.value? {
                ProgramParamValue::Bool(b) => Some(Literal::Bool(b)),
                p => as_num(p).map(Literal::Num),
            }
        }
        _ => {
            cx.d.at(Reason::Type, at, &Msg::NumberExpected("value".to_owned()))
                .expected(["number", "true", "false", "{\"param\": key}"]);
            None
        }
    }
}

/// The output names of a multi-output op (main output's index second).
pub(crate) fn multi_outputs(op: Option<&str>) -> (&'static [&'static str], usize) {
    match op {
        Some("macd") => (&["macd", "signal", "histogram"], 0),
        Some("bollinger") => (&["upper", "middle", "lower", "bandwidth"], 1),
        _ => (&[], 0),
    }
}

/// Resolves a reference string at `at` read by node `here` (or an output, `here` = node
/// count): `None` after an error, or silently when it names a broken node.
pub(super) fn resolve_ref(cx: &mut Cx<'_>, s: &str, at: &str, here: usize) -> Option<Slot> {
    let (id, out) = match s.split_once('.') {
        Some((id, out)) => (id, Some(out)),
        None => (s, None),
    };
    let Some(&j) = cx.ids.get(id) else {
        let earlier: Vec<&str> = cx
            .ids
            .iter()
            .filter(|(_, &j)| j < here)
            .map(|(k, _)| k.as_str())
            .collect();
        let dym = did_you_mean(id, earlier);
        cx.d.at(Reason::UnknownRef, at, &Msg::UnknownRef(s.to_owned()))
            .dym(dym);
        return None;
    };
    if j >= here {
        cx.d.at(Reason::UnknownRef, at, &Msg::LaterRef(s.to_owned()));
        return None;
    }
    let entry = &cx.nodes[j];
    entry.op?;
    let (outs, main) = multi_outputs(entry.op);
    let index = match out {
        None => main,
        Some(o) => match outs.iter().position(|n| *n == o) {
            Some(k) => k,
            None => {
                let dym = did_you_mean(o, outs.iter().copied());
                let m = Msg::NoSuchOutput(id.to_owned(), o.to_owned());
                cx.d.at(Reason::UnknownRef, at, &m)
                    .expected(outs.iter().copied())
                    .dym(dym);
                return None;
            }
        },
    };
    entry.typed.as_ref()?;
    Some(Slot {
        node: j,
        out: index,
    })
}

/// A required reference field.
pub(super) fn ref_arg(
    cx: &mut Cx<'_>,
    f: &mut Fields<'_>,
    name: &'static str,
    here: usize,
) -> Option<Slot> {
    let at = f.ptr(name);
    match f.req(name, &mut cx.d)? {
        Value::String(s) => resolve_ref(cx, s, &at, here),
        _ => {
            cx.d.at(Reason::Type, &at, &Msg::NotString(name.to_owned()))
                .expected(["node id", "id.output"]);
            None
        }
    }
}
