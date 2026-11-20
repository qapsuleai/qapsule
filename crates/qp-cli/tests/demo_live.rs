//! R6/T7: `qp demo --live` end to end without the network — the loopback mock broker and
//! token service (`qp-live` `mock`), history and repair bars from local files
//! (`--live-bars`). The page's start frame says `live`; `/live/stats` answers only with
//! counts; `/live/drop` closes the one connection (DISCONNECT) and opens it again (never
//! two at once), the feeds go stale and are repaired from the files; `/live/quit` closes
//! the connection and ends the process. `--lan` is refused (VERIFY 17).

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use qp_live::mock::{BrokerScript, MockBroker, MockTokenService};
use qp_schema::{Bar, Series, SeriesData, SeriesSource};

const CONTROL: &str = "0123456789abcdef0123456789abcdef";

struct Demo {
    child: Child,
    port: u16,
}

impl Drop for Demo {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

/// `n` daily bars on XIST trading days from 40 days ago (the history window is counted
/// back from now).
fn bars(n: usize, base: f64) -> Vec<Bar> {
    let cal = qp_calendar::ExchangeCalendar::new(&qp_calendar::builtin("XIST").unwrap()).unwrap();
    let mut out = Vec::new();
    let now_s = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let mut day = i64::try_from(now_s / 86_400).unwrap() - 40;
    while out.len() < n {
        if let Some(s) = cal.sessions_on(day).first() {
            #[allow(clippy::cast_precision_loss)]
            let p = base + (out.len() as f64 * 0.3).sin();
            out.push(Bar {
                t: s.open,
                o: p,
                h: p + 1.0,
                l: p - 1.0,
                c: p + 0.2,
                v: 100.0,
            });
        }
        day += 1;
    }
    out
}

fn write_bars(dir: &std::path::Path, sym: &str, b: Vec<Bar>) {
    let s = Series {
        id: sym.to_lowercase(),
        instrument: None,
        interval: Some("1d".to_owned()),
        source: SeriesSource::Inline {},
        data: Some(SeriesData::Bars { bars: b }),
    };
    std::fs::write(
        dir.join(format!("{sym}.1day.unadj.json")),
        serde_json::to_string(&s).unwrap(),
    )
    .unwrap();
}

fn http(port: u16, req: &str) -> (u16, String) {
    let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
    s.set_read_timeout(Some(Duration::from_secs(15))).unwrap();
    s.write_all(req.as_bytes()).unwrap();
    let mut out = String::new();
    let _ = s.read_to_string(&mut out);
    let code = out
        .split_whitespace()
        .nth(1)
        .and_then(|c| c.parse().ok())
        .unwrap_or(0);
    let body = out.split("\r\n\r\n").nth(1).unwrap_or("").to_owned();
    (code, body)
}

fn stats(port: u16) -> serde_json::Value {
    let (code, body) = http(
        port,
        &format!("GET /live/stats HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n"),
    );
    assert_eq!(code, 200, "{body}");
    serde_json::from_str(&body).unwrap()
}

fn post(port: u16, path: &str, secret: Option<&str>) -> (u16, String) {
    let h = secret.map_or(String::new(), |s| format!("X-Qp-Live-Control: {s}\r\n"));
    http(
        port,
        &format!("POST {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n{h}Content-Length: 0\r\n\r\n"),
    )
}

fn wait(port: u16, what: &str, f: impl Fn(&serde_json::Value) -> bool) -> serde_json::Value {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let v = stats(port);
        if f(&v) {
            return v;
        }
        assert!(Instant::now() < deadline, "timeout waiting for {what}: {v}");
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[allow(clippy::zombie_processes)] // Drop kills it; the error path waits.
fn start(
    tokens: &MockTokenService,
    broker: &MockBroker,
    root: &std::path::Path,
    extra: &[&str],
) -> Result<Demo, String> {
    let web = concat!(env!("CARGO_MANIFEST_DIR"), "/../../shells/web");
    let mut child = Command::new(env!("CARGO_BIN_EXE_qp"))
        .current_dir(root)
        .env_clear()
        .env("PATH", std::env::var("PATH").unwrap_or_default())
        .env("MATRIKS_TOKEN_SERVICE_URL", tokens.url())
        .env("MATRIKS_API_USERNAME", qp_live::mock::FAKE_USERNAME)
        .env("MATRIKS_API_PASSWORD", qp_live::mock::FAKE_PASSWORD)
        .env("MATRIKS_HARDWARE_ID", "qapsule-test")
        .env("MATRIKS_SERVICE_TYPE_CODE", "H")
        .env("MATRIKS_MQTT_MODE", "delayed")
        .env("MATRIKS_MQTT_DL_HOST", broker.url("market"))
        .env("QP_LIVE_CONTROL", CONTROL)
        .args(["demo", "--live", "--port", "0", "--interval", "1d"])
        .args([
            "--history",
            "20",
            "--symbols",
            "AAA,BBB",
            "--live-bars",
            "bars",
        ])
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

#[test]
fn live_demo_with_mock_broker_drops_repairs_and_quits() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("bars")).unwrap();
    std::fs::create_dir_all(root.join("fixtures/local")).unwrap();
    write_bars(&root.join("bars"), "AAA", bars(40, 100.0));
    write_bars(&root.join("bars"), "BBB", bars(40, 50.0));
    let tokens = MockTokenService::start_default(3600);
    let broker = MockBroker::start(BrokerScript::default());

    // Real data stays on this machine.
    let e = start(&tokens, &broker, root, &["--lan"])
        .err()
        .expect("--lan is refused");
    assert!(e.contains("--lan"), "{e}");

    let demo = start(&tokens, &broker, root, &[]).expect("live demo starts");
    let port = demo.port;
    let v = wait(port, "live", |v| v["link"] == "live");
    assert_eq!(v["feeds"], 2);
    assert_eq!(v["delayed"], true);
    assert_eq!(v["bindings_not_ready"], 0);

    // The page's start frame says live.
    let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
    write!(
        s,
        "GET /events?principal=arkin HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n"
    )
    .unwrap();
    let mut r = BufReader::new(s.try_clone().unwrap());
    let start_frame = loop {
        let mut l = String::new();
        r.read_line(&mut l).unwrap();
        if let Some(j) = l.strip_prefix("data: ") {
            break serde_json::from_str::<serde_json::Value>(j.trim()).unwrap();
        }
    };
    assert_eq!(start_frame["start"]["info"]["live"], true);
    assert_eq!(start_frame["start"]["info"]["symbols"], 2);
    drop(r);
    drop(s);

    // Control needs the secret.
    assert_eq!(post(port, "/live/drop?secs=1", None).0, 403);
    assert_eq!(
        post(port, "/live/drop?secs=1", Some("wrong-secret-wrong")).0,
        403
    );
    let (code, _) = post(port, "/live/drop?secs=1", Some(CONTROL));
    assert_eq!(code, 200);
    let v = wait(port, "repaired", |v| {
        v["link"] == "live" && v["repairs"].as_array().is_some_and(|a| !a.is_empty())
    });
    assert_eq!(v["bindings_not_ready"], 0);
    assert_eq!(v["capsules"]["born_or_changed_while_stale"], 0);
    assert!(v["repairs"][0]["repair_ms"].as_u64().is_some());
    let st = broker.stats();
    assert_eq!(st.max_concurrent, 1, "never two connections at once");
    assert!(st.connections_total >= 2 && st.disconnects >= 1);

    // Quit: DISCONNECT, then the process ends.
    let (code, _) = post(port, "/live/quit", Some(CONTROL));
    assert_eq!(code, 200);
    let mut demo = demo;
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        if let Some(status) = demo.child.try_wait().unwrap() {
            assert!(status.success());
            break;
        }
        assert!(Instant::now() < deadline, "qp demo did not end");
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(broker.wait_until(Duration::from_secs(5), |s| s.current == 0));
    assert!(broker.stats().disconnects >= 2);
    // The raw log is under the root's fixtures/local/.
    let raw: Vec<_> = std::fs::read_dir(root.join("fixtures/local/live/raw"))
        .unwrap()
        .collect();
    assert_eq!(raw.len(), 1);
}

/// Opens a `mode=replica` event stream; returns its start frame and a channel of the
/// frames after it (one JSON text per frame).
fn replica_stream(port: u16) -> (serde_json::Value, std::sync::mpsc::Receiver<String>) {
    let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
    write!(
        s,
        "GET /events?principal=arkin&mode=replica HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n"
    )
    .unwrap();
    let mut r = BufReader::new(s);
    let start = loop {
        let mut l = String::new();
        assert!(r.read_line(&mut l).unwrap() > 0, "stream ended");
        if let Some(j) = l.strip_prefix("data: ") {
            break serde_json::from_str::<serde_json::Value>(j.trim()).unwrap();
        }
    };
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || loop {
        let mut l = String::new();
        match r.read_line(&mut l) {
            Ok(0) | Err(_) => return,
            Ok(_) => {
                if let Some(j) = l.strip_prefix("data: ") {
                    if tx.send(j.trim().to_owned()).is_err() {
                        return;
                    }
                }
            }
        }
    });
    (start, rx)
}

