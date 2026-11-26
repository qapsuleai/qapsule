//! Shared helpers of the executor tests: a deterministic generator, synthetic bars,
//! checking a program bound to an interval, running it.
#![allow(dead_code)]

use qp_lang::{check, Align, Binding, CheckCtx, Checked, ExecOptions, Executor, Limit, Limits};
use qp_schema::Bar;
use serde_json::{json, Value};

/// xorshift64*: small, deterministic, no dependency.
pub struct Rng(pub u64);

impl Rng {
    pub fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    /// In `[0, 1)`.
    pub fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
    /// In `0..n`.
    pub fn below(&mut self, n: u64) -> u64 {
        self.next() % n.max(1)
    }
    pub fn pick<'a, T>(&mut self, v: &'a [T]) -> &'a T {
        &v[self.below(v.len() as u64) as usize]
    }
}

/// `n` random-walk bars from `t0`, `step` ms apart.
pub fn bars_at(n: usize, seed: u64, t0: i64, step: i64) -> Vec<Bar> {
    let mut r = Rng(seed | 1);
    let mut c = 100.0;
    (0..n)
        .map(|i| {
            let o = c;
            c *= 1.0 + (r.unit() - 0.5) * 0.04;
            let h = o.max(c) * (1.0 + r.unit() * 0.01);
            let l = o.min(c) * (1.0 - r.unit() * 0.01);
            Bar {
                t: t0 + i as i64 * step,
                o,
                h,
                l,
                c,
                v: (r.unit() * 1000.0).floor(),
            }
        })
        .collect()
}

/// 2024-01-01 00:00 UTC (a Monday).
pub const T0: i64 = 1_704_067_200_000;
pub const HOUR: i64 = 3_600_000;

/// `n` hourly UTC bars.
pub fn bars(n: usize, seed: u64) -> Vec<Bar> {
    bars_at(n, seed, T0, HOUR)
}

/// Limits wide enough for generated programs.
pub fn wide() -> Limits {
    Limits::default()
        .with(Limit::MaxCostPerBar, 100_000)
        .with(Limit::MaxNodes, 1000)
        .with(Limit::MaxIrBytes, 1 << 20)
}

/// Checks `v` bound to `interval` and returns the resolved program (panics on errors).
pub fn checked_with(v: &Value, interval: &str, params: Option<&Value>, limits: Limits) -> Checked {
    let ctx = CheckCtx {
        limits,
        binding: Some(Binding {
            interval: Some(interval.to_owned()),
            bars: None,
        }),
        params,
        ..CheckCtx::default()
    };
    let r = check(v, &ctx);
    assert!(r.ok(), "{:#?}", r.errors);
    r.checked.expect("checked")
}

/// [`checked_with`] at `1h`, default values, wide limits.
pub fn checked(v: &Value) -> Checked {
    checked_with(v, "1h", None, wide())
}

/// A closed-window run in UTC alignment.
pub fn run(c: &Checked, bars: &[Bar]) -> Executor {
    Executor::run_closed(c, bars, ExecOptions::default()).expect("runs")
}

/// A closed-window run with `align`.
pub fn run_aligned(c: &Checked, bars: &[Bar], align: Align) -> Executor {
    let opts = ExecOptions {
        align,
        ..ExecOptions::default()
    };
    Executor::run_closed(c, bars, opts).expect("runs")
}

/// A program from nodes and outputs.
pub fn prog(nodes: Value, outputs: Value) -> Value {
    json!({"version": "qp-ir/0", "nodes": nodes, "outputs": outputs})
}

/// A program with parameters.
pub fn prog_p(params: Value, nodes: Value, outputs: Value) -> Value {
    json!({"version": "qp-ir/0", "params": params, "nodes": nodes, "outputs": outputs})
}

/// Values of output `name` on every closed bar.
pub fn values(x: &Executor, name: &str) -> Vec<Option<f64>> {
    let o = x.output(name).expect("output");
    (0..x.closed_len()).map(|i| x.value_at(o, i)).collect()
}

