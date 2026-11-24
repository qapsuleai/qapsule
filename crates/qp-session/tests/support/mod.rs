#![allow(dead_code)]

use std::path::PathBuf;

use qp_schema::protocol::SessionInit;
use qp_session::Session;
use serde_json::{json, Value};

pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn init() -> SessionInit {
    qp_session::read_json(&root().join("fixtures/protocol/init/thyao.json")).unwrap()
}

pub struct H {
    pub s: Session,
}

impl H {
    pub fn new() -> Self {
        Self::with(init())
    }

    pub fn with(init: SessionInit) -> Self {
        Self {
            s: Session::new(init, &root()).unwrap(),
        }
    }

    /// Sends a message; returns the output lines as JSON.
    pub fn send(&mut self, conn: &str, msg: Value) -> Vec<Value> {
        self.raw(conn, &msg.to_string())
    }

    pub fn raw(&mut self, conn: &str, line: &str) -> Vec<Value> {
        self.s
            .handle_raw(conn, line)
            .iter()
            .map(|l| serde_json::to_value(l).unwrap())
            .collect()
    }

    /// Sends and returns the single reply's `msg`.
    pub fn one(&mut self, conn: &str, msg: Value) -> Value {
        let out = self.send(conn, msg);
        assert!(!out.is_empty(), "no reply");
        out[0]["msg"].clone()
    }

    pub fn hello(&mut self, conn: &str, principal: &str, kind: &str) -> Value {
        self.one(
            conn,
            json!({"type":"hello","protocol":"qp/0","principal":principal,"surface":{"kind":kind}}),
        )
    }

    pub fn cmd(&mut self, conn: &str, id: &str, ops: Value) -> Value {
        self.one(conn, json!({"type":"command","id":id,"ops":ops}))
    }

    pub fn query(&mut self, conn: &str, q: Value) -> Value {
        self.one(conn, json!({"type":"query","id":"q","query":q}))
    }

    /// Replay of the log equals the session's document byte for byte.
    pub fn assert_replay(&self) {
        let r = qp_ops::replay(self.s.initial_document(), self.s.log()).unwrap();
        assert_eq!(
            serde_json::to_vec(&r).unwrap(),
            serde_json::to_vec(self.s.document()).unwrap()
        );
    }
}

pub fn code(v: &Value) -> &str {
    v["error"]["code"].as_str().unwrap_or("<none>")
}
