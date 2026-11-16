//! Unit tests: the rule run inside the real engine (scaffold outcome = long fixed horizon,
//! so stories stay open and only the theme rule decides).

use std::collections::BTreeMap;

use qp_capsule::engine::{Engine, EngineConfig};
use qp_capsule::scaffold::{ConstRank, FixedHorizon};
use qp_capsule::{OpenStory, ThemeDecision, ThemeRule};
use qp_schema::capsule::{
    Direction, Edge, EvidenceFamily, FactMeaning, License, LicenseClass, MeaningKind, Stage,
    TimeKind,
};
use qp_schema::{Author, Bar, Fact, Provenance};

use super::KindDir;

const SER: &str = "1d";
const INST: &str = "AAA";

fn t_of(i: u64) -> i64 {
    1_000 + i64::try_from(i).unwrap() * 60_000
}

#[allow(clippy::too_many_arguments)]
fn fact(
    det: &str,
    code: &str,
    kind: MeaningKind,
    dir: Direction,
    fam: EvidenceFamily,
    edge: Edge,
    bar: u64,
) -> Fact {
    Fact {
        id: format!("fx.{INST}.{det}.{code}.{bar}"),
        kind: format!("{det}.{code}"),
        series: SER.to_owned(),
        bar,
        t: t_of(bar),
        text: String::new(),
        evidence: BTreeMap::new(),
        evidence_ref: None,
        meaning: Some(FactMeaning {
            kind,
            direction: dir,
            family: fam,
            time: if edge == Edge::Point {
                TimeKind::Event
            } else {
                TimeKind::State
            },
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
            server: false,
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

fn brk(dir: Direction, bar: u64) -> Fact {
    let code = if dir == Direction::Up { "up" } else { "down" };
    fact(
        "breakout.range",
        code,
        MeaningKind::Breakout,
        dir,
        EvidenceFamily::Price,
        Edge::Point,
        bar,
    )
}

fn vol(dir: Direction, bar: u64) -> Fact {
    fact(
        "flow.volume_surge",
        "surge",
        MeaningKind::Flow,
        dir,
        EvidenceFamily::Volume,
        Edge::Point,
        bar,
    )
}

fn inst_flow(dir: Direction, bar: u64) -> Fact {
    let code = if dir == Direction::Up {
        "net_buy"
    } else {
        "net_sell"
    };
    fact(
        "flow.institutional",
        code,
        MeaningKind::Flow,
        dir,
        EvidenceFamily::Flow,
        Edge::Point,
        bar,
    )
}

fn ob(edge: Edge, bar: u64) -> Fact {
    let code = if edge == Edge::Enter {
        "overbought"
    } else {
        "overbought_end"
    };
    fact(
        "extreme.rsi",
        code,
        MeaningKind::Extreme,
        Direction::Up,
        EvidenceFamily::Price,
        edge,
        bar,
    )
}

fn sq(edge: Edge, bar: u64) -> Fact {
    let code = if edge == Edge::Enter {
        "squeeze"
    } else {
        "squeeze_end"
    };
    fact(
        "squeeze.bandwidth",
        code,
        MeaningKind::Squeeze,
        Direction::Neutral,
        EvidenceFamily::Price,
        edge,
        bar,
    )
}

/// Run the engine over `n` flat bars with the given facts; return (open + closed) stories.
fn run(rule: KindDir, n: u64, facts: &[Fact]) -> Vec<OpenStory> {
    let bars: Vec<Bar> = (0..n)
        .map(|i| Bar {
            t: t_of(i),
            o: 100.0,
            h: 100.0,
            l: 100.0,
            c: 100.0,
            v: 1.0,
        })
        .collect();
    let mut e = Engine::new(EngineConfig {
        theme: Box::new(rule),
        outcome: Box::new(FixedHorizon(1_000)),
        ranker: Box::new(ConstRank),
    });
    for i in 0..n {
        let fs: Vec<Fact> = facts.iter().filter(|f| f.bar == i).cloned().collect();
        e.on_bar(INST, SER, i, &bars, &fs);
    }
    let mut all: Vec<OpenStory> = e.open_stories().to_vec();
    all.extend(e.closed_stories().iter().cloned());
    all
}

fn find<'a>(ss: &'a [OpenStory], fact_id: &str) -> &'a OpenStory {
    ss.iter()
        .find(|s| s.facts.iter().any(|f| f.id == fact_id))
        .expect("fact is in a story")
}

#[test]
fn name_and_entry_point() {
    assert_eq!(super::theme().name(), "kinddir");
}

#[test]
fn same_kind_direction_joins_and_keys_theme() {
    let fs = [
        brk(Direction::Up, 30),
        brk(Direction::Up, 31),
        brk(Direction::Up, 35),
    ];
    let ss = run(KindDir::default(), 40, &fs);
    assert_eq!(ss.len(), 1);
    assert_eq!(ss[0].story.theme, "breakout.up");
    assert_eq!(ss[0].id, "cap.AAA.breakout.up.30");
    assert_eq!(ss[0].facts.len(), 3);
    assert_eq!(ss[0].story.stage, Stage::Strengthened);
}

#[test]
fn gap_starts_a_new_story() {
    // 30 → 40 is exactly the gap (joins); 40 → 51 is past it (new story).
    let fs = [
        brk(Direction::Up, 30),
        brk(Direction::Up, 40),
        brk(Direction::Up, 51),
    ];
    let ss = run(KindDir::default(), 60, &fs);
    assert_eq!(ss.len(), 2);
    assert_eq!(find(&ss, &fs[1].id).id, "cap.AAA.breakout.up.30");
    assert_eq!(find(&ss, &fs[2].id).id, "cap.AAA.breakout.up.51");
}

#[test]
fn later_fact_joins_most_recent_of_two_open_stories() {
    let fs = [
        brk(Direction::Up, 30),
        brk(Direction::Up, 45), // new (gap), first story still open
        brk(Direction::Up, 50),
    ];
    let ss = run(KindDir::default(), 60, &fs);
    assert_eq!(find(&ss, &fs[2].id).id, "cap.AAA.breakout.up.45");
}

#[test]
fn opposite_direction_same_kind_is_separate() {
    let fs = [brk(Direction::Up, 30), brk(Direction::Down, 31)];
    let ss = run(KindDir::default(), 40, &fs);
    assert_eq!(ss.len(), 2);
    let up = find(&ss, &fs[0].id);
    assert_eq!(up.facts.len(), 1);
    assert_eq!(up.story.stage, Stage::Born);
    assert_eq!(find(&ss, &fs[1].id).story.theme, "breakout.down");
}

#[test]
fn volume_on_breakout_bar_confirms() {
    let fs = [brk(Direction::Up, 30), vol(Direction::Up, 30)];
    let ss = run(KindDir::default(), 40, &fs);
    assert_eq!(ss.len(), 1);
    let s = &ss[0];
    assert_eq!(s.story.stage, Stage::Confirmed);
    assert_eq!(
        s.story.families,
        vec![EvidenceFamily::Price, EvidenceFamily::Volume]
    );
}

#[test]
fn flow_window_and_institutional_third_family() {
    let fs = [
        brk(Direction::Up, 30),
        vol(Direction::Up, 31),       // next bar → confirms
        inst_flow(Direction::Up, 31), // same bar → third family
        vol(Direction::Up, 32),       // 2 bars after the breakout → own flow story
    ];
    let ss = run(KindDir::default(), 40, &fs);
    let b = find(&ss, &fs[0].id);
    assert_eq!(b.facts.len(), 3);
    assert_eq!(b.story.families.len(), 3);
    let f = find(&ss, &fs[3].id);
    assert_eq!(f.story.kind, MeaningKind::Flow);
    assert_eq!(f.story.theme, "flow.up");
}

#[test]
fn flow_does_not_keep_price_story_alive() {
    // Volume at 31 confirms; volume at 32 is 2 bars after the last breakout → own story
    // (the joined flow fact does not extend the confirmation window).
    let fs = [
        brk(Direction::Up, 30),
        vol(Direction::Up, 31),
        vol(Direction::Up, 32),
    ];
    let ss = run(KindDir::default(), 40, &fs);
    assert_eq!(find(&ss, &fs[2].id).story.kind, MeaningKind::Flow);
}

#[test]
fn opposite_flow_weakens_or_stays_apart() {
    let fs = [brk(Direction::Up, 30), vol(Direction::Down, 30)];
    let ss = run(KindDir::default(), 40, &fs);
    assert_eq!(ss.len(), 1);
    assert_eq!(ss[0].story.stage, Stage::Weakened);
    assert_eq!(ss[0].story.history.last().unwrap().reason, "opposite_fact");

    let strict_opp = KindDir {
        flow_opposite_weakens: false,
        ..KindDir::default()
    };
    let ss = run(strict_opp, 40, &fs);
    assert_eq!(ss.len(), 2);
}

#[test]
fn strict_variant_never_crosses_kinds() {
    let strict = KindDir {
        flow_confirms: false,
        ..KindDir::default()
    };
    let fs = [brk(Direction::Up, 30), vol(Direction::Up, 30)];
    let ss = run(strict, 40, &fs);
    assert_eq!(ss.len(), 2);
}

#[test]
fn flow_never_attaches_to_extreme_or_squeeze() {
    let fs = [ob(Edge::Enter, 30), vol(Direction::Up, 30)];
    let ss = run(KindDir::default(), 40, &fs);
    assert_eq!(ss.len(), 2);
}

#[test]
fn neutral_flow_stays_neutral() {
    let fs = [brk(Direction::Up, 30), vol(Direction::Neutral, 30)];
    let ss = run(KindDir::default(), 40, &fs);
    assert_eq!(ss.len(), 2);
    assert_eq!(find(&ss, &fs[1].id).story.theme, "flow.neutral");
}

#[test]
fn exit_ends_own_state_even_after_gap() {
    let fs = [ob(Edge::Enter, 30), ob(Edge::Exit, 70)];
    let ss = run(KindDir::default(), 80, &fs);
    assert_eq!(ss.len(), 1);
    let h = ss[0].story.history.last().unwrap();
    assert_eq!(
        (h.stage, h.reason.as_str()),
        (Stage::Weakened, "state_ended")
    );
}

#[test]
fn exit_without_story_is_ignored() {
    let r = KindDir::default();
    assert_eq!(r.decide(&sq(Edge::Exit, 30), &[]), ThemeDecision::Ignore);
    let ss = run(r, 40, &[sq(Edge::Exit, 30)]);
    assert!(ss.is_empty());
}

#[test]
fn re_enter_joins_within_gap_new_after() {
    let fs = [
        ob(Edge::Enter, 30),
        ob(Edge::Exit, 32),
        fact(
            "extreme.rsi",
            "overbought",
            MeaningKind::Extreme,
            Direction::Up,
            EvidenceFamily::Price,
            Edge::Enter,
            36,
        ),
        fact(
            "extreme.rsi",
            "overbought_end",
            MeaningKind::Extreme,
            Direction::Up,
            EvidenceFamily::Price,
            Edge::Exit,
            37,
        ),
        fact(
            "extreme.rsi",
            "overbought",
            MeaningKind::Extreme,
            Direction::Up,
            EvidenceFamily::Price,
            Edge::Enter,
            60,
        ),
    ];
    let ss = run(KindDir::default(), 70, &fs);
    assert_eq!(ss.len(), 2);
    assert_eq!(find(&ss, &fs[0].id).facts.len(), 4);
    assert_eq!(find(&ss, &fs[4].id).facts.len(), 1);
}

#[test]
fn no_join_is_ever_dropped_by_the_engine() {
    // Every joined exit lands in a story holding its enter: all facts end up in stories.
    let fs = [
        sq(Edge::Enter, 30),
        brk(Direction::Up, 31),
        vol(Direction::Up, 31),
        sq(Edge::Exit, 33),
        ob(Edge::Enter, 34),
        ob(Edge::Exit, 36),
        brk(Direction::Down, 38),
        vol(Direction::Up, 38),
    ];
    let ss = run(KindDir::default(), 50, &fs);
    let n: usize = ss.iter().map(|s| s.facts.len()).sum();
    assert_eq!(n, fs.len());
}

#[test]
fn deterministic() {
    let fs = [
        brk(Direction::Up, 30),
        vol(Direction::Up, 30),
        brk(Direction::Down, 31),
        vol(Direction::Down, 31),
    ];
    let a = run(KindDir::default(), 40, &fs);
    let b = run(KindDir::default(), 40, &fs);
    assert_eq!(a, b);
}
