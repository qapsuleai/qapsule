//! Qapsule schema v0. Rust types are the single source of truth.
//!
//! JSON Schema is generated from these types with [`json_schemas`] /
//! [`export_schemas`]; it is never written by hand.

use std::path::{Path, PathBuf};

mod types;

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
        ("Command", schema_of::<Command>()),
        ("Intent", schema_of::<Intent>()),
        ("Ledger", schema_of::<Ledger>()),
    ]
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
