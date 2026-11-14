//! Unknown fields must be rejected by serde too (the protocol authority parses with
//! serde, not with JSON Schema), including on unit-like enum variants.

#[test]
fn inline_source_rejects_unknown_fields() {
    let r: Result<qp_schema::SeriesSource, _> =
        serde_json::from_str(r#"{"kind":"inline","params":{"period":7}}"#);
    assert!(r.is_err(), "extra field on `inline` was accepted: {r:?}");
}

#[test]
fn describe_query_rejects_unknown_fields() {
    let r: Result<qp_schema::protocol::Query, _> =
        serde_json::from_str(r#"{"kind":"describe","verbose":true}"#);
    assert!(r.is_err(), "extra field on `describe` was accepted: {r:?}");
}
