//! Generates the R4 replay conformance transcripts 45–46
//! (`fixtures/protocol/conformance/45-*.json`, `docs/PROTOCOL.md` §15, §21).
//!
//! Each scenario is written as messages; the expected outputs are what the replay host
//! (in process, [`qp_session::conform::transcript_replay_host`]) answers, complete
//! (welcome trimmed to its type, `seq` and `epoch`). `qp conform` then checks them in
//! process, over `qp serve` and over MCP. Deterministic: running it twice writes the
//! same bytes.
//!
//! ```text
//! cargo run -p qp-session --example make_replay_transcripts            # check only
//! cargo run -p qp-session --example make_replay_transcripts -- --write
//! ```

use std::path::{Path, PathBuf};

use qp_schema::protocol::{OutboundLine, SessionInit};
use qp_session::Session;
use serde_json::{json, Value};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn principals() -> Value {
    json!([
        {"id":"host","author":{"kind":"engine","name":"host","version":"0"},"role":"host"},
        {"id":"arkin","author":{"kind":"human","name":"Arkın"},"role":"owner"},
        {"id":"scout","author":{"kind":"agent","name":"scout"},"role":"proposer"},
        {"id":"guest","author":{"kind":"human","name":"Guest"},"role":"viewer"}
    ])
}

/// Two synthetic instruments, 70 trading days, 50 of them history; days moved by their
/// hourly bars; every feed bound and watched by the capsule engine.
fn replay_init() -> Value {
    json!({
        "document": {"schema_version":"0","id":"doc-replay","title":"Yeniden oynatma",
                     "display_tz":"Europe/Istanbul"},
        "principals": principals(),
        "clock": 0,
        "epoch": 0,
        "replay": {"instruments": 2, "days": 70, "history": 50, "fine": "1h",
                   "speed": 8.0, "bind_all": true, "capsules_all": true}
    })
}

fn hello(conn: &str, who: &str) -> (String, Value, Option<&'static str>) {
    (
        conn.to_owned(),
        json!({"type":"hello","protocol":"qp/0","principal":who,"surface":{"kind":"phone"}}),
        None,
    )
}

fn msg(conn: &str, m: Value, note: &'static str) -> (String, Value, Option<&'static str>) {
    (conn.to_owned(), m, (!note.is_empty()).then_some(note))
}

fn ctl(
    conn: &str,
    id: &str,
    c: Value,
    note: &'static str,
) -> (String, Value, Option<&'static str>) {
    msg(conn, json!({"type":"replay","id":id,"control":c}), note)
}

fn trim(v: Value) -> Value {
    if v["type"] == "welcome" {
        return json!({"type":"welcome","seq": v["seq"], "epoch": v["epoch"]});
    }
    v
}

trait Host {
    fn send(&mut self, conn: &str, text: &str) -> Vec<OutboundLine>;
    fn log_len(&self) -> usize;
}

impl Host for qp_session::ReplayHost {
    fn send(&mut self, conn: &str, text: &str) -> Vec<OutboundLine> {
        self.handle_raw(conn, text)
    }
    fn log_len(&self) -> usize {
        self.session().log().len()
    }
}

impl Host for Session {
    fn send(&mut self, conn: &str, text: &str) -> Vec<OutboundLine> {
        self.handle_raw(conn, text)
    }
    fn log_len(&self) -> usize {
        self.log().len()
    }
}

