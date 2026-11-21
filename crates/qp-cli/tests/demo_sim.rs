//! (RP4/T4) `qp demo --sim` and `scripts/demo.sh --sim`: refused with `--lan`,
//! `--https-host` or another `--bind` (real data does not go to the local network;
//! VERIFY 17); on 127.0.0.1 the simulated demo serves the live page and its stats.
//! A temporary made-up trade cache; no network beyond 127.0.0.1.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const CONTROL: &str = "sim-test-control-secret-0123456789";

struct Demo {
    child: Child,
    port: u16,
}

impl Drop for Demo {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Made-up trades of two symbols on 2026-11-17 and 2026-11-18 (one every 61 s).
fn write_cache(root: &std::path::Path) {
    for (i, sym) in ["TEST1", "TEST2"].iter().enumerate() {
        let dir = root.join("fixtures/local/trades").join(sym);
        std::fs::create_dir_all(&dir).unwrap();
        let f = std::fs::File::create(dir.join("2026-11-17_2026-11-18.csv.gz")).unwrap();
        let mut gz = flate2::write::GzEncoder::new(f, flate2::Compression::fast());
        writeln!(
            gz,
            "symbol,date_time,timestamp_ns,bidask,mx_id,price,quantity"
        )
        .unwrap();
        let mut k = 0u64;
        for date in ["2026-11-17", "2026-11-18"] {
            let mut sec = 10 * 3600 + 7 + i as i64;
            while sec < 18 * 3600 {
                let price = 30.0 + i as f64 + ((k * 7) % 13) as f64 * 0.05;
                writeln!(
                    gz,
                    "{sym},{date}T{:02}:{:02}:{:02}.000+0300,{},98,{},{price:.2},{}",
                    sec / 3600,
                    sec / 60 % 60,
                    sec % 60,
                    sec * 1_000_000_000 + 250_000_000,
                    100_000 + k,
                    1 + k % 40
                )
                .unwrap();
                k += 1;
                sec += 61;
            }
        }
        gz.finish().unwrap();
    }
}

#[allow(clippy::zombie_processes)] // Drop kills it; the error path waits.
fn start(root: &std::path::Path, extra: &[&str]) -> Result<Demo, String> {
    let web = concat!(env!("CARGO_MANIFEST_DIR"), "/../../shells/web");
    let mut child = Command::new(env!("CARGO_BIN_EXE_qp"))
        .current_dir(root)
        .env_clear()
        .env("PATH", std::env::var("PATH").unwrap_or_default())
        .env("QP_LIVE_CONTROL", CONTROL)
        .args(["demo", "--sim", "--port", "0", "--interval", "5m"])
        .args(["--symbols", "TEST1,TEST2", "--start", "2026-11-18"])
        .args(["--history", "40", "--sim-speed", "max"])
        .args(["--root", ".", "--web", web])
        .args(extra)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut out = BufReader::new(child.stdout.take().unwrap());
    loop {
        let mut line = String::new();
        if out.read_line(&mut line).unwrap() == 0 {
            let mut err = String::new();
            let _ = child.stderr.take().unwrap().read_to_string(&mut err);
            let _ = child.wait();
            return Err(err);
        }
        if let Some(u) = line.trim().strip_prefix("qp demo: ") {
            let port = u
                .trim_end_matches('/')
                .rsplit(':')
                .next()
                .and_then(|p| p.parse().ok())
                .unwrap();
            std::thread::spawn(move || for _ in out.lines() {});
            return Ok(Demo { child, port });
        }
    }
}

fn stats(port: u16) -> serde_json::Value {
    let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
    write!(
        s,
        "GET /live/stats HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nX-Qp-Live-Control: {CONTROL}\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    let mut r = String::new();
    s.read_to_string(&mut r).unwrap();
    let body = r.split("\r\n\r\n").nth(1).unwrap_or("");
    serde_json::from_str(body).unwrap_or(serde_json::Value::Null)
}

#[test]
fn sim_demo_refuses_the_local_network_and_serves_on_loopback() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    write_cache(root);

    // Real data stays on this machine (VERIFY 17): refused before anything is read.
    for extra in [
        &["--lan"][..],
        &["--https-host", "demo.example.ts.net"][..],
        &["--bind", "0.0.0.0"][..],
    ] {
        let e = start(root, extra).err().expect("refused");
        assert!(e.contains("--sim") && e.contains("--lan"), "{extra:?}: {e}");
    }
    let script = concat!(env!("CARGO_MANIFEST_DIR"), "/../../scripts/demo.sh");
    for extra in [&["--lan"][..], &["--https-host", "x.ts.net"][..]] {
        let out = Command::new("bash")
            .arg(script)
            .arg("--sim")
            .args(extra)
            .args(["--start", "2026-11-18"])
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(2), "{extra:?}");
        assert!(String::from_utf8_lossy(&out.stderr).contains("refused"));
    }

    // On 127.0.0.1: the simulated day goes through the live runtime.
    let demo = start(root, &[]).expect("sim demo starts");
    let deadline = Instant::now() + Duration::from_secs(60);
    let v = loop {
        let v = stats(demo.port);
        if v["messages"].as_u64().unwrap_or(0) >= 900 && v["link"] == "live" {
            break v;
        }
        assert!(Instant::now() < deadline, "sim did not run: {v}");
        std::thread::sleep(Duration::from_millis(100));
    };
    assert_eq!(v["feeds"], 2);
    assert_eq!(v["events_rejected"], 0);
    assert_eq!(v["bindings_not_ready"], 0);
    assert!(v["events"].as_u64().unwrap() >= 900);

    // The page's start frame says simulation.
    let mut s = TcpStream::connect(("127.0.0.1", demo.port)).unwrap();
    write!(
        s,
        "GET /events?principal=arkin HTTP/1.1\r\nHost: 127.0.0.1:{}\r\n\r\n",
        demo.port
    )
    .unwrap();
    let mut r = BufReader::new(s);
    let frame = loop {
        let mut l = String::new();
        r.read_line(&mut l).unwrap();
        if let Some(j) = l.strip_prefix("data: ") {
            break serde_json::from_str::<serde_json::Value>(j.trim()).unwrap();
        }
    };
    assert_eq!(frame["start"]["info"]["sim"], true);
    assert_eq!(frame["start"]["info"]["lan"], false);
}
