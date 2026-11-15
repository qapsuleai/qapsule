//! Harness self-tests: determinism, the control baseline, engine convergence, presets.

use harness::adapter::{BindingSetup, Setup, Side};
use harness::approaches;
use harness::detector::DetectorCfg;
use harness::engine::Engine;
use harness::feed::deliver;
use harness::rng::Rng;
use harness::scenarios;
use harness::sim::run_one;
use harness::spec::{FeedSpec, ScenarioSpec};
use harness::world::{generate, StepKind};

fn n() -> harness::adapter::Approach {
    approaches::find("n").expect("baseline registered")
}

/// Same seed → byte-identical display log, for every preset (2 seeds each; S17 1 seed).
#[test]
fn same_seed_same_display_log() {
    for spec in scenarios::presets() {
        let seeds = if spec.name == "S17" { 1 } else { 2 };
        for seed in 0..seeds {
            let a = run_one(&spec, &n(), seed);
            let b = run_one(&spec, &n(), seed);
            assert!(!a.log.is_empty(), "{} seed {seed}: empty log", spec.name);
            assert_eq!(a.log, b.log, "{} seed {seed}: logs differ", spec.name);
            let (mut ma, mut mb) = (a.metrics.clone(), b.metrics.clone());
            ma.m10_wall_ns = 0;
            mb.m10_wall_ns = 0;
            assert_eq!(ma, mb, "{} seed {seed}: metrics differ", spec.name);
        }
    }
}

/// Different seeds give different worlds.
#[test]
fn different_seed_different_log() {
    let spec = scenarios::find("S0").unwrap();
    assert_ne!(run_one(&spec, &n(), 1).log, run_one(&spec, &n(), 2).log);
}

/// Thread count does not change results.
#[test]
fn threads_do_not_change_results() {
    let spec = scenarios::find("S5").unwrap();
    let (a, _) = harness::run_scenario(&spec, &n(), 0, 6, 1);
    let (b, _) = harness::run_scenario(&spec, &n(), 0, 6, 4);
    let strip = |v: Vec<harness::metrics::Metrics>| {
        v.into_iter()
            .map(|mut m| {
                m.m10_wall_ns = 0;
                m
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(strip(a), strip(b));
}

/// The ideal scenario gives the baseline no hard error; the lagging replica feed does.
#[test]
fn baseline_fails_where_expected() {
    let ideal = scenarios::find("S1").unwrap();
    for seed in 0..5 {
        let m = run_one(&ideal, &n(), seed).metrics;
        assert_eq!(m.errors(), 0, "S1 seed {seed}: {m:?}");
        assert!(m.shows > 0);
    }
    let lag = scenarios::find("S2").unwrap();
    let errors: u64 = (0..5)
        .map(|s| run_one(&lag, &n(), s).metrics.errors())
        .sum();
    assert!(errors > 0);
}

fn setup_for(spec: &ScenarioSpec, world: &harness::world::World) -> Setup {
    Setup {
        side: Side::Replica,
        bindings: (0..world.ids.len())
            .map(|b| BindingSetup {
                id: world.ids[b].clone(),
                binding: qp_schema::data::DataBinding {
                    symbol: world.feeds[b].symbol.clone(),
                    interval: world.feeds[b].interval.clone(),
                    window: qp_schema::data::Window::last(spec.history_bars),
                    feed: None,
                },
                feed: world.feeds[b].clone(),
                interval_ms: world.interval_ms,
            })
            .collect(),
        detector: DetectorCfg::new(spec.detector.clone()),
        extra_studies: 0,
    }
}

/// With reordering and duplicates but no loss, the shared engine (SeqLww) converges to
/// the source's bars, and its incremental claim set equals a batch recomputation.
#[test]
fn engine_converges_under_reorder_and_duplicates() {
    let spec = ScenarioSpec {
        ticks_per_bar: 60,
        live_bars: 20,
        ..ScenarioSpec::default()
    };
    for seed in 0..5 {
        let world = generate(&spec, seed);
        let fs = FeedSpec {
            reorder_p: 0.3,
            reorder_ms: 3_000.0,
            dup_p: 0.2,
            dup_ms: 2_000.0,
            ..FeedSpec::default()
        };
        let (mut evs, _) = deliver(&world, 0, &fs, &mut Rng::new(seed));
        evs.sort_by_key(|(t, _)| *t);
        let setup = setup_for(&spec, &world);
        let mut e = Engine::new(&setup);
        for (_, ev) in &evs {
            e.apply(ev);
        }
        // Source truth: replay the world.
        let mut truth = world.history[0].clone();
        for s in &world.steps {
            if let StepKind::Data { ev, .. } = &s.kind {
                match &ev.change {
                    qp_schema::data::DataChange::Bar { bar } => truth.push(bar.clone()),
                    qp_schema::data::DataChange::Patch { t, fields } => {
                        let i = truth.iter().position(|b| b.t == *t).unwrap();
                        fields.apply_to(&mut truth[i]);
                    }
                }
            }
        }
        assert_eq!(e.bars(0), truth.as_slice(), "seed {seed}");
        let batch = e.detector().claims_batch(e.ids(), &[e.series(0)]);
        assert_eq!(&batch, e.claims(), "seed {seed}");
    }
}

/// Every preset survives a JSON round trip (an auditor edits `harness spec` output).
#[test]
fn presets_round_trip_as_json() {
    for spec in scenarios::presets() {
        spec.validate().unwrap();
        let j = serde_json::to_string(&spec).unwrap();
        let back: ScenarioSpec = serde_json::from_str(&j).unwrap();
        assert_eq!(back, spec, "{}", spec.name);
    }
    // A minimal custom spec: defaults fill the rest.
    let s: ScenarioSpec =
        serde_json::from_str(r#"{"name":"mine","replica_feed":{"delay":{"base_ms":700}}}"#)
            .unwrap();
    assert_eq!(s.replica_feed.delay.base_ms, 700.0);
    assert_eq!(s.history_bars, ScenarioSpec::default().history_bars);
}
