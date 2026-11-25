//! Program parameters (PR §2.1): definitions (`/params/i`) and the values bound to them
//! (the message's `params`, outside the program: their findings carry no pointer and
//! name the key).

use qp_schema::program::{ProgramParamDef, ProgramParamKind, ProgramParamValue};
use serde_json::Value;

use super::fields::Fields;
use super::{Cx, ParamInfo};
use crate::diag::join_i;
use crate::message::{Msg, Space};
use crate::names::{did_you_mean, is_lower_name};
use crate::reason::Reason;

/// A value as a number (int or float), for range checks.
#[allow(clippy::cast_precision_loss)]
pub(super) const fn as_num(v: ProgramParamValue) -> Option<f64> {
    match v {
        ProgramParamValue::Int(i) => Some(i as f64),
        ProgramParamValue::Float(f) => Some(f),
        ProgramParamValue::Bool(_) => None,
    }
}

/// `int`, `float` or `bool`.
const fn kind_name(k: ProgramParamKind) -> &'static str {
    match k {
        ProgramParamKind::Int => "int",
        ProgramParamKind::Float => "float",
        ProgramParamKind::Bool => "bool",
    }
}

/// Reads `v` as a value of `kind` (PR §2.1 "değer yazımı"): `int` a JSON integer (`20.0`
/// is `E_TYPE`), `float` any number (normalized to a float), `bool` true/false.
fn read_value(
    cx: &mut Cx<'_>,
    kind: ProgramParamKind,
    v: &Value,
    at: Option<&str>,
    name: &str,
) -> Option<ProgramParamValue> {
    match (kind, v) {
        (ProgramParamKind::Int, Value::Number(n)) if n.is_i64() => {
            n.as_i64().map(ProgramParamValue::Int)
        }
        (ProgramParamKind::Int, Value::Number(n)) if n.is_u64() => {
            let m = Msg::OutOfRange(name.to_owned(), n.to_string(), "i64".to_owned());
            cx.d.push(Reason::BadParam, at, &m);
            None
        }
        (ProgramParamKind::Float, Value::Number(n)) => n.as_f64().map(ProgramParamValue::Float),
        (ProgramParamKind::Bool, Value::Bool(b)) => Some(ProgramParamValue::Bool(*b)),
        (k, _) => {
            let m = match k {
                ProgramParamKind::Int => Msg::IntExpected(name.to_owned()),
                ProgramParamKind::Float => Msg::NumberExpected(name.to_owned()),
                ProgramParamKind::Bool => Msg::BoolExpected(name.to_owned()),
            };
            cx.d.push(Reason::Type, at, &m).expected([kind_name(k)]);
            None
        }
    }
}

fn range_text(r: (f64, f64)) -> String {
    format!("{}..={}", r.0, r.1)
}

/// Reads every definition; returns the ones without errors (normalized).
pub(super) fn definitions(cx: &mut Cx<'_>, raw: Option<&Value>) -> Vec<ProgramParamDef> {
    let mut defs = Vec::new();
    let Some(raw) = raw else {
        return defs;
    };
    let Value::Array(items) = raw else {
        cx.d.at(Reason::Type, "/params", &Msg::NotArray("params".to_owned()));
        return defs;
    };
    for (i, v) in items.iter().enumerate() {
        let at = join_i("/params", i);
        let before = cx.d.count();
        let info = definition(cx, v, &at);
        let ok = cx.d.count() == before;
        if let Some((key, mut info)) = info {
            if !ok {
                info.kind = None;
            }
            if ok {
                if let Ok(d) = serde_json::from_value::<ProgramParamDef>(v.clone()) {
                    defs.push(d.normalized());
                }
            }
            cx.params.entry(key).or_insert(info);
        }
    }
    defs
}

