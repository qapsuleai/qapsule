use std::collections::BTreeSet;

use super::system::*;
use super::*;

#[test]
fn params_resolve_order_object_user_plan_system() {
    let def = ParamDef {
        scopes: &[
            ParamScope::System,
            ParamScope::Plan,
            ParamScope::User,
            ParamScope::Object,
        ],
        ..SCORECARD_MIN_N
    };
    let all = [
        (ParamScope::System, ParamValue::Int(1)),
        (ParamScope::Plan, ParamValue::Int(2)),
        (ParamScope::User, ParamValue::Int(3)),
        (ParamScope::Object, ParamValue::Int(4)),
    ];
    // Each prefix of the order wins over everything after it.
    let expect = [
        (ParamScope::Object, 4),
        (ParamScope::User, 3),
        (ParamScope::Plan, 2),
        (ParamScope::System, 1),
    ];
    for (drop, (source, v)) in expect.into_iter().enumerate() {
        let values: Vec<_> = all
            .iter()
            .copied()
            .filter(|(s, _)| !RESOLUTION_ORDER[..drop].contains(s))
            .rev()
            .collect();
        let r = resolve(&def, &values);
        assert_eq!(r.source, source);
        assert_eq!(r.value, ParamValue::Int(v));
    }
}

#[test]
fn params_resolve_falls_back_to_system_default() {
    let r = resolve(&SCORECARD_MIN_N, &[]);
    assert_eq!(r.source, ParamScope::System);
    assert_eq!(r.value, ParamValue::Int(20));
    // Undeclared scope, instrument scope and wrong kind are all ignored.
    let ignored = [
        (ParamScope::User, ParamValue::Int(5)),
        (ParamScope::Instrument, ParamValue::Int(6)),
        (ParamScope::System, ParamValue::Float(7.0)),
    ];
    assert_eq!(
        resolve(&SCORECARD_MIN_N, &ignored).value,
        ParamValue::Int(20)
    );
    // A declared plan value wins over the system default.
    let plan = [(ParamScope::Plan, ParamValue::Int(64))];
    let r = resolve(&WATCH_MAX_PER_PERSON, &plan);
    assert_eq!((r.source, r.value), (ParamScope::Plan, ParamValue::Int(64)));
}

#[test]
fn params_defaults_equal_named_constants() {
    let int = |v: i64| ParamValue::Int(v);
    let float = |def: &ParamDef, v: f64| match def.default {
        ParamValue::Float(d) => d.to_bits() == v.to_bits(),
        _ => false,
    };
    let ints = [
        (&SCORECARD_MIN_N, int(SCORECARD_MIN_N_DEFAULT.into())),
        (&GLYPH_MIN_LEDGER_N, int(GLYPH_MIN_LEDGER_N_DEFAULT.into())),
        (
            &WATCH_MAX_PER_PERSON,
            int(WATCH_MAX_PER_PERSON_DEFAULT.into()),
        ),
    ];
    for (def, v) in ints {
        assert_eq!(def.default, v, "{}", def.key);
    }
    let u64s = [
        (&STORY_THEME_GAP_BARS, STORY_THEME_GAP_BARS_DEFAULT),
        (
            &STORY_THEME_FLOW_CONFIRM_BARS,
            STORY_THEME_FLOW_CONFIRM_BARS_DEFAULT,
        ),
        (
            &GESTURE_STORY_ZOOM_MIN_BARS,
            GESTURE_STORY_ZOOM_MIN_BARS_DEFAULT,
        ),
        (
            &DEPTH_CHART_LOOKBACK_BARS,
            DEPTH_CHART_LOOKBACK_BARS_DEFAULT,
        ),
    ];
    for (def, v) in u64s {
        assert_eq!(
            def.default,
            int(i64::try_from(v).unwrap_or(-1)),
            "{}",
            def.key
        );
    }
    let usizes = [
        (&OUTCOME_ATR_FAST, OUTCOME_ATR_FAST_DEFAULT),
        (&OUTCOME_ATR_SLOW, OUTCOME_ATR_SLOW_DEFAULT),
        (
            &DISPUTE_MAX_OPEN_PER_CONN,
            DISPUTE_MAX_OPEN_PER_CONN_DEFAULT,
        ),
        (&DISPUTE_MAX_PER_CONN, DISPUTE_MAX_PER_CONN_DEFAULT),
        (&DISPUTE_MAX_DIGESTS, DISPUTE_MAX_DIGESTS_DEFAULT),
    ];
    for (def, v) in usizes {
        assert_eq!(
            def.default,
            int(i64::try_from(v).unwrap_or(-1)),
            "{}",
            def.key
        );
    }
    assert_eq!(
        OUTCOME_INTRADAY_MAX_STEP_MS.default,
        int(OUTCOME_INTRADAY_MAX_STEP_MS_DEFAULT)
    );
    assert!(float(
        &GLYPH_FRESHNESS_HALF_BARS,
        GLYPH_FRESHNESS_HALF_BARS_DEFAULT
    ));
    assert!(float(&OUTCOME_BARRIER_C_DIR, OUTCOME_BARRIER_C_DIR_DEFAULT));
    assert!(float(
        &OUTCOME_BARRIER_C_NEUTRAL,
        OUTCOME_BARRIER_C_NEUTRAL_DEFAULT
    ));
    // Every registry row is covered above.
    assert_eq!(REGISTRY.len(), 16);
}

fn is_key(key: &str) -> bool {
    let segs: Vec<&str> = key.split('.').collect();
    segs.len() >= 2
        && segs.iter().all(|s| {
            s.starts_with(|c: char| c.is_ascii_lowercase())
                && s.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        })
}

#[test]
fn params_registry_is_well_formed() {
    let mut keys = BTreeSet::new();
    for def in REGISTRY {
        assert!(keys.insert(def.key), "duplicate key {}", def.key);
        assert!(is_key(def.key), "bad key {}", def.key);
        assert_eq!(def.default.kind(), def.kind, "{}: kind", def.key);
        assert!(def.allows(ParamScope::System), "{}: system scope", def.key);
        assert!(!def.unit.is_empty() && !def.description.is_empty());
        if let Some(v) = def.default.as_f64() {
            assert!(def.min.is_none_or(|m| v >= m), "{}: below min", def.key);
            assert!(def.max.is_none_or(|m| v <= m), "{}: above max", def.key);
        }
        assert_eq!(by_key(def.key), Some(*def));
    }
    assert_eq!(by_key("no.such_key"), None);
    assert!(keys.len() >= 10);
}
