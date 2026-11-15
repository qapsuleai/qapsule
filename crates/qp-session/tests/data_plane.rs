//! The data plane in the reference authority (RP2/T4, `docs/PROTOCOL.md` §18): binding
//! validation, anchor stamping and the replay invariant, host data events (gap → stale,
//! rejected events, re-resolution), the reading document never leaking bars.

use std::path::{Path, PathBuf};

use qp_schema::data::{BindingState, DataEvent};
use qp_schema::protocol::SessionInit;
use qp_session::{replay, DataError, Session};
use serde_json::{json, Value};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn init(window: Value) -> SessionInit {
    let mut doc: Value = serde_json::from_str(
        &std::fs::read_to_string(root().join("fixtures/protocol/docs/thyao-bound.json")).unwrap(),
    )
    .unwrap();
    doc["series"][0]["source"]["window"] = window;
    serde_json::from_value(json!({
        "document": doc,
        "clock": 1_760_000_000_000_i64,
        "principals": [
            {"id":"arkin","author":{"kind":"human","name":"arkin"},"role":"owner"},
            {"id":"host","author":{"kind":"engine","name":"host","version":"0"},"role":"host"}
        ],
        "data": {
            "feeds": [{"feed":{"symbol":"THYAO","interval":"1d"},"synthetic":{"n":120,"seed":5}}],
            "entitlements": {"host":["*"], "arkin":["THYAO"]}
        }
    }))
    .unwrap()
}

fn session() -> Session {
    let mut s = Session::new(init(json!({"last": 100})), &root()).unwrap();
    let w = s.handle_raw(
        "a",
        r#"{"type":"hello","protocol":"qp/0","principal":"arkin","surface":{"kind":"headless"}}"#,
    );
    assert_eq!(serde_json::to_value(&w[0].msg).unwrap()["type"], "welcome");
    s
}

fn send(s: &mut Session, msg: &Value) -> Value {
    let out = s.handle_raw("a", &msg.to_string());
    serde_json::to_value(&out[0].msg).unwrap()
}

fn put(s: &mut Session, series: &Value) -> Value {
    send(
        s,
        &json!({"type":"command","id":"c","ops":[{"op":"put_series","series":series}]}),
    )
}

fn last_t(s: &mut Session) -> i64 {
    let r = send(
        s,
        &json!({"type":"query","id":"q","query":{"kind":"at","series":"px"}}),
    );
    r["data"]["at"]["ohlcv"]["t"].as_i64().unwrap()
}

fn ev(seq: u64, change: Value) -> DataEvent {
    serde_json::from_value(
        json!({"feed":{"symbol":"THYAO","interval":"1d"},"seq":seq,"change":change}),
    )
    .unwrap()
}

fn check_replay(s: &Session) {
    let r = replay(s.initial_document(), s.log()).unwrap();
    assert_eq!(
        serde_json::to_vec(&r).unwrap(),
        serde_json::to_vec(s.document()).unwrap()
    );
}

#[test]
fn load_anchors_once_and_replays() {
    let s = session();
    assert_eq!(s.log().len(), 1, "one anchor entry");
    assert_eq!(s.log()[0].author, "qp");
    let px = &s.document().series[0];
    assert!(px.data.is_none(), "the document holds no bound bars");
    assert!(px.binding().unwrap().anchor_t.is_some());
    check_replay(&s);
    // Restarting from the anchored document resolves from the anchor: no new entry.
    let mut i = init(json!({"last": 100}));
    i.document = qp_schema::protocol::DocumentRef::Inline(Box::new(s.document().clone()));
    let s2 = Session::new(i, &root()).unwrap();
    assert!(s2.log().is_empty());
    assert_eq!(s2.binding_state("px"), Some(BindingState::Ready));
}

