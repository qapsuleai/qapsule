//! R4/T2 (§21): the replay host — determinism, controls, permissions, backward seek.

use std::path::{Path, PathBuf};

use qp_schema::protocol::{OutboundLine, ReplayInit};
use qp_session::{demo_init, ReplayHost};
use serde_json::{json, Value};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn host(r: ReplayInit) -> ReplayHost {
    // A directory without data: local falls back to synthetic.
    let none = std::env::temp_dir().join("qp-replay-test-no-local-data");
    ReplayHost::with_local_dir(demo_init(r), &root(), &none).unwrap()
}

fn small() -> ReplayInit {
    ReplayInit {
        instruments: Some(4),
        days: Some(90),
        history: Some(60),
        fine: Some("1h".to_owned()),
        ..ReplayInit::default()
    }
}

fn send(h: &mut ReplayHost, conn: &str, msg: &Value) -> Vec<OutboundLine> {
    h.handle_raw(conn, &msg.to_string())
}

fn text(lines: &[OutboundLine]) -> String {
    lines
        .iter()
        .map(|l| serde_json::to_string(l).unwrap() + "\n")
        .collect()
}

fn hello(h: &mut ReplayHost, conn: &str, who: &str) -> String {
    let mut s = text(&send(
        h,
        conn,
        &json!({"type":"hello","protocol":"qp/0","principal":who,"surface":{"kind":"phone"}}),
    ));
    for (id, topic) in [
        ("c", json!({"kind":"capsules","all":true})),
        ("ch", json!({"kind":"changes"})),
    ] {
        s += &text(&send(
            h,
            conn,
            &json!({"type":"subscribe","id":id,"topic":topic}),
        ));
    }
    s
}

fn ctl(h: &mut ReplayHost, conn: &str, id: &str, control: Value) -> Vec<OutboundLine> {
    send(h, conn, &json!({"type":"replay","id":id,"control":control}))
}

fn script(h: &mut ReplayHost) -> String {
    let mut s = hello(h, "a", "arkin");
    s += &text(&ctl(h, "a", "s1", json!({"kind":"step","n":25})));
    s += &text(&ctl(
        h,
        "a",
        "sp",
        json!({"kind":"speed","steps_per_s":50.0}),
    ));
    s += &text(&ctl(h, "a", "r", json!({"kind":"resume"})));
    // Wall clock: the same steps whatever the wall times are.
    for w in [0_u64, 37, 211, 1000, 1001, 2500] {
        s += &text(&h.pump(w));
    }
    s += &text(&ctl(h, "a", "p", json!({"kind":"pause"})));
    s += &text(&ctl(h, "a", "k", json!({"kind":"seek","step":200})));
    s += &text(&ctl(h, "a", "st", json!({"kind":"status"})));
    s
}

#[test]
fn two_runs_are_byte_identical_and_without_host_notices() {
    let mut a = host(small());
    let mut b = host(small());
    let ta = script(&mut a);
    let tb = script(&mut b);
    assert_eq!(ta, tb);
    assert!(a.notices().is_empty(), "{:?}", a.notices());
    let st = a.state();
    assert_eq!(st.step, 200);
    assert!(st.synthetic && st.paused);
    assert_eq!(st.days.fine, 4 * 30);
    // The recording: (step, control) pairs.
    let at: Vec<u64> = a.controls().iter().map(|c| c.0).collect();
    assert_eq!(at, vec![0, 25, 25, 25 + 125, 25 + 125]);
    assert_eq!(a.session().document(), b.session().document());
}

#[test]
fn pump_steps_follow_speed_not_wall_jitter() {
    let mut a = host(small());
    let _ = ctl_host(&mut a, json!({"kind":"speed","steps_per_s":10.0}));
    let _ = ctl_host(&mut a, json!({"kind":"resume"}));
    assert!(a.pump(5_000).is_empty()); // anchor only
    let _ = a.pump(5_350);
    assert_eq!(a.state().step, 3);
    assert_eq!(a.next_due_in(5_350), Some(50));
    let _ = a.pump(6_000);
    assert_eq!(a.state().step, 10);
}

fn ctl_host(h: &mut ReplayHost, c: Value) -> Value {
    let c = serde_json::from_value(c).unwrap();
    serde_json::to_value(h.host_control(&c).unwrap().0).unwrap()
}

