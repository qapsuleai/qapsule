//! (R5B/R2) The Nöbetçi reference agent's window: the replay steps **as the host**.
//!
//! An agent principal cannot move the replay over MCP (`replay` controls need the role
//! `host` or `owner`; `claude` is a `proposer`), and `qp mcp serve` has no host control
//! besides `--running`/`--seek` at start. So the driver (`agents/run.py window`) takes
//! the watches the agent set over MCP (from the server's `--dump`), and this example
//! replays them deterministically: the agent's marked connection (`--principal`, default
//! `claude`) sends the same `add_watch` operations, subscribes to `intents` and
//! `capsules {all: true}`; a second marked connection subscribes to `capsules` (the
//! person's feed); the host steps `n` days (`ReplayHost::host_control`). Every
//! notification is written as `{day, conn, event}` (`conn`: `intents`, `capsules`,
//! `feed`), one JSON per line, after a `{day, conn: "replay", event: {t, step}}` row
//! with the replay's time after that day's step.
//!
//! Deterministic: no model, no network, no clock; two runs give byte-equal output.
//!
//! ```text
//! cargo run -p qp-session --release --example agent_window -- \
//!   <init.json> <watches.json> <steps> <out.jsonl> [--principal ID] [--root DIR]
//! ```
//!
//! `watches.json`: `[{"text", "match", "for"?, "expires_t"?, "max_matches"?}, …]` — the
//! `add_watch` operation's fields.

use std::io::Write;
use std::path::PathBuf;

use qp_schema::protocol::{ReplayControl, SessionInit};
use qp_session::ReplayHost;
use serde_json::{json, Value};

const C: &str = "agent";
const F: &str = "agent-feed";

fn main() -> Result<(), String> {
    let mut pos: Vec<String> = Vec::new();
    let mut principal = "claude".to_owned();
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--principal" => principal = args.next().ok_or("--principal ID")?,
            "--root" => root = PathBuf::from(args.next().ok_or("--root DIR")?),
            _ => pos.push(a),
        }
    }
    let [init_p, watches_p, steps, out_p] = pos.as_slice() else {
        return Err(
            "usage: agent_window <init.json> <watches.json> <steps> <out.jsonl> \
             [--principal ID] [--root DIR]"
                .into(),
        );
    };
    let steps: u32 = steps.parse().map_err(|e| format!("steps: {e}"))?;
    let init: SessionInit = qp_session::read_json(&PathBuf::from(init_p))?;
    let watches: Value = qp_session::read_json(&PathBuf::from(watches_p))?;
    let watches = watches.as_array().ok_or("watches.json: an array")?;
    let mut host = ReplayHost::new(init, &root)?;

    for conn in [C, F] {
        host.session_mut().mark_agent_connection(conn);
        let r = first_reply(
            &mut host,
            conn,
            &json!({"type":"hello","protocol":"qp/0","principal":principal,
                    "surface":{"kind":"headless"}}),
        )?;
        if r["type"] != "welcome" {
            return Err(format!("hello {conn}: {r}"));
        }
    }
    for (i, w) in watches.iter().enumerate() {
        let mut op = json!({"op": "add_watch", "text": w["text"], "match": w["match"]});
        for k in ["for", "expires_t", "max_matches"] {
            if !w[k].is_null() {
                op[k] = w[k].clone();
            }
        }
        let cmd = json!({"type":"command","id":format!("w{}", i + 1),"ops":[op]});
        let r = first_reply(&mut host, C, &cmd)?;
        if r["type"] != "ack" {
            return Err(format!("add_watch {i}: {r}"));
        }
    }
    for (conn, id, topic) in [
        (C, "s-int", json!({"kind":"intents"})),
        (C, "s-cap", json!({"kind":"capsules","all":true})),
        (F, "s-feed", json!({"kind":"capsules"})),
    ] {
        let r = first_reply(
            &mut host,
            conn,
            &json!({"type":"subscribe","id":id,"topic":topic}),
        )?;
        if r["type"] == "error" {
            return Err(format!("subscribe {id}: {r}"));
        }
    }

    let mut out = std::fs::File::create(out_p).map_err(|e| format!("{out_p}: {e}"))?;
    for day in 1..=steps {
        let (state, lines) = host
            .host_control(&ReplayControl::Step { n: Some(1) })
            .map_err(|e| format!("step {day}: {e:?}"))?;
        // The replay's time after the step: where a later check of this day seeks to.
        let row =
            json!({"day": day, "conn": "replay", "event": {"t": state.t, "step": state.step}});
        writeln!(out, "{row}").map_err(|e| e.to_string())?;
        for l in lines {
            let m = serde_json::to_value(&l.msg).map_err(|e| e.to_string())?;
            if m["type"] != "notify" {
                continue;
            }
            let ev = &m["event"];
            let conn = match (l.conn.as_str(), ev["kind"].as_str()) {
                (C, Some("intent_matched")) => "intents",
                (C, Some("capsule")) => "capsules",
                (F, Some("capsule")) => "feed",
                _ => continue,
            };
            let row = json!({"day": day, "conn": conn, "event": ev});
            writeln!(out, "{row}").map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn first_reply(host: &mut ReplayHost, conn: &str, msg: &Value) -> Result<Value, String> {
    host.handle_raw(conn, &msg.to_string())
        .into_iter()
        .find(|l| l.conn == conn)
        .map(|l| serde_json::to_value(&l.msg).unwrap_or(Value::Null))
        .ok_or_else(|| format!("no reply on {conn}"))
}
