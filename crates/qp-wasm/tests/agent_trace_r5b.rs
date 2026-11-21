//! (R5B, GOAL scope 6 "Gösterimde en az iz"; `docs/PROTOCOL.md` §25) The shell core shows
//! the agent's least trace: an opened capsule's validated comment with the label "ajan
//! yorumu" and its origin (agent, "… adına" the person), and the pending proposals with
//! their author; the reader's accept goes out as `accept_proposal` on the shell's own
//! (human, unmarked) connection and the authority applies it.
#![cfg(feature = "replica")]

use std::path::{Path, PathBuf};

use qp_schema::protocol::{InitRef, Principal, SessionInit, Transcript};
use qp_session::Session;
use qp_wasm::shell::ShellCore;
use serde_json::{json, Value};

const CAP: &str = "cap.THYAO.continuation.down.84";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The capsule entitlement session (transcript 39) with agent `aa` working for `arkin`.
fn init() -> SessionInit {
    let t: Transcript = qp_session::read_json(
        &root().join("fixtures/protocol/conformance/39-capsule-entitlement.json"),
    )
    .unwrap();
    let InitRef::Inline(i) = t.init else {
        panic!("inline init")
    };
    let mut i = *i;
    let pr: Principal = serde_json::from_value(json!({"id":"aa",
        "author":{"kind":"agent","name":"aa","model":"test"},"role":"proposer","on_behalf":"arkin"}))
    .unwrap();
    i.principals.push(pr);
    i
}

struct Rig {
    s: Session,
    core: ShellCore,
}

impl Rig {
    fn new(lang: &str) -> Self {
        let init = init();
        let data = init.data.clone().unwrap_or_default();
        let mut s = Session::new(init, &root()).unwrap();
        let hello = json!({"type":"hello","protocol":"qp/0","principal":"aa","surface":{"kind":"headless"}});
        s.handle_raw("A", &hello.to_string());
        s.mark_agent_connection("A");
        let core = ShellCore::new("arkin", lang, &data, &[]).unwrap();
        let mut r = Rig { s, core };
        r.pump();
        r
    }

    /// Delivers the authority's lines for the shell's connection `p` to the core.
    fn deliver(&mut self, lines: Vec<qp_schema::protocol::OutboundLine>) {
        for l in lines {
            if l.conn == "p" {
                let f = json!({"msg": l.msg}).to_string();
                self.core.on_frame(&f).unwrap();
            }
        }
    }

    /// Sends the core's outbox on `p` until it is empty; returns what was sent.
    fn pump(&mut self) -> Vec<Value> {
        let mut sent = Vec::new();
        for _ in 0..50 {
            let out = self.core.take_outbox();
            if out.is_empty() {
                break;
            }
            for m in out {
                let lines = self.s.handle_raw("p", &m.to_string());
                sent.push(m);
                self.deliver(lines);
            }
        }
        sent
    }

    /// The agent sends `cmd` on its marked connection; the shell gets its notifications.
    fn agent(&mut self, cmd: Value) -> Value {
        let lines = self.s.handle_raw("A", &cmd.to_string());
        let reply = lines
            .iter()
            .find(|l| l.conn == "A")
            .map(|l| serde_json::to_value(&l.msg).unwrap())
            .unwrap();
        self.deliver(lines);
        self.pump();
        reply
    }
}