/// Bits of a value column (bit-equality, `None` distinct).
pub fn bits(v: &[Option<f64>]) -> Vec<Option<u64>> {
    v.iter().map(|x| x.map(f64::to_bits)).collect()
}

/// A program using every kind of node (higher timeframe included) and every output kind.
pub fn kitchen_sink() -> Value {
    prog_p(
        json!([{"key": "len", "kind": "int", "min": 2, "max": 50, "default": 14}]),
        json!([
            {"id": "c", "op": "input", "field": "close"},
            {"id": "h", "op": "input", "field": "high"},
            {"id": "r", "op": "rsi", "x": "c", "period": {"param": "len"}},
            {"id": "m", "op": "macd", "x": "c", "fast": 5, "slow": 13, "signal": 4},
            {"id": "bb", "op": "bollinger", "x": "c", "period": 10, "k": 2.0},
            {"id": "sd", "op": "stdev", "x": "r", "period": 5},
            {"id": "pr", "op": "percentrank", "x": "c", "period": 7},
            {"id": "top", "op": "highest", "x": "h", "period": 10},
            {"id": "prev", "op": "lag", "x": "top", "n": 1},
            {"id": "brk", "op": "gt", "a": "c", "b": "prev"},
            {"id": "dc", "op": "input", "field": "close", "tf": "4h"},
            {"id": "dm", "op": "sma", "x": "dc", "period": 3},
            {"id": "al", "op": "align", "x": "dm"},
            {"id": "x", "op": "cross_above", "a": "c", "b": "al"},
            {"id": "cd", "op": "cooldown", "trigger": "x", "bars": 3},
            {"id": "k30", "op": "const", "value": 30},
            {"id": "lo", "op": "lt", "a": "r", "b": "k30"},
            {"id": "dip", "op": "rearm", "trigger": "lo", "reset": "brk"},
            {"id": "st", "op": "streak", "c": "brk"},
            {"id": "bs", "op": "bars_since", "c": "brk"},
            {"id": "on", "op": "once", "c": "brk"},
            {"id": "a", "op": "atr", "period": 5},
            {"id": "a4", "op": "atr", "period": 3, "tf": "4h"},
            {"id": "a4b", "op": "align", "x": "a4"},
            {"id": "rat", "op": "div", "a": "a", "b": "a4b"},
            {"id": "bk", "op": "bucket", "x": "r", "edges": [30, 50, 70]},
            {"id": "bk2", "op": "bucket", "x": "pr", "edges": [50]},
            {"id": "def", "op": "defined", "x": "al"},
            {"id": "ln", "op": "line", "a": {"t": 1_704_078_000_000_i64, "price": 99.0},
                                       "b": {"t": 1_704_157_200_000_i64, "price": 101.0}},
            {"id": "sum0", "op": "add", "a": "sd", "b": "rat"},
            {"id": "bw", "op": "sub", "a": "bb.upper", "b": "bb"},
            {"id": "sum", "op": "add", "a": "sum0", "b": "bw"},
            {"id": "mix", "op": "max", "a": "sum", "b": "m.histogram"},
            {"id": "both", "op": "and", "a": "on", "b": "def"},
            {"id": "any", "op": "or", "a": "cd", "b": "dip"},
            {"id": "sig", "op": "or", "a": "any", "b": "both"},
            {"id": "z", "op": "add", "a": "st", "b": "ln"},
            {"id": "bk3", "op": "bucket", "x": "z", "edges": [1]},
            {"id": "bk4", "op": "bucket", "x": "bs", "edges": [2]}
        ]),
        json!([
            {"name": "mix", "kind": "series", "node": "mix"},
            {"name": "brk", "kind": "event", "node": "x"},
            {"name": "entry", "kind": "signal", "node": "sig", "side": "al",
             "rule": {"target_pct": 2.0, "stop_pct": 1.0, "max_bars": 12},
             "source": {"strategy": "sink", "version": "1"}},
            {"name": "reg", "kind": "state", "dims": [{"name": "mom", "node": "bk"}, {"name": "rank", "node": "bk2"},
                                                  {"name": "z", "node": "bk3"}, {"name": "since", "node": "bk4"}]}
        ]),
    )
}
