//! One live MQTT-over-WebSocket connection per endpoint, run by one worker thread.
//!
//! Protection rules enforced here (see README):
//! - one connection per endpoint: [`crate::lock::lock_endpoint`] is taken in
//!   [`LiveConnection::open`] and held until the worker ends;
//! - never two sockets at once: the old session is closed (MQTT DISCONNECT + WebSocket close
//!   handshake) before a new one is opened, also for token renewal;
//! - multi-connect notice = stop: DISCONNECT, no retry, permanent `Halted(MultiConnect)`,
//!   endpoint banned for the process and by a marker file until a person resets it;
//! - measured reconnects: deterministic exponential back-off, stop after N failures; no new
//!   token on reconnect unless it expired or the server refused it (CONNACK 4/5, once).

use std::collections::{BTreeSet, VecDeque};
use std::fmt;
use std::io::Write as _;
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
use crate::lock::{default_lock_dir, lock_endpoint, EndpointKey, EndpointLock};
use crate::mqtt::{Connect, Decoder, Packet, SUBACK_FAILURE};
use crate::redact::Redactor;
use crate::transport::{open_websocket, Stream};

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
            .finish()
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

// ---------------------------------------------------------------------------------------
// Multi-connect ban (process registry + marker file).

static BANNED: Mutex<BTreeSet<String>> = Mutex::new(BTreeSet::new());

/// True if the endpoint halted on a multi-connect notice (this process, or the marker file
/// written by any process) and was not reset.
pub fn is_multiconnect_halted(lock_dir: &Path, key: &EndpointKey) -> bool {
    BANNED
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .contains(key.as_str())
        || key.halt_marker(lock_dir).exists()
}

fn ban_endpoint(lock_dir: &Path, key: &EndpointKey) {
    BANNED
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(key.as_str().to_string());
    let at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    if let Ok(mut f) = std::fs::File::create(key.halt_marker(lock_dir)) {
        let _ = writeln!(
            f,
            "halted=multiconnect\nendpoint={key}\nat_unix={at}\npid={}\n\
             Remove only after a person checked why (qp_live::conn::reset_multiconnect_halt).",
            std::process::id()
        );
    }
}

/// Explicit, person-confirmed reset of a multi-connect halt. Requires a non-empty note
/// (who / why); returns false and changes nothing without one.
pub fn reset_multiconnect_halt(lock_dir: &Path, key: &EndpointKey, operator_note: &str) -> bool {
    if operator_note.trim().is_empty() {
        return false;
    }
    BANNED
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(key.as_str());
    let _ = std::fs::remove_file(key.halt_marker(lock_dir));
    true
}

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
        let halted = |key: &EndpointKey| LiveError::Halted {
            key: key.clone(),
            reason: HaltReason::MultiConnect,
        };
        if is_multiconnect_halted(&opts.lock_dir, &key) {
            return Err(halted(&key));
        }
        let lock = lock_endpoint(&opts.lock_dir, &key)?;
        if is_multiconnect_halted(&opts.lock_dir, &key) {
            return Err(halted(&key));
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
    // Held until the worker reaches a final state.
    lock: Option<EndpointLock>,
}

type Ws = WebSocket<Stream>;

impl Worker {
    fn event(&self, e: LiveEvent) {
        self.sink.on_event(&self.key, &e);
    }

    fn mask(&self, s: &str) -> String {
        self.redactor.scrub(s)
    }

