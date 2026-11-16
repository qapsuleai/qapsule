//! Behaviour of the "arc" theme rule through the real engine (scaffold outcome: never
//! judges, long horizon; so every story stays open and only the theme rule groups).

use std::collections::BTreeMap;

use m4_arc::{effective_direction, sealed, theme, theme_with, ArcConfig, WeakOpposite};
use qp_capsule::scaffold::{ConstRank, FixedHorizon};
use qp_capsule::{Engine, EngineConfig, OpenStory, ThemeDecision};
use qp_schema::capsule::{
    Direction, Edge, EvidenceFamily, FactMeaning, License, LicenseClass, MeaningKind, Stage,
    TimeKind,
};
use qp_schema::{Author, Bar, Fact, Provenance};

const INST: &str = "AAA";
const SERIES: &str = "1d";

fn t_of(i: u64) -> i64 {
    1_000 + i64::try_from(i).unwrap() * 60_000
}

fn bars(n: u64) -> Vec<Bar> {
    (0..n)
        .map(|i| Bar {
            t: t_of(i),
            o: 100.0,
            h: 100.0,
            l: 100.0,
            c: 100.0,
            v: 1.0,
        })
        .collect()
}

#[derive(Clone, Copy)]
enum D {
    Up,
    Down,
    Neutral,
}

fn dir(d: D) -> Direction {
    match d {
        D::Up => Direction::Up,
        D::Down => Direction::Down,
        D::Neutral => Direction::Neutral,
    }
}

/// A fact of the real detectors' shape.
fn fact(det: &str, code: &str, kind: MeaningKind, d: D, edge: Edge, bar: u64) -> Fact {
    let family = match det {
        "flow.volume_surge" => EvidenceFamily::Volume,
        "flow.institutional" => EvidenceFamily::Flow,
        _ => EvidenceFamily::Price,
    };
    let time = match edge {
        Edge::Point => TimeKind::Event,
        _ => TimeKind::State,
    };
    Fact {
        id: format!("fx.{INST}.{det}.{code}.{bar}"),
        kind: format!("{det}.{code}"),
        series: SERIES.to_owned(),
        bar,
        t: t_of(bar),
        text: String::new(),
        evidence: BTreeMap::new(),
        evidence_ref: None,
        meaning: Some(FactMeaning {
            kind,
            direction: dir(d),
            family,
            time,
            edge,
            detector: det.to_owned(),
            detector_version: 1,
            instrument: INST.to_owned(),
            strength: 0.5,
            license: License {
                class: LicenseClass::Public,
                entitlement: None,
            },
            geometry: Vec::new(),
            condition: None,
            server: det == "flow.institutional",
        }),
        provenance: Provenance {
            author: Author::Engine {
                name: "test".to_owned(),
                version: "0".to_owned(),
            },
            at: t_of(bar),
            based_on: Vec::new(),
            approved_by: None,
        },
    }
}

fn brk(d: D, bar: u64) -> Fact {
    let code = if matches!(d, D::Up) { "up" } else { "down" };
    fact(
        "breakout.range",
        code,
        MeaningKind::Breakout,
        d,
        Edge::Point,
        bar,
    )
}
fn surge(d: D, bar: u64) -> Fact {
    fact(
        "flow.volume_surge",
        "surge",
        MeaningKind::Flow,
        d,
        Edge::Point,
        bar,
    )
}
fn inst_flow(d: D, bar: u64) -> Fact {
    let code = if matches!(d, D::Up) {
        "net_buy"
    } else {
        "net_sell"
    };
    fact(
        "flow.institutional",
        code,
        MeaningKind::Flow,
        d,
        Edge::Point,
        bar,
    )
}
fn bull(bar: u64) -> Fact {
    fact(
        "continuation.ma_stack",
        "bull",
        MeaningKind::Continuation,
        D::Up,
        Edge::Enter,
        bar,
    )
}
fn bull_end(bar: u64) -> Fact {
    fact(
        "continuation.ma_stack",
        "bull_end",
        MeaningKind::Continuation,
        D::Up,
        Edge::Exit,
        bar,
    )
}
fn bearish_div(bar: u64) -> Fact {
    fact(
        "reversal.rsi_divergence",
        "bearish",
        MeaningKind::Reversal,
        D::Down,
        Edge::Point,
        bar,
    )
}
fn overbought(bar: u64) -> Fact {
    fact(
        "extreme.rsi",
        "overbought",
        MeaningKind::Extreme,
        D::Up,
        Edge::Enter,
        bar,
    )
}
fn squeeze(bar: u64) -> Fact {
    fact(
        "squeeze.bandwidth",
        "squeeze",
        MeaningKind::Squeeze,
        D::Neutral,
        Edge::Enter,
        bar,
    )
}
fn squeeze_end(bar: u64) -> Fact {
    fact(
        "squeeze.bandwidth",
        "squeeze_end",
        MeaningKind::Squeeze,
        D::Neutral,
        Edge::Exit,
        bar,
    )
}

