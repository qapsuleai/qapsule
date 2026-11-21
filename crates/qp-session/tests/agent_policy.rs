//! (R5B, `docs/PROTOCOL.md` §25) The agent surface of the authority: human decisions,
//! what an agent may not write.

mod common;

use common::{code, init, H};
use qp_schema::protocol::{Principal, SessionInit};
use serde_json::{json, Value};

fn principal(v: Value) -> Principal {
    serde_json::from_value(v).unwrap()
}

/// The `thyao` init with two more agents: `bot` (role `owner`) and `scribe` (role
/// `proposer`, works for `viewer`).
fn agent_init() -> SessionInit {
    let mut i = init();
    i.principals.push(principal(
        json!({"id":"bot","author":{"kind":"agent","name":"bot"},"role":"owner"}),
    ));
    i.principals.push(principal(
        json!({"id":"scribe","author":{"kind":"agent","name":"scribe"},
        "role":"proposer","on_behalf":"viewer"}),
    ));
    i
}

fn h() -> H {
    let mut h = H::with(agent_init());
    h.hello("a", "arkin", "desktop");
    h.hello("s", "scout", "headless");
    h.hello("b", "bot", "headless");
    h.hello("w", "scribe", "headless");
    h
}

/// A view proposal by `conn` (`put_view` of a copy of `main` under a new id).
fn propose_view(h: &mut H, conn: &str, id: &str, view: &str) -> String {
    let mut v = serde_json::to_value(&h.s.document().views[0]).unwrap();
    v["id"] = json!(view);
    let r = h.one(
        conn,
        json!({"type":"command","id":id,"mode":"propose","ops":[{"op":"put_view","view":v}]}),
    );
    assert_eq!(r["status"], "proposed", "{r}");
    r["proposal"].as_str().unwrap().to_owned()
}

fn decide(h: &mut H, conn: &str, mode: &str, op: Value) -> Value {
    h.one(
        conn,
        json!({"type":"command","id":"d","mode":mode,"ops":[op]}),
    )
}

fn assert_human_only(r: &Value) {
    assert_eq!(code(r), "forbidden", "{r}");
    assert_eq!(r["error"]["reason"], "human_only", "{r}");
    assert_eq!(r["error"]["path"], "/ops/0", "{r}");
}

#[test]
fn an_agent_cannot_decide_its_own_proposal_in_any_mode() {
    let mut h = h();
    let p = propose_view(&mut h, "s", "c1", "v-scout");
    let before = h.s.log().len();
    for mode in ["auto", "direct", "propose"] {
        for op in ["accept_proposal", "reject_proposal"] {
            let r = decide(&mut h, "s", mode, json!({"op":op,"proposal":p}));
            assert_human_only(&r);
        }
    }
    assert_eq!(h.s.log().len(), before, "nothing was written");
    let r = h.query(
        "a",
        json!({"kind":"list","collection":"proposals","state":"pending"}),
    );
    assert_eq!(r["data"]["items"].as_array().unwrap().len(), 1);
}

#[test]
fn an_owner_agent_cannot_decide_anything() {
    let mut h = h();
    let p = propose_view(&mut h, "s", "c1", "v-scout");
    // Another agent's proposal, by an agent whose role is `owner`.
    for mode in ["auto", "direct"] {
        assert_human_only(&decide(
            &mut h,
            "b",
            mode,
            json!({"op":"accept_proposal","proposal":p}),
        ));
        assert_human_only(&decide(
            &mut h,
            "b",
            mode,
            json!({"op":"reject_proposal","proposal":p,"note":"no"}),
        ));
        assert_human_only(&decide(
            &mut h,
            "b",
            mode,
            json!({"op":"approve_finding","finding":"fnd-overbought"}),
        ));
        assert_human_only(&decide(
            &mut h,
            "b",
            mode,
            json!({"op":"reject_finding","finding":"fnd-overbought"}),
        ));
    }
    // Its own proposal neither.
    let own = h.one(
        "b",
        json!({"type":"command","id":"c2","mode":"propose","ops":[{"op":"set_view_range","view":"main","from_bar":10,"to_bar":20}]}),
    );
    let own = own["proposal"].as_str().unwrap().to_owned();
    assert_human_only(&decide(
        &mut h,
        "b",
        "auto",
        json!({"op":"accept_proposal","proposal":own}),
    ));
    // A decision op anywhere in a command refuses the whole command.
    let r = h.one(
        "b",
        json!({"type":"command","id":"c3","ops":[
            {"op":"set_view_range","view":"main","from_bar":10,"to_bar":20},
            {"op":"approve_finding","finding":"fnd-overbought"}]}),
    );
    assert_eq!(r["error"]["reason"], "human_only");
    assert_eq!(r["error"]["path"], "/ops/1");
    // An agent cannot propose a decision either.
    let r = h.one(
        "s",
        json!({"type":"command","id":"c4","mode":"propose","ops":[{"op":"approve_finding","finding":"fnd-overbought"}]}),
    );
    assert_human_only(&r);
}

