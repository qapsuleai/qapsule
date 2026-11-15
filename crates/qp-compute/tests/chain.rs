//! A study can read another study's output and several bar fields; all modes agree.

mod common;

use std::collections::BTreeMap;

use qp_compute::{run, Chain, Core, Ema, Rsi, Streaming, Study};

#[test]
fn ema_of_rsi_matches_manual_composition() {
    let x: Vec<f64> = (0..80)
        .map(|i| 100.0 + f64::from(i % 7) - f64::from(i % 3) * 1.5)
        .collect();
    let mut chain = Chain::new(
        common::study("rsi", &[("period", 5.0)]),
        0,
        common::study("ema", &[("period", 3.0)]),
    );
    let out = run(&mut chain, &[&x]).unwrap();

    // Manual: rsi, drop warmup, ema, re-pad.
    let rsi = common::batch("rsi", &[("period", 5.0)], &x)[0].clone();
    let defined: Vec<f64> = rsi.iter().flatten().copied().collect();
    let ema = common::batch("ema", &[("period", 3.0)], &defined)[0].clone();
    let mut want = vec![None; x.len() - defined.len()];
    want.extend(ema);
    assert_eq!(out[0], want);
    assert_eq!(chain.warmup(), vec![5 + 2]);
    assert_eq!(chain.inputs(), vec!["close".to_owned()]);

    // Revise every bar a few times: same bits.
    chain.reset();
    let mut o = vec![None];
    let mut rev = Vec::new();
    for &xi in &x {
        chain.push(&[xi * 0.9], &mut o);
        chain.update_last(&[xi * 1.1], &mut o);
        chain.update_last(&[xi], &mut o);
        rev.push(o[0]);
    }
    assert_eq!(rev, out[0]);
}

/// A two-input core used only to exercise the multi-input interface: the midpoint
/// `(high + low) / 2` smoothed by an EMA.
#[derive(Clone)]
struct MidEma(Ema);

impl Core for MidEma {
    const NAME: &'static str = "mid_ema";
    const OUTPUTS: &'static [&'static str] = &["mid", "mid_ema"];
    const N_IN: usize = 2;
    fn params(&self) -> BTreeMap<String, f64> {
        BTreeMap::new()
    }
    fn warmup(&self) -> Vec<usize> {
        vec![0, self.0.period() - 1]
    }
    fn peek(&self, x: &[f64], out: &mut [Option<f64>]) {
        let mid = (x[0] + x[1]) / 2.0;
        out[0] = Some(mid);
        out[1] = self.0.peek1(mid);
    }
    fn commit(&mut self, x: &[f64]) {
        self.0.commit1((x[0] + x[1]) / 2.0);
    }
    fn reset(&mut self) {
        self.0.reset();
    }
}

#[test]
fn multi_input_multi_output() {
    let high = [11.0, 12.0, 13.0, 12.0];
    let low = [9.0, 10.0, 11.0, 10.0];
    let mut s = Streaming::with_inputs(
        MidEma(Ema::new(2).unwrap()),
        vec!["high".into(), "low".into()],
    );
    let out = run(&mut s, &[&high, &low]).unwrap();
    assert_eq!(out[0], vec![Some(10.0), Some(11.0), Some(12.0), Some(11.0)]);
    assert_eq!(out[1][0], None);
    assert_eq!(out[1][1], Some(10.5));
    // wrong arity is an error, not a panic
    assert!(run(&mut s, &[&high]).is_err());
    let _ = Rsi::new(3).unwrap();
}

/// Test-only study with a value-dependent `None`: the input if `>= 10`, else `None`.
#[derive(Clone)]
struct AtLeast10;

impl Core for AtLeast10 {
    const NAME: &'static str = "at_least_10";
    const OUTPUTS: &'static [&'static str] = &["v"];
    const N_IN: usize = 1;
    fn params(&self) -> BTreeMap<String, f64> {
        BTreeMap::new()
    }
    fn warmup(&self) -> Vec<usize> {
        vec![0]
    }
    fn peek(&self, x: &[f64], out: &mut [Option<f64>]) {
        out[0] = (x[0] >= 10.0).then_some(x[0]);
    }
    fn commit(&mut self, _x: &[f64]) {}
    fn reset(&mut self) {}
}

/// R1 audit finding 4: a revision that turns the first study's output from a value into
/// `None` must also take the bar back out of the second study.
#[test]
fn revision_to_none_is_not_committed_downstream() {
    let mk = || {
        Chain::new(
            Box::new(Streaming::new(AtLeast10, qp_compute::Field::Close)),
            0,
            common::study("sma", &[("period", 2.0)]),
        )
    };
    // Batch over the final prices [20, 5, 40]: the gate gives [20, None, 40], so the
    // SMA(2) sees [20, 40] and the last bar is 30.
    let mut batch = mk();
    let want = run(&mut batch, &[&[20.0, 5.0, 40.0]]).unwrap();
    assert_eq!(want[0], vec![None, None, Some(30.0)]);

    // Streaming: bar 1 first arrives as 30 (defined), is revised to 5 (None).
    let mut s = mk();
    let mut o = vec![None];
    s.push(&[20.0], &mut o);
    s.push(&[30.0], &mut o);
    assert_eq!(o[0], Some(25.0));
    s.update_last(&[5.0], &mut o);
    assert_eq!(o[0], None);
    s.push(&[40.0], &mut o);
    // Before the fix the stale 30 was committed into the SMA: (30 + 40) / 2 = 35.
    assert_eq!(o[0], Some(30.0));

    // And back: None -> value -> None -> value on the same bar.
    let mut s = mk();
    s.push(&[20.0], &mut o);
    s.push(&[5.0], &mut o);
    s.update_last(&[30.0], &mut o);
    assert_eq!(o[0], Some(25.0));
    s.update_last(&[6.0], &mut o);
    assert_eq!(o[0], None);
    s.push(&[40.0], &mut o);
    assert_eq!(o[0], Some(30.0));
}
