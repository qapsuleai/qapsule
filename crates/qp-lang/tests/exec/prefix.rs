//! Prefix stability (PR §10, VERIFY 10): for random programs, random data and random `n`,
//! the closed-window run of the first `n` bars equals the full run's first `n` bars —
//! values, events, state keys, provenance — and a signal's outcome only differs by being
//! still `open` in the prefix. Fixed seeds; a small deterministic program generator.

use qp_lang::{Executor, SignalHit};
use qp_schema::program::OutcomeKind;
use serde_json::{json, Value};

use crate::support::*;

/// A random valid program: base-clock arithmetic, kernels, history and state nodes over
/// bar fields, sometimes a `4h` subtree brought down with `align`; four outputs.
pub fn random_program(r: &mut Rng) -> Value {
    let mut nodes: Vec<Value> = Vec::new();
    let mut nums: Vec<String> = Vec::new();
    let mut bools: Vec<String> = Vec::new();
    let mut id = 0;
    let mut fresh = |nodes: &mut Vec<Value>, mut v: Value| {
        id += 1;
        let name = format!("n{id}");
        v["id"] = json!(name);
        nodes.push(v);
        name
    };
    for f in ["close", "high", "low", "volume"] {
        let n = fresh(&mut nodes, json!({"op": "input", "field": f}));
        nums.push(n);
    }
    if r.below(2) == 0 {
        let hc = fresh(
            &mut nodes,
            json!({"op": "input", "field": "close", "tf": "4h"}),
        );
        let hk = *r.pick(&["sma", "ema", "highest", "rsi"]);
        let hs = fresh(
            &mut nodes,
            json!({"op": hk, "x": hc, "period": 1 + r.below(5)}),
        );
        let al = fresh(&mut nodes, json!({"op": "align", "x": hs}));
        nums.push(al);
    }
    for _ in 0..(4 + r.below(10)) {
        let v = random_node(r, &nums, &bools);
        let op = v["op"].as_str().unwrap_or("").to_owned();
        let name = fresh(&mut nodes, v);
        match op.as_str() {
            "gt" | "lt" | "ge" | "le" | "cross_above" | "cross_below" | "once" | "rearm"
            | "cooldown" | "and" | "or" => bools.push(name),
            "macd" => nums.push(format!("{name}.signal")),
            "bollinger" => nums.push(format!("{name}.upper")),
            _ => nums.push(name),
        }
    }
    if bools.is_empty() {
        let (a, b) = (nums[0].clone(), nums[1].clone());
        bools.push(fresh(&mut nodes, json!({"op": "gt", "a": a, "b": b})));
    }
    let s = r.pick(&nums).clone();
    let e = r.pick(&bools).clone();
    let g = r.pick(&bools).clone();
    let bx = r.pick(&nums).clone();
    let bk = fresh(
        &mut nodes,
        json!({"op": "bucket", "x": bx, "edges": [0.0, 50.0, 100.0]}),
    );
    prog(
        Value::Array(nodes),
        json!([
            {"name": "s", "kind": "series", "node": s},
            {"name": "e", "kind": "event", "node": e},
            {"name": "g", "kind": "signal", "node": g, "side": *r.pick(&["al", "sat"]),
             "rule": {"target_pct": 1.0, "stop_pct": 0.8, "max_bars": 1 + r.below(10)},
             "source": {"strategy": "gen", "version": "1"}},
            {"name": "k", "kind": "state", "dims": [{"name": "b", "node": bk}]}
        ]),
    )
}

