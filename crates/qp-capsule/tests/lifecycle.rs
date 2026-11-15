//! Every row of the `docs/CAPSULE.md` §9.3 transition table, the step order, ids, the
//! ledger (§9.4), retraction (K37), candidates (K28) and the glyph spec.

mod common;

use common::*;
use qp_capsule::engine::{capsule_id, finding_id, Engine, EngineConfig};
use qp_capsule::scaffold::{ConstRank, FixedHorizon, KindTheme};
use qp_capsule::{glyph_spec, OpenStory, ThemeDecision, ThemeRule};
use qp_schema::capsule::{
    Direction, Edge, EvidenceFamily, GlyphPhase, LedgerStat, MeaningKind, Stage, TimeKind,
};
use qp_schema::{Fact, Outcome};

const A: &str = "AAA";

fn engine(h: u32) -> Engine {
    Engine::new(test_config(h))
}

fn stage_reason(e: &qp_capsule::StoryEvent) -> (Stage, &str) {
    (e.transition.stage, e.transition.reason.as_str())
}

fn snap_json(e: &Engine) -> String {
    serde_json::to_string(&e.snapshot()).unwrap()
}

#[test]
fn born_new_story_with_deterministic_ids() {
    let mut e = engine(10);
    let b = bars(&[100.0]);
    let ev = e.on_bar(A, SERIES, 0, &b, &[up(A, "d1", 0)]);
    assert_eq!(ev.len(), 1);
    assert_eq!(stage_reason(&ev[0]), (Stage::Born, "new_story"));
    assert_eq!(ev[0].capsule, "cap.AAA.breakout.0");
    assert_eq!(ev[0].finding, "fnd.AAA.breakout.0");
    assert_eq!(capsule_id(A, "breakout", 0), ev[0].capsule);
    assert_eq!(finding_id(&ev[0].capsule), ev[0].finding);
    let s = &ev[0].story;
    assert_eq!(s.stage, Stage::Born);
    assert_eq!(s.families, vec![EvidenceFamily::Price]);
    assert_eq!(s.ledger_key, "breakout/d1@1");
    assert_eq!(s.ledger_at_birth, LedgerStat::default());
    assert_eq!(s.horizon_bars, 10);
    assert_eq!(ev[0].transition.facts, vec!["fx.AAA.d1.up.0".to_owned()]);
    assert_eq!(e.open_stories().len(), 1);
}

#[test]
fn strengthened_same_family() {
    let mut e = engine(10);
    let b = bars(&[100.0, 100.0]);
    e.on_bar(A, SERIES, 0, &b, &[up(A, "d1", 0)]);
    let ev = e.on_bar(A, SERIES, 1, &b, &[up(A, "d2", 1)]);
    assert_eq!(stage_reason(&ev[0]), (Stage::Strengthened, "same_family"));
    assert_eq!(ev[0].story.families, vec![EvidenceFamily::Price]);
    assert_eq!(ev[0].story.last_bar, 1);
}

#[test]
fn confirmed_new_family_grows_families() {
    let mut e = engine(10);
    let b = bars(&[100.0, 100.0]);
    e.on_bar(A, SERIES, 0, &b, &[up(A, "d1", 0)]);
    let v = fact(
        A,
        "vol",
        "spike",
        MeaningKind::Breakout,
        Direction::Up,
        EvidenceFamily::Volume,
        TimeKind::Event,
        Edge::Point,
        1,
    );
    let ev = e.on_bar(A, SERIES, 1, &b, &[v]);
    assert_eq!(stage_reason(&ev[0]), (Stage::Confirmed, "new_family"));
    assert_eq!(
        ev[0].story.families,
        vec![EvidenceFamily::Price, EvidenceFamily::Volume]
    );
    assert_eq!(glyph_spec(&ev[0].story, 1, 0).confirmations, 2);
}