    fn run(mut self) {
        for topic in std::mem::take(&mut self.refused_at_start) {
            self.refuse_level2(topic);
        }
        let mut backoff = Backoff::new(self.opts.backoff.clone());
        let mut renewed: Option<AccessToken> = None;
        loop {
            if self.drain_commands() {
                break;
            }
            let token = if let Some(t) = renewed.take() {
                Ok(t)
            } else if self.force_refresh {
                self.auth.refresh()
            } else {
                self.auth.token()
            };
            let token = match token {
                Ok(t) => {
                    self.force_refresh = false;
                    t
                }
                Err(AuthError::RateLimited { wait }) => {
                    if self.sleep(wait) {
                        break;
                    }
                    continue;
                }
                Err(AuthError::Halted { .. }) => return self.halt(HaltReason::AuthUnavailable),
                Err(error) => {
                    self.event(LiveEvent::AuthFailed { error });
                    if self.back_off(&mut backoff) {
                        return;
                    }
                    continue;
                }
            };
            let attempt = backoff.failures() + 1;
            self.shared.set(ConnState::Connecting { attempt });
            self.event(LiveEvent::Connecting { attempt });
            match self.session(&token) {
                SessionEnd::Stopped => break,
                SessionEnd::Halt(reason) => return self.halt(reason),
                SessionEnd::AuthRefused(code) => {
                    self.event(LiveEvent::Disconnected {
                        reason: DisconnectReason::ConnackRefused(code),
                    });
                    if self.auth_retry_used {
                        return self.halt(HaltReason::AuthRejected { code });
                    }
                    self.auth_retry_used = true;
                    self.force_refresh = true;
                    self.event(LiveEvent::TokenRefreshForced { code });
                }
                SessionEnd::Renew(t) => {
                    renewed = Some(t);
                    self.event(LiveEvent::Disconnected {
                        reason: DisconnectReason::TokenRenewal,
                    });
                }
                SessionEnd::Lost { reason, stable } => {
                    self.event(LiveEvent::Disconnected { reason });
                    if stable {
                        backoff.reset();
                    }
                    if self.back_off(&mut backoff) {
                        return;
                    }
                }
            }
        }
        self.finish_closed();
    }

    /// Releases the locks, then publishes `Closed`.
    fn finish_closed(&mut self) {
        self.lock = None;
        self.shared.set(ConnState::Closed);
        self.event(LiveEvent::Closed);
    }

    /// Records a failure and sleeps. Returns true if the worker must end (halted or stopped).
    fn back_off(&mut self, backoff: &mut Backoff) -> bool {
        match backoff.record_failure() {
            BackoffStep::Retry { failures, delay } => {
                self.shared.set(ConnState::BackingOff { failures });
                self.event(LiveEvent::BackingOff { failures, delay });
                if self.sleep(delay) {
                    self.finish_closed();
                    return true;
                }
                false
            }
            BackoffStep::Stop { failures } => {
                self.halt(HaltReason::TooManyFailures { failures });
                true
            }
        }
    }

    /// Bans (multi-connect), releases the locks, then publishes the final `Halted` state.
    fn halt(&mut self, reason: HaltReason) {
        if reason == HaltReason::MultiConnect {
            ban_endpoint(&self.opts.lock_dir, &self.key);
        }
        self.lock = None;
        self.shared.set(ConnState::Halted(reason.clone()));
        self.event(LiveEvent::Halted { reason });
    }

    fn add_topics(&mut self, topics: Vec<String>) -> Vec<String> {
        let mut added = Vec::new();
        for t in topics {
            if is_level2_trade_topic(&t) {
                self.refuse_level2(t);
            } else if !t.is_empty() && self.topics.insert(t.clone()) {
                added.push(t);
            }
        }
        added
    }

    /// A level-2 trade topic is refused here; nothing goes to the server.
    fn refuse_level2(&self, topic: String) {
        self.event(LiveEvent::SubscribeRejected {
            topic,
            code: SUBACK_FAILURE,
        });
    }

    fn remove_topics(&mut self, topics: Vec<String>) -> Vec<String> {
        topics
            .into_iter()
            .filter(|t| self.topics.remove(t))
            .collect()
    }

    /// Applies pending commands; true if stop was requested.
    fn drain_commands(&mut self) -> bool {
        loop {
            match self.rx.try_recv() {
                Ok(Command::Stop) | Err(TryRecvError::Disconnected) => return true,
                Ok(Command::Subscribe(t)) => {
                    self.add_topics(t);
                }
                Ok(Command::Unsubscribe(t)) => {
                    self.remove_topics(t);
                }
                Err(TryRecvError::Empty) => return false,
            }
        }
    }

