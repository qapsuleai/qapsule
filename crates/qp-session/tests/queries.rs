//! Queries, presence, input and the presenter.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::cognitive_complexity
)]

mod support;

use serde_json::json;
use support::{code, H};

fn two() -> H {
    let mut h = H::new();
    h.hello("a", "arkin", "desktop");
    h.hello("s", "scout", "headless");
    h
}

#[test]
fn get_list_document_log() {
    let mut h = two();
    let r = h.query("s", json!({"kind":"get","id":"rsi41"}));
    assert_eq!(code(&r), "not_found");
    assert_eq!(r["error"]["did_you_mean"], json!(["rsi14"]));
    let r = h.query("s", json!({"kind":"list","collection":"views"}));
    assert_eq!(r["data"]["items"][0]["id"], "main");
    let r = h.query(
        "s",
        json!({"kind":"list","collection":"views","state":"pending"}),
    );
    assert_eq!(code(&r), "invalid");
    let r = h.query("s", json!({"kind":"document"}));
    assert_eq!(r["data"]["document"]["id"], "doc-thyao");
    for i in 0..3 {
        h.cmd(
            "a",
            &format!("c{i}"),
            json!([{"op":"add_intent","text":"x"}]),
        );
    }
    h.cmd("s", "p", json!([{"op":"add_intent","text":"y"}]));
    let r = h.query("s", json!({"kind":"log","limit":2}));
    let seqs: Vec<_> = r["data"]["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["seq"].clone())
        .collect();
    assert_eq!(seqs, vec![json!(3), json!(4)]);
    let r = h.query("s", json!({"kind":"log","author":"scout"}));
    assert_eq!(r["data"]["entries"].as_array().unwrap().len(), 1);
    assert_eq!(r["seq"], 4);
}

#[test]
fn study_values_and_reductions() {
    let mut h = two();
    let r = h.query(
        "s",
        json!({"kind":"study","series":"macd","range":{"last":3},"output":"signal"}),
    );
    let v = &r["data"]["values"];
    assert_eq!(v["from_bar"], 197);
    assert_eq!(v["t"].as_array().unwrap().len(), 3);
    assert_eq!(v["outputs"].as_object().unwrap().len(), 1);
    let r = h.query(
        "s",
        json!({"kind":"study","series":"rsi14","range":{"from_bar":0,"to_bar":20}}),
    );
    assert!(
        r["data"]["values"]["outputs"]["rsi"][0].is_null(),
        "warmup is null"
    );
    for how in ["min", "max", "first", "last"] {
        let r = h.query("s", json!({"kind":"study","series":"rsi14","reduce":how}));
        assert!(r["data"]["values"]["rsi"]["bar"].is_u64(), "{how}");
    }
    let r = h.query(
        "s",
        json!({"kind":"study","series":"rsi14","reduce":"count"}),
    );
    assert_eq!(r["data"]["values"]["rsi"]["value"], 186.0);
    let r = h.query("s", json!({"kind":"study","series":"px","range":{"from_bar":0,"to_bar":1},"reduce":"mean","output":"c"}));
    assert!(r["data"]["values"]["c"]["bar"].is_null());
    let t0 = h.s.document().series[0].bars().unwrap()[5].t;
    let r = h.query(
        "s",
        json!({"kind":"study","series":"px","range":{"from_t":t0,"to_t":t0},"output":"c"}),
    );
    assert_eq!(r["data"]["values"]["from_bar"], 5);
    let r = h.query(
        "s",
        json!({"kind":"study","series":"px","range":{"last":2,"from_bar":1}}),
    );
    assert_eq!(code(&r), "invalid");
    let r = h.query("s", json!({"kind":"study","series":"macd","output":"sig"}));
    assert_eq!(r["error"]["did_you_mean"], json!(["signal"]));
    let r = h.query("s", json!({"kind":"at","series":"px","t":t0}));
    assert_eq!(r["data"]["at"]["bar"], 5);
    let r = h.query("s", json!({"kind":"at","series":"rsi14","bar":1}));
    assert_eq!(code(&r), "invalid");
    let r = h.query("s", json!({"kind":"at","series":"px"}));
    assert_eq!(r["data"]["at"]["bar"], 199);
}

#[test]
fn study_cache_follows_parameter_changes() {
    let mut h = two();
    let before = h.query(
        "s",
        json!({"kind":"study","series":"rsi14","range":{"last":1}}),
    );
    h.cmd(
        "a",
        "c",
        json!([{"op":"set_params","series":"rsi14","params":{"period":5}}]),
    );
    let after = h.query(
        "s",
        json!({"kind":"study","series":"rsi14","range":{"last":1}}),
    );
    assert_ne!(before["data"], after["data"]);
    h.cmd("a", "u", json!([{"op":"undo"}]));
    let again = h.query(
        "s",
        json!({"kind":"study","series":"rsi14","range":{"last":1}}),
    );
    assert_eq!(before["data"], again["data"]);
}

#[test]
fn describe_lists_everything() {
    let mut h = two();
    let r = h.query("a", json!({"kind":"describe"}));
    let d = &r["data"]["description"];
    assert_eq!(d["ops"].as_array().unwrap().len(), 38); // 22 in qp/0 (RD1) + 10 grammar ops (R2) + split_layer (R2/T8a) + set_audience, set_sensitivity (R3) + add_comment, add_watch, remove_watch (R5B)
    assert!(d["queries"].as_array().unwrap().contains(&json!("present")));
    assert_eq!(d["topics"].as_array().unwrap().len(), 12); // + view (RP2/T5), claims (RP2/T8), capsules (R3), viewer (RP3/M15), stories (R5/M18), comments (R5B)
    assert!(d["queries"].as_array().unwrap().contains(&json!("stories")));
    assert_eq!(d["role"], "owner");
    assert_eq!(d["principals"].as_array().unwrap().len(), 6);
}

#[test]
fn presence_input_and_screen() {
    let mut h = H::new();
    h.hello("p", "arkin", "phone");
    h.hello("s", "scout", "headless");
    let r = h.query("s", json!({"kind":"attention","principal":"arkin"}));
    assert_eq!(
        r["data"]["attention"]["viewport"],
        json!({"view":"main","from_bar":140,"to_bar":199})
    );
    let r = h.one(
        "p",
        json!({"type":"input","id":"i","gesture":{"kind":"pan","view":"main","bars":-500}}),
    );
    assert_eq!(r["data"]["viewport"]["from_bar"], 0);
    assert_eq!(r["data"]["viewport"]["to_bar"], 59);
    let r = h.one(
        "p",
        json!({"type":"input","id":"i","gesture":{"kind":"zoom","view":"main","factor":0.1}}),
    );
    assert_eq!(
        r["data"]["viewport"],
        json!({"view":"main","from_bar":0,"to_bar":199})
    );
    let r = h.one("p", json!({"type":"input","id":"i","gesture":{"kind":"zoom","view":"main","factor":1000.0,"anchor_bar":100}}));
    let vp = &r["data"]["viewport"];
    assert_eq!(
        vp["to_bar"].as_u64().unwrap() - vp["from_bar"].as_u64().unwrap(),
        1,
        "at least 2 bars"
    );
    let r = h.one("p", json!({"type":"input","id":"i","gesture":{"kind":"look","view":"main","from_bar":10,"to_bar":300}}));
    assert_eq!(code(&r), "invalid");
    let r = h.one(
        "p",
        json!({"type":"input","id":"i","gesture":{"kind":"tap","view":"main","panel":9,"bar":3}}),
    );
    assert_eq!(code(&r), "invalid");
    let r = h.one("p", json!({"type":"input","id":"i","gesture":{"kind":"look","view":"main","from_bar":100,"to_bar":160}}));
    assert_eq!(r["data"]["viewport"]["from_bar"], 100);
    let r = h.query("s", json!({"kind":"screen","principal":"arkin"}));
    let sc = &r["data"]["screens"][0];
    assert_eq!(sc["surface"], "phone");
    assert_eq!(sc["capsules"], json!(["cap-overbought"]));
    let close = h.s.document().series[0].bars().unwrap()[160].c;
    assert_eq!(sc["last_close"], close);
    let r = h.query("s", json!({"kind":"screen"}));
    assert_eq!(
        r["data"]["screens"],
        json!([]),
        "a headless connection has no screen"
    );
    // x.domain sets the initial range for new connections.
    h.cmd(
        "p",
        "c",
        json!([{"op":"set_view_range","view":"main","from_bar":20,"to_bar":30}]),
    );
    h.hello("d", "editor", "desktop");
    let r = h.query("s", json!({"kind":"attention","principal":"editor"}));
    assert_eq!(
        r["data"]["attention"]["viewport"],
        json!({"view":"main","from_bar":20,"to_bar":30})
    );
}

#[test]
fn attention_directives_keep_the_last_sixteen() {
    let mut h = two();
    for i in 0..20 {
        let r = h.one(
            "s",
            json!({"type":"attend","id":format!("a{i}"),"directive":{"to":"arkin","steps":[{"target":{"series":"px","from_bar":i,"to_bar":i}}]}}),
        );
        assert_eq!(r["created"], json!([format!("att-{}", i + 1)]));
    }
    let r = h.query("s", json!({"kind":"attention","principal":"arkin"}));
    let d = r["data"]["attention"]["directives"].as_array().unwrap();
    assert_eq!(d.len(), 16);
    assert_eq!(d[0]["id"], "att-5");
    let r = h.one("s", json!({"type":"attend","id":"x","directive":{"to":"arkin","steps":[{"target":{"series":"pxx"}}]}}));
    assert_eq!(r["error"]["path"], "/directive/steps/0/target/series");
}

#[test]
fn present_depths_and_defaults() {
    let mut h = two();
    h.hello("v", "arkin", "voice");
    let r = h.query("v", json!({"kind":"present"}));
    let p = &r["data"]["presentation"];
    assert_eq!(p["depth"], "sentence");
    assert!(p.get("glyphs").is_none() && p.get("view").is_none());
    let r = h.query("s", json!({"kind":"present","surface":"phone"}));
    let p = &r["data"]["presentation"];
    assert_eq!(p["glyphs"].as_array().unwrap().len(), 3);
    assert_eq!(p["spark"]["t"].as_array().unwrap().len(), 60);
    assert!(p.get("view").is_none());
    let r = h.query("s", json!({"kind":"present"}));
    assert_eq!(
        r["data"]["presentation"]["view"]["layers"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    h.cmd(
        "a",
        "c",
        json!([{"op":"reject_finding","finding":"fnd-overbought"}]),
    );
    let r = h.query("s", json!({"kind":"present"}));
    assert_eq!(code(&r), "not_found", "no live finding left");
}
