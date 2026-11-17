//! R5/M19 (K53, K42; `docs/CAPSULE.md` §9.10, `docs/PROTOCOL.md` §23.2): the feed's
//! coin-flip rule on the product path — set by `init.feed_rule`, it never looks ahead
//! (moving time on does not change a past decision), hidden stories keep counting in the
//! ledger, "few cases" is never hidden, the feed and the story list agree, the empty
//! feed's count, and the K42 exemption as a parameter.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use qp_schema::data::DataEvent;
use qp_schema::protocol::SessionInit;
use qp_session::Session;
use serde_json::{json, Value};

const BARS_SEED: u64 = 7;
const K: usize = 300;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn feed() -> Value {
    json!({"symbol":"THYAO","interval":"1d","feed":"bist"})
}

/// One instrument, `K` initial bars, the capsule engine on it, the host's rule `rule`.
fn init(rule: Option<Value>) -> SessionInit {
    let mut v = json!({
        "document": {
            "schema_version": "0", "id": "doc-m19", "title": "Akış kuralı",
            "series": [{"id":"px","instrument":{"symbol":"THYAO"},"interval":"1d",
                        "source":{"kind":"bind","symbol":"THYAO","interval":"1d",
                                  "window":{"last":10000},"feed":"bist"}}]
        },
        "principals": [
            {"id":"host","author":{"kind":"human","name":"host"},"role":"host"},
            {"id":"ana","author":{"kind":"human","name":"Ana"},"role":"owner"}
        ],
        "clock": 1_760_000_000_000_i64,
        "epoch": 0,
        "data": {
            "feeds": [{"feed": feed(), "synthetic": {"n": K, "seed": BARS_SEED}}],
            "entitlements": {"host": ["*"], "ana": ["*"]}
        },
        "capsules": {"series": ["px"]}
    });
    if let Some(r) = rule {
        v["feed_rule"] = r;
    }
    serde_json::from_value(v).unwrap()
}

fn send(s: &mut Session, msg: &Value) -> Vec<Value> {
    s.handle_raw("t", &msg.to_string())
        .iter()
        .map(|l| serde_json::to_value(&l.msg).unwrap())
        .collect()
}

fn data(s: &mut Session, q: Value) -> Value {
    let l = send(s, &json!({"type":"query","id":"q","query":q}));
    l[0]["data"].clone()
}

fn audience(s: &mut Session, sensitivity: &str, positions: &Value) {
    send(
        s,
        &json!({"type":"command","id":"a","ops":[{"op":"set_audience","audience":{
            "group":"future","sensitivity":sensitivity,"watchlist":["THYAO"],
            "positions":positions}}]}),
    );
}

/// A session with `rule`, Ana at `balanced`, the bars up to `upto` delivered.
fn session(rule: Option<Value>, upto: usize) -> Session {
    let mut s = Session::new(init(rule), &root()).unwrap();
    send(
        &mut s,
        &json!({"type":"hello","protocol":"qp/0","principal":"ana","surface":{"kind":"phone"}}),
    );
    audience(&mut s, "balanced", &json!({}));
    advance(&mut s, K, upto);
    s
}

fn advance(s: &mut Session, from: usize, to: usize) {
    let bars = qp_source::synthetic_bars(to, BARS_SEED);
    for (i, b) in bars.iter().enumerate().take(to).skip(from) {
        let ev: DataEvent = serde_json::from_value(
            json!({"feed": feed(), "seq": i - K + 1, "change": {"kind":"bar","bar": b}}),
        )
        .unwrap();
        s.data_event(&ev).unwrap();
    }
}

/// The story rows by id.
fn rows(s: &mut Session) -> BTreeMap<String, Value> {
    let d = data(
        s,
        json!({"kind":"stories","instrument":"THYAO","limit":500}),
    );
    d["stories"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| (r["id"].as_str().unwrap().to_owned(), r.clone()))
        .collect()
}

fn marked(r: &BTreeMap<String, Value>) -> Vec<&Value> {
    r.values()
        .filter(|x| x["coin_flip"] == json!(true))
        .collect()
}

const RULE: &str = "rate55n10";
const SHORT: usize = 420;
const LONG: usize = 540;

fn rule() -> Option<Value> {
    Some(json!({"rule": RULE}))
}

