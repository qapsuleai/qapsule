//! JSON Schema generation and export of the root types (the schema is generated from
//! the Rust types, never written by hand).

use std::path::{Path, PathBuf};

use crate::{
    capsule, comment, data, market, program, protocol, scene, Calendar, Capsule, Command, Document,
    Fact, Finding, Instrument, Intent, Ledger, Series, StudyOutput, View,
};

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
        ("Program", schema_of::<program::Program>()),
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
