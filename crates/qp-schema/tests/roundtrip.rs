//! Fixtures -> types -> JSON -> types.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::cognitive_complexity
)]

mod support;

use qp_schema::{
    Author, Capsule, CapsuleState, Command, Document, Instrument, Op, Provenance, Series,
    SeriesSource, SCHEMA_VERSION,
};
use serde::{de::DeserializeOwned, Serialize};

fn roundtrip<T>(name: &str) -> T
where
    T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug,
{
    let original = support::fixture_value(name);
    let typed: T = serde_json::from_value(original.clone()).expect("fixture deserializes");
    let json = serde_json::to_string(&typed).expect("serializes");
    let back: T = serde_json::from_str(&json).expect("deserializes again");
    assert_eq!(typed, back, "typed round-trip differs for {name}");
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(original, value, "value-level round-trip differs for {name}");
    typed
}

#[test]
fn capsule_fixture_roundtrips() {
    let capsule: Capsule = roundtrip("capsule.example.json");
    assert_eq!(capsule.state, CapsuleState::Live);
}

#[test]
fn document_fixture_roundtrips() {
    let doc: Document = roundtrip("document.example.json");
    assert_eq!(doc.schema_version, SCHEMA_VERSION);
    assert!(!doc.series.is_empty());
    assert!(!doc.views.is_empty());
    assert!(!doc.findings.is_empty());
    assert!(!doc.capsules.is_empty());
}

#[test]
fn capsule_fixture_matches_document_capsule() {
    let capsule: Capsule =
        serde_json::from_value(support::fixture_value("capsule.example.json")).unwrap();
    let doc: Document =
        serde_json::from_value(support::fixture_value("document.example.json")).unwrap();
    assert_eq!(doc.capsules[0], capsule);
    assert!(doc.findings.iter().any(|f| f.id == capsule.finding));
}

#[test]
fn every_capsule_state_roundtrips() {
    for (state, text) in [
        (CapsuleState::Draft, "\"draft\""),
        (CapsuleState::Live, "\"live\""),
        (CapsuleState::Invalidated, "\"invalidated\""),
        (CapsuleState::Expired, "\"expired\""),
        (CapsuleState::Approved, "\"approved\""),
        (CapsuleState::Rejected, "\"rejected\""),
    ] {
        assert_eq!(serde_json::to_string(&state).unwrap(), text);
        assert_eq!(serde_json::from_str::<CapsuleState>(text).unwrap(), state);
    }
}

#[test]
fn command_roundtrips() {
    let cmd = Command {
        provenance: Provenance {
            author: Author::Human {
                name: "analyst".into(),
            },
            at: 1_794_646_800_000,
            based_on: vec![],
            approved_by: None,
        },
        op: Op::PutSeries {
            series: Series {
                id: "garan-5m".into(),
                instrument: Some(Instrument {
                    symbol: "GARAN".into(),
                    exchange: Some("BIST".into()),
                    calendar: Some("XIST".into()),
                }),
                interval: Some("5m".into()),
                source: SeriesSource::Matriks {
                    symbol: "GARAN".into(),
                    interval: "5m".into(),
                },
                data: None,
            },
        },
    };
    let json = serde_json::to_string(&cmd).unwrap();
    assert_eq!(serde_json::from_str::<Command>(&json).unwrap(), cmd);
}

#[test]
fn unknown_fields_are_rejected() {
    let mut v = support::fixture_value("capsule.example.json");
    v["surprise"] = serde_json::json!(1);
    assert!(serde_json::from_value::<Capsule>(v).is_err());

    let src = serde_json::json!({"kind": "file", "path": "a.json", "extra": true});
    assert!(serde_json::from_value::<SeriesSource>(src).is_err());
}
