//! RL1 kernels (`stdev`, `percentrank`, `docs/PROGRAMS.md` §4.2), in a topic folder so
//! `tests/` stays within its file budget:
//! - hand-computed small examples;
//! - batch = streaming = "revise the last bar" (several wrong revisions) = from-scratch
//!   reference with the same operations in the same order, bit for bit;
//! - warmup, `discard_last`, `extend`, period bounds, the `Chain` rule for undefined
//!   inputs, and the separate digest is stable (older digests unchanged).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::cognitive_complexity
)]

use proptest::prelude::*;
use proptest::test_runner::RngSeed;
use qp_compute::sweep::{bench_closes, cross_target_digest, r3_studies_digest, rl1_studies_digest};
use qp_compute::{
    run, Chain, Field, PercentRank, Stdev, Streaming, Study, MAX_PERIOD, STUDY_NAMES,
};

fn studies(p: usize) -> Vec<Box<dyn Study>> {
    vec![
        Box::new(Streaming::new(Stdev::new(p).unwrap(), Field::Close)),
        Box::new(Streaming::new(PercentRank::new(p).unwrap(), Field::Close)),
    ]
}

fn bits(v: &[Option<f64>]) -> Vec<Option<u64>> {
    v.iter().map(|x| x.map(f64::to_bits)).collect()
}

fn batch(s: &mut dyn Study, x: &[f64]) -> Vec<Option<f64>> {
    run(s, &[x]).unwrap().remove(0)
}

fn revised(s: &mut dyn Study, x: &[f64], noise: &[f64]) -> Vec<Option<f64>> {
    s.reset();
    let mut o = vec![None];
    let mut out = Vec::new();
    for (i, &xi) in x.iter().enumerate() {
        let a = noise[i % noise.len()];
        let b = noise[(i * 7 + 3) % noise.len()];
        s.push(&[xi * a], &mut o);
        s.update_last(&[xi + b], &mut o);
        s.update_last(&[xi * b], &mut o);
        s.update_last(&[xi], &mut o);
        out.push(o[0]);
    }
    out
}

/// From scratch, per bar, with the spec's operations in the spec's order.
fn reference(name: &str, p: usize, x: &[f64]) -> Vec<Option<f64>> {
    let pf = f64::from(u32::try_from(p).unwrap());
    (0..x.len())
        .map(|j| match name {
            "stdev" => (j + 1 >= p).then(|| {
                let w = &x[j + 1 - p..=j];
                let mut s = 0.0;
                for v in w {
                    s += v;
                }
                let m = s / pf;
                let mut q = 0.0;
                for v in w {
                    q += (v - m) * (v - m);
                }
                (q / pf).sqrt()
            }),
            _ => (j >= p).then(|| {
                let n = (1..=p).filter(|k| x[j - k] <= x[j]).count();
                100.0 * f64::from(u32::try_from(n).unwrap()) / pf
            }),
        })
        .collect()
}

fn warmup_of(name: &str, p: usize) -> usize {
    if name == "stdev" {
        p - 1
    } else {
        p
    }
}

