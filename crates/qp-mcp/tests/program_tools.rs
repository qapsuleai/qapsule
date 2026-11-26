//! (RL1, PS §3.3) The thin surface's program tools: listed on the agent's endpoint only,
//! text or IR forwarded as the program queries, `program_add` lowering text and sending
//! one `add_program` command that becomes a proposal. The tests that need the program
//! subsystem registered in the session are ignored until `qp-program` (rl1/program) is
//! merged.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

use qp_mcp::surfaces::{self, thin::program::PROGRAM_TOOLS};
use qp_mcp::{Config, Hub, McpServer, NotifyMode, Options};
use qp_schema::protocol::Surface;
use serde_json::{json, Value};

const TEXT: &str = "r = rsi(close, 14)\nseries rsi_line = r\nevent up = cross_above(r, 70)\n";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn hub() -> qp_mcp::SharedHub {
    let r = root();
    let init = qp_session::read_json(&r.join("fixtures/protocol/init/thyao.json")).unwrap();
    Hub::shared(qp_session::Session::new(init, &r).unwrap())
}

fn headless() -> Surface {
    serde_json::from_value(json!({"kind": "headless"})).unwrap()
}

fn served(h: &qp_mcp::SharedHub, principal: &str) -> McpServer {
    McpServer::open(
        h.clone(),
        "m",
        Config::new(principal, headless()),
        surfaces::by_name("thin").unwrap(),
        Options {
            notify: NotifyMode::Spec,
            common_tools: true,
            log: None,
            capsule_format: None,
        },
    )
    .unwrap()
}

fn rpc(s: &mut McpServer, id: u64, method: &str, mut params: Value) -> Value {
    params["_meta"]["io.modelcontextprotocol/protocolVersion"] = json!("2026-07-28");
    let out = s.handle_message(&json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}));
    assert_eq!(out.len(), 1, "{out:?}");
    out[0].clone()
}

fn call(s: &mut McpServer, id: u64, name: &str, args: Value) -> Value {
    rpc(
        s,
        id,
        "tools/call",
        json!({"name": name, "arguments": args}),
    )["result"]
        .clone()
}

/// The document and its pending proposals as the owner reads them.
fn state(h: &qp_mcp::SharedHub) -> String {
    let mut out = String::new();
    for q in [
        json!({"kind":"document"}),
        json!({"kind":"list","collection":"proposals"}),
    ] {
        let m = json!({"type":"query","id":"q","query":q});
        let r = h.borrow_mut().send("audit", &m.to_string());
        assert_eq!(r[0]["type"], "result", "{r:?}");
        out.push_str(&r[0]["data"].to_string());
    }
    out
}

