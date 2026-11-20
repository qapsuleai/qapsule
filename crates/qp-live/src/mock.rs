//! Loopback-only test doubles: a plain `ws://` MQTT broker and an HTTP token service, both on
//! 127.0.0.1 with an OS-assigned port. Never reaches any external host.
//!
//! Available with the `mock` feature (and in this crate's own tests) so other crates' tests
//! can use them too.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;
use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine as _;
use tungstenite::handshake::server::{Request, Response};
use tungstenite::http::HeaderValue;
use tungstenite::Message;

use crate::config::ConfigError;
use crate::config::{
    LiveConfig, VAR_API_PASSWORD, VAR_API_USERNAME, VAR_HARDWARE_ID, VAR_MQTT_DL_HOST,
    VAR_MQTT_MODE, VAR_SERVICE_TYPE_CODE, VAR_TOKEN_SERVICE_URL,
};
use crate::conn::{LiveEvent, LiveMessage, LiveSink, Notice, NoticeClassifier};
use crate::lock::EndpointKey;
use crate::mqtt::{Decoder, Packet};

/// Fake credentials used by [`mock_config`]. Never real values.
pub const FAKE_USERNAME: &str = "mock-svc-user-4711";
pub const FAKE_PASSWORD: &str = "mock-svc-pass-0815";
pub const FAKE_HARDWARE_ID: &str = "qapsule-mocktest";

/// Topic and payload the [`test_classifier`] treats as a multi-connect notice. (The real
/// classifier decodes the Messenger protobuf; that lives in another crate.)
pub const MULTICONNECT_TOPIC: &str = "mx/messenger";
pub const MULTICONNECT_PAYLOAD: &[u8] = b"code=DISCONNECT;subject=MULTICONNECT";

/// Classifier for tests: [`MULTICONNECT_PAYLOAD`] on [`MULTICONNECT_TOPIC`] is a
/// multi-connect notice; anything else on that topic is `Notice::Other`.
pub fn test_classifier() -> NoticeClassifier {
    Arc::new(|topic: &str, payload: &[u8]| {
        if topic != MULTICONNECT_TOPIC {
            return None;
        }
        if payload == MULTICONNECT_PAYLOAD {
            Some(Notice::MultiConnect)
        } else {
            Some(Notice::Other(String::from_utf8_lossy(payload).into_owned()))
        }
    })
}

/// A configuration pointing at the mocks, with fake credentials, read through
/// [`LiveConfig::from_lookup`] (the process environment is not touched).
pub fn mock_config(tokens: &MockTokenService, broker_url: &str) -> Result<LiveConfig, ConfigError> {
    mock_config_urls(&tokens.url(), broker_url)
}