#[test]
fn weakened_opposite_fact() {
    let mut e = engine(10);
    let b = bars(&[100.0, 100.0]);
    e.on_bar(A, SERIES, 0, &b, &[up(A, "d1", 0)]);
    let dn = fact(
        A,
        "d1",
        "down",
        MeaningKind::Breakout,
        Direction::Down,
        EvidenceFamily::Price,
        TimeKind::Event,
        Edge::Point,
        1,
    );
    let ev = e.on_bar(A, SERIES, 1, &b, &[dn]);
    assert_eq!(stage_reason(&ev[0]), (Stage::Weakened, "opposite_fact"));
    assert_eq!(ev[0].story.direction, Direction::Up);
}

fn state(det: &str, edge: Edge, bar: u64) -> Fact {
    fact(
        A,
        det,
        "above",
        MeaningKind::Extreme,
        Direction::Up,
        EvidenceFamily::Price,
        TimeKind::State,
        edge,
        bar,
    )
}

#[test]
fn weakened_state_ended_and_foreign_exit_ignored() {
    let mut e = engine(10);
    let b = bars(&[100.0, 100.0, 100.0]);
    let ev = e.on_bar(A, SERIES, 0, &b, &[state("rsi", Edge::Enter, 0)]);
    assert_eq!(ev[0].story.time, TimeKind::State);
    // Exit of another detector's state: joins by theme rule? The scaffold ignores it
    // (no enter of that detector), so nothing happens.
    let ev = e.on_bar(A, SERIES, 1, &b, &[state("other", Edge::Exit, 1)]);
    assert!(ev.is_empty());
    let ev = e.on_bar(A, SERIES, 2, &b, &[state("rsi", Edge::Exit, 2)]);
    assert_eq!(stage_reason(&ev[0]), (Stage::Weakened, "state_ended"));
}

/// A theme rule that always joins the first open story (to reach the engine's own
/// "exit not of the story's state" branch).
struct JoinFirst;
impl ThemeRule for JoinFirst {
    fn name(&self) -> &'static str {
        "test.join_first"
    }
    fn decide(&self, f: &Fact, open: &[&OpenStory]) -> ThemeDecision {
        match open.first() {
            Some(s) => ThemeDecision::Join(s.id.clone()),
            None => ThemeDecision::New {
                theme: f.meaning.as_ref().unwrap().kind.name().to_owned(),
            },
        }
    }
}

#[test]
fn engine_ignores_exit_of_a_state_not_in_the_story_and_unknown_join() {
    let mut e = Engine::new(EngineConfig {
        theme: Box::new(JoinFirst),
        outcome: Box::new(FixedHorizon(10)),
        ranker: Box::new(ConstRank),
    });
    let b = bars(&[100.0, 100.0]);
    e.on_bar(A, SERIES, 0, &b, &[up(A, "d1", 0)]);
    let ev = e.on_bar(A, SERIES, 1, &b, &[state("rsi", Edge::Exit, 1)]);
    assert!(ev.is_empty());
    assert_eq!(e.open_stories()[0].facts.len(), 1);
}

#[test]
fn realized_by_outcome_rule_is_a_ledger_hit() {
    let mut e = engine(10);
    let b = bars(&[100.0, 104.0, 100.0]);
    e.on_bar(A, SERIES, 0, &b, &[up(A, "d1", 0)]);
    let ev = e.on_bar(A, SERIES, 1, &b, &[]);
    assert_eq!(stage_reason(&ev[0]), (Stage::Realized, "outcome_hit"));
    assert!(e.open_stories().is_empty());
    assert_eq!(e.closed_stories().len(), 1);
    let l = e.ledger();
    assert_eq!(l.len(), 1);
    assert_eq!(l[0].outcome, Outcome::Confirmed);
    assert_eq!(l[0].stage, Some(Stage::Realized));
    assert_eq!(l[0].key.as_deref(), Some("breakout/d1@1"));
    assert_eq!(l[0].capsule.as_deref(), Some("cap.AAA.breakout.0"));
    assert_eq!(l[0].finding, "fnd.AAA.breakout.0");
    assert_eq!(l[0].bar, Some(1));
    assert_eq!(l[0].recorded_at, t_of(1));
    // Strictly before: a close at t1 does not count at t1.
    assert_eq!(e.ledger_stat("breakout/d1@1", t_of(1)).n, 0);
    assert_eq!(
        e.ledger_stat("breakout/d1@1", t_of(2)),
        LedgerStat { n: 1, realized: 1 }
    );
}

