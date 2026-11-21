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

// ---------------------------------------------------------------------------
// What an agent may not write (§25.3)
// ---------------------------------------------------------------------------

fn policy_h() -> H {
    let mut i = agent_init();
    i.agent_policy = Some("r5b".to_owned());
    let mut h = H::with(i);
    h.hello("a", "arkin", "desktop");
    h.hello("s", "scout", "headless");
    h.hello("e", "editor", "headless");
    h.hello("b", "bot", "headless");
    h
}

fn engine_writes() -> Vec<Value> {
    let bar = json!({"t":1_900_000_000_000_i64,"o":1.0,"h":2.0,"l":0.5,"c":1.5,"v":10.0});
    vec![
        json!({"op":"add_fact","kind":"cross_above","series":"rsi14","bar":10,"text":"made up"}),
        json!({"op":"add_finding","claim":"Momentum güçlü.","facts":["fact-rsi-cross70"],
               "confidence":0.6,
               "invalidation":{"kind":"threshold","series":"rsi14","op":"cross_below","value":70.0},
               "capsule":"Momentum güçlü"}),
        json!({"op":"update_finding","finding":"fnd-overbought","confidence":0.99}),
        json!({"op":"put_series","series":{"id":"fake","source":{"kind":"inline"},
               "data":{"kind":"bars","bars":[bar]}}}),
        json!({"op":"append_bars","series":"px","bars":[bar]}),
        json!({"op":"remove","id":"fact-rsi-cross70"}),
        json!({"op":"remove","id":"fnd-overbought"}),
        json!({"op":"set_audience","principal":"viewer","audience":{"group":"future"}}),
        json!({"op":"set_sensitivity","principal":"viewer","sensitivity":"all"}),
    ]
}

fn assert_cannot_write(r: &Value) {
    assert_eq!(code(r), "forbidden", "{r}");
    assert_eq!(r["error"]["reason"], "agent_cannot_write", "{r}");
}

#[test]
fn under_the_r5b_policy_an_agent_writes_no_engine_items_directly_or_as_a_proposal() {
    let mut h = policy_h();
    assert!(h.s.agent_policy());
    let doc = serde_json::to_vec(h.s.document()).unwrap();
    for op in engine_writes() {
        for (conn, mode) in [
            ("e", "direct"),
            ("e", "auto"),
            ("e", "propose"),
            ("s", "auto"),
            ("s", "propose"),
            ("b", "direct"),
        ] {
            let r = h.one(
                conn,
                json!({"type":"command","id":"x","mode":mode,"ops":[op.clone()]}),
            );
            assert_cannot_write(&r);
        }
    }
    // Undo, and the latest bar.
    for op in [
        json!({"op":"undo"}),
        json!({"op":"update_last_bar","series":"px","bar":{"t":0,"o":1.0,"h":1.0,"l":1.0,"c":1.0,"v":1.0}}),
    ] {
        assert_cannot_write(&h.one("e", json!({"type":"command","id":"x","ops":[op]})));
    }
    assert_eq!(serde_json::to_vec(h.s.document()).unwrap(), doc);
    assert!(h.s.log().is_empty());
    // Views and drawings remain open to the agent, as proposals only: `auto` becomes a
    // proposal even with the `editor` role; `direct` is refused.
    let r = h.cmd(
        "e",
        "v1",
        json!([{"op":"set_view_range","view":"main","from_bar":10,"to_bar":40}]),
    );
    assert_eq!(r["status"], "proposed", "{r}");
    let r = h.one(
        "e",
        json!({"type":"command","id":"v1d","mode":"direct","ops":[{"op":"set_view_range","view":"main","from_bar":10,"to_bar":40}]}),
    );
    assert_eq!(code(&r), "forbidden", "{r}");
    assert_eq!(r["error"]["reason"], "proposal_only", "{r}");
    let r = h.cmd(
        "s",
        "v2",
        json!([{"op":"add_study","study":"ema","inputs":["px"],"params":{"period":9},"show":{"view":"main","panel":0}}]),
    );
    assert_eq!(r["status"], "proposed", "{r}");
    // A study under an existing id would replace a series: refused.
    let r = h.cmd(
        "s",
        "v2b",
        json!([{"op":"add_study","id":"rsi14","study":"ema","inputs":["px"],"params":{"period":9}}]),
    );
    assert_cannot_write(&r);
    // Not even its own audience or an intent (watches are the agent's way, §25.5).
    for op in [
        json!({"op":"set_audience","audience":{"group":"future"}}),
        json!({"op":"add_intent","text":"x"}),
        json!({"op":"put_series","series":{"id":"s2","source":{"kind":"study","study":"ema","inputs":["px"],"params":{"period":3}}}}),
        json!({"op":"set_params","series":"rsi14","params":{"period":3}}),
    ] {
        assert_cannot_write(&h.cmd("s", "v3", json!([op])));
    }
    // A person writes as before.
    let r = h.cmd(
        "a",
        "p1",
        json!([{"op":"update_finding","finding":"fnd-overbought","confidence":0.7}]),
    );
    assert_eq!(r["status"], "applied", "{r}");
}