/// Like [`mock_config`] with explicit addresses (e.g. a closed port, to test failures).
pub fn mock_config_urls(token_url: &str, broker_url: &str) -> Result<LiveConfig, ConfigError> {
    let vars: BTreeMap<&str, String> = [
        (VAR_TOKEN_SERVICE_URL, token_url.to_string()),
        (VAR_API_USERNAME, FAKE_USERNAME.to_string()),
        (VAR_API_PASSWORD, FAKE_PASSWORD.to_string()),
        (VAR_HARDWARE_ID, FAKE_HARDWARE_ID.to_string()),
        (VAR_SERVICE_TYPE_CODE, "H".to_string()),
        (VAR_MQTT_MODE, "delayed".to_string()),
        (VAR_MQTT_DL_HOST, broker_url.to_string()),
    ]
    .into_iter()
    .collect();
    LiveConfig::from_lookup(|k| vars.get(k).cloned(), "mock-host")
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn wait_poll(timeout: Duration, mut f: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if f() {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

// ---------------------------------------------------------------------------------------
// Token service

struct TokenShared {
    expected_basic: String,
    requests: AtomicU64,
    lifetime_secs: AtomicI64,
    fail_next: AtomicU32,
    refuse_next: AtomicU32,
    issued: Mutex<Vec<String>>,
    stop: AtomicBool,
}

/// HTTP token service on 127.0.0.1. Issues unsigned JWTs with `exp = now + lifetime`.
pub struct MockTokenService {
    addr: SocketAddr,
    shared: Arc<TokenShared>,
    thread: Option<JoinHandle<()>>,
}

impl fmt::Debug for MockTokenService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MockTokenService").finish_non_exhaustive()
    }
}

impl MockTokenService {
    /// Accepts Basic `username:password`; tokens live `lifetime_secs`.
    pub fn start(username: &str, password: &str, lifetime_secs: i64) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
        let addr = listener.local_addr().expect("local addr");
        let shared = Arc::new(TokenShared {
            expected_basic: format!(
                "Basic {}",
                STANDARD.encode(format!("{username}:{password}"))
            ),
            requests: AtomicU64::new(0),
            lifetime_secs: AtomicI64::new(lifetime_secs),
            fail_next: AtomicU32::new(0),
            refuse_next: AtomicU32::new(0),
            issued: Mutex::new(Vec::new()),
            stop: AtomicBool::new(false),
        });
        let s = shared.clone();
        let thread = std::thread::spawn(move || {
            for conn in listener.incoming() {
                if s.stop.load(Ordering::SeqCst) {
                    break;
                }
                if let Ok(stream) = conn {
                    handle_token_request(&s, stream);
                }
            }
        });
        Self {
            addr,
            shared,
            thread: Some(thread),
        }
    }

    /// Uses the fake credentials of [`mock_config`].
    pub fn start_default(lifetime_secs: i64) -> Self {
        Self::start(FAKE_USERNAME, FAKE_PASSWORD, lifetime_secs)
    }

    pub fn url(&self) -> String {
        format!("http://{}/issue", self.addr)
    }

    pub fn requests(&self) -> u64 {
        self.shared.requests.load(Ordering::SeqCst)
    }

    pub fn issued(&self) -> Vec<String> {
        self.shared.issued.lock().unwrap().clone()
    }

    pub fn set_lifetime_secs(&self, secs: i64) {
        self.shared.lifetime_secs.store(secs, Ordering::SeqCst);
    }

    /// Next `n` requests answer HTTP 500 (body echoes the Authorization header, to prove
    /// bodies are never logged).
    pub fn fail_next(&self, n: u32) {
        self.shared.fail_next.store(n, Ordering::SeqCst);
    }

    /// Next `n` requests answer `success: false`.
    pub fn refuse_next(&self, n: u32) {
        self.shared.refuse_next.store(n, Ordering::SeqCst);
    }
}

impl Drop for MockTokenService {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.addr);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

fn take_one(a: &AtomicU32) -> bool {
    a.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |v| v.checked_sub(1))
        .is_ok()
}

