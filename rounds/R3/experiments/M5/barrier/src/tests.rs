use super::*;

use qp_capsule::{EngineConfig, InstrumentInput, OpenStory};
use qp_schema::capsule::{LedgerStat, Stage, Story, Trust};

fn bar(i: usize, c: f64, range: f64) -> Bar {
    Bar {
        t: 1_700_000_000_000 + i64::try_from(i).unwrap() * 86_400_000,
        o: c,
        h: c + range / 2.0,
        l: c - range / 2.0,
        c,
        v: 1000.0,
    }
}

/// `n` flat bars at 100 with high−low = 2 (so ATR(14) = ATR(100) = 2 exactly).
fn flat(n: usize) -> Vec<Bar> {
    (0..n).map(|i| bar(i, 100.0, 2.0)).collect()
}

fn story(kind: MeaningKind, dir: Direction, time: TimeKind, born: u64) -> OpenStory {
    OpenStory {
        id: format!("cap.TEST.x.{born}"),
        story: Story {
            instrument: "TEST".to_owned(),
            theme: "x".to_owned(),
            kind,
            direction: dir,
            time,
            stage: Stage::Born,
            families: vec![],
            strength: 0.5,
            importance: 0.0,
            tier: 3,
            series: "1day".to_owned(),
            born_bar: born,
            last_bar: born,
            horizon_bars: base_params(kind).horizon,
            ledger_key: String::new(),
            ledger_at_birth: LedgerStat::default(),
            trust: Trust::Replica,
            history: vec![],
        },
        facts: vec![],
    }
}

fn judge_all(r: &Barrier, s: &OpenStory, bars: &[Bar]) -> Option<(u64, Verdict, &'static str)> {
    let born = usize::try_from(s.story.born_bar).unwrap();
    let h = r.params(s.story.kind).horizon as usize;
    for now in born + 1..=born + h {
        if now >= bars.len() {
            break;
        }
        if let Some((v, c)) = r.judge(s, &bars[..=now], now as u64) {
            return Some((now as u64, v, c));
        }
    }
    None
}

#[test]
fn wilder_atr_matches_hand_value() {
    let b = flat(30);
    // Every TR = 2 (flat closes, range 2).
    assert_eq!(wilder_atr(&b[..14], 14), Some(2.0));
    assert_eq!(wilder_atr(&b[..13], 14), None);
    assert_eq!(wilder_atr(&b, 14), Some(2.0));
}

#[test]
fn breakout_up_target_and_stop() {
    let r = Barrier::new("t", 1.0, 1.0);
    // Breakout: k = 0.3·√10 ≈ 0.949, ATR = 2 → barrier ≈ 1.897.
    let mut b = flat(40);
    b.push(bar(40, 101.0, 2.0)); // +1
    b.push(bar(41, 101.85, 2.0)); // +1.85 (< 1.897)
    b.push(bar(42, 101.95, 2.0)); // +1.95 → target
    let s = story(MeaningKind::Breakout, Direction::Up, TimeKind::Event, 39);
    assert_eq!(
        judge_all(&r, &s, &b),
        Some((42, Verdict::Realized, "barrier.target"))
    );

    let mut b = flat(40);
    b.push(bar(40, 98.05, 2.0)); // −1.95 → stop
    assert_eq!(
        judge_all(&r, &s, &b),
        Some((40, Verdict::Invalidated, "barrier.stop"))
    );
    // Same path for a down story: target.
    let s = story(MeaningKind::Breakout, Direction::Down, TimeKind::Event, 39);
    assert_eq!(
        judge_all(&r, &s, &b),
        Some((40, Verdict::Realized, "barrier.target"))
    );
}

#[test]
fn directional_undecided_inside_corridor_then_expires() {
    let r = Barrier::new("t", 1.0, 1.0);
    let b = flat(80);
    let s = story(
        MeaningKind::Continuation,
        Direction::Up,
        TimeKind::State,
        39,
    );
    assert_eq!(judge_all(&r, &s, &b), None);
    assert_eq!(r.horizon(MeaningKind::Continuation, TimeKind::State), 20);
}

#[test]
fn neutral_expansion_and_no_expansion() {
    let r = Barrier::new("t", 1.0, 1.0);
    // Squeeze: k_n = 0.65·√20 ≈ 2.907, ATR(100) = 2 → ≈ 5.81.
    let s = story(
        MeaningKind::Squeeze,
        Direction::Neutral,
        TimeKind::State,
        120,
    );
    let mut b = flat(121);
    for i in 121..=125 {
        b.push(bar(
            i,
            100.0 - 1.4 * f64::from(u32::try_from(i - 120).unwrap()),
            2.0,
        ));
    }
    // closes 98.6, 97.2, 95.8, 94.4, 93: |−5.6| at 124 < 5.81, |−7| at 125.
    assert_eq!(
        judge_all(&r, &s, &b),
        Some((125, Verdict::Realized, "barrier.expansion"))
    );
    // Flat → invalidated exactly at the horizon (born + 20), before the engine expires it.
    let b = flat(200);
    assert_eq!(
        judge_all(&r, &s, &b),
        Some((140, Verdict::Invalidated, "barrier.no_expansion"))
    );
    // Neutral before ATR(100) is warm: no verdict (expires).
    let s = story(
        MeaningKind::Squeeze,
        Direction::Neutral,
        TimeKind::State,
        50,
    );
    assert_eq!(judge_all(&r, &s, &b), None);
}