#[test]
fn a_marked_connection_cannot_decide_even_with_a_human_identity() {
    let mut h = h();
    let p = propose_view(&mut h, "s", "c1", "v-scout");
    h.s.mark_agent_connection("m");
    assert!(h.s.is_agent_connection("m"));
    assert!(!h.s.is_agent_connection("a"));
    h.hello("m", "arkin", "headless");
    for op in [
        json!({"op":"accept_proposal","proposal":p}),
        json!({"op":"reject_proposal","proposal":p}),
        json!({"op":"approve_finding","finding":"fnd-overbought"}),
        json!({"op":"reject_finding","finding":"fnd-overbought"}),
    ] {
        assert_human_only(&decide(&mut h, "m", "auto", op));
    }
    // Marking after hello counts too.
    h.hello("m2", "arkin", "headless");
    h.s.mark_agent_connection("m2");
    assert_human_only(&decide(
        &mut h,
        "m2",
        "direct",
        json!({"op":"accept_proposal","proposal":p}),
    ));
}

#[test]
fn a_human_on_an_unmarked_connection_accepts_and_provenance_is_kept() {
    let mut h = h();
    let p = propose_view(&mut h, "s", "c1", "v-scout");
    assert!(h.s.document().views.iter().all(|v| v.id != "v-scout"));
    let r = decide(
        &mut h,
        "a",
        "auto",
        json!({"op":"accept_proposal","proposal":p}),
    );
    assert_eq!(r["status"], "applied", "{r}");
    let e = h.s.log().last().unwrap();
    assert_eq!(e.author, "scout");
    assert_eq!(e.approved_by.as_deref(), Some("arkin"));
    let v =
        h.s.document()
            .views
            .iter()
            .find(|v| v.id == "v-scout")
            .expect("applied");
    // The view carries no provenance of its own; the log is the audit trail. The
    // proposal records the decider.
    let _ = v;
    let prop = h.s.document().proposals.iter().find(|x| x.id == p).unwrap();
    assert_eq!(serde_json::to_value(prop.state).unwrap(), "accepted");
    assert_eq!(
        serde_json::to_value(prop.decided_by.as_ref().unwrap()).unwrap(),
        json!({"kind":"human","name":"arkin"})
    );
    assert_eq!(
        serde_json::to_value(&prop.provenance.author).unwrap(),
        json!({"kind":"agent","name":"scout"})
    );
    h.assert_replay();
}

#[test]
fn an_accepted_finding_proposal_is_authored_by_the_agent_and_approved_by_the_human() {
    let mut h = h();
    let r = h.one(
        "s",
        json!({"type":"command","id":"c1","ops":[{"op":"add_finding","claim":"Momentum güçlü.",
            "facts":["fact-rsi-cross70"],"confidence":0.6,
            "invalidation":{"kind":"threshold","series":"rsi14","op":"cross_below","value":70.0},
            "capsule":"Momentum güçlü"}]}),
    );
    assert_eq!(r["status"], "proposed", "{r}");
    let p = r["proposal"].as_str().unwrap().to_owned();
    let fid = r["created"][0].as_str().unwrap().to_owned();
    assert!(h.s.document().findings.iter().all(|f| f.id != fid));
    let r = decide(
        &mut h,
        "a",
        "auto",
        json!({"op":"accept_proposal","proposal":p}),
    );
    assert_eq!(r["status"], "applied", "{r}");
    let f =
        h.s.document()
            .findings
            .iter()
            .find(|f| f.id == fid)
            .unwrap();
    assert_eq!(
        serde_json::to_value(&f.provenance.author).unwrap(),
        json!({"kind":"agent","name":"scout"})
    );
    assert_eq!(
        serde_json::to_value(f.provenance.approved_by.as_ref().unwrap()).unwrap(),
        json!({"kind":"human","name":"arkin"})
    );
}

#[test]
fn rejected_and_pending_proposals_stay_out_of_the_document() {
    let mut h = h();
    let p1 = propose_view(&mut h, "s", "c1", "v-one");
    let _p2 = propose_view(&mut h, "s", "c2", "v-two");
    let r = decide(
        &mut h,
        "a",
        "auto",
        json!({"op":"reject_proposal","proposal":p1,"note":"no"}),
    );
    assert_eq!(r["status"], "applied", "{r}");
    let ids: Vec<&str> = h.s.document().views.iter().map(|v| v.id.as_str()).collect();
    assert!(
        !ids.contains(&"v-one") && !ids.contains(&"v-two"),
        "{ids:?}"
    );
    h.assert_replay();
}

#[test]
fn errors_before_r5b_carry_no_reason() {
    let mut h = h();
    h.hello("v", "viewer", "desktop");
    let r = decide(
        &mut h,
        "v",
        "auto",
        json!({"op":"set_view_range","view":"main","from_bar":1,"to_bar":2}),
    );
    assert_eq!(code(&r), "forbidden");
    assert!(r["error"].get("reason").is_none(), "{r}");
    // A human without `decide` gets the old `forbidden`, without a reason.
    let p = propose_view(&mut h, "s", "c1", "v-x");
    let r = decide(
        &mut h,
        "v",
        "auto",
        json!({"op":"accept_proposal","proposal":p}),
    );
    assert_eq!(code(&r), "forbidden");
    assert!(r["error"].get("reason").is_none(), "{r}");
}