#[test]
fn bound_series_validation() {
    let mut s = session();
    let base = json!({"id":"px2","source":{"kind":"bind","symbol":"THYAO","interval":"1d","window":{"last":10}}});
    let cases = [
        (json!({"anchor_t": 5}), "/ops/0/series/source/anchor_t"),
        (json!({"interval": "60m"}), "/ops/0/series/source/interval"),
        (json!({"interval": "2x"}), "/ops/0/series/source/interval"),
        (json!({"symbol": "thy ao"}), "/ops/0/series/source/symbol"),
        (json!({"feed": "-x"}), "/ops/0/series/source/feed"),
        (
            json!({"window": {"last": 0}}),
            "/ops/0/series/source/window",
        ),
        (
            json!({"window": {"last": 100_001}}),
            "/ops/0/series/source/window",
        ),
        (
            json!({"window": {"from_t": 5, "to_t": 4}}),
            "/ops/0/series/source/window",
        ),
    ];
    for (patch, path) in cases {
        let mut ser = base.clone();
        for (k, v) in patch.as_object().unwrap() {
            ser["source"][k] = v.clone();
        }
        let r = put(&mut s, &ser);
        assert_eq!(r["error"]["code"], "invalid", "{patch}: {r}");
        assert_eq!(r["error"]["path"], path, "{patch}: {r}");
    }
    let r = put(
        &mut s,
        &json!({"id":"px2","source":{"kind":"bind","symbol":"THYAO","interval":"60m","window":{"last":10}}}),
    );
    assert_eq!(r["error"]["did_you_mean"], json!(["1h"]), "{r}");
    let mut with_data = base.clone();
    with_data["data"] =
        json!({"kind":"bars","bars":[{"t":1,"o":1.0,"h":1.0,"l":1.0,"c":1.0,"v":1.0}]});
    assert_eq!(
        put(&mut s, &with_data)["error"]["path"],
        "/ops/0/series/data"
    );
    let mut sym = base.clone();
    sym["instrument"] = json!({"symbol":"ASELS"});
    assert_eq!(
        put(&mut s, &sym)["error"]["path"],
        "/ops/0/series/instrument/symbol"
    );
    let mut iv = base.clone();
    iv["interval"] = json!("1w");
    assert_eq!(put(&mut s, &iv)["error"]["path"], "/ops/0/series/interval");
    // Bar writes on a bound series.
    let r = send(
        &mut s,
        &json!({"type":"command","id":"c","ops":[{"op":"update_last_bar","series":"px","bar":{"t":1,"o":1.0,"h":1.0,"l":1.0,"c":1.0,"v":1.0}}]}),
    );
    assert_eq!(r["error"]["code"], "invalid", "{r}");
    assert!(r["error"]["message"].as_str().unwrap().contains("bound"));
    // A valid one resolves and is anchored by an engine entry right after the command.
    let n = s.log().len();
    let r = put(&mut s, &base);
    assert_eq!(r["status"], "applied", "{r}");
    assert_eq!(s.log().len(), n + 2);
    assert_eq!(s.log()[n + 1].author, "qp");
    assert_eq!(s.binding_state("px2"), Some(BindingState::Ready));
    check_replay(&s);
}

#[test]
fn bound_series_limit() {
    let mut s = session();
    let ops: Vec<Value> = (0..64)
        .map(|i| json!({"op":"put_series","series":{"id":format!("b{i}"),"source":{"kind":"bind","symbol":"THYAO","interval":"1d","window":{"last":5}}}}))
        .collect();
    for k in 0..3 {
        let r = send(
            &mut s,
            &json!({"type":"command","id":format!("c{k}"),"ops":ops.iter().map(|o| {
                let mut o = o.clone();
                let id = format!("{}-{k}", o["series"]["id"].as_str().unwrap());
                o["series"]["id"] = json!(id);
                o
            }).collect::<Vec<_>>()}),
        );
        assert_eq!(r["status"], "applied", "{r}");
    }
    // 1 + 192 bound now; 64 more passes the 256 limit at op 63.
    let r = send(&mut s, &json!({"type":"command","id":"c9","ops":ops}));
    assert_eq!(r["error"]["code"], "limit", "{r}");
    assert_eq!(r["error"]["path"], "/ops/63/series/source/kind", "{r}");
}

