//! Tests of the harness itself: determinism, re-scoring, labeling, and that each
//! look-ahead / leak check catches a deliberately cheating dummy.

use std::collections::BTreeMap;
use std::rc::Rc;

use harness_sim::common::{self, DataKind};
use harness_sim::episodes::{self, EP_TAIL};
use harness_sim::{control, m3, m4, m5, m6, reference, sim};
use qp_capsule::{
    FlowObservation, InstrumentInput, OpenStory, OutcomeRule, QualMessage, RankCtx, Ranker,
    TrustPath, Verdict,
};
use qp_schema::capsule::{Direction, MeaningKind, TimeKind, Trust};
use qp_schema::{Bar, Fact};

fn small(seed: u64) -> common::Dataset {
    common::synthetic_sized(seed, 4, 300)
}

// ---------------------------------------------------------------------------
// Cheating dummies (side channel: they are built with the whole data set)
// ---------------------------------------------------------------------------

/// Ranker that peeks at the close 5 bars ahead.
struct PeekRanker(Vec<InstrumentInput>);

impl Ranker for PeekRanker {
    fn name(&self) -> &'static str {
        "test.peek_ranker"
    }
    fn rank(&self, s: &OpenStory, _ctx: &RankCtx) -> (f64, u8) {
        let inp = self
            .0
            .iter()
            .find(|x| x.symbol == s.story.instrument)
            .expect("instrument");
        let i = usize::try_from(s.story.last_bar).unwrap_or(0);
        let fut = inp.bars.get(i + 5).map_or(0.0, |b| b.c);
        let now = inp.bars[i].c;
        (fut - now, if fut > now { 1 } else { 3 })
    }
}

/// Outcome rule that peeks at the close 3 bars ahead through a side channel.
struct PeekOutcome(Vec<InstrumentInput>);

impl OutcomeRule for PeekOutcome {
    fn name(&self) -> &'static str {
        "test.peek_outcome"
    }
    fn horizon(&self, _k: MeaningKind, _t: TimeKind) -> u32 {
        10
    }
    fn judge(&self, s: &OpenStory, bars: &[Bar], now: u64) -> Option<(Verdict, &'static str)> {
        let inp = self.0.iter().find(|x| x.symbol == s.story.instrument)?;
        let i = usize::try_from(now).ok()?;
        let fut = inp.bars.get(i + 3)?.c;
        if now <= s.story.born_bar {
            return None;
        }
        Some(if fut > bars[i].c {
            (Verdict::Realized, "peek.up")
        } else {
            (Verdict::Invalidated, "peek.down")
        })
    }
}

/// Trust path that leaks: prints the rounded `net` in an attachment on every bar.
struct LeakyTrust;

impl TrustPath for LeakyTrust {
    fn name(&self) -> &'static str {
        "test.leaky"
    }
    fn observe(&mut self, obs: &FlowObservation, _facts: &[Fact]) -> Vec<QualMessage> {
        let net = obs.values.get("net").copied().unwrap_or(0.0);
        vec![QualMessage {
            t: obs.t,
            bar: obs.bar,
            instrument: obs.instrument.clone(),
            codes: vec!["flow".to_owned()],
            tr: "Sunucu doğruladı: kurumsal akış.".to_owned(),
            en: "Server-verified: institutional flow.".to_owned(),
            trust: Trust::Server,
            shown_bar: obs.bar,
            attachments: BTreeMap::from([("hint".to_owned(), format!("{net:.0}"))]),
        }]
    }
}

// ---------------------------------------------------------------------------
// Determinism and re-scoring
// ---------------------------------------------------------------------------

#[test]
fn datasets_are_deterministic() {
    let a = small(7);
    let b = small(7);
    assert_eq!(a.inputs, b.inputs);
    let c = small(8);
    assert_ne!(a.inputs, c.inputs);
    let (e1, x1) = episodes::build(3);
    let (e2, x2) = episodes::build(3);
    assert_eq!(e1.inputs, e2.inputs);
    assert_eq!(x1, x2);
}

#[test]
fn par_map_keeps_order() {
    let v: Vec<u64> = (0..100).collect();
    let r = common::par_map(&v, |x| x * 3);
    assert_eq!(r, v.iter().map(|x| x * 3).collect::<Vec<_>>());
}

