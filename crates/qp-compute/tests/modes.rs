//! Property tests (fixed seed, reproducible):
//! - batch = streaming = "revise the last bar", bit for bit;
//! - output length = bar count; warmup values are `None`, all later values finite.

mod common;

use proptest::prelude::*;
use proptest::test_runner::RngSeed;
use qp_compute::{run, Study};

fn prices() -> impl Strategy<Value = Vec<f64>> {
    // Finite positive prices with two decimals, like real quotes, plus some flat runs.
    prop::collection::vec((1u32..=2_000_000u32, 0u8..4), 0..120).prop_map(|v| {
        let mut out = Vec::with_capacity(v.len());
        for (cents, rep) in v {
            let x = f64::from(cents) / 100.0;
            for _ in 0..=u32::from(rep % 2) {
                out.push(x);
            }
        }
        out
    })
}

fn spec() -> impl Strategy<Value = (&'static str, Vec<(&'static str, f64)>)> {
    let p = || 1u32..40;
    prop_oneof![
        p().prop_map(|n| ("sma", vec![("period", f64::from(n))])),
        p().prop_map(|n| ("ema", vec![("period", f64::from(n))])),
        p().prop_map(|n| ("rsi", vec![("period", f64::from(n))])),
        (1u32..20, 1u32..20, p()).prop_map(|(f, d, s)| (
            "macd",
            vec![
                ("fast", f64::from(f)),
                ("slow", f64::from(f + d)),
                ("signal", f64::from(s))
            ]
        )),
        (p(), 1u32..40).prop_map(|(n, k)| (
            "bollinger",
            vec![("period", f64::from(n)), ("k", f64::from(k) / 10.0)]
        )),
    ]
}

fn bits(v: &[Vec<Option<f64>>]) -> Vec<Vec<Option<u64>>> {
    v.iter()
        .map(|c| c.iter().map(|x| x.map(f64::to_bits)).collect())
        .collect()
}

/// Streaming run where every bar first arrives with wrong prices, is revised a few
/// times, and is finally revised to its true price.
fn revised(study: &mut dyn Study, x: &[f64], noise: &[f64]) -> Vec<Vec<Option<f64>>> {
    study.reset();
    let n_out = study.outputs().len();
    let mut outs = vec![Vec::new(); n_out];
    let mut o = vec![None; n_out];
    for (i, &xi) in x.iter().enumerate() {
        let a = noise[i % noise.len()];
        let b = noise[(i * 7 + 3) % noise.len()];
        study.push(&[xi * a], &mut o);
        study.update_last(&[xi * b], &mut o);
        study.update_last(&[xi + a], &mut o);
        study.update_last(&[xi], &mut o);
        for (c, v) in outs.iter_mut().zip(&o) {
            c.push(*v);
        }
    }
    outs
}

fn streamed(study: &mut dyn Study, x: &[f64]) -> Vec<Vec<Option<f64>>> {
    study.reset();
    let n_out = study.outputs().len();
    let mut outs = vec![Vec::new(); n_out];
    let mut o = vec![None; n_out];
    for &xi in x {
        study.push(&[xi], &mut o);
        for (c, v) in outs.iter_mut().zip(&o) {
            c.push(*v);
        }
    }
    outs
}

proptest! {
    #![proptest_config(ProptestConfig {
        rng_seed: RngSeed::Fixed(0x0051_2026),
        failure_persistence: None,
        cases: 512,
        ..ProptestConfig::default()
    })]

    #[test]
    fn three_modes_are_bit_identical(
        x in prices(),
        (name, kv) in spec(),
        noise in prop::collection::vec(0.5f64..1.5, 1..8),
    ) {
        let mut s = common::study(name, &kv);
        let batch = run(s.as_mut(), &[&x]).unwrap();
        let stream = streamed(s.as_mut(), &x);
        let rev = revised(s.as_mut(), &x, &noise);
        prop_assert_eq!(bits(&batch), bits(&stream), "batch vs streaming");
        prop_assert_eq!(bits(&batch), bits(&rev), "batch vs revised");
    }

    #[test]
    fn length_warmup_and_finiteness(x in prices(), (name, kv) in spec()) {
        let mut s = common::study(name, &kv);
        let outs = run(s.as_mut(), &[&x]).unwrap();
        let warm = s.warmup();
        prop_assert_eq!(outs.len(), s.outputs().len());
        for (col, w) in outs.iter().zip(warm) {
            prop_assert_eq!(col.len(), x.len());
            for (i, v) in col.iter().enumerate() {
                if i < w {
                    prop_assert!(v.is_none(), "{} index {} inside warmup {} is {:?}", name, i, w, v);
                } else {
                    prop_assert!(v.is_some_and(f64::is_finite), "{} index {} after warmup {} is {:?}", name, i, w, v);
                }
            }
        }
    }
}

/// Naive O(n * p) reference: SMA and two-pass population std over each full window.
fn naive_sma_std(x: &[f64], p: usize) -> Vec<Option<(f64, f64)>> {
    (0..x.len())
        .map(|i| {
            if i + 1 < p {
                return None;
            }
            let w = &x[i + 1 - p..=i];
            let n = w.len() as f64;
            let mean = w.iter().sum::<f64>() / n;
            let var = w.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / n;
            Some((mean, var.sqrt()))
        })
        .collect()
}

