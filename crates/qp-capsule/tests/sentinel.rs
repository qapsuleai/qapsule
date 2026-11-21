//! (R5B/M31) The sentinel's notification filter (`qp_capsule::sentinel`).

use std::collections::BTreeSet;

use qp_capsule::sentinel::{
    apply, Filter, SentinelAudience, SentinelEvent, Transition, WatchMatch,
};
use qp_schema::capsule::{Direction, MeaningKind, Sensitivity, Stage};

const STAGES: [Stage; 8] = [
    Stage::Born,
    Stage::Strengthened,
    Stage::Weakened,
    Stage::Confirmed,
    Stage::Realized,
    Stage::Invalidated,
    Stage::Expired,
    Stage::Retracted,
];

fn set(xs: &[&str]) -> BTreeSet<String> {
    xs.iter().map(|s| (*s).to_owned()).collect()
}

fn audience(sens: Sensitivity) -> SentinelAudience {
    SentinelAudience {
        positions: set(&["HELD"]),
        watchlist: set(&["HELD", "WL"]),
        intents_on: set(&["W"]),
        sensitivity: sens,
    }
}

fn tr(day: u32, inst: &str, n: u32, stage: Stage, tier: u8) -> Transition {
    Transition {
        day,
        capsule: format!("cap.{inst}.breakout.up.{n}"),
        instrument: inst.to_owned(),
        kind: MeaningKind::Breakout,
        direction: Direction::Up,
        stage,
        tier,
        relevant: inst == "HELD" || inst == "W",
        coin_flip: false,
    }
}

fn keys(f: Filter, evs: &[SentinelEvent], a: &SentinelAudience) -> Vec<(u32, String, Stage)> {
    apply(f, evs, a)
        .into_iter()
        .map(|n| (n.day, n.capsule, n.stage))
        .collect()
}