    /// Sleeps while applying commands; true if stop was requested.
    fn sleep(&mut self, d: Duration) -> bool {
        let deadline = Instant::now() + d;
        loop {
            let now = Instant::now();
            if now >= deadline {
                return false;
            }
            match self.rx.recv_timeout(deadline - now) {
                Ok(Command::Stop) | Err(RecvTimeoutError::Disconnected) => return true,
                Ok(Command::Subscribe(t)) => {
                    self.add_topics(t);
                }
                Ok(Command::Unsubscribe(t)) => {
                    self.remove_topics(t);
                }
                Err(RecvTimeoutError::Timeout) => return false,
            }
        }
    }

    fn batches(&self, topics: impl IntoIterator<Item = String>) -> Vec<Vec<String>> {
        let n = self.opts.subscribe_batch.max(1);
        let all: Vec<String> = topics.into_iter().collect();
        all.chunks(n).map(|c| c.to_vec()).collect()
    }

    fn session(&mut self, token: &AccessToken) -> SessionEnd {
        let mut ws = match open_websocket(
            &self.url,
            &self.settings.ws_subprotocol,
            self.settings.connect_timeout,
        ) {
            Ok(ws) => ws,
            Err(e) => {
                return SessionEnd::Lost {
                    reason: DisconnectReason::ConnectFailed(Masked::new(&self.redactor, &e)),
                    stable: false,
                }
            }
        };
        let mut connect = Connect::new(
            self.settings.protocol,
            &self.client_id,
            self.settings.keepalive_secs,
        );
        connect.username = Some("JWT".to_string());
        connect.password = Some(token.expose().as_bytes().to_vec());
        if let Err(r) = self.send(&mut ws, &Packet::Connect(connect)) {
            self.close_ws(&mut ws);
            return SessionEnd::Lost {
                reason: r,
                stable: false,
            };
        }
        let mut dec = Decoder::new();
        let deadline = Instant::now() + self.settings.connect_timeout;
        let code = loop {
            if Instant::now() >= deadline {
                self.close_ws(&mut ws);
                return SessionEnd::Lost {
                    reason: DisconnectReason::ConnackTimeout,
                    stable: false,
                };
            }
            match self.poll_read(&mut ws, &mut dec) {
                Err(reason) => {
                    self.close_ws(&mut ws);
                    return SessionEnd::Lost {
                        reason,
                        stable: false,
                    };
                }
                Ok(ps) => {
                    if let Some(code) = ps.iter().find_map(|p| match p {
                        Packet::ConnAck { code, .. } => Some(*code),
                        _ => None,
                    }) {
                        break code;
                    }
                }
            }
        };
        match code {
            0 => {}
            4 | 5 => {
                self.close_ws(&mut ws);
                return SessionEnd::AuthRefused(code);
            }
            1 | 2 => {
                self.close_ws(&mut ws);
                return SessionEnd::Halt(HaltReason::ProtocolRejected { code });
            }
            other => {
                self.close_ws(&mut ws);
                return SessionEnd::Lost {
                    reason: DisconnectReason::ConnackRefused(other),
                    stable: false,
                };
            }
        }
        self.auth_retry_used = false;
        self.session += 1;
        let session = self.session;
        self.shared.sessions.fetch_add(1, Ordering::Relaxed);
        let _ = ws
            .get_ref()
            .tcp()
            .set_read_timeout(Some(self.opts.poll_interval));
        self.shared.set(ConnState::Connected { session });
        self.event(LiveEvent::Connected { session });

        let connected_at = Instant::now();
        let stable_after = self.opts.backoff.stable_after;
        let keepalive = Duration::from_secs(u64::from(self.settings.keepalive_secs.max(1)));
        let ping_every = keepalive * 3 / 4;
        let mut queue: VecDeque<Vec<String>> = self.batches(self.topics.iter().cloned()).into();
        let mut inflight: Option<(u16, Vec<String>)> = None;
        let mut next_pid: u16 = 0;
        let mut pid = || {
            next_pid = next_pid.wrapping_add(1).max(1);
            next_pid
        };
        let mut last_sent = Instant::now();
        let mut ping_sent: Option<Instant> = None;

        loop {
            let lost = |reason| SessionEnd::Lost {
                reason,
                stable: connected_at.elapsed() >= stable_after,
            };
            // Commands.
            loop {
                match self.rx.try_recv() {
                    Ok(Command::Stop) | Err(TryRecvError::Disconnected) => {
                        self.close_ws(&mut ws);
                        return SessionEnd::Stopped;
                    }
                    Ok(Command::Subscribe(t)) => {
                        let new = self.add_topics(t);
                        queue.extend(self.batches(new));
                    }
                    Ok(Command::Unsubscribe(t)) => {
                        let gone = self.remove_topics(t);
                        if !gone.is_empty() {
                            let p = Packet::Unsubscribe {
                                packet_id: pid(),
                                topics: gone.clone(),
                            };
                            if let Err(r) = self.send(&mut ws, &p) {
                                self.close_ws(&mut ws);
                                return lost(r);
                            }
                            last_sent = Instant::now();
                            let topics = gone.iter().map(|t| self.mask(t)).collect();
                            self.event(LiveEvent::Unsubscribed { topics });
                        }
                    }
                    Err(TryRecvError::Empty) => break,
                }
            }
            // One SUBSCRIBE in flight at a time.
            if inflight.is_none() {
                if let Some(batch) = queue.pop_front() {
                    let batch: Vec<String> = batch
                        .into_iter()
                        .filter(|t| self.topics.contains(t))
                        .collect();
                    if !batch.is_empty() {
                        let id = pid();
                        let p = Packet::Subscribe {
                            packet_id: id,
                            topics: batch.iter().map(|t| (t.clone(), 0)).collect(),
                        };
                        if let Err(r) = self.send(&mut ws, &p) {
                            self.close_ws(&mut ws);
                            return lost(r);
                        }
                        last_sent = Instant::now();
                        inflight = Some((id, batch));
                    }
                }
            }
            // Token renewal on an open connection (opt-in). The new token is obtained first
            // (rate limited by the auth client); only a token different from the current one
            // closes the connection, so a refused / rate-limited request never causes a
            // reconnect loop. Old connection closed before the new one is opened.
            if self.opts.renewal == TokenRenewal::ReconnectBeforeExpiry
                && token.remaining() <= self.auth.policy().refresh_before
            {
                if let Ok(fresh) = self.auth.token() {
                    if !fresh.same_value(token) {
                        self.close_ws(&mut ws);
                        return SessionEnd::Renew(fresh);
                    }
                }
            }
            // Keepalive.
            match ping_sent {
                Some(at) if at.elapsed() > keepalive => {
                    self.close_ws(&mut ws);
                    return lost(DisconnectReason::KeepaliveTimeout);
                }
                None if last_sent.elapsed() >= ping_every => {
                    if let Err(r) = self.send(&mut ws, &Packet::PingReq) {
                        self.close_ws(&mut ws);
                        return lost(r);
                    }
                    last_sent = Instant::now();
                    ping_sent = Some(last_sent);
                }
                _ => {}
            }
            // Read.
            let packets = match self.poll_read(&mut ws, &mut dec) {
                Ok(ps) => ps,
                Err(reason) => {
                    self.close_ws(&mut ws);
                    return lost(reason);
                }
            };
            for p in packets {
                match p {
                    Packet::Publish {
                        topic,
                        payload,
                        qos,
                        packet_id,
                        ..
                    } => {
                        if qos > 0 {
                            if let Some(id) = packet_id {
                                if let Err(r) = self.send(&mut ws, &Packet::PubAck(id)) {
                                    self.close_ws(&mut ws);
                                    return lost(r);
                                }
                            }
                        }
                        self.seq += 1;
                        self.shared.messages.fetch_add(1, Ordering::Relaxed);
                        self.sink.on_message(&LiveMessage {
                            endpoint: &self.key,
                            seq: self.seq,
                            session,
                            topic: &topic,
                            payload: &payload,
                            received_at_us: now_us(),
                        });
                        let notice = self
                            .opts
                            .classifier
                            .as_ref()
                            .and_then(|c| c(&topic, &payload));
                        match notice {
                            Some(Notice::MultiConnect) => {
                                self.close_ws(&mut ws);
                                return SessionEnd::Halt(HaltReason::MultiConnect);
                            }
                            Some(Notice::Other(text)) => self.event(LiveEvent::Notice {
                                topic: self.mask(&topic),
                                text: Masked::new(&self.redactor, &text),
                            }),
                            None => {}
                        }
                    }
                    Packet::SubAck { packet_id, codes } => {
                        let Some((id, batch)) = inflight.take() else {
                            continue;
                        };
                        if id != packet_id {
                            inflight = Some((id, batch));
                            continue;
                        }
                        for (i, t) in batch.iter().enumerate() {
                            let code = codes.get(i).copied().unwrap_or(SUBACK_FAILURE);
                            if code == SUBACK_FAILURE {
                                // Not re-subscribed on reconnect unless asked again.
                                self.topics.remove(t);
                                self.event(LiveEvent::SubscribeRejected {
                                    topic: self.mask(t),
                                    code,
                                });
                            } else {
                                self.event(LiveEvent::Subscribed {
                                    topic: self.mask(t),
                                });
                            }
                        }
                    }
                    Packet::PingResp => ping_sent = None,
                    Packet::Disconnect => {
                        self.close_ws(&mut ws);
                        return lost(DisconnectReason::ServerClosed);
                    }
                    _ => {}
                }
            }
        }
    }

