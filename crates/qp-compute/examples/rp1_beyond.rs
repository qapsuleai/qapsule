//! RP1/E7: probes of approaches that are fast but break (or change) the compute invariants.
//! Nothing here is used by the library. Each probe times a "current definition" kernel and
//! a "changed" kernel written in the same style (4 lanes, structure of arrays), then
//! compares the changed outputs with the library's single runs bit by bit.
//!
//! ```sh
//! cargo run --release -p qp-compute --example rp1_beyond [-- --reps 7]
//! ```
//! The digest column must be compared across targets (aarch64, x86_64, wasm32).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::cognitive_complexity
)]

use std::time::Instant;

use qp_compute::profiles::a_series;
use qp_compute::sweep::{bench_closes, sweep_each, FNV_OFFSET};

const NAN: f64 = f64::NAN;

fn reps() -> usize {
    let a: Vec<String> = std::env::args().collect();
    a.iter()
        .position(|x| x == "--reps")
        .and_then(|i| a.get(i + 1))
        .and_then(|v| v.parse().ok())
        .unwrap_or(7)
}

fn median_ms(reps: usize, mut f: impl FnMut() -> f64) -> f64 {
    let mut t = Vec::new();
    let mut sink = 0.0;
    for _ in 0..reps {
        let t0 = Instant::now();
        sink += f();
        t.push(t0.elapsed().as_secs_f64() * 1e3);
    }
    std::hint::black_box(sink);
    t.sort_by(f64::total_cmp);
    t[t.len() / 2]
}

/// Library single-run reference for `name` over `periods` (output 0).
fn reference(name: &str, periods: &[usize], x: &[f64]) -> Vec<Vec<f64>> {
    let grid: Vec<_> = periods
        .iter()
        .map(|&p| std::collections::BTreeMap::from([("period".to_owned(), p as f64)]))
        .collect();
    let mut out = Vec::new();
    sweep_each(name, &grid, x, |_, c| out.push(c.columns[0].clone())).unwrap();
    out
}

#[derive(Default)]
struct Diff {
    n: usize,
    differ: usize,
    max_abs: f64,
    max_rel: f64,
    max_ulp: u64,
    digest: u64,
}

fn diff(got: &[Vec<f64>], want: &[Vec<f64>], from: usize) -> Diff {
    let mut d = Diff {
        digest: FNV_OFFSET,
        ..Diff::default()
    };
    for (g, w) in got.iter().zip(want) {
        for (i, (&a, &b)) in g.iter().zip(w).enumerate() {
            d.digest = qp_compute::profiles::fnv_u64(d.digest, a.to_bits());
            if i < from || (a.is_nan() && b.is_nan()) {
                continue;
            }
            d.n += 1;
            if a.to_bits() != b.to_bits() {
                d.differ += 1;
                let abs = (a - b).abs();
                d.max_abs = d.max_abs.max(abs);
                d.max_rel = d.max_rel.max(abs / b.abs().max(1e-300));
                if a.is_sign_positive() == b.is_sign_positive() {
                    d.max_ulp = d.max_ulp.max(a.to_bits().abs_diff(b.to_bits()));
                }
            }
        }
    }
    d
}

fn row(name: &str, cur_ms: f64, new_ms: f64, d: &Diff) {
    println!(
        "{name:<28} {cur_ms:>8.2} {new_ms:>8.2} {:>6.2}x  {:>6.2}% {:>9.2e} {:>9.2e} {:>8}  {:016x}",
        cur_ms / new_ms,
        100.0 * d.differ as f64 / d.n.max(1) as f64,
        d.max_abs,
        d.max_rel,
        d.max_ulp,
        d.digest
    );
}

// --- EMA: current formula vs FMA ------------------------------------------------------

