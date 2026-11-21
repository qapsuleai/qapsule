//! (R5B/T2) `qp mcp serve` end to end: stdio and Streamable HTTP over a real socket,
//! the local-only defaults (Origin, token, non-local address), agent principals only,
//! `render`, the measurement log, and `qp demo --mcp`.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use serde_json::{json, Value};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

const SMALL: [&str; 6] = ["--instruments", "2", "--days", "20", "--history", "10"];

fn qp() -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_qp"));
    c.current_dir(root());
    c
}

/// A stdio server; `ask` sends a request and returns its response (skipping
/// notifications).
struct Stdio1 {
    child: Child,
    stdin: std::process::ChildStdin,
    out: BufReader<std::process::ChildStdout>,
}

impl Stdio1 {
    fn start(args: &[&str]) -> Self {
        let mut child = qp()
            .args(["mcp", "serve"])
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let stdin = child.stdin.take().unwrap();
        let out = BufReader::new(child.stdout.take().unwrap());
        Self { child, stdin, out }
    }

    fn ask(&mut self, req: &Value) -> Value {
        writeln!(self.stdin, "{req}").unwrap();
        self.stdin.flush().unwrap();
        loop {
            let mut line = String::new();
            assert!(self.out.read_line(&mut line).unwrap() > 0, "server closed");
            let v: Value = serde_json::from_str(&line).unwrap();
            if v.get("id") == req.get("id") {
                return v;
            }
        }
    }

    fn finish(mut self) {
        drop(self.stdin);
        assert!(self.child.wait().unwrap().success());
    }
}

fn initialize(id: u64) -> Value {
    json!({"jsonrpc":"2.0","id":id,"method":"initialize","params":{
        "protocolVersion":"2025-11-25","capabilities":{},
        "clientInfo":{"name":"test","version":"0"}}})
}

fn tool(id: u64, name: &str, args: Value) -> Value {
    json!({"jsonrpc":"2.0","id":id,"method":"tools/call",
        "params":{"name":name,"arguments":args}})
}

#[test]
fn stdio_initialize_list_and_read_with_a_log() {
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("calls.jsonl");
    let mut s =
        Stdio1::start(&[&SMALL[..], &["--log", log.to_str().unwrap(), "--run", "t1"]].concat());
    let init = s.ask(&initialize(1));
    assert_eq!(init["result"]["protocolVersion"], "2025-11-25");
    let welcome = &init["result"]["_meta"]["dev.qapsule/welcome"];
    assert_eq!(welcome["type"], "welcome");
    assert_eq!(welcome["principal"], "claude");
    let list = s.ask(&json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}));
    let tools = list["result"]["tools"].as_array().unwrap();
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    for n in [
        "query_list",
        "op_add_study",
        "replay_step",
        "render",
        "notifications",
    ] {
        assert!(names.contains(&n), "missing {n}");
    }
    let q = tools.iter().find(|t| t["name"] == "query_list").unwrap();
    assert_eq!(q["annotations"]["readOnlyHint"], true);
    assert_eq!(q["annotations"]["destructiveHint"], false);
    let r = s.ask(&tool(3, "query_list", json!({"collection": "series"})));
    assert_eq!(r["result"]["isError"], false, "{r}");
    assert_eq!(r["result"]["structuredContent"]["type"], "result");
    let st = s.ask(&tool(4, "replay_status", json!({})));
    assert_eq!(
        st["result"]["structuredContent"]["data"]["replay"]["paused"],
        true
    );
    s.finish();
    let text = std::fs::read_to_string(&log).unwrap();
    let lines: Vec<Value> = text
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(lines.len(), 2, "{text}");
    assert_eq!(lines[0]["tool"], "query_list");
    assert_eq!(lines[0]["run"], "t1");
    assert_eq!(lines[0]["principal"], "claude");
    assert_eq!(lines[0]["ok"], true);
    assert_eq!(lines[1]["tool"], "replay_status");
}