#[test]
fn invalidated_by_outcome_rule_is_a_ledger_miss() {
    let mut e = engine(10);
    let b = bars(&[100.0, 96.0]);
    e.on_bar(A, SERIES, 0, &b, &[up(A, "d1", 0)]);
    let ev = e.on_bar(A, SERIES, 1, &b, &[]);
    assert_eq!(stage_reason(&ev[0]), (Stage::Invalidated, "outcome_miss"));
    assert_eq!(e.ledger()[0].outcome, Outcome::Refuted);
    assert_eq!(e.ledger()[0].stage, Some(Stage::Invalidated));
}

#[test]
fn expired_at_horizon_is_a_ledger_miss() {
    let mut e = engine(2);
    let b = bars(&[100.0, 100.0, 100.0]);
    e.on_bar(A, SERIES, 0, &b, &[up(A, "d1", 0)]);
    assert!(e.on_bar(A, SERIES, 1, &b, &[]).is_empty());
    let ev = e.on_bar(A, SERIES, 2, &b, &[]);
    assert_eq!(stage_reason(&ev[0]), (Stage::Expired, "horizon"));
    assert_eq!(e.ledger()[0].outcome, Outcome::Refuted);
    assert_eq!(e.ledger()[0].stage, Some(Stage::Expired));
}

#[test]
fn verdict_comes_before_horizon() {
    // Horizon reached on the same bar as a hit: step 1 wins.
    let mut e = engine(1);
    let b = bars(&[100.0, 104.0]);
    e.on_bar(A, SERIES, 0, &b, &[up(A, "d1", 0)]);
    let ev = e.on_bar(A, SERIES, 1, &b, &[]);
    assert_eq!(ev.len(), 1);
    assert_eq!(ev[0].transition.stage, Stage::Realized);
}

#[test]
fn closed_story_takes_no_fact_same_theme_starts_new() {
    let mut e = engine(10);
    let b = bars(&[100.0, 104.0]);
    e.on_bar(A, SERIES, 0, &b, &[up(A, "d1", 0)]);
    let ev = e.on_bar(A, SERIES, 1, &b, &[up(A, "d1", 1)]);
    // (1) realized first, then (3) the fact starts a new story.
    assert_eq!(ev.len(), 2);
    assert_eq!(ev[0].transition.stage, Stage::Realized);
    assert_eq!(ev[0].capsule, "cap.AAA.breakout.0");
    assert_eq!(ev[1].transition.stage, Stage::Born);
    assert_eq!(ev[1].capsule, "cap.AAA.breakout.1");
    // The close on this bar is not visible at this bar's birth.
    assert_eq!(ev[1].story.ledger_at_birth.n, 0);
    let b2 = bars(&[100.0, 104.0, 104.0, 108.0, 100.0]);
    e.on_bar(A, SERIES, 2, &b2, &[]);
    e.on_bar(A, SERIES, 3, &b2, &[]); // cap.1 realized at bar 3
    let ev = e.on_bar(A, SERIES, 4, &b2, &[up(A, "d1", 4)]);
    assert_eq!(
        ev[0].story.ledger_at_birth,
        LedgerStat { n: 2, realized: 2 }
    );
}

