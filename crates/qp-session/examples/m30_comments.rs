//! (R5B/M30) Runs the M30 comment set through the authority at each validator strictness
//! and writes one JSON line per draft and strictness: `{id, strictness, decision,
//! checks, told}` — `checks` are the validator's real checks (the host's audit,
//! `Session::comment_audit`), `told` the reason codes the writer received (a restricted
//! writer is told less, §25.4). Deterministic: no model, no network.
//!
//! ```text
//! cargo run -p qp-session --release --example m30_comments -- \
//!   rounds/R5B/experiments/harness > <scratchpad>/m30.jsonl
//! ```

use std::collections::BTreeSet;
use std::io::Write;
use std::path::PathBuf;

use qp_schema::comment::Strictness;
use qp_schema::protocol::SessionInit;
use qp_session::ReplayHost;
use serde_json::{json, Value};

fn main() -> Result<(), String> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let harness = std::env::args().nth(1).map_or_else(
        || root.join("rounds/R5B/experiments/harness"),
        PathBuf::from,
    );
    let set: Value = qp_session::read_json(&harness.join("comments/comments.json"))?;
    let items = set["items"].as_array().ok_or("comments.json: no items")?;
    let base: Value = qp_session::read_json(&harness.join("session/init.r5b.json"))?;
    let out = std::io::stdout();
    let mut out = out.lock();
    for (level, name) in [
        (Strictness::Cites, "cites"),
        (Strictness::CitesNumbers, "cites_numbers"),
        (Strictness::PerSentence, "per_sentence"),
    ] {
        let mut init: SessionInit =
            serde_json::from_value(base.clone()).map_err(|e| format!("init: {e}"))?;
        init.comment_strictness = Some(level);
        let mut host = ReplayHost::new(init, &root)?;
        let mut said: BTreeSet<String> = BTreeSet::new();
        for item in items {
            let who = item["principal"].as_str().ok_or("item: principal")?;
            let conn = format!("c-{who}");
            if said.insert(conn.clone()) {
                hello(&mut host, &conn, who)?;
            }
            let d = &item["draft"];
            let mut op = json!({"op": "add_comment", "about": item["about"],
                "lang": d["lang"], "text": d["text"], "cites": d["cites"]});
            if d["external"].as_array().is_some_and(|e| !e.is_empty()) {
                op["external"] = d["external"].clone();
            }
            let id = item["id"].as_str().ok_or("item: id")?;
            let cmd = json!({"type": "command", "id": id, "ops": [op]});
            let before = host.session().comment_audit().len();
            let reply = first_reply(&mut host, &conn, &cmd)?;
            let audit = host.session().comment_audit();
            let (decision, checks, told) = match reply["type"].as_str() {
                Some("ack") => ("accept", Vec::new(), Vec::new()),
                _ => {
                    let checks: BTreeSet<String> = if audit.len() > before {
                        audit[audit.len() - 1]
                            .reasons
                            .iter()
                            .map(|r| check_name(&r.check))
                            .collect()
                    } else {
                        // Refused before the validator (e.g. an unknown capsule in
                        // `about`): the error code stands for the check.
                        std::iter::once(format!(
                            "error:{}",
                            reply["error"]["code"].as_str().unwrap_or("?")
                        ))
                        .collect()
                    };
                    let told: Vec<String> = reply["error"]["reasons"]
                        .as_array()
                        .map(|rs| {
                            rs.iter()
                                .filter_map(|r| r["code"].as_str().map(str::to_owned))
                                .collect()
                        })
                        .unwrap_or_default();
                    ("reject", checks.into_iter().collect(), told)
                }
            };
            let row = json!({"id": id, "strictness": name, "decision": decision,
                "checks": checks, "told": told});
            writeln!(out, "{row}").map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn check_name(c: &qp_schema::comment::Check) -> String {
    serde_json::to_value(c)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default()
}

fn hello(host: &mut ReplayHost, conn: &str, who: &str) -> Result<(), String> {
    let h = json!({"type":"hello","protocol":"qp/0","principal":who,"surface":{"kind":"headless"}});
    let r = first_reply(host, conn, &h)?;
    if r["type"] != "welcome" {
        return Err(format!("hello {who}: {r}"));
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
