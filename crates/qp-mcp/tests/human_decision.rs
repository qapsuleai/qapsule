//! (R5B, Arkin's condition; PROTOCOL §25.2) A proposal is accepted or rejected only by a
//! command of human origin. An agent on its MCP endpoint cannot decide its own proposal
//! (nor another agent's): the decision tools are not offered, a call to one is an
//! unknown tool, and the same op sent through the generic `command` tool is refused by
//! the authority (`forbidden`, `reason: human_only`) — even for a human principal on a
//! connection the host marked as an agent's. The proposal stays pending and out of the
//! document until a person decides it on a connection of their own.

use std::path::{Path, PathBuf};

use qp_mcp::surfaces::{self, thin::HUMAN_DECISION_TOOLS};
use qp_mcp::{Config, Hub, McpServer, NotifyMode, Options, SharedHub};
use qp_schema::protocol::Surface;
use serde_json::{json, Value};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn hub() -> SharedHub {
    let r = root();
    let init = qp_session::read_json(&r.join("fixtures/protocol/init/thyao.json")).unwrap();
    Hub::shared(qp_session::Session::new(init, &r).unwrap())
}

fn headless() -> Surface {
    serde_json::from_value(json!({"kind": "headless"})).unwrap()
}

/// An agent's endpoint as `qp mcp serve` opens it (registry surface, marked connection).
fn served(h: &SharedHub, conn: &str, principal: &str) -> McpServer {
    McpServer::open(
        h.clone(),
        conn,
        Config::new(principal, headless()),
        surfaces::by_name("thin").unwrap(),
        Options {
            notify: NotifyMode::Spec,
            common_tools: true,
            log: None,
        },
    )
    .unwrap()
}

fn rpc(s: &mut McpServer, id: u64, method: &str, mut params: Value) -> Value {
    params["_meta"]["io.modelcontextprotocol/protocolVersion"] = json!("2026-07-28");
    let out = s.handle_message(&json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}));
    out.into_iter()
        .rev()
        .find(|m| m.get("id") == Some(&json!(id)))
        .expect("a response")
}

fn call(s: &mut McpServer, id: u64, name: &str, args: Value) -> Value {
    rpc(
        s,
        id,
        "tools/call",
        json!({"name": name, "arguments": args}),
    )
}

/// A person's own connection (not marked), speaking raw `qp/0`.
fn person(h: &SharedHub, conn: &str, who: &str) {
    let hello = json!({"type":"hello","protocol":"qp/0","principal":who,
        "surface":{"kind":"desktop"}});
    let r = h.borrow_mut().send(conn, &hello.to_string());
    assert_eq!(r[0]["type"], "welcome", "{r:?}");
}

fn pending(h: &SharedHub, conn: &str) -> Vec<String> {
    let q = json!({"type":"query","id":"lp","query":{"kind":"list","collection":"proposals","state":"pending"}});
    let r = h.borrow_mut().send(conn, &q.to_string());
    r[0]["data"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["id"].as_str().unwrap().to_owned())
        .collect()
}

fn view_domain(h: &SharedHub, conn: &str) -> Value {
    let q = json!({"type":"query","id":"gv","query":{"kind":"get","id":"main"}});
    let r = h.borrow_mut().send(conn, &q.to_string());
    r[0]["data"]["item"]["x"]["domain"].clone()
}

/// The agent proposes a change to view `main`; returns the proposal id.
fn propose(agent: &mut McpServer, id: u64) -> String {
    let r = call(
        agent,
        id,
        "op_set_view_range",
        json!({"view":"main","from_bar":100,"to_bar":150}),
    );
    let ack = &r["result"]["structuredContent"];
    assert_eq!(ack["type"], "ack", "{r}");
    assert_eq!(ack["status"], "proposed", "{r}");
    ack["proposal"].as_str().unwrap().to_owned()
}

fn decision(op: &str, proposal: &str) -> Value {
    if op.ends_with("_proposal") {
        json!({"op": op, "proposal": proposal})
    } else {
        json!({"op": op, "finding": "fnd-overbought"})
    }
}

#[test]
fn the_agent_endpoint_offers_no_decision_tool() {
    let h = hub();
    let mut agent = served(&h, "mcp-1", "scout");
    let list = rpc(&mut agent, 1, "tools/list", json!({}));
    let names: Vec<&str> = list["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    for t in HUMAN_DECISION_TOOLS {
        assert!(!names.contains(&t), "{t} is listed");
    }
    // The generic command tool is still there (the authority guards it).
    assert!(names.contains(&"command"));
}