    fn send(&self, ws: &mut Ws, p: &Packet) -> Result<(), DisconnectReason> {
        ws.send(Message::Binary(p.encode().into()))
            .map_err(|e| self.ws_error(e))
    }

    fn ws_error(&self, e: tungstenite::Error) -> DisconnectReason {
        use std::io::ErrorKind as K;
        match e {
            tungstenite::Error::ConnectionClosed
            | tungstenite::Error::AlreadyClosed
            | tungstenite::Error::Protocol(
                tungstenite::error::ProtocolError::ResetWithoutClosingHandshake,
            ) => DisconnectReason::ServerClosed,
            tungstenite::Error::Io(io)
                if matches!(
                    io.kind(),
                    K::UnexpectedEof | K::ConnectionReset | K::ConnectionAborted | K::BrokenPipe
                ) =>
            {
                DisconnectReason::ServerClosed
            }
            tungstenite::Error::Protocol(p) => {
                DisconnectReason::Protocol(Masked::new(&self.redactor, &p.to_string()))
            }
            other => DisconnectReason::Io(Masked::new(&self.redactor, &other.to_string())),
        }
    }

    /// One read with the socket timeout; returns the complete packets received (may be none).
    fn poll_read(&self, ws: &mut Ws, dec: &mut Decoder) -> Result<Vec<Packet>, DisconnectReason> {
        match ws.read() {
            Ok(Message::Binary(b)) => {
                dec.push(&b);
                let mut out = Vec::new();
                loop {
                    match dec.next_packet() {
                        Ok(Some(p)) => out.push(p),
                        Ok(None) => return Ok(out),
                        Err(e) => {
                            return Err(DisconnectReason::Protocol(Masked::new(
                                &self.redactor,
                                &e.to_string(),
                            )))
                        }
                    }
                }
            }
            Ok(Message::Close(_)) => Err(DisconnectReason::ServerClosed),
            Ok(_) => Ok(Vec::new()),
            Err(tungstenite::Error::Io(e))
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                Ok(Vec::new())
            }
            Err(e) => Err(self.ws_error(e)),
        }
    }

    /// MQTT DISCONNECT, WebSocket close handshake (bounded wait), TCP shutdown. After this
    /// returns the socket is gone, so a following connect never overlaps it.
    fn close_ws(&self, ws: &mut Ws) {
        let _ = ws.send(Message::Binary(Packet::Disconnect.encode().into()));
        let _ = ws.close(None);
        let _ = ws
            .get_ref()
            .tcp()
            .set_read_timeout(Some(Duration::from_millis(50)));
        let deadline = Instant::now() + self.opts.close_timeout;
        while Instant::now() < deadline {
            match ws.read() {
                Ok(_) => {}
                Err(tungstenite::Error::Io(e))
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) =>
                {
                    let _ = ws.flush();
                }
                Err(_) => break,
            }
        }
        ws.get_ref().shutdown();
    }
}

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
        ban_endpoint(dir.path(), &key);
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