#[test]
fn without_the_policy_only_marked_connections_follow_the_write_rule() {
    let mut h = h();
    // RD1 transcript 05: an agent proposes a finding; unchanged without the policy.
    let r = h.cmd("s", "c1", json!([engine_writes()[1].clone()]));
    assert_eq!(r["status"], "proposed", "{r}");
    // On a marked connection the same command is refused.
    h.s.mark_agent_connection("m");
    h.hello("m", "scout", "headless");
    let r = h.cmd("m", "c2", json!([engine_writes()[1].clone()]));
    assert_cannot_write(&r);
    // Even with a human identity on the marked connection.
    h.hello("m2", "arkin", "headless");
    h.s.mark_agent_connection("m2");
    for op in engine_writes() {
        assert_cannot_write(&h.cmd("m2", "c3", json!([op])));
    }
}

#[test]
fn an_unknown_agent_policy_is_refused() {
    let mut i = agent_init();
    i.agent_policy = Some("r9".to_owned());
    let e = qp_session::Session::new(i, &common::root())
        .err()
        .expect("refused");
    assert!(e.contains("init.agent_policy"), "{e}");
}

/// One sample of every op the allow-list names, and of every other op (keyed by the
/// op's wire name). The test below reads the op names from the generated schema: a new
/// `Op` variant without a sample here fails it, and `agent_op` denies it by default.
fn samples() -> Vec<(&'static str, Value)> {
    let bar = json!({"t":1,"o":1.0,"h":1.0,"l":1.0,"c":1.0,"v":1.0});
    let cond = json!({"kind":"threshold","series":"rsi14","op":"lt","value":30.0});
    vec![
        // Its view is filled in from the document by the test.
        ("put_view", json!({"op":"put_view","view":null})),
        (
            "insert_layer",
            json!({"op":"insert_layer","view":"main","layer":{"mark":"line","series":"rsi14","panel":1}}),
        ),
        (
            "delete_layer",
            json!({"op":"delete_layer","view":"main","layer":"l1"}),
        ),
        (
            "move_layer",
            json!({"op":"move_layer","view":"main","layer":"l1"}),
        ),
        (
            "split_layer",
            json!({"op":"split_layer","view":"main","layer":"l1"}),
        ),
        (
            "set_encoding",
            json!({"op":"set_encoding","view":"main","layer":"l1","encoding":{}}),
        ),
        (
            "insert_panel",
            json!({"op":"insert_panel","view":"main","panel":{"id":7}}),
        ),
        (
            "delete_panel",
            json!({"op":"delete_panel","view":"main","panel":1}),
        ),
        (
            "move_panel",
            json!({"op":"move_panel","view":"main","panel":1}),
        ),
        (
            "set_panel",
            json!({"op":"set_panel","view":"main","panel":1}),
        ),
        (
            "set_scale",
            json!({"op":"set_scale","view":"main","scale":"y0"}),
        ),
        (
            "set_emphasis",
            json!({"op":"set_emphasis","view":"main","emphasis":[]}),
        ),
        (
            "set_view_range",
            json!({"op":"set_view_range","view":"main","from_bar":1,"to_bar":2}),
        ),
        (
            "add_narrative",
            json!({"op":"add_narrative","about":"fnd-overbought","steps":[]}),
        ),
        (
            "show_finding",
            json!({"op":"show_finding","finding":"fnd-overbought"}),
        ),
        (
            "add_study",
            json!({"op":"add_study","study":"ema","inputs":["px"]}),
        ),
        (
            "add_comment",
            json!({"op":"add_comment","about":{"capsule":"c"},"lang":"tr","text":"t","cites":["fnd-overbought"]}),
        ),
        (
            "add_watch",
            json!({"op":"add_watch","text":"t","match":{"instrument":"THYAO"}}),
        ),
        ("remove_watch", json!({"op":"remove_watch","intent":"i"})),
        // Denied.
        (
            "put_series",
            json!({"op":"put_series","series":{"id":"x","source":{"kind":"inline"}}}),
        ),
        (
            "set_params",
            json!({"op":"set_params","series":"rsi14","params":{}}),
        ),
        (
            "add_layer",
            json!({"op":"add_layer","view":"main","layer":{"mark":"line","series":"rsi14","panel":1}}),
        ),
        (
            "remove_layer",
            json!({"op":"remove_layer","view":"main","index":0}),
        ),
        (
            "add_fact",
            json!({"op":"add_fact","kind":"k","series":"px","bar":1,"text":"t"}),
        ),
        (
            "add_finding",
            json!({"op":"add_finding","claim":"c","facts":[],"confidence":0.5,"invalidation":cond}),
        ),
        (
            "update_finding",
            json!({"op":"update_finding","finding":"fnd-overbought"}),
        ),
        (
            "approve_finding",
            json!({"op":"approve_finding","finding":"fnd-overbought"}),
        ),
        (
            "reject_finding",
            json!({"op":"reject_finding","finding":"fnd-overbought"}),
        ),
        ("add_intent", json!({"op":"add_intent","text":"t"})),
        (
            "set_intent_active",
            json!({"op":"set_intent_active","intent":"i","active":false}),
        ),
        ("remove", json!({"op":"remove","id":"main"})),
        (
            "append_bars",
            json!({"op":"append_bars","series":"px","bars":[bar]}),
        ),
        (
            "update_last_bar",
            json!({"op":"update_last_bar","series":"px","bar":bar}),
        ),
        ("undo", json!({"op":"undo"})),
        (
            "accept_proposal",
            json!({"op":"accept_proposal","proposal":"p"}),
        ),
        (
            "reject_proposal",
            json!({"op":"reject_proposal","proposal":"p"}),
        ),
        (
            "set_audience",
            json!({"op":"set_audience","audience":{"group":"future"}}),
        ),
        (
            "set_sensitivity",
            json!({"op":"set_sensitivity","sensitivity":null}),
        ),
    ]
}

