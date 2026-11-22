//! RP1/E2: the structure-of-arrays lane kernels are bit-identical to single runs, for any
//! parameter list (shuffled, duplicated, odd counts), any lane count, series shorter than
//! the warmup, flat stretches and zero prices; `sweep_series_each` = one run per series.

mod support;

use std::collections::BTreeMap;

use qp_compute::sweep::{bench_closes, sweep_each_lanes, sweep_series_each, SERIES_WINDOW};
use qp_compute::STUDY_NAMES;

fn bits(v: &[Option<f64>]) -> Vec<Option<u64>> {
    v.iter().map(|x| x.map(f64::to_bits)).collect()
}

struct Lcg(u64);
impl Lcg {
    fn next(&mut self, n: u64) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 33) % n
    }
}

fn random_params(name: &str, rng: &mut Lcg) -> BTreeMap<String, f64> {
    let p = |rng: &mut Lcg, hi: u64| (1 + rng.next(hi)) as f64;
    match name {
        "macd" => {
            let fast = p(rng, 30);
            let slow = fast + p(rng, 40);
            BTreeMap::from([
                ("fast".to_owned(), fast),
                ("slow".to_owned(), slow),
                ("signal".to_owned(), p(rng, 30)),
            ])
        }
        "bollinger" => BTreeMap::from([
            ("period".to_owned(), p(rng, 60)),
            ("k".to_owned(), 0.25 * p(rng, 16)),
        ]),
        _ => BTreeMap::from([("period".to_owned(), p(rng, 80))]),
    }
}

/// A series with random walk, flat stretches, a run of zeros and a jump.
fn awkward_series(n: usize, seed: u64) -> Vec<f64> {
    let mut x = bench_closes(n, seed);
    for i in 0..n {
        if (i / 37) % 5 == 2 {
            x[i] = x[i - i % 37];
        }
        if (i / 50) % 9 == 4 {
            x[i] = 0.0;
        }
    }
    x
}

fn single(name: &str, p: &BTreeMap<String, f64>, x: &[f64]) -> Vec<Vec<Option<f64>>> {
    let kv: Vec<(&str, f64)> = p.iter().map(|(k, v)| (k.as_str(), *v)).collect();
    support::batch(name, &kv, x)
}

#[test]
fn lane_kernels_equal_single_runs_on_random_parameter_lists() {
    let mut rng = Lcg(2026);
    for (n, seed) in [(0, 1), (1, 2), (7, 3), (120, 4), (900, 5)] {
        let x = awkward_series(n, seed);
        for name in STUDY_NAMES {
            let mut grid: Vec<BTreeMap<String, f64>> =
                (0..23).map(|_| random_params(name, &mut rng)).collect();
            grid.push(grid[3].clone()); // a duplicate
            let singles: Vec<_> = grid.iter().map(|p| single(name, p, &x)).collect();
            for lanes in [None, Some(1), Some(2), Some(3), Some(4), Some(8)] {
                let mut seen = vec![false; grid.len()];
                sweep_each_lanes(name, &grid, &x, lanes, |vi, cols| {
                    assert!(!seen[vi], "variant {vi} visited twice");
                    seen[vi] = true;
                    assert_eq!(cols.columns.len(), singles[vi].len());
                    for (j, s) in singles[vi].iter().enumerate() {
                        assert_eq!(
                            bits(&cols.to_options(j)),
                            bits(s),
                            "{name} n={n} variant {vi} {:?} output {j} lanes {lanes:?}",
                            grid[vi]
                        );
                    }
                })
                .unwrap();
                assert!(seen.iter().all(|&s| s), "{name}: not every variant visited");
            }
        }
    }
}

#[test]
fn series_lanes_equal_single_runs_in_series_order() {
    let mut rng = Lcg(77);
    // More than one window, mixed lengths (incl. empty and shorter than warmup).
    let lens = [300, 0, 300, 5, 1200, 300, 300, 1200, 41, 300];
    let series: Vec<Vec<f64>> = (0..SERIES_WINDOW + 37)
        .map(|i| awkward_series(lens[i % lens.len()], 100 + i as u64))
        .collect();
    let xs: Vec<&[f64]> = series.iter().map(Vec::as_slice).collect();
    for name in STUDY_NAMES {
        for _ in 0..3 {
            let p = random_params(name, &mut rng);
            let mut next = 0;
            sweep_series_each(name, &p, &xs, |si, cols| {
                assert_eq!(si, next, "{name}: series visited out of order");
                next += 1;
                for (j, s) in single(name, &p, xs[si]).iter().enumerate() {
                    assert_eq!(
                        bits(&cols.to_options(j)),
                        bits(s),
                        "{name} {p:?} series {si} output {j}"
                    );
                }
            })
            .unwrap();
            assert_eq!(next, xs.len());
        }
    }
    assert!(sweep_series_each("nope", &BTreeMap::new(), &xs, |_, _| {}).is_err());
    let bad = BTreeMap::from([("period".to_owned(), 0.0)]);
    assert!(sweep_series_each("rsi", &bad, &xs, |_, _| panic!("no work")).is_err());
}
