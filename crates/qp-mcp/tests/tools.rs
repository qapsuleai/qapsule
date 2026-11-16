//! Generated tools: coverage of the protocol schema, validity of the input schemas
//! against the protocol's valid examples, JSON-RPC handling, conformance over MCP.

use std::path::{Path, PathBuf};

use qp_mcp::conform::{run_file, to_tool};
use qp_mcp::tools::{self, ToolKind, COMMAND_EXTRAS, GENERIC};
use qp_mcp::{Config, Hub, McpServer};
use qp_schema::protocol::{Surface, SurfaceKind};
use serde_json::{json, Value};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn tags(def: &str, tag: &str) -> Vec<String> {
    let s = qp_schema::json_schema("ClientMsg").unwrap();
    s["$defs"][def]["oneOf"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["properties"][tag]["const"].as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn every_op_and_query_has_a_tool_in_schema_order() {
    let ops = tags("Op", "op");
    let queries = tags("Query", "kind");
    assert!(ops.len() >= 20 && queries.len() >= 9);
    let mut expected: Vec<String> = ops.iter().map(|o| format!("op_{o}")).collect();
    expected.extend(queries.iter().map(|q| format!("query_{q}")));
    expected.extend(
        tags("ReplayControl", "kind")
            .iter()
            .map(|k| format!("replay_{k}")),
    );
    expected.extend(GENERIC.iter().map(|g| (*g).to_owned()));
    let names: Vec<&str> = tools::tools().iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names, expected);
    for t in tools::tools() {
        assert!(!t.description.trim().is_empty(), "{}", t.name);
        assert!(t.name.len() <= 128);
        assert!(t
            .name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "_-.".contains(c)));
        assert_eq!(t.input_schema["type"], "object", "{}", t.name);
        let props = t.input_schema["properties"].as_object().unwrap();
        match &t.kind {
            ToolKind::Op(_) => {
                assert!(!props.contains_key("op"));
                for k in COMMAND_EXTRAS {
                    assert!(props.contains_key(k), "{} lacks {k}", t.name);
                }
            }
            ToolKind::Query(_) | ToolKind::Replay(_) => assert!(!props.contains_key("kind")),
            ToolKind::Message(_) => {
                assert!(!props.contains_key("type"));
                let req = t.input_schema["required"].as_array().unwrap();
                assert!(!req.contains(&json!("id")));
            }
        }
        // Every tool schema compiles on its own (its `$defs` travel with it).
        jsonschema_compiles(&t.input_schema);
    }
}

fn jsonschema_compiles(s: &Value) {
    if let Err(e) = qp_schema::validate_with_schema(s, &json!(null)) {
        assert!(!e.iter().any(|m| m.contains("does not compile")), "{e:?}");
    }
}

#[test]
fn no_op_field_collides_with_command_fields() {
    let s = qp_schema::json_schema("ClientMsg").unwrap();
    for v in s["$defs"]["Op"]["oneOf"].as_array().unwrap() {
        for k in COMMAND_EXTRAS {
            assert!(v["properties"].get(k).is_none(), "op field `{k}` in {v}");
        }
    }
}

fn read(p: &Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
}

fn accepts(tool: &str, args: &Value) {
    let t = tools::find(tool).unwrap_or_else(|| panic!("no tool {tool}"));
    if let Err(e) = qp_schema::validate_with_schema(&t.input_schema, args) {
        panic!("{tool} rejects {args}: {e:?}");
    }
}

#[test]
fn tool_schemas_accept_the_valid_examples() {
    let dir = root().join("fixtures/protocol/valid");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    files.sort();
    let mut checked = 0;
    for f in files {
        let name = f.file_name().unwrap().to_string_lossy().into_owned();
        let v = read(&f);
        if name.starts_with("op.") {
            let mut a = v.as_object().unwrap().clone();
            let op = a.remove("op").unwrap();
            accepts(&format!("op_{}", op.as_str().unwrap()), &Value::Object(a));
            checked += 1;
        } else if name.starts_with("query.") {
            let mut a = v.as_object().unwrap().clone();
            let kind = a.remove("kind").unwrap();
            accepts(
                &format!("query_{}", kind.as_str().unwrap()),
                &Value::Object(a),
            );
            checked += 1;
        } else if name.starts_with("client_msg.") {
            let ty = v["type"].as_str().unwrap();
            if GENERIC.contains(&ty) {
                let mut a = v.as_object().unwrap().clone();
                a.remove("type");
                accepts(ty, &Value::Object(a.clone()));
                a.remove("id");
                accepts(ty, &Value::Object(a));
                checked += 1;
            }
            if let Some((tool, args, _)) = to_tool(&v) {
                accepts(&tool, &args);
                checked += 1;
            }
        }
    }
    assert!(checked >= 15, "only {checked} examples checked");
}