fn handle_token_request(s: &TokenShared, mut stream: TcpStream) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    let header_end = loop {
        if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break i + 4;
        }
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => return,
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
        }
    };
    let head = String::from_utf8_lossy(&buf[..header_end]).to_string();
    let mut lines = head.lines();
    let request_line = lines.next().unwrap_or("");
    let mut content_length = 0usize;
    let mut auth = String::new();
    for l in lines {
        if let Some((k, v)) = l.split_once(':') {
            match k.trim().to_ascii_lowercase().as_str() {
                "content-length" => content_length = v.trim().parse().unwrap_or(0),
                "authorization" => auth = v.trim().to_string(),
                _ => {}
            }
        }
    }
    while buf.len() < header_end + content_length {
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
        }
    }
    s.requests.fetch_add(1, Ordering::SeqCst);
    let body = &buf[header_end..buf.len().min(header_end + content_length)];
    let parsed: serde_json::Value = serde_json::from_slice(body).unwrap_or_default();
    let (status, out) = if !request_line.starts_with("POST ") {
        (405, "{}".to_string())
    } else if auth != s.expected_basic {
        (401, format!("{{\"error\":\"bad credentials {auth}\"}}"))
    } else if parsed.get("typeCode").and_then(|v| v.as_str()).is_none()
        || parsed.get("hardwareId").and_then(|v| v.as_str()).is_none()
    {
        (400, "{}".to_string())
    } else if take_one(&s.fail_next) {
        (500, format!("{{\"error\":\"internal {auth}\"}}"))
    } else if take_one(&s.refuse_next) {
        (
            200,
            format!("{{\"success\":false,\"code\":\"E_DENIED\",\"message\":\"no {auth}\"}}"),
        )
    } else {
        let n = s.requests.load(Ordering::SeqCst);
        let hwid = parsed["hardwareId"].as_str().unwrap_or("");
        let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"none","typ":"JWT"}"#);
        let claims = serde_json::json!({
            "sub": format!("QTX-{hwid}"),
            "exp": unix_now() + s.lifetime_secs.load(Ordering::SeqCst),
            "n": n,
        });
        let payload = URL_SAFE_NO_PAD.encode(claims.to_string().as_bytes());
        let jwt = format!("{header}.{payload}.mocksig{n}");
        s.issued.lock().unwrap().push(jwt.clone());
        (
            200,
            serde_json::json!({
                "success": true,
                "code": "0",
                "result": {"token": jwt, "userName": "x", "password": "y", "typeCode": "H"},
            })
            .to_string(),
        )
    };
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        405 => "Method Not Allowed",
        _ => "Internal Server Error",
    };
    let resp = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{out}",
        out.len()
    );
    let _ = stream.write_all(resp.as_bytes());
    let _ = stream.flush();
    let _ = stream.shutdown(Shutdown::Both);
}

// ---------------------------------------------------------------------------------------
// Broker

/// Scripted broker behaviour.
#[derive(Clone, Debug, Default)]
pub struct BrokerScript {
    /// CONNACK code per CONNECT, consumed in order; empty = 0 (accepted).
    pub connack_codes: VecDeque<u8>,
    /// SUBACK code per topic; missing = 0 (granted QoS 0).
    pub suback_codes: BTreeMap<String, u8>,
    /// Messages published right after a topic is granted (like an initial state).
    pub initial: BTreeMap<String, Vec<Vec<u8>>>,
    /// The next `n` TCP connections are closed before the WebSocket handshake.
    pub refuse_handshakes: u32,
}

/// One CONNECT as the broker saw it.
#[derive(Clone)]
pub struct ConnectRecord {
    pub ws_subprotocol: Option<String>,
    pub protocol_name: String,
    pub protocol_level: u8,
    pub client_id: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub keepalive: u16,
    pub clean_session: bool,
}

impl fmt::Debug for ConnectRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ConnectRecord")
            .field("ws_subprotocol", &self.ws_subprotocol)
            .field("protocol_name", &self.protocol_name)
            .field("protocol_level", &self.protocol_level)
            .field("client_id", &self.client_id)
            .field("username", &self.username)
            .field("password", &self.password.as_ref().map(|_| "[redacted]"))
            .field("keepalive", &self.keepalive)
            .field("clean_session", &self.clean_session)
            .finish()
    }
}

/// Per-path (endpoint) counters of the broker, e.g. `market` and `trade`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PathStats {
    /// WebSocket connections accepted on this path.
    pub connections_total: usize,
    /// Open right now.
    pub current: usize,
    /// Largest number of simultaneously open connections on this path.
    pub max_concurrent: usize,
    /// Every topic in every SUBSCRIBE packet on this path, in order.
    pub subscribed_topics: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct MockStats {
    /// TCP connections handled (refused handshakes not included).
    pub connections_total: usize,
    pub refused_handshakes: usize,
    /// Connections open right now.
    pub current: usize,
    /// Largest number of simultaneously open connections ever seen.
    pub max_concurrent: usize,
    /// CONNECTs answered with CONNACK 0.
    pub sessions_accepted: usize,
    pub connects: Vec<ConnectRecord>,
    pub subscribe_packets: Vec<Vec<String>>,
    pub unsubscribe_packets: Vec<Vec<String>>,
    pub pings: usize,
    pub disconnects: usize,
    /// The same counters per request path (without slashes: `market`, `trade`).
    pub by_path: BTreeMap<String, PathStats>,
}

