//! R1 schema additions: nullable warmup points, `StudyOutput`, `Instrument`, `Calendar`,
//! `display_tz`, library validation and the bar series fixture format.

mod common;

use std::collections::BTreeMap;
use std::path::PathBuf;

use qp_schema::{
    parse_bar_series, validate, validate_bars, Bar, Calendar, HalfDay, Point, Series, Session,
    StudyOutput, Weekday,
};

fn bars_fixture_text() -> String {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/bars/sample.json");
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
}

#[test]
fn bar_fixture_is_a_valid_series() {
    let text = bars_fixture_text();
    let value: serde_json::Value = serde_json::from_str(&text).unwrap();
    validate("Series", &value).unwrap();
    let series = parse_bar_series(&text).unwrap();
    assert_eq!(series.symbol(), Some("SAMPLE"));
    assert_eq!(series.interval.as_deref(), Some("1d"));
    assert_eq!(series.bars().unwrap().len(), 200);
}

#[test]
fn bar_fixture_with_wrong_shape_is_rejected() {
    let mut value: serde_json::Value = serde_json::from_str(&bars_fixture_text()).unwrap();
    value["data"]["bars"][0]["c"] = serde_json::json!("98.93");
    assert!(validate("Series", &value).is_err());

    // The R0 format (`symbol` + top-level `bars`) is no longer accepted.
    let old = serde_json::json!({"symbol": "X", "interval": "1d", "bars": []});
    assert!(validate("Series", &old).is_err());
}

#[test]
fn validate_rejects_unknown_type() {
    let err = validate("Nope", &serde_json::json!({})).unwrap_err();
    assert!(err[0].contains("unknown type"), "{err:?}");
}

#[test]
fn validate_bars_semantics() {
    let ok = Bar {
        t: 1,
        o: 1.0,
        h: 2.0,
        l: 0.5,
        c: 1.5,
        v: 10.0,
    };
    assert!(validate_bars(&[]).is_err());
    assert!(validate_bars(std::slice::from_ref(&ok)).is_ok());
    assert!(validate_bars(&[Bar {
        h: 1.2,
        ..ok.clone()
    }])
    .is_err());
    assert!(validate_bars(&[Bar {
        l: 1.2,
        ..ok.clone()
    }])
    .is_err());
    assert!(validate_bars(&[Bar {
        v: f64::NAN,
        ..ok.clone()
    }])
    .is_err());
    assert!(validate_bars(&[ok.clone(), ok.clone()]).is_err()); // t not increasing
    assert!(validate_bars(&[ok.clone(), Bar { t: 2, ..ok }]).is_ok());
}

#[test]
fn warmup_points_are_null() {
    let p = Point { t: 5, v: None };
    let json = serde_json::to_string(&p).unwrap();
    assert_eq!(json, r#"{"t":5,"v":null}"#);
    assert_eq!(serde_json::from_str::<Point>(&json).unwrap(), p);
    validate(
        "Series",
        &serde_json::json!({
            "id": "s", "source": {"kind": "inline"},
            "data": {"kind": "points", "points": [{"t": 1, "v": null}, {"t": 2, "v": 1.5}]}
        }),
    )
    .unwrap();
}

#[test]
fn study_output_roundtrips_and_validates() {
    let out = StudyOutput {
        study: "macd".into(),
        params: BTreeMap::from([("fast".into(), 12.0), ("slow".into(), 26.0)]),
        t: vec![1, 2, 3],
        outputs: BTreeMap::from([
            ("macd".into(), vec![None, Some(0.5), Some(-0.25)]),
            ("signal".into(), vec![None, None, Some(0.1)]),
        ]),
        warmup: BTreeMap::from([("macd".into(), 1), ("signal".into(), 2)]),
    };
    let value = serde_json::to_value(&out).unwrap();
    validate("StudyOutput", &value).unwrap();
    assert_eq!(serde_json::from_value::<StudyOutput>(value).unwrap(), out);
    validate(
        "StudyOutput",
        &common::fixture_value("study_output.example.json"),
    )
    .unwrap();
}

#[test]
fn calendar_roundtrips_and_validates() {
    let cal = Calendar {
        id: "XNYS".into(),
        tz: "America/New_York".into(),
        sessions: vec![Session {
            open: "09:30".into(),
            close: "16:00".into(),
        }],
        weekend: vec![Weekday::Sat, Weekday::Sun],
        holidays: vec!["2026-11-26".into()],
        half_days: vec![HalfDay {
            date: "2026-11-27".into(),
            close: "13:00".into(),
        }],
    };
    let value = serde_json::to_value(&cal).unwrap();
    validate("Calendar", &value).unwrap();
    assert_eq!(
        serde_json::from_value::<Calendar>(value.clone()).unwrap(),
        cal
    );

    let mut bad = value;
    bad["sessions"][0]["open"] = serde_json::json!("9:30");
    assert!(validate("Calendar", &bad).is_err());
}

#[test]
fn document_display_tz_and_instrument() {
    let doc = common::fixture_value("document.example.json");
    validate("Document", &doc).unwrap();
    let doc: qp_schema::Document = serde_json::from_value(doc).unwrap();
    assert_eq!(doc.display_tz.as_deref(), Some("Europe/Istanbul"));
    let s: &Series = &doc.series[0];
    let inst = s.instrument.as_ref().unwrap();
    assert_eq!(
        (
            inst.symbol.as_str(),
            inst.exchange.as_deref(),
            inst.calendar.as_deref()
        ),
        ("THYAO", Some("BIST"), Some("XIST"))
    );
}