/// Run the engine with the arc rule over `facts` (any order); every story stays open.
fn run_with(cfg: Option<ArcConfig>, facts: &[Fact]) -> Engine {
    let n = facts.iter().map(|f| f.bar).max().unwrap_or(0) + 1;
    let b = bars(n);
    let mut e = Engine::new(EngineConfig {
        theme: cfg.map_or_else(theme, theme_with),
        outcome: Box::new(FixedHorizon(1_000)),
        ranker: Box::new(ConstRank),
    });
    for i in 0..n {
        let fs: Vec<Fact> = facts.iter().filter(|f| f.bar == i).cloned().collect();
        e.on_bar(INST, SERIES, i, &b, &fs);
    }
    e
}

fn run(facts: &[Fact]) -> Engine {
    run_with(None, facts)
}

/// Fact ids per story, in birth order.
fn groups(e: &Engine) -> Vec<Vec<String>> {
    e.open_stories()
        .iter()
        .chain(e.closed_stories())
        .map(|s| s.facts.iter().map(|f| f.id.clone()).collect())
        .collect()
}

fn story_of<'a>(e: &'a Engine, fact_id: &str) -> &'a OpenStory {
    e.open_stories()
        .iter()
        .find(|s| s.facts.iter().any(|f| f.id == fact_id))
        .expect("fact in a story")
}

#[test]
fn same_direction_chapters_form_one_arc() {
    let fs = [
        brk(D::Up, 10),
        surge(D::Up, 10),
        bull(12),
        brk(D::Up, 13),
        overbought(15),
        inst_flow(D::Up, 18),
    ];
    let e = run(&fs);
    assert_eq!(e.open_stories().len(), 1, "{:?}", groups(&e));
    let s = &e.open_stories()[0];
    assert_eq!(s.facts.len(), 6);
    // The kind is the opening chapter's; families grow with each new family.
    assert_eq!(s.story.kind, MeaningKind::Breakout);
    assert_eq!(s.story.direction, Direction::Up);
    assert_eq!(
        s.story.families,
        vec![
            EvidenceFamily::Price,
            EvidenceFamily::Volume,
            EvidenceFamily::Flow
        ]
    );
    assert!(s.id.starts_with("cap.AAA.arc.up."));
    let stages: Vec<Stage> = s.story.history.iter().map(|t| t.stage).collect();
    assert_eq!(
        stages,
        vec![
            Stage::Born,
            Stage::Confirmed,
            Stage::Strengthened,
            Stage::Strengthened,
            Stage::Strengthened,
            Stage::Confirmed
        ]
    );
}

#[test]
fn gap_ends_the_arc() {
    // Default gap is 5 bars between the arc's last chapter and the new fact.
    let e = run(&[brk(D::Up, 10), brk(D::Up, 15), brk(D::Up, 21)]);
    assert_eq!(
        groups(&e),
        vec![
            vec![
                "fx.AAA.breakout.range.up.10".to_owned(),
                "fx.AAA.breakout.range.up.15".to_owned()
            ],
            vec!["fx.AAA.breakout.range.up.21".to_owned()],
        ]
    );
    // A wider gap keeps it.
    let wide = ArcConfig {
        gap: 6,
        ..ArcConfig::default()
    };
    let e = run_with(
        Some(wide),
        &[brk(D::Up, 10), brk(D::Up, 15), brk(D::Up, 21)],
    );
    assert_eq!(e.open_stories().len(), 1);
}

#[test]
fn opposite_breakout_breaks_and_old_arc_is_not_resumed() {
    let e = run(&[brk(D::Up, 10), brk(D::Down, 12), brk(D::Up, 13)]);
    let g = groups(&e);
    assert_eq!(g.len(), 3, "{g:?}");
    assert!(g.iter().all(|x| x.len() == 1));
}

#[test]
fn opposite_reversal_is_the_last_chapter() {
    let e = run(&[brk(D::Up, 10), bearish_div(12), brk(D::Up, 13)]);
    let g = groups(&e);
    assert_eq!(g.len(), 2, "{g:?}");
    let a = story_of(&e, "fx.AAA.breakout.range.up.10");
    assert_eq!(a.facts.len(), 2);
    assert_eq!(a.story.stage, Stage::Weakened);
    assert_eq!(a.story.history[1].reason, "opposite_fact");
    assert!(sealed(a));
    // The next up breakout starts a new arc.
    let b = story_of(&e, "fx.AAA.breakout.range.up.13");
    assert_eq!(b.facts.len(), 1);
}

#[test]
fn opposite_weak_fact_gets_a_counter_story_and_the_arc_goes_on() {
    let e = run(&[
        brk(D::Up, 10),
        inst_flow(D::Down, 11),
        surge(D::Down, 12),
        brk(D::Up, 13),
    ]);
    let a = story_of(&e, "fx.AAA.breakout.range.up.10");
    assert_eq!(a.facts.len(), 2, "{:?}", groups(&e));
    assert!(a
        .facts
        .iter()
        .any(|f| f.id == "fx.AAA.breakout.range.up.13"));
    let c1 = story_of(&e, "fx.AAA.flow.institutional.net_sell.11");
    assert_eq!(c1.story.direction, Direction::Down);
    assert!(c1.id.starts_with("cap.AAA.side.flow.down."));
    // Volume and institutional flow are both kind flow, direction down: one counter story.
    assert_eq!(c1.facts.len(), 2);
}