impl MockStats {
    /// Counters of one path (empty if never used).
    pub fn path(&self, path: &str) -> PathStats {
        self.by_path
            .get(path.trim_matches('/'))
            .cloned()
            .unwrap_or_default()
    }
}

enum BrokerCmd {
    Publish {
        topic: String,
        payload: Vec<u8>,
        only_subscribed: bool,
    },
    Drop,
}

struct BrokerShared {
    script: Mutex<BrokerScript>,
    stats: Mutex<MockStats>,
    conns: Mutex<Vec<(u64, Sender<BrokerCmd>)>>,
    next_id: AtomicU64,
    stop: AtomicBool,
}

/// Plain `ws://` MQTT broker on 127.0.0.1. Accepts any path.
pub struct MockBroker {
    addr: SocketAddr,
    shared: Arc<BrokerShared>,
    thread: Option<JoinHandle<()>>,
}

impl fmt::Debug for MockBroker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MockBroker").finish_non_exhaustive()
    }
}

impl MockBroker {
    pub fn start(script: BrokerScript) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
        let addr = listener.local_addr().expect("local addr");
        let shared = Arc::new(BrokerShared {
            script: Mutex::new(script),
            stats: Mutex::new(MockStats::default()),
            conns: Mutex::new(Vec::new()),
            next_id: AtomicU64::new(1),
            stop: AtomicBool::new(false),
        });
        let s = shared.clone();
        let thread = std::thread::spawn(move || {
            for conn in listener.incoming() {
                if s.stop.load(Ordering::SeqCst) {
                    break;
                }
                let Ok(stream) = conn else { continue };
                let refuse = {
                    let mut sc = s.script.lock().unwrap();
                    if sc.refuse_handshakes > 0 {
                        sc.refuse_handshakes -= 1;
                        true
                    } else {
                        false
                    }
                };
                if refuse {
                    s.stats.lock().unwrap().refused_handshakes += 1;
                    let _ = stream.shutdown(Shutdown::Both);
                    continue;
                }
                let id = s.next_id.fetch_add(1, Ordering::SeqCst);
                let (tx, rx) = mpsc::channel();
                s.conns.lock().unwrap().push((id, tx));
                let gauge = Gauge::open(s.clone());
                let s2 = s.clone();
                std::thread::spawn(move || {
                    serve_connection(&s2, stream, rx, gauge);
                    s2.conns.lock().unwrap().retain(|(i, _)| *i != id);
                });
            }
        });
        Self {
            addr,
            shared,
            thread: Some(thread),
        }
    }

    /// `ws://127.0.0.1:<port>/<path>`.
    pub fn url(&self, path: &str) -> String {
        format!("ws://{}/{}", self.addr, path.trim_start_matches('/'))
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn stats(&self) -> MockStats {
        self.shared.stats.lock().unwrap().clone()
    }

    pub fn script(&self, f: impl FnOnce(&mut BrokerScript)) {
        f(&mut self.shared.script.lock().unwrap());
    }

    /// Publishes to connections subscribed to exactly `topic`.
    pub fn publish(&self, topic: &str, payload: &[u8]) {
        self.send_all(BrokerCmd::Publish {
            topic: topic.to_string(),
            payload: payload.to_vec(),
            only_subscribed: true,
        });
    }

    /// Publishes to every connection (subscribed or not), like a server notice.
    pub fn broadcast(&self, topic: &str, payload: &[u8]) {
        self.send_all(BrokerCmd::Publish {
            topic: topic.to_string(),
            payload: payload.to_vec(),
            only_subscribed: false,
        });
    }

    /// Sends the multi-connect notice ([`MULTICONNECT_TOPIC`] / [`MULTICONNECT_PAYLOAD`]).
    pub fn send_multiconnect(&self) {
        self.broadcast(MULTICONNECT_TOPIC, MULTICONNECT_PAYLOAD);
    }

    /// Closes every open connection from the server side (TCP shutdown, no MQTT goodbye).
    pub fn drop_connections(&self) {
        for (_, tx) in self.shared.conns.lock().unwrap().iter() {
            let _ = tx.send(BrokerCmd::Drop);
        }
    }

    /// Polls the statistics until `pred` holds or `timeout` passes.
    pub fn wait_until(&self, timeout: Duration, pred: impl Fn(&MockStats) -> bool) -> bool {
        wait_poll(timeout, || pred(&self.stats()))
    }

    fn send_all(&self, cmd: BrokerCmd) {
        for (_, tx) in self.shared.conns.lock().unwrap().iter() {
            let c = match &cmd {
                BrokerCmd::Publish {
                    topic,
                    payload,
                    only_subscribed,
                } => BrokerCmd::Publish {
                    topic: topic.clone(),
                    payload: payload.clone(),
                    only_subscribed: *only_subscribed,
                },
                BrokerCmd::Drop => BrokerCmd::Drop,
            };
            let _ = tx.send(c);
        }
    }
}