#[test]
fn data_events_gap_stale_reject_and_refresh() {
    let mut s = session();
    let t = last_t(&mut s);
    let next = t + 86_400_000;
    let bar = json!({"kind":"bar","bar":{"t":next,"o":100.0,"h":101.0,"l":99.0,"c":100.5,"v":1.0}});
    assert!(s.data_event(&ev(1, bar.clone())).unwrap().is_empty());
    assert_eq!(last_t(&mut s), next);
    // Rejected: h < l. seq does not advance.
    let bad = json!({"kind":"patch","t":next,"fields":{"h":50.0}});
    assert!(matches!(
        s.data_event(&ev(2, bad)),
        Err(DataError::Rejected { .. })
    ));
    // Unknown feed.
    let other: DataEvent = serde_json::from_value(json!({"feed":{"symbol":"ASELS","interval":"1d"},"seq":1,"change":{"kind":"patch","t":1,"fields":{"c":1.0}}})).unwrap();
    assert!(matches!(
        s.data_event(&other),
        Err(DataError::UnknownFeed(_))
    ));
    // Gap (3 after 1): stale; reads answer no_data; the document never changes.
    let doc = serde_json::to_vec(s.document()).unwrap();
    let patch = json!({"kind":"patch","t":next,"fields":{"c":100.7}});
    assert!(s.data_event(&ev(3, patch)).unwrap().is_empty());
    assert_eq!(s.binding_state("px"), Some(BindingState::Stale));
    let r = send(
        &mut s,
        &json!({"type":"query","id":"q","query":{"kind":"study","series":"rsi14"}}),
    );
    assert_eq!(r["error"]["code"], "no_data", "{r}");
    let r = send(
        &mut s,
        &json!({"type":"query","id":"q","query":{"kind":"binding"}}),
    );
    assert_eq!(r["data"]["bindings"][0]["state"], "stale");
    assert!(r["data"]["bindings"][0]["reason"]
        .as_str()
        .unwrap()
        .starts_with("gap"));
    assert_eq!(serde_json::to_vec(s.document()).unwrap(), doc);
    // The host re-resolves: ready again, from the anchor, with the provider's bars.
    s.refresh_bindings();
    assert_eq!(s.binding_state("px"), Some(BindingState::Ready));
    assert_eq!(last_t(&mut s), next);
    check_replay(&s);
}

#[test]
fn reading_never_leaks_bound_bars() {
    let mut s = session();
    for q in [
        json!({"kind":"document"}),
        json!({"kind":"get","id":"px"}),
        json!({"kind":"list","collection":"series","full":true}),
        json!({"kind":"log"}),
    ] {
        let r = send(&mut s, &json!({"type":"query","id":"q","query":q}));
        let text = r.to_string();
        assert!(!text.contains("\"bars\":["), "{q}: bars leaked");
    }
}