#[test]
fn facts_processed_in_detector_code_order() {
    let mut e = engine(10);
    let b = bars(&[100.0]);
    let v = fact(
        A,
        "a.vol",
        "spike",
        MeaningKind::Breakout,
        Direction::Up,
        EvidenceFamily::Volume,
        TimeKind::Event,
        Edge::Point,
        0,
    );
    // Given in reverse; "a.vol" < "z.price" so the volume fact gives birth.
    let ev = e.on_bar(A, SERIES, 0, &b, &[up(A, "z.price", 0), v]);
    assert_eq!(ev.len(), 2);
    assert_eq!(ev[0].transition.stage, Stage::Born);
    assert_eq!(ev[0].story.ledger_key, "breakout/a.vol@1");
    assert_eq!(stage_reason(&ev[1]), (Stage::Confirmed, "new_family"));
    // Bar-level rank is the same on both snapshots.
    assert_eq!(ev[0].story.importance, ev[1].story.importance);
}

#[test]
fn new_with_existing_id_joins_that_story() {
    struct AlwaysNew;
    impl ThemeRule for AlwaysNew {
        fn name(&self) -> &'static str {
            "test.always_new"
        }
        fn decide(&self, _f: &Fact, _o: &[&OpenStory]) -> ThemeDecision {
            ThemeDecision::New {
                theme: "t".to_owned(),
            }
        }
    }
    let mut e = Engine::new(EngineConfig {
        theme: Box::new(AlwaysNew),
        outcome: Box::new(FixedHorizon(10)),
        ranker: Box::new(ConstRank),
    });
    let b = bars(&[100.0]);
    let ev = e.on_bar(A, SERIES, 0, &b, &[up(A, "d1", 0), up(A, "d2", 0)]);
    assert_eq!(ev.len(), 2);
    assert_eq!(ev[1].transition.stage, Stage::Strengthened);
    assert_eq!(e.open_stories().len(), 1);
}

#[test]
fn foreign_instrument_series_bar_and_meaningless_facts_are_skipped() {
    let mut e = engine(10);
    let b = bars(&[100.0, 100.0]);
    let mut other_series = up(A, "d1", 1);
    other_series.series = "1h".to_owned();
    let mut no_meaning = up(A, "d2", 1);
    no_meaning.meaning = None;
    let facts = [up("BBB", "d1", 1), up(A, "d1", 0), other_series, no_meaning];
    assert!(e.on_bar(A, SERIES, 1, &b, &facts).is_empty());
}

#[test]
fn ranker_sees_no_look_ahead_and_ranks_closes() {
    let mut e = engine(10);
    let b = bars(&[100.0, 104.0, 104.0]);
    let ev0 = e.on_bar(A, SERIES, 0, &b, &[up(A, "d1", 0)]);
    assert_eq!(ev0[0].story.tier, 1); // stories_all = 0
    let ev1 = e.on_bar(A, SERIES, 1, &b, &[up(A, "d1", 1)]);
    // The close at bar 1 is not in the ledger the ranker sees at bar 1.
    assert!(ev1.iter().all(|x| x.story.importance < 10.0));
    assert_eq!(ev1[1].story.tier, 2); // second story: stories_all = 1
    let ev2 = e.on_bar(A, SERIES, 2, &b, &[up(A, "d1", 2)]);
    assert_eq!(ev2[0].transition.stage, Stage::Strengthened);
    // Now the bar-1 close is known: ledger.n = 1, realized = 1.
    assert!(ev2[0].story.importance >= 1100.0);
}

#[test]
fn retracted_open_story_never_in_ledger() {
    let mut e = engine(10);
    let b = bars(&[100.0, 100.0]);
    e.on_bar(A, SERIES, 0, &b, &[up(A, "d1", 0)]);
    let ev = e
        .retract("cap.AAA.breakout.0", 1, t_of(1), "data_corrected")
        .unwrap();
    assert_eq!(stage_reason(&ev), (Stage::Retracted, "data_corrected"));
    assert!(e.open_stories().is_empty());
    assert!(e.ledger().is_empty());
    assert!(e.ledger_records().is_empty());
    assert_eq!(e.ledger_stat("breakout/d1@1", i64::MAX).n, 0);
    // Twice, unknown, or back in time: refused.
    assert!(e.retract("cap.AAA.breakout.0", 1, t_of(1), "x").is_none());
    assert!(e.retract("cap.nope", 1, t_of(1), "x").is_none());
    assert_eq!(glyph_spec(&ev.story, 1, 0).phase, GlyphPhase::Retracted);
}