impl Drop for MockBroker {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.addr);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// Counts an open connection; `close()` (or drop) decrements once. The broker decrements
/// *before* it answers a goodbye, so a client that waited for the answer can never be
/// counted twice.
struct Gauge {
    shared: Arc<BrokerShared>,
    open: bool,
    /// Request path, once the WebSocket handshake told it.
    path: Option<String>,
}

impl Gauge {
    fn open(shared: Arc<BrokerShared>) -> Self {
        {
            let mut st = shared.stats.lock().unwrap();
            st.connections_total += 1;
            st.current += 1;
            st.max_concurrent = st.max_concurrent.max(st.current);
        }
        Self {
            shared,
            open: true,
            path: None,
        }
    }

    /// Counts this connection under `path` too (called once, after the handshake).
    fn set_path(&mut self, path: &str) {
        if !self.open || self.path.is_some() {
            return;
        }
        let p = path.trim_matches('/').to_string();
        {
            let mut st = self.shared.stats.lock().unwrap();
            let ps = st.by_path.entry(p.clone()).or_default();
            ps.connections_total += 1;
            ps.current += 1;
            ps.max_concurrent = ps.max_concurrent.max(ps.current);
        }
        self.path = Some(p);
    }

    fn close(&mut self) {
        if self.open {
            self.open = false;
            let mut st = self.shared.stats.lock().unwrap();
            st.current -= 1;
            if let Some(p) = &self.path {
                if let Some(ps) = st.by_path.get_mut(p) {
                    ps.current -= 1;
                }
            }
        }
    }
}

impl Drop for Gauge {
    fn drop(&mut self) {
        self.close();
    }
}

