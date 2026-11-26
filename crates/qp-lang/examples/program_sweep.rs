//! K17 through the program path (RL1, VERIFY 19): the five PR §14 programs over 10 000
//! bars × 1 000 parameter sets (the K17 grid, `qp_compute::sweep::bench_grid`), single
//! thread, through `qp_lang::sweep` (checker per set + shared path). Every variation is
//! compared, output by output, with `qp-compute`'s `sweep_each` on the same closes (PR §14:
//! bit-equal). `--exec` adds the reference: one `Executor::run_closed` per set (`sma`).
//! Release build; also runs as wasm32-wasip1 under `node:wasi`.
//!
//! ```sh
//! cargo run --release -p qp-lang --example program_sweep [-- --reps 7] [--exec]
//! ```

#![allow(clippy::cast_precision_loss, clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;
use std::time::Instant;

use qp_compute::sweep::{bench_closes, bench_grid, sweep_each, FNV_OFFSET};
use qp_lang::{check, sweep, Binding, CheckCtx, ExecOptions, Executor, SweepPath};
use qp_schema::Bar;
use serde_json::{json, Value};

const N: usize = 10_000;

fn arg(name: &str) -> Option<String> {
    let a: Vec<String> = std::env::args().collect();
    a.iter()
        .position(|x| x == name)
        .and_then(|i| a.get(i + 1))
        .cloned()
}

fn int(key: &str, default: i64) -> Value {
    json!({"key": key, "kind": "int", "min": 1, "max": 2000, "default": default})
}

fn series(name: &str, node: &str) -> Value {
    json!({"name": name, "kind": "series", "node": node})
}

/// The PR §14 program of `study` as IR.
fn program(study: &str) -> Value {
    let close = json!({"id": "_1", "op": "input", "field": "close"});
    let (params, node, outputs) = match study {
        "sma" | "ema" | "rsi" => (
            json!([int("period", 20)]),
            json!({"id": "_2", "op": study, "x": "_1", "period": {"param": "period"}}),
            json!([series(study, "_2")]),
        ),
        "macd" => (
            json!([int("fast", 12), int("slow", 26), int("signal", 9)]),
            json!({"id": "m", "op": "macd", "x": "_1", "fast": {"param": "fast"},
                   "slow": {"param": "slow"}, "signal": {"param": "signal"}}),
            json!([
                series("macd", "m.macd"),
                series("signal", "m.signal"),
                series("histogram", "m.histogram")
            ]),
        ),
        _ => (
            json!([int("period", 20),
                   {"key": "k", "kind": "float", "min": 0.0001, "max": 100.0, "default": 2.0}]),
            json!({"id": "b", "op": "bollinger", "x": "_1", "period": {"param": "period"},
                   "k": {"param": "k"}}),
            json!([
                series("upper", "b.upper"),
                series("middle", "b.middle"),
                series("lower", "b.lower"),
                series("bandwidth", "b.bandwidth")
            ]),
        ),
    };
    json!({"version": "qp-ir/0", "params": params, "nodes": [close, node], "outputs": outputs})
}

/// The K17 grid of `study` as parameter sets (integers stay integers).
fn sets(study: &str) -> Vec<Value> {
    bench_grid(study)
        .iter()
        .map(|g| {
            let m: serde_json::Map<String, Value> = g
                .iter()
                .map(|(k, v)| {
                    #[allow(clippy::cast_possible_truncation)]
                    let v = if k == "k" { json!(v) } else { json!(*v as i64) };
                    (k.clone(), v)
                })
                .collect();
            Value::Object(m)
        })
        .collect()
}

fn fnv(mut h: u64, v: f64) -> u64 {
    for b in v.to_bits().to_le_bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

fn stats(mut t: Vec<f64>) -> (f64, f64, f64) {
    t.sort_by(f64::total_cmp);
    (t[0], t[t.len() / 2], t[t.len() - 1])
}

fn ctx() -> CheckCtx<'static> {
    CheckCtx {
        binding: Some(Binding {
            interval: Some("1h".to_owned()),
            bars: None,
        }),
        ..CheckCtx::default()
    }
}

