//! R3/T3 studies (`highest`, `lowest`, `atr`):
//! - batch = streaming = "revise the last bar" (with several wrong revisions), bit for bit;
//! - against a from-scratch reference (window max/min exact; ATR with the same operations
//!   in the same order, so also bit for bit);
//! - warmup, `discard_last`, `extend`, the separate digest is stable.

use proptest::prelude::*;
use proptest::test_runner::RngSeed;
use qp_compute::sweep::{bench_closes, cross_target_digest, r3_studies_digest};
use qp_compute::{run, Atr, Field, Highest, Lowest, Streaming, Study};

fn hlc_names() -> Vec<String> {
    vec!["high".into(), "low".into(), "close".into()]
}

fn studies(p: usize) -> Vec<Box<dyn Study>> {
    vec![
        Box::new(Streaming::new(Highest::new(p).unwrap(), Field::High)),
        Box::new(Streaming::new(Lowest::new(p).unwrap(), Field::Low)),
        Box::new(Streaming::with_inputs(Atr::new(p).unwrap(), hlc_names())),
    ]
}

/// Inputs per study kind: `highest` reads `h`, `lowest` `l`, `atr` `(h, l, c)`.
fn input(s: &dyn Study, h: f64, l: f64, c: f64) -> Vec<f64> {
    match s.name() {
        "highest" => vec![h],
        "lowest" => vec![l],
        _ => vec![h, l, c],
    }
}

fn bits(v: &[Option<f64>]) -> Vec<Option<u64>> {
    v.iter().map(|x| x.map(f64::to_bits)).collect()
}

fn bars(closes: &[f64]) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let h = closes
        .iter()
        .enumerate()
        .map(|(i, c)| c + 0.01 * f64::from(u32::try_from(i % 7).unwrap()))
        .collect();
    let l = closes
        .iter()
        .enumerate()
        .map(|(i, c)| c - 0.01 * f64::from(u32::try_from(i % 5).unwrap()))
        .collect();
    (h, l, closes.to_vec())
}

fn batch(s: &mut dyn Study, h: &[f64], l: &[f64], c: &[f64]) -> Vec<Option<f64>> {
    let cols: Vec<&[f64]> = match s.name() {
        "highest" => vec![h],
        "lowest" => vec![l],
        _ => vec![h, l, c],
    };
    run(s, &cols).unwrap().remove(0)
}

fn revised(s: &mut dyn Study, h: &[f64], l: &[f64], c: &[f64], noise: &[f64]) -> Vec<Option<f64>> {
    s.reset();
    let mut o = vec![None];
    let mut out = Vec::new();
    for i in 0..c.len() {
        let a = noise[i % noise.len()];
        let b = noise[(i * 7 + 3) % noise.len()];
        s.push(&input(s, h[i] * a, l[i] * b, c[i] * a), &mut o);
        s.update_last(&input(s, h[i] + a, l[i] - b, c[i] * b), &mut o);
        s.update_last(&input(s, h[i] * b, l[i] * a, c[i] + a), &mut o);
        s.update_last(&input(s, h[i], l[i], c[i]), &mut o);
        out.push(o[0]);
    }
    out
}

fn reference(name: &str, p: usize, h: &[f64], l: &[f64], c: &[f64]) -> Vec<Option<f64>> {
    let pf = f64::from(u32::try_from(p).unwrap());
    let mut atr = 0.0;
    let mut sum = 0.0;
    (0..c.len())
        .map(|i| match name {
            "highest" => (i + 1 >= p).then(|| {
                h[i + 1 - p..=i]
                    .iter()
                    .copied()
                    .fold(f64::NEG_INFINITY, f64::max)
            }),
            "lowest" => (i + 1 >= p).then(|| {
                l[i + 1 - p..=i]
                    .iter()
                    .copied()
                    .fold(f64::INFINITY, f64::min)
            }),
            _ => {
                let tr = if i == 0 {
                    h[0] - l[0]
                } else {
                    let pc = c[i - 1];
                    (h[i] - l[i]).max((h[i] - pc).abs()).max((l[i] - pc).abs())
                };
                if i + 1 < p {
                    sum += tr;
                    None
                } else if i + 1 == p {
                    atr = (sum + tr) / pf;
                    Some(atr)
                } else {
                    atr = (atr * (pf - 1.0) + tr) / pf;
                    Some(atr)
                }
            }
        })
        .collect()
}

