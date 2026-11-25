//! Statements to IR parts: parameters, bindings, outputs, the refusal.

use qp_schema::protocol::Span;
use serde_json::{json, Map, Value};

use super::args::{float_arg, int_arg, num_value};
use super::state::{Def, Lower};
use crate::ast::{Located, OutKind, ParamLit, ParamStmt, SignalClauses, Sp, Stmt};
use crate::diag::{Diag, Reason};
use crate::msg::Msg;
use crate::names::{did_you_mean, reserved, text_name, REJECT_CODES};

/// The text name a statement defines (parameters and bindings share one namespace).
pub(super) fn defines(s: &Stmt) -> Option<(&str, Span)> {
    match s {
        Stmt::Param(p) => Some((&p.name.v, p.name.span)),
        Stmt::Bind { name, .. } => Some((&name.v, name.span)),
        _ => None,
    }
}

/// The refusal IR when the file holds a `reject`; errors when it is not alone.
pub(super) fn reject(lw: &mut Lower, stmts: &[Located]) -> Option<Value> {
    let (code, reason, span) = stmts.iter().find_map(|s| match &s.stmt {
        Stmt::Reject { code, reason } => Some((code, reason, s.span)),
        _ => None,
    })?;
    if stmts.len() > 1 || !lw.diags.is_empty() {
        lw.diags
            .push(Diag::new(Reason::Parse, Msg::RejectAlone, span));
    }
    if !REJECT_CODES.contains(&code.v.as_str()) {
        let d = Diag::new(Reason::Parse, Msg::RejectCode(code.v.clone()), code.span)
            .expected(REJECT_CODES)
            .suggest(did_you_mean(&code.v, REJECT_CODES.iter().copied()));
        lw.diags.push(d);
    }
    lw.map.add("/reject".into(), span);
    lw.map.add("/reject/code".into(), code.span);
    Some(json!({"reject": {"code": code.v, "reason": reason}}))
}

/// Lowers one statement.
pub(super) fn statement(lw: &mut Lower, s: &Located, seen_body: &mut bool) {
    match &s.stmt {
        Stmt::Param(p) => {
            if *seen_body {
                lw.diags
                    .push(Diag::new(Reason::Parse, Msg::ParamAfterBinding, s.span));
            }
            param(lw, p, s.span);
        }
        Stmt::Bind { name, expr } => {
            *seen_body = true;
            let ok = check_name(lw, name);
            let root = ok.then_some(name.v.as_str());
            let def = match lw.expr(expr, root) {
                Some(r) => Def::Node(r),
                None => Def::Poisoned,
            };
            if ok {
                lw.names.insert(name.v.clone(), def);
            }
        }
        Stmt::Output { kind, name, expr } => {
            *seen_body = true;
            let at = format!("/outputs/{}", lw.outputs.len());
            let node = lw.expr(expr, None);
            lw.map.add(at.clone(), s.span);
            lw.map.add(format!("{at}/node"), expr.span);
            let kind = match kind {
                OutKind::Series => "series",
                OutKind::Event => "event",
            };
            lw.outputs
                .push(json!({"name": name.v, "kind": kind, "node": node.unwrap_or_default()}));
        }
        Stmt::Signal {
            name,
            expr,
            clauses,
        } => {
            *seen_body = true;
            signal(lw, name, expr, clauses, s.span);
        }
        Stmt::State { name, dims } => {
            *seen_body = true;
            state(lw, name, dims, s.span);
        }
        Stmt::Reject { .. } => {}
    }
}

/// Checks a new text name: valid, not reserved, not yet defined. Records an error and
/// returns false otherwise.
fn check_name(lw: &mut Lower, name: &Sp<String>) -> bool {
    let msg = if reserved(&name.v) {
        Msg::Reserved(name.v.clone())
    } else if !text_name(&name.v) {
        Msg::BadName(name.v.clone())
    } else if lw.names.get(&name.v).is_some_and(|d| *d != Def::Poisoned) {
        lw.diags.push(Diag::new(
            Reason::DuplicateName,
            Msg::Duplicate(name.v.clone()),
            name.span,
        ));
        return false;
    } else {
        return true;
    };
    lw.diags.push(Diag::new(Reason::Parse, msg, name.span));
    // Known from here on, so that its uses do not add "unknown name" noise.
    lw.names.entry(name.v.clone()).or_insert(Def::Poisoned);
    false
}

