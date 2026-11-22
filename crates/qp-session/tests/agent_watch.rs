//! (R5B, `docs/PROTOCOL.md` §25.5) Watches: lifecycle, limit, deterministic matching.

mod support;

use qp_schema::data::DataEvent;
use qp_schema::protocol::SessionInit;
use qp_session::Session;
use serde_json::{json, Value};
use support::root;

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
        json!({"type":"command","id":"w1","ops":[{"op":"add_watch","text":"THYAO'da bir şey olursa",
            "match":{"instrument":"THYAO"},"max_matches":3}]}),
    ));
    all.extend(send(
        &mut s,
        "b",
        json!({"type":"command","id":"w2","ops":[{"op":"add_watch","text":"olumsuz geçiş",
            "match":{"instrument":"THYAO","negative":true}}]}),
    ));
    all.extend(send(
        &mut s,
        "b",
        json!({"type":"command","id":"w3","ops":[{"op":"add_watch","text":"süresi dolan",
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
    let by_text = |t: &str| doc.intents.iter().find(|i| i.text == t).unwrap();
    let get = |t: &str| serde_json::to_value(by_text(t)).unwrap();
    let any = get("THYAO'da bir şey olursa");
    let any_id = by_text("THYAO'da bir şey olursa").id.clone();
    assert!(any_id.starts_with("watch-"), "{any_id}");
    assert_eq!(any["state"], "matched", "{any}");
    assert_eq!(any["matches"], 3);
    assert_eq!(any["active"], false);
    assert_eq!(any["watch"]["for"], "ana");
    assert_eq!(any["provenance"]["author"]["kind"], "agent");
    assert_eq!(get("süresi dolan")["state"], "expired");
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
        .filter(|l| l["conn"] == "a" && l["msg"]["event"]["intent"] == any_id.as_str())
        .count();
    assert_eq!(w_any, 3, "max_matches");
}

#[test]
fn removal_and_the_limit_per_person() {
    let mut s = Session::new(init(), &root()).unwrap();
    let w = hello(&mut s, "b", "bot");
    assert_eq!(w["msg"]["limits"]["max_watches"], 32);
    let mut ids = Vec::new();
    for _ in 0..32 {
        let r = send(
            &mut s,
            "b",
            json!({"type":"command","id":"c","ops":[{"op":"add_watch",
                "text":"x","match":{"instrument":"THYAO"}}]}),
        );
        assert_eq!(r[0]["msg"]["status"], "applied", "{r:?}");
        ids.push(r[0]["msg"]["created"][0].as_str().unwrap().to_owned());
    }
    let (w0, w1) = (ids[0].clone(), ids[1].clone());
    let r = send(
        &mut s,
        "b",
        json!({"type":"command","id":"c","ops":[{"op":"add_watch","text":"x","match":{}}]}),
    );
    assert_eq!(r[0]["msg"]["error"]["code"], "limit", "{r:?}");
    let r = send(
        &mut s,
        "b",
        json!({"type":"command","id":"c","ops":[{"op":"remove_watch","intent":w0}]}),
    );
    assert_eq!(r[0]["msg"]["status"], "applied", "{r:?}");
    let i = s.document().intents.iter().find(|i| i.id == w0).unwrap();
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
        json!({"type":"command","id":"c","ops":[{"op":"remove_watch","intent":w1}]}),
    );
    // (Privacy) Another person's watch reads as a missing one; no suggestion names it.
    assert_eq!(r[0]["msg"]["error"]["code"], "not_found", "{r:?}");
    assert!(r[0]["msg"]["error"].get("did_you_mean").is_none(), "{r:?}");
    // Nor switch it off through the intent op; the owner of the watch is told to use
    // `remove_watch`.
    let r = send(
        &mut s,
        "e",
        json!({"type":"command","id":"c","ops":[{"op":"set_intent_active","intent":w1,"active":false}]}),
    );
    // (eve is a viewer: her role refuses the intent op before any lookup, the same for
    // every id; the lookup itself is tested with an editor in `agent_policy.rs`.)
    assert_eq!(r[0]["msg"]["error"]["code"], "forbidden", "{r:?}");
    let r = send(
        &mut s,
        "b",
        json!({"type":"command","id":"c","ops":[{"op":"set_intent_active","intent":w1,"active":false}]}),
    );
    assert_eq!(
        r[0]["msg"]["error"]["reason"], "agent_cannot_write",
        "{r:?}"
    );
    // Nor does she read it anywhere.
    let d = send(
        &mut s,
        "e",
        json!({"type":"query","id":"d","query":{"kind":"document"}}),
    );
    assert!(!d[0].to_string().contains(&format!("\"{w1}\"")), "{d:?}");
    let r = send(
        &mut s,
        "b",
        json!({"type":"command","id":"c","ops":[{"op":"add_watch","for":"eve","text":"x","match":{}}]}),
    );
    assert_eq!(r[0]["msg"]["error"]["code"], "forbidden", "{r:?}");
}

#[test]
fn a_watch_makes_stories_relevant_to_its_person_only() {
    let mut s = Session::new(init(), &root()).unwrap();
    hello(&mut s, "a", "ana");
    hello(&mut s, "e", "eve");
    s.mark_agent_connection("b");
    hello(&mut s, "b", "bot");
    let r = send(
        &mut s,
        "b",
        json!({"type":"command","id":"w","ops":[{"op":"add_watch","text":"x","match":{"instrument":"THYAO"}}]}),
    );
    assert_eq!(r[0]["msg"]["status"], "applied", "{r:?}");
    let q = json!({"type":"query","id":"q","query":{"kind":"capsules","limit":3}});
    let rel = |r: &[Value]| {
        r[0]["msg"]["data"]["capsules"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["relevant"] == true)
    };
    assert!(rel(&send(&mut s, "a", q.clone())), "ana's watch");
    assert!(rel(&send(&mut s, "b", q.clone())), "her agent reads as ana");
    assert!(!rel(&send(&mut s, "e", q)), "not eve's");
}

/// (Privacy, §25.5) Every path that could touch or reveal another person's watch answers
/// as for an id that does not exist (the id aside).
#[test]
fn another_persons_watch_id_reads_like_a_missing_id_on_every_path() {
    let mut i = init();
    // `eve`'s agent (a reader for an entitled person; any person would do).
    i.principals.push(
        serde_json::from_value(json!({"id":"eb","author":{"kind":"agent","name":"eb"},
            "role":"proposer","on_behalf":"eve"}))
        .unwrap(),
    );
    let mut s = Session::new(i, &root()).unwrap();
    hello(&mut s, "b", "bot");
    hello(&mut s, "E", "eb");
    hello(&mut s, "e", "eve");
    let r = send(
        &mut s,
        "b",
        json!({"type":"command","id":"c","ops":[{"op":"add_watch","text":"x","match":{"instrument":"THYAO"}}]}),
    );
    let theirs = r[0]["msg"]["created"][0].as_str().unwrap().to_owned();
    // A missing id of the same shape (its last digit changed).
    let missing = format!("{}9", &theirs[..theirs.len() - 1]);
    type Path = (&'static str, Box<dyn Fn(&str) -> Value>);
    let paths: Vec<Path> = vec![
        (
            "remove_watch",
            Box::new(
                |id: &str| json!({"type":"command","id":"c","ops":[{"op":"remove_watch","intent":id}]}),
            ),
        ),
        (
            "set_intent_active",
            Box::new(
                |id: &str| json!({"type":"command","id":"c","ops":[{"op":"set_intent_active","intent":id,"active":false}]}),
            ),
        ),
        (
            "remove",
            Box::new(|id: &str| json!({"type":"command","id":"c","ops":[{"op":"remove","id":id}]})),
        ),
        (
            "add_intent id",
            Box::new(
                |id: &str| json!({"type":"command","id":"c","ops":[{"op":"add_intent","id":id,"text":"t"}]}),
            ),
        ),
        (
            "add_narrative about",
            Box::new(|id: &str| {
                json!({"type":"command","id":"c","ops":[{"op":"add_narrative","about":id,
            "steps":[{"at_ms":0,"target":{"view":"x"},"say":"s"}]}]})
            }),
        ),
        (
            "get",
            Box::new(|id: &str| json!({"type":"query","id":"q","query":{"kind":"get","id":id}})),
        ),
    ];
    for conn in ["E", "e"] {
        for (name, msg) in &paths {
            let a = send(&mut s, conn, msg(&theirs));
            let b = send(&mut s, conn, msg(&missing));
            let norm = |v: &[Value], id: &str| v[0]["msg"].to_string().replace(id, "ID");
            assert_eq!(norm(&a, &theirs), norm(&b, &missing), "{conn} {name}");
            assert!(
                !a[0]["msg"].to_string().contains("intents"),
                "{conn} {name}: {a:?}"
            );
        }
        let l = send(
            &mut s,
            conn,
            json!({"type":"query","id":"q","query":{"kind":"list","collection":"intents"}}),
        );
        assert!(!l[0].to_string().contains(&theirs), "{conn} list");
    }
}
