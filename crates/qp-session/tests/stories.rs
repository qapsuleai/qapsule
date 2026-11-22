//! R5/M18 (`docs/PROTOCOL.md` §23): the instrument's story list — non-interference of
//! the licensed flow, what an unentitled reader's bytes may hold, and the K53 coin-flip
//! rule's place in the feed.
//!
//! The R3/K1 lesson (`rounds/R3/AUDIT.md` Y1): a value scan does not see a number derived
//! from licensed data (the flow's strength reached readers as `confidence`). So the main
//! test is a comparison: same public bars, different licensed flows → every line a reader
//! without the flow entitlement gets (story answers, `story` events) is byte-identical
//! once the server comments (the one channel left, Ö37) are taken out. The scan tests add
//! the field and digit rules on top.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::cognitive_complexity
)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use qp_schema::data::DataEvent;
use qp_schema::protocol::{OutboundLine, SessionInit};
use qp_session::Session;
use serde_json::{json, Value};

const BARS_SEED: u64 = 7;
const K: usize = 300;
const N: usize = 360;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn feed() -> Value {
    json!({"symbol":"THYAO","interval":"1d","feed":"bist"})
}

/// Transcript 60's construction with flow seed `seed` (0: no licensed flow at all).
fn init(seed: u64) -> SessionInit {
    let flow = if seed == 0 {
        json!([])
    } else {
        json!([{"series": "px", "seed": seed}])
    };
    serde_json::from_value(json!({
        "document": {
            "schema_version": "0", "id": "doc-stories", "title": "Hikâye",
            "series": [{"id":"px","instrument":{"symbol":"THYAO"},"interval":"1d",
                        "source":{"kind":"bind","symbol":"THYAO","interval":"1d",
                                  "window":{"last":10000},"feed":"bist"}}]
        },
        "principals": [
            {"id":"host","author":{"kind":"human","name":"host"},"role":"host"},
            {"id":"ana","author":{"kind":"human","name":"Ana"},"role":"viewer"},
            {"id":"bora","author":{"kind":"human","name":"Bora"},"role":"viewer"},
            {"id":"can","author":{"kind":"human","name":"Can"},"role":"viewer"}
        ],
        "clock": 1_760_000_000_000_i64,
        "epoch": 0,
        "data": {
            "feeds": [{"feed": feed(), "synthetic": {"n": K, "seed": BARS_SEED}}],
            "entitlements": {"host": ["*"], "ana": ["*"], "bora": ["bist/THYAO"], "can": []}
        },
        "capsules": {"series": ["px"], "flow": flow}
    }))
    .unwrap()
}

fn send(s: &mut Session, conn: &str, msg: &Value) -> Vec<OutboundLine> {
    s.handle_raw(conn, &msg.to_string())
}

fn stories_q(q: Value) -> Value {
    json!({"type":"query","id":"q","query":q})
}

/// One run: hellos, audiences, `stories` subscriptions, the bars `K..N` one by one, an
/// audience change, then story queries (with a followed page cursor) per connection.
fn run(seed: u64, rule: Option<qp_session::CoinFlipRule>) -> Vec<OutboundLine> {
    let mut s = Session::new(init(seed), &root()).unwrap();
    s.set_coin_flip_rule(rule);
    let mut out = Vec::new();
    for (c, who, lang) in [("t", "ana", "tr"), ("b", "bora", "tr"), ("c", "can", "en")] {
        out.extend(send(
            &mut s,
            c,
            &json!({"type":"hello","protocol":"qp/0","principal":who,
                    "surface":{"kind":"phone","lang":lang}}),
        ));
        out.extend(send(
            &mut s,
            c,
            &json!({"type":"command","id":"aud","ops":[{"op":"set_audience",
            "audience":{"group":"future","sensitivity":"balanced","watchlist":["THYAO"]}}]}),
        ));
        out.extend(send(
            &mut s,
            c,
            &stories_q(json!({"kind":"stories","instrument":"THYAO","range":{"last":40}})),
        ));
        out.extend(send(
            &mut s,
            c,
            &json!({"type":"subscribe","id":"st","topic":{"kind":"stories","instrument":"THYAO"}}),
        ));
    }
    let bars = qp_source::synthetic_bars(N, BARS_SEED);
    for (i, b) in bars.iter().enumerate().take(N).skip(K) {
        let ev: DataEvent = serde_json::from_value(json!({"feed": feed(), "seq": i - K + 1,
            "change": {"kind":"bar","bar": b}}))
        .unwrap();
        out.extend(s.data_event(&ev).unwrap());
    }
    for c in ["t", "b", "c"] {
        out.extend(send(
            &mut s,
            c,
            &json!({"type":"command","id":"lvl","ops":[{"op":"set_sensitivity","sensitivity":"important"}]}),
        ));
        out.extend(send(
            &mut s,
            c,
            &stories_q(json!({"kind":"stories","instrument":"THYAO","limit":500})),
        ));
        let first = send(
            &mut s,
            c,
            &stories_q(
                json!({"kind":"stories","instrument":"THYAO","range":{"last":120},"limit":7}),
            ),
        );
        let next = serde_json::to_value(&first[0].msg).unwrap()["data"]["next"].clone();
        out.extend(first);
        if next.is_string() {
            out.extend(send(
                &mut s,
                c,
                &stories_q(
                    json!({"kind":"stories","instrument":"THYAO","range":{"last":120},
                    "limit":7,"after":next}),
                ),
            ));
        }
    }
    out
}