/// A replica's bars per feed: the start frame's initial bars plus `events`, as the page's
/// core builds its provider (`ShellCore::new`); every event must apply.
fn replica_bars(
    start: &serde_json::Value,
    events: &[qp_schema::data::DataEvent],
) -> std::collections::BTreeMap<String, Vec<Bar>> {
    let data: qp_schema::protocol::InitData =
        serde_json::from_value(start["start"]["data"].clone()).unwrap();
    let mut p = qp_source::MemoryProvider::from_init(&data, std::path::Path::new(".")).unwrap();
    for ev in events {
        p.apply(ev).unwrap();
    }
    data.feeds
        .iter()
        .map(|f| {
            let s = p.feed(&f.feed).unwrap();
            (f.feed.symbol.clone(), s.bars().to_vec())
        })
        .collect()
}

/// A made-up price card message (protobuf) with `updateDate` at exchange time `t`.
fn card(t: i64, last: f64, cum: f64, first: bool) -> Vec<u8> {
    use prost::Message as _;
    let l = t + 3 * 3_600_000;
    let day = l.div_euclid(86_400_000);
    let (y, m, d) = qp_calendar::civil_from_days(day);
    let ms = l.rem_euclid(86_400_000);
    let ud = format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}.{:03}+03",
        ms / 3_600_000,
        ms / 60_000 % 60,
        ms / 1000 % 60,
        ms % 1000
    );
    qp_matriks::messages::SymbolMessage {
        symbol_id: 7,
        update_date: Some(ud),
        last: Some(last),
        open: first.then_some(last),
        daily_volume: Some(cum),
        daily_quantity: Some(cum / 10.0),
        stock_status: first.then_some(2),
        ..Default::default()
    }
    .encode_to_vec()
}

