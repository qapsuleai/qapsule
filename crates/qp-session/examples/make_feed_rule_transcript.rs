//! Generates the R5/M19 conformance transcript 64 (`fixtures/protocol/conformance/
//! 64-feed-rule.json`, `docs/PROTOCOL.md` §23.2, `docs/CAPSULE.md` §9.10): the host's
//! coin-flip rule on (`init.feed_rule`) — the feed (`capsules` with `audience`) and the
//! story list (`stories`) at `balanced` (coin-flip stories hidden, counted in
//! `coin_flip_hidden`) and at `all` (in the feed, marked `coin_flip`), the live `story`
//! events, and the K42 exemption with a position.
//!
//! The expected outputs are the reference authority's answers (welcome trimmed to its type
//! and `seq`). `qp conform` checks them in process, over `qp serve` and over MCP.
//! Deterministic: two runs write the same bytes.
//!
//! ```text
//! cargo run -p qp-session --example make_feed_rule_transcript            # check only
//! cargo run -p qp-session --example make_feed_rule_transcript -- --write
//! ```

use std::path::{Path, PathBuf};

use qp_schema::data::DataEvent;
use qp_schema::protocol::SessionInit;
use qp_session::Session;
use serde_json::{json, Value};

const SEED: u64 = 7;
/// Bars of the initial feed.
const K: usize = 1200;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn feed() -> Value {
    json!({"symbol":"THYAO","interval":"1d","feed":"bist"})
}

fn init() -> Value {
    json!({
        "document": {
            "schema_version": "0", "id": "doc-feed-rule", "title": "Akış kuralı",
            "series": [{"id":"px","instrument":{"symbol":"THYAO"},"interval":"1d",
                        "source":{"kind":"bind","symbol":"THYAO","interval":"1d",
                                  "window":{"last":10000},"feed":"bist"}}]
        },
        "principals": [
            {"id":"host","author":{"kind":"human","name":"host"},"role":"host"},
            {"id":"ana","author":{"kind":"human","name":"Ana"},"role":"viewer"}
        ],
        "clock": 1_760_000_000_000_i64,
        "epoch": 0,
        "data": {
            "feeds": [{"feed": feed(), "synthetic": {"n": K, "seed": SEED}}],
            "entitlements": {"host": ["*"], "ana": ["*"]}
        },
        "capsules": {"series": ["px"]},
        "feed_rule": {"rule": "rate55"}
    })
}

