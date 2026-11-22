//! Pipeline: look-ahead test (§9.4), determinism and replay, with fake detectors.

mod support;

use std::collections::BTreeSet;

use qp_capsule::engine::Engine;
use qp_capsule::pipeline::simulate_engine;
use qp_capsule::{simulate, InstrumentInput, SimResult, StoryEvent};
use qp_schema::capsule::Stage;
use qp_schema::Bar;
use support::*;

const N: u64 = 240;
const HORIZON: u32 = 8;

fn inputs(a: Vec<Bar>, b: Vec<Bar>) -> Vec<InstrumentInput> {
    vec![
        InstrumentInput {
            symbol: "BBB".to_owned(),
            series: SERIES.to_owned(),
            bars: b,
            flow: None,
        },
        InstrumentInput {
            symbol: "AAA".to_owned(),
            series: SERIES.to_owned(),
            bars: a,
            flow: None,
        },
    ]
}

fn base() -> Vec<InstrumentInput> {
    inputs(walk(7, N), walk(11, N))
}

fn run(inp: &[InstrumentInput]) -> SimResult {
    simulate(inp, &make_detectors, test_config(HORIZON))
}

fn json<T: serde::Serialize>(x: &T) -> String {
    serde_json::to_string(x).unwrap()
}

/// Replace every bar with `t >= cut` by a different deterministic walk (different
/// level, volatility and volume) so the future really differs.
fn perturb(inp: &[InstrumentInput], cut: i64, seed: u64) -> Vec<InstrumentInput> {
    inp.iter()
        .enumerate()
        .map(|(k, x)| {
            let mut r = Lcg(seed + k as u64);
            let mut bars = x.bars.clone();
            let mut c = 90.0;
            for b in bars.iter_mut().filter(|b| b.t >= cut) {
                c *= 1.0 + (r.next_f() - 0.5) * 0.08;
                b.o = c;
                b.h = c;
                b.l = c;
                b.c = c;
                b.v = 500.0 + r.next_f() * 4000.0;
            }
            InstrumentInput { bars, ..x.clone() }
        })
        .collect()
}

fn before(ev: &[StoryEvent], cut: i64) -> Vec<StoryEvent> {
    ev.iter()
        .filter(|e| e.transition.t < cut)
        .cloned()
        .collect()
}

fn keys(r: &SimResult) -> BTreeSet<String> {
    r.events
        .iter()
        .map(|e| e.story.ledger_key.clone())
        .collect()
}

#[test]
fn the_run_is_not_trivial() {
    let r = run(&base());
    let stages: BTreeSet<_> = r
        .events
        .iter()
        .map(|e| format!("{:?}", e.transition.stage))
        .collect();
    for s in [
        Stage::Born,
        Stage::Strengthened,
        Stage::Confirmed,
        Stage::Weakened,
        Stage::Realized,
        Stage::Invalidated,
        Stage::Expired,
    ] {
        assert!(
            stages.contains(&format!("{s:?}")),
            "missing {s:?}: {stages:?}"
        );
    }
    assert!(r.ledger.len() > 10);
    assert!(r.events.iter().any(|e| e.story.ledger_at_birth.n > 0));
    let syms: BTreeSet<_> = r
        .events
        .iter()
        .map(|e| e.story.instrument.clone())
        .collect();
    assert_eq!(syms.len(), 2);
}

#[test]
fn look_ahead_shuffling_the_future_does_not_change_the_past() {
    let base = base();
    let r0 = run(&base);
    for cut_bar in [40, 97, 150, 201] {
        let cut = t_of(cut_bar);
        for seed in [1, 2, 3] {
            let r1 = run(&perturb(&base, cut, seed));
            // The perturbation is effective: the futures differ.
            assert_ne!(json(&r0.events), json(&r1.events));

            // Every event before the cut, byte-identical (includes every birth before the
            // cut with its ledger_at_birth, and every rank computed before the cut).
            let (b0, b1) = (before(&r0.events, cut), before(&r1.events, cut));
            assert!(!b0.is_empty());
            assert_eq!(json(&b0), json(&b1), "cut {cut_bar} seed {seed}");

            // ledger_at_birth of stories born before the cut.
            let lab = |r: &SimResult| -> Vec<_> {
                r.events
                    .iter()
                    .filter(|e| e.transition.stage == Stage::Born && e.transition.t < cut)
                    .map(|e| (e.capsule.clone(), e.story.ledger_at_birth))
                    .collect()
            };
            assert_eq!(lab(&r0), lab(&r1));

            // The ledger as seen at the cut.
            for k in keys(&r0).union(&keys(&r1)) {
                assert_eq!(r0.ledger_stat(k, cut), r1.ledger_stat(k, cut), "{k}");
            }
            let rows = |r: &SimResult| -> Vec<_> {
                r.ledger
                    .iter()
                    .filter(|e| e.recorded_at < cut)
                    .map(json)
                    .collect()
            };
            assert_eq!(rows(&r0), rows(&r1));
        }
    }
}

#[test]
fn determinism_two_runs_identical() {
    let a = run(&base());
    let b = run(&base());
    assert_eq!(json(&a.events), json(&b.events));
    assert_eq!(json(&a.ledger), json(&b.ledger));
    assert_eq!(json(&a.facts), json(&b.facts));
    // Input order of instruments does not matter (global order is t, symbol).
    let mut rev = base();
    rev.reverse();
    let c = run(&rev);
    assert_eq!(json(&a.events), json(&c.events));
}

#[test]
fn replay_same_facts_into_fresh_engine_reproduces_history() {
    let inp = base();
    let (eng, facts) = simulate_engine(&inp, &make_detectors, test_config(HORIZON));
    let mut fresh = Engine::new(test_config(HORIZON));
    let mut order: Vec<(i64, &str, usize, usize)> = Vec::new();
    for (k, x) in inp.iter().enumerate() {
        for (i, b) in x.bars.iter().enumerate() {
            order.push((b.t, x.symbol.as_str(), k, i));
        }
    }
    order.sort_unstable();
    for (_, sym, k, i) in order {
        let fs: Vec<_> = facts
            .iter()
            .filter(|f| f.bar == i as u64 && f.meaning.as_ref().unwrap().instrument == sym)
            .cloned()
            .collect();
        fresh.on_bar(sym, SERIES, i as u64, &inp[k].bars, &fs);
    }
    assert_eq!(json(&eng.snapshot()), json(&fresh.snapshot()));
}

#[test]
fn fact_ids_are_deterministic_and_unique() {
    let r = run(&base());
    let ids: BTreeSet<_> = r.facts.iter().map(|f| f.id.clone()).collect();
    assert_eq!(ids.len(), r.facts.len());
    assert!(r.facts[0].id.starts_with("fx."));
}