#[test]
fn an_agent_cannot_decide_its_own_proposal_over_mcp() {
    let h = hub();
    let mut agent = served(&h, "mcp-1", "scout");
    person(&h, "arkin-desk", "arkin");
    let before = view_domain(&h, "arkin-desk");
    let pid = propose(&mut agent, 1);
    assert_eq!(pending(&h, "arkin-desk"), vec![pid.clone()]);

    // 1. A decision tool by name: unknown on the agent's endpoint.
    for (i, t) in HUMAN_DECISION_TOOLS.iter().enumerate() {
        let r = call(&mut agent, 10 + i as u64, t, json!({"proposal": pid}));
        assert_eq!(r["error"]["code"], -32602, "{t}: {r}");
    }
    // 2. The same ops through the generic `command` tool, in every mode: refused by
    //    the authority as a human decision.
    let mut id = 20;
    for op in [
        "accept_proposal",
        "reject_proposal",
        "approve_finding",
        "reject_finding",
    ] {
        for mode in [None, Some("auto"), Some("direct"), Some("propose")] {
            id += 1;
            let mut args = json!({"id": format!("c{id}"), "ops": [decision(op, &pid)]});
            if let Some(m) = mode {
                args["mode"] = json!(m);
            }
            let r = call(&mut agent, id, "command", args);
            let reply = &r["result"]["structuredContent"];
            assert_eq!(r["result"]["isError"], true, "{op} {mode:?}: {r}");
            assert_eq!(reply["error"]["code"], "forbidden", "{op} {mode:?}: {r}");
            assert_eq!(reply["error"]["reason"], "human_only", "{op} {mode:?}: {r}");
        }
    }
    // Nothing changed: still pending, not in the document.
    assert_eq!(pending(&h, "arkin-desk"), vec![pid.clone()]);
    assert_eq!(view_domain(&h, "arkin-desk"), before);

    // 3. Another agent (even one whose role writes directly) cannot decide it either.
    let mut other = served(&h, "mcp-2", "editor");
    let r = call(
        &mut other,
        40,
        "command",
        json!({"id":"c40","ops":[decision("accept_proposal", &pid)]}),
    );
    assert_eq!(
        r["result"]["structuredContent"]["error"]["reason"], "human_only",
        "{r}"
    );

    // 4. A human principal on a connection the host marked as an agent's: refused.
    let mut marked_owner = served(&h, "mcp-3", "arkin");
    let r = call(
        &mut marked_owner,
        41,
        "command",
        json!({"id":"c41","ops":[decision("accept_proposal", &pid)]}),
    );
    assert_eq!(
        r["result"]["structuredContent"]["error"]["reason"], "human_only",
        "{r}"
    );
    assert_eq!(pending(&h, "arkin-desk"), vec![pid.clone()]);
    assert_eq!(view_domain(&h, "arkin-desk"), before);

    // 5. The person, on their own connection, accepts: applied, authored by the agent,
    //    approved by the person.
    let cmd = json!({"type":"command","id":"h1","ops":[{"op":"accept_proposal","proposal":pid}]});
    let r = h.borrow_mut().send("arkin-desk", &cmd.to_string());
    assert_eq!(r[0]["type"], "ack", "{r:?}");
    assert_eq!(r[0]["status"], "applied", "{r:?}");
    assert!(pending(&h, "arkin-desk").is_empty());
    assert_ne!(view_domain(&h, "arkin-desk"), before);
    let q = json!({"type":"query","id":"lg","query":{"kind":"log","limit":1}});
    let r = h.borrow_mut().send("arkin-desk", &q.to_string());
    let entry = &r[0]["data"]["entries"][0];
    assert_eq!(entry["author"], "scout", "{entry}");
    assert_eq!(entry["approved_by"], "arkin", "{entry}");
}

#[test]
fn a_rejection_is_a_human_decision_too_and_leaves_no_trace_in_the_document() {
    let h = hub();
    let mut agent = served(&h, "mcp-1", "scout");
    person(&h, "arkin-desk", "arkin");
    let before = view_domain(&h, "arkin-desk");
    let pid = propose(&mut agent, 1);
    let r = call(
        &mut agent,
        2,
        "command",
        json!({"id":"c2","ops":[{"op":"reject_proposal","proposal":pid,"note":"self"}]}),
    );
    assert_eq!(
        r["result"]["structuredContent"]["error"]["reason"], "human_only",
        "{r}"
    );
    assert_eq!(pending(&h, "arkin-desk"), vec![pid.clone()]);
    let cmd = json!({"type":"command","id":"h1","ops":[{"op":"reject_proposal","proposal":pid,"note":"hayır"}]});
    let r = h.borrow_mut().send("arkin-desk", &cmd.to_string());
    assert_eq!(r[0]["status"], "applied", "{r:?}");
    assert!(pending(&h, "arkin-desk").is_empty());
    assert_eq!(view_domain(&h, "arkin-desk"), before);
}
