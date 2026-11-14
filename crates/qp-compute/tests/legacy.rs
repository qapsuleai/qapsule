//! Comparison with the legacy Q-Charts batch indicators (oracle outputs recorded in
//! `fixtures/oracle/legacy_indicators.json` by `tools/legacy-oracle`).
//!
//! Legacy outputs are trimmed (no warmup values) and aligned to the last bar; ours have
//! one value per bar with leading `None` (K13). Tolerance: 1e-9 relative (absolute below 1).

mod common;

use std::collections::BTreeMap;
use std::path::PathBuf;

use common::{batch, close};
use serde_json::Value;

const TOL: f64 = 1e-9;

fn oracle() -> Value {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/oracle/legacy_indicators.json");
    serde_json::from_str(&std::fs::read_to_string(&p).unwrap()).unwrap()
}

fn floats(v: &Value) -> Vec<f64> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect()
}

#[test]
fn matches_legacy_batch_outputs() {
    let doc = oracle();
    let inputs: BTreeMap<String, Vec<f64>> = doc["inputs"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, v)| (k.clone(), floats(v)))
        .collect();
    let mut compared = 0usize;
    let mut bit_equal = 0usize;
    let mut max_rel: BTreeMap<String, f64> = BTreeMap::new();
    for case in doc["cases"].as_array().unwrap() {
        let study = case["study"].as_str().unwrap();
        let x = &inputs[case["input"].as_str().unwrap()];
        let kv: Vec<(String, f64)> = case["params"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), v.as_f64().unwrap()))
            .collect();
        let kv_ref: Vec<(&str, f64)> = kv.iter().map(|(k, v)| (k.as_str(), *v)).collect();
        let ours = batch(study, &kv_ref, x);
        let names = common::study(study, &kv_ref).outputs();
        for (name, col) in names.iter().zip(&ours) {
            let theirs = floats(&case["outputs"][*name]);
            let warm = x.len() - theirs.len();
            let label = format!("{study}{kv:?}/{name} on {}", case["input"]);
            assert_eq!(col.len(), x.len(), "{label}: length");
            assert!(
                col[..warm].iter().all(Option::is_none),
                "{label}: warmup not null"
            );
            for (i, (o, t)) in col[warm..].iter().zip(&theirs).enumerate() {
                let o = o.unwrap_or_else(|| panic!("{label}: None after warmup at {}", warm + i));
                assert!(
                    close(o, *t, TOL),
                    "{label}[{}]: ours {o:e}, legacy {t:e}",
                    warm + i
                );
                let rel = (o - t).abs() / t.abs().max(1.0);
                let e = max_rel.entry((*study).to_owned()).or_insert(0.0);
                *e = e.max(rel);
                compared += 1;
                bit_equal += usize::from(o.to_bits() == t.to_bits());
            }
        }
    }
    println!(
        "compared {compared} values, {bit_equal} bit-equal; max rel diff per study: {max_rel:?}"
    );
    assert!(compared > 10_000);
}