fn surface(kind: SurfaceKind) -> Surface {
    serde_json::from_value(json!({ "kind": kind })).unwrap()
}

fn hub() -> qp_mcp::SharedHub {
    let r = root();
    let init = qp_session::read_json(&r.join("fixtures/protocol/init/thyao.json")).unwrap();
    Hub::shared(qp_session::Session::new(init, &r).unwrap())
}

fn rpc(s: &mut McpServer, id: u64, method: &str, params: Value) -> Vec<Value> {
    s.handle_message(&json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}))
}

#[test]
fn jsonrpc_lifecycle_and_errors() {
    let h = hub();
    let mut s =
        McpServer::connect(h, "m", Config::new("arkin", surface(SurfaceKind::Desktop))).unwrap();
    assert_eq!(s.welcome()["type"], "welcome");
    let init = rpc(
        &mut s,
        1,
        "initialize",
        json!({"protocolVersion":"2025-11-25",
        "capabilities":{}, "clientInfo":{"name":"t","version":"0"}}),
    );
    assert_eq!(init[0]["result"]["protocolVersion"], "2025-11-25");
    assert!(init[0]["result"]["capabilities"]["tools"].is_object());
    assert_eq!(init[0]["result"]["serverInfo"]["name"], "qapsule");
    assert!(s
        .handle_message(&json!({"jsonrpc":"2.0","method":"notifications/initialized"}))
        .is_empty());
    let d = rpc(&mut s, 2, "server/discover", json!({}));
    assert!(d[0]["result"]["supportedVersions"]
        .as_array()
        .unwrap()
        .contains(&json!("2026-07-28")));
    assert_eq!(
        rpc(&mut s, 3, "ping", json!({}))[0]["result"]["resultType"],
        "complete"
    );
    let l = rpc(&mut s, 4, "tools/list", json!({}));
    assert_eq!(
        l[0]["result"]["tools"].as_array().unwrap().len(),
        tools::tools().len()
    );
    assert_eq!(
        rpc(&mut s, 5, "nope", json!({}))[0]["error"]["code"],
        -32601
    );
    assert_eq!(
        rpc(
            &mut s,
            6,
            "tools/call",
            json!({"name":"op_nope","arguments":{}})
        )[0]["error"]["code"],
        -32602
    );
    let bad = rpc(
        &mut s,
        7,
        "tools/list",
        json!({"_meta":{"io.modelcontextprotocol/protocolVersion":"1900-01-01"}}),
    );
    assert_eq!(bad[0]["error"]["code"], -32022);
    assert!(s.handle_line("{nope")[0].contains("-32700"));
    // A protocol error is a tool execution error.
    let e = rpc(
        &mut s,
        8,
        "tools/call",
        json!({"name":"op_add_study",
        "arguments":{"study":"rsi","inputs":["nope"]}}),
    );
    let r = &e[0]["result"];
    assert_eq!(r["isError"], true);
    assert_eq!(r["structuredContent"]["type"], "error");
    assert_eq!(r["structuredContent"]["id"], "mcp-1");
    let text: Value = serde_json::from_str(r["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(text, r["structuredContent"]);
}

#[test]
fn notifications_reach_the_subscriber_adapter() {
    let h = hub();
    let mut a = McpServer::connect(
        h.clone(),
        "a",
        Config::new("arkin", surface(SurfaceKind::Desktop)),
    )
    .unwrap();
    let mut s =
        McpServer::connect(h, "s", Config::new("scout", surface(SurfaceKind::Headless))).unwrap();
    let sub = rpc(
        &mut s,
        1,
        "tools/call",
        json!({"name":"subscribe","arguments":{"id":"sub1","topic":{"kind":"changes"}}}),
    );
    assert_eq!(
        sub[0]["result"]["structuredContent"]["type"], "ack",
        "{sub:?}"
    );
    let ack = rpc(
        &mut a,
        1,
        "tools/call",
        json!({"name":"op_add_study",
        "arguments":{"study":"sma","inputs":["px"],"params":{"period":5}},
        "_meta":{"dev.qapsule/id":"c1"}}),
    );
    assert_eq!(ack.len(), 1);
    assert_eq!(
        ack[0]["result"]["structuredContent"]["status"], "applied",
        "{ack:?}"
    );
    assert_eq!(ack[0]["result"]["structuredContent"]["id"], "c1");
    // Delivered before the response of the subscriber's next request.
    let next = rpc(&mut s, 2, "ping", json!({}));
    assert_eq!(next.len(), 2);
    assert_eq!(next[0]["method"], "notifications/message");
    assert_eq!(next[0]["params"]["data"]["type"], "notify");
    assert_eq!(next[0]["params"]["data"]["sub"], "sub1");
    assert!(next[1]["result"].is_object());
}

#[test]
fn conformance_transcripts_pass_over_mcp() {
    let r = root();
    let mut files = qp_session::conform::collect_files(&[
        r.join("fixtures/protocol/conformance"),
        r.join("crates/qp-session/tests/transcripts"),
    ])
    .unwrap();
    files.sort();
    assert!(files.len() >= 16);
    let mut calls = 0;
    for f in &files {
        let (o, s) = run_file(f, &r);
        assert!(o.failure.is_none(), "{}: {:?}", o.name, o.failure);
        calls += s.tool_calls;
    }
    assert!(calls > 50);
}

/// R2/T10: the MCP tools are regenerated from the schema, so the grammar operations
/// and the point query `hit` appear without hand-written tools. RD1 had 37 tools
/// (22 ops + 10 queries + 5 generic); R2 adds 10 grammar ops, `split_layer` (design A
/// fix, T9) and `hit` (49).
#[test]
fn grammar_ops_and_hit_are_generated_tools() {
    let grammar = [
        "insert_layer",
        "delete_layer",
        "move_layer",
        "set_encoding",
        "insert_panel",
        "delete_panel",
        "move_panel",
        "set_panel",
        "set_scale",
        "set_emphasis",
        "split_layer",
    ];
    for op in grammar {
        let name = format!("op_{op}");
        let t = tools::find(&name).unwrap_or_else(|| panic!("missing tool {name}"));
        assert!(matches!(t.kind, ToolKind::Op(_)), "{name}");
        let props = t.input_schema["properties"].as_object().unwrap();
        assert!(props.contains_key("view"), "{name} lacks view");
    }
    let hit = tools::find("query_hit").expect("missing tool query_hit");
    assert!(matches!(hit.kind, ToolKind::Query(_)));
    let props = hit.input_schema["properties"].as_object().unwrap();
    for k in ["view", "x", "y", "width", "height"] {
        assert!(props.contains_key(k), "query_hit lacks {k}");
    }
    let names: Vec<&str> = tools::tools().iter().map(|t| t.name.as_str()).collect();
    // RD1: 37; R2: grammar ops + `query_hit`; RP2: `query_binding`, `query_verify_evidence`;
    // R3 (§20): `op_set_audience`, `op_set_sensitivity`, `query_capsules`,
    // `query_capsule`, `query_ledger`.
    for k in ["query_binding", "query_verify_evidence"] {
        assert!(names.contains(&k), "missing tool {k}");
    }
    let r3 = [
        "op_set_audience",
        "op_set_sensitivity",
        "query_capsules",
        "query_capsule",
        "query_ledger",
    ];
    for k in r3 {
        assert!(names.contains(&k), "missing tool {k}");
    }
    // R4/T2 (§21): one tool per replay control and the generic `replay`.
    let r4 = [
        "replay_status",
        "replay_pause",
        "replay_resume",
        "replay_speed",
        "replay_step",
        "replay_seek",
        "replay",
    ];
    for k in r4 {
        assert!(names.contains(&k), "missing tool {k}");
    }
    assert_eq!(
        names.len(),
        37 + grammar.len() + 1 + 2 + r3.len() + r4.len(),
        "{names:?}"
    );
}