fn ema_lanes<const FMA: bool>(x: &[f64], periods: &[usize]) -> Vec<Vec<f64>> {
    let n = x.len();
    let mut out = vec![vec![NAN; n]; periods.len()];
    for (c, chunk) in periods.chunks(4).enumerate() {
        let l_n = chunk.len();
        let mut prev = [0.0; 4];
        let mut a = [0.0; 4];
        let maxp = *chunk.iter().max().unwrap();
        // Seed exactly like the library (mean of the first p inputs), per lane.
        for l in 0..l_n {
            let p = chunk[l];
            let pf = p as f64;
            a[l] = 2.0 / (pf + 1.0);
            let mut s = 0.0;
            for i in 0..p.min(n) {
                if i + 1 < p {
                    s += x[i];
                } else {
                    prev[l] = (s + x[i]) / pf;
                    out[c * 4 + l][i] = prev[l];
                }
            }
            for i in p..maxp.min(n) {
                prev[l] = if FMA {
                    x[i].mul_add(a[l], prev[l] * (1.0 - a[l]))
                } else {
                    x[i] * a[l] + prev[l] * (1.0 - a[l])
                };
                out[c * 4 + l][i] = prev[l];
            }
        }
        for i in maxp..n {
            let mut v = [0.0; 4];
            for l in 0..4 {
                v[l] = if FMA {
                    x[i].mul_add(a[l], prev[l] * (1.0 - a[l]))
                } else {
                    x[i] * a[l] + prev[l] * (1.0 - a[l])
                };
                prev[l] = v[l];
            }
            for l in 0..l_n {
                out[c * 4 + l][i] = v[l];
            }
        }
    }
    out
}

// --- RSI: division vs reciprocal multiplication --------------------------------------

fn rsi_lanes<const RECIP: bool>(x: &[f64], periods: &[usize]) -> Vec<Vec<f64>> {
    let n = x.len();
    let mut out = vec![vec![NAN; n]; periods.len()];
    for (c, chunk) in periods.chunks(4).enumerate() {
        let l_n = chunk.len();
        let (mut ag, mut al, mut pf, mut pm1, mut inv) =
            ([0.0; 4], [0.0; 4], [1.0; 4], [0.0; 4], [1.0; 4]);
        let maxp = *chunk.iter().max().unwrap();
        let start = (maxp + 1).min(n);
        for l in 0..4 {
            let p = chunk[l.min(l_n - 1)];
            pf[l] = p as f64;
            pm1[l] = pf[l] - 1.0;
            inv[l] = 1.0 / pf[l];
            let (mut gs, mut ls) = (0.0, 0.0);
            for i in 1..start {
                let ch = x[i] - x[i - 1];
                let (g, lo) = if ch > 0.0 { (ch, 0.0) } else { (0.0, ch.abs()) };
                if i < p {
                    gs += g;
                    ls += lo;
                    continue;
                } else if i == p {
                    ag[l] = (gs + g) / pf[l];
                    al[l] = (ls + lo) / pf[l];
                } else if RECIP {
                    ag[l] = (ag[l] * pm1[l] + g) * inv[l];
                    al[l] = (al[l] * pm1[l] + lo) * inv[l];
                } else {
                    ag[l] = (ag[l] * pm1[l] + g) / pf[l];
                    al[l] = (al[l] * pm1[l] + lo) / pf[l];
                }
                let v = if al[l] == 0.0 {
                    if ag[l] == 0.0 {
                        50.0
                    } else {
                        100.0
                    }
                } else {
                    100.0 - 100.0 / (1.0 + ag[l] / al[l])
                };
                if l < l_n {
                    out[c * 4 + l][i] = v;
                }
            }
        }
        for i in start..n {
            let ch = x[i] - x[i - 1];
            let (g, lo) = if ch > 0.0 { (ch, 0.0) } else { (0.0, ch.abs()) };
            let mut v = [0.0; 4];
            for l in 0..4 {
                let (a, b) = if RECIP {
                    (
                        (ag[l] * pm1[l] + g) * inv[l],
                        (al[l] * pm1[l] + lo) * inv[l],
                    )
                } else {
                    ((ag[l] * pm1[l] + g) / pf[l], (al[l] * pm1[l] + lo) / pf[l])
                };
                ag[l] = a;
                al[l] = b;
                let r = 100.0 - 100.0 / (1.0 + a / b);
                let z = if a == 0.0 { 50.0 } else { 100.0 };
                v[l] = if b == 0.0 { z } else { r };
            }
            for l in 0..l_n {
                out[c * 4 + l][i] = v[l];
            }
        }
    }
    out
}

// --- SMA: rolling sum vs prefix sums vs integer cents ---------------------------------

