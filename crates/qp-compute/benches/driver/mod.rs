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

/// Profile A per lane count (median ms), to tune the lane constants per target.
pub fn lane_table(reps: usize) {
    println!("profile A by lane count (median ms; 'default' = the target's tuned value)");
    println!(
        "{:<10} {:>8} {:>8} {:>8} {:>8} {:>8}",
        "study", "default", "1", "2", "4", "8"
    );
    let x = pr::a_series();
    let n = x.len();
    for name in STUDY_NAMES {
        let grid = qp_compute::sweep::bench_grid(name);
        let mut row = format!("{name:<10}");
        for lanes in [None, Some(1), Some(2), Some(4), Some(8)] {
            let (_, md, _) = time_reps(reps, || {
                let mut sink = 0.0;
                qp_compute::sweep::sweep_each_lanes(name, &grid, &x, lanes, |_, c| {
                    sink += c.columns[0][n - 1];
                })
                .expect("valid grid");
                sink
            });
            row.push_str(&format!(" {md:>8.2}"));
        }
        println!("{row}");
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
        if arg("--baseline").is_some() {
            let mut d = FNV_OFFSET;
            pr::c_run_baseline(fam, &x, |_, c| d = fnv1a_columns(d, c));
            let s = time_reps(reps, || {
                let mut sink = 0.0;
                pr::c_run_baseline(fam, &x, |_, c| sink += c.columns[0][n - 1]);
                sink
            });
            line("C", &format!("{fam} (T1)"), 1000, s, d);
        }
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
        "D   {:<14} {:>8} {mn:>9.2} {md:>9.2} {:>9} {mx:>9.2}  (ms: build + 10,000 bars, push)",
        "attach",
        warm_t.len(),
        ""
    );
    let mut fast_t = Vec::new();
    for _ in 0..reps.max(1) {
        let t0 = Instant::now();
        let mut bank = Bank::new(&specs);
        bank.warm(&hist);
        fast_t.push(t0.elapsed().as_secs_f64() * 1e3);
        std::hint::black_box(&bank);
    }
    let (mn, md, mx) = stats(&mut fast_t);
    println!(
        "D   {:<14} {:>8} {mn:>9.2} {md:>9.2} {:>9} {mx:>9.2}  (ms: build + 10,000 bars, extend)",
        "attach (warm)",
        fast_t.len(),
        ""
    );
}

/// Content-addressed cache costs (RP1/E4): hashing, hit, miss, repeated sweep.
pub fn cache_probe(reps: usize) {
    use qp_compute::graph::{content_hash, ResultCache};
    println!("cache: content-addressed results (10,000-bar input; times in microseconds)");
    let x = pr::a_series();
    let per = |f: &mut dyn FnMut(), k: usize| {
        let mut t = Vec::new();
        for _ in 0..reps {
            let t0 = Instant::now();
            for _ in 0..k {
                f();
            }
            t.push(t0.elapsed().as_secs_f64() * 1e6 / k as f64);
        }
        stats(&mut t).1
    };
    let hash = per(
        &mut || {
            std::hint::black_box(content_hash(std::hint::black_box(&x)));
        },
        100,
    );
    let h = content_hash(&x);
    let p14 = std::collections::BTreeMap::from([("period".to_owned(), 14.0)]);
    let miss = per(
        &mut || {
            let mut c = ResultCache::new();
            std::hint::black_box(c.get_or_compute(h, "rsi", &p14, &x).unwrap());
        },
        20,
    );
    let mut cache = ResultCache::new();
    let _ = cache.get_or_compute(h, "rsi", &p14, &x).unwrap();
    let hit = per(
        &mut || {
            std::hint::black_box(cache.get_or_compute(h, "rsi", &p14, &x).unwrap());
        },
        1000,
    );
    let grid = qp_compute::sweep::bench_grid("rsi");
    for p in &grid {
        let _ = cache.get_or_compute(h, "rsi", p, &x).unwrap();
    }
    let rehit = per(
        &mut || {
            let h2 = content_hash(&x);
            for p in &grid {
                std::hint::black_box(cache.get_or_compute(h2, "rsi", p, &x).unwrap());
            }
        },
        1,
    );
    let fresh = per(
        &mut || {
            pr::a_run("rsi", &x, |_, c| {
                std::hint::black_box(c.columns[0][0]);
            });
        },
        1,
    );
    println!("hash 10k bars       {hash:>10.2}");
    println!("rsi(14) miss        {miss:>10.2}  (compute + insert)");
    println!("rsi(14) hit         {hit:>10.2}");
    println!("rsi sweep, 1000 hits {rehit:>9.2}  (rehash + 1,000 lookups)");
    println!("rsi sweep, fresh    {fresh:>10.2}");
    println!(
        "cached entries: {}, memory ~ {:.1} MB",
        cache.len(),
        (cache.len() * x.len() * 8) as f64 / 1e6
    );
/// Thread scaling of the reducing parallel sweep (RP1/E3), profile A, median ms.
pub fn thread_table(reps: usize) {
    use qp_compute::par::sweep_reduce_par;
    let x = pr::a_series();
    let n = x.len();
    let counts = [1usize, 2, 4, 8, 12, 16];
    for (label, heavy) in [
        ("reduce = last value", false),
        ("reduce = read every output", true),
    ] {
        println!("threads, profile A, {label} (median ms)");
        println!(
            "{:<10} {}",
            "study",
            counts.map(|c| format!("{c:>8}")).join("")
        );
        for name in STUDY_NAMES {
            let grid = qp_compute::sweep::bench_grid(name);
            let mut row = format!("{name:<10}");
            for &t in &counts {
                let (_, md, _) = time_reps(reps, || {
                    let r = sweep_reduce_par(name, &grid, &x, t, |c| {
                        if heavy {
                            fnv1a_columns(FNV_OFFSET, c) as f64
                        } else {
                            c.columns[0][n - 1]
                        }
                    })
                    .unwrap();
                    r[r.len() - 1]
                });
                row.push_str(&format!("{md:>8.2}"));
            }
            println!("{row}");
        }
    }
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
            "lanes" => lane_table(reps),
            "cache" => cache_probe(reps),
            "threads" => thread_table(reps),
            "" => {}
            other => println!("unknown profile {other}"),
        }
    }
}