#[test]
fn weak_opposite_join_mode_weakens_the_arc() {
    let cfg = ArcConfig {
        weak_opposite: WeakOpposite::Join,
        ..ArcConfig::default()
    };
    let e = run_with(Some(cfg), &[brk(D::Up, 10), inst_flow(D::Down, 11)]);
    assert_eq!(e.open_stories().len(), 1);
    assert_eq!(e.open_stories()[0].story.stage, Stage::Weakened);
    // The counter fact itself is in the arc: the engine keeps the arc's direction.
    let s = &e.open_stories()[0];
    assert_eq!(s.story.direction, Direction::Up);
    assert_eq!(s.story.history[1].reason, "opposite_fact");
}

#[test]
fn squeeze_prelude_resolves_into_the_arc() {
    let e = run(&[
        squeeze(10),
        brk(D::Up, 18),
        surge(D::Up, 18),
        brk(D::Up, 19),
        squeeze_end(21),
    ]);
    assert_eq!(e.open_stories().len(), 1, "{:?}", groups(&e));
    let s = &e.open_stories()[0];
    assert_eq!(s.story.kind, MeaningKind::Squeeze);
    // The engine keeps the birth direction; the rule reads the arc's direction.
    assert_eq!(s.story.direction, Direction::Neutral);
    assert_eq!(effective_direction(s), Direction::Up);
    assert_eq!(s.story.stage, Stage::Weakened);
    assert_eq!(s.story.history.last().unwrap().reason, "state_ended");
    // A resolved prelude is an up arc: a down breakout breaks it.
    let e = run(&[squeeze(10), brk(D::Up, 18), brk(D::Down, 20)]);
    assert_eq!(e.open_stories().len(), 2);
}

#[test]
fn prelude_is_not_resolved_by_a_weak_fact_or_after_its_window() {
    // A flow fact alone does not decide the squeeze's direction.
    let e = run(&[squeeze(10), surge(D::Up, 12)]);
    assert_eq!(e.open_stories().len(), 2);
    // Beyond prelude_gap (10) the breakout starts its own arc.
    let e = run(&[squeeze(10), brk(D::Up, 21)]);
    assert_eq!(e.open_stories().len(), 2);
    let cfg = ArcConfig {
        prelude: false,
        ..ArcConfig::default()
    };
    let e = run_with(Some(cfg), &[squeeze(10), brk(D::Up, 12)]);
    assert_eq!(e.open_stories().len(), 2);
}

#[test]
fn squeeze_inside_a_move_is_a_pause_of_the_arc() {
    let e = run(&[brk(D::Up, 10), squeeze(12), brk(D::Up, 14)]);
    assert_eq!(e.open_stories().len(), 1);
}

#[test]
fn weak_arc_is_taken_over_by_a_strong_chapter() {
    let e = run(&[inst_flow(D::Up, 10), brk(D::Up, 11), brk(D::Up, 12)]);
    assert_eq!(e.open_stories().len(), 1);
    assert_eq!(e.open_stories()[0].story.kind, MeaningKind::Flow);
}

#[test]
fn exits_go_to_their_state_and_seal_a_trend_arc() {
    let e = run(&[
        brk(D::Up, 10),
        bull(11),
        bull_end(14),
        brk(D::Up, 15),
        fact(
            "extreme.rsi",
            "oversold_end",
            MeaningKind::Extreme,
            D::Down,
            Edge::Exit,
            16,
        ),
    ]);
    let a = story_of(&e, "fx.AAA.breakout.range.up.10");
    assert_eq!(a.facts.len(), 3);
    assert_eq!(a.story.history[2].reason, "state_ended");
    assert!(sealed(a));
    // After the trend ended the next breakout is a new arc; the orphan exit is ignored.
    assert_eq!(e.open_stories().len(), 2);
    let all: usize = e.open_stories().iter().map(|s| s.facts.len()).sum();
    assert_eq!(all, 4);
}

#[test]
fn decide_is_pure_and_runs_are_identical() {
    let fs = [
        squeeze(5),
        brk(D::Up, 10),
        surge(D::Up, 10),
        bull(12),
        inst_flow(D::Down, 13),
        bearish_div(15),
        brk(D::Down, 16),
        brk(D::Down, 30),
    ];
    let a = run(&fs).snapshot();
    let b = run(&fs).snapshot();
    assert_eq!(a, b);
    let e = run(&fs[..4]);
    let open: Vec<&OpenStory> = e.open_stories().iter().collect();
    let rule = theme();
    let probe = brk(D::Up, 13);
    let d1 = rule.decide(&probe, &open);
    let d2 = rule.decide(&probe, &open);
    assert_eq!(d1, d2);
    assert!(matches!(d1, ThemeDecision::Join(_)));
    assert_eq!(rule.name(), "arc");
}

#[test]
fn facts_without_meaning_are_ignored() {
    let mut f = brk(D::Up, 3);
    f.meaning = None;
    assert_eq!(theme().decide(&f, &[]), ThemeDecision::Ignore);
}
