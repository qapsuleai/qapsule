//! Sweep = individual runs, bit for bit, for every study and every lane count; and the
//! accuracy of the rolling Bollinger variance on a long series.

mod support;

use std::collections::BTreeMap;

use qp_compute::sweep::{bench_closes, bench_grid, sweep, sweep_each_lanes};
use qp_compute::STUDY_NAMES;

fn bits(v: &[Option<f64>]) -> Vec<Option<u64>> {
    v.iter().map(|x| x.map(f64::to_bits)).collect()
}

#[test]
fn sweep_equals_single_runs_for_every_lane_count() {
    let x = bench_closes(700, 99);
    for name in STUDY_NAMES {
        // A slice of the benchmark grid with an odd count, so partial lane chunks run too.
        let grid: Vec<BTreeMap<String, f64>> = bench_grid(name).into_iter().step_by(37).collect();
        let singles: Vec<Vec<Vec<Option<f64>>>> = grid
            .iter()
            .map(|p| {
                let kv: Vec<(&str, f64)> = p.iter().map(|(k, v)| (k.as_str(), *v)).collect();
                support::batch(name, &kv, &x)
            })
            .collect();
        for lanes in [None, Some(1), Some(2), Some(3), Some(4), Some(8)] {
            let mut seen = 0;
            sweep_each_lanes(name, &grid, &x, lanes, |vi, cols| {
                for (j, single) in singles[vi].iter().enumerate() {
                    assert_eq!(
                        bits(&cols.to_options(j)),
                        bits(single),
                        "{name} variant {vi} output {j} lanes {lanes:?}"
                    );
                }
                seen += 1;
            })
            .unwrap();
            assert_eq!(seen, grid.len());
        }
    }
}

#[test]
fn sweep_rejects_invalid_sets_before_work() {
    let x = bench_closes(50, 1);
    let bad = vec![
        BTreeMap::from([("period".to_owned(), 5.0)]),
        BTreeMap::from([("period".to_owned(), 0.0)]),
    ];
    assert!(sweep("sma", &bad, &x).is_err());
    let bad_macd = vec![BTreeMap::from([
        ("fast".to_owned(), 26.0),
        ("slow".to_owned(), 12.0),
    ])];
    assert!(sweep("macd", &bad_macd, &x).is_err());
    assert!(sweep("nope", &[], &x).is_err());
}

#[test]
#[allow(clippy::needless_range_loop)]
fn flat_windows_are_exact_at_real_price_levels() {
    let mut x = bench_closes(300, 5);
    let level = 123.45;
    x.extend(std::iter::repeat_n(level, 40));
    x.extend(bench_closes(100, 6).iter().map(|v| v + 20.0));
    for p in [2.0, 5.0, 20.0] {
        let out = support::batch("bollinger", &[("period", p), ("k", 2.0)], &x);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let first_flat = 300 + p as usize - 1;
        for i in first_flat..340 {
            // The band width is exactly zero; the middle (a rolling-sum SMA, like the
            // legacy one) may carry a ~1e-13 residue from earlier bars.
            let m = out[0][i].unwrap();
            assert!(
                support::close(m, level, support::REL_TOL),
                "middle p={p} i={i}: {m}"
            );
            assert_eq!(out[1][i], Some(m), "upper p={p} i={i}");
            assert_eq!(out[2][i], Some(m), "lower p={p} i={i}");
            assert_eq!(out[3][i], Some(0.0), "bandwidth p={p} i={i}");
        }
    }
}

/// Textbook windowed Welford (mean and M2 both rolled), for comparison only.
fn welford_std(x: &[f64], p: usize) -> Vec<f64> {
    let (mut mean, mut m2) = (0.0_f64, 0.0_f64);
    let mut out = vec![f64::NAN; x.len()];
    for i in 0..x.len() {
        let n = (i.min(p - 1) + 1) as f64;
        let d = x[i] - mean;
        mean += d / n;
        m2 += d * (x[i] - mean);
        if i + 1 >= p {
            out[i] = (m2 / p as f64).max(0.0).sqrt();
            if p > 1 {
                let y = x[i + 1 - p];
                let d = y - mean;
                let mean_after = mean - d / (p - 1) as f64;
                m2 -= d * (y - mean_after);
                mean = mean_after;
            } else {
                mean = 0.0;
                m2 = 0.0;
            }
        }
    }
    out
}

