//! K42/A28 on a replay: item validity, detail glyph validity and the TR/EN sentence agree.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use qp_schema::protocol::ReplayInit;
use qp_session::{demo_init, ReplayHost};
use serde_json::{json, Value};

fn root() -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../..") }
fn q(h: &mut ReplayHost, c: &str, v: Value) -> Value {
    let r = h.handle_raw(c, &v.to_string());
    serde_json::to_value(&r.last().unwrap().msg).unwrap()
}

fn main() {
    let none = std::env::temp_dir().join("verify-e1-no-local");
    let mut h = ReplayHost::with_local_dir(demo_init(ReplayInit { instruments: Some(6), days: Some(90), history: Some(60), seed: Some(9), fine: Some("1h".into()), ..ReplayInit::default() }), &root(), &none).unwrap();
    q(&mut h, "a", json!({"type":"hello","protocol":"qp/0","principal":"arkin","surface":{"kind":"phone"}}));
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut bad = Vec::new();
    let mut checked_detail = 0;
    let closed = ["realized", "invalidated", "expired", "retracted"];
    for round in 0..40 {
        q(&mut h, "a", json!({"type":"replay","id":"s","control":{"kind":"step","n":9}}));
        let v = q(&mut h, "a", json!({"type":"query","id":"l","query":{"kind":"capsules"}}));
        let items = v["data"]["capsules"].as_array().cloned().unwrap_or_default();
        for it in &items {
            let val = it["validity"].as_str().unwrap_or("?").to_owned();
            let st = it["stage"].as_str().unwrap_or("?");
            let (tr, en) = (it["sentence"]["tr"].as_str().unwrap_or(""), it["sentence"]["en"].as_str().unwrap_or(""));
            *counts.entry(val.clone()).or_default() += 1;
            if val == "closed" && (tr.contains("Hâlâ geçerli.") || en.contains("still holds")) { *counts.entry(format!("closed({st})+sentence says still valid")).or_default() += 1; if counts.get("_ex").is_none() { counts.insert("_ex".into(), 1); println!("example closed+still: {} | {tr} | {en}", it["id"]); } }
            if val == "closed" { *counts.entry(format!("closed({st})")).or_default() += 1; }
            if val == "valid" && (tr.contains("bilinmiyor") || en.contains("unknown")) { *counts.entry("valid+sentence says unknown".into()).or_default() += 1; }
            let says_no_tr = tr.to_lowercase().contains("artık");
            let says_no_en = en.to_lowercase().contains("no longer");
            let ok = match val.as_str() {
                "no_longer" => says_no_tr && says_no_en && !closed.contains(&st),
                "valid" => !says_no_tr && !says_no_en && !closed.contains(&st),
                "closed" => closed.contains(&st),
                _ => false,
            };
            if !ok { bad.push(format!("round {round} {} {val} {st}: {tr} || {en}", it["id"])); }
            if val == "no_longer" || (round % 10 == 0 && checked_detail < 400) {
                let d = q(&mut h, "a", json!({"type":"query","id":"d","query":{"kind":"capsule","capsule":it["id"]}}));
                checked_detail += 1;
                let det = &d["data"]["detail"];
                let gv = det["glyph"]["validity"].as_str().unwrap_or("none");
                let iv = det["item"]["validity"].as_str().unwrap_or("none");
                if gv != val || iv != val { bad.push(format!("round {round} {}: list {val} detail item {iv} glyph {gv}", it["id"])); }
            }
        }
    }
    println!("validity counts over 40 snapshots: {counts:?}; details checked {checked_detail}; disagreements {}", bad.len());
    for b in bad.iter().take(8) { println!("  {b}"); }
}
