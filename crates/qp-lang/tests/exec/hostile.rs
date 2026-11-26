//! Hostile programs within the default limits stay cheap (iterative evaluation, bounded
//! state): the largest windows, the longest chains, `try`'s bar limit.

use std::time::{Duration, Instant};

use qp_lang::{check, Binding, CheckCtx, Limits};
use serde_json::{json, Value};

use crate::support::*;

fn checked_default(p: &Value) -> qp_lang::Checked {
    let ctx = CheckCtx {
        limits: Limits::default(),
        binding: Some(Binding {
            interval: Some("1h".to_owned()),
            bars: None,
        }),
        ..CheckCtx::default()
    };
    let r = check(p, &ctx);
    assert!(r.ok(), "{:#?}", r.errors);
    r.checked.unwrap()
}

fn quick(t: Instant) {
    let lim = Duration::from_secs(if cfg!(debug_assertions) { 20 } else { 2 });
    assert!(t.elapsed() < lim, "{:?}", t.elapsed());
}

#[test]
fn longest_lag_chain_at_the_window_limit() {
    // 150 lags of 2000 bars each (the IR size limit, 8192 bytes, binds first): 300k
    // remembered values.
    let mut nodes = vec![json!({"id": "n0", "op": "input", "field": "close"})];
    for i in 1..=150 {
        nodes.push(
            json!({"id": format!("n{i}"), "op": "lag", "x": format!("n{}", i - 1), "n": 2000}),
        );
    }
    let p = prog(
        Value::Array(nodes),
        json!([{"name": "s", "kind": "series", "node": "n150"}]),
    );
    let c = checked_default(&p);
    let t = Instant::now();
    let x = run(&c, &bars(20_000, 1));
    quick(t);
    assert_eq!(
        x.value_at(0, 19_999),
        None,
        "150 × 2000 bars back is before the data"
    );
}

#[test]
fn widest_costly_window() {
    // stdev over 660 bars: the per-bar cost limit (666) bounds the O(period) kernel.
    let p = prog(
        json!([{"id": "c", "op": "input", "field": "close"},
               {"id": "s", "op": "stdev", "x": "c", "period": 660}]),
        json!([{"name": "s", "kind": "series", "node": "s"}]),
    );
    let c = checked_default(&p);
    let t = Instant::now();
    let x = run(&c, &bars(20_000, 2));
    quick(t);
    assert!(x.value_at(0, 19_999).is_some());
}
