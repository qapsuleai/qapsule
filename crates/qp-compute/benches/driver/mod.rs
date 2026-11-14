//! Shared timing driver for the RP1 load profiles (`qp_compute::profiles`), used by
//! `benches/profiles.rs` and `examples/sweep_bench.rs` (so CI prints it too).
//!
//! Timing lives here, outside the library: the compute code never reads a clock.
//! Every line ends with the digest of all outputs (computed in an untimed warm-up run);
//! a correct optimization keeps every digest unchanged.

#![allow(dead_code)]

use std::time::Instant;

use qp_compute::profiles::{self as pr, Bank};
use qp_compute::sweep::{fnv1a_columns, FNV_OFFSET};
use qp_compute::STUDY_NAMES;

/// `--name value` from the command line.
pub fn arg(name: &str) -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

/// `--name n` as a number.
pub fn arg_usize(name: &str, default: usize) -> usize {
    arg(name).and_then(|v| v.parse().ok()).unwrap_or(default)
}

/// Best-effort description of the CPU (for tying numbers to hardware).
pub fn machine() -> String {
    let arch = std::env::consts::ARCH;
    let os = std::env::consts::OS;
    let mut model = String::from("?");
    if let Ok(s) = std::fs::read_to_string("/proc/cpuinfo") {
        if let Some(l) = s.lines().find(|l| l.starts_with("model name")) {
            model = l.split(':').nth(1).unwrap_or("?").trim().to_owned();
        }
    } else if os == "macos" {
        if let Ok(o) = std::process::Command::new("sysctl")
            .args(["-n", "machdep.cpu.brand_string"])
            .output()
        {
            model = String::from_utf8_lossy(&o.stdout).trim().to_owned();
        }
    }
    let cores = std::thread::available_parallelism().map_or(0, std::num::NonZero::get);
    format!("{arch}-{os}, cpu: {model}, threads: {cores}")
}

/// min / median / max of `t` (sorted in place).
fn stats(t: &mut [f64]) -> (f64, f64, f64) {
    t.sort_by(f64::total_cmp);
    (t[0], t[t.len() / 2], t[t.len() - 1])
}

fn time_reps(reps: usize, mut f: impl FnMut() -> f64) -> (f64, f64, f64) {
    let mut sink = 0.0;
    let mut t = Vec::with_capacity(reps);
    for _ in 0..reps {
        let t0 = Instant::now();
        sink += f();
        t.push(t0.elapsed().as_secs_f64() * 1e3);
    }
    std::hint::black_box(sink);
    stats(&mut t)
}

fn line(profile: &str, case: &str, items: usize, (mn, md, mx): (f64, f64, f64), digest: u64) {
    println!("{profile:<3} {case:<14} {items:>8} {mn:>9.2} {md:>9.2} {mx:>9.2}  {digest:016x}");
}

fn header(title: &str) {
    println!("{title}");
    println!(
        "{:<3} {:<14} {:>8} {:>9} {:>9} {:>9}  digest",
        "", "case", "items", "min ms", "med ms", "max ms"
    );
}

/// Profile A (10k bars × 1,000 variants per study).
pub fn profile_a(reps: usize) {
    header("profile A: one series, 1,000 parameter sets (10,000 bars)");
    let x = pr::a_series();
    for name in STUDY_NAMES {
        let mut d = FNV_OFFSET;
        pr::a_run(name, &x, |_, c| d = fnv1a_columns(d, c));
        let n = x.len();
        let s = time_reps(reps, || {
            let mut sink = 0.0;
            pr::a_run(name, &x, |_, c| sink += c.columns[0][n - 1]);
            sink
        });
        line("A", name, 1000, s, d);
    }
}

/// Profile B (2,500 series, one study each).
pub fn profile_b(reps: usize) {
    header("profile B: 2,500 series (500 symbols x 5 timeframes, 9.25M bars), one study");
    let series = pr::b_series();
    for name in STUDY_NAMES {
        let mut d = FNV_OFFSET;
        pr::b_run(name, &series, |_, c| d = fnv1a_columns(d, c));
        let s = time_reps(reps, || {
            let mut sink = 0.0;
            pr::b_run(name, &series, |_, c| {
                sink += c.columns[0][c.columns[0].len() - 1];
            });
            sink
        });
        line("B", name, series.len(), s, d);
    }
}

/// Profile C (chained studies, 1,000 variants per family).
pub fn profile_c(reps: usize) {
    header("profile C: chained studies, 1,000 variants per family (10,000 bars)");
    let x = pr::a_series();
    let n = x.len();
    for fam in pr::C_FAMILIES {
        let mut d = FNV_OFFSET;
        pr::c_run(fam, &x, |_, c| d = fnv1a_columns(d, c));
        let s = time_reps(reps, || {
            let mut sink = 0.0;
            pr::c_run(fam, &x, |_, c| sink += c.columns[0][n - 1]);
            sink
        });
        line("C", fam, 1000, s, d);
    }
}

/// Profile D (live bank of 500 studies): per-operation latency in microseconds.
pub fn profile_d(reps: usize) {
    println!("profile D: live bank of 500 studies on 10,000 bars; latency per operation (all 500)");
    println!(
        "{:<3} {:<14} {:>8} {:>9} {:>9} {:>9} {:>9}  digest",
        "", "case", "ops", "min us", "med us", "p99 us", "max us"
    );
    let digest = pr::d_digest();
    let specs = pr::d_specs();
    let (hist, ticks, new_bars) = pr::d_prices();
    let mut tick_t = Vec::new();
    let mut bar_t = Vec::new();
    let mut warm_t = Vec::new();
    let mut sink = 0.0;
    for _ in 0..reps.max(1) {
        let t0 = Instant::now();
        let mut bank = Bank::new(&specs);
        for &x in &hist {
            bank.push(x);
        }
        warm_t.push(t0.elapsed().as_secs_f64() * 1e3);
        for &t in &ticks {
            let t0 = Instant::now();
            bank.update_last(t);
            tick_t.push(t0.elapsed().as_secs_f64() * 1e6);
            sink += bank.sink();
        }
        for &x in &new_bars {
            let t0 = Instant::now();
            bank.push(x);
            bar_t.push(t0.elapsed().as_secs_f64() * 1e6);
            sink += bank.sink();
        }
    }
    std::hint::black_box(sink);
    for (case, t) in [("tick", &mut tick_t), ("new bar", &mut bar_t)] {
        let n = t.len();
        let (mn, md, mx) = stats(t);
        let p99 = t[(n * 99) / 100];
        println!("D   {case:<14} {n:>8} {mn:>9.2} {md:>9.2} {p99:>9.2} {mx:>9.2}  {digest:016x}");
    }
    let (mn, md, mx) = stats(&mut warm_t);
    println!(
        "D   {:<14} {:>8} {mn:>9.2} {md:>9.2} {:>9} {mx:>9.2}  (ms: build + 10,000 bars)",
        "attach",
        warm_t.len(),
        ""
    );
}

/// Runs the profiles selected by `--only` (default `a,b,c,d`), `--reps` times each.
pub fn run(default_only: &str, default_reps: usize) {
    let reps = arg_usize("--reps", default_reps).max(1);
    let only = arg("--only").unwrap_or_else(|| default_only.to_owned());
    println!("machine: {}", machine());
    println!("reps={reps} (single thread, after one warm-up run that also computes the digest)");
    for p in only.split(',') {
        println!();
        match p.trim() {
            "a" => profile_a(reps),
            "b" => profile_b(reps),
            "c" => profile_c(reps),
            "d" => profile_d(reps),
            "" => {}
            other => println!("unknown profile {other}"),
        }
    }
}
