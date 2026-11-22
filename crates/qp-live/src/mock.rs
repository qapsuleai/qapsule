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

mod issuer;
mod serve;

pub use issuer::MockTokenService;
use serve::serve_connection;

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
    /// (RP4/T5) PINGREQ is counted but not answered: the path looks dead to the client
    /// (its keepalive times out), the server never closes anything.
    pub ignore_pings: bool,
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
    /// Close from the server side: every connection (`None`) or those on one path.
    Drop(Option<String>),
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
            let _ = tx.send(BrokerCmd::Drop(None));
        }
    }

    /// (RP4) Closes, from the server side, only the connections on request path `path`
    /// (`market`, `trade`); the others stay open.
    pub fn drop_path(&self, path: &str) {
        let p = path.trim_matches('/').to_string();
        for (_, tx) in self.shared.conns.lock().unwrap().iter() {
            let _ = tx.send(BrokerCmd::Drop(Some(p.clone())));
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
                BrokerCmd::Drop(p) => BrokerCmd::Drop(p.clone()),
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