#[test]
fn m3_run_is_deterministic_and_rescoring_is_byte_identical() {
    let ds = small(1);
    let make = |_: &[InstrumentInput]| -> Box<dyn Ranker> { Box::new(control::RoundRobinRanker) };
    let a = m3::run_dataset(&ds, "control", &make);
    let b = m3::run_dataset(&ds, "control", &make);
    assert_eq!(common::to_json(&a), common::to_json(&b));
    let s1 = common::to_json(&m3::score("control", "synthetic", std::slice::from_ref(&a)));
    let back: m3::M3Raw = serde_json::from_str(&common::to_json(&a)).expect("parse");
    let s2 = common::to_json(&m3::score("control", "synthetic", &[back]));
    assert_eq!(s1, s2);
    assert!(a
        .lookahead
        .iter()
        .all(|c| c.mismatched == 0 && c.future_changed > 0));
}

#[test]
fn m3_score_counts_by_hand() {
    // Two instruments, 12 days; births: inst 0 on days 0, 2, 3 (tier 1, 1, 3), inst 1 on
    // day 9 (tier 2). Story 0 realized, 1 expired, 2 realized, 3 invalidated.
    let raw = m3::M3Raw {
        dataset: "x".into(),
        alt: "t".into(),
        rules: vec![],
        n_days: 12,
        first_day: 0,
        instruments: vec!["A".into(), "B".into()],
        regime: None,
        events: vec![
            [0, 0, 0, 1, 1],
            [1, 0, 2, 1, 1],
            [2, 0, 3, 1, 3],
            [0, 0, 4, 5, 1],
            [3, 1, 9, 1, 2],
            [1, 0, 9, 7, 1],
            [2, 0, 10, 5, 3],
            [3, 1, 11, 6, 2],
        ],
        lookahead: vec![],
    };
    let s = m3::score("t", "synthetic", &[raw]);
    let d = &s.datasets[0];
    // important: births on days 0 and 2 → mean 2/12.
    let imp = &d.levels[0];
    assert!((imp.births.mean.unwrap() - 2.0 / 12.0).abs() < 1e-12);
    assert_eq!(imp.births.max, Some(1.0));
    // repeat within 5: day 2 follows day 0 → 1 of 2.
    assert_eq!(imp.repeat5, Some(0.5));
    // all: inst 0 days 0, 2, 3 → repeats 2 of 3 (+ inst 1 day 9 → 0) = 2/4; run 2 (days 2,3).
    let all = &d.levels[2];
    assert_eq!(all.repeat5, Some(0.5));
    assert_eq!(all.longest_run, 2);
    // Outcome: tier 1 closed 2 (realized 1), tier 2 closed 1 (0), tier 3 closed 1 (1).
    assert_eq!(d.outcome.by_tier, vec![[2, 1], [1, 0], [1, 1]]);
    assert_eq!(d.outcome.diff_1_3, Some(0.5 - 1.0));
    assert_eq!(d.outcome.base, Some(0.5));
}

#[test]
fn m3_lookahead_check_catches_a_peeking_ranker() {
    let ds = small(2);
    let make = |inp: &[InstrumentInput]| -> Box<dyn Ranker> { Box::new(PeekRanker(inp.to_vec())) };
    let r = m3::run_dataset(&ds, "cheat", &make);
    assert!(
        r.lookahead.iter().any(|c| c.tier_mismatched > 0),
        "{:?}",
        r.lookahead
    );
    let s = m3::score("cheat", "synthetic", &[r]);
    assert!(!s.lookahead_ok);
}

// ---------------------------------------------------------------------------
// M5 look-ahead guard and shuffle test
// ---------------------------------------------------------------------------

#[test]
fn m5_guard_counts_long_slices() {
    let v = Rc::new(std::cell::Cell::new(0));
    let g = sim::GuardedOutcome {
        inner: Box::new(reference::RefOutcome::default()),
        violations: Rc::clone(&v),
    };
    let ds = small(1);
    let bars = &ds.inputs[0].bars;
    let (e, _) = sim::run(
        &ds.inputs[..1],
        Box::new(reference::RefTheme),
        Box::new(reference::RefOutcome::default()),
        Box::new(reference::RefRanker),
    );
    let s = e.closed_stories().first().expect("a story");
    let now = s.story.born_bar + 1;
    let n = usize::try_from(now).unwrap();
    let _ = g.judge(s, &bars[..=n], now);
    assert_eq!(v.get(), 0);
    let _ = g.judge(s, &bars[..n + 5], now);
    assert_eq!(v.get(), 1, "a slice beyond now is counted");
}

fn m5_run(ds: &common::Dataset, make: m5::DataOutcome<'_>) -> m5::M5Raw {
    let names: Vec<String> = reference::RefOutcome::perturbations()
        .into_iter()
        .map(|x| x.0)
        .collect();
    let make_pert = |name: &str| -> Box<dyn OutcomeRule> {
        reference::RefOutcome::perturbations()
            .into_iter()
            .find(|x| x.0 == name)
            .unwrap()
            .1
    };
    m5::run_dataset(ds, "t", make, &names, &make_pert)
}