/// Every variation, output by output, against `sweep_each` (FNV of each column); returns
/// the digest over all values and the path.
fn verify(study: &str, closes: &[f64], bars: &[Bar]) -> (u64, SweepPath) {
    let grid: Vec<BTreeMap<String, f64>> = bench_grid(study);
    let mut want: Vec<BTreeMap<&'static str, u64>> = vec![BTreeMap::new(); grid.len()];
    sweep_each(study, &grid, closes, |v, c| {
        for (j, name) in c.names.iter().enumerate() {
            let h = c.columns[j].iter().fold(FNV_OFFSET, |h, x| fnv(h, *x));
            want[v].insert(name, h);
        }
    })
    .unwrap();
    let mut got: Vec<u64> = vec![0; grid.len()];
    let path = sweep(
        &program(study),
        &ctx(),
        &sets(study),
        bars,
        &ExecOptions::default(),
        |v| {
            let mut all = FNV_OFFSET;
            for (j, name) in v.names().iter().enumerate() {
                let mut h = FNV_OFFSET;
                for i in 0..v.len() {
                    let x = v.value(j, i).unwrap_or(f64::NAN);
                    h = fnv(h, x);
                    all = fnv(all, x);
                }
                assert_eq!(
                    want[v.index()][name.as_str()],
                    h,
                    "{study} set {} {name}",
                    v.index()
                );
            }
            got[v.index()] = all;
        },
    )
    .unwrap();
    (
        got.iter()
            .fold(FNV_OFFSET, |h, x| fnv(h, f64::from_bits(*x))),
        path,
    )
}

fn main() {
    let reps: usize = arg("--reps").and_then(|v| v.parse().ok()).unwrap_or(7);
    let closes = bench_closes(N, 20_261_114);
    let bars: Vec<Bar> = closes
        .iter()
        .enumerate()
        .map(|(i, &c)| Bar {
            t: 1_704_067_200_000 + i as i64 * 3_600_000,
            o: c,
            h: c,
            l: c,
            c,
            v: 0.0,
        })
        .collect();
    println!("bars={N} variants=1000 reps={reps} (single thread; check + sweep, ms)");
    for study in ["sma", "ema", "rsi", "macd", "bollinger"] {
        let (digest, path) = verify(study, &closes, &bars);
        let (ir, sets) = (program(study), sets(study));
        let mut t = Vec::new();
        let mut sink = 0.0;
        for _ in 0..reps {
            let s = Instant::now();
            sweep(&ir, &ctx(), &sets, &bars, &ExecOptions::default(), |v| {
                sink += v.value(0, N - 1).unwrap_or(0.0);
            })
            .unwrap();
            t.push(s.elapsed().as_secs_f64() * 1e3);
        }
        // The checker's share: one check per set, nothing run.
        let mut tc = Vec::new();
        for _ in 0..reps {
            let s = Instant::now();
            for p in &sets {
                let cx = CheckCtx {
                    params: Some(p),
                    ..ctx()
                };
                sink += f64::from(u8::from(check(&ir, &cx).checked.is_some()));
            }
            tc.push(s.elapsed().as_secs_f64() * 1e3);
        }
        std::hint::black_box(sink);
        let (a, b, c) = stats(t);
        let (_, bc, _) = stats(tc);
        println!(
            "{study:<10} min {a:7.2}  med {b:7.2}  max {c:7.2}  (check alone med {bc:6.2})  {path:?}  digest {digest:016x} (= sweep_each)"
        );
    }
    if std::env::args().any(|a| a == "--exec") {
        let ir = program("sma");
        let pre: Vec<_> = sets("sma")
            .iter()
            .map(|p| {
                let cx = CheckCtx {
                    params: Some(p),
                    ..ctx()
                };
                check(&ir, &cx).checked.unwrap()
            })
            .collect();
        let mut t = Vec::new();
        let mut sink = 0.0;
        for _ in 0..reps {
            let s = Instant::now();
            for c in &pre {
                let x = Executor::run_closed(c, &bars, ExecOptions::default()).unwrap();
                sink += x.value_at(0, N - 1).unwrap_or(0.0);
            }
            t.push(s.elapsed().as_secs_f64() * 1e3);
        }
        std::hint::black_box(sink);
        let (a, b, c) = stats(t);
        println!("sma executor per set (reference)  min {a:.2}  med {b:.2}  max {c:.2}");
    }
}
