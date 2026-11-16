//! Replay control edge cases (verify-e1).
use std::path::{Path, PathBuf};
use qp_schema::protocol::ReplayInit;
use qp_session::{demo_init, ReplayHost};
use serde_json::json;
fn root() -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../..") }
fn main() {
    let none = std::env::temp_dir().join("verify-e1-no-local");
    let mut h = ReplayHost::with_local_dir(demo_init(ReplayInit { instruments: Some(3), days: Some(40), history: Some(20), fine: Some("1h".into()), ..ReplayInit::default() }), &root(), &none).unwrap();
    let mut c = |v: serde_json::Value| { let r = h.handle_raw("a", &json!({"type":"replay","id":"x","control":v}).to_string()); serde_json::to_string(&r.first().map(|l| &l.msg)).unwrap().chars().take(330).collect::<String>() };
    let _ = h_hello();
    fn h_hello() {}
    println!("before hello: {}", c(json!({"kind":"status"})));
    let mut h2 = ReplayHost::with_local_dir(demo_init(ReplayInit { instruments: Some(3), days: Some(40), history: Some(20), fine: Some("1h".into()), ..ReplayInit::default() }), &root(), &none).unwrap();
    let _ = h2.handle_raw("a", &json!({"type":"hello","protocol":"qp/0","principal":"arkin","surface":{"kind":"phone"}}).to_string());
    let mut c2 = |v: serde_json::Value| { let _ = h2.handle_raw("a", &json!({"type":"hello","protocol":"qp/0","principal":"arkin","surface":{"kind":"phone"}}).to_string()); let r = h2.handle_raw("a", &json!({"type":"replay","id":"x","control":v}).to_string()); let m = serde_json::to_value(&r.first().unwrap().msg).unwrap(); if m["type"] == "error" { format!("ERROR {} {} {}", m["error"]["code"], m["error"]["path"], m["error"]["message"]) } else { let s = &m["data"]["replay"]; format!("step {} / {} t {} done {} reset {} epoch {} (+{} lines)", s["step"], s["steps"], s["t"], s["done"], s["reset"], s["epoch"], r.len() - 1) } };
    for v in [json!({"kind":"seek","date":"2023-13-40"}), json!({"kind":"seek","date":"2031-01-01"}), json!({"kind":"seek","date":"2001-01-01"}),
              json!({"kind":"seek","t":-5}), json!({"kind":"seek","step":999999}), json!({"kind":"step","n":0}), json!({"kind":"speed","steps_per_s":0.0}),
              json!({"kind":"speed","steps_per_s":1e9}), json!({"kind":"seek","step":3}), json!({"kind":"seek","step":3}), json!({"kind":"seek"}), json!({"kind":"step","n":5,"extra":1})] {
        println!("{v} -> {}", c2(v.clone()));
    }
}
