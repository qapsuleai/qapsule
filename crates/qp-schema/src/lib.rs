//! Qapsule schema v0. Rust types are the single source of truth.
//!
//! JSON Schema is generated from these types with [`json_schemas`] /
//! [`export_schemas`]; it is never written by hand.

use std::path::{Path, PathBuf};

pub mod capsule;
pub mod comment;
pub mod data;
pub mod grammar;
pub mod market;
pub mod protocol;
pub mod scene;
mod types;

pub use grammar::{Encoding, Layer, Mark, Panel, Scale, ScaleKind, View};
pub use types::*;

fn schema_of<T: schemars::JsonSchema>() -> serde_json::Value {
    schemars::schema_for!(T).to_value()
}

/// All root schemas, in stable order: (type name, JSON Schema as serde_json::Value).
pub fn json_schemas() -> Vec<(&'static str, serde_json::Value)> {
    vec![
        ("Document", schema_of::<Document>()),
        ("Capsule", schema_of::<Capsule>()),
        ("Series", schema_of::<Series>()),
        ("Fact", schema_of::<Fact>()),
        ("Finding", schema_of::<Finding>()),
        ("View", schema_of::<View>()),
        ("SceneDesc", schema_of::<scene::SceneDesc>()),
        ("Command", schema_of::<Command>()),
        ("Intent", schema_of::<Intent>()),
        ("Ledger", schema_of::<Ledger>()),
        ("StudyOutput", schema_of::<StudyOutput>()),
        ("Instrument", schema_of::<Instrument>()),
        ("Calendar", schema_of::<Calendar>()),
        ("DataBinding", schema_of::<data::DataBinding>()),
        ("DataEvent", schema_of::<data::DataEvent>()),
        ("EvidenceRef", schema_of::<data::EvidenceRef>()),
        ("Story", schema_of::<capsule::Story>()),
        ("GlyphSpec", schema_of::<capsule::GlyphSpec>()),
        ("ClientMsg", schema_of::<protocol::ClientMsg>()),
        ("ServerMsg", schema_of::<protocol::ServerMsg>()),
        ("Op", schema_of::<protocol::Op>()),
        ("ChangeOp", schema_of::<protocol::ChangeOp>()),
        ("LogEntry", schema_of::<protocol::LogEntry>()),
        ("Query", schema_of::<protocol::Query>()),
        ("InboundLine", schema_of::<protocol::InboundLine>()),
        ("OutboundLine", schema_of::<protocol::OutboundLine>()),
        ("SessionInit", schema_of::<protocol::SessionInit>()),
        ("Transcript", schema_of::<protocol::Transcript>()),
        ("MarketStatus", schema_of::<market::MarketStatus>()),
        ("Comment", schema_of::<protocol::Comment>()),
        ("CommentDraft", schema_of::<comment::CommentDraft>()),
    ]
}

/// The generated JSON Schema of root type `name`, if there is one.
#[must_use]
pub fn json_schema(name: &str) -> Option<serde_json::Value> {
    json_schemas()
        .into_iter()
        .find(|(n, _)| *n == name)
        .map(|(_, s)| s)
}

/// Names of all root types, in [`json_schemas`] order.
#[must_use]
pub fn root_type_names() -> Vec<&'static str> {
    json_schemas().into_iter().map(|(n, _)| n).collect()
}

/// Validates `instance` against an explicit JSON Schema value.
///
/// # Errors
/// One message per violation (`"<message> at <json pointer>"`), or a single message if
/// the schema itself does not compile.
#[cfg(feature = "validate")]
pub fn validate_with_schema(
    schema: &serde_json::Value,
    instance: &serde_json::Value,
) -> Result<(), Vec<String>> {
    let validator = jsonschema::validator_for(schema)
        .map_err(|e| vec![format!("schema does not compile: {e}")])?;
    let errors: Vec<String> = validator
        .iter_errors(instance)
        .map(|e| format!("{} at {}", e, e.instance_path()))
        .collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Validates `instance` against the generated schema of root type `type_name`.
///
/// # Errors
/// One message per violation, or a single message for an unknown type name.
#[cfg(feature = "validate")]
pub fn validate(type_name: &str, instance: &serde_json::Value) -> Result<(), Vec<String>> {
    let schema = json_schema(type_name).ok_or_else(|| {
        vec![format!(
            "unknown type `{type_name}`; expected one of: {}",
            root_type_names().join(", ")
        )]
    })?;
    validate_with_schema(&schema, instance)
}

/// Semantic checks on a bar list that JSON Schema cannot express: non-empty, all values
/// finite, `h >= max(o, c)`, `l <= min(o, c)`, `l <= h`, strictly increasing `t`.
///
/// # Errors
/// The first violation found, naming the bar index.
pub fn validate_bars(bars: &[Bar]) -> Result<(), String> {
    if bars.is_empty() {
        return Err("bar list is empty".to_owned());
    }
    let mut prev_t: Option<i64> = None;
    for (i, b) in bars.iter().enumerate() {
        if [b.o, b.h, b.l, b.c, b.v].iter().any(|v| !v.is_finite()) {
            return Err(format!("bar {i}: non-finite value"));
        }
        if b.h < b.o.max(b.c) || b.l > b.o.min(b.c) || b.l > b.h {
            return Err(format!("bar {i}: inconsistent OHLC"));
        }
        if prev_t.is_some_and(|p| b.t <= p) {
            return Err(format!("bar {i}: timestamp not increasing"));
        }
        prev_t = Some(b.t);
    }
    Ok(())
}

/// Parses a bar series document (a [`Series`] with embedded `bars` data, the
/// `fixtures/bars/*.json` format) and runs [`validate_bars`] on it.
///
/// # Errors
/// Malformed JSON, a series without embedded bars, or a [`validate_bars`] failure.
pub fn parse_bar_series(json: &str) -> Result<Series, String> {
    let series: Series =
        serde_json::from_str(json).map_err(|e| format!("invalid series JSON: {e}"))?;
    let bars = series
        .bars()
        .ok_or_else(|| "series has no embedded bars (`data.kind` must be `bars`)".to_owned())?;
    validate_bars(bars)?;
    Ok(series)
}

/// Writes `<Name>.schema.json` (pretty, trailing newline) for each root type into `dir` (creates dir).
pub fn export_schemas(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    std::fs::create_dir_all(dir)?;
    let mut written = Vec::new();
    for (name, schema) in json_schemas() {
        let path = dir.join(format!("{name}.schema.json"));
        let mut text = serde_json::to_string_pretty(&schema).map_err(std::io::Error::other)?;
        text.push('\n');
        std::fs::write(&path, text)?;
        written.push(path);
    }
    Ok(written)
}
