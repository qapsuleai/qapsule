#![allow(dead_code)]

use std::path::PathBuf;

/// `fixtures/schema/` at the repo root.
pub fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/schema")
}

/// Reads a fixture as text.
pub fn fixture_text(name: &str) -> String {
    let path = fixtures_dir().join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// Reads a fixture as a JSON value.
pub fn fixture_value(name: &str) -> serde_json::Value {
    serde_json::from_str(&fixture_text(name)).expect("fixture is valid JSON")
}

/// Generated schema for a root type.
pub fn schema(name: &str) -> serde_json::Value {
    qp_schema::json_schemas()
        .into_iter()
        .find(|(n, _)| *n == name)
        .unwrap_or_else(|| panic!("no root schema named {name}"))
        .1
}

/// Validates `instance` against the generated schema of root type `name`;
/// returns the error messages.
pub fn validation_errors(name: &str, instance: &serde_json::Value) -> Vec<String> {
    let schema = schema(name);
    let validator = jsonschema::validator_for(&schema).expect("generated schema compiles");
    validator
        .iter_errors(instance)
        .map(|e| format!("{} at {}", e, e.instance_path()))
        .collect()
}
