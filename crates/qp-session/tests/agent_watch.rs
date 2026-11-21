//! (R5B, `docs/PROTOCOL.md` §25.5) Watches: lifecycle, limit, deterministic matching.

mod common;

use common::root;
use qp_schema::data::DataEvent;
use qp_schema::protocol::SessionInit;
use qp_session::Session;
use serde_json::{json, Value};

const SEED: u64 = 7;
const K: usize = 300;

fn feed() -> Value {
    json!({"symbol":"THYAO","interval":"1d","feed":"bist"})
}

fn init() -> SessionInit {
    serde_json::from_value(json!({
        "document": {
            "schema_version": "0", "id": "doc-watch", "title": "Niyet",
            "series": [{"id":"px","instrument":{"symbol":"THYAO"},"interval":"1d",
                        "source":{"kind":"bind","symbol":"THYAO","interval":"1d",
                                  "window":{"last":10000},"feed":"bist"}}]
        },
        "principals": [
            {"id":"host","author":{"kind":"human","name":"host"},"role":"host"},
            {"id":"ana","author":{"kind":"human","name":"Ana"},"role":"viewer"},
            {"id":"bot","author":{"kind":"agent","name":"bot"},"role":"proposer","on_behalf":"ana"},
            {"id":"eve","author":{"kind":"human","name":"Eve"},"role":"viewer"}
        ],
        "clock": 1_760_000_000_000_i64,
        "data": {
            "feeds": [{"feed": feed(), "synthetic": {"n": K, "seed": SEED}}],
            "entitlements": {"host": ["*"], "ana": ["*"], "eve": ["*"]}
        },
        "capsules": {"series": ["px"]},
        "agent_policy": "r5b"
    }))
    .unwrap()
}

fn lines(v: &[qp_schema::protocol::OutboundLine]) -> Vec<Value> {
    v.iter().map(|l| serde_json::to_value(l).unwrap()).collect()
}

fn send(s: &mut Session, conn: &str, msg: Value) -> Vec<Value> {
    lines(&s.handle_raw(conn, &msg.to_string()))
}

fn bar(s: &mut Session, i: usize) -> Vec<Value> {
    let b = qp_source::synthetic_bars(i + 1, SEED)[i].clone();
    let ev: DataEvent = serde_json::from_value(
        json!({"feed": feed(), "seq": i - K + 1, "change": {"kind":"bar","bar": b}}),
    )
    .unwrap();
    lines(&s.data_event(&ev).unwrap())
}

fn hello(s: &mut Session, conn: &str, p: &str) -> Value {
    send(
        s,
        conn,
        json!({"type":"hello","protocol":"qp/0","principal":p,"surface":{"kind":"headless"}}),
    )
    .remove(0)
}

/// A replay with watches; every output line, in order.
fn run() -> (Vec<Value>, Session) {
    let mut s = Session::new(init(), &root()).unwrap();
    let mut all = Vec::new();
    all.push(hello(&mut s, "a", "ana"));
    s.mark_agent_connection("b");
    all.push(hello(&mut s, "b", "bot"));
    all.push(hello(&mut s, "e", "eve"));
    for c in ["a", "b", "e"] {
        all.extend(send(
            &mut s,
            c,
            json!({"type":"subscribe","id":"w","topic":{"kind":"intents"}}),
        ));
    }
    all.extend(send(
        &mut s,
        "b",
        json!({"type":"command","id":"w1","ops":[{"op":"add_watch","id":"w-any","text":"THYAO'da bir şey olursa",
            "match":{"instrument":"THYAO"},"max_matches":3}]}),
    ));
    all.extend(send(
        &mut s,
        "b",
        json!({"type":"command","id":"w2","ops":[{"op":"add_watch","id":"w-neg","text":"olumsuz geçiş",
            "match":{"instrument":"THYAO","negative":true}}]}),
    ));
    all.extend(send(
        &mut s,
        "b",
        json!({"type":"command","id":"w3","ops":[{"op":"add_watch","id":"w-old","text":"süresi dolan",
            "match":{"instrument":"THYAO"},"expires_t":0}]}),
    ));
    for i in K..K + 80 {
        all.extend(bar(&mut s, i));
    }
    (all, s)
}