/// Every combination of instrument, stage, tier, relevance, coin flip, sensitivity and a
/// preceding same-day notification of the same capsule: no filter drops a negative
/// transition on a held instrument (K42).
#[test]
fn no_filter_drops_a_negative_transition_on_a_position() {
    let mut checked = 0;
    for sens in [
        Sensitivity::Important,
        Sensitivity::Balanced,
        Sensitivity::All,
    ] {
        let a = audience(sens);
        for inst in ["HELD", "WL", "W", "OTHER"] {
            for stage in STAGES {
                for tier in 1..=3u8 {
                    for relevant in [false, true] {
                        for coin_flip in [false, true] {
                            for before in [None, Some(Stage::Born), Some(Stage::Weakened)] {
                                let mut evs = Vec::new();
                                if let Some(b) = before {
                                    let mut t = tr(1, inst, 1, b, 1);
                                    t.relevant = relevant;
                                    evs.push(SentinelEvent::Transition(t));
                                }
                                let mut t = tr(1, inst, 1, stage, tier);
                                t.relevant = relevant;
                                t.coin_flip = coin_flip;
                                let must = stage.is_negative() && inst == "HELD";
                                evs.push(SentinelEvent::Transition(t.clone()));
                                for f in Filter::ALL {
                                    let got = keys(f, &evs, &a);
                                    if must {
                                        assert!(
                                            got.contains(&(1, t.capsule.clone(), stage)),
                                            "{} dropped {t:?} after {before:?}",
                                            f.name()
                                        );
                                        checked += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    assert!(checked > 500, "{checked}");
}

#[test]
fn every_sends_feed_transitions_and_matches() {
    let a = audience(Sensitivity::Balanced);
    let evs = vec![
        SentinelEvent::Transition(tr(1, "WL", 1, Stage::Born, 2)),
        SentinelEvent::Transition(tr(1, "WL", 2, Stage::Born, 3)), // tier 3: not in feed
        SentinelEvent::Transition(tr(1, "OTHER", 3, Stage::Born, 1)), // not watched
        SentinelEvent::Transition(tr(1, "WL", 1, Stage::Strengthened, 1)),
        SentinelEvent::Match(WatchMatch {
            day: 1,
            intent: "watch-1-0".into(),
            text: None,
            capsule: "cap.OTHER.breakout.up.3".into(),
            stage: Stage::Born,
        }),
    ];
    let got = keys(Filter::Every, &evs, &a);
    assert_eq!(
        got,
        vec![
            (1, "cap.WL.breakout.up.1".into(), Stage::Born),
            (1, "cap.OTHER.breakout.up.3".into(), Stage::Born),
            (1, "cap.WL.breakout.up.1".into(), Stage::Strengthened),
        ]
    );
}

#[test]
fn a_match_and_its_transition_are_one_notification() {
    let a = audience(Sensitivity::All);
    let t = tr(2, "HELD", 1, Stage::Weakened, 3);
    let evs = vec![
        SentinelEvent::Transition(t.clone()),
        SentinelEvent::Match(WatchMatch {
            day: 2,
            intent: "watch-1-0".into(),
            text: Some("olumsuz".into()),
            capsule: t.capsule.clone(),
            stage: Stage::Weakened,
        }),
    ];
    for f in Filter::ALL {
        let n = apply(f, &evs, &a);
        assert_eq!(n.len(), 1, "{}", f.name());
        assert!(
            n[0].text
                .starts_with("HELD yukarı kırılım zayıfladı (pozisyon; izleme: olumsuz"),
            "{}",
            n[0].text
        );
    }
}

#[test]
fn tier_sens_sends_births_by_tier_and_involved_negatives_only() {
    let a = audience(Sensitivity::Balanced);
    let mut cf = tr(1, "WL", 4, Stage::Born, 1);
    cf.coin_flip = true;
    let evs = vec![
        SentinelEvent::Transition(tr(1, "WL", 1, Stage::Born, 2)),
        SentinelEvent::Transition(tr(1, "WL", 2, Stage::Born, 3)),
        SentinelEvent::Transition(tr(1, "WL", 1, Stage::Confirmed, 1)),
        SentinelEvent::Transition(tr(1, "WL", 3, Stage::Weakened, 1)),
        SentinelEvent::Transition(tr(1, "W", 5, Stage::Invalidated, 3)),
        SentinelEvent::Transition(cf),
    ];
    assert_eq!(
        keys(Filter::TierSens, &evs, &a),
        vec![
            (1, "cap.WL.breakout.up.1".into(), Stage::Born),
            (1, "cap.W.breakout.up.5".into(), Stage::Invalidated),
        ]
    );
}

#[test]
fn tier_repeat_hold_suppresses_repeats_and_prefers_holdings() {
    let a = audience(Sensitivity::Balanced);
    let evs = vec![
        // Held: born + confirmed the same day → one; then weakened (escalation) → sent.
        SentinelEvent::Transition(tr(1, "HELD", 1, Stage::Born, 1)),
        SentinelEvent::Transition(tr(1, "HELD", 1, Stage::Confirmed, 1)),
        SentinelEvent::Transition(tr(1, "HELD", 1, Stage::Weakened, 1)),
        // Watch list, tier 2 birth: not held → dropped; tier 1 birth → kept.
        SentinelEvent::Transition(tr(1, "WL", 2, Stage::Born, 2)),
        SentinelEvent::Transition(tr(1, "WL", 3, Stage::Born, 1)),
        // Watched instrument, negative, no match: not held → dropped.
        SentinelEvent::Transition(tr(1, "W", 4, Stage::Weakened, 3)),
        // Held tier 2 birth on the next day → kept.
        SentinelEvent::Transition(tr(2, "HELD", 5, Stage::Born, 2)),
    ];
    assert_eq!(
        keys(Filter::TierRepeatHold, &evs, &a),
        vec![
            (1, "cap.HELD.breakout.up.1".into(), Stage::Born),
            (1, "cap.HELD.breakout.up.1".into(), Stage::Weakened),
            (1, "cap.WL.breakout.up.3".into(), Stage::Born),
            (2, "cap.HELD.breakout.up.5".into(), Stage::Born),
        ]
    );
    assert_eq!(keys(Filter::TierSens, &evs, &a).len(), 6);
}

#[test]
fn names_round_trip_and_output_is_deterministic() {
    for f in Filter::ALL {
        assert_eq!(Filter::by_name(f.name()), Some(f));
    }
    assert_eq!(Filter::by_name("nope"), None);
    let a = audience(Sensitivity::Balanced);
    let evs: Vec<SentinelEvent> = STAGES
        .iter()
        .enumerate()
        .map(|(i, s)| SentinelEvent::Transition(tr(1, "HELD", 1, *s, (i % 3 + 1) as u8)))
        .collect();
    for f in Filter::ALL {
        assert_eq!(apply(f, &evs, &a), apply(f, &evs, &a));
    }
}
