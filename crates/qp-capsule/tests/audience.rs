//! Sensitivity filter (§9.8).

mod common;

use std::collections::BTreeMap;

use common::*;
use qp_capsule::admits;
use qp_capsule::engine::Engine;
use qp_schema::capsule::{Audience, Sensitivity, Stage, Story, UserGroup};

fn story(inst: &str, tier: u8, stage: Stage) -> Story {
    let mut e = Engine::new(test_config(10));
    let b = bars(&[100.0]);
    let mut s = e.on_bar(inst, SERIES, 0, &b, &[up(inst, "d1", 0)])[0]
        .story
        .clone();
    s.tier = tier;
    s.stage = stage;
    s
}

fn aud(group: UserGroup, s: Option<Sensitivity>) -> Audience {
    Audience {
        group,
        sensitivity: s,
        watchlist: Vec::new(),
        positions: BTreeMap::new(),
    }
}

#[test]
fn tier_filter_per_level() {
    let cases = [
        (Sensitivity::Important, [true, false, false]),
        (Sensitivity::Balanced, [true, true, false]),
        (Sensitivity::All, [true, true, true]),
    ];
    for (lvl, want) in cases {
        let a = aud(UserGroup::Future, Some(lvl));
        for (i, w) in want.iter().enumerate() {
            let tier = u8::try_from(i + 1).unwrap();
            assert_eq!(
                admits(&a, &story("X", tier, Stage::Born), &[]),
                *w,
                "{lvl:?} {tier}"
            );
        }
    }
}

#[test]
fn group_default_applies_when_unset() {
    let s3 = story("X", 3, Stage::Born);
    assert!(!admits(&aud(UserGroup::Future, None), &s3, &[]));
    assert!(admits(&aud(UserGroup::Habituated, None), &s3, &[]));
}

#[test]
fn position_and_intent_override_level() {
    let mut a = aud(UserGroup::Future, Some(Sensitivity::Important));
    a.positions.insert("POS".to_owned(), -100.0);
    a.positions.insert("FLAT".to_owned(), 0.0);
    for st in [Stage::Weakened, Stage::Invalidated, Stage::Born] {
        assert!(admits(&a, &story("POS", 3, st), &[]));
        assert!(admits(&a, &story("INT", 3, st), &["INT".to_owned()]));
    }
    // Zero quantity is not a position; no intent → tier filter.
    assert!(!admits(&a, &story("FLAT", 3, Stage::Invalidated), &[]));
    assert!(!admits(
        &a,
        &story("OTHER", 2, Stage::Weakened),
        &["INT".to_owned()]
    ));
}

#[test]
fn admits_never_changes_the_audience() {
    let a = aud(UserGroup::Future, Some(Sensitivity::Balanced));
    let before = serde_json::to_string(&a).unwrap();
    let _ = admits(&a, &story("X", 3, Stage::Born), &["X".to_owned()]);
    assert_eq!(serde_json::to_string(&a).unwrap(), before);
}