/// Removes the server comments (mühür messages) from a message.
fn strip_comments(v: &mut Value) {
    if let Value::Object(m) = v {
        m.remove("comments");
        for x in m.values_mut() {
            strip_comments(x);
        }
    }
}

/// Connection `conn`'s lines, server comment events dropped and `comments` removed.
fn stripped(out: &[OutboundLine], conn: &str) -> Vec<String> {
    out.iter()
        .filter(|l| l.conn == conn)
        .filter_map(|l| {
            let mut v = serde_json::to_value(&l.msg).unwrap();
            if v.pointer("/event/kind") == Some(&json!("server_comment")) {
                return None;
            }
            strip_comments(&mut v);
            Some(serde_json::to_string(&v).unwrap())
        })
        .collect()
}

fn lines_of(out: &[OutboundLine], conn: &str) -> Vec<String> {
    out.iter()
        .filter(|l| l.conn == conn)
        .map(|l| serde_json::to_string(&l.msg).unwrap())
        .collect()
}

/// Every story row in `conn`'s lines (answers and events).
fn rows_of(out: &[OutboundLine], conn: &str) -> Vec<Value> {
    let mut rows = Vec::new();
    for l in out.iter().filter(|l| l.conn == conn) {
        let v = serde_json::to_value(&l.msg).unwrap();
        if let Some(a) = v.pointer("/data/stories").and_then(Value::as_array) {
            rows.extend(a.iter().cloned());
        }
        if v.pointer("/event/kind") == Some(&json!("story")) {
            rows.push(v["event"]["story"].clone());
        }
    }
    rows
}

#[test]
fn licensed_flow_does_not_interfere_with_the_story_list() {
    let base = run(7, None);
    let mut ana_differs = false;
    let mut comments_differ = false;
    let mut events = 0;
    for seed in [1_u64, 2, 42] {
        let other = run(seed, None);
        for c in ["b", "c"] {
            let x = stripped(&base, c);
            let y = stripped(&other, c);
            assert!(x.len() > 20, "{c}: {} lines", x.len());
            assert!(x == y, "seed 7 vs {seed}: {c} differs");
            comments_differ |= lines_of(&base, c) != lines_of(&other, c);
        }
        ana_differs |= lines_of(&base, "t") != lines_of(&other, "t");
    }
    for c in ["b", "c"] {
        events += stripped(&base, c)
            .iter()
            .filter(|l| l.contains("\"kind\":\"story\""))
            .count();
    }
    assert!(
        events > 5,
        "story events reached the unentitled readers: {events}"
    );
    // Without any licensed flow: the same (what the reader would get if the licensed data
    // did not exist), mühür aside.
    let none = run(0, None);
    for c in ["b", "c"] {
        assert!(
            stripped(&base, c) == stripped(&none, c),
            "flow vs none: {c} differs"
        );
    }
    // Positive controls.
    assert!(
        ana_differs,
        "the entitled reader's story list never changed"
    );
    assert!(comments_differ, "the server comments never changed");
}

/// The keys a story row may carry (no evidence value, confidence, raw importance,
/// strength, ledger counts or data digest — for anyone).
const ROW_KEYS: &[&str] = &[
    "id",
    "finding",
    "series",
    "kind",
    "direction",
    "tier",
    "born_bar",
    "born_t",
    "last_bar",
    "last_t",
    "closed_bar",
    "closed_t",
    "stage",
    "outcome",
    "validity",
    "in_feed",
    "hidden",
    "coin_flip",
    "face",
    "redacted",
];
const FACE_KEYS: &[&str] = &[
    "event",
    "direction_word",
    "when",
    "moved",
    "validity_phrase",
    "ledger_short",
    "few_cases",
    "ledger_n",
];

