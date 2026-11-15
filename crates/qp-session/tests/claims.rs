//! Claim evidence and the `claims` topic (RP2/T8, `docs/PROTOCOL.md` §19): evidence
//! values are data-plane values and never reach a principal without the feed's data
//! entitlement (§18.3); the meaning (t, maturity, cleanliness, series ids) does.

use std::path::{Path, PathBuf};

use qp_schema::data::DataEvent;
use qp_schema::protocol::SessionInit;
use qp_session::Session;
use serde_json::{json, Value};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The init of transcript 34 with an extra viewer `guest` that has no data entitlement.
fn init() -> (SessionInit, Vec<DataEvent>) {
    let t: Value = serde_json::from_str(
        &std::fs::read_to_string(root().join("fixtures/protocol/conformance/34-consistency.json"))
            .unwrap(),
    )
    .unwrap();
    let mut init = t["init"].clone();
    init["principals"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"guest","author":{"kind":"human","name":"guest"},"role":"viewer"}));
    let events = t["steps"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|s| s.get("data").cloned())
        .map(|d| serde_json::from_value(d).unwrap())
        .collect();
    (serde_json::from_value(init).unwrap(), events)
}

fn msgs(lines: &[qp_schema::protocol::OutboundLine], conn: &str) -> Vec<Value> {
    lines
        .iter()
        .filter(|l| l.conn == conn)
        .map(|l| serde_json::to_value(&l.msg).unwrap())
        .collect()
}

/// Every `evidence` object inside `v`.
fn evidences(v: &Value, out: &mut Vec<Value>) {
    match v {
        Value::Object(m) => {
            for (k, x) in m {
                if k == "evidence" && x.get("maturity").is_some() {
                    out.push(x.clone());
                }
                evidences(x, out);
            }
        }
        Value::Array(a) => a.iter().for_each(|x| evidences(x, out)),
        _ => {}
    }
}

#[test]
fn evidence_values_never_reach_an_unentitled_principal() {
    let (init, events) = init();
    let mut s = Session::new(init, &root()).unwrap();
    let subs = [
        json!({"kind":"claims"}),
        json!({"kind":"condition","condition":{"kind":"threshold","series":"px","op":"gt","value":105.0}}),
        json!({"kind":"finding","finding":"fnd-range"}),
        json!({"kind":"changes"}),
        json!({"kind":"intents"}),
    ];
    for (conn, who) in [("a", "arkin"), ("g", "guest")] {
        let w = s.handle_raw(
            conn,
            &json!({"type":"hello","protocol":"qp/0","principal":who,"surface":{"kind":"desktop"}})
                .to_string(),
        );
        assert_eq!(msgs(&w, conn)[0]["type"], "welcome");
        for (i, topic) in subs.iter().enumerate() {
            // A topic the authority refuses to an unentitled principal is simply not
            // watched (the point is that nothing it does receive carries values).
            let _ = s.handle_raw(
                conn,
                &json!({"type":"subscribe","id":format!("s{i}"),"topic":topic}).to_string(),
            );
        }
    }
    let (mut ga, mut gg) = (Vec::new(), Vec::new());
    for ev in &events {
        let lines = s.data_event(ev).unwrap();
        for m in msgs(&lines, "a") {
            evidences(&m, &mut ga);
        }
        for m in msgs(&lines, "g") {
            evidences(&m, &mut gg);
        }
    }
    for conn in ["a", "g"] {
        let r = s.handle_raw(
            conn,
            &json!({"type":"query","id":"l","query":{"kind":"log"}}).to_string(),
        );
        let m = &msgs(&r, conn)[0];
        assert_eq!(m["type"], "result", "{m}");
        evidences(m, if conn == "a" { &mut ga } else { &mut gg });
    }
    // The entitled owner gets the values …
    assert!(!ga.is_empty());
    assert!(ga
        .iter()
        .any(|e| e["values"][0]["value"].is_number() && e["values"][0]["prev"].is_number()));
    // … the unentitled guest gets the meaning only (it does receive evidence: the
    // invalidation's `change`, the `log` entry, document claims).
    assert!(!gg.is_empty());
    for e in &gg {
        for v in e["values"].as_array().unwrap() {
            assert!(v.get("value").is_none() && v.get("prev").is_none(), "{e}");
            assert!(v["series"].is_string() && v["t"].is_number(), "{e}");
        }
        assert!(e["maturity"].is_string() && e["clean"].is_boolean() && e["t"].is_number());
    }
    // The stored log keeps the values (replay and the entitled readers need them).
    assert!(s
        .log()
        .iter()
        .filter_map(|e| e.evidence.as_ref())
        .any(|ev| ev.values[0].value.is_some()));
}
