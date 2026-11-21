//! Generates the R5B conformance transcripts 65–70 (`docs/PROTOCOL.md` §25): the human
//! decision rule, what an agent may not write, comments (accepted, rejected, for an
//! unentitled person), watches (lifecycle and matching) and the proposal preview.
//!
//! The expected outputs are the reference authority's answers (welcome trimmed to its type
//! and `seq`). `qp conform` checks them in process, over `qp serve` and over MCP.
//! Deterministic: two runs write the same bytes.
//!
//! ```text
//! cargo run -p qp-session --example make_agent_transcripts            # check only
//! cargo run -p qp-session --example make_agent_transcripts -- --write
//! ```

use std::path::{Path, PathBuf};

use qp_schema::data::DataEvent;
use qp_schema::protocol::SessionInit;
use qp_session::Session;
use serde_json::{json, Value};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

enum In {
    Send(&'static str, Value, &'static str),
    Data(Value, &'static str),
}

fn hello(conn: &'static str, p: &str, note: &'static str) -> In {
    In::Send(
        conn,
        json!({"type":"hello","protocol":"qp/0","principal":p,"surface":{"kind":"headless"}}),
        note,
    )
}

fn cmd(conn: &'static str, id: &str, mode: Option<&str>, ops: Value, note: &'static str) -> In {
    let mut m = json!({"type":"command","id":id,"ops":ops});
    if let Some(md) = mode {
        m["mode"] = json!(md);
    }
    In::Send(conn, m, note)
}

fn q(conn: &'static str, id: &str, query: Value, note: &'static str) -> In {
    In::Send(conn, json!({"type":"query","id":id,"query":query}), note)
}

fn trim(v: Value) -> Value {
    if v["type"] == "welcome" {
        return json!({"type":"welcome","seq": v["seq"]});
    }
    v
}

fn build(name: &str, description: &str, covers: &[&str], init: Value, steps: Vec<In>) -> Value {
    let si: SessionInit = serde_json::from_value(init.clone()).expect("init");
    let mut s = Session::new(si, &root()).expect("session");
    let mut out = Vec::new();
    for st in steps {
        let (mut step, lines) = match st {
            In::Send(conn, msg, note) => {
                let lines = s.handle_raw(conn, &msg.to_string());
                (
                    json!({"send": {"conn": conn, "msg": msg}, "note": note}),
                    lines,
                )
            }
            In::Data(ev, note) => {
                let e: DataEvent = serde_json::from_value(ev.clone()).expect("event");
                let lines = s.data_event(&e).expect("data event applies");
                (json!({"data": ev, "note": note}), lines)
            }
        };
        step["expect"] = Value::Array(
            lines
                .iter()
                .map(
                    |l| json!({"conn": l.conn, "msg": trim(serde_json::to_value(&l.msg).unwrap())}),
                )
                .collect(),
        );
        out.push(step);
    }
    json!({
        "name": name,
        "description": description,
        "covers": covers,
        "init": init,
        "steps": out,
        "final": {"log_len": s.log().len()}
    })
}

/// The `thyao` init with the agents `bot` (role `owner`) and `scribe` (works for
/// `viewer`), optionally with the R5B agent policy.
fn thyao(policy: bool) -> Value {
    let mut i: Value =
        qp_session::read_json(&root().join("fixtures/protocol/init/thyao.json")).expect("init");
    let ps = i["principals"].as_array_mut().unwrap();
    ps.push(json!({"id":"bot","author":{"kind":"agent","name":"bot"},"role":"owner"}));
    ps.push(json!({"id":"scribe","author":{"kind":"agent","name":"scribe"},"role":"proposer","on_behalf":"viewer"}));
    if policy {
        i["agent_policy"] = json!("r5b");
    }
    i
}

fn t65() -> Value {
    let view_range = json!([{"op":"set_view_range","view":"main","from_bar":150,"to_bar":199}]);
    build(
        "65-human-decision",
        "R5B (§25.2): only a person decides. An agent cannot accept or reject its own proposal (any mode), an agent whose role is `owner` cannot decide another agent's proposal nor approve a finding (`forbidden`, `reason: human_only`); the person accepts: the entry is the agent's, approved by the person.",
        &["human-decision", "proposal", "forbidden", "provenance"],
        thyao(false),
        vec![
            hello("a", "arkin", "A person (`human`, owner)."),
            hello("s", "scout", "An agent (`proposer`)."),
            hello("b", "bot", "An agent whose role is `owner`."),
            cmd("s", "p1", None, view_range, "The agent's write becomes a proposal."),
            cmd("s", "d1", None, json!([{"op":"accept_proposal","proposal":"prop-1"}]), "Its own proposal: `human_only`."),
            cmd("s", "d2", Some("direct"), json!([{"op":"reject_proposal","proposal":"prop-1"}]), "Nor reject it, in any mode."),
            cmd("b", "d3", None, json!([{"op":"accept_proposal","proposal":"prop-1"}]), "An owner agent cannot decide either."),
            cmd("b", "d4", Some("propose"), json!([{"op":"approve_finding","finding":"fnd-overbought"}]), "Nor approve a finding, not even as a proposal."),
            q("a", "l1", json!({"kind":"list","collection":"proposals","state":"pending"}), "Still pending."),
            cmd("a", "d5", None, json!([{"op":"accept_proposal","proposal":"prop-1"}]), "The person accepts."),
            q("a", "g1", json!({"kind":"log","limit":1}), "The entry: author `scout`, `approved_by` `arkin`."),
        ],
    )
}

fn t66() -> Value {
    build(
        "66-agent-cannot-write",
        "R5B (§25.3): with `init.agent_policy = \"r5b\"` an agent command follows the allow-list: facts, findings, series and undo are `agent_cannot_write` in every mode; views are proposals only (`auto` becomes a proposal even for an `editor`; `direct` is `proposal_only`); a person writes as before.",
        &["agent-policy", "agent-cannot-write", "proposal-only", "forbidden"],
        thyao(true),
        vec![
            hello("a", "arkin", "A person."),
            hello("e", "editor", "An agent whose role is `editor`."),
            hello("s", "scout", "An agent (`proposer`)."),
            cmd("e", "w1", Some("direct"), json!([{"op":"add_fact","kind":"cross_above","series":"rsi14","bar":10,"text":"made up"}]), "A fact: never from an agent."),
            cmd("s", "w2", Some("propose"), json!([{"op":"add_finding","claim":"Momentum güçlü.","facts":["fact-rsi-cross70"],"confidence":0.6,"invalidation":{"kind":"threshold","series":"rsi14","op":"cross_below","value":70.0}}]), "A finding: not even as a proposal."),
            cmd("e", "w3", None, json!([{"op":"put_series","series":{"id":"s2","source":{"kind":"study","study":"ema","inputs":["px"],"params":{"period":3}}}}]), "A series, from any source."),
            cmd("e", "w4", None, json!([{"op":"undo"}]), "Undo."),
            cmd("e", "v1", Some("direct"), json!([{"op":"set_view_range","view":"main","from_bar":150,"to_bar":199}]), "A view op sent `direct`: `proposal_only`."),
            cmd("e", "v2", None, json!([{"op":"set_view_range","view":"main","from_bar":150,"to_bar":199}]), "The same with `auto`: a proposal, even for an editor."),
            cmd("a", "p1", None, json!([{"op":"update_finding","finding":"fnd-overbought","confidence":0.7}]), "A person writes as before."),
        ],
    )
}

/// Transcript 39's init (licensed flow; `ana` entitled, `can` not) with two agents: `aa`
/// for `ana`, `cc` for `can`.
fn flow_init() -> Value {
    let t: Value = qp_session::read_json(
        &root().join("fixtures/protocol/conformance/39-capsule-entitlement.json"),
    )
    .expect("39");
    let mut i = t["init"].clone();
    let ps = i["principals"].as_array_mut().unwrap();
    ps.push(json!({"id":"aa","author":{"kind":"agent","name":"aa","model":"test"},"role":"proposer","on_behalf":"ana"}));
    ps.push(json!({"id":"cc","author":{"kind":"agent","name":"cc","model":"test"},"role":"proposer","on_behalf":"can"}));
    i["agent_policy"] = json!("r5b");
    i
}

const CAP: &str = "cap.THYAO.continuation.down.84";

fn comment(conn: &'static str, id: &str, text: &str, cites: Value, note: &'static str) -> In {
    cmd(
        conn,
        id,
        None,
        json!([{"op":"add_comment","about":{"capsule":CAP},"lang":"tr","text":text,"cites":cites}]),
        note,
    )
}

fn t67() -> Value {
    build(
        "67-comment",
        "R5B (§25.4): an agent's comment is validated against what it may read (for `ana`, entitled): accepted with its provenance and `on_behalf`, sent on `comments` and in the `capsule` answer; rejected with machine-readable `reasons` (a number the evidence does not give, advice language, an unknown id).",
        &["comment", "validator", "comment-rejected", "comments-topic"],
        flow_init(),
        vec![
            hello("a", "ana", "The person."),
            hello("A", "aa", "Her agent."),
            In::Send("a", json!({"type":"subscribe","id":"cm","topic":{"kind":"comments","instrument":"THYAO"}}), "New comments on THYAO."),
            comment("A", "k1", "THYAO kısa ortalamanın (5,23) altında kalıyor [1].", json!([CAP]), "Every number is in the cited capsule's evidence (its sentence as ana reads it)."),
            comment("A", "k2", "THYAO 5,87 seviyesinde [1].", json!([CAP]), "A number the evidence does not give: `unsupported_number`."),
            comment("A", "k3", "THYAO'yu şimdi almak için fırsat [1].", json!([CAP]), "Advice: `advice_word`."),
            comment("A", "k4", "Kısa ortalamanın altında.", json!(["cap.THYAO.none.1"]), "An id that does not exist: `unknown_id`."),
            q("a", "c1", json!({"kind":"capsule","capsule":CAP}), "The capsule answer carries the comment."),
        ],
    )
}

fn t68() -> Value {
    build(
        "68-comment-unentitled",
        "R5B (§25.4, §25.7): an agent working for `can` (no licensed data) cannot carry a licensed number: the draft is rejected and the reasons are the same bytes as for a made-up number (no oracle); a comment without numbers passes, but `can` sees THYAO redacted, so a restricted reader receives no comment on it at all (structural, audit B1): only `ana` gets it; the entitled reader alone gets a comment with numbers.",
        &["comment", "license", "non-interference", "on-behalf"],
        flow_init(),
        vec![
            hello("a", "ana", "Entitled."),
            hello("c", "can", "Not entitled."),
            hello("C", "cc", "The agent working for can."),
            hello("A", "aa", "The agent working for ana."),
            In::Send("c", json!({"type":"subscribe","id":"cm","topic":{"kind":"comments"}}), "can's comments."),
            In::Send("a", json!({"type":"subscribe","id":"cm","topic":{"kind":"comments"}}), "ana's comments."),
            comment("C", "k1", "THYAO kısa ortalamanın (5,23) altında [1].", json!([CAP]), "5,23 is the authority's number, licensed for can: rejected as an unsupported number, no token."),
            comment("C", "k2", "THYAO kısa ortalamanın (5,87) altında [1].", json!([CAP]), "A made-up number: the same error bytes."),
            comment("C", "k3", "THYAO'da düşüş eğilimi sürüyor [1].", json!([CAP]), "No number: accepted; only ana gets it (can sees THYAO redacted: structural rule)."),
            comment("A", "k4", "THYAO kısa ortalamanın (5,23) altında kalıyor [1].", json!([CAP]), "ana's agent with the number: only ana gets it."),
        ],
    )
}

const SEED: u64 = 7;
const K: usize = 300;

fn watch_init() -> Value {
    json!({
        "document": {
            "schema_version": "0", "id": "doc-watch", "title": "Niyet",
            "series": [{"id":"px","instrument":{"symbol":"THYAO"},"interval":"1d",
                        "source":{"kind":"bind","symbol":"THYAO","interval":"1d",
                                  "window":{"last":10000},"feed":"bist"}}]
        },
        "principals": [
            {"id":"host","author":{"kind":"human","name":"host"},"role":"host"},
            {"id":"ana","author":{"kind":"human","name":"Ana"},"role":"viewer"},
            {"id":"bot","author":{"kind":"agent","name":"bot"},"role":"proposer","on_behalf":"ana"},
            {"id":"eve","author":{"kind":"human","name":"Eve"},"role":"viewer"}
        ],
        "clock": 1_760_000_000_000_i64,
        "data": {
            "feeds": [{"feed": {"symbol":"THYAO","interval":"1d","feed":"bist"}, "synthetic": {"n": K, "seed": SEED}}],
            "entitlements": {"host": ["*"], "ana": ["*"], "eve": ["*"]}
        },
        "capsules": {"series": ["px"]},
        "agent_policy": "r5b"
    })
}

fn bar(i: usize) -> Value {
    let b = qp_source::synthetic_bars(i + 1, SEED)[i].clone();
    json!({"feed": {"symbol":"THYAO","interval":"1d","feed":"bist"}, "seq": i - K + 1,
           "change": {"kind":"bar","bar": b}})
}

fn t69() -> Value {
    // The authority assigns watch ids (`watch-<seq>-0`): the first three commands follow
    // the load's engine entries.
    let si: SessionInit = serde_json::from_value(watch_init()).expect("init");
    let l = Session::new(si, &root()).expect("session").log().len();
    let w_any = format!("watch-{}-0", l + 1);
    let w_neg = format!("watch-{}-0", l + 2);
    let mut steps = vec![
        hello("a", "ana", "The person."),
        hello("b", "bot", "Her agent (`on_behalf: ana`); `welcome.limits.max_watches` is 32 with the agent policy."),
        hello("e", "eve", "Another person."),
        In::Send("a", json!({"type":"subscribe","id":"w","topic":{"kind":"intents"}}), "ana's intents."),
        In::Send("b", json!({"type":"subscribe","id":"w","topic":{"kind":"intents"}}), "The agent's."),
        In::Send("e", json!({"type":"subscribe","id":"w","topic":{"kind":"intents"}}), "eve's: she gets none of ana's matches."),
        cmd("b", "w1", None, json!([{"op":"add_watch","text":"THYAO'da bir şey olursa","match":{"instrument":"THYAO"},"max_matches":2}]), "A watch for ana (the agent's person), at most two matches."),
        cmd("b", "w2", None, json!([{"op":"add_watch","text":"THYAO'da olumsuz geçiş","match":{"instrument":"THYAO","negative":true}}]), "Negative transitions only."),
        cmd("b", "w3", None, json!([{"op":"add_watch","text":"süresi dolan","match":{"instrument":"THYAO"},"expires_t":0}]), "Already past its time: expires at the next engine look."),
        cmd("b", "w4", None, json!([{"op":"add_watch","for":"eve","text":"x","match":{}}]), "Another person's watch: `forbidden`."),
        cmd("e", "w5", None, json!([{"op":"remove_watch","intent":w_any}]), "eve cannot see ana's watch: `not_found`."),
    ];
    for i in K..K + 40 {
        steps.push(In::Data(bar(i), "A final bar: the engine's look; matching watches send `intent_matched` to ana and her agent."));
    }
    steps.push(cmd(
        "b",
        "w6",
        None,
        json!([{"op":"remove_watch","intent":w_neg}]),
        "Removed (`state: removed`).",
    ));
    steps.push(q(
        "a",
        "l1",
        json!({"kind":"list","collection":"intents"}),
        "Lifecycle: `matched` (2), `expired`, `removed`.",
    ));
    steps.push(q(
        "e",
        "l2",
        json!({"kind":"list","collection":"intents"}),
        "eve sees none of ana's watches.",
    ));
    build(
        "69-watch",
        "R5B (§25.5): watches — `add_watch` for the agent's person, matching on the engine's story transitions (deterministic, in the engine entry), `intent_matched` to the person and her agent only, `max_matches`, expiry, removal, privacy.",
        &["watch", "intent-matched", "intents", "lifecycle", "privacy"],
        watch_init(),
        steps,
    )
}

fn t70() -> Value {
    build(
        "70-preview",
        "R5B (§25.6): the `preview` of a pending proposal — its changes and the scene summary of the views it changes — without changing the document; a decided proposal is `not_pending`.",
        &["preview", "proposal"],
        thyao(false),
        vec![
            hello("a", "arkin", "A person."),
            hello("s", "scout", "An agent."),
            cmd("s", "p1", None, json!([{"op":"add_study","id":"ema9","study":"ema","inputs":["px"],"params":{"period":9},"show":{"view":"main","panel":0}}]), "The agent proposes an EMA on the price panel."),
            q("a", "v1", json!({"kind":"preview","proposal":"prop-1","width":800,"height":400}), "What accepting it would do: `put_series` + `insert_layer`, view `main` with layer `ema9`."),
            q("a", "l1", json!({"kind":"log"}), "Nothing was written but the proposal."),
            cmd("a", "d1", None, json!([{"op":"reject_proposal","proposal":"prop-1"}]), "The person rejects it."),
            q("s", "v2", json!({"kind":"preview","proposal":"prop-1"}), "No longer pending."),
        ],
    )
}

fn main() {
    let write = std::env::args().any(|a| a == "--write");
    let all = [
        ("65-human-decision", t65()),
        ("66-agent-cannot-write", t66()),
        ("67-comment", t67()),
        ("68-comment-unentitled", t68()),
        ("69-watch", t69()),
        ("70-preview", t70()),
    ];
    let mut differ = 0;
    for (name, t) in all {
        let path = root().join(format!("fixtures/protocol/conformance/{name}.json"));
        let mut text = serde_json::to_string_pretty(&t).expect("json");
        text.push('\n');
        let same = std::fs::read_to_string(&path).is_ok_and(|x| x == text);
        if write {
            if !same {
                std::fs::write(&path, &text).expect("write");
            }
            println!("{name}: {}", if same { "unchanged" } else { "written" });
        } else {
            println!("{name}: {}", if same { "same" } else { "DIFFERS" });
            if !same {
                differ += 1;
            }
        }
    }
    if differ > 0 {
        std::process::exit(1);
    }
}