#[test]
fn matching_is_deterministic_byte_for_byte() {
    let (a, _) = run();
    let (b, _) = run();
    let ta: Vec<String> = a.iter().map(Value::to_string).collect();
    let tb: Vec<String> = b.iter().map(Value::to_string).collect();
    assert_eq!(ta, tb);
    let matched: Vec<&Value> = a
        .iter()
        .filter(|l| l["msg"]["event"]["kind"] == "intent_matched")
        .collect();
    assert!(matched.len() >= 3, "{} matches", matched.len());
}

#[test]
fn lifecycle_matched_expired_and_events_to_the_person_only() {
    let (all, s) = run();
    let doc = s.document();
    let get =
        |id: &str| serde_json::to_value(doc.intents.iter().find(|i| i.id == id).unwrap()).unwrap();
    let any = get("w-any");
    assert_eq!(any["state"], "matched", "{any}");
    assert_eq!(any["matches"], 3);
    assert_eq!(any["active"], false);
    assert_eq!(any["watch"]["for"], "ana");
    assert_eq!(any["provenance"]["author"]["kind"], "agent");
    assert_eq!(get("w-old")["state"], "expired");
    // The person and her agent get the matches; another person none.
    let to = |c: &str| {
        all.iter()
            .filter(|l| l["conn"] == c && l["msg"]["event"]["kind"] == "intent_matched")
            .count()
    };
    assert!(to("a") >= 3);
    assert_eq!(to("a"), to("b"));
    assert_eq!(to("e"), 0);
    let w_any = all
        .iter()
        .filter(|l| l["conn"] == "a" && l["msg"]["event"]["intent"] == "w-any")
        .count();
    assert_eq!(w_any, 3, "max_matches");
}

#[test]
fn removal_and_the_limit_per_person() {
    let mut s = Session::new(init(), &root()).unwrap();
    let w = hello(&mut s, "b", "bot");
    assert_eq!(w["msg"]["limits"]["max_watches"], 32);
    for i in 0..32 {
        let r = send(
            &mut s,
            "b",
            json!({"type":"command","id":"c","ops":[{"op":"add_watch","id":format!("w{i}"),
                "text":"x","match":{"instrument":"THYAO"}}]}),
        );
        assert_eq!(r[0]["msg"]["status"], "applied", "{r:?}");
    }
    let r = send(
        &mut s,
        "b",
        json!({"type":"command","id":"c","ops":[{"op":"add_watch","text":"x","match":{}}]}),
    );
    assert_eq!(r[0]["msg"]["error"]["code"], "limit", "{r:?}");
    let r = send(
        &mut s,
        "b",
        json!({"type":"command","id":"c","ops":[{"op":"remove_watch","intent":"w0"}]}),
    );
    assert_eq!(r[0]["msg"]["status"], "applied", "{r:?}");
    let i = s.document().intents.iter().find(|i| i.id == "w0").unwrap();
    assert_eq!(serde_json::to_value(i.state).unwrap(), "removed");
    let r = send(
        &mut s,
        "b",
        json!({"type":"command","id":"c","ops":[{"op":"add_watch","text":"x","match":{}}]}),
    );
    assert_eq!(r[0]["msg"]["status"], "applied", "{r:?}");
    // Another person's watch is theirs.
    hello(&mut s, "e", "eve");
    let r = send(
        &mut s,
        "e",
        json!({"type":"command","id":"c","ops":[{"op":"remove_watch","intent":"w1"}]}),
    );
    assert_eq!(r[0]["msg"]["error"]["code"], "forbidden", "{r:?}");
    let r = send(
        &mut s,
        "b",
        json!({"type":"command","id":"c","ops":[{"op":"add_watch","for":"eve","text":"x","match":{}}]}),
    );
    assert_eq!(r[0]["msg"]["error"]["code"], "forbidden", "{r:?}");
}
