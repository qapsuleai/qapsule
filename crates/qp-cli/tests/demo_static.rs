//! RP3/M13: `qp demo` serves the pre-compressed package by `Accept-Encoding`
//! (`Content-Encoding`, the compressed file's bytes, `application/wasm`, `Vary`), the file
//! itself without it, never a compressed copy older than its source, and answers a matching
//! `If-None-Match` with `304`. The Host rule is unchanged.

use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, SystemTime};

struct Demo {
    child: Child,
    port: u16,
}

impl Drop for Demo {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

fn start(web: &Path) -> Demo {
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
        .arg(web)
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

struct Res {
    status: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Res {
    fn header(&self, k: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(k))
            .map(|(_, v)| v.as_str())
    }
}

fn get(port: u16, path: &str, extra: &str) -> Res {
    let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
    write!(
        s,
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n{extra}\r\n"
    )
    .unwrap();
    let mut raw = Vec::new();
    s.read_to_end(&mut raw).unwrap();
    let split = raw.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
    let head = String::from_utf8(raw[..split].to_vec()).unwrap();
    let mut lines = head.split("\r\n");
    let status = lines.next().unwrap().to_owned();
    let headers = lines
        .filter_map(|l| l.split_once(':'))
        .map(|(k, v)| (k.trim().to_owned(), v.trim().to_owned()))
        .collect();
    Res {
        status,
        headers,
        body: raw[split + 4..].to_vec(),
    }
}

fn set_age(p: &Path, secs_ago: u64) {
    let t = SystemTime::now() - Duration::from_secs(secs_ago);
    File::options()
        .write(true)
        .open(p)
        .unwrap()
        .set_modified(t)
        .unwrap();
}

#[test]
fn demo_serves_precompressed_files_by_accept_encoding() {
    let dir = tempfile::tempdir().unwrap();
    let web = dir.path();
    fs::create_dir_all(web.join("pkg")).unwrap();
    fs::write(web.join("index.html"), "<!doctype html><p>x</p>").unwrap();
    let raw = b"\0asm\x01\0\0\0 raw module bytes".to_vec();
    let gz = b"\x1f\x8b gzip bytes of the module".to_vec();
    let br = b"brotli bytes".to_vec();
    let wasm = web.join("pkg/qp_wasm_bg.wasm");
    fs::write(&wasm, &raw).unwrap();
    fs::write(web.join("pkg/qp_wasm_bg.wasm.gz"), &gz).unwrap();
    set_age(&wasm, 60);
    let d = start(web);
    let p = d.port;
    let path = "/pkg/qp_wasm_bg.wasm";

    // gzip accepted (WebKit's header): the .gz bytes, gzip, the source's type.
    let r = get(p, path, "Accept-Encoding: gzip, deflate\r\n");
    assert!(r.status.contains("200"), "{}", r.status);
    assert_eq!(r.header("Content-Encoding"), Some("gzip"));
    assert_eq!(r.header("Content-Type"), Some("application/wasm"));
    assert_eq!(r.header("Vary"), Some("Accept-Encoding"));
    assert_eq!(
        r.header("Content-Length"),
        Some(gz.len().to_string().as_str())
    );
    assert!(r.header("Content-Security-Policy").is_some());
    assert_eq!(r.body, gz);

    // No Accept-Encoding, identity only, or gzip refused: the file itself.
    for extra in [
        "",
        "Accept-Encoding: identity\r\n",
        "Accept-Encoding: gzip;q=0\r\n",
    ] {
        let r = get(p, path, extra);
        assert!(r.status.contains("200"));
        assert_eq!(r.header("Content-Encoding"), None, "{extra:?}");
        assert_eq!(r.header("Content-Type"), Some("application/wasm"));
        assert_eq!(r.body, raw, "{extra:?}");
    }

    // Revalidation: the same representation's ETag gives 304 without a body; the ETag of
    // another encoding does not.
    let r = get(p, path, "Accept-Encoding: gzip\r\n");
    let etag = r.header("ETag").unwrap().to_owned();
    assert_eq!(r.header("Cache-Control"), Some("no-cache"));
    let r304 = get(
        p,
        path,
        &format!("Accept-Encoding: gzip\r\nIf-None-Match: {etag}\r\n"),
    );
    assert!(r304.status.contains("304"), "{}", r304.status);
    assert!(r304.body.is_empty());
    assert_eq!(r304.header("ETag"), Some(etag.as_str()));
    let other = get(p, path, &format!("If-None-Match: {etag}\r\n"));
    assert!(other.status.contains("200"));
    assert_eq!(other.body, raw);

    // brotli, when present and accepted, wins over gzip (Chromium on localhost).
    fs::write(web.join("pkg/qp_wasm_bg.wasm.br"), &br).unwrap();
    let r = get(p, path, "Accept-Encoding: gzip, deflate, br, zstd\r\n");
    assert_eq!(r.header("Content-Encoding"), Some("br"));
    assert_eq!(r.body, br);
    let r = get(p, path, "Accept-Encoding: gzip, deflate\r\n");
    assert_eq!(r.header("Content-Encoding"), Some("gzip"));

    // A compressed copy older than its source (rebuilt, not re-compressed) is never served.
    set_age(&web.join("pkg/qp_wasm_bg.wasm.gz"), 3600);
    set_age(&web.join("pkg/qp_wasm_bg.wasm.br"), 3600);
    let r = get(p, path, "Accept-Encoding: gzip, deflate, br\r\n");
    assert_eq!(r.header("Content-Encoding"), None);
    assert_eq!(r.body, raw);

    // The Host rule still applies to static files; a missing file is 404 whatever is asked.
    let mut s = TcpStream::connect(("127.0.0.1", p)).unwrap();
    write!(
        s,
        "GET {path} HTTP/1.1\r\nHost: evil.example\r\nAccept-Encoding: gzip\r\n\r\n"
    )
    .unwrap();
    let mut out = String::new();
    let _ = s.read_to_string(&mut out);
    assert!(out.starts_with("HTTP/1.1 403"), "{out}");
    assert!(get(p, "/pkg/missing.wasm", "Accept-Encoding: gzip\r\n")
        .status
        .contains("404"));
}