#[test]
fn backward_seek_rebuilds_the_same_authority_state() {
    let mut a = host(small());
    let _ = ctl_host(&mut a, json!({"kind":"step","n":150}));
    let doc150 = serde_json::to_string(a.session().document()).unwrap();
    let log150 = serde_json::to_string(a.session().log()).unwrap();
    let _ = ctl_host(&mut a, json!({"kind":"step","n":100}));
    let st = ctl_host(&mut a, json!({"kind":"seek","step":150}));
    assert_eq!(st["reset"], true);
    assert_eq!(st["epoch"], 1);
    assert_eq!(
        serde_json::to_string(a.session().document()).unwrap(),
        doc150
    );
    assert_eq!(serde_json::to_string(a.session().log()).unwrap(), log150);
    // Every connection is closed: a message needs a new hello.
    let out = send(
        &mut a,
        "a",
        &json!({"type":"query","id":"q","query":{"kind":"ledger"}}),
    );
    assert_eq!(
        serde_json::to_value(&out[0].msg).unwrap()["error"]["code"],
        "handshake"
    );
}

#[test]
fn seek_by_date_and_time() {
    let mut a = host(small());
    let from = a.state().from_t.unwrap();
    let st = ctl_host(&mut a, json!({"kind":"seek","t":from}));
    assert_eq!(st["step"], 1);
    // A date: nothing of that day is taken; the next step opens it.
    let st = ctl_host(&mut a, json!({"kind":"seek","date":"2023-04-20"}));
    let t = st["t"].as_i64().unwrap();
    let start = qp_source::date_start("2023-04-20").unwrap();
    assert!(t < start);
    let st = ctl_host(&mut a, json!({"kind":"step"}));
    assert!(st["t"].as_i64().unwrap() >= start);
}

#[test]
fn permissions_and_errors() {
    let mut a = host(small());
    hello(&mut a, "g", "guest");
    hello(&mut a, "s", "scout");
    hello(&mut a, "h", "host");
    let v = |l: &[OutboundLine]| serde_json::to_value(&l[0].msg).unwrap();
    // Readers may ask the status; only host and owner control the timeline.
    assert_eq!(
        v(&ctl(&mut a, "g", "1", json!({"kind":"status"})))["type"],
        "result"
    );
    let e = v(&ctl(&mut a, "g", "2", json!({"kind":"step"})));
    assert_eq!(e["error"]["code"], "forbidden");
    let e = v(&ctl(&mut a, "s", "3", json!({"kind":"pause"})));
    assert_eq!(e["error"]["code"], "forbidden");
    assert_eq!(
        v(&ctl(&mut a, "h", "4", json!({"kind":"step"})))["type"],
        "result"
    );
    let e = v(&ctl(
        &mut a,
        "h",
        "5",
        json!({"kind":"speed","steps_per_s":0.0}),
    ));
    assert_eq!(e["error"]["code"], "invalid");
    assert_eq!(e["error"]["path"], "/control/steps_per_s");
    let e = v(&ctl(
        &mut a,
        "h",
        "6",
        json!({"kind":"seek","t":1,"step":2}),
    ));
    assert_eq!(e["error"]["path"], "/control");
    let e = v(&ctl(&mut a, "h", "7", json!({"kind":"jump"})));
    assert_eq!(e["error"]["code"], "schema");
    // Before hello: the authority's handshake error.
    let e = v(&ctl(&mut a, "x", "8", json!({"kind":"status"})));
    assert_eq!(e["error"]["code"], "handshake");
}

#[test]
fn a_bare_authority_answers_unsupported() {
    let init: qp_schema::protocol::SessionInit = serde_json::from_value(json!({
        "document":"fixtures/protocol/docs/thyao.json",
        "principals":[{"id":"arkin","author":{"kind":"human","name":"arkin"},"role":"owner"}],
        "clock": 0
    }))
    .unwrap();
    let mut s = qp_session::Session::new(init, &root()).unwrap();
    s.handle_raw(
        "a",
        &json!({"type":"hello","protocol":"qp/0","principal":"arkin","surface":{"kind":"phone"}})
            .to_string(),
    );
    let out = s.handle_raw(
        "a",
        &json!({"type":"replay","id":"r","control":{"kind":"status"}}).to_string(),
    );
    assert_eq!(
        serde_json::to_value(&out[0].msg).unwrap()["error"]["code"],
        "unsupported"
    );
}

#[test]
fn local_without_data_says_synthetic() {
    let a = host(ReplayInit {
        source: qp_schema::protocol::ReplaySourceKind::Local,
        ..small()
    });
    let st = a.state();
    assert!(st.synthetic);
    assert!(st.note.is_some());
    assert_eq!(st.instruments, 4);
}