/// A parameter literal as JSON for a kind: `float` writes numbers as floats; `int`
/// writes an integer when the literal has no fraction (else a float, which the checker
/// answers with `E_TYPE`).
fn lit(kind: &str, l: &ParamLit) -> Value {
    match l {
        ParamLit::Bool(b) => Value::Bool(*b),
        ParamLit::Num { text, neg } => {
            if kind != "float" && !text.contains('.') {
                if let Ok(i) = text.parse::<i64>() {
                    return Value::from(if *neg { -i } else { i });
                }
            }
            let f = text.parse::<f64>().unwrap_or(f64::NAN);
            num_value(if *neg { -f } else { f })
        }
    }
}

fn param(lw: &mut Lower, p: &ParamStmt, span: Span) {
    let ok = check_name(lw, &p.name);
    let kind = p.kind.v.as_str();
    if !matches!(kind, "int" | "float" | "bool") {
        let d = Diag::new(Reason::Type, Msg::ParamKind(kind.to_owned()), p.kind.span)
            .expected(&["int", "float", "bool"]);
        lw.diags.push(d);
    }
    let i = lw.params.len();
    let at = format!("/params/{i}");
    let mut m = Map::new();
    m.insert("key".into(), Value::String(p.name.v.clone()));
    m.insert("kind".into(), Value::String(kind.to_owned()));
    if let Some(u) = &p.unit {
        m.insert("unit".into(), Value::String(u.clone()));
    }
    if let Some((lo, hi)) = &p.range {
        m.insert("min".into(), lit(kind, &lo.v));
        m.insert("max".into(), lit(kind, &hi.v));
        lw.map.add(format!("{at}/min"), lo.span);
        lw.map.add(format!("{at}/max"), hi.span);
    }
    m.insert("default".into(), lit(kind, &p.default.v));
    if let Some(d) = &p.description {
        m.insert("description".into(), Value::String(d.clone()));
    }
    lw.map.add(at.clone(), span);
    lw.map.add(format!("{at}/key"), p.name.span);
    lw.map.add(format!("{at}/kind"), p.kind.span);
    lw.map.add(format!("{at}/default"), p.default.span);
    lw.params.push(Value::Object(m));
    if ok {
        lw.names.insert(p.name.v.clone(), Def::Param);
    }
}

fn signal(
    lw: &mut Lower,
    name: &Sp<String>,
    expr: &crate::ast::Expr,
    cl: &SignalClauses,
    span: Span,
) {
    let at = format!("/outputs/{}", lw.outputs.len());
    let node = lw.expr(expr, None);
    lw.map.add(at.clone(), span);
    lw.map.add(format!("{at}/node"), expr.span);
    let mut m = Map::new();
    m.insert("name".into(), Value::String(name.v.clone()));
    m.insert("kind".into(), Value::String("signal".into()));
    m.insert("node".into(), Value::String(node.unwrap_or_default()));
    if let Some(side) = &cl.side {
        m.insert("side".into(), Value::String(side.v.clone()));
        lw.map.add(format!("{at}/side"), side.span);
    }
    let mut rule = Map::new();
    for (key, e) in [("target_pct", &cl.target), ("stop_pct", &cl.stop)] {
        if let Some(e) = e {
            let ctx = format!("`{}`", key.trim_end_matches("_pct"));
            if let Some(v) = float_arg(lw, e, &ctx) {
                rule.insert(key.into(), v);
            }
            lw.map.add(format!("{at}/rule/{key}"), e.span);
        }
    }
    if let Some(e) = &cl.max_bars {
        if let Some(v) = int_arg(lw, e, "`max_bars`") {
            rule.insert("max_bars".into(), v);
        }
        lw.map.add(format!("{at}/rule/max_bars"), e.span);
    }
    if !rule.is_empty() {
        m.insert("rule".into(), Value::Object(rule));
    }
    if let Some(src) = &cl.source {
        let (s, v) = &src.v;
        m.insert("source".into(), json!({"strategy": s, "version": v}));
        lw.map.add(format!("{at}/source"), src.span);
    }
    lw.outputs.push(Value::Object(m));
}

fn state(lw: &mut Lower, name: &Sp<String>, dims: &[(Sp<String>, crate::ast::Expr)], span: Span) {
    let at = format!("/outputs/{}", lw.outputs.len());
    let mut list = Vec::new();
    for (j, (dim, e)) in dims.iter().enumerate() {
        let node = lw.expr(e, None);
        lw.map.add(format!("{at}/dims/{j}"), dim.span);
        lw.map.add(format!("{at}/dims/{j}/node"), e.span);
        list.push(json!({"name": dim.v, "node": node.unwrap_or_default()}));
    }
    lw.map.add(at, span);
    lw.outputs
        .push(json!({"name": name.v, "kind": "state", "dims": list}));
}
