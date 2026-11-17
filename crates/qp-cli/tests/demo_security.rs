//! R4/T5 security review: `qp demo` serves only its own Host, and `/send` needs the
//! connection's secret, a same-origin `Origin` and a JSON body.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};

struct Demo {
    child: Child,
    port: u16,
}

impl Drop for Demo {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

fn start() -> Demo {
    start_with(&[])
}

fn start_with(extra: &[&str]) -> Demo {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let mut child = Command::new(env!("CARGO_BIN_EXE_qp"))
        .args([
            "demo",
            "--synthetic",
            "--paused",
            "--port",
            "0",
            "--instruments",
            "3",
            "--days",
            "40",
            "--web",
        ])
        .arg(format!("{root}/shells/web"))
        .args(extra)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    let mut out = BufReader::new(child.stdout.take().unwrap());
    let mut line = String::new();
    out.read_line(&mut line).unwrap();
    let port = line
        .trim()
        .trim_end_matches('/')
        .rsplit(':')
        .next()
        .and_then(|p| p.parse().ok())
        .unwrap_or_else(|| panic!("no port in {line:?}"));
    std::thread::spawn(move || for _ in out.lines() {});
    Demo { child, port }
}

/// Opens the event stream and returns (stream kept open, conn, token).
fn events(port: u16) -> (TcpStream, String, String) {
    let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
    write!(
        s,
        "GET /events?principal=arkin HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n"
    )
    .unwrap();
    let mut r = BufReader::new(s.try_clone().unwrap());
    loop {
        let mut l = String::new();
        r.read_line(&mut l).unwrap();
        if let Some(j) = l.strip_prefix("data: ") {
            let v: serde_json::Value = serde_json::from_str(j.trim()).unwrap();
            let st = &v["start"];
            return (
                s,
                st["conn"].as_str().unwrap().to_owned(),
                st["token"].as_str().unwrap().to_owned(),
            );
        }
    }
}

fn request(port: u16, head: &str, body: &str) -> String {
    let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
    write!(s, "{head}Content-Length: {}\r\n\r\n{body}", body.len()).unwrap();
    let mut out = String::new();
    let _ = s.read_to_string(&mut out);
    out.lines().next().unwrap_or_default().to_owned()
}

fn send(port: u16, conn: &str, host: &str, origin: &str, ctype: &str, token: &str) -> String {
    let body =
        r#"[{"type":"hello","protocol":"qp/0","principal":"arkin","surface":{"kind":"phone"}}]"#;
    request(
        port,
        &format!(
            "POST /send?conn={conn} HTTP/1.1\r\nHost: {host}\r\nOrigin: {origin}\r\n\
Content-Type: {ctype}\r\nX-Qp-Token: {token}\r\n"
        ),
        body,
    )
}

#[test]
fn demo_rejects_foreign_hosts_origins_and_secrets() {
    let d = start();
    let p = d.port;
    let host = format!("127.0.0.1:{p}");
    let origin = format!("http://{host}");
    let (_keep, conn, token) = events(p);
    assert_eq!(token.len(), 32, "128-bit secret");
    let (_keep2, conn2, token2) = events(p);
    assert_ne!(token, token2);

    // A static file with a foreign Host (DNS rebinding) is refused; with ours, served.
    let get = |h: &str| request(p, &format!("GET /index.html HTTP/1.1\r\nHost: {h}\r\n"), "");
    assert!(get("evil.example:80").contains("403"));
    assert!(get(&host).contains("200"));
    assert!(get(&format!("localhost:{p}")).contains("200"));

    let json = "application/json";
    // Wrong secret, another connection's secret, no secret.
    assert!(send(p, &conn, &host, &origin, json, "00").contains("403"));
    assert!(send(p, &conn, &host, &origin, json, &token2).contains("403"));
    assert!(send(p, &conn2, &host, &origin, json, &token).contains("403"));
    // Wrong Host, wrong Origin, wrong content type.
    assert!(send(p, &conn, "evil.example", &origin, json, &token).contains("403"));
    assert!(send(p, &conn, &host, "http://evil.example", json, &token).contains("403"));
    assert!(send(p, &conn, &host, &origin, "text/plain", &token).contains("415"));
    // Right everything.
    assert!(send(p, &conn, &host, &origin, json, &token).contains("204"));
}

#[test]
fn https_host_is_accepted_only_when_named() {
    // (RP3) Without `--https-host`, a proxy name and an https origin are refused.
    let d = start();
    let p = d.port;
    let (_keep, conn, token) = events(p);
    let json = "application/json";
    let get = |h: &str| request(p, &format!("GET /index.html HTTP/1.1\r\nHost: {h}\r\n"), "");
    assert!(get("demo.example.ts.net").contains("403"));
    let host = format!("127.0.0.1:{p}");
    assert!(send(p, &conn, &host, &format!("https://{host}"), json, &token).contains("403"));
    drop(d);

    // With it: that name (and NAME:443) as Host, `https://NAME` as Origin; nothing else.
    let d = start_with(&["--https-host", "demo.example.ts.net"]);
    let p = d.port;
    let (_keep, conn, token) = events(p);
    let get = |h: &str| request(p, &format!("GET /index.html HTTP/1.1\r\nHost: {h}\r\n"), "");
    assert!(get("demo.example.ts.net").contains("200"));
    assert!(get("demo.example.ts.net:443").contains("200"));
    assert!(get("evil.example").contains("403"));
    let name = "demo.example.ts.net";
    assert!(send(p, &conn, name, "https://demo.example.ts.net", json, &token).contains("204"));
    assert!(send(p, &conn, name, "http://demo.example.ts.net", json, &token).contains("403"));
    assert!(send(p, &conn, name, "https://evil.example", json, &token).contains("403"));
    assert!(send(p, &conn, name, "https://demo.example.ts.net", json, "00").contains("403"));
    // The local origin keeps working as before.
    let host = format!("127.0.0.1:{p}");
    assert!(send(p, &conn, &host, &format!("http://{host}"), json, &token).contains("204"));
}
