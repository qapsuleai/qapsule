//! R4/T3 (K42): validity on the item, the glyph and the sentence say the same thing in
//! Turkish and English (A28, VERIFY 11); only negative transitions of a story about a
//! position pass any sensitivity (A22); the ledger key carries the direction (A23).
//!
//! The bound THYAO feed of the R3 transcripts (38–44), fed bar by bar.

use std::path::{Path, PathBuf};

use qp_schema::data::DataEvent;
use qp_schema::protocol::SessionInit;
use qp_schema::Bar;
use qp_session::Session;
use serde_json::{json, Value};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn bars() -> Vec<Bar> {
    qp_source::synthetic_market(1, 160, 1)[0].bars.clone()
}

fn init_k(k: usize) -> SessionInit {
    let t: Value = serde_json::from_str(
        &std::fs::read_to_string(
            root().join("fixtures/protocol/conformance/38-capsule-lifecycle.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let mut v = t["init"].clone();
    v["data"]["feeds"][0]["bars"] = json!(bars()[..k]);
    serde_json::from_value(v).unwrap()
}

fn ev(seq: usize, b: &Bar) -> DataEvent {
    serde_json::from_value(
        json!({"feed":{"symbol":"THYAO","interval":"1d","feed":"bist"},
        "seq": seq, "change": {"kind":"bar","bar": b}}),
    )
    .unwrap()
}

fn send(s: &mut Session, conn: &str, msg: &Value) -> Vec<Value> {
    s.handle_raw(conn, &msg.to_string())
        .iter()
        .map(|l| serde_json::to_value(&l.msg).unwrap())
        .collect()
}

fn hello(s: &mut Session, conn: &str, who: &str, lang: &str) {
    send(
        s,
        conn,
        &json!({"type":"hello","protocol":"qp/0","principal":who,"surface":{"kind":"headless","lang":lang}}),
    );
}

fn q(s: &mut Session, conn: &str, query: Value) -> Value {
    send(s, conn, &json!({"type":"query","id":"q","query":query})).remove(0)
}

/// VERIFY 11: at every bar, for every open capsule, the item's validity, the glyph's
/// validity and the sentence agree, in Turkish and in English.
#[test]
fn headline_and_glyph_agree_in_both_languages() {
    let b = bars();
    let k = 60;
    let mut s = Session::new(init_k(k), &root()).unwrap();
    hello(&mut s, "t", "ana", "tr");
    hello(&mut s, "e", "ana", "en");
    let mut no_longer = 0;
    let mut valid = 0;
    let mut closed = 0;
    for (i, bar) in b.iter().enumerate().skip(k) {
        s.data_event(&ev(i - k + 1, bar)).unwrap();
        let list = q(&mut s, "t", json!({"kind":"capsules"}));
        for it in list["data"]["capsules"].as_array().unwrap() {
            let id = it["id"].as_str().unwrap();
            let v = it["validity"].as_str().unwrap().to_owned();
            for (conn, lang) in [("t", "tr"), ("e", "en")] {
                let d = q(&mut s, conn, json!({"kind":"capsule","capsule":id}));
                let det = &d["data"]["detail"];
                assert_eq!(det["item"]["validity"], v.as_str(), "{id} item ({lang})");
                assert_eq!(det["glyph"]["validity"], v.as_str(), "{id} glyph ({lang})");
                let text = det["item"]["sentence"]["text"]
                    .as_str()
                    .unwrap()
                    .to_lowercase();
                let says_no = match lang {
                    "tr" => text.contains("artık"),
                    _ => text.contains("no longer"),
                };
                match v.as_str() {
                    "no_longer" => assert!(says_no, "{id} at bar {i} ({lang}): {text}"),
                    "valid" => assert!(!says_no, "{id} at bar {i} ({lang}): {text}"),
                    _ => {}
                }
            }
            match v.as_str() {
                "no_longer" => no_longer += 1,
                "valid" => valid += 1,
                _ => closed += 1,
            }
            assert_eq!(
                v == "closed",
                matches!(
                    it["stage"].as_str().unwrap(),
                    "realized" | "invalidated" | "expired" | "retracted"
                ),
                "{id}: closed iff the stage closes"
            );
        }
    }
    assert!(
        no_longer > 0 && valid > 0 && closed > 0,
        "{no_longer} {valid} {closed}"
    );
}

/// A22: with `important` and a position, a tier > 1 birth does not reach the holder; a
/// negative transition of a story about the position does; every notified non-negative
/// transition is tier 1.
#[test]
fn only_negative_transitions_override_the_level() {
    let b = bars();
    let k = 60;
    let mut s = Session::new(init_k(k), &root()).unwrap();
    hello(&mut s, "a", "ana", "tr");
    send(
        &mut s,
        "a",
        &json!({"type":"subscribe","id":"k","topic":{"kind":"capsules"}}),
    );
    send(
        &mut s,
        "a",
        &json!({"type":"command","id":"c","ops":[{"op":"set_audience","audience":{"group":"future","positions":{"THYAO":100}}}]}),
    );
    let mut negative_low_tier = 0;
    for (i, bar) in b.iter().enumerate().skip(k) {
        for l in s.data_event(&ev(i - k + 1, bar)).unwrap() {
            let m = serde_json::to_value(&l.msg).unwrap();
            if m["event"]["kind"] != "capsule" {
                continue;
            }
            let stage = m["event"]["transition"]["stage"].as_str().unwrap();
            let tier = m["event"]["capsule"]["tier"].as_u64().unwrap();
            let negative = matches!(stage, "weakened" | "invalidated" | "retracted");
            assert!(
                negative || tier == 1,
                "bar {i}: {stage} at tier {tier} passed"
            );
            if negative && tier > 1 {
                negative_low_tier += 1;
            }
        }
    }
    assert!(negative_low_tier > 0, "the run shows the override");
}

/// A23: every ledger row key names a direction; up and down rows of one detector differ.
#[test]
fn ledger_rows_carry_the_direction() {
    let mut s = Session::new(init_k(160), &root()).unwrap();
    hello(&mut s, "a", "ana", "tr");
    let l = q(&mut s, "a", json!({"kind":"ledger"}));
    let rows = l["data"]["rows"].as_array().unwrap();
    assert!(!rows.is_empty());
    for r in rows {
        let key = r["key"].as_str().unwrap();
        let dir = r["direction"].as_str().unwrap();
        assert_eq!(key.split('/').nth(1), Some(dir), "{key}");
    }
    let brk: Vec<&str> = rows
        .iter()
        .filter(|r| r["detector"] == "breakout.range")
        .map(|r| r["direction"].as_str().unwrap())
        .collect();
    assert_eq!(brk, ["down", "up"]);
}