#[test]
fn no_verdict_without_atr_or_on_birth_bar() {
    let r = Barrier::new("t", 1.0, 1.0);
    let mut b = flat(12);
    b.push(bar(12, 150.0, 2.0));
    let s = story(MeaningKind::Breakout, Direction::Up, TimeKind::Event, 11);
    assert_eq!(judge_all(&r, &s, &b), None, "ATR(14) not warm at bar 11");
    let s = story(MeaningKind::Breakout, Direction::Up, TimeKind::Event, 12);
    assert_eq!(r.judge(&s, &b, 12), None, "never on the birth bar");
}

#[test]
fn verdict_is_first_passage_even_if_calls_are_skipped() {
    let r = Barrier::new("t", 1.0, 1.0);
    let mut b = flat(40);
    b.push(bar(40, 96.0, 2.0)); // stop first
    b.push(bar(41, 110.0, 2.0)); // target later
    let s = story(MeaningKind::Breakout, Direction::Up, TimeKind::Event, 39);
    // Called only at 41: still the stop (first crossing in (born, now]).
    assert_eq!(
        r.judge(&s, &b, 41),
        Some((Verdict::Invalidated, "barrier.stop"))
    );
}

#[test]
fn perturbations_scale_horizon_and_barriers() {
    let p = perturbations();
    assert_eq!(p.len(), 4);
    let base = Barrier::new("t", 1.0, 1.0);
    for kind in MeaningKind::ALL {
        let h = f64::from(base.params(kind).horizon);
        let hs: Vec<u32> = p
            .iter()
            .map(|(_, r)| r.horizon(kind, TimeKind::Event))
            .collect();
        assert_eq!(hs[0], (h * 0.8).round() as u32);
        assert_eq!(hs[1], (h * 1.2).round() as u32);
        assert_eq!(hs[2], base.params(kind).horizon);
        assert_eq!(hs[3], base.params(kind).horizon);
    }
    assert_eq!(outcome().name(), "barrier");
}

#[test]
fn judge_depends_only_on_bars_up_to_now() {
    // For every story position and every `now`, the verdict on the prefix equals the
    // verdict when everything after `now` is replaced with wild values (the rule only
    // receives `bars[..=now]` from the engine; this also checks it never indexes past it
    // through the cache).
    let r = Barrier::new("t", 1.0, 1.0);
    let base: Vec<Bar> = (0..160)
        .map(|i| {
            let x = f64::from(u32::try_from(i).unwrap());
            bar(
                i,
                100.0 + 5.0 * (x * 0.37).sin() + 0.05 * x,
                1.5 + (x * 0.11).cos(),
            )
        })
        .collect();
    for born in [20u64, 60, 110, 130] {
        for dir in [Direction::Up, Direction::Down, Direction::Neutral] {
            let s = story(MeaningKind::Reversal, dir, TimeKind::Event, born);
            for now in born + 1..born + 16 {
                let n = usize::try_from(now).unwrap();
                let mut alt = base.clone();
                for (j, b) in alt.iter_mut().enumerate().skip(n + 1) {
                    b.c = if j % 2 == 0 { 1.0 } else { 1000.0 };
                }
                let a = r.judge(&s, &base[..=n], now);
                let fresh = Barrier::new("t2", 1.0, 1.0);
                let c = fresh.judge(&s, &alt[..=n], now);
                assert_eq!(a, c, "born {born} now {now} dir {dir:?}");
            }
        }
    }
}

/// Engine-level look-ahead test: bars after `cut` are shuffled; every ledger row closed
/// at or before `cut` and every event at or before `cut` must be identical.
#[test]
fn engine_ledger_unchanged_when_future_is_shuffled() {
    let market = qp_source::synthetic_market(4, 300, 7);
    let inputs: Vec<InstrumentInput> = market
        .iter()
        .map(|m| InstrumentInput {
            symbol: m.symbol.clone(),
            series: "1day".to_owned(),
            bars: m.bars.clone(),
            flow: None,
        })
        .collect();
    let cfg = || EngineConfig {
        theme: Box::new(qp_capsule::scaffold::KindTheme),
        outcome: outcome(),
        ranker: Box::new(qp_capsule::scaffold::ConstRank),
    };
    let base = qp_capsule::simulate(&inputs, &qp_facts::default_detectors, cfg());
    let cut_i = 180usize;
    let cut_t = inputs[0].bars[cut_i].t;
    let mut shuffled = inputs.clone();
    for (k, inp) in shuffled.iter_mut().enumerate() {
        // Deterministic permutation of the bar values after the cut (times kept).
        let tail: Vec<Bar> = inp.bars[cut_i + 1..].to_vec();
        let n = tail.len();
        for (j, b) in inp.bars[cut_i + 1..].iter_mut().enumerate() {
            let src = &tail[(j * 7 + 3 + k) % n];
            let t = b.t;
            *b = src.clone();
            b.t = t;
        }
    }
    let alt = qp_capsule::simulate(&shuffled, &qp_facts::default_detectors, cfg());
    let rows = |r: &qp_capsule::SimResult| -> Vec<String> {
        r.ledger
            .iter()
            .filter(|e| e.recorded_at <= cut_t)
            .map(|e| format!("{e:?}"))
            .collect()
    };
    let a = rows(&base);
    assert!(
        a.len() > 20,
        "enough closed rows before the cut: {}",
        a.len()
    );
    assert_eq!(a, rows(&alt));
    let ev = |r: &qp_capsule::SimResult| -> Vec<String> {
        r.events
            .iter()
            .filter(|e| e.transition.t <= cut_t)
            .map(|e| format!("{e:?}"))
            .collect()
    };
    assert_eq!(ev(&base), ev(&alt));
    // The future did change.
    assert_ne!(base.ledger.len(), 0);
    assert_ne!(
        format!("{:?}", base.ledger.last()),
        format!("{:?}", alt.ledger.last())
    );
}