#[test]
fn stdio_render_returns_a_png() {
    let mut s = Stdio1::start(&[
        "--init",
        "fixtures/protocol/init/thyao.json",
        "--principal",
        "scout",
    ]);
    let r = s.ask(&tool(
        1,
        "render",
        json!({"view": "main", "width": 300, "height": 200}),
    ));
    let c = &r["result"]["content"][0];
    assert_eq!(c["type"], "image", "{r}");
    assert_eq!(c["mimeType"], "image/png");
    assert!(c["data"].as_str().unwrap().starts_with("iVBORw0KGgo"));
    s.finish();
}

#[test]
fn a_human_principal_is_refused() {
    for (args, why) in [
        (vec!["--principal", "arkin"], "not an agent"),
        (vec!["--principal", "nobody"], "not a principal"),
        (vec!["--source", "live"], "unsupported"),
    ] {
        let out = qp()
            .args(["mcp", "serve"])
            .args(SMALL)
            .args(&args)
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(err.contains(why), "{args:?}: {err}");
    }
}

#[test]
fn a_non_local_address_needs_a_token() {
    let out = qp()
        .args(["mcp", "serve", "--http", "--listen", "0.0.0.0"])
        .args(SMALL)
        .env_remove("QP_MCP_TEST_TOKEN")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("--token-env"));
}

/// An HTTP server; killed on drop.
struct Http {
    child: Child,
    port: u16,
}

impl Drop for Http {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Http {
    fn start(args: &[&str], env: &[(&str, &str)]) -> Self {
        let mut c = qp();
        c.args(["mcp", "serve", "--http"]).args(SMALL).args(args);
        for (k, v) in env {
            c.env(k, v);
        }
        let mut child = c
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut err = BufReader::new(child.stderr.take().unwrap());
        let mut line = String::new();
        err.read_line(&mut line).unwrap();
        let v: Value = serde_json::from_str(&line).unwrap_or_else(|_| panic!("{line:?}"));
        let url = v["listening"].as_str().unwrap().to_owned();
        assert!(
            url.starts_with("http://127.0.0.1:") && url.ends_with("/mcp"),
            "{url}"
        );
        let port = url
            .trim_end_matches("/mcp")
            .rsplit(':')
            .next()
            .unwrap()
            .parse()
            .unwrap();
        std::thread::spawn(move || for _ in err.lines() {});
        Self { child, port }
    }
}

/// One HTTP exchange: (status, headers lower-cased, body).
fn http(
    port: u16,
    method: &str,
    headers: &[(&str, &str)],
    body: Option<&Value>,
) -> (u16, Vec<(String, String)>, String) {
    let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
    let body = body.map(Value::to_string).unwrap_or_default();
    let mut req = format!("{method} /mcp HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n");
    for (k, v) in headers {
        req.push_str(&format!("{k}: {v}\r\n"));
    }
    if !body.is_empty() {
        req.push_str("Content-Type: application/json\r\n");
        req.push_str("Accept: application/json, text/event-stream\r\n");
    }
    req.push_str(&format!("Content-Length: {}\r\n\r\n{body}", body.len()));
    s.write_all(req.as_bytes()).unwrap();
    let mut r = BufReader::new(s);
    let mut status = String::new();
    r.read_line(&mut status).unwrap();
    let code = status.split_whitespace().nth(1).unwrap().parse().unwrap();
    let mut hs = Vec::new();
    loop {
        let mut h = String::new();
        r.read_line(&mut h).unwrap();
        if h == "\r\n" || h.is_empty() {
            break;
        }
        let (k, v) = h.split_once(':').unwrap();
        hs.push((k.trim().to_ascii_lowercase(), v.trim().to_owned()));
    }
    let mut text = String::new();
    let _ = r.read_to_string(&mut text);
    (code, hs, text)
}

fn header<'a>(hs: &'a [(String, String)], k: &str) -> Option<&'a str> {
    hs.iter().find(|(x, _)| x == k).map(|(_, v)| v.as_str())
}

