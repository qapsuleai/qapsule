//! Sweep benchmark: 10,000 bars x 1,000 parameter sets per study, single thread.
//!
//! ```sh
//! cargo run --release -p qp-compute --example sweep_bench [-- --bars 10000 --reps 15]
//! ```
//! Prints one line per study: min / median / max wall time over `reps` runs (after one
//! warm-up run) and the output digest (identical on every target).

use std::time::Instant;

use qp_compute::sweep::{
    bench_closes, bench_grid, fnv1a_columns, sweep_each, sweep_each_lanes, FNV_OFFSET,
};
use qp_compute::STUDY_NAMES;

fn arg(name: &str, default: usize) -> usize {
    let args: Vec<String> = std::env::args().collect();
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn main() {
    let bars = arg("--bars", 10_000);
    let reps = arg("--reps", 15);
    // --lanes 0 (default) = each study's tuned default.
    let lanes = match arg("--lanes", 0) {
        0 => None,
        l => Some(l),
    };
    let x = bench_closes(bars, 20_261_114);
    println!("bars={bars} reps={reps} (single thread, release)");
    println!(
        "{:<10} {:>9} {:>9} {:>9} {:>9}  digest",
        "study", "variants", "min ms", "med ms", "max ms"
    );
    for name in STUDY_NAMES {
        let grid = bench_grid(name);
        let mut digest = FNV_OFFSET;
        // Warm-up run also computes the digest of all outputs.
        sweep_each(name, &grid, &x, |_, c| digest = fnv1a_columns(digest, c)).unwrap();
        let mut times = Vec::with_capacity(reps);
        let mut sink = 0.0;
        for _ in 0..reps {
            let t0 = Instant::now();
            sweep_each_lanes(name, &grid, &x, lanes, |_, c| {
                sink += c.columns[0][bars - 1]
            })
            .unwrap();
            times.push(t0.elapsed().as_secs_f64() * 1e3);
        }
        times.sort_by(f64::total_cmp);
        println!(
            "{:<10} {:>9} {:>9.2} {:>9.2} {:>9.2}  {digest:016x}",
            name,
            grid.len(),
            times[0],
            times[reps / 2],
            times[reps - 1]
        );
        std::hint::black_box(sink);
    }
}
