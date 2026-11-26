//! Policy (PS §2.2) and the live budget (PS §2.3).

use serde_json::json;

use crate::rig::{code, reason, H, SMA_TEXT};

#[test]
fn an_agent_proposes_and_the_person_decides() {
    let mut h = H::new();
    let ir = h.ir(SMA_TEXT);
    let op =
        json!([{"op":"add_program","id":"p1","program":ir,"inputs":["px"],"show":{"view":"main"}}]);
    // Direct: refused, proposal only.
    let r = h.cmd_mode("s", "direct", op.clone());
    assert_eq!(code(&r), "forbidden", "{r}");
    assert_eq!(reason(&r), "proposal_only", "{r}");
    // Auto: a proposal; nothing in the document yet.
    let r = h.cmd_mode("s", "auto", op);
    assert_eq!(r["status"], "proposed", "{r}");
    let pid = r["proposal"].as_str().unwrap().to_owned();
    assert!(h.series("p1.avg").is_null());
    // The agent cannot accept its own proposal; the person does.
    let r = h.cmd("s", json!([{"op":"accept_proposal","proposal":pid}]));
    assert_eq!(reason(&r), "human_only", "{r}");
    let r = h.cmd("a", json!([{"op":"accept_proposal","proposal":pid}]));
    assert_eq!(r["status"], "applied", "{r}");
    let p = &h.series("p1.avg")["source"]["program"];
    assert_eq!(p["owner"], "arkin", "the decider owns it");
    let e = h.s.log().last().unwrap();
    assert!(e.approved_by.is_some());
    h.assert_replay();
}

#[test]
fn an_agent_proposes_new_values_but_never_removes() {
    let mut h = H::new();
    h.add("p1", SMA_TEXT);
    let r = h.cmd_mode(
        "s",
        "auto",
        json!([{"op":"set_program_params","program":"p1","params":{"len":8}}]),
    );
    assert_eq!(r["status"], "proposed", "{r}");
    for mode in ["auto", "propose", "direct"] {
        let r = h.cmd_mode("s", mode, json!([{"op":"remove_program","program":"p1"}]));
        assert_eq!(code(&r), "forbidden", "{mode}: {r}");
        assert_eq!(reason(&r), "agent_cannot_write", "{mode}: {r}");
    }
    // An existing instance id: never, not even as a proposal.
    let ir = h.ir(SMA_TEXT);
    let r = h.cmd_mode(
        "s",
        "auto",
        json!([{"op":"add_program","id":"p1","program":ir,"inputs":["px"]}]),
    );
    assert_eq!(reason(&r), "agent_cannot_write", "{r}");
    // A viewer may not write programs.
    let r = h.cmd_mode(
        "v",
        "direct",
        json!([{"op":"remove_program","program":"p1"}]),
    );
    assert_eq!(code(&r), "forbidden", "{r}");
}

#[test]
fn the_per_document_budget_refuses_in_the_check_phase() {
    let mut h = H::new();
    let ir = h.ir(SMA_TEXT);
    let mut refused = None;
    for k in 0..40 {
        let r = h.cmd(
            "a",
            json!([{"op":"add_program","id":format!("b{k}"),"program":ir,"inputs":["px"]}]),
        );
        if r["type"] == "error" {
            refused = Some((k, r));
            break;
        }
    }
    let (k, r) = refused.expect("the budget refuses before 40 instances");
    assert_eq!(reason(&r), "E_BUDGET", "{r}");
    assert_eq!(code(&r), "limit", "{r}");
    assert_eq!(r["error"]["path"], "/ops/0", "{r}");
    assert!(
        k <= 32,
        "at most program.live.doc.max_instances (32) fit: {k}"
    );
    let n =
        h.s.document()
            .series
            .iter()
            .filter(|s| s.id.starts_with('b') && s.id.contains('.'))
            .count();
    assert_eq!(n, 2 * k, "nothing of the refused command applied");
    // Removing one frees room.
    let r = h.cmd("a", json!([{"op":"remove_program","program":"b0"}]));
    assert_eq!(r["status"], "applied", "{r}");
    let r = h.cmd(
        "a",
        json!([{"op":"add_program","id":"again","program":ir,"inputs":["px"]}]),
    );
    assert_eq!(r["status"], "applied", "{r}");
}

#[test]
fn the_budget_is_rechecked_when_a_proposal_is_accepted() {
    let mut h = H::new();
    let ir = h.ir(SMA_TEXT);
    // The agent's proposal is made while there is room …
    let r = h.cmd_mode(
        "s",
        "auto",
        json!([{"op":"add_program","id":"late","program":ir,"inputs":["px"]}]),
    );
    assert_eq!(r["status"], "proposed", "{r}");
    let pid = r["proposal"].as_str().unwrap().to_owned();
    // … then the person fills their budget …
    for k in 0..40 {
        let r = h.cmd(
            "a",
            json!([{"op":"add_program","id":format!("b{k}"),"program":ir,"inputs":["px"]}]),
        );
        if r["type"] == "error" {
            break;
        }
    }
    // … and accepting it is refused; the proposal stays pending.
    let r = h.cmd("a", json!([{"op":"accept_proposal","proposal":pid}]));
    assert_eq!(reason(&r), "E_BUDGET", "{r}");
    let st = serde_json::to_value(h.s.document().proposals[0].state).unwrap();
    assert_eq!(st, "pending");
}
