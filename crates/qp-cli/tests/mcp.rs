//! `qp mcp` over stdio and `qp conform --transport mcp`, end to end.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command as Proc, Stdio};

use assert_cmd::Command;
use serde_json::{json, Value};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn mcp_stdio_smoke() {
    let bin = assert_cmd::cargo::cargo_bin("qp");
    let mut child = Proc::new(bin)
        .current_dir(root())
        .args([
            "mcp",
            "--init",
            "fixtures/protocol/init/thyao.json",
            "--principal",
            "arkin",
            "--surface",
            "desktop",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    let mut ask = |req: Value| -> Value {
        writeln!(stdin, "{req}").unwrap();
        stdin.flush().unwrap();
        loop {
            let mut line = String::new();
            assert!(
                stdout.read_line(&mut line).unwrap() > 0,
                "qp mcp closed stdout"
            );
            let v: Value = serde_json::from_str(&line).unwrap();
            if v.get("id") == req.get("id") {
                return v;
            }
        }
    };
    let init = ask(
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{
        "protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"smoke","version":"0"}}}),
    );
    assert_eq!(init["result"]["protocolVersion"], "2025-11-25");
    assert_eq!(
        init["result"]["_meta"]["dev.qapsule/welcome"]["type"],
        "welcome"
    );
    let list = ask(json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}));
    let names: Vec<&str> = list["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"op_add_study") && names.contains(&"query_get"));
    let add = ask(
        json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{
        "name":"op_add_study","arguments":{"id":"sma9","study":"sma","inputs":["px"],
        "params":{"period":9}}}}),
    );
    let r = &add["result"];
    assert_eq!(r["isError"], false, "{add}");
    assert_eq!(r["structuredContent"]["type"], "ack");
    assert_eq!(r["structuredContent"]["status"], "applied");
    let get = ask(
        json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{
        "name":"query_get","arguments":{"id":"sma9"}}}),
    );
    let sc = &get["result"]["structuredContent"];
    assert_eq!(sc["type"], "result", "{get}");
    assert!(sc.to_string().contains("\"sma\""), "{sc}");
    drop(stdin);
    assert!(child.wait().unwrap().success());
}

#[test]
fn conform_over_mcp() {
    let out = Command::cargo_bin("qp")
        .unwrap()
        .current_dir(root())
        .args([
            "conform",
            "fixtures/protocol/conformance",
            "crates/qp-session/tests/transcripts",
            "--transport",
            "mcp",
        ])
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{text}");
    assert!(text.contains(" 0 failed (mcp)"), "{text}");
}