#[test]
fn rolling_variance_accuracy_on_long_series() {
    // 10,000-bar benchmark series (prices 88..317); compare std with a two-pass
    // computation per window.
    let x = bench_closes(10_000, 20_261_114);
    let (mut worst_abs, mut worst_rel_price, mut worst_welford) = (0.0_f64, 0.0_f64, 0.0_f64);
    for p in [2usize, 5, 20, 50, 101] {
        let out = support::batch("bollinger", &[("period", p as f64), ("k", 1.0)], &x);
        let wf = welford_std(&x, p);
        for i in p - 1..x.len() {
            let w = &x[i + 1 - p..=i];
            let n = p as f64;
            let mean = w.iter().sum::<f64>() / n;
            let sd = (w.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / n).sqrt();
            let ours = out[1][i].unwrap() - out[0][i].unwrap();
            worst_abs = worst_abs.max((ours - sd).abs());
            worst_rel_price = worst_rel_price.max((ours - sd).abs() / mean);
            worst_welford = worst_welford.max((wf[i] - sd).abs());
        }
    }
    println!(
        "std error vs two-pass over 10k bars: ours max abs {worst_abs:e} (rel. to price {worst_rel_price:e}); textbook Welford max abs {worst_welford:e}"
    );
    // Relative to the price level (the bands are price-sized), see `support::BAND_REL_TOL`.
    // The rolled M2 drifts with the series length, hence the looser bound for 10k bars.
    assert!(worst_rel_price < 1e-9, "{worst_rel_price:e}");
}

/// Random walk in cents around `level` (deterministic LCG, steps up to ±0.2%).
fn walk(n: usize, level: f64, seed: u64) -> Vec<f64> {
    let mut s = seed;
    let mut x = level;
    (0..n)
        .map(|_| {
            s = s
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let u = f64::from(u32::try_from(s >> 32).unwrap()) / 2_147_483_648.0 - 1.0;
            x = ((x * (1.0 + 0.002 * u)) * 100.0).round() / 100.0;
            x
        })
        .collect()
}

/// R1 audit finding 5: at BTC-like prices an absolute 1e-9 is a statement about the price
/// scale, not about accuracy. 2,000 bars at ~50,000 and ~100,000: the SMA / middle band
/// meet `REL_TOL`, the outer bands `BAND_REL_TOL`, while their absolute error is well
/// above the old 1e-9 (asserted, so the case keeps demonstrating it).
#[test]
fn high_price_levels_meet_the_relative_tolerance() {
    for (level, seed) in [(50_000.0, 7u64), (100_000.0, 11)] {
        let x = walk(2_000, level, seed);
        let (mut mid_abs, mut mid_rel, mut band_abs, mut band_rel) = (0.0_f64, 0.0, 0.0, 0.0);
        for p in [2usize, 3, 5, 20, 50, 101] {
            let sma = support::batch("sma", &[("period", p as f64)], &x);
            let bb = support::batch("bollinger", &[("period", p as f64), ("k", 2.0)], &x);
            for i in p - 1..x.len() {
                let w = &x[i + 1 - p..=i];
                let n = p as f64;
                let mean = w.iter().sum::<f64>() / n;
                let sd = (w.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / n).sqrt();
                let rel = |a: f64, b: f64| (a - b).abs() / a.abs().max(b.abs()).max(1.0);
                for ours in [sma[0][i].unwrap(), bb[0][i].unwrap()] {
                    assert!(
                        support::close(ours, mean, support::REL_TOL),
                        "middle, level {level} p={p} i={i}: {ours} vs {mean}"
                    );
                    mid_abs = f64::max(mid_abs, (ours - mean).abs());
                    mid_rel = f64::max(mid_rel, rel(ours, mean));
                }
                for (ours, want) in [
                    (bb[1][i].unwrap(), mean + 2.0 * sd),
                    (bb[2][i].unwrap(), mean - 2.0 * sd),
                ] {
                    assert!(
                        support::close(ours, want, support::BAND_REL_TOL),
                        "band, level {level} p={p} i={i}: {ours} vs {want}"
                    );
                    band_abs = f64::max(band_abs, (ours - want).abs());
                    band_rel = f64::max(band_rel, rel(ours, want));
                }
            }
        }
        println!(
            "level {level}: middle max abs {mid_abs:e} rel {mid_rel:e}; bands max abs {band_abs:e} rel {band_rel:e}"
        );
        assert!(
            band_abs > 1e-9,
            "the old absolute 1e-9 check would pass: {band_abs:e}"
        );
    }
}
