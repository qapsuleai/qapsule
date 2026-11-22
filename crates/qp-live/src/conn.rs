//! One live MQTT-over-WebSocket connection per endpoint, run by one worker thread.
//!
//! Protection rules enforced here (see README):
//! - one connection per endpoint: [`crate::lock::lock_endpoint`] is taken in
//!   [`LiveConnection::open`] and held until the worker ends;
//! - never two sockets at once: the old session is closed (MQTT DISCONNECT + WebSocket close
//!   handshake) before a new one is opened, also for token renewal;
//! - multi-connect notice = stop: DISCONNECT, no retry, permanent `Halted(MultiConnect)`,
//!   endpoint banned for the process and by a marker file until a person resets it;
//! - (A52, K64) unexplained server closes: 2 within a sliding 10 minutes = the same stop
//!   (`Halted(UnexplainedServerClose)`); what counts is in [`crate::halt`];
//! - measured reconnects: deterministic exponential back-off, stop after N failures; no new
//!   token on reconnect unless it expired or the server refused it (CONNACK 4/5, once).

use std::collections::{BTreeSet, VecDeque};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use tungstenite::{Message, WebSocket};

use crate::auth::{AccessToken, AuthClient};
use crate::backoff::{Backoff, BackoffPolicy, BackoffStep};
use crate::config::{Endpoint, LiveConfig, MqttSettings};
use crate::error::{AuthError, HaltReason, LiveError, Masked};
use crate::halt::{
    halt_status, is_unexplained_server_close, mark_halted, system_clock, Clock, ServerCloseRule,
    ServerCloseWindow,
};
use crate::lock::{default_lock_dir, lock_endpoint, EndpointKey, EndpointLock};
use crate::mqtt::{Connect, Decoder, Packet, SUBACK_FAILURE};
use crate::redact::Redactor;
use crate::transport::{open_websocket, Stream};

mod session;
mod worker;

/// What a classifier recognised in an incoming message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Notice {
    /// The server reports another session with the same identity: stop for good.
    MultiConnect,
    /// Any other server notice worth reporting (text is masked before it is shown).
    Other(String),
}

/// Inspects every incoming PUBLISH (topic, payload). The Messenger protobuf decoding lives
/// in another crate; it plugs in here.
pub type NoticeClassifier = Arc<dyn Fn(&str, &[u8]) -> Option<Notice> + Send + Sync>;

/// One received PUBLISH.
#[derive(Clone, Copy, Debug)]
pub struct LiveMessage<'a> {
    pub endpoint: &'a EndpointKey,
    /// Local sequence number, per connection object, increasing across reconnects
    /// (Matriks messages carry none).
    pub seq: u64,
    /// Session number (1 = first successful connect, +1 per reconnect).
    pub session: u32,
    pub topic: &'a str,
    pub payload: &'a [u8],
    /// Arrival time, UTC microseconds since the Unix epoch (host clock).
    pub received_at_us: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DisconnectReason {
    /// The server closed the connection (EOF, reset, WebSocket close).
    ServerClosed,
    Io(Masked),
    Protocol(Masked),
    /// TCP / TLS / WebSocket handshake failed.
    ConnectFailed(Masked),
    ConnackTimeout,
    ConnackRefused(u8),
    KeepaliveTimeout,
    /// We closed it to reconnect with a renewed token.
    TokenRenewal,
}

impl fmt::Display for DisconnectReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DisconnectReason::ServerClosed => f.write_str("server closed the connection"),
            DisconnectReason::Io(m) => write!(f, "i/o error: {m}"),
            DisconnectReason::Protocol(m) => write!(f, "protocol error: {m}"),
            DisconnectReason::ConnectFailed(m) => write!(f, "connect failed: {m}"),
            DisconnectReason::ConnackTimeout => f.write_str("no CONNACK in time"),
            DisconnectReason::ConnackRefused(c) => write!(f, "CONNACK refused (code {c})"),
            DisconnectReason::KeepaliveTimeout => f.write_str("no PINGRESP within keepalive"),
            DisconnectReason::TokenRenewal => f.write_str("closed to reconnect with a new token"),
        }
    }
}

/// Connection events. `Display` text is masked and safe to log.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LiveEvent {
    Connecting { attempt: u32 },
    Connected { session: u32 },
    Subscribed { topic: String },
    SubscribeRejected { topic: String, code: u8 },
    Unsubscribed { topics: Vec<String> },
    Disconnected { reason: DisconnectReason },
    BackingOff { failures: u32, delay: Duration },
    AuthFailed { error: AuthError },
    TokenRefreshForced { code: u8 },
    Notice { topic: String, text: Masked },
    Halted { reason: HaltReason },
    Closed,
}