fn serve_connection(
    s: &BrokerShared,
    stream: TcpStream,
    rx: Receiver<BrokerCmd>,
    mut gauge: Gauge,
) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let mut chosen: Option<String> = None;
    let mut req_path = String::new();
    let callback = |req: &Request, mut resp: Response| {
        req_path = req.uri().path().to_string();
        if let Some(p) = req.headers().get("Sec-WebSocket-Protocol") {
            let first = p
                .to_str()
                .unwrap_or("")
                .split(',')
                .next()
                .unwrap_or("")
                .trim()
                .to_string();
            if let Ok(v) = HeaderValue::from_str(&first) {
                resp.headers_mut().insert("Sec-WebSocket-Protocol", v);
            }
            chosen = Some(first);
        }
        Ok(resp)
    };
    let mut ws = match tungstenite::accept_hdr(stream, callback) {
        Ok(ws) => ws,
        Err(_) => return,
    };
    gauge.set_path(&req_path);
    let path_key = req_path.trim_matches('/').to_string();
    let _ = ws
        .get_ref()
        .set_read_timeout(Some(Duration::from_millis(20)));
    let mut dec = Decoder::new();
    let mut subscribed: BTreeSet<String> = BTreeSet::new();
    let send = |ws: &mut tungstenite::WebSocket<TcpStream>, p: &Packet| {
        ws.send(Message::Binary(p.encode().into())).is_ok()
    };
    loop {
        if s.stop.load(Ordering::SeqCst) {
            gauge.close();
            let _ = ws.get_ref().shutdown(Shutdown::Both);
            return;
        }
        while let Ok(cmd) = rx.try_recv() {
            match cmd {
                BrokerCmd::Publish {
                    topic,
                    payload,
                    only_subscribed,
                } => {
                    if !only_subscribed || subscribed.contains(&topic) {
                        send(
                            &mut ws,
                            &Packet::Publish {
                                topic,
                                payload,
                                qos: 0,
                                retain: false,
                                dup: false,
                                packet_id: None,
                            },
                        );
                    }
                }
                BrokerCmd::Drop => {
                    gauge.close();
                    let _ = ws.get_ref().shutdown(Shutdown::Both);
                    return;
                }
            }
        }
        match ws.read() {
            Ok(Message::Binary(b)) => {
                dec.push(&b);
                while let Ok(Some(p)) = dec.next_packet() {
                    match p {
                        Packet::Connect(c) => {
                            let code = s.script.lock().unwrap().connack_codes.pop_front();
                            let code = code.unwrap_or(0);
                            {
                                let mut st = s.stats.lock().unwrap();
                                st.connects.push(ConnectRecord {
                                    ws_subprotocol: chosen.clone(),
                                    protocol_name: c.protocol_name,
                                    protocol_level: c.protocol_level,
                                    client_id: c.client_id,
                                    username: c.username,
                                    password: c
                                        .password
                                        .map(|p| String::from_utf8_lossy(&p).into_owned()),
                                    keepalive: c.keepalive,
                                    clean_session: c.clean_session,
                                });
                                if code == 0 {
                                    st.sessions_accepted += 1;
                                }
                            }
                            if code != 0 {
                                gauge.close();
                                send(
                                    &mut ws,
                                    &Packet::ConnAck {
                                        session_present: false,
                                        code,
                                    },
                                );
                                let _ = ws.get_ref().shutdown(Shutdown::Both);
                                return;
                            }
                            send(
                                &mut ws,
                                &Packet::ConnAck {
                                    session_present: false,
                                    code: 0,
                                },
                            );
                        }
                        Packet::Subscribe { packet_id, topics } => {
                            let names: Vec<String> =
                                topics.iter().map(|(t, _)| t.clone()).collect();
                            {
                                let mut st = s.stats.lock().unwrap();
                                st.subscribe_packets.push(names.clone());
                                st.by_path
                                    .entry(path_key.clone())
                                    .or_default()
                                    .subscribed_topics
                                    .extend(names.iter().cloned());
                            }
                            let (codes, initial) = {
                                let sc = s.script.lock().unwrap();
                                let codes: Vec<u8> = names
                                    .iter()
                                    .map(|t| sc.suback_codes.get(t).copied().unwrap_or(0))
                                    .collect();
                                let mut initial = Vec::new();
                                for (t, c) in names.iter().zip(&codes) {
                                    if *c != crate::mqtt::SUBACK_FAILURE {
                                        for m in sc.initial.get(t).into_iter().flatten() {
                                            initial.push((t.clone(), m.clone()));
                                        }
                                    }
                                }
                                (codes, initial)
                            };
                            for (t, c) in names.iter().zip(&codes) {
                                if *c != crate::mqtt::SUBACK_FAILURE {
                                    subscribed.insert(t.clone());
                                }
                            }
                            send(&mut ws, &Packet::SubAck { packet_id, codes });
                            for (topic, payload) in initial {
                                send(
                                    &mut ws,
                                    &Packet::Publish {
                                        topic,
                                        payload,
                                        qos: 0,
                                        retain: false,
                                        dup: false,
                                        packet_id: None,
                                    },
                                );
                            }
                        }
                        Packet::Unsubscribe { packet_id, topics } => {
                            for t in &topics {
                                subscribed.remove(t);
                            }
                            s.stats.lock().unwrap().unsubscribe_packets.push(topics);
                            send(&mut ws, &Packet::UnsubAck(packet_id));
                        }
                        Packet::PingReq => {
                            s.stats.lock().unwrap().pings += 1;
                            send(&mut ws, &Packet::PingResp);
                        }
                        Packet::Disconnect => {
                            s.stats.lock().unwrap().disconnects += 1;
                            gauge.close();
                        }
                        _ => {}
                    }
                }
            }
            Ok(Message::Close(_)) => {
                gauge.close();
                // tungstenite queued the close reply; flush it, then drop.
                let _ = ws.flush();
                return;
            }
            Ok(_) => {}
            Err(tungstenite::Error::Io(e))
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) => {}
            Err(_) => {
                gauge.close();
                return;
            }
        }
    }
}