#[test]
fn hand_examples() {
    // Classic population example: mean 5, variance 4.
    let x = [2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
    let mut s = Streaming::new(Stdev::new(8).unwrap(), Field::Close);
    let out = batch(&mut s, &x);
    assert_eq!(out[..7], [None; 7]);
    assert_eq!(out[7], Some(2.0));

    // p = 2 over 1, 3, 3, 7: deviations ±1, 0, ±2.
    let mut s = Streaming::new(Stdev::new(2).unwrap(), Field::Close);
    assert_eq!(
        batch(&mut s, &[1.0, 3.0, 3.0, 7.0]),
        vec![None, Some(1.0), Some(0.0), Some(2.0)]
    );

    // p = 1: one value, deviation 0.
    let mut s = Streaming::new(Stdev::new(1).unwrap(), Field::Close);
    assert_eq!(batch(&mut s, &[5.0, -3.0]), vec![Some(0.0), Some(0.0)]);

    // percentrank(3) over 1, 2, 3, 2, 5, 0, 0:
    // j=3: x=2, previous [1,2,3] -> 1,2 are <= 2 -> 2/3; j=4: x=5, [2,3,2] -> 3/3;
    // j=5: x=0, [3,2,5] -> 0; j=6: x=0, [2,5,0] -> the equal 0 counts -> 1/3.
    let mut s = Streaming::new(PercentRank::new(3).unwrap(), Field::Close);
    let out = batch(&mut s, &[1.0, 2.0, 3.0, 2.0, 5.0, 0.0, 0.0]);
    assert_eq!(
        out,
        vec![
            None,
            None,
            None,
            Some(200.0 / 3.0),
            Some(100.0),
            Some(0.0),
            Some(100.0 / 3.0)
        ]
    );
    // percentrank(1): previous value <= current -> 100, else 0.
    let mut s = Streaming::new(PercentRank::new(1).unwrap(), Field::Close);
    assert_eq!(
        batch(&mut s, &[3.0, 3.0, 2.0, 4.0]),
        vec![None, Some(100.0), Some(0.0), Some(100.0)]
    );
}

#[test]
fn batch_streaming_revised_and_reference_agree() {
    let x = bench_closes(1_500, 11);
    let noise = [1.01, 0.97, 1.2, 0.5, 1.0];
    for p in [1, 2, 3, 14, 20, 50, 120] {
        for mut s in studies(p) {
            let b = batch(s.as_mut(), &x);
            let r = revised(s.as_mut(), &x, &noise);
            let want = reference(s.name(), p, &x);
            assert_eq!(bits(&b), bits(&r), "{} {p}: batch vs revised", s.name());
            assert_eq!(bits(&b), bits(&want), "{} {p}: vs reference", s.name());
            let w = warmup_of(s.name(), p);
            assert_eq!(b.iter().take_while(|v| v.is_none()).count(), w);
            assert_eq!(s.warmup(), vec![w]);
            assert_eq!(s.params()["period"], f64::from(u32::try_from(p).unwrap()));
        }
    }
}

#[test]
fn discard_and_extend_match_push() {
    let x = bench_closes(400, 5);
    for p in [1, 2, 20] {
        for mut s in studies(p) {
            let b = batch(s.as_mut(), &x);
            s.reset();
            s.extend(&[&x[..300]]);
            let mut o = vec![None];
            for i in 300..x.len() {
                s.push(&[x[i] * 3.0], &mut o);
                s.discard_last();
                s.push(&[x[i]], &mut o);
                assert_eq!(o[0].map(f64::to_bits), b[i].map(f64::to_bits));
            }
        }
    }
}

#[test]
fn period_bounds_and_registry_unchanged() {
    assert!(Stdev::new(0).is_err());
    assert!(PercentRank::new(0).is_err());
    assert!(Stdev::new(MAX_PERIOD + 1).is_err());
    assert!(PercentRank::new(MAX_PERIOD + 1).is_err());
    assert!(Stdev::new(MAX_PERIOD).is_ok());
    assert!(PercentRank::new(MAX_PERIOD).is_ok());
    // Not exposed through `add_study`'s registry.
    assert_eq!(STUDY_NAMES, ["sma", "ema", "rsi", "macd", "bollinger"]);
    assert!(qp_compute::create("stdev", &std::collections::BTreeMap::new(), Field::Close).is_err());
    assert!(qp_compute::create(
        "percentrank",
        &std::collections::BTreeMap::new(),
        Field::Close
    )
    .is_err());
}

/// Test-only first study whose `None` depends on the value: defined iff `x >= 100`.
#[derive(Default)]
struct Gate;

impl Study for Gate {
    fn name(&self) -> &'static str {
        "gate"
    }
    fn params(&self) -> std::collections::BTreeMap<String, f64> {
        std::collections::BTreeMap::new()
    }
    fn inputs(&self) -> Vec<String> {
        vec!["close".into()]
    }
    fn outputs(&self) -> &'static [&'static str] {
        &["gate"]
    }
    fn warmup(&self) -> Vec<usize> {
        vec![0]
    }
    fn push(&mut self, input: &[f64], out: &mut [Option<f64>]) {
        out[0] = (input[0] >= 100.0).then_some(input[0]);
    }
    fn update_last(&mut self, input: &[f64], out: &mut [Option<f64>]) {
        self.push(input, out);
    }
    fn discard_last(&mut self) {}
    fn reset(&mut self) {}
}