proptest! {
    #![proptest_config(ProptestConfig {
        rng_seed: RngSeed::Fixed(0x0051_2027),
        failure_persistence: None,
        cases: 256,
        ..ProptestConfig::default()
    })]

    /// Rolling sums (SMA) and the windowed Welford variance (Bollinger) match a
    /// from-scratch computation within a *relative* tolerance (`common::REL_TOL`,
    /// `common::BAND_REL_TOL`), also on long series far from zero (levels up to 1e6).
    #[test]
    fn rolling_state_matches_naive_reference(
        steps in prop::collection::vec(-300i32..300, 1..1500),
        level in prop::sample::select(vec![1.0, 100.0, 10_000.0, 1_000_000.0]),
        p in 1usize..60,
    ) {
        let mut acc = 0i64;
        let x: Vec<f64> = steps
            .iter()
            .map(|s| {
                acc += i64::from(*s);
                level + (acc as f64) / 100.0
            })
            .collect();
        let want = naive_sma_std(&x, p);
        let sma = common::batch("sma", &[("period", p as f64)], &x);
        let bb = common::batch("bollinger", &[("period", p as f64), ("k", 1.0)], &x);
        for i in 0..x.len() {
            match want[i] {
                None => prop_assert!(sma[0][i].is_none() && bb[1][i].is_none()),
                Some((m, sd)) => {
                    let s = sma[0][i].unwrap();
                    prop_assert!(common::close(s, m, common::REL_TOL), "sma[{}] {} vs {}", i, s, m);
                    let mid = bb[0][i].unwrap();
                    prop_assert!(common::close(mid, m, common::REL_TOL), "middle[{}] {} vs {}", i, mid, m);
                    // The std itself is tiny next to the price; its error is relative to
                    // the price level, so compare the price-sized bands (k = 1).
                    let (up, lo) = (bb[1][i].unwrap(), bb[2][i].unwrap());
                    prop_assert!(common::close(up, m + sd, common::BAND_REL_TOL), "upper[{}] {} vs {}", i, up, m + sd);
                    prop_assert!(common::close(lo, m - sd, common::BAND_REL_TOL), "lower[{}] {} vs {}", i, lo, m - sd);
                }
            }
        }
    }
}

/// Test-only study whose `None` depends on the value, not on the bar count: `gate(level)`
/// passes the input through when it is `>= level` and outputs `None` otherwise. Revising
/// a bar can therefore turn a defined value into `None` and back (R1 audit finding 4).
#[derive(Clone)]
struct Gate(f64);

impl qp_compute::Core for Gate {
    const NAME: &'static str = "gate";
    const OUTPUTS: &'static [&'static str] = &["gate"];
    const N_IN: usize = 1;
    fn params(&self) -> std::collections::BTreeMap<String, f64> {
        std::collections::BTreeMap::from([("level".to_owned(), self.0)])
    }
    fn warmup(&self) -> Vec<usize> {
        vec![0]
    }
    fn peek(&self, x: &[f64], out: &mut [Option<f64>]) {
        out[0] = (x[0] >= self.0).then_some(x[0]);
    }
    fn commit(&mut self, _x: &[f64]) {}
    fn reset(&mut self) {}
}

type Spec = (&'static str, Vec<(&'static str, f64)>);

/// First stage of a chain: the value-dependent gate (`Some(level)`) or a built-in study
/// (whose `None` is index-based).
fn first_spec() -> impl Strategy<Value = (Option<f64>, Spec)> {
    prop_oneof![
        (1u32..20_000).prop_map(|l| (Some(f64::from(l)), ("", vec![]))),
        spec().prop_map(|s| (None, s)),
    ]
}

fn first_study(gate: Option<f64>, name: &str, kv: &[(&str, f64)]) -> Box<dyn Study> {
    match gate {
        Some(level) => Box::new(qp_compute::Streaming::new(
            Gate(level),
            qp_compute::Field::Close,
        )),
        None => common::study(name, kv),
    }
}

/// Reference for `second(first.output[which])`: run `first` in batch, feed only its
/// defined values to `second` in batch, scatter the results back to those bars.
fn chain_reference(
    first: &mut dyn Study,
    which: usize,
    second: &mut dyn Study,
    x: &[f64],
) -> Vec<Vec<Option<f64>>> {
    let f = run(first, &[x]).unwrap();
    let col = &f[which];
    let defined: Vec<f64> = col.iter().flatten().copied().collect();
    let s = run(second, &[&defined]).unwrap();
    s.iter()
        .map(|sc| {
            let mut it = sc.iter();
            col.iter()
                .map(|v| v.and_then(|_| *it.next().unwrap()))
                .collect()
        })
        .collect()
}

proptest! {
    #![proptest_config(ProptestConfig {
        rng_seed: RngSeed::Fixed(0x0051_2028),
        failure_persistence: None,
        cases: 512,
        ..ProptestConfig::default()
    })]

    /// `Chain`: batch = streaming = "revise the last bar" = reference, bit for bit, also
    /// when revisions turn the first study's output from a value into `None` and back.
    #[test]
    fn chain_modes_are_bit_identical(
        x in prices(),
        (gate, (fname, fkv)) in first_spec(),
        which_seed in 0usize..4,
        (sname, skv) in spec(),
        noise in prop::collection::vec(0.5f64..1.5, 1..8),
    ) {
        let first = first_study(gate, fname, &fkv);
        let which = which_seed % first.outputs().len();
        let mut chain = qp_compute::Chain::new(first, which, common::study(sname, &skv));
        let want = chain_reference(
            first_study(gate, fname, &fkv).as_mut(),
            which,
            common::study(sname, &skv).as_mut(),
            &x,
        );
        let batch = run(&mut chain, &[&x]).unwrap();
        let stream = streamed(&mut chain, &x);
        let rev = revised(&mut chain, &x, &noise);
        prop_assert_eq!(bits(&batch), bits(&want), "batch vs reference");
        prop_assert_eq!(bits(&batch), bits(&stream), "batch vs streaming");
        prop_assert_eq!(bits(&batch), bits(&rev), "batch vs revised");
    }
}