// ---------------------------------------------------------------------------------------
// Recording sink

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordedMessage {
    pub seq: u64,
    pub session: u32,
    pub topic: String,
    pub payload: Vec<u8>,
    pub received_at_us: i64,
}

/// Sink that keeps everything (for tests).
#[derive(Default)]
pub struct RecordingSink {
    messages: Mutex<Vec<RecordedMessage>>,
    events: Mutex<Vec<(EndpointKey, LiveEvent)>>,
}

impl RecordingSink {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn messages(&self) -> Vec<RecordedMessage> {
        self.messages.lock().unwrap().clone()
    }

    pub fn events(&self) -> Vec<LiveEvent> {
        self.events
            .lock()
            .unwrap()
            .iter()
            .map(|(_, e)| e.clone())
            .collect()
    }

    /// `Display` and `Debug` of every event (what a logger would write).
    pub fn event_texts(&self) -> Vec<String> {
        self.events
            .lock()
            .unwrap()
            .iter()
            .flat_map(|(k, e)| [format!("{k}: {e}"), format!("{e:?}")])
            .collect()
    }

    pub fn count(&self, pred: impl Fn(&LiveEvent) -> bool) -> usize {
        self.events
            .lock()
            .unwrap()
            .iter()
            .filter(|(_, e)| pred(e))
            .count()
    }

    pub fn wait_for(&self, timeout: Duration, pred: impl Fn(&LiveEvent) -> bool) -> bool {
        wait_poll(timeout, || self.count(&pred) > 0)
    }

    pub fn wait_messages(&self, timeout: Duration, n: usize) -> bool {
        wait_poll(timeout, || self.messages.lock().unwrap().len() >= n)
    }
}

impl LiveSink for RecordingSink {
    fn on_message(&self, msg: &LiveMessage<'_>) {
        self.messages.lock().unwrap().push(RecordedMessage {
            seq: msg.seq,
            session: msg.session,
            topic: msg.topic.to_string(),
            payload: msg.payload.to_vec(),
            received_at_us: msg.received_at_us,
        });
    }

    fn on_event(&self, endpoint: &EndpointKey, event: &LiveEvent) {
        self.events
            .lock()
            .unwrap()
            .push((endpoint.clone(), event.clone()));
    }
}