fn run(
    name: &str,
    description: &str,
    covers: &[&str],
    init_v: Value,
    steps: Vec<(String, Value, Option<&'static str>)>,
) -> Value {
    let init: SessionInit = serde_json::from_value(init_v.clone()).expect("init");
    let mut h: Box<dyn Host> = if init.replay.is_some() {
        Box::new(qp_session::conform::transcript_replay_host(&init, &root()).expect("host"))
    } else {
        Box::new(Session::new(init, &root()).expect("session"))
    };
    let mut out_steps = Vec::new();
    for (conn, m, note) in steps {
        let lines = h.send(&conn, &m.to_string());
        let mut step = json!({"send": {"conn": conn, "msg": m}});
        if let Some(n) = note {
            step["note"] = json!(n);
        }
        step["expect"] = Value::Array(
            lines
                .iter()
                .map(
                    |l| json!({"conn": l.conn, "msg": trim(serde_json::to_value(&l.msg).unwrap())}),
                )
                .collect(),
        );
        out_steps.push(step);
    }
    json!({
        "name": name,
        "description": description,
        "covers": covers,
        "init": init_v,
        "steps": out_steps,
        "final": {"log_len": h.log_len()}
    })
}

fn t45() -> Value {
    run(
        "45-replay-control",
        "R4 (§21): a replay host over the synthetic market (two instruments, daily bars moved inside the day by their hourly bars, the capsule engine on both). The owner drives the timeline with `replay` messages: status, one step (the day opens), a day of steps (the notifications of the steps follow the `result`; a closed bar goes through the capsule engine), pause / speed / resume / pause (the content never depends on the wall clock), seek forward by date and by time. A viewer may read the status but not move the timeline; invalid controls are `invalid` with a path, an unknown control a schema error. Seeking back rebuilds the authority (epoch 1): every connection is closed and says hello again, and the document is the one the steps up to that position produce.",
        &["replay", "replay-status", "replay-step", "replay-speed", "replay-pause", "replay-resume", "replay-seek", "replay-reset", "forbidden"],
        replay_init(),
        vec![
            hello("a", "arkin"),
            msg("a", json!({"type":"subscribe","id":"caps","topic":{"kind":"capsules","all":true}}), "Every capsule transition, for every reader preference."),
            hello("g", "guest"),
            ctl("a", "r1", json!({"kind":"status"}), "Paused at the start: virtual time = the last history bar; synthetic data, said so."),
            ctl("a", "r2", json!({"kind":"step"}), "One step: the first replayed day opens (a `bar` event per instrument at the session open)."),
            ctl("a", "r3", json!({"kind":"step","n":12}), "The rest of that day and the next open: the first day's bar becomes final and the engine writes its entries."),
            ctl("a", "r4", json!({"kind":"speed","steps_per_s":20.0}), ""),
            ctl("a", "r5", json!({"kind":"resume"}), "Running: steps follow the wall clock at 20 steps/s; no wall time passes in a transcript."),
            ctl("a", "r6", json!({"kind":"pause"}), ""),
            ctl("a", "r7", json!({"kind":"seek","date":"2023-03-20"}), "Forward to the start of a day (nothing of that day taken): the steps in between are taken; their notifications follow."),
            ctl("a", "r8", json!({"kind":"seek","t":1_679_400_000_000_i64}), "Forward to a virtual time."),
            ctl("g", "r9", json!({"kind":"status"}), "A viewer reads the status."),
            ctl("g", "r10", json!({"kind":"step"}), "A viewer may not move the shared timeline."),
            ctl("a", "r11", json!({"kind":"speed","steps_per_s":0.0}), ""),
            ctl("a", "r12", json!({"kind":"seek","t":1,"step":2}), "Exactly one of `t`, `date`, `step`."),
            ctl("a", "r13", json!({"kind":"rewind"}), "Not a control: the authority's schema error."),
            ctl("a", "r14", json!({"kind":"seek","step":5}), "Back: the authority is rebuilt and advanced to step 5 silently (`reset`, epoch 1)."),
            msg("a", json!({"type":"query","id":"q1","query":{"kind":"ledger"}}), "The connection was closed by the reset."),
            hello("a", "arkin"),
            ctl("a", "r15", json!({"kind":"status"}), ""),
        ],
    )
}

fn t46() -> Value {
    run(
        "46-replay-unsupported",
        "R4 (§21): an authority without `init.replay` has no replay source; a `replay` message is `unsupported`, a malformed one a schema error.",
        &["replay", "unsupported"],
        json!({
            "document": "fixtures/protocol/docs/thyao.json",
            "principals": principals(),
            "clock": 1_700_000_000_000_i64,
            "epoch": 0
        }),
        vec![
            hello("a", "arkin"),
            ctl("a", "r1", json!({"kind":"status"}), ""),
            ctl("a", "r2", json!({"kind":"step","n":"many"}), ""),
        ],
    )
}

fn pretty(v: &Value) -> String {
    let mut s = serde_json::to_string_pretty(v).expect("json");
    s.push('\n');
    s
}

fn main() {
    let write = std::env::args().any(|a| a == "--write");
    let dir = root().join("fixtures/protocol/conformance");
    let mut stale = 0;
    for t in [t45(), t46()] {
        let name = t["name"].as_str().unwrap().to_owned();
        let path = dir.join(format!("{name}.json"));
        let text = pretty(&t);
        let same = std::fs::read_to_string(&path).is_ok_and(|x| x == text);
        if write && !same {
            std::fs::write(&path, &text).expect("write");
            println!("wrote {}", path.display());
        } else if !same {
            stale += 1;
            println!("differs: {}", path.display());
        } else {
            println!("same: {}", path.display());
        }
    }
    if stale > 0 && !write {
        std::process::exit(1);
    }
}