#[test]
fn http_initialize_list_and_read_on_a_real_socket() {
    let srv = Http::start(&[], &[]);
    let (code, hs, body) = http(srv.port, "POST", &[], Some(&initialize(1)));
    assert_eq!(code, 200, "{body}");
    let sid = header(&hs, "mcp-session-id")
        .expect("session id")
        .to_owned();
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["result"]["protocolVersion"], "2025-11-25");
    let s = [("Mcp-Session-Id", sid.as_str())];
    let (code, _, body) = http(
        srv.port,
        "POST",
        &s,
        Some(&json!({"jsonrpc":"2.0","method":"notifications/initialized"})),
    );
    assert_eq!(code, 202, "{body}");
    let (code, _, body) = http(
        srv.port,
        "POST",
        &s,
        Some(&json!({"jsonrpc":"2.0","id":2,"method":"tools/list"})),
    );
    assert_eq!(code, 200);
    let v: Value = serde_json::from_str(&body).unwrap();
    assert!(v["result"]["tools"].as_array().unwrap().len() > 40);
    let (code, _, body) = http(
        srv.port,
        "POST",
        &s,
        Some(&tool(3, "query_list", json!({"collection": "series"}))),
    );
    assert_eq!(code, 200);
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["result"]["structuredContent"]["type"], "result", "{v}");
    // A modern, stateless request (no session) works too, as another agent principal.
    let modern = json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{
        "name":"query_describe","arguments":{},
        "_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28"}}});
    let (code, _, body) = http(
        srv.port,
        "POST",
        &[
            ("X-Qp-Principal", "claude-guest"),
            ("MCP-Protocol-Version", "2026-07-28"),
        ],
        Some(&modern),
    );
    assert_eq!(code, 200, "{body}");
    assert!(body.contains("claude-guest"), "{body}");
    // A human principal cannot be picked; an unknown session is 404.
    let (code, _, _) = http(
        srv.port,
        "POST",
        &[("X-Qp-Principal", "arkin")],
        Some(&modern),
    );
    assert_eq!(code, 403);
    let (code, _, _) = http(
        srv.port,
        "POST",
        &[("Mcp-Session-Id", "nope")],
        Some(&modern),
    );
    assert_eq!(code, 404);
    // Header and body versions must agree.
    let (code, _, _) = http(
        srv.port,
        "POST",
        &[("MCP-Protocol-Version", "2025-06-18")],
        Some(&modern),
    );
    assert_eq!(code, 400);
    // The session ends with DELETE.
    let (code, _, _) = http(srv.port, "DELETE", &s, None);
    assert_eq!(code, 204);
}

#[test]
fn http_refuses_a_foreign_origin_and_a_foreign_host() {
    let srv = Http::start(&[], &[]);
    let ping = json!({"jsonrpc":"2.0","id":1,"method":"ping"});
    let ok_origin = format!("http://localhost:{}", srv.port);
    for (origin, want) in [
        ("http://evil.example", 403),
        ("http://127.0.0.1.evil.example", 403),
        ("http://localhost:1", 403),
        ("http://127.0.0.1", 200),
        (ok_origin.as_str(), 200),
    ] {
        let (code, _, _) = http(srv.port, "POST", &[("Origin", origin)], Some(&ping));
        assert_eq!(code, want, "{origin}");
    }
    // A request naming another Host (DNS rebinding) is refused.
    let mut s = TcpStream::connect(("127.0.0.1", srv.port)).unwrap();
    let b = ping.to_string();
    write!(
        s,
        "POST /mcp HTTP/1.1\r\nHost: evil.example\r\nContent-Type: application/json\r\n\
Content-Length: {}\r\n\r\n{b}",
        b.len()
    )
    .unwrap();
    let mut resp = String::new();
    let _ = s.read_to_string(&mut resp);
    assert!(resp.starts_with("HTTP/1.1 403"), "{resp}");
}

#[test]
fn http_with_a_token_needs_the_right_bearer() {
    let token = "0123456789abcdef0123456789abcdef";
    let srv = Http::start(
        &["--token-env", "QP_MCP_TEST_TOKEN"],
        &[("QP_MCP_TEST_TOKEN", token)],
    );
    let ping = json!({"jsonrpc":"2.0","id":1,"method":"ping"});
    let (code, hs, _) = http(srv.port, "POST", &[], Some(&ping));
    assert_eq!(code, 401);
    assert_eq!(header(&hs, "www-authenticate"), Some("Bearer"));
    let (code, _, _) = http(
        srv.port,
        "POST",
        &[("Authorization", "Bearer 0123456789abcdef0123456789abcdeX")],
        Some(&ping),
    );
    assert_eq!(code, 401);
    let good = format!("Bearer {token}");
    let (code, _, body) = http(
        srv.port,
        "POST",
        &[("Authorization", good.as_str())],
        Some(&ping),
    );
    assert_eq!(code, 200, "{body}");
}