#[test]
fn m5_shuffle_test_passes_for_reference_and_catches_a_peeking_rule() {
    let ds = small(3);
    let honest = |_: &[InstrumentInput]| -> Box<dyn OutcomeRule> {
        Box::new(reference::RefOutcome::default())
    };
    let r = m5_run(&ds, &honest);
    let s = m5::score_one(&r);
    assert!(s.lookahead_ok, "{:?}", r.lookahead);
    assert!(r.lookahead.iter().all(|c| c.future_changed > 0));

    let cheat =
        |inp: &[InstrumentInput]| -> Box<dyn OutcomeRule> { Box::new(PeekOutcome(inp.to_vec())) };
    let r = m5_run(&ds, &cheat);
    let s = m5::score_one(&r);
    assert!(!s.lookahead_ok);
    assert!(
        r.lookahead.iter().any(|c| c.verdict_mismatched > 0),
        "{:?}",
        r.lookahead
    );
}

#[test]
fn m5_rescoring_is_byte_identical_and_control_is_meaningless() {
    let ds = small(4);
    let make =
        |_: &[InstrumentInput]| -> Box<dyn OutcomeRule> { Box::new(control::AlwaysRealized) };
    let names = vec!["h-20%".to_owned()];
    let mp = |_: &str| -> Box<dyn OutcomeRule> { Box::new(control::AlwaysRealized) };
    let r = m5::run_dataset(&ds, "control", &make, &names, &mp);
    let r2 = m5::run_dataset(&ds, "control", &make, &names, &mp);
    assert_eq!(common::to_json(&r), common::to_json(&r2));
    let back: m5::M5Raw = serde_json::from_str(&common::to_json(&r)).unwrap();
    assert_eq!(
        common::to_json(&m5::score_one(&r)),
        common::to_json(&m5::score_one(&back))
    );
    let s = m5::score_one(&r);
    assert_eq!(s.decoy[2], Some(0.0), "always-realized: real − decoy = 0");
    assert_eq!(s.rate, Some(1.0));
}

// ---------------------------------------------------------------------------
// M4 labeling and pair metrics
// ---------------------------------------------------------------------------

#[test]
fn episodes_are_labeled_and_windows_do_not_overlap() {
    let (ds, eps) = episodes::build(1);
    assert!(eps.len() >= 200, "{} episodes", eps.len());
    for (i, e) in eps.iter().enumerate() {
        assert_eq!(e.id as usize, i);
        assert!(e.start < e.end);
        assert!((e.end as usize) <= ds.inputs[e.inst as usize].bars.len());
        for o in &eps[i + 1..] {
            if o.inst == e.inst {
                assert!(o.start >= e.end, "windows overlap");
            }
        }
        assert_eq!(
            episodes::label(&eps, e.inst, u64::from(e.start)),
            i64::from(e.id)
        );
        assert_eq!(
            episodes::label(&eps, e.inst, u64::from(e.end - 1)),
            i64::from(e.id)
        );
        assert_ne!(
            episodes::label(&eps, e.inst, u64::from(e.end)),
            i64::from(e.id)
        );
    }
    const { assert!(EP_TAIL > 0) };
    // Every template fires at least one fact in most of its episodes.
    let r = m4::run_dataset(&ds, &eps, true, "control", &|| {
        Box::new(control::SingletonTheme)
    });
    let sc = m4::score_one(&r);
    let l = sc.labeled.unwrap();
    assert!(l.episodes_with_facts * 10 >= l.episodes * 9, "{l:?}");
    // Singleton control: no reduction, no wrong merge, every pair split.
    assert_eq!(sc.reduction, Some(0.0));
    assert_eq!(l.pairs_same_story, 0);
    assert_eq!(l.split_rate, Some(1.0));
}

