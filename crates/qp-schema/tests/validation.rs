//! Fixtures and serialized values validate against the generated JSON Schema.

mod common;

use qp_schema::{Author, CapsuleState, Command, Op, Provenance};

fn assert_valid(name: &str, instance: &serde_json::Value) {
    let errors = common::validation_errors(name, instance);
    assert!(errors.is_empty(), "{name} invalid: {errors:#?}");
}

#[test]
fn committed_capsule_fixture_is_valid() {
    assert_valid("Capsule", &common::fixture_value("capsule.example.json"));
}

#[test]
fn committed_document_fixture_is_valid() {
    assert_valid("Document", &common::fixture_value("document.example.json"));
}

#[test]
fn document_parts_are_valid_against_their_root_schemas() {
    let doc = common::fixture_value("document.example.json");
    for (root, key) in [
        ("Series", "series"),
        ("View", "views"),
        ("Fact", "facts"),
        ("Finding", "findings"),
        ("Capsule", "capsules"),
        ("Intent", "intents"),
    ] {
        for item in doc[key].as_array().expect("array") {
            assert_valid(root, item);
        }
    }
    assert_valid("Ledger", &doc["ledger"]);
}

#[test]
fn serialized_command_is_valid() {
    let cmd = Command {
        provenance: Provenance {
            author: Author::Agent {
                name: "scout".into(),
                model: None,
            },
            at: 1_794_646_800_000,
            based_on: vec!["fnd-thyao-rsi-overbought".into()],
        },
        op: Op::SetCapsuleState {
            capsule: "cap-thyao-rsi-overbought".into(),
            state: CapsuleState::Approved,
        },
    };
    assert_valid("Command", &serde_json::to_value(&cmd).unwrap());
}

#[test]
fn schema_rejects_bad_documents() {
    let mut doc = common::fixture_value("document.example.json");
    doc["capsules"][0]["state"] = serde_json::json!("archived");
    assert!(!common::validation_errors("Document", &doc).is_empty());

    let mut doc = common::fixture_value("document.example.json");
    doc["findings"][0]["confidence"] = serde_json::json!(1.5);
    assert!(!common::validation_errors("Document", &doc).is_empty());

    let mut doc = common::fixture_value("document.example.json");
    doc["unexpected"] = serde_json::json!(true);
    assert!(!common::validation_errors("Document", &doc).is_empty());

    let mut doc = common::fixture_value("document.example.json");
    doc["series"][0]["data"]["bars"][0]["t"] = serde_json::json!("2026-11-11");
    assert!(!common::validation_errors("Document", &doc).is_empty());
}