#[test]
fn retracting_a_closed_story_does_not_change_earlier_births() {
    let key = "breakout/d1@1";
    let mut e = engine(10);
    // bar 0 born, bar 1 realized (close t1), bar 2 born B (sees n=1),
    // retract the first at bar 3, bar 4 born C (sees n=0).
    let b = bars(&[100.0, 104.0, 104.0, 104.0, 104.0, 104.0]);
    e.on_bar(A, SERIES, 0, &b, &[up(A, "d1", 0)]);
    e.on_bar(A, SERIES, 1, &b, &[]);
    let born_b = e.on_bar(A, SERIES, 2, &b, &[up(A, "d1", 2)]);
    assert_eq!(born_b[0].story.ledger_at_birth.n, 1);
    let before = serde_json::to_string(&born_b).unwrap();
    let r = e
        .retract("cap.AAA.breakout.0", 3, t_of(3), "dispute_upheld")
        .unwrap();
    assert_eq!(r.transition.stage, Stage::Retracted);
    assert_eq!(r.story.history.len(), 3); // born, realized, retracted
    let born_c = e.on_bar(A, SERIES, 4, &b, &[]); // nothing; B still open
    assert!(born_c.is_empty());
    // B's birth event is unchanged in the history.
    let hist_b: Vec<_> = e
        .events()
        .iter()
        .filter(|x| x.capsule == "cap.AAA.breakout.2")
        .cloned()
        .collect();
    assert_eq!(serde_json::to_string(&hist_b).unwrap(), before);
    // Point in time: questions at or before the retraction see the row, later ones not.
    assert_eq!(e.ledger_stat(key, t_of(2)).n, 1);
    assert_eq!(e.ledger_stat(key, t_of(3)).n, 1);
    assert_eq!(e.ledger_stat(key, t_of(3) + 1).n, 0);
    assert!(e.ledger().is_empty());
    assert_eq!(e.ledger_records().len(), 1);
    assert_eq!(e.ledger_records()[0].retracted_at, Some(t_of(3)));
    // A new birth after the retraction no longer counts it.
    let mut e2 = engine(10);
    e2.on_bar(A, SERIES, 0, &b, &[up(A, "d1", 0)]);
    e2.on_bar(A, SERIES, 1, &b, &[]);
    e2.retract("cap.AAA.breakout.0", 3, t_of(3), "data_corrected");
    let ev = e2.on_bar(A, SERIES, 4, &b, &[up(A, "d1", 4)]);
    assert_eq!(ev[0].story.ledger_at_birth.n, 0);
}

