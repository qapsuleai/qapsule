//! One op case: a fresh `thyao` session (owner `a`, a `viewer` on `v` subscribed to
//! `changes`, proposer agent `s`), optional preparation commands, then the command under
//! test. A case that logs locks the reply, every notification and the whole new log
//! entry (changes, inverse, summary); one that does not locks the full error.

use serde_json::{json, Value};

use super::rig::Rig;
use super::snap::{jdigest, order, Book};

/// The standard rig: `a` arkin (owner), `v` viewer (subscribed to changes), `s` scout
/// (proposer agent, subscribed to changes), `e` editor (editor agent).
pub fn fresh() -> Rig {
    let mut r = Rig::thyao();
    r.hello("a", "arkin", "desktop");
    r.hello("v", "viewer", "phone");
    r.hello("s", "scout", "headless");
    r.hello("e", "editor", "headless");
    r.sub("v", "ch", json!({"kind":"changes"}));
    r.sub("s", "ch", json!({"kind":"changes"}));
    r
}

fn kinds(v: &Value) -> String {
    v.as_array()
        .map(|a| {
            a.iter()
                .map(|c| c["op"].as_str().unwrap_or("?").to_owned())
                .collect::<Vec<_>>()
                .join(",")
        })
        .unwrap_or_default()
}

/// Records the outcome of `out` (the lines of one command) on `rig` as case `name`;
/// `before` is the log length before the command.
pub fn record(book: &mut Book, name: &str, rig: &Rig, before: usize, out: &[Value]) {
    if rig.log_len() > before {
        let e = rig.last_entry();
        let sum = format!(
            "{} :: summary={} changes=[{}] inverse=[{}]",
            order(out),
            e["summary"],
            kinds(&e["changes"]),
            kinds(&e["inverse"])
        );
        book.put(name, &jdigest(&(out, &e)), &sum);
    } else {
        let errs: Vec<String> = out
            .iter()
            .filter(|l| l["msg"]["type"] == "error")
            .map(|l| l["msg"]["error"].to_string())
            .collect();
        let sum = if errs.is_empty() {
            order(out)
        } else {
            format!("{} :: {}", order(out), errs.join(" ; "))
        };
        book.put(name, &jdigest(&out), &sum);
    }
}

/// Runs `ops` on conn `conn` of `rig` after `prep` (owner commands) and records it.
pub fn on(book: &mut Book, name: &str, rig: &mut Rig, prep: &[Value], conn: &str, ops: Value) {
    for (i, p) in prep.iter().enumerate() {
        let out = rig.cmd("a", &format!("p{i}"), p.clone());
        assert!(
            out.iter().all(|l| l["msg"]["type"] != "error"),
            "{name}: preparation {i} failed: {out:?}"
        );
    }
    let before = rig.log_len();
    let out = rig.cmd(conn, "c", ops);
    record(book, name, rig, before, &out);
    assert!(rig.replay_ok(), "{name}: replay differs");
}

/// A case on a fresh standard rig, by the owner.
pub fn case(book: &mut Book, name: &str, prep: &[Value], ops: Value) {
    let mut rig = fresh();
    on(book, name, &mut rig, prep, "a", ops);
}

/// A case on a fresh standard rig, by `conn`.
pub fn case_by(book: &mut Book, name: &str, prep: &[Value], conn: &str, ops: Value) {
    let mut rig = fresh();
    on(book, name, &mut rig, prep, conn, ops);
}