fn kernel(mk: usize, p: usize) -> Box<dyn Study> {
    if mk == 0 {
        Box::new(Streaming::new(Stdev::new(p).unwrap(), Field::Close))
    } else {
        Box::new(Streaming::new(PercentRank::new(p).unwrap(), Field::Close))
    }
}

/// The `Chain` rule: where the input is undefined the output is undefined and the kernel
/// does not advance; the kernel sees exactly the defined values, also when a live-bar
/// revision flips the input between defined and undefined.
#[test]
fn chain_rule_for_undefined_inputs() {
    let x = bench_closes(600, 3);
    let defined_mask: Vec<bool> = x.iter().map(|&v| v >= 100.0).collect();
    assert!(defined_mask.iter().any(|d| *d) && defined_mask.iter().any(|d| !*d));
    let defined: Vec<f64> = x.iter().copied().filter(|&v| v >= 100.0).collect();
    for p in [1, 2, 5, 14] {
        for mk in [0, 1] {
            // Expected: the kernel over the defined values, re-spread, None elsewhere.
            let mut k = batch(kernel(mk, p).as_mut(), &defined).into_iter();
            let want: Vec<Option<f64>> = defined_mask
                .iter()
                .map(|&d| if d { k.next().unwrap() } else { None })
                .collect();

            let mut chain = Chain::new(Box::new(Gate), 0, kernel(mk, p));
            let out = run(&mut chain, &[&x]).unwrap().remove(0);
            assert_eq!(bits(&out), bits(&want), "batch {mk} {p}");

            // Every bar flips undefined -> defined -> undefined -> its final input.
            chain.reset();
            let mut o = vec![None];
            let mut got = Vec::new();
            for &xi in &x {
                chain.push(&[50.0], &mut o);
                chain.update_last(&[xi + 200.0], &mut o);
                chain.update_last(&[1.0], &mut o);
                chain.update_last(&[xi], &mut o);
                got.push(o[0]);
            }
            assert_eq!(bits(&got), bits(&want), "revised {mk} {p}");
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 256,
        rng_seed: RngSeed::Fixed(0x524c_3149),
        ..ProptestConfig::default()
    })]

    #[test]
    fn prop_modes_agree(
        cents in prop::collection::vec(1u32..50_000, 0..150),
        p in 1usize..30,
        noise in prop::collection::vec(0.5f64..2.0, 1..5),
    ) {
        // Cent prices: ties are frequent, which exercises `<=` in percentrank.
        let x: Vec<f64> = cents.iter().map(|&c| f64::from(c % 97 + c / 1000) / 100.0).collect();
        for mut s in studies(p) {
            let b = batch(s.as_mut(), &x);
            let r = revised(s.as_mut(), &x, &noise);
            let want = reference(s.name(), p, &x);
            prop_assert_eq!(bits(&b), bits(&r));
            prop_assert_eq!(bits(&b), bits(&want));
        }
    }
}

#[test]
fn digests_are_stable() {
    // The older digests are unchanged by the new kernels.
    assert_eq!(cross_target_digest(), 0x7b1e_99d4_0889_f83c);
    assert_eq!(r3_studies_digest(), 0x914f_cfdb_9f65_3a6d);
    assert_eq!(rl1_studies_digest(), 0xe196_f240_e628_c64d);
}
