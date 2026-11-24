//! A session (or a replay host, for inits with `replay`) driven through the public
//! surface only: `handle_raw`, `data_event`, the log, the documents and `replay`.

use std::path::PathBuf;

use qp_schema::data::DataEvent;
use qp_schema::protocol::{OutboundLine, SessionInit};
use qp_session::{ReplayHost, Session};
use serde_json::{json, Value};

pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The conformance transcript `name` (file stem) as JSON.
pub fn transcript(name: &str) -> Value {
    let p = root().join(format!("fixtures/protocol/conformance/{name}.json"));
    serde_json::from_str(&std::fs::read_to_string(p).expect("transcript")).expect("json")
}

/// The init of transcript `name`, as JSON (inline inits only).
pub fn init_of(name: &str) -> Value {
    transcript(name)["init"].clone()
}

fn lines(v: &[OutboundLine]) -> Vec<Value> {
    v.iter()
        .map(|l| serde_json::to_value(l).expect("line"))
        .collect()
}

enum Target {
    S(Box<Session>),
    R(Box<ReplayHost>),
}

pub struct Rig {
    t: Target,
}

impl Rig {
    /// The `thyao` example init (owner `arkin`, proposer agent `scout`, editor agent
    /// `editor`, `viewer`, `feed`, `host`).
    pub fn thyao() -> Self {
        Self::with(
            qp_session::read_json(&root().join("fixtures/protocol/init/thyao.json")).unwrap(),
        )
    }

    /// An init given as JSON.
    pub fn json(v: Value) -> Self {
        Self::with(serde_json::from_value(v).expect("init"))
    }

    /// A transcript's init.
    pub fn of(name: &str) -> Self {
        Self::json(init_of(name))
    }

    pub fn with(init: SessionInit) -> Self {
        let t = if init.replay.is_some() {
            Target::R(Box::new(
                qp_session::conform::transcript_replay_host(&init, &root()).unwrap(),
            ))
        } else {
            Target::S(Box::new(Session::new(init, &root()).unwrap()))
        };
        Self { t }
    }

    pub fn session(&self) -> &qp_session::Authority {
        match &self.t {
            Target::S(s) => s,
            Target::R(r) => r.session(),
        }
    }

    pub fn session_mut(&mut self) -> &mut qp_session::Authority {
        match &mut self.t {
            Target::S(s) => s,
            Target::R(r) => r.session_mut(),
        }
    }

    pub fn raw(&mut self, conn: &str, line: &str) -> Vec<Value> {
        match &mut self.t {
            Target::S(s) => lines(&s.handle_raw(conn, line)),
            Target::R(r) => lines(&r.handle_raw(conn, line)),
        }
    }

    pub fn send(&mut self, conn: &str, msg: &Value) -> Vec<Value> {
        self.raw(conn, &msg.to_string())
    }

    pub fn data(&mut self, ev: &Value) -> Vec<Value> {
        let ev: DataEvent = serde_json::from_value(ev.clone()).expect("data event");
        match &mut self.t {
            Target::S(s) => lines(&s.data_event(&ev).expect("data event accepted")),
            Target::R(r) => lines(&r.data_event(&ev).expect("data event accepted")),
        }
    }

    /// A data event that may be refused: the lines, or the refusal's text.
    pub fn data_try(&mut self, ev: &DataEvent) -> Result<Vec<Value>, String> {
        match &mut self.t {
            Target::S(s) => s
                .data_event(ev)
                .map(|l| lines(&l))
                .map_err(|e| format!("{e}")),
            Target::R(r) => r
                .data_event(ev)
                .map(|l| lines(&l))
                .map_err(|e| format!("{e}")),
        }
    }

    pub fn hello(&mut self, conn: &str, principal: &str, kind: &str) -> Vec<Value> {
        self.send(
            conn,
            &json!({"type":"hello","protocol":"qp/0","principal":principal,"surface":{"kind":kind}}),
        )
    }

    pub fn cmd(&mut self, conn: &str, id: &str, ops: Value) -> Vec<Value> {
        self.send(conn, &json!({"type":"command","id":id,"ops":ops}))
    }

    pub fn sub(&mut self, conn: &str, id: &str, topic: Value) -> Vec<Value> {
        self.send(conn, &json!({"type":"subscribe","id":id,"topic":topic}))
    }

    pub fn query(&mut self, conn: &str, id: &str, q: Value) -> Vec<Value> {
        self.send(conn, &json!({"type":"query","id":id,"query":q}))
    }

    pub fn log_len(&self) -> usize {
        self.session().log().len()
    }

    /// The last log entry as JSON.
    pub fn last_entry(&self) -> Value {
        serde_json::to_value(self.session().log().last().expect("an entry")).unwrap()
    }

    /// Replay of the log from the initial document, byte-equal to the document.
    pub fn replay_ok(&self) -> bool {
        let s = self.session();
        let r = qp_ops::replay(s.initial_document(), s.log()).expect("replay");
        serde_json::to_vec(&r).unwrap() == serde_json::to_vec(s.document()).unwrap()
    }

    /// `final` line of a scenario: document and log digests, log length, replay.
    pub fn fin(&self) -> (String, String) {
        let s = self.session();
        let d = super::snap::jdigest(&(s.document(), s.log()));
        let sum = format!(
            "log={} doc={} replay={}",
            s.log().len(),
            super::snap::jdigest(s.document()),
            if self.replay_ok() { "equal" } else { "DIFFERS" }
        );
        (d, sum)
    }
}
