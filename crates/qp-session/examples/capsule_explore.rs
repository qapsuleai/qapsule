//! Exploration helper (R3/S-C): which capsule transitions a synthetic bound feed gives,
//! bar by bar, with the authority's capsule engine. Prints one line per engine entry.
//!
//! ```text
//! cargo run -p qp-session --example capsule_explore -- <seed> <n> <k>
//! ```

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::cognitive_complexity
)]

use std::path::Path;

use qp_schema::protocol::SessionInit;
use qp_session::Session;
use serde_json::{json, Value};

fn main() {
    let args: Vec<u64> = std::env::args()
        .skip(1)
        .map(|a| a.parse().expect("number"))
        .collect();
    let seed = args.first().copied().unwrap_or(1);
    let n = args.get(1).copied().unwrap_or(160) as usize;
    let k = args.get(2).copied().unwrap_or(60) as usize;
    let bars = &qp_source::synthetic_market(1, n, seed)[0].bars;
    let init: SessionInit = serde_json::from_value(json!({
        "document": {"schema_version":"0","id":"d","series":[{"id":"px","source":{"kind":"bind","symbol":"THYAO","interval":"1d","window":{"last":10000},"feed":"bist"}}]},
        "principals": [
            {"id":"host","author":{"kind":"human","name":"host"},"role":"host"},
            {"id":"ana","author":{"kind":"human","name":"ana"},"role":"viewer"},
            {"id":"bora","author":{"kind":"human","name":"bora"},"role":"viewer"}
        ],
        "clock": 0,
        "data": {"feeds":[{"feed":{"symbol":"THYAO","interval":"1d","feed":"bist"},"bars": bars[..k]}],
                 "entitlements":{"host":["*"],"ana":["*"],"bora":["bist/THYAO"]}},
        "capsules": {"series":["px"],"flow":[{"series":"px","seed":seed}]}
    }))
    .unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut s = Session::new(init, &root).unwrap();
    s.handle_raw(
        "a",
        &json!({"type":"hello","protocol":"qp/0","principal":"ana","surface":{"kind":"headless"}})
            .to_string(),
    );
    s.handle_raw(
        "a",
        &json!({"type":"subscribe","id":"c","topic":{"kind":"capsules","all":true}}).to_string(),
    );
    s.handle_raw(
        "b",
        &json!({"type":"hello","protocol":"qp/0","principal":"bora","surface":{"kind":"headless"}})
            .to_string(),
    );
    s.handle_raw(
        "b",
        &json!({"type":"subscribe","id":"c","topic":{"kind":"capsules","all":true}}).to_string(),
    );
    println!("load: log {}", s.log().len());
    for (i, b) in bars.iter().enumerate().skip(k) {
        let ev = serde_json::from_value(json!({"feed":{"symbol":"THYAO","interval":"1d","feed":"bist"},"seq": (i - k) as u64 + 1,"change":{"kind":"bar","bar":b}})).unwrap();
        let lines = s.data_event(&ev).unwrap();
        if i < k + 3 {
            println!(
                "bar {i}: {} lines, log {}, state {:?}",
                lines.len(),
                s.log().len(),
                s.binding_state("px")
            );
        }
        for l in &lines {
            let v: Value = serde_json::to_value(&l.msg).unwrap();
            if v["event"]["kind"] == "capsule" && l.conn == "a" {
                println!(
                    "bar {i}: {} {} {} tier {} | {}",
                    v["event"]["capsule"]["id"].as_str().unwrap(),
                    v["event"]["transition"]["stage"].as_str().unwrap(),
                    v["event"]["transition"]["reason"].as_str().unwrap(),
                    v["event"]["capsule"]["tier"],
                    v["event"]["capsule"]["sentence"]["tr"].as_str().unwrap()
                );
            } else if v["event"]["kind"] == "server_comment" {
                println!("bar {i}: comment {}", v["event"]["comment"]["codes"]);
            }
        }
    }
    println!("log {}", s.log().len());
}
