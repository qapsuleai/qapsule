//! K42/A22 at protocol level on a replay: births obey the level, only negative
//! transitions of position-related stories bypass it.
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use qp_schema::protocol::ReplayInit;
use qp_session::{demo_init, ReplayHost};
use serde_json::{json, Value};

fn root() -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../..") }

fn main() {
    for (level, max_tier) in [("important", 1_u64), ("balanced", 2)] {
        let none = std::env::temp_dir().join("verify-e1-no-local");
        let mut h = ReplayHost::with_local_dir(demo_init(ReplayInit { instruments: Some(8), days: Some(120), history: Some(60), seed: Some(5), fine: Some("1h".into()), ..ReplayInit::default() }), &root(), &none).unwrap();
        let mut send = |h: &mut ReplayHost, c: &str, v: Value| h.handle_raw(c, &v.to_string());
        send(&mut h, "a", json!({"type":"hello","protocol":"qp/0","principal":"arkin","surface":{"kind":"phone"}}));
        send(&mut h, "g", json!({"type":"hello","protocol":"qp/0","principal":"guest","surface":{"kind":"phone"}}));
        let r = send(&mut h, "a", json!({"type":"command","id":"a1","ops":[{"op":"set_audience","audience":{"group":"future","sensitivity":level,"positions":{"SYN02":10,"SYN05":-3}}}]}));
        assert!(serde_json::to_string(&r).unwrap().contains("ack"), "{r:?}");
        send(&mut h, "a", json!({"type":"subscribe","id":"k","topic":{"kind":"capsules"}}));
        send(&mut h, "g", json!({"type":"subscribe","id":"k","topic":{"kind":"capsules","all":true}}));
        let r = h.handle_raw("a", &json!({"type":"replay","id":"s","control":{"kind":"step","n":100000}}).to_string());
        let mut all = Vec::new();
        let mut got = BTreeSet::new();
        for l in &r {
            let v = serde_json::to_value(&l.msg).unwrap();
            if v["type"] != "notify" || v["event"]["kind"] != "capsule" { continue; }
            let c = &v["event"]["capsule"];
            let key = (c["id"].as_str().unwrap().to_owned(), v["event"]["transition"]["bar"].as_u64().unwrap(), v["event"]["transition"]["stage"].as_str().unwrap().to_owned());
            if l.conn == "g" { all.push((key, c["tier"].as_u64().unwrap(), c["instrument"].as_str().unwrap().to_owned(), c["relevant"].clone())); }
            else if l.conn == "a" { got.insert(key); }
        }
        let neg = |s: &str| matches!(s, "weakened" | "invalidated" | "retracted");
        let pos = |i: &str| i == "SYN02" || i == "SYN05";
        let (mut wrong_in, mut missing, mut pos_births_hi, mut pos_neg_hi, mut delivered_pos_neg_hi) = (0, 0, 0, 0, 0);
        for (key, tier, inst, _rel) in &all {
            let expected = *tier <= max_tier || (pos(inst) && neg(&key.2));
            let delivered = got.contains(key);
            if expected && !delivered { missing += 1; if missing < 3 { eprintln!("missing {key:?} tier {tier} {inst}") } }
            if !expected && delivered { wrong_in += 1; if wrong_in < 3 { eprintln!("wrongly delivered {key:?} tier {tier} {inst}") } }
            if pos(inst) && key.2 == "born" && *tier > max_tier { pos_births_hi += 1 }
            if pos(inst) && neg(&key.2) && *tier > max_tier { pos_neg_hi += 1; if delivered { delivered_pos_neg_hi += 1 } }
        }
        let extra = got.iter().filter(|k| !all.iter().any(|a| &a.0 == *k)).count();
        println!("{level}: transitions(all) {} delivered to arkin {} | missing {missing} wrongly delivered {wrong_in} not-in-all {extra} | position births above level {pos_births_hi} (must not arrive) | position negative transitions above level {pos_neg_hi}, delivered {delivered_pos_neg_hi}", all.len(), got.len());
    }
}