#[test]
fn a_past_decision_does_not_change_when_time_moves_on() {
    // Same session, time moved on.
    let mut s = session(rule(), SHORT);
    let before = rows(&mut s);
    assert!(!marked(&before).is_empty(), "the rule marks some stories");
    advance(&mut s, SHORT, LONG);
    let after = rows(&mut s);
    assert!(after.len() > before.len(), "new stories were born");
    for (id, r) in &before {
        assert_eq!(after[id]["coin_flip"], r["coin_flip"], "{id}");
    }
    // A session that never saw the bars after SHORT decides the same for those stories
    // as one that has them (the decision reads only the ledger at birth).
    let mut long = session(rule(), LONG);
    let long_rows = rows(&mut long);
    for (id, r) in &before {
        assert_eq!(long_rows[id]["coin_flip"], r["coin_flip"], "{id}");
    }
}

#[test]
fn hidden_stories_keep_counting_in_the_ledger() {
    let ledger = json!({"kind":"ledger","by_kind":true});
    let mut on = session(rule(), LONG);
    let mut off = session(None, LONG);
    let r = rows(&mut on);
    assert!(r.values().any(|x| x["hidden"] == json!("coin_flip")));
    assert_eq!(data(&mut on, ledger.clone()), data(&mut off, ledger));
    // Without a rule nothing is marked (the protocol default; the R4 bytes).
    assert!(marked(&rows(&mut off)).is_empty());
}

#[test]
fn few_cases_are_never_hidden_and_the_feed_agrees_with_the_list() {
    let mut s = session(rule(), LONG);
    let r = rows(&mut s);
    let mut few = 0;
    for x in r.values() {
        let n = x["face"]["ledger_n"].as_u64().unwrap();
        if n < 10 {
            few += 1;
            assert_ne!(x["coin_flip"], json!(true), "{x}");
        }
        if x["coin_flip"] == json!(true) {
            assert!(n >= 10, "{x}");
        }
    }
    assert!(few > 0, "some stories have few cases");
    // The feed's count of coin-flip-hidden stories is the list's.
    let f = data(
        &mut s,
        json!({"kind":"capsules","audience":true,"limit":500}),
    );
    let hidden = r
        .values()
        .filter(|x| x["hidden"] == json!("coin_flip"))
        .count();
    assert!(hidden > 0);
    assert_eq!(
        f["coin_flip_hidden"],
        json!(hidden),
        "{}",
        f["coin_flip_hidden"]
    );
    for c in f["capsules"].as_array().unwrap() {
        assert_ne!(c["coin_flip"], json!(true), "hidden below `all`: {c}");
    }
    // At the most open sensitivity they are in the feed, marked.
    audience(&mut s, "all", &json!({}));
    let f = data(
        &mut s,
        json!({"kind":"capsules","audience":true,"limit":500}),
    );
    assert!(f.get("coin_flip_hidden").is_none(), "{f}");
    let shown: Vec<&Value> = f["capsules"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["coin_flip"] == json!(true))
        .collect();
    assert_eq!(shown.len(), marked(&r).len());
}

#[test]
fn the_k42_exemption_is_a_parameter() {
    let held = json!({"THYAO": 100.0});
    let place = |exempt: bool| {
        let mut s = session(Some(json!({"rule": RULE, "exempt_negative": exempt})), LONG);
        audience(&mut s, "balanced", &held);
        rows(&mut s)
    };
    let with = place(true);
    let without = place(false);
    let neg = |x: &Value| {
        ["weakened", "invalidated", "retracted"].contains(&x["stage"].as_str().unwrap())
    };
    let cases: Vec<&String> = with
        .iter()
        .filter(|(_, x)| x["coin_flip"] == json!(true) && neg(x))
        .map(|(id, _)| id)
        .collect();
    assert!(
        !cases.is_empty(),
        "a coin-flip story on the holding turned negative"
    );
    for id in cases {
        assert_eq!(with[id]["in_feed"], json!(true), "exempt: {}", with[id]);
        assert_eq!(without[id]["in_feed"], json!(false), "{}", without[id]);
        assert_eq!(without[id]["hidden"], json!("coin_flip"));
    }
    // The ledger does not see the exemption either.
    assert_eq!(
        with.len(),
        without.len(),
        "the story list is the same, only the feed place differs"
    );
}

#[test]
fn an_unknown_rule_is_refused_with_the_names() {
    let Err(e) = Session::new(init(Some(json!({"rule":"nope"}))), &root()) else {
        panic!("an unknown rule must be refused");
    };
    assert!(e.contains("rate55") && e.contains("nope"), "{e}");
}
