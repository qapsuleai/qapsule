//! The extension test's rig: an authority built with a list of the test's own, messages.

use std::collections::VecDeque;
use std::path::PathBuf;

use qp_authority::Replica;
use qp_schema::protocol::{OutboundLine, SessionInit};
use qp_session::{Authority, Subsystems, STANDARD};
use qp_subsystem::Subsystem;
use serde_json::{json, Value};

pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn init() -> SessionInit {
    qp_session::read_json(&root().join("fixtures/protocol/init/thyao.json")).unwrap()
}

/// `STANDARD` followed by `extra` (a list lives as long as the test process).
pub fn list(extra: &[&'static dyn Subsystem]) -> Subsystems {
    let mut v: Vec<&'static dyn Subsystem> = STANDARD.list().to_vec();
    v.extend_from_slice(extra);
    Subsystems::new(Box::leak(v.into_boxed_slice()))
}

/// An authority built with `subs` (the composition's public API: `Authority` + `Subsystems`).
pub fn with_list(subs: Subsystems) -> Authority {
    Authority::new(init(), &root(), subs).unwrap()
}

pub fn send(s: &mut Authority, conn: &str, msg: Value) -> Vec<Value> {
    s.handle_raw(conn, &msg.to_string())
        .iter()
        .map(|l| serde_json::to_value(l).unwrap())
        .collect()
}

pub fn hello(s: &mut Authority, conn: &str, principal: &str) {
    let r = send(
        s,
        conn,
        json!({"type":"hello","protocol":"qp/0","principal":principal,"surface":{"kind":"headless"}}),
    );
    assert_eq!(r[0]["msg"]["type"], "welcome", "{r:?}");
}

/// Delivers the authority's lines for connection `r` to the replica, and the replica's
/// requests back, until both are quiet.
pub fn deliver(a: &mut Authority, r: &mut Replica, lines: Vec<OutboundLine>) {
    let mut q: VecDeque<OutboundLine> = lines.into();
    while let Some(l) = q.pop_front() {
        if l.conn != "r" {
            continue;
        }
        r.on_line(&serde_json::to_string(&l).unwrap()).unwrap();
        for m in r.take_outbox() {
            q.extend(a.handle_raw("r", &m.to_string()));
        }
    }
}