#[test]
fn http_listen_streams_a_resource_update() {
    let srv = Http::start(&[], &[]);
    let meta = json!({"io.modelcontextprotocol/protocolVersion":"2026-07-28"});
    // `claude` subscribes to proposals through the thin surface.
    let sub = json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{
        "name":"subscribe","arguments":{"id":"s1","topic":{"kind":"proposals"}},
        "_meta": meta}});
    let (code, _, body) = http(srv.port, "POST", &[], Some(&sub));
    assert_eq!(code, 200, "{body}");
    // Open the listen stream.
    let mut s = TcpStream::connect(("127.0.0.1", srv.port)).unwrap();
    let listen = json!({"jsonrpc":"2.0","id":2,"method":"subscriptions/listen",
        "params":{"filter":{"resourceSubscriptions":["qapsule://notifications"]},"_meta":meta}})
    .to_string();
    write!(
        s,
        "POST /mcp HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nContent-Type: application/json\r\n\
Accept: application/json, text/event-stream\r\nContent-Length: {}\r\n\r\n{listen}",
        srv.port,
        listen.len()
    )
    .unwrap();
    let mut r = BufReader::new(s);
    let mut data = Vec::new();
    let step = || {
        // `claude-guest` (a proposer) writes: a proposal.
        let st = json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{
            "name":"op_add_study",
            "arguments":{"study":"sma","inputs":["syn01"],"params":{"period":5}},
            "_meta": meta}});
        let (code, _, body) = http(
            srv.port,
            "POST",
            &[("X-Qp-Principal", "claude-guest")],
            Some(&st),
        );
        assert_eq!(code, 200, "{body}");
    };
    let mut stepped = false;
    while data.len() < 2 {
        let mut l = String::new();
        assert!(r.read_line(&mut l).unwrap() > 0, "stream closed");
        if let Some(j) = l.strip_prefix("data: ") {
            data.push(serde_json::from_str::<Value>(j.trim()).unwrap());
            if !stepped {
                stepped = true;
                step();
            }
        }
    }
    assert_eq!(
        data[0]["method"],
        "notifications/subscriptions/acknowledged"
    );
    assert_eq!(data[1]["method"], "notifications/resources/updated");
    assert_eq!(data[1]["params"]["uri"], "qapsule://notifications");
    // The notifications are read from the resource.
    let read = json!({"jsonrpc":"2.0","id":4,"method":"resources/read",
        "params":{"uri":"qapsule://notifications","_meta":meta}});
    let (_, _, body) = http(srv.port, "POST", &[], Some(&read));
    let v: Value = serde_json::from_str(&body).unwrap();
    let text: Value =
        serde_json::from_str(v["result"]["contents"][0]["text"].as_str().unwrap()).unwrap();
    assert!(
        !text["notifications"].as_array().unwrap().is_empty(),
        "{text}"
    );
}

#[test]
fn demo_serves_the_same_session_at_mcp() {
    let mut child = qp()
        .args([
            "demo",
            "--synthetic",
            "--paused",
            "--mcp",
            "--port",
            "0",
            "--instruments",
            "2",
            "--days",
            "20",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    let mut out = BufReader::new(child.stdout.take().unwrap());
    let mut line = String::new();
    out.read_line(&mut line).unwrap();
    let port: u16 = line
        .trim()
        .trim_end_matches('/')
        .rsplit(':')
        .next()
        .and_then(|p| p.parse().ok())
        .unwrap_or_else(|| panic!("no port in {line:?}"));
    std::thread::spawn(move || for _ in out.lines() {});
    let d = json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{
        "name":"query_describe","arguments":{}}});
    let (code, _, body) = http(port, "POST", &[], Some(&d));
    assert_eq!(code, 200, "{body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["result"]["structuredContent"]["type"], "result", "{v}");
    assert!(body.contains("claude"), "{body}");
    let (code, _, _) = http(port, "POST", &[("Origin", "http://evil.example")], Some(&d));
    assert_eq!(code, 403);
    let _ = child.kill();
    let _ = child.wait();
}
