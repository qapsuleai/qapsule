//! The program tests' rig: the `thyao` session with an agent, programs as text and IR.

use std::path::PathBuf;

use qp_schema::protocol::SessionInit;
use qp_session::Session;
use serde_json::{json, Value};

pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A program with a parameter, a series and an event output (PR §12.5 style).
pub const SMA_TEXT: &str = "param len int = 5 in 2..50\nm = sma(close, len)\nseries avg = m\nevent up = close > m and defined(m)\n";

/// A program with a signal output (try only).
pub const SIGNAL_TEXT: &str = "r = rsi(close, 14)\nentry = cross_below(r, 30)\nsignal dip = entry side al target 3% stop 1.5% max_bars 24 source \"dip\" \"1\"\n";

pub struct H {
    pub s: Session,
}

impl H {
    /// `thyao` with `arkin` (owner, human) on `a`, `scout` (agent, proposer, working for
    /// arkin) on `s`,
    /// `viewer` on `v`, the host on `h`; the R5B agent policy on.
    pub fn new() -> Self {
        let mut init: SessionInit =
            qp_session::read_json(&root().join("fixtures/protocol/init/thyao.json")).unwrap();
        // (R5B, §25.3) The agent write rule holds for agent-origin commands.
        init.agent_policy = Some("r5b".to_owned());
        // `scout` works for `arkin`: its accepted programs are charged to arkin.
        for p in &mut init.principals {
            if p.id == "scout" {
                p.on_behalf = Some("arkin".to_owned());
            }
        }
        Self::with(
            init,
            &[
                ("a", "arkin"),
                ("s", "scout"),
                ("v", "viewer"),
                ("h", "host"),
            ],
        )
    }

    pub fn with(init: SessionInit, conns: &[(&str, &str)]) -> Self {
        let mut h = Self {
            s: Session::new(init, &root()).unwrap(),
        };
        for (c, p) in conns {
            let w = h.one(
                c,
                json!({"type":"hello","protocol":"qp/0","principal":p,"surface":{"kind":"headless"}}),
            );
            assert_eq!(w["type"], "welcome", "{w}");
        }
        h
    }

    pub fn one(&mut self, conn: &str, msg: Value) -> Value {
        let out = self.s.handle_raw(conn, &msg.to_string());
        assert!(!out.is_empty(), "no reply");
        serde_json::to_value(&out[0].msg).unwrap()
    }

    /// The host moves the clock to `now`.
    pub fn clock(&mut self, now: i64) {
        let out = self
            .s
            .handle_raw("h", &json!({"type":"clock","now":now}).to_string());
        assert!(out.is_empty(), "{out:?}");
    }

    pub fn cmd(&mut self, conn: &str, ops: Value) -> Value {
        self.one(conn, json!({"type":"command","id":"c","ops":ops}))
    }

    pub fn cmd_mode(&mut self, conn: &str, mode: &str, ops: Value) -> Value {
        self.one(
            conn,
            json!({"type":"command","id":"c","mode":mode,"ops":ops}),
        )
    }

    pub fn query(&mut self, conn: &str, q: Value) -> Value {
        self.one(conn, json!({"type":"query","id":"q","query":q}))
    }

    /// The IR of `text` (through `check_program`).
    pub fn ir(&mut self, text: &str) -> Value {
        let r = self.query("a", json!({"kind":"check_program","source":text}));
        assert_eq!(r["data"]["ok"], true, "{r}");
        r["data"]["ir"].clone()
    }

    /// `add_program` of `text` as instance `id` over `px` by `arkin`, shown in `main`.
    pub fn add(&mut self, id: &str, text: &str) -> Value {
        let ir = self.ir(text);
        self.cmd(
            "a",
            json!([{"op":"add_program","id":id,"program":ir,"inputs":["px"],
                    "show":{"view":"main"}}]),
        )
    }

    pub fn series(&self, id: &str) -> Value {
        let d = serde_json::to_value(self.s.document()).unwrap();
        d["series"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["id"] == id)
            .cloned()
            .unwrap_or(Value::Null)
    }

    pub fn layers(&self) -> Vec<Value> {
        let d = serde_json::to_value(self.s.document()).unwrap();
        d["views"][0]["layers"].as_array().unwrap().clone()
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

pub fn reason(v: &Value) -> &str {
    v["error"]["reason"].as_str().unwrap_or("<none>")
}
