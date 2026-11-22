//! (R5B/M29) The thin surface lays out its capsule replies in the server's capsule
//! format (`qp mcp serve --capsule-format`): `card` text or `layered` JSON for
//! `query_capsule` / `query_capsules`; `json` (or none) leaves the protocol reply as is.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::cognitive_complexity
)]

use std::path::{Path, PathBuf};

use qp_mcp::{surfaces, Config, Hub, McpServer, NotifyMode, Options, SharedHub};
use qp_schema::protocol::{ReplayInit, Surface};
use serde_json::{json, Value};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The synthetic replay (as `tests/surface_mixed.rs`): three instruments, `claude`
/// reads everything.
fn hub() -> SharedHub {
    let all = || vec!["*".to_owned()];
    let mut init = qp_session::demo_init(ReplayInit {
        instruments: Some(3),
        days: Some(90),
        history: Some(60),
        entitlements: [
            ("host".to_owned(), all()),
            ("arkin".to_owned(), all()),
            ("claude".to_owned(), all()),
        ]
        .into(),
        ..ReplayInit::default()
    });
    init.principals.push(
        serde_json::from_value(
            json!({"id": "claude", "author": {"kind": "agent", "name": "claude"},
            "role": "proposer", "on_behalf": "arkin"}),
        )
        .unwrap(),
    );
    Hub::shared_replay(
        qp_session::ReplayHost::with_local_dir(init, &root(), Path::new("")).unwrap(),
    )
}

fn served(h: &SharedHub, format: Option<&str>) -> McpServer {
    let surface: Surface = serde_json::from_value(json!({"kind": "headless"})).unwrap();
    McpServer::open(
        h.clone(),
        &format!("mcp-{}", format.unwrap_or("none")),
        Config::new("claude", surface),
        surfaces::by_name("thin").unwrap(),
        Options {
            notify: NotifyMode::Spec,
            common_tools: true,
            log: None,
            capsule_format: format.map(str::to_owned),
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
    )["result"]
        .clone()
}

fn first_capsule(s: &mut McpServer) -> String {
    let r = call(s, 1, "query_capsules", json!({}));
    r["structuredContent"]["data"]["capsules"][0]["id"]
        .as_str()
        .unwrap()
        .to_owned()
}

#[test]
fn card_and_layered_lay_out_the_same_capsule() {
    let h = hub();
    let mut plain = served(&h, None);
    let id = first_capsule(&mut plain);
    let raw = call(&mut plain, 2, "query_capsule", json!({"capsule": id}));
    let item = &raw["structuredContent"]["data"]["detail"]["item"];
    assert_eq!(item["id"], id.as_str());

    let mut card = served(&h, Some("card"));
    let r = call(&mut card, 3, "query_capsule", json!({"capsule": id}));
    assert!(r.get("structuredContent").is_none(), "{r}");
    let text = r["content"][0]["text"].as_str().unwrap();
    assert!(text.starts_with(&format!("Capsule {id}")), "{text}");
    assert!(text.contains("Evidence:"), "{text}");
    let r = call(&mut card, 4, "query_capsules", json!({}));
    let text = r["content"][0]["text"].as_str().unwrap();
    assert!(text.contains(&id), "{text}");

    let mut layered = served(&h, Some("layered"));
    let r = call(&mut layered, 5, "query_capsule", json!({"capsule": id}));
    let v: Value = serde_json::from_str(r["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(v["summary"]["id"], id.as_str());
    assert_eq!(v["summary"]["importance"], item["importance"]);

    // json: the protocol reply, as without a format
    let mut json_fmt = served(&h, Some("json"));
    let r = call(&mut json_fmt, 6, "query_capsule", json!({"capsule": id}));
    assert_eq!(
        r["structuredContent"]["data"],
        raw["structuredContent"]["data"]
    );
}

#[test]
fn formatted_tools_declare_no_output_schema() {
    let h = hub();
    let mut card = served(&h, Some("card"));
    let list = rpc(&mut card, 1, "tools/list", json!({}));
    for t in list["result"]["tools"].as_array().unwrap() {
        let name = t["name"].as_str().unwrap();
        let formatted = surfaces::thin::FORMATTED_TOOLS.contains(&name);
        if formatted || name == "query_get" {
            assert_eq!(t.get("outputSchema").is_none(), formatted, "{name}");
        }
    }
    // an error stays the protocol error
    let r = call(&mut card, 2, "query_capsule", json!({"capsule": "nope"}));
    assert_eq!(r["isError"], true, "{r}");
}
