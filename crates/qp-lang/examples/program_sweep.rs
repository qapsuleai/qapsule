//! K17 through the program path (RL1, VERIFY 19): the PR §14 `sma` program over 10 000
//! bars × 1 000 parameter values (period 2..=1001, `qp_compute::sweep::bench_grid`), one
//! executor per value, single thread. The same closes as `qp-compute`'s `sweep_bench`, so
//! the digest must equal its `sma` digest (PR §14: bit-equal). Release build; also runs
//! as wasm32-wasip1 under `node:wasi`.
//!
//! ```sh
//! cargo run --release -p qp-lang --example program_sweep [-- --reps 7]
//! ```

#![allow(clippy::cast_precision_loss, clippy::unwrap_used, clippy::expect_used)]

use std::time::Instant;

use qp_compute::sweep::{bench_closes, bench_grid, FNV_OFFSET};
use qp_lang::{check, Binding, CheckCtx, Checked, ExecOptions, Executor};
use qp_schema::Bar;

/// PR §14 `sma` as IR.
const SMA: &str = r#"{"version":"qp-ir/0","params":[{"key":"period","kind":"int","min":1,"max":2000,"default":20}],"nodes":[{"id":"c","op":"input","field":"close"},{"id":"s","op":"sma","x":"c","period":{"param":"period"}}],"outputs":[{"name":"sma","kind":"series","node":"s"}]}"#;

fn reps() -> usize {
    let a: Vec<String> = std::env::args().collect();
    a.iter()
        .position(|x| x == "--reps")
        .and_then(|i| a.get(i + 1))
        .and_then(|v| v.parse().ok())
        .unwrap_or(7)
}

fn checked(ir: &serde_json::Value, period: f64) -> Checked {
    #[allow(clippy::cast_possible_truncation)]
    let params = serde_json::json!({ "period": period as i64 });
    let ctx = CheckCtx {
        binding: Some(Binding {
            interval: Some("1h".to_owned()),
            bars: None,
        }),
        params: Some(&params),
        ..CheckCtx::default()
    };
    check(ir, &ctx).checked.expect("checks")
}

fn fnv(mut h: u64, v: f64) -> u64 {
    for b in v.to_bits().to_le_bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

fn median(mut t: Vec<f64>) -> (f64, f64, f64) {
    t.sort_by(f64::total_cmp);
    (t[0], t[t.len() / 2], t[t.len() - 1])
}

fn main() {
    let n = 10_000;
    let reps = reps();
    let bars: Vec<Bar> = bench_closes(n, 20_261_114)
        .into_iter()
        .enumerate()
        .map(|(i, c)| Bar {
            t: 1_704_067_200_000 + i as i64 * 3_600_000,
            o: c,
            h: c,
            l: c,
            c,
            v: 0.0,
        })
        .collect();
    let ir: serde_json::Value = serde_json::from_str(SMA).unwrap();
    let grid: Vec<f64> = bench_grid("sma").iter().map(|g| g["period"]).collect();
    // Warm-up and digest (NaN for undefined, as the sweep's warmup slots).
    let mut digest = FNV_OFFSET;
    for &p in &grid {
        let x = Executor::run_closed(&checked(&ir, p), &bars, ExecOptions::default()).unwrap();
        for i in 0..n {
            digest = fnv(digest, x.value_at(0, i).unwrap_or(f64::NAN));
        }
    }
    let pre: Vec<Checked> = grid.iter().map(|&p| checked(&ir, p)).collect();
    let (mut run, mut full) = (Vec::new(), Vec::new());
    let mut sink = 0.0;
    for _ in 0..reps {
        let t = Instant::now();
        for c in &pre {
            let x = Executor::run_closed(c, &bars, ExecOptions::default()).unwrap();
            sink += x.value_at(0, n - 1).unwrap_or(0.0);
        }
        run.push(t.elapsed().as_secs_f64() * 1e3);
        let t = Instant::now();
        for &p in &grid {
            let x = Executor::run_closed(&checked(&ir, p), &bars, ExecOptions::default()).unwrap();
            sink += x.value_at(0, n - 1).unwrap_or(0.0);
        }
        full.push(t.elapsed().as_secs_f64() * 1e3);
    }
    std::hint::black_box(sink);
    let (a, b, c) = median(run);
    let (d, e, f) = median(full);
    println!(
        "bars={n} variants={} reps={reps} (single thread)",
        grid.len()
    );
    println!("executor only      min {a:.2}  med {b:.2}  max {c:.2} ms");
    println!("check + executor   min {d:.2}  med {e:.2}  max {f:.2} ms");
    println!("digest {digest:016x}");
}