/// One random base-clock node over the numbers and booleans so far.
fn random_node(r: &mut Rng, nums: &[String], bools: &[String]) -> Value {
    let a = r.pick(nums).clone();
    let b = r.pick(nums).clone();
    match r.below(10) {
        0 => {
            json!({"op": *r.pick(&["sma", "ema", "rsi", "highest", "lowest", "stdev", "percentrank"]),
                    "x": a, "period": 1 + r.below(12)})
        }
        1 => {
            json!({"op": *r.pick(&["add", "sub", "mul", "div", "min", "max"]), "a": a, "b": b})
        }
        2 => json!({"op": "lag", "x": a, "n": r.below(4)}),
        3 => {
            json!({"op": *r.pick(&["gt", "lt", "ge", "le", "cross_above", "cross_below"]), "a": a, "b": b})
        }
        4 if !bools.is_empty() => {
            let c = r.pick(bools).clone();
            json!({"op": *r.pick(&["streak", "bars_since"]), "c": c})
        }
        5 if !bools.is_empty() => {
            let c = r.pick(bools).clone();
            let d = r.pick(bools).clone();
            match r.below(4) {
                0 => json!({"op": "once", "c": c}),
                1 => json!({"op": "rearm", "trigger": c, "reset": d}),
                2 => json!({"op": "cooldown", "trigger": c, "bars": r.below(5)}),
                _ => json!({"op": *r.pick(&["and", "or"]), "a": c, "b": d}),
            }
        }
        6 => {
            json!({"op": "macd", "x": a, "fast": 2, "slow": 3 + r.below(6), "signal": 1 + r.below(4)})
        }
        7 => json!({"op": "bollinger", "x": a, "period": 2 + r.below(8), "k": 1.5}),
        8 => json!({"op": "atr", "period": 1 + r.below(6)}),
        _ => json!({"op": *r.pick(&["neg", "abs"]), "x": a}),
    }
}

/// The signal records of output `o` born on the first `n` closed bars.
fn hits(x: &Executor, o: usize, n: usize) -> Vec<SignalHit> {
    (0..n).filter_map(|i| x.signals_at(o, i).cloned()).collect()
}

fn same_prefix(full: &Executor, part: &Executor, n: usize, case: u64) {
    assert_eq!(part.closed_len(), n, "case {case}");
    for o in 0..full.outputs().len() {
        for i in 0..n {
            let (a, b) = (full.value_at(o, i), part.value_at(o, i));
            assert_eq!(
                a.map(f64::to_bits),
                b.map(f64::to_bits),
                "case {case} out {o} bar {i}"
            );
            assert_eq!(
                full.state_at(o, i),
                part.state_at(o, i),
                "case {case} key {i}"
            );
            assert_eq!(
                full.lineage(o, i),
                part.lineage(o, i),
                "case {case} lo {o} {i}"
            );
        }
        let (hf, hp) = (hits(full, o, n), hits(part, o, n));
        assert_eq!(hf.len(), hp.len(), "case {case} records");
        for (f, p) in hf.iter().zip(&hp) {
            assert_eq!(
                (f.bar, f.entry.to_bits(), f.lo, &f.state),
                (p.bar, p.entry.to_bits(), p.lo, &p.state)
            );
            if p.outcome == OutcomeKind::Open {
                assert!(
                    f.exit_bar.is_none_or(|b| b >= n as u64),
                    "case {case}: open in prefix"
                );
            } else {
                assert_eq!(f, p, "case {case}: resolved outcome");
            }
        }
    }
}

#[test]
fn prefix_stability_over_random_programs() {
    let mut cases = 0;
    let mut nonempty = 0;
    for case in 0..240u64 {
        let mut r = Rng(0x9E37_79B9_7F4A_7C15 ^ (case * 0x1000_0000_01B3 + 1));
        let p = random_program(&mut r);
        let c = checked(&p);
        let len = 40 + r.below(300) as usize;
        let b = bars(len, case + 100);
        let n = 1 + r.below(len as u64) as usize;
        let full = run(&c, &b);
        let part = run(&c, &b[..n]);
        same_prefix(&full, &part, n, case);
        cases += 1;
        nonempty += usize::from((0..n).any(|i| full.value_at(1, i) == Some(1.0)));
    }
    assert!(cases >= 200);
    assert!(nonempty > 20, "events occur in many cases ({nonempty})");
}
