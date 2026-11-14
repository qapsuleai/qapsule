//! Shared sub-computations in the sweep (RP1/E1): runs of variants with an equal shared
//! key take a different code path; every output must still equal a single run bit for
//! bit, in any list order, with duplicates, mixed runs and singles, short series, and for
//! every lane count.

mod common;

use std::collections::BTreeMap;

use qp_compute::sweep::{bench_closes, bench_grid, sweep, sweep_each_lanes};

type P = BTreeMap<String, f64>;

fn bits(v: &[Option<f64>]) -> Vec<Option<u64>> {
    v.iter().map(|x| x.map(f64::to_bits)).collect()
}

fn p(kv: &[(&str, f64)]) -> P {
    common::params(kv)
}

/// Deterministic Fisher–Yates shuffle (integer LCG).
fn shuffled(mut v: Vec<P>, seed: u64) -> Vec<P> {
    let mut s = seed;
    for i in (1..v.len()).rev() {
        s = s
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        #[allow(clippy::cast_possible_truncation)]
        let j = ((s >> 33) as usize) % (i + 1);
        v.swap(i, j);
    }
    v
}

fn check(name: &str, list: &[P], x: &[f64]) {
    let singles: Vec<Vec<Vec<Option<f64>>>> = list
        .iter()
        .map(|p| {
            let kv: Vec<(&str, f64)> = p.iter().map(|(k, v)| (k.as_str(), *v)).collect();
            common::batch(name, &kv, x)
        })
        .collect();
    for lanes in [None, Some(1), Some(2), Some(3), Some(4), Some(8)] {
        let mut next = 0;
        sweep_each_lanes(name, list, x, lanes, |vi, cols| {
            assert_eq!(vi, next, "{name}: visit order, lanes {lanes:?}");
            next += 1;
            assert_eq!(cols.columns.len(), singles[vi].len());
            for (j, single) in singles[vi].iter().enumerate() {
                assert_eq!(cols.columns[j].len(), x.len());
                assert_eq!(
                    bits(&cols.to_options(j)),
                    bits(single),
                    "{name} variant {vi} {:?} output {j} lanes {lanes:?} bars {}",
                    list[vi],
                    x.len()
                );
            }
        })
        .unwrap();
        assert_eq!(next, list.len());
    }
    // The materializing form keeps the input order too.
    let all = sweep(name, list, x).unwrap();
    for (vi, cols) in all.iter().enumerate() {
        for (j, single) in singles[vi].iter().enumerate() {
            assert_eq!(
                bits(&cols.to_options(j)),
                bits(single),
                "{name} sweep() {vi}"
            );
        }
    }
}

fn macd(f: f64, s: f64, g: f64) -> P {
    p(&[("fast", f), ("slow", s), ("signal", g)])
}

fn boll(period: f64, k: f64) -> P {
    p(&[("period", period), ("k", k)])
}

fn macd_lists() -> Vec<Vec<P>> {
    let grid = bench_grid("macd");
    let mut mixed = vec![
        macd(3.0, 7.0, 2.0),
        macd(5.0, 9.0, 4.0),
        macd(5.0, 9.0, 1.0),
        macd(5.0, 9.0, 13.0),
        macd(2.0, 30.0, 9.0),
    ];
    // A run of 11 (odd, longer than every lane count), then singles, then a duplicate run.
    mixed.extend((1..=11).map(|g| macd(12.0, 26.0, f64::from(g))));
    mixed.push(macd(1.0, 2.0, 1.0));
    mixed.push(macd(12.0, 26.0, 9.0));
    mixed.extend(std::iter::repeat_n(macd(4.0, 6.0, 3.0), 3));
    mixed.push(macd(1.0, 2.0, 1.0));
    vec![
        grid[..47].to_vec(),
        grid[480..600].to_vec(),
        shuffled(grid[..200].to_vec(), 3),
        mixed,
        // Defaults filled in: only `signal` given, the rest default.
        vec![p(&[("signal", 3.0)]), p(&[("signal", 5.0)]), p(&[])],
    ]
}

fn bollinger_lists() -> Vec<Vec<P>> {
    let grid = bench_grid("bollinger");
    let mut mixed = vec![boll(1.0, 2.0), boll(1.0, 0.5), boll(1.0, 2.0)];
    mixed.push(boll(7.0, 1.5));
    mixed.extend((1..=9).map(|k| boll(20.0, 0.3 * f64::from(k))));
    mixed.push(boll(3.0, 1.0));
    mixed.extend(std::iter::repeat_n(boll(2.0, 2.0), 4));
    mixed.push(boll(500.0, 1.0));
    mixed.push(boll(500.0, 3.0));
    vec![
        grid[..37].to_vec(),
        grid[945..].to_vec(),
        shuffled(grid[..300].to_vec(), 11),
        mixed,
        vec![p(&[("k", 1.0)]), p(&[]), p(&[("k", 3.0)])],
    ]
}

#[test]
fn macd_shared_line_equals_single_runs() {
    let x = bench_closes(400, 21);
    for list in macd_lists() {
        check("macd", &list, &x);
    }
}

#[test]
fn bollinger_shared_std_equals_single_runs() {
    let mut x = bench_closes(400, 22);
    // A flat stretch and a zero middle exercise the exact-flat and bandwidth branches.
    x.extend(std::iter::repeat_n(50.0, 30));
    x.extend(std::iter::repeat_n(0.0, 25));
    x.extend(bench_closes(60, 23));
    for list in bollinger_lists() {
        check("bollinger", &list, &x);
    }
}

#[test]
fn shared_paths_on_series_shorter_than_periods() {
    for n in [0, 1, 2, 5, 12, 27, 31] {
        let x = bench_closes(n, 40 + n as u64);
        for list in macd_lists() {
            check("macd", &list, &x);
        }
        for list in bollinger_lists() {
            check("bollinger", &list, &x);
        }
    }
}