#[test]
fn program_tools_are_listed_on_the_agent_endpoint_only() {
    let h = hub();
    let mut s = served(&h, "scout");
    let l = rpc(&mut s, 1, "tools/list", json!({}));
    let tools = l["result"]["tools"].as_array().unwrap();
    for name in PROGRAM_TOOLS {
        let t = tools
            .iter()
            .find(|t| t["name"] == name)
            .unwrap_or_else(|| panic!("{name} listed"));
        let p = &t["inputSchema"]["properties"]["program"];
        assert!(p.is_object(), "{name}: {t}");
        assert_eq!(
            t["annotations"]["readOnlyHint"],
            name != "program_add",
            "{name}"
        );
        assert!(t["description"].as_str().unwrap().len() < 400, "{name}");
    }
    // Text or IR: the `program` argument takes a string or an object.
    let check = tools.iter().find(|t| t["name"] == "program_check").unwrap();
    let schema = check["inputSchema"].to_string();
    assert!(
        schema.contains("\"string\"") && schema.contains("\"object\""),
        "{schema}"
    );
    // No decision tool, and the generated program tools stay.
    for name in ["op_accept_proposal", "op_reject_proposal"] {
        assert!(tools.iter().all(|t| t["name"] != name));
    }
    for name in ["query_check_program", "query_try_program", "op_add_program"] {
        assert!(tools.iter().any(|t| t["name"] == name), "{name}");
    }
    // The RD1 conformance adapter offers the protocol tools only.
    let mut plain = McpServer::connect(h, "p", Config::new("arkin", headless())).unwrap();
    let l = rpc(&mut plain, 1, "tools/list", json!({}));
    let names: Vec<&str> = l["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|t| t["name"].as_str())
        .collect();
    assert!(PROGRAM_TOOLS.iter().all(|n| !names.contains(n)));
}

#[test]
fn program_add_with_text_that_does_not_lower_sends_nothing() {
    let h = hub();
    {
        let mut hb = h.borrow_mut();
        let hello = json!({"type":"hello","protocol":"qp/0","principal":"arkin",
            "surface":{"kind":"headless"}});
        let r = hb.send("audit", &hello.to_string());
        assert_eq!(r[0]["type"], "welcome", "{r:?}");
    }
    let before = state(&h);
    let mut s = served(&h, "scout");
    let r = call(
        &mut s,
        2,
        "program_add",
        json!({"program": "r = rsj(close, 14)\nseries x = r\n", "inputs": ["px"]}),
    );
    assert_eq!(r["isError"], true, "{r}");
    let e = &r["structuredContent"]["error"];
    assert_eq!(e["reason"], "E_UNKNOWN_OP", "{r}");
    assert_eq!(e["path"], "/program");
    assert_eq!(e["did_you_mean"][0], "rsi");
    assert_eq!(e["errors"][0]["span"]["line"], 1);
    assert_eq!(state(&h), before);
    // Bad arguments are a JSON-RPC error.
    let bad = rpc(
        &mut s,
        3,
        "tools/call",
        json!({"name": "program_check", "arguments": {"program": 3}}),
    );
    assert_eq!(bad["error"]["code"], -32602, "{bad}");
}

#[test]
fn program_check_forwards_text_and_ir() {
    let h = hub();
    let mut s = served(&h, "scout");
    let r = call(&mut s, 1, "program_check", json!({"program": TEXT}));
    let d = &r["structuredContent"]["data"];
    assert_eq!(d["kind"], "program_check", "{r}");
    assert_eq!(d["ok"], true, "{r}");
    let ir = d["ir"].clone();
    assert!(ir.is_object(), "{r}");
    let r2 = call(
        &mut s,
        2,
        "program_check",
        json!({"program": ir, "inputs": ["px"]}),
    );
    let d2 = &r2["structuredContent"]["data"];
    assert_eq!(d2["ok"], true, "{r2}");
    assert_eq!(d2["program_id"], d["program_id"]);
    assert!(d2["cost"]["bars"].is_u64(), "{r2}");
    // Errors are in the answer, with spans, in the asked language.
    let r3 = call(
        &mut s,
        3,
        "program_check",
        json!({"program": "r = rsj(close, 14)\nseries x = r\n", "lang": "tr"}),
    );
    let d3 = &r3["structuredContent"]["data"];
    assert_eq!(r3["isError"], false, "{r3}");
    assert_eq!(d3["ok"], false);
    assert_eq!(d3["errors"][0]["reason"], "E_UNKNOWN_OP");
}

#[test]
fn program_try_and_explain_forward() {
    let h = hub();
    let mut s = served(&h, "scout");
    let r = call(
        &mut s,
        1,
        "program_try",
        json!({"program": TEXT, "inputs": ["px"]}),
    );
    let d = &r["structuredContent"]["data"];
    assert_eq!(d["kind"], "program_try", "{r}");
    assert_eq!(d["series"][0]["name"], "rsi_line", "{r}");
    assert_eq!(d["event"][0]["name"], "up", "{r}");
    let r = call(&mut s, 2, "program_explain", json!({"program": TEXT}));
    let d = &r["structuredContent"]["data"];
    assert_eq!(d["kind"], "program_explain", "{r}");
    assert!(d["text"].as_str().unwrap().contains("rsi("), "{r}");
    // A try needs a bar series: the protocol error is a tool error.
    let r = call(&mut s, 3, "program_try", json!({"program": TEXT}));
    assert_eq!(r["isError"], true, "{r}");
}

#[test]
fn program_add_from_an_agent_is_a_proposal() {
    let h = hub();
    let mut s = served(&h, "scout");
    let r = call(
        &mut s,
        1,
        "program_add",
        json!({"id": "p1", "program": TEXT, "inputs": ["px"], "reason": "rsi watch"}),
    );
    let ack = &r["structuredContent"];
    assert_eq!(ack["type"], "ack", "{r}");
    assert_eq!(ack["status"], "proposed", "{r}");
    assert!(ack["proposal"].is_string(), "{r}");
    // `direct` is refused for an agent (proposal only).
    let r = call(
        &mut s,
        2,
        "program_add",
        json!({"id": "p2", "program": TEXT, "inputs": ["px"], "mode": "direct"}),
    );
    assert_eq!(r["isError"], true, "{r}");
    assert_eq!(
        r["structuredContent"]["error"]["reason"], "proposal_only",
        "{r}"
    );
}