fn sma_rolling(x: &[f64], periods: &[usize]) -> Vec<Vec<f64>> {
    let n = x.len();
    periods
        .iter()
        .map(|&p| {
            let mut o = vec![NAN; n];
            let pf = p as f64;
            let mut sum = 0.0;
            for i in 0..n {
                if i + 1 >= p {
                    let s = sum + x[i];
                    o[i] = s / pf;
                    sum = s - x[i + 1 - p];
                } else {
                    sum += x[i];
                }
            }
            o
        })
        .collect()
}

fn sma_prefix(x: &[f64], periods: &[usize]) -> Vec<Vec<f64>> {
    let n = x.len();
    let mut pre = vec![0.0; n + 1];
    for i in 0..n {
        pre[i + 1] = pre[i] + x[i];
    }
    periods
        .iter()
        .map(|&p| {
            let mut o = vec![NAN; n];
            let pf = p as f64;
            for i in p.saturating_sub(1)..n {
                o[i] = (pre[i + 1] - pre[i + 1 - p]) / pf;
            }
            o
        })
        .collect()
}

fn sma_cents(x: &[f64], periods: &[usize]) -> Vec<Vec<f64>> {
    let n = x.len();
    let c: Vec<i64> = x.iter().map(|v| (v * 100.0).round() as i64).collect();
    periods
        .iter()
        .map(|&p| {
            let mut o = vec![NAN; n];
            let d = p as f64 * 100.0;
            let mut sum = 0i64;
            for i in 0..n {
                sum += c[i];
                if i + 1 >= p {
                    o[i] = sum as f64 / d;
                    sum -= c[i + 1 - p];
                }
            }
            o
        })
        .collect()
}

// --- f32 ------------------------------------------------------------------------------

fn ema_f32(x: &[f64], periods: &[usize]) -> Vec<Vec<f64>> {
    let xf: Vec<f32> = x.iter().map(|&v| v as f32).collect();
    let n = x.len();
    let mut out = vec![vec![NAN; n]; periods.len()];
    for (c, chunk) in periods.chunks(8).enumerate() {
        let l_n = chunk.len();
        let mut prev = [0f32; 8];
        let mut a = [0f32; 8];
        let maxp = *chunk.iter().max().unwrap();
        for l in 0..l_n {
            let p = chunk[l];
            a[l] = 2.0 / (p as f32 + 1.0);
            let mut s = 0f32;
            for i in 0..maxp.min(n) {
                if i + 1 < p {
                    s += xf[i];
                } else if i + 1 == p {
                    prev[l] = (s + xf[i]) / p as f32;
                    out[c * 8 + l][i] = f64::from(prev[l]);
                } else {
                    prev[l] = xf[i] * a[l] + prev[l] * (1.0 - a[l]);
                    out[c * 8 + l][i] = f64::from(prev[l]);
                }
            }
        }
        for i in maxp..n {
            let mut v = [0f32; 8];
            for l in 0..8 {
                v[l] = xf[i] * a[l] + prev[l] * (1.0 - a[l]);
                prev[l] = v[l];
            }
            for l in 0..l_n {
                out[c * 8 + l][i] = f64::from(v[l]);
            }
        }
    }
    out
}

// --- late start -----------------------------------------------------------------------

fn late_start(name: &str, period: usize, x: &[f64], window: usize, warm: usize) -> f64 {
    let n = x.len();
    let full = reference(name, &[period], x);
    let s = n - window - warm;
    let part = reference(name, &[period], &x[s..]);
    let mut worst: f64 = 0.0;
    for i in n - window..n {
        let (a, b) = (part[0][i - s], full[0][i]);
        if a.is_nan() || b.is_nan() {
            return f64::INFINITY;
        }
        worst = worst.max((a - b).abs() / b.abs().max(1e-300));
    }
    worst
}