#[test]
fn unentitled_bytes_hold_no_licensed_story_value_or_digit() {
    let out = run(7, None);
    let ana: BTreeSet<String> = rows_of(&out, "t")
        .iter()
        .map(|r| r["id"].as_str().unwrap().to_owned())
        .collect();
    let bora: BTreeSet<String> = rows_of(&out, "b")
        .iter()
        .map(|r| r["id"].as_str().unwrap().to_owned())
        .collect();
    let licensed: Vec<&String> = ana.difference(&bora).collect();
    assert!(
        licensed.iter().any(|i| i.contains(".flow.")),
        "the entitled reader has licensed flow stories: {licensed:?}"
    );
    for c in ["b", "c"] {
        for l in lines_of(&out, c) {
            for id in &licensed {
                assert!(
                    !l.contains(&format!("\"{id}\"")),
                    "{c} got licensed id {id}"
                );
            }
            for word in [
                "\"confidence\"",
                "\"importance\"",
                "\"strength\"",
                "\"evidence_ref\"",
                "\"values\"",
                "\"ledger_at_birth\"",
                "\"digest\"",
            ] {
                assert!(!l.contains(word), "{c} got {word}");
            }
        }
    }
    for c in ["t", "b", "c"] {
        let rows = rows_of(&out, c);
        assert!(!rows.is_empty());
        for r in &rows {
            for k in r.as_object().unwrap().keys() {
                assert!(ROW_KEYS.contains(&k.as_str()), "{c}: row key {k}");
            }
            for k in r["face"].as_object().unwrap().keys() {
                assert!(FACE_KEYS.contains(&k.as_str()), "{c}: face key {k}");
            }
        }
    }
    // Can may read no data: every row is digit-free (K33).
    let can = rows_of(&out, "c");
    for r in &can {
        assert_eq!(r["redacted"], json!(true), "{r}");
        assert!(r["face"].get("ledger_n").is_none(), "{r}");
        for (k, v) in r["face"].as_object().unwrap() {
            if let Some(t) = v.as_str() {
                assert!(
                    !t.chars().any(|ch| ch.is_ascii_digit()),
                    "can face {k}: {t}"
                );
            }
        }
    }
    // Bora may read the bars: digits in his faces are allowed, and present somewhere.
    assert!(rows_of(&out, "b")
        .iter()
        .any(|r| r["face"].get("ledger_n").is_some()));
}

/// A test rule: a ledger at birth with at least 4 closed rows and a realized share below
/// 60 % (R5/M19: the product's rules are `qp_capsule::feedrule::FEED_RULES`; this one marks
/// more stories on the short test history).
const HALF: qp_session::CoinFlipRule = qp_session::CoinFlipRule {
    hide: qp_capsule::feedrule::HideRule::Rate { min_n: 4, tau: 0.6 },
    exempt_negative: true,
};

fn query_one(s: &mut Session, conn: &str, q: Value) -> Value {
    let l = send(s, conn, &stories_q(q));
    serde_json::to_value(&l[0].msg).unwrap()["data"].clone()
}

#[test]
fn coin_flip_rule_marks_hides_below_all_and_keeps_the_ledger() {
    let mut s = Session::new(init(7), &root()).unwrap();
    send(
        &mut s,
        "t",
        &json!({"type":"hello","protocol":"qp/0","principal":"ana","surface":{"kind":"phone"}}),
    );
    let ledger_q = json!({"kind":"ledger","by_kind":true});
    let ledger_before = query_one(&mut s, "t", ledger_q.clone());
    let list_q = json!({"kind":"stories","instrument":"THYAO","limit":500});
    let feed_q = json!({"kind":"capsules","audience":true});
    let unmarked = query_one(&mut s, "t", list_q.clone());
    assert!(unmarked["stories"]
        .as_array()
        .unwrap()
        .iter()
        .all(|r| r.get("coin_flip").is_none()));
    s.set_coin_flip_rule(Some(HALF));
    for (level, shown) in [("important", false), ("balanced", false), ("all", true)] {
        send(
            &mut s,
            "t",
            &json!({"type":"command","id":"l","ops":[{"op":"set_audience",
                "audience":{"group":"habituated","sensitivity":level}}]}),
        );
        let list = query_one(&mut s, "t", list_q.clone());
        let rows = list["stories"].as_array().unwrap();
        let marked: Vec<&Value> = rows
            .iter()
            .filter(|r| r["coin_flip"] == json!(true))
            .collect();
        assert!(!marked.is_empty(), "the rule marks some stories");
        let feed = query_one(&mut s, "t", feed_q.clone());
        let in_feed: BTreeSet<&str> = feed["capsules"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["id"].as_str().unwrap())
            .collect();
        for r in &marked {
            let id = r["id"].as_str().unwrap();
            // (No position or intent here: the K42 override does not apply.)
            if shown {
                assert_eq!(r["in_feed"], json!(r.get("hidden").is_none()), "{r}");
            } else {
                assert_eq!(r["in_feed"], json!(false), "{level}: {r}");
                assert_eq!(r["hidden"], json!("coin_flip"), "{level}: {r}");
                assert!(!in_feed.contains(id), "{level}: {id} in the feed");
            }
        }
        // The row's feed place is the feed's rule: a row in the feed is in the answer.
        for r in rows {
            let id = r["id"].as_str().unwrap();
            assert_eq!(
                r["in_feed"] == json!(true),
                in_feed.contains(id),
                "{level}: {id}"
            );
        }
        if shown {
            assert!(marked.iter().any(|r| r["in_feed"] == json!(true)));
        }
    }
    // K53 rule 2: hidden stories still count in the ledger.
    assert_eq!(query_one(&mut s, "t", ledger_q), ledger_before);
}
