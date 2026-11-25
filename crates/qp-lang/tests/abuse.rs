//! Hostile programs (RL1 GOAL §7 "kötü niyet"; security review of LA): reused operands,
//! oversized input, long names, deep chains and error floods stay cheap and bounded.

mod support;

use std::time::{Duration, Instant};

use qp_lang::{did_you_mean, Limit, Limits, OP_NAMES};
use serde_json::{json, Value};
use support::*;

/// Generous limits so that only the property under test can stop the check.
fn wide() -> Limits {
    Limits::default()
        .with(Limit::MaxNodes, 20_000)
        .with(Limit::MaxIrBytes, 50_000_000)
}

fn quick(t: Instant) {
    assert!(t.elapsed() < Duration::from_secs(2), "{:?}", t.elapsed());
}

#[test]
fn reused_and_chain_is_linear() {
    // `y_k = and(y_{k-1}, y_{k-1})`, 30 levels: a recursive walk would take 2^30 steps.
    let mut nodes = vec![
        json!({"id": "c", "op": "input", "field": "close"}),
        json!({"id": "k", "op": "const", "value": 1}),
        json!({"id": "y0", "op": "gt", "a": "c", "b": "k"}),
    ];
    for i in 1..=30 {
        let prev = format!("y{}", i - 1);
        nodes.push(json!({"id": format!("y{i}"), "op": "and", "a": prev, "b": prev}));
    }
    let p = prog(
        Value::Array(nodes),
        json!([{"name": "e", "kind": "event", "node": "y30"}]),
    );
    let t = Instant::now();
    let r = chk(&p);
    quick(t);
    assert!(r.ok(), "{:#?}", r.errors);
    assert!(r
        .warnings
        .iter()
        .any(|w| w.reason == "W_CONSECUTIVE_EVENTS"));
}

fn chain(n: usize, op: &str) -> Value {
    let mut nodes = vec![json!({"id": "n0", "op": "input", "field": "close"})];
    for i in 1..n {
        nodes.push(json!({"id": format!("n{i}"), "op": op, "x": format!("n{}", i - 1)}));
    }
    prog(
        Value::Array(nodes),
        json!([{"name": "s", "kind": "series", "node": format!("n{}", n - 1)}]),
    )
}

#[test]
fn oversized_input_is_refused_before_node_work() {
    let t = Instant::now();
    let r = chk(&chain(10_000, "neg"));
    quick(t);
    assert_eq!(errs(&r), vec![("E_TOO_LARGE".to_owned(), String::new())]);
    // Over the node limit (bytes allowed): the limit error alone, no per-node errors.
    let r = chk_limits(
        &chain(10_000, "nope"),
        Limits::default().with(Limit::MaxIrBytes, 50_000_000),
    );
    assert_eq!(
        errs(&r),
        vec![("E_TOO_MANY_NODES".to_owned(), "/nodes".to_owned())]
    );
    // Over the output limit: likewise.
    let mut p = close_series();
    p["outputs"] = Value::Array(
        (0..100)
            .map(|i| json!({"name": format!("o{i}"), "kind": "bad"}))
            .collect(),
    );
    let r = chk_limits(&p, wide());
    assert_eq!(
        errs(&r),
        vec![("E_TOO_LARGE".to_owned(), "/outputs".to_owned())]
    );
}

#[test]
fn ten_thousand_node_chain_has_no_deep_recursion() {
    let l = wide().with(Limit::MaxCostPerBar, 1_000_000);
    let t = Instant::now();
    let r = chk_limits(&chain(10_000, "neg"), l);
    quick(t);
    assert!(r.ok(), "{:?}", r.errors.first());
    assert_eq!(r.checked.map(|c| c.nodes.len()), Some(10_000));
    // At the limit and one above.
    let at = l.with(Limit::MaxNodes, 10_000);
    assert!(chk_limits(&chain(10_000, "neg"), at).ok());
    let r = chk_limits(&chain(10_001, "neg"), at);
    assert_eq!(
        errs(&r),
        vec![("E_TOO_MANY_NODES".to_owned(), "/nodes".to_owned())]
    );
}

#[test]
fn error_flood_is_capped() {
    let l = wide().with(Limit::CheckMaxErrors, 50);
    let t = Instant::now();
    let r = chk_limits(&chain(5_000, "nope"), l);
    quick(t);
    assert_eq!(r.errors.len(), 50);
    assert!(r.truncated);
}

#[test]
fn megabyte_names_do_not_blow_up_suggestions() {
    let long = "a".repeat(1 << 20);
    let t = Instant::now();
    assert!(did_you_mean(&long, OP_NAMES).is_empty());
    let p = prog(
        json!([{"id": long, "op": long, "field": "close"}]),
        json!([{"name": "s", "kind": "series", "node": "b".repeat(1 << 20)}]),
    );
    let r = chk_limits(&p, wide());
    quick(t);
    assert!(!r.ok());
    assert!(r
        .errors
        .iter()
        .all(|e| e.message.chars().count() <= 513 && e.did_you_mean.is_empty()));
}