#[test]
fn an_opened_capsule_shows_the_agent_comment_with_its_label_and_origin() {
    let mut r = Rig::new("tr");
    let ack = r.agent(json!({"type":"command","id":"k","ops":[{"op":"add_comment",
        "about":{"capsule":CAP},"lang":"tr",
        "text":"THYAO kısa ortalamanın (5,23) altında kalıyor [1].","cites":[CAP]}]}));
    assert_eq!(ack["status"], "applied", "{ack}");
    // Nothing open: no comment shown.
    assert_eq!(r.core.comments_json(), json!([]));
    r.core.open_capsule(CAP);
    let sent = r.pump();
    assert!(
        sent.iter()
            .any(|m| m["query"]["kind"] == "capsule" && m["query"]["capsule"] == CAP),
        "{sent:?}"
    );
    let rows = r.core.comments_json();
    assert_eq!(rows.as_array().unwrap().len(), 1, "{rows}");
    assert_eq!(rows[0]["label"], "ajan yorumu");
    assert_eq!(rows[0]["origin"], "aa, arkin adına");
    assert_eq!(
        rows[0]["text"],
        "THYAO kısa ortalamanın (5,23) altında kalıyor [1]."
    );
    // Back to the feed: the comment goes with the open capsule.
    r.core.back();
    r.pump();
    assert_eq!(r.core.comments_json(), json!([]));
}

#[test]
fn the_comment_label_follows_the_reader_language() {
    let mut r = Rig::new("en");
    r.agent(json!({"type":"command","id":"k","ops":[{"op":"add_comment",
        "about":{"capsule":CAP},"lang":"tr",
        "text":"THYAO kısa ortalamanın (5,23) altında kalıyor [1].","cites":[CAP]}]}));
    r.core.open_capsule(CAP);
    r.pump();
    let rows = r.core.comments_json();
    assert_eq!(rows[0]["label"], "agent comment", "{rows}");
    assert_eq!(rows[0]["origin"], "aa, on behalf of arkin", "{rows}");
}

#[test]
fn a_pending_proposal_is_listed_and_the_readers_accept_applies_it() {
    let mut r = Rig::new("tr");
    assert_eq!(r.core.proposals_json(), json!([]));
    let ack = r.agent(
        json!({"type":"command","id":"p","ops":[{"op":"show_finding",
        "finding":"fnd.THYAO.continuation.down.84","view":"v-show"}]}),
    );
    assert_eq!(ack["status"], "proposed", "{ack}");
    let pid = ack["proposal"].as_str().unwrap().to_owned();
    // The proposals topic told the shell; it asked the pending list again.
    let rows = r.core.proposals_json();
    assert_eq!(rows.as_array().unwrap().len(), 1, "{rows}");
    assert_eq!(rows[0]["id"], pid.as_str());
    assert_eq!(rows[0]["author"], "Öneren: aa (ajan)");
    assert_eq!(rows[0]["text"], "show_finding");
    // An unknown id sends nothing.
    r.core.decide("prop-nope", true);
    assert!(r.core.take_outbox().is_empty());
    // Accept: exactly the human decision op, on the shell's own connection.
    r.core.decide(&pid, true);
    let out = r.core.take_outbox();
    assert_eq!(out.len(), 1, "{out:?}");
    assert_eq!(out[0]["type"], "command");
    assert_eq!(
        out[0]["ops"],
        json!([{"op":"accept_proposal","proposal":pid}])
    );
    let lines = r.s.handle_raw("p", &out[0].to_string());
    let reply = serde_json::to_value(&lines.iter().find(|l| l.conn == "p").unwrap().msg).unwrap();
    assert_eq!(reply["type"], "ack", "{reply}");
    assert_eq!(reply["status"], "applied", "{reply}");
    r.deliver(lines);
    r.pump();
    assert_eq!(r.core.proposals_json(), json!([]), "no longer pending");
}

#[test]
fn reject_sends_reject_proposal() {
    let mut r = Rig::new("tr");
    let ack = r.agent(
        json!({"type":"command","id":"p","ops":[{"op":"show_finding",
        "finding":"fnd.THYAO.continuation.down.84","view":"v-show"}]}),
    );
    let pid = ack["proposal"].as_str().unwrap().to_owned();
    r.core.decide(&pid, false);
    let out = r.pump();
    assert_eq!(
        out[0]["ops"],
        json!([{"op":"reject_proposal","proposal":pid}])
    );
    assert_eq!(r.core.proposals_json(), json!([]));
}