impl fmt::Display for LiveEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LiveEvent::Connecting { attempt } => write!(f, "connecting (attempt {attempt})"),
            LiveEvent::Connected { session } => write!(f, "connected (session {session})"),
            LiveEvent::Subscribed { topic } => write!(f, "subscribed {topic}"),
            LiveEvent::SubscribeRejected { topic, code } => {
                write!(f, "subscription refused {topic} (code 0x{code:02x})")
            }
            LiveEvent::Unsubscribed { topics } => {
                write!(f, "unsubscribed {} topic(s)", topics.len())
            }
            LiveEvent::Disconnected { reason } => write!(f, "disconnected: {reason}"),
            LiveEvent::BackingOff { failures, delay } => write!(
                f,
                "retrying in {delay:?} after {failures} consecutive failure(s)"
            ),
            LiveEvent::AuthFailed { error } => write!(f, "token request failed: {error}"),
            LiveEvent::TokenRefreshForced { code } => write!(
                f,
                "server refused the token (CONNACK {code}); requesting a new token once"
            ),
            LiveEvent::Notice { topic, text } => write!(f, "server notice on {topic}: {text}"),
            LiveEvent::Halted { reason } => write!(f, "halted: {reason}"),
            LiveEvent::Closed => f.write_str("closed"),
        }
    }
}

/// Receives messages and events. Called on the connection's worker thread; keep it quick.
pub trait LiveSink: Send + Sync {
    fn on_message(&self, msg: &LiveMessage<'_>);
    fn on_event(&self, endpoint: &EndpointKey, event: &LiveEvent);
}

/// What to do when the token nears expiry while the connection is open.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TokenRenewal {
    /// Keep the open connection; a new token is used only on the next reconnect.
    #[default]
    OnReconnectOnly,
    /// Close (DISCONNECT) and reconnect with a new token shortly before expiry.
    ReconnectBeforeExpiry,
}

#[derive(Clone)]
pub struct ConnectOptions {
    /// Topics to subscribe (re-subscribed after every reconnect).
    pub topics: Vec<String>,
    /// Topics per SUBSCRIBE packet (1 = one by one). One packet in flight at a time.
    pub subscribe_batch: usize,
    /// Directory of the machine lock and halt marker files.
    pub lock_dir: PathBuf,
    pub backoff: BackoffPolicy,
    pub classifier: Option<NoticeClassifier>,
    pub renewal: TokenRenewal,
    /// Socket read timeout of the worker loop (command / keepalive granularity).
    pub poll_interval: Duration,
    /// How long to wait for the server's WebSocket close reply.
    pub close_timeout: Duration,
    /// (A52) Unexplained server closes that halt, and the clock their window is read on.
    pub server_close: ServerCloseRule,
    pub clock: Clock,
}

impl fmt::Debug for ConnectOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ConnectOptions")
            .field("topics", &self.topics.len())
            .field("subscribe_batch", &self.subscribe_batch)
            .field("lock_dir", &self.lock_dir)
            .field("backoff", &self.backoff)
            .field("classifier", &self.classifier.is_some())
            .field("renewal", &self.renewal)
            .field("poll_interval", &self.poll_interval)
            .field("close_timeout", &self.close_timeout)
            .field("server_close", &self.server_close)
            .finish_non_exhaustive()
    }
}

