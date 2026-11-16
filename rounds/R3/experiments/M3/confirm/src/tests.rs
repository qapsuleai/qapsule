use std::collections::BTreeMap;

use qp_capsule::{OpenStory, RankCtx, Ranker};
use qp_schema::capsule::{
    Direction, Edge, EvidenceFamily, FactMeaning, LedgerStat, License, LicenseClass, MeaningKind,
    Stage, Story, TimeKind, Trust,
};
use qp_schema::{Author, Fact, Provenance};

use super::{ranker, Config, ConfirmRanker};

fn fact(
    bar: u64,
    det: &str,
    fam: EvidenceFamily,
    dir: Direction,
    edge: Edge,
    strength: f64,
) -> Fact {
    Fact {
        id: format!("fx.T.{det}.x.{bar}"),
        kind: "x".to_owned(),
        series: "T.1day".to_owned(),
        bar,
        t: i64::try_from(bar).unwrap() * 86_400_000,
        text: String::new(),
        evidence: BTreeMap::new(),
        evidence_ref: None,
        meaning: Some(FactMeaning {
            kind: MeaningKind::Flow,
            direction: dir,
            family: fam,
            time: TimeKind::Event,
            edge,
            detector: det.to_owned(),
            detector_version: 1,
            instrument: "T".to_owned(),
            strength,
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
            at: 0,
            based_on: Vec::new(),
            approved_by: None,
        },
    }
}

/// A story from its facts (the first one gave birth); families = distinct families of
/// same-direction facts, strength = max same-direction strength, like the engine.
fn story(facts: Vec<Fact>) -> OpenStory {
    let lead = facts[0].meaning.clone().unwrap();
    let mut families = Vec::new();
    let mut strength: f64 = 0.0;
    for f in &facts {
        let m = f.meaning.as_ref().unwrap();
        if m.direction == lead.direction && m.edge != Edge::Exit {
            if !families.contains(&m.family) {
                families.push(m.family);
            }
            strength = strength.max(m.strength);
        }
    }
    families.sort();
    OpenStory {
        id: "cap.T.flow.up.0".to_owned(),
        story: Story {
            instrument: "T".to_owned(),
            theme: "flow.up".to_owned(),
            kind: lead.kind,
            direction: lead.direction,
            time: lead.time,
            stage: Stage::Born,
            families,
            strength,
            importance: 0.0,
            tier: 3,
            series: "T.1day".to_owned(),
            born_bar: 0,
            last_bar: 0,
            horizon_bars: 10,
            ledger_key: "flow/x@1".to_owned(),
            ledger_at_birth: LedgerStat::default(),
            trust: Trust::Replica,
            history: Vec::new(),
        },
        facts,
    }
}

/// A context where this kind is average (kind share 1/6 over 30 instruments, this
/// instrument at the average count) and the ledger is empty.
fn avg_ctx() -> RankCtx {
    RankCtx {
        ledger: LedgerStat::default(),
        ledger_all: LedgerStat {
            n: 1000,
            realized: 470,
        },
        same_kind_on_instrument: 10,
        bars_seen: 300,
        same_kind_all: 600,
        stories_all: 3600,
        instruments: 30,
    }
}

const UP: Direction = Direction::Up;
const DOWN: Direction = Direction::Down;
const P: Edge = Edge::Point;
use EvidenceFamily::{Flow, Price, Volume};

#[test]
fn name_and_entry_point() {
    assert_eq!(ranker().name(), "confirm");
}

#[test]
fn two_families_are_tier_one_whatever_the_secondary() {
    let r = ConfirmRanker::default();
    let s = story(vec![
        fact(0, "flow.a", Flow, UP, P, 0.0),
        fact(0, "vol.a", Volume, UP, P, 0.0),
    ]);
    // Bad track record, common kind, crowded instrument, zero strength.
    let ctx = RankCtx {
        ledger: LedgerStat {
            n: 500,
            realized: 50,
        },
        same_kind_on_instrument: 400,
        same_kind_all: 3000,
        ..avg_ctx()
    };
    let (imp, tier) = r.rank(&s, &ctx);
    assert_eq!(tier, 1);
    assert!(imp >= 2.0);
}

#[test]
fn one_more_family_always_outranks_secondary_evidence() {
    let r = ConfirmRanker::default();
    let best_single = story(vec![fact(0, "flow.a", Flow, UP, P, 1.0)]);
    let great = RankCtx {
        ledger: LedgerStat {
            n: 500,
            realized: 500,
        },
        same_kind_on_instrument: 0,
        same_kind_all: 0,
        ..avg_ctx()
    };
    let worst_double = story(vec![
        fact(0, "flow.a", Flow, UP, P, 0.0),
        fact(0, "vol.a", Volume, UP, P, 0.0),
    ]);
    let awful = RankCtx {
        ledger: LedgerStat {
            n: 500,
            realized: 0,
        },
        same_kind_on_instrument: 1000,
        same_kind_all: 3599,
        ..avg_ctx()
    };
    let (a, _) = r.rank(&best_single, &great);
    let (b, _) = r.rank(&worst_double, &awful);
    assert!(a < 2.0 && b >= 2.0 && b > a, "{a} {b}");
}

#[test]
fn same_family_twice_is_one_confirmation() {
    let r = ConfirmRanker::default();
    let s = story(vec![
        fact(0, "price.a", Price, UP, P, 0.3),
        fact(1, "price.b", Price, UP, P, 0.3),
        fact(2, "price.c", Price, UP, P, 0.3),
    ]);
    let p = r.parts(&s, &avg_ctx());
    assert_eq!(p.support, 1);
    assert!((p.confirmation - 1.0).abs() < 1e-12);
    assert_ne!(r.rank(&s, &avg_ctx()).1, 1);
}

#[test]
fn a_story_rises_when_confirmed() {
    let r = ConfirmRanker::default();
    let ctx = avg_ctx();
    let born = story(vec![fact(0, "flow.a", Flow, UP, P, 0.2)]);
    let (i0, t0) = r.rank(&born, &ctx);
    let confirmed = story(vec![
        fact(0, "flow.a", Flow, UP, P, 0.2),
        fact(3, "vol.a", Volume, UP, P, 0.2),
    ]);
    let (i1, t1) = r.rank(&confirmed, &ctx);
    assert!(t0 > 1);
    assert_eq!(t1, 1);
    assert!(i1 > i0 + 0.99);
}

#[test]
fn contradiction_and_state_end_lower_the_rank() {
    let r = ConfirmRanker::default();
    let ctx = avg_ctx();
    let two = story(vec![
        fact(0, "flow.a", Flow, UP, P, 0.5),
        fact(0, "vol.a", Volume, UP, P, 0.5),
    ]);
    let contradicted = story(vec![
        fact(0, "flow.a", Flow, UP, P, 0.5),
        fact(0, "vol.a", Volume, UP, P, 0.5),
        fact(2, "price.a", Price, DOWN, P, 0.9),
    ]);
    let (i2, t2) = r.rank(&two, &ctx);
    let (ic, tc) = r.rank(&contradicted, &ctx);
    assert_eq!(t2, 1);
    assert!(ic < i2 && tc > t2, "{ic} {tc}");
    assert_eq!(r.parts(&contradicted, &ctx).oppose, 1);

    // A state that ended: same story with an exit of its own detector.
    let entered = story(vec![fact(0, "x.state", Price, UP, Edge::Enter, 0.5)]);
    let ended = story(vec![
        fact(0, "x.state", Price, UP, Edge::Enter, 0.5),
        fact(4, "x.state", Price, UP, Edge::Exit, 0.0),
    ]);
    let pe = r.parts(&ended, &ctx);
    assert!(pe.ended && !r.parts(&entered, &ctx).ended);
    assert!(r.rank(&ended, &ctx).0 < r.rank(&entered, &ctx).0);
    assert!(r.rank(&ended, &ctx).1 >= 2);
    // An exit of a detector that never entered the story does not end it.
    let other_exit = story(vec![
        fact(0, "x.state", Price, UP, Edge::Enter, 0.5),
        fact(4, "y.state", Price, UP, Edge::Exit, 0.0),
    ]);
    assert!(!r.parts(&other_exit, &ctx).ended);
}

#[test]
fn cold_start_track_is_neutral_and_shrinks() {
    let r = ConfirmRanker::default();
    let empty = RankCtx::default();
    assert!((r.track(&empty) - 0.5).abs() < 1e-12);
    // Few closed capsules barely move the track; many do.
    let few = RankCtx {
        ledger: LedgerStat { n: 3, realized: 3 },
        ..avg_ctx()
    };
    let many = RankCtx {
        ledger: LedgerStat {
            n: 300,
            realized: 270,
        },
        ..avg_ctx()
    };
    let bad = RankCtx {
        ledger: LedgerStat {
            n: 300,
            realized: 30,
        },
        ..avg_ctx()
    };
    assert!(r.track(&few) > 0.5 && r.track(&few) < 0.65);
    assert!(r.track(&many) > 0.95);
    assert!(r.track(&bad) < 0.05);
}

#[test]
fn rarity_prefers_rare_kinds_and_quiet_instruments() {
    let r = ConfirmRanker::default();
    let s = story(vec![fact(0, "flow.a", Flow, UP, P, 0.5)]);
    let common = avg_ctx();
    let rare_kind = RankCtx {
        same_kind_all: 30,
        same_kind_on_instrument: 0,
        ..avg_ctx()
    };
    let crowded = RankCtx {
        same_kind_on_instrument: 40,
        ..avg_ctx()
    };
    assert!(r.rarity(&s, &rare_kind) > r.rarity(&s, &common));
    assert!(r.rarity(&s, &crowded) < r.rarity(&s, &common));
    for c in [common, rare_kind, crowded, RankCtx::default()] {
        let x = r.rarity(&s, &c);
        assert!((0.0..=1.0).contains(&x));
    }
}

#[test]
fn bounded_and_finite_on_degenerate_inputs() {
    let r = ConfirmRanker::default();
    let s = story(vec![fact(0, "flow.a", Flow, UP, P, f64::NAN)]);
    for ctx in [
        RankCtx::default(),
        RankCtx {
            ledger: LedgerStat {
                n: u32::MAX,
                realized: u32::MAX,
            },
            ledger_all: LedgerStat {
                n: u32::MAX,
                realized: 0,
            },
            same_kind_on_instrument: u32::MAX,
            bars_seen: u64::MAX,
            same_kind_all: u32::MAX,
            stories_all: u32::MAX,
            instruments: 0,
        },
    ] {
        let (imp, tier) = r.rank(&s, &ctx);
        assert!(imp.is_finite());
        assert!((1..=3).contains(&tier));
    }
}

#[test]
fn tier_is_monotone_in_importance_for_one_family() {
    // Within a confirmation level, a higher secondary score never gets a worse tier.
    let r = ConfirmRanker::default();
    let s = story(vec![fact(0, "flow.a", Flow, UP, P, 0.5)]);
    let mut last: Option<(f64, u8)> = None;
    let mut pts = Vec::new();
    for n in (0..=300).step_by(30) {
        for k in (0..=60).step_by(10) {
            let ctx = RankCtx {
                ledger: LedgerStat {
                    n: 300,
                    realized: n,
                },
                same_kind_on_instrument: k,
                ..avg_ctx()
            };
            pts.push(r.rank(&s, &ctx));
        }
    }
    pts.sort_by(|a, b| a.0.total_cmp(&b.0));
    for p in pts {
        if let Some(l) = last {
            assert!(p.1 <= l.1, "{l:?} then {p:?}");
        }
        last = Some(p);
    }
}

#[test]
fn deterministic_and_pure() {
    let r = ConfirmRanker::default();
    let s = story(vec![
        fact(0, "flow.a", Flow, UP, P, 0.4),
        fact(1, "vol.a", Volume, UP, P, 0.7),
    ]);
    let ctx = avg_ctx();
    let a = r.rank(&s, &ctx);
    for _ in 0..10 {
        assert_eq!(r.rank(&s, &ctx).0.to_bits(), a.0.to_bits());
        assert_eq!(r.rank(&s, &ctx).1, a.1);
    }
    assert_eq!(ConfirmRanker::default().cfg, Config::default());
}