/// The allow-list (§25.3): name → how an agent may carry it.
fn allowed(name: &str) -> Option<qp_session::agent::AgentOp> {
    use qp_session::agent::AgentOp::{Direct, Proposal};
    match name {
        "put_view" | "insert_layer" | "delete_layer" | "move_layer" | "split_layer"
        | "set_encoding" | "insert_panel" | "delete_panel" | "move_panel" | "set_panel"
        | "set_scale" | "set_emphasis" | "set_view_range" | "add_narrative" | "show_finding"
        | "add_study" => Some(Proposal),
        "add_comment" | "add_watch" | "remove_watch" => Some(Direct),
        _ => None,
    }
}

#[test]
fn the_agent_allow_list_covers_every_op_of_the_schema() {
    let schema = qp_schema::json_schema("Op").unwrap();
    let names: Vec<String> = schema["oneOf"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["properties"]["op"]["const"].as_str().unwrap().to_owned())
        .collect();
    assert!(names.len() > 30, "{names:?}");
    let samples = samples();
    let doc = H::with(agent_init()).s.document().clone();
    for name in &names {
        let (_, sample) = samples.iter().find(|(n, _)| n == name).unwrap_or_else(|| {
            panic!("op `{name}` has no sample: add one and decide whether agents may send it")
        });
        let mut sample = sample.clone();
        let view = serde_json::to_value(&doc.views[0]).unwrap();
        if name == "put_view" {
            sample["view"] = view.clone();
        }
        if name == "insert_panel" {
            sample["panel"] = view["panels"][0].clone();
        }
        let op: qp_schema::protocol::Op =
            serde_json::from_value(sample).unwrap_or_else(|e| panic!("sample of `{name}`: {e}"));
        let got = qp_session::agent::agent_op(&doc, &op);
        let want = allowed(name).unwrap_or(qp_session::agent::AgentOp::Deny);
        assert_eq!(got, want, "op `{name}`");
    }
    for (n, _) in &samples {
        assert!(names.iter().any(|x| x == n), "sample `{n}` is not an op");
    }
}

#[test]
fn every_denied_op_is_refused_on_the_wire_in_every_mode() {
    let mut h = policy_h();
    for (name, sample) in samples() {
        if allowed(name).is_some() {
            continue;
        }
        for mode in ["auto", "direct", "propose"] {
            let r = h.one(
                "e",
                json!({"type":"command","id":"x","mode":mode,"ops":[sample.clone()]}),
            );
            assert_eq!(code(&r), "forbidden", "{name} {mode}: {r}");
            let reason = r["error"]["reason"].as_str().unwrap_or_default();
            assert!(
                reason == "agent_cannot_write" || reason == "human_only",
                "{name} {mode}: {r}"
            );
        }
    }
    assert!(h.s.log().is_empty());
}