fn main() {
    let reps = reps();
    let x = a_series();
    let n = x.len();
    let periods: Vec<usize> = (2..=1001).collect();
    println!(
        "machine: {}-{}",
        std::env::consts::ARCH,
        std::env::consts::OS
    );
    println!(
        "10,000 bars x 1,000 periods; times = median ms of {reps} runs; differences vs the library"
    );
    println!(
        "{:<28} {:>8} {:>8} {:>7}  {:>7} {:>9} {:>9} {:>8}  digest",
        "probe", "cur ms", "new ms", "speed", "differ", "max abs", "max rel", "max ulp"
    );

    let want = reference("ema", &periods, &x);
    let t_cur = median_ms(reps, || ema_lanes::<false>(&x, &periods)[999][n - 1]);
    let t_new = median_ms(reps, || ema_lanes::<true>(&x, &periods)[999][n - 1]);
    let cur = ema_lanes::<false>(&x, &periods);
    let d0 = diff(&cur, &want, 0);
    assert_eq!(
        d0.differ, 0,
        "probe's current-definition EMA must match the library"
    );
    row(
        "7.1 ema fma",
        t_cur,
        t_new,
        &diff(&ema_lanes::<true>(&x, &periods), &want, 0),
    );

    let want = reference("rsi", &periods, &x);
    let t_cur = median_ms(reps, || rsi_lanes::<false>(&x, &periods)[999][n - 1]);
    let t_new = median_ms(reps, || rsi_lanes::<true>(&x, &periods)[999][n - 1]);
    let d0 = diff(&rsi_lanes::<false>(&x, &periods), &want, 0);
    assert_eq!(
        d0.differ, 0,
        "probe's current-definition RSI must match the library"
    );
    row(
        "7.2 rsi reciprocal",
        t_cur,
        t_new,
        &diff(&rsi_lanes::<true>(&x, &periods), &want, 0),
    );

    let want = reference("sma", &periods, &x);
    let t_cur = median_ms(reps, || sma_rolling(&x, &periods)[999][n - 1]);
    let d0 = diff(&sma_rolling(&x, &periods), &want, 0);
    assert_eq!(d0.differ, 0, "probe's rolling SMA must match the library");
    let t_new = median_ms(reps, || sma_prefix(&x, &periods)[999][n - 1]);
    row(
        "7.3 sma prefix sums (10k)",
        t_cur,
        t_new,
        &diff(&sma_prefix(&x, &periods), &want, 0),
    );
    let t_new = median_ms(reps, || sma_cents(&x, &periods)[999][n - 1]);
    row(
        "7.6 sma integer cents",
        t_cur,
        t_new,
        &diff(&sma_cents(&x, &periods), &want, 0),
    );

    // Long series: prefix-sum error growth (5 periods, 1M bars).
    let big = bench_closes(1_000_000, 99);
    let few = [2, 20, 200, 1000, 5000];
    let want_big = reference("sma", &few, &big);
    let t_cur = median_ms(reps.min(3), || sma_rolling(&big, &few)[0][1]);
    let t_new = median_ms(reps.min(3), || sma_prefix(&big, &few)[0][1]);
    row(
        "7.3 sma prefix sums (1M)",
        t_cur,
        t_new,
        &diff(&sma_prefix(&big, &few), &want_big, 0),
    );
    let t_new = median_ms(reps.min(3), || sma_cents(&big, &few)[0][1]);
    row(
        "7.6 sma integer cents (1M)",
        t_cur,
        t_new,
        &diff(&sma_cents(&big, &few), &want_big, 0),
    );

    let want = reference("ema", &periods, &x);
    let t_cur = median_ms(reps, || ema_lanes::<false>(&x, &periods)[999][n - 1]);
    let t_new = median_ms(reps, || ema_f32(&x, &periods)[999][n - 1]);
    row(
        "7.4 ema f32 (8 lanes)",
        t_cur,
        t_new,
        &diff(&ema_f32(&x, &periods), &want, 0),
    );

    println!();
    println!("7.5 late start: max relative error in the last 500 bars when the run starts `warm` bars before them");
    println!(
        "{:<10} {:>6} {:>10} {:>10} {:>10} {:>10} {:>10}",
        "study", "period", "w=p+1", "w=50", "w=200", "w=1000", "w=5000"
    );
    for (name, p) in [
        ("ema", 20),
        ("ema", 200),
        ("rsi", 14),
        ("rsi", 100),
        ("sma", 20),
    ] {
        let e: Vec<String> = [0, 50, 200, 1000, 5000]
            .iter()
            .map(|&w| format!("{:>10.2e}", late_start(name, p, &x, 500, w.max(p + 1))))
            .collect();
        println!("{name:<10} {p:>6} {}", e.join(" "));
    }
}
