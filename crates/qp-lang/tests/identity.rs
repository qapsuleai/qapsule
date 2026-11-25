//! `program_id` and `instance_id` (PR §2, §2.1): stable across the identity rules,
//! anchored to an externally computed SHA-256 of the canonical form.
#![allow(clippy::panic)]

mod support;

use qp_lang::{instance_id, program_id, sha256_hex};
use qp_schema::program::Program;
use serde_json::json;
use support::*;

/// The `sma` program of PR §14.
fn sma14() -> serde_json::Value {
    json!({"version": "qp-ir/0",
           "params": [{"key": "period", "kind": "int", "min": 1, "max": 2000, "default": 20}],
           "nodes": [{"id": "_1", "op": "input", "field": "close"},
                     {"id": "_2", "op": "sma", "x": "_1", "period": {"param": "period"}}],
           "outputs": [{"name": "sma", "kind": "series", "node": "_2"}]})
}

fn pid(v: &serde_json::Value) -> String {
    chk(v).program_id.unwrap_or_default()
}

#[test]
fn anchored_to_the_canonical_form() {
    // `printf '%s' '<canonical JSON>' | shasum -a 256` (computed outside the crate).
    let want = "sha256:b1fa385bb57e79fa5906442c0cbeade38f3bb658078072ebeaf8a508ebae8ee3";
    let r = chk(&sma14());
    assert!(r.ok(), "{:#?}", r.errors);
    assert_eq!(r.program_id.as_deref(), Some(want));
    // instance_id = SHA-256("<program_id>\n{\"period\":20}").
    let want_i = "sha256:0af8abb92513c6a92678e4422be5855e83ce9ba8502329b5903e0b4893be0944";
    assert_eq!(r.instance_id.as_deref(), Some(want_i));
    let c = r
        .checked
        .as_ref()
        .map(|c| (c.program_id.as_str(), c.instance_id.as_str()));
    assert_eq!(c, Some((want, want_i)));
}

#[test]
fn library_functions_agree_with_the_check() {
    let p: Program = serde_json::from_value(sma14()).unwrap_or_else(|e| panic!("{e}"));
    let id = program_id(&p).unwrap_or_default();
    assert_eq!(Some(id.clone()), chk(&sma14()).program_id);
    let ident = p.identity_json().unwrap_or_default();
    assert_eq!(id, format!("sha256:{}", sha256_hex(ident.as_bytes())));
    let values = [(
        "period".to_owned(),
        qp_schema::program::ProgramParamValue::Int(20),
    )]
    .into();
    assert_eq!(
        Some(instance_id(&id, &values).unwrap_or_default()),
        chk(&sma14()).instance_id
    );
}

#[test]
fn absent_equals_empty_and_order_does_not_matter() {
    let a = close_series();
    let mut b = close_series();
    b["params"] = json!([]);
    assert_eq!(pid(&a), pid(&b));
    // Field order in the incoming JSON does not matter.
    let c: serde_json::Value = serde_json::from_str(
        r#"{"outputs":[{"node":"c","kind":"series","name":"c"}],"nodes":[{"field":"close","op":"input","id":"c"}],"version":"qp-ir/0"}"#,
    )
    .unwrap_or_default();
    assert_eq!(pid(&a), pid(&c));
}

#[test]
fn descriptions_do_not_enter_the_identity() {
    let mut p = sma14();
    let base = pid(&p);
    p["params"][0]["unit"] = json!("bars");
    p["params"][0]["description"] = json!("window");
    assert_eq!(pid(&p), base);
    p["params"][0]["max"] = json!(1999);
    assert_ne!(pid(&p), base);
}

#[test]
fn float_parameters_are_normalized() {
    let with = |default: serde_json::Value| {
        prog_p(
            json!([{"key": "k", "kind": "float", "min": 1, "max": 3, "default": default}]),
            json!([{"id": "c", "op": "input", "field": "close"},
                   {"id": "b", "op": "bollinger", "x": "c", "period": 20, "k": {"param": "k"}}]),
            json!([{"name": "m", "kind": "series", "node": "b"}]),
        )
    };
    assert_eq!(pid(&with(json!(2))), pid(&with(json!(2.0))));
    let r = chk(&with(json!(2)));
    assert_eq!(
        r.params[0].default,
        qp_schema::program::ProgramParamValue::Float(2.0)
    );
    // A float literal written as an integer is the same program.
    let lit = |k: serde_json::Value| {
        prog(
            json!([{"id": "c", "op": "input", "field": "close"},
                   {"id": "b", "op": "bollinger", "x": "c", "period": 20, "k": k}]),
            json!([{"name": "m", "kind": "series", "node": "b"}]),
        )
    };
    assert_eq!(pid(&lit(json!(2))), pid(&lit(json!(2.0))));
}

#[test]
fn binding_is_not_in_the_identity_and_programs_differ() {
    assert_eq!(
        pid(&sma14()),
        chk_bound(&sma14(), "1h").program_id.unwrap_or_default()
    );
    let mut other = sma14();
    other["nodes"][1]["op"] = json!("ema");
    assert_ne!(pid(&sma14()), pid(&other));
    assert!(pid(&sma14()).starts_with("sha256:") && pid(&sma14()).len() == 7 + 64);
}

#[test]
fn identity_survives_errors_that_still_parse() {
    // A program with a range error still has an identity, but no instance.
    let r = chk(&sma(json!(5000)));
    assert!(!r.ok() && r.program_id.is_some() && r.instance_id.is_none());
}