impl ConnectOptions {
    /// Defaults: one-by-one subscriptions, default lock dir, default back-off, no classifier.
    pub fn new(topics: Vec<String>) -> Self {
        Self {
            topics,
            subscribe_batch: 1,
            lock_dir: default_lock_dir(),
            backoff: BackoffPolicy::default(),
            classifier: None,
            renewal: TokenRenewal::default(),
            poll_interval: Duration::from_millis(200),
            close_timeout: Duration::from_secs(2),
            server_close: ServerCloseRule::default(),
            clock: system_clock(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConnState {
    Starting,
    Connecting {
        attempt: u32,
    },
    Connected {
        session: u32,
    },
    BackingOff {
        failures: u32,
    },
    /// Final; no automatic retry.
    Halted(HaltReason),
    /// Final; closed on request.
    Closed,
}

impl ConnState {
    pub fn is_final(&self) -> bool {
        matches!(self, ConnState::Halted(_) | ConnState::Closed)
    }
}

// ---------------------------------------------------------------------------------------
// Level-2 trade topics (RP4, K66): never subscribed.

/// True for a level-2 trade topic: `mx/trade/<SYM>@lvl2` or `mx/tradeex/<SYM>@lvl2`, also
/// inside a `mx/user/<sub>/` mirror. Level 2 carries the buyer and seller brokers (AKDE),
/// which Qapsule does not take (K66, RP4 GOAL "Veri"), whatever the licence allows. Such a
/// topic is refused locally by [`LiveConnection`] (a `SubscribeRejected` event with code
/// `0x80`) and never reaches the server. The price card `mx/symbol/<SYM>@lvl2` is not a
/// trade topic and is not affected.
pub fn is_level2_trade_topic(topic: &str) -> bool {
    let t = topic.trim();
    let is_trade = t.starts_with("mx/trade/")
        || t.starts_with("mx/tradeex/")
        || t.contains("/mx/trade/")
        || t.contains("/mx/tradeex/");
    is_trade
        && t.rsplit_once('@')
            .is_some_and(|(_, lvl)| lvl.trim().eq_ignore_ascii_case("lvl2"))
}

// Person-reset halts (multi-connect, A52): process registry + marker file, in `halt`.
pub use crate::halt::{is_multiconnect_halted, reset_multiconnect_halt};

// ---------------------------------------------------------------------------------------

struct Shared {
    state: Mutex<ConnState>,
    cv: Condvar,
    messages: AtomicU64,
    sessions: AtomicU32,
}

impl Shared {
    fn set(&self, s: ConnState) {
        let mut g = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if !g.is_final() {
            *g = s;
        }
        self.cv.notify_all();
    }
}

enum Command {
    Subscribe(Vec<String>),
    Unsubscribe(Vec<String>),
    Stop,
}

/// Handle of a live connection. Dropping it closes the connection.
pub struct LiveConnection {
    key: EndpointKey,
    lock_file: PathBuf,
    shared: Arc<Shared>,
    tx: Sender<Command>,
    thread: Option<JoinHandle<()>>,
}

impl fmt::Debug for LiveConnection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LiveConnection")
            .field("key", &self.key)
            .field("state", &self.state())
            .finish_non_exhaustive()
    }
}

impl LiveConnection {
    /// Takes the endpoint locks (in-process + machine) and starts the worker thread.
    /// Fails without touching the network if the endpoint is locked or halted.
    pub fn open(
        cfg: &LiveConfig,
        endpoint: &Endpoint,
        auth: Arc<AuthClient>,
        opts: ConnectOptions,
        sink: Arc<dyn LiveSink>,
    ) -> Result<Self, LiveError> {
        let key = endpoint.key();
        if let Some(reason) = halt_status(&opts.lock_dir, &key) {
            return Err(LiveError::Halted { key, reason });
        }
        let lock = lock_endpoint(&opts.lock_dir, &key)?;
        if let Some(reason) = halt_status(&opts.lock_dir, &key) {
            return Err(LiveError::Halted { key, reason });
        }
        let lock_file = lock.lock_file().to_path_buf();
        let shared = Arc::new(Shared {
            state: Mutex::new(ConnState::Starting),
            cv: Condvar::new(),
            messages: AtomicU64::new(0),
            sessions: AtomicU32::new(0),
        });
        let (tx, rx) = mpsc::channel();
        let redactor = cfg.redactor().clone();
        let (refused, topics): (Vec<String>, Vec<String>) = opts
            .topics
            .iter()
            .cloned()
            .partition(|t| is_level2_trade_topic(t));
        let topics: BTreeSet<String> = topics.into_iter().collect();
        let server_closes = ServerCloseWindow::new(opts.server_close.clone());
        let worker = Worker {
            key: key.clone(),
            url: endpoint.url().to_string(),
            settings: cfg.mqtt.clone(),
            client_id: client_id(cfg.hardware_id(), &key),
            auth,
            opts,
            sink,
            shared: shared.clone(),
            rx,
            redactor: redactor.clone(),
            topics,
            refused_at_start: refused,
            seq: 0,
            session: 0,
            auth_retry_used: false,
            force_refresh: false,
            server_closes,
            lock: Some(lock),
        };
        let thread = std::thread::Builder::new()
            .name(format!("qp-live-{key}"))
            .spawn(move || worker.run())
            .map_err(|e| LiveError::Io(Masked::new(&redactor, &e.to_string())))?;
        Ok(Self {
            key,
            lock_file,
            shared,
            tx,
            thread: Some(thread),
        })
    }

    pub fn key(&self) -> &EndpointKey {
        &self.key
    }

    pub fn lock_file(&self) -> &Path {
        &self.lock_file
    }

    pub fn state(&self) -> ConnState {
        self.shared
            .state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Waits until `pred` holds; returns that state, or `None` on timeout.
    pub fn wait_for(
        &self,
        timeout: Duration,
        pred: impl Fn(&ConnState) -> bool,
    ) -> Option<ConnState> {
        let deadline = Instant::now() + timeout;
        let mut g = self.shared.state.lock().unwrap_or_else(|e| e.into_inner());
        loop {
            if pred(&g) {
                return Some(g.clone());
            }
            let now = Instant::now();
            if now >= deadline {
                return None;
            }
            g = self
                .shared
                .cv
                .wait_timeout(g, deadline - now)
                .unwrap_or_else(|e| e.into_inner())
                .0;
        }
    }

    /// Messages received so far (all sessions).
    pub fn message_count(&self) -> u64 {
        self.shared.messages.load(Ordering::Relaxed)
    }

    /// Successful sessions so far.
    pub fn session_count(&self) -> u32 {
        self.shared.sessions.load(Ordering::Relaxed)
    }

    /// Adds topics (kept for reconnects).
    pub fn subscribe(&self, topics: Vec<String>) {
        let _ = self.tx.send(Command::Subscribe(topics));
    }

    /// Removes topics.
    pub fn unsubscribe(&self, topics: Vec<String>) {
        let _ = self.tx.send(Command::Unsubscribe(topics));
    }

    /// Closes (DISCONNECT), waits for the worker, releases the locks. Returns the halt as an
    /// error if the connection had halted.
    pub fn close(mut self) -> Result<(), LiveError> {
        self.shutdown();
        match self.state() {
            ConnState::Halted(reason) => Err(LiveError::Halted {
                key: self.key.clone(),
                reason,
            }),
            _ => Ok(()),
        }
    }

    fn shutdown(&mut self) {
        let _ = self.tx.send(Command::Stop);
        if let Some(h) = self.thread.take() {
            let _ = h.join();
        }
    }
}

impl Drop for LiveConnection {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// `qp-<fnv32(hwid|endpoint)>-<pid>`: deterministic per identity and endpoint, unique per
/// process, at most 22 characters (MQTT 3.1 limit is 23).
pub fn client_id(hardware_id: &str, key: &EndpointKey) -> String {
    let mut h: u32 = 0x811c_9dc5;
    for b in hardware_id
        .bytes()
        .chain(std::iter::once(b'|'))
        .chain(key.as_str().bytes())
    {
        h ^= u32::from(b);
        h = h.wrapping_mul(0x0100_0193);
    }
    format!("qp-{h:08x}-{}", std::process::id())
}

fn now_us() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_micros() as i64)
        .unwrap_or(0)
}

enum SessionEnd {
    Stopped,
    Halt(HaltReason),
    AuthRefused(u8),
    /// Closed to reconnect with this (already obtained) token.
    Renew(AccessToken),
    Lost {
        reason: DisconnectReason,
        stable: bool,
    },
}

struct Worker {
    key: EndpointKey,
    url: String,
    settings: MqttSettings,
    client_id: String,
    auth: Arc<AuthClient>,
    opts: ConnectOptions,
    sink: Arc<dyn LiveSink>,
    shared: Arc<Shared>,
    rx: Receiver<Command>,
    redactor: Redactor,
    topics: BTreeSet<String>,
    /// Level-2 trade topics given to `open`; reported (refused) when the worker starts.
    refused_at_start: Vec<String>,
    seq: u64,
    session: u32,
    auth_retry_used: bool,
    force_refresh: bool,
    /// (A52) Unexplained server closes in the sliding window.
    server_closes: ServerCloseWindow,
    // Held until the worker reaches a final state.
    lock: Option<EndpointLock>,
}

type Ws = WebSocket<Stream>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_id_is_deterministic_short_and_endpoint_specific() {
        let a = client_id("qapsule-box", &EndpointKey::new("delayed-market"));
        let b = client_id("qapsule-box", &EndpointKey::new("delayed-market"));
        let c = client_id("qapsule-box", &EndpointKey::new("delayed-news"));
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert!(a.len() <= 23, "{a}");
        assert!(a.starts_with("qp-"));
        assert!(!a.contains("box"));
    }

    #[test]
    fn level2_trade_topics_are_recognised() {
        for t in [
            "mx/trade/TEST1@lvl2",
            "mx/tradeex/TEST1@lvl2",
            "mx/trade/TEST1@LVL2",
            "mx/user/u1/mx/trade/TEST1@lvl2",
        ] {
            assert!(is_level2_trade_topic(t), "{t}");
        }
        for t in [
            "mx/trade/TEST1",
            "mx/trade/TEST1@lvl0",
            "mx/tradeex/TEST1",
            "mx/symbol/TEST1@lvl2",
            "mx/timestamp",
        ] {
            assert!(!is_level2_trade_topic(t), "{t}");
        }
    }

    #[test]
    fn reset_requires_a_note() {
        let dir = tempfile::tempdir().unwrap();
        let key = EndpointKey::new("unit-ban");
        mark_halted(dir.path(), &key, &HaltReason::MultiConnect);
        assert!(is_multiconnect_halted(dir.path(), &key));
        assert!(!reset_multiconnect_halt(dir.path(), &key, "  "));
        assert!(is_multiconnect_halted(dir.path(), &key));
        assert!(reset_multiconnect_halt(
            dir.path(),
            &key,
            "checked by operator"
        ));
        assert!(!is_multiconnect_halted(dir.path(), &key));
    }
}