/// Security review: a layer reading a bound series only through a channel reference
/// (`y2`, a colour condition operand) is not drawn for an unentitled asker either, and
/// its values reach neither `screen` nor `hit`.
#[test]
fn channel_references_to_a_bound_series_are_gated() {
    let i: SessionInit = serde_json::from_value(json!({
        "document": "fixtures/protocol/docs/bound-mixed.json",
        "clock": 1_760_000_000_000_i64,
        "principals": [
            {"id":"arkin","author":{"kind":"human","name":"arkin"},"role":"owner"},
            {"id":"viewer","author":{"kind":"human","name":"viewer"},"role":"viewer"},
            {"id":"host","author":{"kind":"engine","name":"host","version":"0"},"role":"host"}
        ],
        "data": {
            "feeds": [{"feed":{"symbol":"THYAO","interval":"1d"},"synthetic":{"n":200,"seed":11}}],
            "entitlements": {"host":["*"], "arkin":["THYAO"]},
            "pending": [{"symbol":"ASELS","interval":"1d"}]
        }
    }))
    .unwrap();
    let mut s = Session::new(i, &root()).unwrap();
    let hello = |s: &mut Session, c: &str, p: &str| {
        s.handle_raw(
            c,
            &json!({"type":"hello","protocol":"qp/0","principal":p,"surface":{"kind":"desktop"}})
                .to_string(),
        );
    };
    hello(&mut s, "a", "arkin");
    hello(&mut s, "v", "viewer");
    let view = json!({"id":"mix","x":{"kind":"time"},"y":[{"kind":"linear"}],"layers":[
        {"id":"plain","mark":"line","series":"idx","panel":0,"encoding":{"y":"c"}},
        {"id":"tinted","mark":"line","series":"idx","panel":0,"encoding":{"y":"c",
            "color":{"when":{"field":"c","op":"gt","than":{"series":"px","field":"c"}},"then":"up","else":"down"}}},
        {"id":"banded","mark":"band","series":"idx","panel":0,"encoding":{"y":"c","y2":{"series":"px","field":"c"}}}
    ]});
    let ack = s.handle_raw(
        "a",
        &json!({"type":"command","id":"c1","ops":[{"op":"put_view","view":view}]}).to_string(),
    );
    let ack = serde_json::to_value(&ack[0].msg).unwrap();
    assert_eq!(ack["status"], "applied", "{ack}");
    let one = |s: &mut Session, c: &str, m: Value| {
        serde_json::to_value(&s.handle_raw(c, &m.to_string())[0].msg).unwrap()
    };
    for c in ["a", "v"] {
        let r = one(
            &mut s,
            c,
            json!({"type":"input","id":"i","gesture":{"kind":"look","view":"mix","from_bar":0,"to_bar":39}}),
        );
        assert_eq!(r["type"], "result", "{r}");
    }
    let layers = |r: &Value| r["data"]["screens"][0]["panels"][0]["layers"].clone();
    let r = one(
        &mut s,
        "a",
        json!({"type":"query","id":"q","query":{"kind":"screen"}}),
    );
    let la = layers(&r);
    assert_eq!(la.as_array().unwrap().len(), 3, "{r}");
    assert!(la
        .as_array()
        .unwrap()
        .iter()
        .all(|l| l.get("binding").is_none()));
    let r = one(
        &mut s,
        "v",
        json!({"type":"query","id":"q","query":{"kind":"screen"}}),
    );
    let lv = layers(&r);
    let ids: Vec<(String, Option<String>)> = lv
        .as_array()
        .unwrap()
        .iter()
        .map(|l| {
            (
                l["id"].as_str().unwrap().to_owned(),
                l.get("binding").and_then(Value::as_str).map(str::to_owned),
            )
        })
        .collect();
    assert_eq!(
        ids,
        vec![
            ("plain".to_owned(), None),
            ("tinted".to_owned(), Some("no_entitlement".to_owned())),
            ("banded".to_owned(), Some("no_entitlement".to_owned())),
        ],
        "{r}"
    );
    for l in lv.as_array().unwrap().iter().skip(1) {
        assert_eq!(l["items"], 0);
        assert!(l.get("last").is_none(), "{l}");
    }
    // hit: nothing of the gated layers anywhere in the plot.
    for x in (60..1150).step_by(37) {
        for y in (20..560).step_by(41) {
            let r = one(
                &mut s,
                "v",
                json!({"type":"query","id":"h","query":{"kind":"hit","view":"mix","x":x,"y":y}}),
            );
            assert_eq!(r["type"], "result", "{r}");
            if let Some(l) = r["data"]["hit"]["item"]["layer"].as_str() {
                assert_eq!(l, "plain", "{r}");
            }
        }
    }
}
