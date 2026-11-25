//! Shared helpers of the `qp-lang` checker tests.
#![allow(dead_code)]

use qp_lang::{check, Binding, CheckCtx, CheckReport, Limits};
use serde_json::{json, Value};

/// Checks `v` with default limits, unbound.
pub fn chk(v: &Value) -> CheckReport {
    check(v, &CheckCtx::default())
}

/// Checks `v` bound to a series of `interval`.
pub fn chk_bound(v: &Value, interval: &str) -> CheckReport {
    let ctx = CheckCtx {
        binding: Some(Binding {
            interval: Some(interval.to_owned()),
            bars: Some(3000),
        }),
        ..CheckCtx::default()
    };
    check(v, &ctx)
}

/// Checks `v` with `limits`.
pub fn chk_limits(v: &Value, limits: Limits) -> CheckReport {
    check(
        v,
        &CheckCtx {
            limits,
            ..CheckCtx::default()
        },
    )
}

/// Checks `v` with bound values `params`.
pub fn chk_params(v: &Value, params: &Value) -> CheckReport {
    check(
        v,
        &CheckCtx {
            params: Some(params),
            ..CheckCtx::default()
        },
    )
}

/// `(reason, at)` of every error.
pub fn errs(r: &CheckReport) -> Vec<(String, String)> {
    r.errors
        .iter()
        .map(|e| (e.reason.clone(), e.at.clone().unwrap_or_default()))
        .collect()
}

/// Asserts the report has exactly one error, of `reason` at `at`, and returns it.
pub fn only(r: &CheckReport, reason: &str, at: &str) -> qp_schema::protocol::ProgramError {
    assert_eq!(
        errs(r),
        vec![(reason.to_owned(), at.to_owned())],
        "{:#?}",
        r.errors
    );
    r.errors[0].clone()
}

/// A program: `nodes` and `outputs` as given, version `qp-ir/0`.
pub fn prog(nodes: Value, outputs: Value) -> Value {
    json!({"version": "qp-ir/0", "nodes": nodes, "outputs": outputs})
}

/// A program with parameters.
pub fn prog_p(params: Value, nodes: Value, outputs: Value) -> Value {
    json!({"version": "qp-ir/0", "params": params, "nodes": nodes, "outputs": outputs})
}

/// `close` → `series c`.
pub fn close_series() -> Value {
    prog(
        json!([{"id": "c", "op": "input", "field": "close"}]),
        json!([{"name": "c", "kind": "series", "node": "c"}]),
    )
}

/// `series s = sma(close, <period>)`.
pub fn sma(period: Value) -> Value {
    prog(
        json!([{"id": "c", "op": "input", "field": "close"},
               {"id": "s", "op": "sma", "x": "c", "period": period}]),
        json!([{"name": "s", "kind": "series", "node": "s"}]),
    )
}

/// An int parameter definition.
pub fn int_param(key: &str, min: i64, max: i64, default: i64) -> Value {
    json!({"key": key, "kind": "int", "min": min, "max": max, "default": default})
}