/// RP4/T5 (R6 observation, REPORT §10): a full replica (`mode=replica`) that connects
/// after live bars were added gets the same series as one connected from the start —
/// the real hub, the real Matriks feed engine, the mock broker; byte for byte.
#[test]
fn late_replica_gets_the_live_bars_of_an_early_one() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("bars")).unwrap();
    std::fs::create_dir_all(root.join("fixtures/local")).unwrap();
    write_bars(&root.join("bars"), "AAA", bars(40, 100.0));
    write_bars(&root.join("bars"), "BBB", bars(40, 50.0));
    let tokens = MockTokenService::start_default(3600);
    let broker = MockBroker::start(BrokerScript::default());
    let demo = start(&tokens, &broker, root, &[]).expect("live demo starts");
    let port = demo.port;
    wait(port, "live", |v| v["link"] == "live");
    assert!(broker.wait_until(Duration::from_secs(10), |s| s
        .subscribe_packets
        .iter()
        .map(Vec::len)
        .sum::<usize>()
        >= 2));

    let (early_start, early_rx) = replica_stream(port);
    let mut early_events: Vec<qp_schema::data::DataEvent> =
        serde_json::from_value(early_start["start"]["events"].clone()).unwrap();
    // The trading session after each feed's last history bar: new daily bars.
    let cal = qp_calendar::ExchangeCalendar::new(&qp_calendar::builtin("XIST").unwrap()).unwrap();
    let last_t = early_start["start"]["data"]["feeds"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            f["bars"].as_array().unwrap().last().unwrap()["t"]
                .as_i64()
                .unwrap()
        })
        .max()
        .unwrap();
    let mut day = (last_t + 3 * 3_600_000).div_euclid(86_400_000) + 1;
    let mut sessions = Vec::new();
    while sessions.len() < 2 {
        if let Some(s) = cal.sessions_on(day).first() {
            sessions.push(s.open);
        }
        day += 1;
    }
    let mut n_msgs = 0;
    for (k, open) in sessions.iter().enumerate() {
        for i in 0..6i32 {
            for (sym, base) in [("AAA", 100.0), ("BBB", 50.0)] {
                let t = open + 3_600_000 + i64::from(i) * 60_000;
                let first = k == 0 && i == 0;
                let cum =
                    1000.0 * f64::from(i + 1) + 10_000.0 * f64::from(u8::try_from(k).unwrap());
                broker.publish(
                    &format!("mx/symbol/{sym}@lvl2"),
                    &card(t, base + f64::from(i) * 0.1, cum, first),
                );
                n_msgs += 1;
            }
        }
    }
    let v = wait(port, "live events", |v| {
        v["messages"].as_u64().unwrap_or(0) >= n_msgs && v["events"].as_u64().unwrap_or(0) > 0
    });
    let hub_events = v["events"].as_u64().unwrap();

    // The late replica connects now.
    let (late_start, _late_rx) = replica_stream(port);
    let late_events: Vec<qp_schema::data::DataEvent> =
        serde_json::from_value(late_start["start"]["events"].clone()).unwrap();
    assert!(
        late_events.len() as u64 >= hub_events,
        "the start frame carries every live event ({} < {hub_events})",
        late_events.len()
    );
    // The early replica's data frames until it has as many events.
    let deadline = Instant::now() + Duration::from_secs(20);
    while early_events.len() < late_events.len() {
        assert!(Instant::now() < deadline, "early replica did not catch up");
        if let Ok(f) = early_rx.recv_timeout(Duration::from_millis(200)) {
            let v: serde_json::Value = serde_json::from_str(&f).unwrap();
            if let Some(d) = v.get("data") {
                let evs: Vec<qp_schema::data::DataEvent> =
                    serde_json::from_value(d.clone()).unwrap();
                early_events.extend(evs);
            }
        }
    }
    assert_eq!(
        serde_json::to_string(&early_events[..late_events.len()]).unwrap(),
        serde_json::to_string(&late_events).unwrap(),
        "the same events in the same order"
    );
    assert_eq!(
        early_start["start"]["data"], late_start["start"]["data"],
        "the same initial bars"
    );
    let early_bars = replica_bars(&early_start, &early_events[..late_events.len()]);
    let late_bars = replica_bars(&late_start, &late_events);
    for (sym, b) in &late_bars {
        let hist = late_start["start"]["data"]["feeds"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["feed"]["symbol"] == sym.as_str())
            .unwrap()["bars"]
            .as_array()
            .unwrap()
            .len();
        assert!(
            b.len() > hist,
            "{sym}: live bars added ({} <= {hist})",
            b.len()
        );
    }
    assert_eq!(
        serde_json::to_vec(&early_bars).unwrap(),
        serde_json::to_vec(&late_bars).unwrap(),
        "late replica's series byte-equal to the early one's"
    );
    let (code, _) = post(port, "/live/quit", Some(CONTROL));
    assert_eq!(code, 200);
    drop(demo);
}
