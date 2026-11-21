//! (R5B/T2) The served endpoint: surface registry, common tools (`render`,
//! `notifications`), notification delivery by MCP revision, the measurement log.

use std::cell::RefCell;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use qp_mcp::calllog::CallLog;
use qp_mcp::surfaces::{self, input_schema};
use qp_mcp::{tools, Config, Hub, McpServer, NotifyMode, Options, NOTIFY_URI};
use qp_schema::protocol::Surface;
use serde_json::{json, Value};

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

fn served(h: &qp_mcp::SharedHub, conn: &str, principal: &str, log: Option<CallLog>) -> McpServer {
    McpServer::open(
        h.clone(),
        conn,
        Config::new(principal, headless()),
        surfaces::by_name("thin").unwrap(),
        Options {
            notify: NotifyMode::Spec,
            common_tools: true,
            log: log.map(CallLog::shared),
        },
    )
    .unwrap()
}

const MODERN: &str = "2026-07-28";

fn rpc(s: &mut McpServer, id: u64, method: &str, mut params: Value) -> Vec<Value> {
    params["_meta"]["io.modelcontextprotocol/protocolVersion"] = json!(MODERN);
    s.handle_message(&json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}))
}

fn call(s: &mut McpServer, id: u64, name: &str, args: Value) -> Value {
    let out = rpc(
        s,
        id,
        "tools/call",
        json!({"name": name, "arguments": args}),
    );
    assert_eq!(out.len(), 1, "{out:?}");
    out[0]["result"].clone()
}

#[test]
fn registry_has_thin_first_and_it_mirrors_the_generated_tools() {
    assert_eq!(surfaces::names()[0], "thin");
    assert!(surfaces::by_name("nope").is_none());
    let thin = surfaces::by_name("thin").unwrap();
    let defs = thin.tools();
    let names: Vec<&str> = defs.iter().map(|t| t.name.as_str()).collect();
    let generated: Vec<&str> = tools::tools().iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names, generated);
    for t in &defs {
        let read = t.name.starts_with("query_") || t.name == "query" || t.name == "replay_status";
        let a = t.to_mcp()["annotations"].clone();
        assert_eq!(a["readOnlyHint"], read, "{}", t.name);
        assert_eq!(a["destructiveHint"], false, "{}", t.name);
    }
}

#[test]
fn input_schema_helper_generates_from_types() {
    let s = input_schema::<qp_mcp::common::RenderArgs>();
    assert_eq!(s["type"], "object");
    assert!(s["properties"]["view"].is_object());
    assert_eq!(s["required"], json!(["view"]));
    assert!(s.get("$schema").is_none());
}

