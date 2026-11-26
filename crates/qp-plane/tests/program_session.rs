//! A program event series under a live session (PS §1.2, D1): the existing `threshold`
//! condition `ge 1` fires when the event bar closes (on the next bar's first data), at the
//! event bar, with final evidence; consecutive event bars give one notification.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use std::path::{Path, PathBuf};

use qp_schema::data::DataEvent;
use qp_schema::protocol::SessionInit;
use qp_session::Session;
use serde_json::{json, Value};
use support::{program_series, ABOVE};

const DAY: i64 = 86_400_000;
const T0: i64 = 1_759_708_800_000;
const CLOCK: i64 = 1_760_000_000_000;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn bar(k: i64, c: f64) -> Value {
    json!({"t": T0 + k * DAY, "o": c, "h": c + 0.5, "l": c - 0.5, "c": c, "v": 1000.0})
}

fn init() -> SessionInit {
    let mut p = program_series("p1", ABOVE, "up", &json!({}), CLOCK);
    p["interval"] = json!("1d");
    let doc = json!({
        "schema_version": "0", "id": "doc-prog",
        "series": [
            {"id": "px", "instrument": {"symbol": "THYAO"}, "interval": "1d",
             "source": {"kind": "bind", "symbol": "THYAO", "interval": "1d", "window": {"last": 10}}},
            p
        ]
    });
    serde_json::from_value(json!({
        "document": doc, "clock": CLOCK, "epoch": 0,
        "principals": [
            {"id": "arkin", "author": {"kind": "human", "name": "arkin"}, "role": "owner"},
            {"id": "host", "author": {"kind": "engine", "name": "host", "version": "0"}, "role": "host"}
        ],
        "data": {"feeds": [{"feed": {"symbol": "THYAO", "interval": "1d"},
            "bars": (0..5).map(|k| bar(k, 100.0 + k as f64)).collect::<Vec<_>>()}],
            "entitlements": {"host": ["*"], "arkin": ["*"]}}
    }))
    .unwrap()
}

fn send(s: &mut Session, msg: &Value) -> Vec<Value> {
    s.handle_raw("a", &msg.to_string())
        .iter()
        .filter(|l| l.conn == "a")
        .map(|l| serde_json::to_value(&l.msg).unwrap())
        .collect()
}

fn data(s: &mut Session, seq: u64, change: Value) -> Vec<Value> {
    let ev: DataEvent = serde_json::from_value(
        json!({"feed": {"symbol": "THYAO", "interval": "1d"}, "seq": seq, "change": change}),
    )
    .unwrap();
    s.data_event(&ev)
        .unwrap()
        .iter()
        .filter(|l| l.conn == "a")
        .map(|l| serde_json::to_value(&l.msg).unwrap())
        .collect()
}

/// The `condition_met` events of subscription `c` in `msgs`.
fn met(msgs: &[Value]) -> Vec<Value> {
    msgs.iter()
        .filter(|m| m["sub"] == "c" && m["event"]["kind"] == "condition_met")
        .map(|m| m["event"].clone())
        .collect()
}

#[test]
fn threshold_fires_once_on_the_closed_event_bar_with_final_evidence() {
    let mut s = Session::new(init(), &root()).unwrap();
    let w = send(
        &mut s,
        &json!({"type":"hello","protocol":"qp/0","principal":"arkin","surface":{"kind":"desktop"}}),
    );
    assert_eq!(w[0]["type"], "welcome", "{w:?}");
    let cond = json!({"kind":"threshold","series":"p1.up","op":"ge","value":1.0});
    for (id, topic) in [
        ("k", json!({"kind":"claims"})),
        ("c", json!({"kind":"condition","condition":cond})),
    ] {
        let a = send(&mut s, &json!({"type":"subscribe","id":id,"topic":topic}));
        assert_eq!(a[0]["status"], "subscribed", "{a:?}");
    }
    // The live bar t4 crosses 105: an event series has no value on the live bar.
    let m = data(
        &mut s,
        1,
        json!({"kind":"patch","t":T0 + 4 * DAY,"fields":{"c":106.0,"h":106.5}}),
    );
    assert!(met(&m).is_empty(), "no condition on the live bar: {m:?}");
    // The next bar closes t4: the condition is met at the event bar, final at once.
    let m = data(&mut s, 2, json!({"kind":"bar","bar":bar(5, 106.5)}));
    let e = met(&m);
    assert_eq!(e.len(), 1, "{m:?}");
    assert_eq!(e[0]["bar"], 4);
    assert_eq!(e[0]["t"], T0 + 4 * DAY);
    assert_eq!(e[0]["evidence"]["maturity"], "final", "{e:?}");
    assert_eq!(e[0]["evidence"]["values"][0]["value"], 1.0);
    assert_eq!(
        e[0]["evidence"]["feeds"],
        json!([{"symbol":"THYAO","interval":"1d"}])
    );
    // t5 is an event bar too (consecutive): recorded in the series, no second notification.
    let m = data(&mut s, 3, json!({"kind":"bar","bar":bar(6, 99.0)}));
    assert!(met(&m).is_empty(), "coalesced: {m:?}");
    let m = data(&mut s, 4, json!({"kind":"bar","bar":bar(7, 99.5)}));
    assert!(met(&m).is_empty(), "{m:?}");
    // The series holds every event bar (K13: full length, `null` on the live bar t7).
    let r = send(
        &mut s,
        &json!({"type":"query","id":"q","query":{"kind":"study","series":"p1.up"}}),
    );
    assert_eq!(r[0]["type"], "result", "{}", r[0]);
    let v = &r[0]["data"]["values"]["outputs"]["up"];
    assert_eq!(
        v,
        &json!([0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0, null]),
        "{}",
        r[0]
    );
}