#[test]
fn candidates_are_pure_and_never_in_ledger() {
    let mut e = engine(10);
    let b = bars(&[100.0, 104.0, 104.0, 104.0]);
    e.on_bar(A, SERIES, 0, &b, &[up(A, "d1", 0)]);
    e.on_bar(A, SERIES, 1, &b, &[]); // realized → one ledger row
    e.on_bar(A, SERIES, 2, &b, &[up(A, "d1", 2)]); // open story
    let before = snap_json(&e);
    let ledger_before = e.ledger();

    let vol = fact(
        A,
        "vol",
        "spike",
        MeaningKind::Breakout,
        Direction::Up,
        EvidenceFamily::Volume,
        TimeKind::Event,
        Edge::Point,
        3,
    );
    let ext = state("rsi", Edge::Enter, 3);
    let c = e.candidates(A, SERIES, 3, &b, &[vol, ext]);
    assert_eq!(c.len(), 2);
    // Join of the open story: would be confirmed.
    let j = c
        .iter()
        .find(|x| x.capsule == "cap.AAA.breakout.2")
        .unwrap();
    assert_eq!(j.story.stage, Stage::Candidate);
    assert_eq!(j.transition.stage, Stage::Confirmed);
    assert_eq!(j.story.history.last().unwrap().stage, Stage::Candidate);
    assert_eq!(j.story.history[0].stage, Stage::Born);
    // Birth: would be born.
    let n = c.iter().find(|x| x.capsule == "cap.AAA.extreme.3").unwrap();
    assert_eq!(n.transition.stage, Stage::Born);
    assert_eq!(n.story.stage, Stage::Candidate);
    assert_eq!(n.story.history.len(), 1);
    assert_eq!(glyph_spec(&n.story, 3, 0).phase, GlyphPhase::Candidate);

    assert_eq!(snap_json(&e), before);
    assert_eq!(e.ledger(), ledger_before);
    // Candidates are not in the event list either.
    assert!(e.events().iter().all(|x| x.story.stage != Stage::Candidate));
}

#[test]
fn candidates_skip_stories_that_would_close() {
    let mut e = engine(10);
    let b = bars(&[100.0, 104.0]);
    e.on_bar(A, SERIES, 0, &b, &[up(A, "d1", 0)]);
    let before = snap_json(&e);
    // On bar 1 the open story would be realized; the provisional fact starts a new one.
    let c = e.candidates(A, SERIES, 1, &b, &[up(A, "d1", 1)]);
    assert_eq!(c.len(), 1);
    assert_eq!(c[0].capsule, "cap.AAA.breakout.1");
    assert_eq!(snap_json(&e), before);
    assert!(e.ledger().is_empty());
}

#[test]
fn glyph_spec_channels() {
    let mut e = engine(10);
    let b = bars(&[100.0, 104.0, 100.0]);
    e.on_bar(A, SERIES, 0, &b, &[up(A, "d1", 0)]);
    e.on_bar(A, SERIES, 1, &b, &[]);
    let ev = e.on_bar(A, SERIES, 2, &b, &[up(A, "d1", 2)]);
    let s = &ev[0].story;
    let g = glyph_spec(s, 2, 1);
    assert_eq!(g.confidence, Some(1.0));
    assert_eq!(g.freshness, 1.0);
    assert_eq!(g.phase, GlyphPhase::Final);
    assert!(!g.server);
    assert_eq!(glyph_spec(s, 2, 2).confidence, None);
    assert_eq!(glyph_spec(s, 7, 1).freshness, 0.5);
    assert!(glyph_spec(s, 100, 1).freshness < glyph_spec(s, 8, 1).freshness);
}

#[test]
fn theme_rule_sees_only_same_instrument_and_series() {
    struct Check;
    impl ThemeRule for Check {
        fn name(&self) -> &'static str {
            "test.check"
        }
        fn decide(&self, f: &Fact, open: &[&OpenStory]) -> ThemeDecision {
            let m = f.meaning.as_ref().unwrap();
            assert!(open
                .iter()
                .all(|s| s.story.instrument == m.instrument && s.story.series == f.series));
            KindTheme.decide(f, open)
        }
    }
    let mut e = Engine::new(EngineConfig {
        theme: Box::new(Check),
        outcome: Box::new(FixedHorizon(10)),
        ranker: Box::new(ConstRank),
    });
    let b = bars(&[100.0, 100.0]);
    e.on_bar(A, SERIES, 0, &b, &[up(A, "d1", 0)]);
    e.on_bar("BBB", SERIES, 0, &b, &[up("BBB", "d1", 0)]);
    e.on_bar(A, SERIES, 1, &b, &[up(A, "d1", 1)]);
    e.on_bar("BBB", SERIES, 1, &b, &[up("BBB", "d1", 1)]);
    assert_eq!(e.open_stories().len(), 2);
    assert_eq!(e.open_stories()[0].story.instrument, A);
}