#[test]
fn batch_streaming_revised_and_reference_agree() {
    let closes = bench_closes(1_500, 11);
    let (h, l, c) = bars(&closes);
    let noise = [1.01, 0.97, 1.2, 0.5, 1.0];
    for p in [1, 2, 3, 14, 20, 50, 120] {
        for mut s in studies(p) {
            let b = batch(s.as_mut(), &h, &l, &c);
            let r = revised(s.as_mut(), &h, &l, &c, &noise);
            let want = reference(s.name(), p, &h, &l, &c);
            assert_eq!(bits(&b), bits(&r), "{} {p}: batch vs revised", s.name());
            assert_eq!(bits(&b), bits(&want), "{} {p}: vs reference", s.name());
            assert_eq!(b.iter().take_while(|v| v.is_none()).count(), p - 1);
            assert_eq!(s.warmup(), vec![p - 1]);
        }
    }
}

#[test]
fn discard_and_extend_match_push() {
    let closes = bench_closes(400, 5);
    let (h, l, c) = bars(&closes);
    for p in [1, 2, 20] {
        for mut s in studies(p) {
            let b = batch(s.as_mut(), &h, &l, &c);
            // extend the first 300, then push the rest.
            s.reset();
            let cols: Vec<&[f64]> = match s.name() {
                "highest" => vec![&h[..300]],
                "lowest" => vec![&l[..300]],
                _ => vec![&h[..300], &l[..300], &c[..300]],
            };
            s.extend(&cols);
            let mut o = vec![None];
            for i in 300..c.len() {
                // A bar that is pushed then discarded leaves no trace.
                s.push(&input(s.as_ref(), h[i] * 3.0, l[i] * 0.1, c[i]), &mut o);
                s.discard_last();
                s.push(&input(s.as_ref(), h[i], l[i], c[i]), &mut o);
                assert_eq!(o[0].map(f64::to_bits), b[i].map(f64::to_bits));
            }
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 256,
        rng_seed: RngSeed::Fixed(0x5233_5433),
        ..ProptestConfig::default()
    })]

    #[test]
    fn prop_modes_agree(
        cents in prop::collection::vec((1u32..50_000, 0u32..300, 0u32..300), 0..150),
        p in 1usize..30,
        noise in prop::collection::vec(0.5f64..2.0, 1..5),
    ) {
        let c: Vec<f64> = cents.iter().map(|&(x, _, _)| f64::from(x) / 100.0).collect();
        let h: Vec<f64> = cents.iter().map(|&(x, a, _)| f64::from(x + a) / 100.0).collect();
        let l: Vec<f64> = cents.iter().map(|&(x, _, b)| f64::from(x.saturating_sub(b)) / 100.0).collect();
        for mut s in studies(p) {
            let b = batch(s.as_mut(), &h, &l, &c);
            let r = revised(s.as_mut(), &h, &l, &c, &noise);
            let want = reference(s.name(), p, &h, &l, &c);
            prop_assert_eq!(bits(&b), bits(&r));
            prop_assert_eq!(bits(&b), bits(&want));
        }
    }
}

#[test]
fn digests_are_stable() {
    // The pre-R3 cross-target digest is unchanged by the new studies.
    assert_eq!(cross_target_digest(), 0x7b1e_99d4_0889_f83c);
    assert_eq!(r3_studies_digest(), 0x914f_cfdb_9f65_3a6d);
}