enum In {
    Send(Value, &'static str),
    Data(usize, &'static str),
}

fn q(id: &str, query: Value, note: &'static str) -> In {
    In::Send(json!({"type":"query","id":id,"query":query}), note)
}

fn audience(id: &str, sensitivity: &str, positions: Value, note: &'static str) -> In {
    In::Send(
        json!({"type":"command","id":id,"ops":[{"op":"set_audience","audience":{
            "group":"future","sensitivity":sensitivity,"watchlist":["THYAO"],
            "positions":positions}}]}),
        note,
    )
}

fn trim(v: Value) -> Value {
    if v["type"] == "welcome" {
        return json!({"type":"welcome","seq": v["seq"]});
    }
    v
}

fn t64() -> Value {
    let stories = json!({"kind":"stories","instrument":"THYAO","range":{"last":40}});
    let feed_q = json!({"kind":"capsules","audience":true,"limit":30});
    let steps = vec![
        In::Send(
            json!({"type":"hello","protocol":"qp/0","principal":"ana","surface":{"kind":"phone"}}),
            "The host runs the coin-flip rule `rate55` (hit rate below 0.55 with at least 20 closed cases in the ledger at birth; `init.feed_rule`).",
        ),
        audience("s1", "balanced", json!({}), "Ana reads tiers 1–2 of THYAO."),
        q("l1", stories.clone(), "The story list: a coin-flip story is listed with `coin_flip: true` and, below the `all` sensitivity, `in_feed: false, hidden: coin_flip`; a story with fewer than 20 cases (`face.few_cases`) is never hidden."),
        q("f1", feed_q.clone(), "The feed leaves the coin-flip stories out and says how many (`coin_flip_hidden`): the empty-feed text counts them."),
        audience("s2", "all", json!({}), "The most open sensitivity."),
        q("f2", feed_q.clone(), "At `all` the coin-flip stories are in the feed, marked `coin_flip: true` (K53 rule 4); nothing is counted as hidden."),
        audience("s3", "balanced", json!({"THYAO": 100.0}), "A position on THYAO, back at `balanced`."),
        q("l2", stories.clone(), "(K42) With the exemption (the default), a negative stage (`weakened`, `invalidated`, `retracted`) of a coin-flip story about the position stays in the feed."),
        In::Send(
            json!({"type":"subscribe","id":"st","topic":{"kind":"stories","instrument":"THYAO"}}),
            "Live updates of the list.",
        ),
        In::Data(K, "A final bar: changed rows come as `story` events with their feed place and mark."),
        In::Data(K + 1, "Another bar."),
        q("ld", json!({"kind":"ledger","by_kind":true}), "The ledger counts every closed story, hidden or not (K53 rule 2)."),
    ];
    let init_v = init();
    let init: SessionInit = serde_json::from_value(init_v.clone()).expect("init");
    let mut s = Session::new(init, &root()).expect("session");
    let mut out_steps = Vec::new();
    for st in steps {
        let (mut step, lines) = match st {
            In::Send(msg, note) => {
                let lines = s.handle_raw("a", &msg.to_string());
                (
                    json!({"send": {"conn": "a", "msg": msg}, "note": note}),
                    lines,
                )
            }
            In::Data(i, note) => {
                let b = qp_source::synthetic_bars(i + 1, SEED)[i].clone();
                let ev =
                    json!({"feed": feed(), "seq": i - K + 1, "change": {"kind":"bar","bar": b}});
                let e: DataEvent = serde_json::from_value(ev.clone()).expect("event");
                let lines = s.data_event(&e).expect("data event applies");
                (json!({"data": ev, "note": note}), lines)
            }
        };
        step["expect"] = Value::Array(
            lines
                .iter()
                .map(
                    |l| json!({"conn": l.conn, "msg": trim(serde_json::to_value(&l.msg).unwrap())}),
                )
                .collect(),
        );
        out_steps.push(step);
    }
    json!({
        "name": "64-feed-rule",
        "description": "R5/M19 (K53, K42; PROTOCOL §23.2, CAPSULE §9.10): the host's coin-flip rule on — the feed and the story list at `balanced` and `all`, the hidden count, the K42 exemption with a position, the live `story` events and the ledger, which the rule never touches.",
        "covers": ["feed-rule", "coin-flip", "stories", "capsules", "feed-hidden", "k42"],
        "init": init_v,
        "steps": out_steps,
        "final": {"log_len": s.log().len()}
    })
}

/// What the transcript shows, for the report: rows, marks and hidden counts per step.
fn stats(t: &Value) {
    for st in t["steps"].as_array().unwrap() {
        for e in st["expect"].as_array().unwrap() {
            let d = &e["msg"]["data"];
            if let Some(rows) = d["stories"].as_array() {
                let cf = rows
                    .iter()
                    .filter(|r| r["coin_flip"] == json!(true))
                    .count();
                let hid = rows
                    .iter()
                    .filter(|r| r["hidden"] == json!("coin_flip"))
                    .count();
                let few = rows
                    .iter()
                    .filter(|r| r["face"]["few_cases"] == json!(true))
                    .count();
                println!(
                    "stories: {} rows, coin_flip {cf}, hidden coin_flip {hid}, few cases {few}",
                    rows.len()
                );
            }
            if let Some(c) = d["capsules"].as_array() {
                let cf = c.iter().filter(|r| r["coin_flip"] == json!(true)).count();
                println!(
                    "capsules: {} items, coin_flip {cf}, coin_flip_hidden {}",
                    c.len(),
                    d["coin_flip_hidden"]
                );
            }
            if e["msg"]["event"]["kind"] == "story" {
                println!(
                    "story event: coin_flip {} in_feed {}",
                    e["msg"]["event"]["story"]["coin_flip"], e["msg"]["event"]["story"]["in_feed"]
                );
            }
        }
    }
}

fn main() {
    let t = t64();
    if std::env::args().any(|a| a == "--stats") {
        stats(&t);
        return;
    }
    let write = std::env::args().any(|a| a == "--write");
    let path = root().join("fixtures/protocol/conformance/64-feed-rule.json");
    let mut text = serde_json::to_string_pretty(&t).expect("json");
    text.push('\n');
    let same = std::fs::read_to_string(&path).is_ok_and(|x| x == text);
    if write && !same {
        std::fs::write(&path, &text).expect("write");
        println!("wrote {}", path.display());
    } else if same {
        println!("same: {}", path.display());
    } else {
        println!("differs: {}", path.display());
        std::process::exit(1);
    }
}