#[test]
fn served_endpoint_lists_common_tools_and_the_notifications_resource() {
    let h = hub();
    let mut s = served(&h, "m", "scout", None);
    let l = rpc(&mut s, 1, "tools/list", json!({}));
    let tools = l[0]["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), tools::tools().len() + 2);
    assert_eq!(tools[tools.len() - 2]["name"], "render");
    assert_eq!(tools[tools.len() - 1]["name"], "notifications");
    assert!(l[0]["result"]["ttlMs"].is_u64());
    let r = rpc(&mut s, 2, "resources/list", json!({}));
    assert_eq!(r[0]["result"]["resources"][0]["uri"], NOTIFY_URI);
    let bad = rpc(
        &mut s,
        3,
        "resources/read",
        json!({"uri": "qapsule://nope"}),
    );
    assert_eq!(bad[0]["error"]["code"], -32002);
    // The RD1 adapter (conformance) offers the protocol tools only.
    let mut plain = McpServer::connect(h, "p", Config::new("arkin", headless())).unwrap();
    let l = rpc(&mut plain, 1, "tools/list", json!({}));
    assert_eq!(
        l[0]["result"]["tools"].as_array().unwrap().len(),
        tools::tools().len()
    );
}

#[test]
fn render_returns_a_png_image() {
    let h = hub();
    let mut s = served(&h, "m", "scout", None);
    let r = call(
        &mut s,
        1,
        "render",
        json!({"view": "main", "width": 320, "height": 200}),
    );
    assert_eq!(r["isError"], false, "{r}");
    let img = &r["content"][0];
    assert_eq!(img["type"], "image");
    assert_eq!(img["mimeType"], "image/png");
    // The PNG signature (89 50 4E 47 0D 0A 1A 0A) in base64.
    assert!(img["data"].as_str().unwrap().starts_with("iVBORw0KGgo"));
    assert_eq!(r["content"][1]["type"], "text");
    // An unknown view is a tool error; a bad size is an invalid-params error.
    let e = call(&mut s, 2, "render", json!({"view": "nope"}));
    assert_eq!(e["isError"], true);
    let bad = rpc(
        &mut s,
        3,
        "tools/call",
        json!({"name": "render", "arguments": {"view": "main", "width": 9}}),
    );
    assert_eq!(bad[0]["error"]["code"], -32602);
}

/// K33: `render` does not draw data the agent is not entitled to.
#[test]
fn render_refuses_a_view_of_unentitled_data() {
    use qp_schema::protocol::ReplayInit;
    let r = root();
    let init = qp_session::demo_init(ReplayInit {
        instruments: Some(2),
        days: Some(20),
        history: Some(10),
        entitlements: [
            ("host".to_owned(), vec!["*".to_owned()]),
            ("arkin".to_owned(), vec!["*".to_owned()]),
            ("scout".to_owned(), Vec::new()),
        ]
        .into(),
        ..ReplayInit::default()
    });
    let h = Hub::shared_replay(
        qp_session::ReplayHost::with_local_dir(init, &r, Path::new("")).unwrap(),
    );
    let mut owner = McpServer::connect(h.clone(), "a", Config::new("arkin", headless())).unwrap();
    let list = owner
        .call_tool("query_list", json!({"collection": "series"}), None)
        .unwrap();
    let series = list["structuredContent"]["data"]["items"][0]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let put = owner
        .call_tool(
            "op_put_view",
            json!({"view": {"id": "v", "x": {"kind": "time"},
                "panels": [{"id": 0, "y": {"kind": "linear"}}],
                "layers": [{"id": "c", "mark": "candle", "series": series, "panel": 0}]}}),
            None,
        )
        .unwrap();
    assert_eq!(put["structuredContent"]["status"], "applied", "{put}");
    let ok = owner_render(&h, "arkin");
    assert_eq!(ok["isError"], false, "{ok}");
    let no = owner_render(&h, "scout");
    assert_eq!(no["isError"], true, "{no}");
    assert!(no["content"].to_string().contains("not readable"), "{no}");
}

fn owner_render(h: &qp_mcp::SharedHub, principal: &str) -> Value {
    let mut s = served(h, &format!("r-{principal}"), principal, None);
    call(
        &mut s,
        1,
        "render",
        json!({"view": "v", "width": 200, "height": 120}),
    )
}

#[test]
fn modern_clients_get_resource_updates_and_read_the_notifications() {
    let h = hub();
    let mut s = served(&h, "s", "scout", None);
    let mut owner = McpServer::connect(h.clone(), "a", Config::new("arkin", headless())).unwrap();
    let sub = call(
        &mut s,
        1,
        "subscribe",
        json!({"id": "sub1", "topic": {"kind": "changes"}}),
    );
    assert_eq!(sub["structuredContent"]["type"], "ack", "{sub}");
    let ack = rpc(&mut s, 2, "subscriptions/listen", json!({}));
    assert_eq!(ack.len(), 1);
    assert_eq!(ack[0]["method"], "notifications/subscriptions/acknowledged");
    assert_eq!(
        ack[0]["params"]["resourceSubscriptions"],
        json!([NOTIFY_URI])
    );
    assert!(s.deliver(false).updated.is_none());
    let w = owner
        .call_tool(
            "op_add_study",
            json!({"study":"sma","inputs":["px"],"params":{"period":5}}),
            None,
        )
        .unwrap();
    assert_eq!(w["structuredContent"]["status"], "applied", "{w}");
    // No `notifications/message` in a modern response…
    let p = rpc(&mut s, 3, "ping", json!({}));
    assert_eq!(p.len(), 1);
    // …but one `resources/updated` signal, then the notification is read once.
    let d = s.deliver(false);
    assert!(d.messages.is_empty());
    let u = d.updated.expect("an update signal");
    assert_eq!(u["method"], "notifications/resources/updated");
    assert_eq!(u["params"]["uri"], NOTIFY_URI);
    assert_eq!(
        u["params"]["_meta"]["io.modelcontextprotocol/subscriptionId"],
        2
    );
    assert!(s.deliver(false).updated.is_none());
    let r = rpc(&mut s, 4, "resources/read", json!({"uri": NOTIFY_URI}));
    let text: Value =
        serde_json::from_str(r[0]["result"]["contents"][0]["text"].as_str().unwrap()).unwrap();
    let items = text["notifications"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["msg"]["sub"], "sub1");
    // Taken: the tool finds nothing more.
    let n = call(&mut s, 5, "notifications", json!({}));
    assert_eq!(n["structuredContent"]["notifications"], json!([]));
    // Cancelling the listen ends the signals.
    s.handle_message(&json!({"jsonrpc":"2.0","method":"notifications/cancelled",
        "params":{"requestId": 2}}));
    assert!(!s.listening());
}

#[test]
fn legacy_clients_get_notifications_message_on_their_stream() {
    let h = hub();
    let mut s = served(&h, "s", "scout", None);
    let mut owner = McpServer::connect(h.clone(), "a", Config::new("arkin", headless())).unwrap();
    let init = s.handle_message(&json!({"jsonrpc":"2.0","id":1,"method":"initialize",
        "params":{"protocolVersion":"2025-11-25","capabilities":{},
                  "clientInfo":{"name":"t","version":"0"}}}));
    assert_eq!(init[0]["result"]["protocolVersion"], "2025-11-25");
    assert_eq!(s.legacy(), Some("2025-11-25"));
    s.call_tool(
        "subscribe",
        json!({"id": "sub1", "topic": {"kind": "changes"}}),
        None,
    )
    .unwrap();
    owner
        .call_tool(
            "op_add_study",
            json!({"study":"sma","inputs":["px"],"params":{"period":5}}),
            None,
        )
        .unwrap();
    // Without a stream they wait; with one they go out as `notifications/message`.
    assert!(s.deliver(false).messages.is_empty());
    let d = s.deliver(true);
    assert_eq!(d.messages.len(), 1);
    assert_eq!(d.messages[0]["method"], "notifications/message");
    assert_eq!(d.messages[0]["params"]["data"]["sub"], "sub1");
    assert!(s.deliver(true).messages.is_empty());
}

#[derive(Clone, Default)]
struct Buf(Rc<RefCell<Vec<u8>>>);

impl Write for Buf {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(b);
        Ok(b.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn call_log_has_one_line_per_tool_call() {
    let h = hub();
    let buf = Buf::default();
    let mut s = served(
        &h,
        "m",
        "scout",
        Some(CallLog::to_writer(Box::new(buf.clone()))),
    );
    s.set_run(Some("r1".to_owned()));
    call(&mut s, 1, "query_get", json!({"id": "main"}));
    call(&mut s, 2, "query_get", json!({"id": "nope"}));
    rpc(&mut s, 3, "ping", json!({}));
    let text = String::from_utf8(buf.0.borrow().clone()).unwrap();
    let lines: Vec<Value> = text
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(lines.len(), 2, "{text}");
    for (l, ok) in lines.iter().zip([true, false]) {
        let keys: Vec<&String> = l.as_object().unwrap().keys().collect();
        assert_eq!(
            keys,
            [
                "args_bytes",
                "ms",
                "ok",
                "principal",
                "result_bytes",
                "run",
                "tool"
            ]
        );
        assert_eq!(l["run"], "r1");
        assert_eq!(l["principal"], "scout");
        assert_eq!(l["tool"], "query_get");
        assert_eq!(l["ok"], ok);
        assert!(l["args_bytes"].as_u64().unwrap() > 0);
        assert!(l["result_bytes"].as_u64().unwrap() > 0);
    }
}