#[test]
fn m4_pair_metrics_by_hand() {
    // Story 0: facts of episodes 0, 0, 1 → 3 pairs, 2 wrong; story 1: episode 1 and
    // background → no labeled pair, background-mixed. Episode 0: 2 facts same story →
    // 1 pair, 0 split. Episode 1: one in story 0, one in story 1 → 1 pair, split.
    let ep = |id, start, end| episodes::Episode {
        id,
        inst: 0,
        start,
        end,
        template: 0,
        part: 0,
        dir: 0,
    };
    let raw = m4::M4Raw {
        dataset: "x".into(),
        alt: "t".into(),
        rules: vec![],
        labeled: true,
        instruments: vec!["A".into()],
        episodes: vec![ep(0, 0, 10), ep(1, 20, 30)],
        facts: vec![
            [0, 1, 2, 0, 1, 0, 0, 0, 0],
            [0, 2, 2, 0, 2, 0, 0, 0, 1],
            [0, 21, 1, 1, 1, 0, 1, 0, 1],
            [0, 22, 2, 0, 1, 0, 1, 1, 0],
            [0, 40, 2, 0, 1, 0, -1, 1, 1],
            [0, 41, 4, 0, 1, 2, -1, -1, 2],
        ],
        stories: vec![[0, 2, 0, 1, 3], [0, 2, 0, 1, 2]],
    };
    let s = m4::score_one(&raw);
    let l = s.labeled.unwrap();
    assert_eq!((l.pairs_same_story, l.pairs_wrong), (3, 2));
    assert_eq!(l.stories_mixing, 1);
    assert_eq!(l.stories_mixing_background, 1);
    assert_eq!((l.pairs_same_episode, l.pairs_split), (2, 1));
    // Lost: one ignored; fact 2 (reversal/down) mismatched in kind; story 0 drops the
    // volume family (bit 2).
    assert_eq!(s.lost.ignored, 1);
    assert_eq!(s.lost.mismatched, 1);
    assert_eq!(s.lost.kind_differs, 1);
    assert_eq!(s.lost.dropped_families, 1);
    assert_eq!(s.reduction, Some(1.0 - 2.0 / 6.0));
}

// ---------------------------------------------------------------------------
// M6
// ---------------------------------------------------------------------------

fn tiny_flow_set() -> common::Dataset {
    common::synthetic_sized(5, 1, 90)
}

#[test]
fn m6_oracle_attack_recovers_the_value_from_a_leaky_path() {
    let ds = tiny_flow_set();
    let make = || -> Box<dyn TrustPath> { Box::new(LeakyTrust) };
    let r = m6::run_dataset(&ds, 5, "leaky", &make, 10);
    assert!(!r.oracle.is_empty());
    for o in &r.oracle {
        assert!(o.truth_kept);
        // Only the true grid point survives: the value is pinned to grid resolution
        // (rounding to an integer is far finer than one grid step).
        assert_eq!(
            o.parts,
            vec![[1000, 1000]],
            "target {} truth {}",
            o.bar,
            o.truth
        );
    }
    let s = m6::score("leaky", "synthetic", &[r]);
    assert!(
        s.pooled.oracle.bits[0].unwrap() > 5.0,
        "{:?}",
        s.pooled.oracle
    );
    assert!(s.pooled.digit_attach > 0);
    assert_eq!(s.pooled.digit_text, 0);
}

#[test]
fn m6_control_is_deterministic_honest_and_leaks_less() {
    let ds = tiny_flow_set();
    let make = || -> Box<dyn TrustPath> { Box::new(control::EchoTrust) };
    let a = m6::run_dataset(&ds, 5, "control", &make, 30);
    let b = m6::run_dataset(&ds, 5, "control", &make, 30);
    assert_eq!(common::to_json(&a), common::to_json(&b));
    let s = m6::score("control", "synthetic", std::slice::from_ref(&a));
    let back: m6::M6Raw = serde_json::from_str(&common::to_json(&a)).unwrap();
    assert_eq!(
        common::to_json(&s),
        common::to_json(&m6::score("control", "synthetic", &[back]))
    );
    let p = &s.pooled;
    assert_eq!(p.server_share, Some(1.0));
    assert_eq!(p.digit_text + p.digit_codes + p.digit_attach, 0);
    assert_eq!(p.wording, [Some(1.0), Some(1.0)]);
    assert_eq!(p.utility, Some(1.0));
    assert_eq!(p.oracle.truth_not_kept, 0);
    // Echo gives an interval much wider than one grid step for ordinary bars.
    assert!(a.oracle.iter().filter(|o| !o.fact).all(|o| o
        .parts
        .iter()
        .map(|x| x[1] - x[0] + 1)
        .sum::<u32>()
        > 1));
}

#[test]
fn wording_lexicon() {
    assert!(m6::tr_verified("Sunucu doğruladı: olağandışı alış."));
    assert!(m6::tr_verified("Sunucuda teyit edildi."));
    assert!(!m6::tr_verified("Olağandışı alış."));
    assert!(m6::en_verified("Server-verified: unusual buying."));
    assert!(m6::en_verified("Verified by the server."));
    assert!(!m6::en_verified("Unusual buying."));
}

#[test]
fn raw_names_and_kinds() {
    assert_eq!(
        common::raw_file_name(DataKind::Synthetic, "synthetic-s1"),
        "synthetic-s1.json"
    );
    assert_eq!(
        common::raw_file_name(DataKind::Synthetic, "episodes-s1"),
        "synthetic-episodes-s1.json"
    );
    assert_eq!(DataKind::parse("real"), Some(DataKind::Real));
    let _ = Direction::Up;
}