/// One definition: its key and what nodes see of it (`None` without a usable key).
fn definition(cx: &mut Cx<'_>, v: &Value, at: &str) -> Option<(String, ParamInfo)> {
    let mut f = Fields::new(v, at, "params[]", &mut cx.d)?;
    let key = f.req_str("key", &mut cx.d).map(str::to_owned);
    if let Some(k) = &key {
        if !is_lower_name(k) {
            cx.d.at(Reason::BadParam, &f.ptr("key"), &Msg::BadName(k.clone()))
                .expected(["^[a-z][a-z0-9_]{0,31}$"]);
        } else if cx.params.contains_key(k) {
            cx.d.at(
                Reason::DuplicateName,
                &f.ptr("key"),
                &Msg::Duplicate(Space::Param, k.clone()),
            );
        }
    }
    let kind = match f.req_str("kind", &mut cx.d) {
        Some("int") => Some(ProgramParamKind::Int),
        Some("float") => Some(ProgramParamKind::Float),
        Some("bool") => Some(ProgramParamKind::Bool),
        Some(other) => {
            let m = Msg::UnknownField(other.to_owned());
            cx.d.at(Reason::Type, &f.ptr("kind"), &m)
                .expected(["int", "float", "bool"]);
            None
        }
        None => None,
    };
    f.opt_str("unit", &mut cx.d);
    f.opt_str("description", &mut cx.d);
    let (min, max, default) = (f.opt("min"), f.opt("max"), f.req("default", &mut cx.d));
    let name = key.clone().unwrap_or_default();
    let info = kind.map(|k| bounds(cx, &f, k, (min, max, default), &name));
    f.finish(&mut cx.d, &[]);
    Some((key?, info?))
}

/// Range and default of a definition of `kind`.
fn bounds(
    cx: &mut Cx<'_>,
    f: &Fields<'_>,
    kind: ProgramParamKind,
    (min, max, default): (Option<&Value>, Option<&Value>, Option<&Value>),
    key: &str,
) -> ParamInfo {
    let read = |cx: &mut Cx<'_>, field: &'static str, v: Option<&Value>| {
        v.and_then(|v| read_value(cx, kind, v, Some(&f.ptr(field)), field))
    };
    let default = read(cx, "default", default);
    let range = if kind == ProgramParamKind::Bool {
        for (field, v) in [("min", min), ("max", max)] {
            if v.is_some() {
                cx.d.at(
                    Reason::BadParam,
                    &f.ptr(field),
                    &Msg::ParamBoolRange(key.to_owned()),
                );
            }
        }
        None
    } else {
        for (field, v) in [("min", min), ("max", max)] {
            if v.is_none() {
                cx.d.at(Reason::MissingArg, &f.at, &Msg::Missing(field.to_owned()))
                    .expected([field]);
            }
        }
        let lo = read(cx, "min", min).and_then(as_num);
        let hi = read(cx, "max", max).and_then(as_num);
        lo.zip(hi)
    };
    if let Some((lo, hi)) = range {
        if lo > hi {
            cx.d.at(
                Reason::ParamRange,
                &f.ptr("min"),
                &Msg::ParamMinMax(key.to_owned()),
            );
        } else if let Some(v) = default.and_then(as_num) {
            if v < lo || v > hi {
                let m = Msg::ParamValueRange(key.to_owned(), v.to_string(), range_text((lo, hi)));
                cx.d.at(Reason::ParamRange, &f.ptr("default"), &m)
                    .expected([range_text((lo, hi))]);
            }
        }
    }
    ParamInfo {
        kind: Some(kind),
        range,
        value: default.map(|v| norm(kind, v)),
    }
}

/// `v` normalized to `kind`: a `float` parameter's integer-written value as a float.
fn norm(kind: ProgramParamKind, v: ProgramParamValue) -> ProgramParamValue {
    match kind {
        ProgramParamKind::Float => v.as_float(),
        _ => v,
    }
}

/// Binds the message's values (`ctx.params`) over the defaults.
pub(super) fn bind(cx: &mut Cx<'_>) {
    let Some(raw) = cx.ctx.params else {
        return;
    };
    let Value::Object(map) = raw else {
        cx.d.push(Reason::Type, None, &Msg::NotObject("params".to_owned()));
        return;
    };
    for (key, v) in map {
        let Some(info) = cx.params.get(key).cloned() else {
            let dym = did_you_mean(key, cx.params.keys().map(String::as_str));
            cx.d.push(Reason::UnknownParam, None, &Msg::UnknownParam(key.clone()))
                .expected(cx.params.keys().cloned().collect::<Vec<_>>())
                .dym(dym);
            continue;
        };
        let Some(kind) = info.kind else { continue };
        let mut value = read_value(cx, kind, v, None, key);
        if let (Some(x), Some(r)) = (value.and_then(as_num), info.range) {
            if x < r.0 || x > r.1 {
                let m = Msg::ParamValueRange(key.clone(), x.to_string(), range_text(r));
                cx.d.push(Reason::ParamRange, None, &m)
                    .expected([range_text(r)]);
                value = None;
            }
        }
        if let Some(p) = cx.params.get_mut(key) {
            p.value = value.map(|v| norm(kind, v));
        }
    }
}
