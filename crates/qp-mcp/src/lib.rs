//! MCP server adapter over the `qp/0` reference authority (`rounds/RD1/EMBEDDING.md` §1).
//!
//! - [`Hub`]: one [`qp_session::Session`] shared by any number of adapters, with one
//!   queue of pending `notify` messages per protocol connection.
//! - [`McpServer`]: one MCP endpoint bound to one principal, one device surface and one
//!   tool surface ([`surfaces`]). It sends the `hello` itself at [`McpServer::open`],
//!   then answers JSON-RPC 2.0: `initialize`, `server/discover`, `ping`, `tools/list`,
//!   `tools/call`, `resources/list`, `resources/read` and (R5B, [`NotifyMode::Spec`])
//!   `subscriptions/listen` (and ignores other notifications).
//! - [`tools`]: the protocol tool list, generated from the protocol JSON Schema.
//! - [`surfaces`]: (R5B, M28) the tool surfaces `qp mcp serve --surface` chooses from;
//!   [`common`]: the tools every served surface gets (`render`, `notifications`).
//! - [`calllog`]: (R5B) the measurement log of tool calls.
//! - [`conform`]: the conformance transcript runner over MCP.
//!
//! The thin surface adds no state to the protocol: every tool call becomes exactly one
//! `qp/0` client message handed to [`qp_session::Session::handle_raw`], and the reply
//! message is the tool result.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use qp_schema::protocol::Surface;
use qp_session::{ReplayHost, Session};
use serde_json::{json, Map, Value};

pub mod calllog;
pub mod common;
pub mod conform;
pub mod surfaces;
pub mod tools;

pub use surfaces::{Ctx, ToolDef, ToolSurface};

/// Modern MCP revisions served statelessly (per-request `_meta`).
pub const MODERN_VERSIONS: [&str; 1] = ["2026-07-28"];
/// Legacy MCP revisions served after an `initialize` handshake.
pub const LEGACY_VERSIONS: [&str; 2] = ["2025-11-25", "2025-06-18"];
/// `_meta` key of a tool call carrying the `qp/0` request id to use.
pub const META_REQUEST_ID: &str = "dev.qapsule/id";
/// `_meta` key on notifications: the hub's emission index (merges several streams).
pub const META_ORDER: &str = "dev.qapsule/order";
/// `_meta` key carrying the `welcome` message of the adapter's `hello`.
pub const META_WELCOME: &str = "dev.qapsule/welcome";

/// A pending `notify` message for one connection.
#[derive(Debug, Clone)]
pub struct Queued {
    /// Emission index across the whole hub (strictly increasing).
    pub order: u64,
    /// The protocol message (a `ServerMsg`, normally `notify`).
    pub msg: Value,
}

/// The authority behind a hub: a bare session, or (R4, §21) a replay host whose
/// `replay` messages it answers itself. (A plain session is kept inline: the hub is
/// built once; boxing it would only add an indirection on every call.)
#[allow(clippy::large_enum_variant)]
enum Authority {
    Plain(Session),
    Replay(Box<ReplayHost>),
}

/// One session shared by several adapters (and raw connections).
pub struct Hub {
    session: Authority,
    queues: BTreeMap<String, Vec<Queued>>,
    order: u64,
}

/// Shared handle to a [`Hub`]. Single-threaded: adapters run on one thread.
pub type SharedHub = Rc<RefCell<Hub>>;

impl Hub {
    /// Wraps a session.
    #[must_use]
    pub fn new(session: Session) -> Self {
        Self {
            session: Authority::Plain(session),
            queues: BTreeMap::new(),
            order: 0,
        }
    }

    /// Wraps a session in a shared handle.
    #[must_use]
    pub fn shared(session: Session) -> SharedHub {
        Rc::new(RefCell::new(Self::new(session)))
    }

    /// (R4, §21) Wraps a replay host: `replay` tool calls reach it.
    #[must_use]
    pub fn with_replay(host: ReplayHost) -> Self {
        Self {
            session: Authority::Replay(Box::new(host)),
            queues: BTreeMap::new(),
            order: 0,
        }
    }

    /// [`Hub::with_replay`] in a shared handle.
    #[must_use]
    pub fn shared_replay(host: ReplayHost) -> SharedHub {
        Rc::new(RefCell::new(Self::with_replay(host)))
    }

    /// The session.
    #[must_use]
    pub fn session(&self) -> &Session {
        match &self.session {
            Authority::Plain(s) => s,
            Authority::Replay(h) => h.session(),
        }
    }

    /// The session, mutable (host-side reads such as compiling a scene).
    pub fn session_mut(&mut self) -> &mut Session {
        match &mut self.session {
            Authority::Plain(s) => s,
            Authority::Replay(h) => h.session_mut(),
        }
    }

    /// The authority's epoch (a backward replay seek starts a new authority, §21.3).
    #[must_use]
    pub fn epoch(&self) -> u64 {
        match &self.session {
            Authority::Plain(_) => 0,
            Authority::Replay(h) => h.state().epoch,
        }
    }

    /// (R5B) Marks `conn` as an agent's connection (PLAN §1): every MCP connection of
    /// `qp mcp serve` is one, and the client cannot undo it. The single place the
    /// adapter tells the authority.
    pub fn mark_agent_connection(&mut self, conn: &str) {
        self.session_mut().mark_agent_connection(conn);
    }

    /// Queues output lines of the host's own actions (replay steps, host controls, a
    /// co-hosted shell's messages) by their connection.
    pub fn queue(&mut self, lines: Vec<qp_schema::protocol::OutboundLine>) {
        for line in lines {
            let msg = serde_json::to_value(&line.msg).unwrap_or(Value::Null);
            self.order += 1;
            self.queues.entry(line.conn).or_default().push(Queued {
                order: self.order,
                msg,
            });
        }
    }

    /// (R5B) The replay moves with the wall clock (`ReplayHost::pump`): takes the due
    /// steps and queues their notifications. Nothing without a replay host.
    pub fn pump(&mut self, wall_ms: u64) {
        let lines = match &mut self.session {
            Authority::Plain(_) => return,
            Authority::Replay(h) => h.pump(wall_ms),
        };
        self.queue(lines);
    }

    /// Wall milliseconds until the next replay step is due (`ReplayHost::next_due_in`);
    /// `Some(0)` when a running replay has no anchor yet; `None` when nothing is due.
    #[must_use]
    pub fn next_due_in(&self, wall_ms: u64) -> Option<u64> {
        match &self.session {
            Authority::Plain(_) => None,
            Authority::Replay(h) => h.next_due_in(wall_ms).or_else(|| {
                let st = h.state();
                (!st.paused && !st.done).then_some(0)
            }),
        }
    }

    /// Applies a replay control as the host (no connection; e.g. `--seek`) and queues
    /// the notifications its steps caused.
    ///
    /// # Errors
    /// No replay host, or the control is invalid.
    pub fn host_control(
        &mut self,
        control: &qp_schema::protocol::ReplayControl,
    ) -> Result<qp_schema::protocol::ReplayState, String> {
        let Authority::Replay(h) = &mut self.session else {
            return Err("no replay: the session has no `init.replay`".to_owned());
        };
        let (st, lines) = h
            .host_control(control)
            .map_err(|e| serde_json::to_string(&e).unwrap_or(e.message))?;
        self.queue(lines);
        Ok(st)
    }

    /// The replay host, if the hub has one.
    pub fn replay_host(&mut self) -> Option<&mut ReplayHost> {
        match &mut self.session {
            Authority::Plain(_) => None,
            Authority::Replay(h) => Some(h),
        }
    }

    /// Hands raw message text on `conn` to the session. Returns the messages for `conn`
    /// that are not `notify` (the reply), in order. Every other output line (`notify`
    /// for `conn`, anything for other connections) is queued by its connection.
    pub fn send(&mut self, conn: &str, text: &str) -> Vec<Value> {
        let mut reply = Vec::new();
        let lines = match &mut self.session {
            Authority::Plain(s) => s.handle_raw(conn, text),
            Authority::Replay(h) => h.handle_raw(conn, text),
        };
        for line in lines {
            let msg = serde_json::to_value(&line.msg).unwrap_or(Value::Null);
            let is_notify = msg.get("type").and_then(Value::as_str) == Some("notify");
            self.order += 1;
            if line.conn == conn && !is_notify {
                reply.push(msg);
            } else {
                self.queues.entry(line.conn).or_default().push(Queued {
                    order: self.order,
                    msg,
                });
            }
        }
        reply
    }

    /// The host's data path (§18.4; not an MCP tool, agents cannot inject data): hands a
    /// data event to the session and queues every output line by its connection.
    ///
    /// # Errors
    /// The session rejected the event.
    pub fn data_event(
        &mut self,
        ev: &qp_schema::data::DataEvent,
    ) -> Result<(), qp_session::DataError> {
        let lines = match &mut self.session {
            Authority::Plain(s) => s.data_event(ev)?,
            Authority::Replay(h) => h.data_event(ev)?,
        };
        for line in lines {
            let msg = serde_json::to_value(&line.msg).unwrap_or(Value::Null);
            self.order += 1;
            self.queues.entry(line.conn).or_default().push(Queued {
                order: self.order,
                msg,
            });
        }
        Ok(())
    }

    /// Takes the pending messages of `conn`.
    pub fn take(&mut self, conn: &str) -> Vec<Queued> {
        self.queues.remove(conn).unwrap_or_default()
    }

    /// Connections with pending messages.
    #[must_use]
    pub fn pending_conns(&self) -> Vec<String> {
        self.queues.keys().cloned().collect()
    }
}

/// Who the adapter speaks as; becomes its `hello`.
#[derive(Debug, Clone)]
pub struct Config {
    /// Principal id.
    pub principal: String,
    /// Surface.
    pub surface: Surface,
    /// Protocol version string sent in `hello` (normally `"qp/0"`).
    pub protocol: String,
    /// Protocol features requested in `hello`.
    pub features: Vec<String>,
}

impl Config {
    /// `qp/0`, no features.
    #[must_use]
    pub fn new(principal: &str, surface: Surface) -> Self {
        Self {
            principal: principal.to_owned(),
            surface,
            protocol: "qp/0".to_owned(),
            features: Vec::new(),
        }
    }

    fn hello(&self) -> Value {
        let mut h = json!({
            "type": "hello",
            "protocol": self.protocol,
            "principal": self.principal,
            "surface": self.surface,
        });
        if !self.features.is_empty() {
            h["features"] = json!(self.features);
        }
        h
    }
}

/// A JSON-RPC error.
#[derive(Debug, Clone, PartialEq)]
pub struct RpcError {
    /// Code.
    pub code: i64,
    /// Message.
    pub message: String,
    /// Optional data.
    pub data: Option<Value>,
}

impl RpcError {
    /// An error without data.
    pub fn new(code: i64, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    /// The JSON-RPC `error` object.
    #[must_use]
    pub fn to_json(&self) -> Value {
        let mut e = json!({"code": self.code, "message": self.message});
        if let Some(d) = &self.data {
            e["data"] = d.clone();
        }
        e
    }
}

/// URI of the resource holding an endpoint's pending notifications.
pub const NOTIFY_URI: &str = "qapsule://notifications";
/// Most notifications an endpoint keeps unread (the oldest are dropped and counted).
pub const INBOX_CAP: usize = 10_000;
/// `ttlMs` of list results (the lists do not change while the server runs).
pub const LIST_TTL_MS: u64 = 300_000;

/// An endpoint's unread notifications (bounded).
#[derive(Debug, Default)]
pub struct Inbox {
    items: std::collections::VecDeque<Queued>,
    dropped: u64,
}

impl Inbox {
    /// Adds notifications (dropping the oldest beyond [`INBOX_CAP`]).
    pub fn extend(&mut self, q: impl IntoIterator<Item = Queued>) {
        for x in q {
            if self.items.len() == INBOX_CAP {
                self.items.pop_front();
                self.dropped += 1;
            }
            self.items.push_back(x);
        }
    }

    /// Takes every unread notification, oldest first.
    pub fn drain(&mut self) -> Vec<Queued> {
        self.items.drain(..).collect()
    }

    /// Unread count.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// No unread notification.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The newest unread notification's hub order.
    #[must_use]
    pub fn last_order(&self) -> Option<u64> {
        self.items.back().map(|q| q.order)
    }

    /// Notifications dropped unread so far.
    #[must_use]
    pub fn dropped(&self) -> u64 {
        self.dropped
    }
}

/// How an endpoint delivers notifications.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum NotifyMode {
    /// RD1 behaviour (in-process adapters, the conformance runner): every pending
    /// notification goes out as `notifications/message` before the next response.
    #[default]
    Inline,
    /// (R5B, `qp mcp serve`) By the MCP revision the client speaks: a legacy client
    /// (`initialize` with 2025-11-25/2025-06-18) gets `notifications/message` on its
    /// stream ([`McpServer::deliver`]); a modern client (2026-07-28) gets
    /// `notifications/resources/updated` for [`NOTIFY_URI`] on its `subscriptions/listen`
    /// stream and reads the resource (or calls the `notifications` tool), and
    /// `notifications/message` only inline for a request that gave a `logLevel`.
    Spec,
}

/// Options of an endpoint.
#[derive(Clone, Default)]
pub struct Options {
    /// Notification delivery.
    pub notify: NotifyMode,
    /// Offer the common tools ([`common`]: `render`, `notifications`).
    pub common_tools: bool,
    /// Measurement log (one line per tool call).
    pub log: Option<calllog::SharedLog>,
}

impl std::fmt::Debug for Options {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Options")
            .field("notify", &self.notify)
            .field("common_tools", &self.common_tools)
            .field("log", &self.log.is_some())
            .finish()
    }
}

/// What [`McpServer::deliver`] has for the client.
#[derive(Debug, Default)]
pub struct Delivery {
    /// `notifications/message` (legacy clients), oldest first.
    pub messages: Vec<Value>,
    /// `notifications/resources/updated` for the open `subscriptions/listen`, if new
    /// notifications arrived since the last signal.
    pub updated: Option<Value>,
}

/// One MCP endpoint over a protocol connection.
pub struct McpServer {
    hub: SharedHub,
    conn: String,
    config: Config,
    welcome: Value,
    epoch: u64,
    next_id: u64,
    surface: Box<dyn ToolSurface>,
    opts: Options,
    /// The legacy revision negotiated by `initialize`, if any.
    legacy: Option<String>,
    inbox: Inbox,
    /// The open `subscriptions/listen` request id.
    listen: Option<Value>,
    /// Hub order of the newest notification signalled to the listener.
    signalled: u64,
    /// Measurement run label (`X-Qp-Run`).
    run: Option<String>,
}

fn hello_reply(hub: &SharedHub, conn: &str, config: &Config) -> Result<Value, Value> {
    let reply = hub.borrow_mut().send(conn, &config.hello().to_string());
    let first = reply
        .into_iter()
        .next()
        .unwrap_or_else(|| json!({"type":"error","code":"internal","message":"no reply to hello"}));
    if first.get("type").and_then(Value::as_str) == Some("welcome") {
        Ok(first)
    } else {
        Err(first)
    }
}

impl McpServer {
    /// Opens protocol connection `conn` on the hub and sends the configured `hello`:
    /// the thin surface, RD1 notification delivery, no common tools.
    ///
    /// # Errors
    /// The authority's reply when it is not `welcome` (normally a `handshake` error).
    pub fn connect(hub: SharedHub, conn: &str, config: Config) -> Result<Self, Value> {
        Self::open(
            hub,
            conn,
            config,
            surfaces::thin::with_decisions(),
            Options::default(),
        )
    }

    /// Opens protocol connection `conn` with a tool surface and options. With
    /// [`NotifyMode::Spec`] (the served endpoint) the connection is an agent's
    /// connection ([`Hub::mark_agent_connection`]).
    ///
    /// # Errors
    /// The authority's reply when it is not `welcome`.
    pub fn open(
        hub: SharedHub,
        conn: &str,
        config: Config,
        surface: Box<dyn ToolSurface>,
        opts: Options,
    ) -> Result<Self, Value> {
        let welcome = hello_reply(&hub, conn, &config)?;
        if opts.notify == NotifyMode::Spec {
            hub.borrow_mut().mark_agent_connection(conn);
        }
        let epoch = hub.borrow().epoch();
        Ok(Self {
            hub,
            conn: conn.to_owned(),
            config,
            welcome,
            epoch,
            next_id: 0,
            surface,
            opts,
            legacy: None,
            inbox: Inbox::default(),
            listen: None,
            signalled: 0,
            run: None,
        })
    }

    /// The `welcome` the authority answered the adapter's `hello` with.
    #[must_use]
    pub fn welcome(&self) -> &Value {
        &self.welcome
    }

    /// The protocol connection id.
    #[must_use]
    pub fn conn(&self) -> &str {
        &self.conn
    }

    /// The principal.
    #[must_use]
    pub fn principal(&self) -> &str {
        &self.config.principal
    }

    /// The surface's name.
    #[must_use]
    pub fn surface_name(&self) -> &'static str {
        self.surface.name()
    }

    /// The measurement run label of the next tool calls (`X-Qp-Run`).
    pub fn set_run(&mut self, run: Option<String>) {
        self.run = run;
    }

    /// The legacy revision negotiated by `initialize`, if any.
    #[must_use]
    pub fn legacy(&self) -> Option<&str> {
        self.legacy.as_deref()
    }

    /// Marks the endpoint as a legacy client's (its session said `initialize`).
    pub fn set_legacy(&mut self, version: Option<String>) {
        self.legacy = version;
    }

    /// A `subscriptions/listen` is open.
    #[must_use]
    pub fn listening(&self) -> bool {
        self.listen.is_some()
    }

    /// Closes the open `subscriptions/listen` (its stream ended).
    pub fn stop_listening(&mut self) {
        self.listen = None;
    }

    fn pull(&mut self) {
        let q = self.hub.borrow_mut().take(&self.conn);
        self.inbox.extend(q);
    }

    /// Takes the pending `notify` messages of this connection (in-process API).
    pub fn take_notifies(&mut self) -> Vec<Queued> {
        self.pull();
        self.inbox.drain()
    }

    /// Takes the pending `notify` messages as MCP `notifications/message`.
    pub fn take_notifications(&mut self) -> Vec<Value> {
        self.take_notifies().into_iter().map(notification).collect()
    }

    /// Unread notifications.
    pub fn pending(&mut self) -> usize {
        self.pull();
        self.inbox.len()
    }

    /// ([`NotifyMode::Spec`]) What to send the client now: for a legacy client with a
    /// stream to send on (`legacy_stream`), every pending notification as
    /// `notifications/message` (taken); for an open `subscriptions/listen`, one
    /// `notifications/resources/updated` when notifications arrived since the last
    /// signal (they stay unread until the resource is read).
    pub fn deliver(&mut self, legacy_stream: bool) -> Delivery {
        self.pull();
        let mut d = Delivery::default();
        if self.legacy.is_some() && legacy_stream {
            d.messages = self.inbox.drain().into_iter().map(notification).collect();
            return d;
        }
        if let (Some(id), Some(last)) = (&self.listen, self.inbox.last_order()) {
            if last > self.signalled {
                self.signalled = last;
                d.updated = Some(json!({
                    "jsonrpc": "2.0",
                    "method": "notifications/resources/updated",
                    "params": {
                        "uri": NOTIFY_URI,
                        "_meta": {"io.modelcontextprotocol/subscriptionId": id},
                    }
                }));
            }
        }
        d
    }

    /// Handles one stdio line; returns the lines to write (notifications first, then
    /// the response, if any).
    pub fn handle_line(&mut self, line: &str) -> Vec<String> {
        let out = match serde_json::from_str::<Value>(line) {
            Ok(v) => self.handle_message(&v),
            Err(e) => vec![error_response(
                &Value::Null,
                &RpcError::new(-32700, format!("parse error: {e}")),
            )],
        };
        out.iter().map(Value::to_string).collect()
    }

    /// Handles one JSON-RPC message; returns the messages to send (pending
    /// notifications first, as the mode says, then the response to a request).
    pub fn handle_message(&mut self, msg: &Value) -> Vec<Value> {
        let Some(obj) = msg.as_object() else {
            let why = if msg.is_array() {
                "JSON-RPC batches are not supported"
            } else {
                "a JSON-RPC message must be an object"
            };
            return vec![error_response(&Value::Null, &RpcError::new(-32600, why))];
        };
        let id = obj.get("id").cloned();
        if obj.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
            return vec![error_response(
                &id.unwrap_or(Value::Null),
                &RpcError::new(-32600, "`jsonrpc` must be \"2.0\""),
            )];
        }
        let Some(method) = obj.get("method").and_then(Value::as_str) else {
            // A response from the client (we never send requests): ignore.
            if id.is_some() && (obj.contains_key("result") || obj.contains_key("error")) {
                return Vec::new();
            }
            return vec![error_response(
                &id.unwrap_or(Value::Null),
                &RpcError::new(-32600, "missing `method`"),
            )];
        };
        let empty = Value::Object(Map::new());
        let params = obj.get("params").unwrap_or(&empty);
        let Some(id) = id.filter(|i| i.is_string() || i.is_i64() || i.is_u64()) else {
            // Notifications (`notifications/initialized`, `notifications/cancelled`, …)
            // need no answer; a null id is not a valid request id.
            if obj.contains_key("id") {
                return vec![error_response(
                    &Value::Null,
                    &RpcError::new(-32600, "request id must be a string or an integer"),
                )];
            }
            if method == "notifications/cancelled"
                && self.listen.is_some()
                && params.get("requestId") == self.listen.as_ref()
            {
                self.listen = None;
            }
            return Vec::new();
        };
        if method == "subscriptions/listen" && self.opts.notify == NotifyMode::Spec {
            return match check_version(params) {
                Ok(()) => vec![self.listen(&id, params)],
                Err(e) => vec![error_response(&id, &e)],
            };
        }
        let started = std::time::Instant::now();
        let result = self.request(method, params);
        if method == "tools/call" {
            self.log_call(params, &result, started);
        }
        let mut out = match self.opts.notify {
            NotifyMode::Inline => self.take_notifications(),
            NotifyMode::Spec => {
                let log_level = params
                    .pointer("/_meta")
                    .and_then(|m| m.get("io.modelcontextprotocol/logLevel"))
                    .is_some();
                if log_level {
                    self.take_notifications()
                } else {
                    Vec::new()
                }
            }
        };
        if method == "initialize" && result.is_ok() {
            let v = params.get("protocolVersion").and_then(Value::as_str);
            self.legacy = Some(
                v.filter(|v| LEGACY_VERSIONS.contains(v))
                    .unwrap_or(LEGACY_VERSIONS[0])
                    .to_owned(),
            );
        }
        out.push(match result {
            Ok(r) => json!({"jsonrpc": "2.0", "id": id, "result": r}),
            Err(e) => error_response(&id, &e),
        });
        out
    }

    /// `subscriptions/listen`: acknowledges the accepted filter (only [`NOTIFY_URI`] can
    /// be subscribed; the lists never change) and keeps the request open.
    fn listen(&mut self, id: &Value, params: &Value) -> Value {
        let asked: Vec<&str> = params
            .pointer("/filter/resourceSubscriptions")
            .or_else(|| params.get("resourceSubscriptions"))
            .and_then(Value::as_array)
            .map_or_else(
                || vec![NOTIFY_URI],
                |a| a.iter().filter_map(Value::as_str).collect(),
            );
        let accepted: Vec<&str> = asked.into_iter().filter(|u| *u == NOTIFY_URI).collect();
        self.listen = Some(id.clone());
        self.pull();
        // Unread notifications are signalled at once by the next `deliver`.
        self.signalled = 0;
        json!({
            "jsonrpc": "2.0",
            "method": "notifications/subscriptions/acknowledged",
            "params": {
                "toolsListChanged": false,
                "promptsListChanged": false,
                "resourcesListChanged": false,
                "resourceSubscriptions": accepted,
                "_meta": {"io.modelcontextprotocol/subscriptionId": id},
            }
        })
    }

    fn log_call(
        &mut self,
        params: &Value,
        result: &Result<Value, RpcError>,
        started: std::time::Instant,
    ) {
        let Some(log) = &self.opts.log else { return };
        let tool = params.get("name").and_then(Value::as_str).unwrap_or("");
        let args_bytes = params.get("arguments").map_or(0, |a| a.to_string().len());
        let (ok, result_bytes) = match result {
            Ok(r) => (
                r.get("isError").and_then(Value::as_bool) != Some(true),
                r.to_string().len(),
            ),
            Err(e) => (false, e.to_json().to_string().len()),
        };
        let line = calllog::Line {
            run: self.run.clone(),
            principal: self.config.principal.clone(),
            tool: tool.to_owned(),
            args_bytes,
            result_bytes,
            ok,
            ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        };
        log.borrow_mut().write(&line);
    }

    /// (Spec) After a backward seek the authority is new (§21.3): say `hello` again on
    /// the same connection id. Subscriptions are not restored (the agent subscribes
    /// again; the replay state says `reset`).
    fn reconnect_if_reset(&mut self) {
        if self.opts.notify != NotifyMode::Spec {
            return;
        }
        let epoch = self.hub.borrow().epoch();
        if epoch == self.epoch {
            return;
        }
        if let Ok(w) = hello_reply(&self.hub, &self.conn, &self.config) {
            self.welcome = w;
            self.hub.borrow_mut().mark_agent_connection(&self.conn);
        }
        self.epoch = epoch;
    }

    fn request(&mut self, method: &str, params: &Value) -> Result<Value, RpcError> {
        if method != "initialize" {
            check_version(params)?;
        }
        match method {
            "initialize" => Ok(self.initialize(params)),
            "server/discover" => Ok(self.discover()),
            "ping" => Ok(json!({"resultType": "complete"})),
            "tools/list" => Ok(json!({
                "resultType": "complete",
                "tools": self.tool_defs().iter().map(ToolDef::to_mcp).collect::<Vec<_>>(),
                "ttlMs": LIST_TTL_MS,
                "cacheScope": "private",
            })),
            "resources/list" => Ok(json!({
                "resultType": "complete",
                "resources": [{
                    "uri": NOTIFY_URI,
                    "name": "notifications",
                    "title": "Pending qp/0 notifications",
                    "description": "Your unread qp/0 `notify` messages (subscriptions), \
                        oldest first; reading takes them.",
                    "mimeType": "application/json",
                }],
                "ttlMs": LIST_TTL_MS,
                "cacheScope": "private",
            })),
            "resources/templates/list" => Ok(json!({
                "resultType": "complete", "resourceTemplates": [],
                "ttlMs": LIST_TTL_MS, "cacheScope": "private",
            })),
            "resources/read" => {
                let uri = params.get("uri").and_then(Value::as_str).unwrap_or("");
                if uri != NOTIFY_URI {
                    return Err(RpcError {
                        code: -32002,
                        message: "Resource not found".to_owned(),
                        data: Some(json!({"uri": uri})),
                    });
                }
                self.reconnect_if_reset();
                let v = {
                    let mut ctx = self.ctx(None);
                    common::notifications_json(&mut ctx)
                };
                Ok(json!({
                    "resultType": "complete",
                    "contents": [{"uri": NOTIFY_URI, "mimeType": "application/json",
                                  "text": v.to_string()}],
                    "ttlMs": 0,
                    "cacheScope": "private",
                }))
            }
            "tools/call" => {
                let name = params
                    .get("name")
                    .and_then(Value::as_str)
                    .ok_or_else(|| RpcError::new(-32602, "tools/call needs a string `name`"))?;
                let args = params.get("arguments").cloned().unwrap_or(json!({}));
                let rid = params
                    .pointer("/_meta")
                    .and_then(|m| m.get(META_REQUEST_ID))
                    .and_then(Value::as_str)
                    .map(str::to_owned);
                self.call_tool(name, args, rid)
            }
            other => Err(RpcError::new(-32601, format!("method not found: {other}"))),
        }
    }

    /// The tools offered: the surface's, then the common ones it does not define.
    #[must_use]
    pub fn tool_defs(&self) -> Vec<ToolDef> {
        let mut t = self.surface.tools();
        if self.opts.common_tools {
            for c in common::tools() {
                if !t.iter().any(|x| x.name == c.name) {
                    t.push(c);
                }
            }
        }
        t
    }

    fn capabilities() -> Value {
        json!({
            "tools": {"listChanged": false},
            "logging": {},
            "resources": {"subscribe": true, "listChanged": false},
        })
    }

    fn server_info() -> Value {
        json!({"name": "qapsule", "title": "Qapsule qp/0", "version": env!("CARGO_PKG_VERSION")})
    }

    fn instructions(&self) -> String {
        self.surface.instructions(&self.config.principal)
    }

    fn initialize(&self, params: &Value) -> Value {
        let asked = params.get("protocolVersion").and_then(Value::as_str);
        let version = asked
            .filter(|v| LEGACY_VERSIONS.contains(v))
            .unwrap_or(LEGACY_VERSIONS[0]);
        json!({
            "resultType": "complete",
            "protocolVersion": version,
            "capabilities": Self::capabilities(),
            "serverInfo": Self::server_info(),
            "instructions": self.instructions(),
            "_meta": {META_WELCOME: self.welcome},
        })
    }

    fn discover(&self) -> Value {
        let mut versions: Vec<&str> = MODERN_VERSIONS.to_vec();
        versions.extend(LEGACY_VERSIONS);
        json!({
            "resultType": "complete",
            "supportedVersions": versions,
            "capabilities": Self::capabilities(),
            "instructions": self.instructions(),
            "_meta": {
                "io.modelcontextprotocol/serverInfo": Self::server_info(),
                META_WELCOME: self.welcome,
            },
        })
    }

    fn ctx(&mut self, request_id: Option<String>) -> Ctx<'_> {
        Ctx {
            hub: &self.hub,
            conn: &self.conn,
            principal: &self.config.principal,
            next_id: &mut self.next_id,
            request_id,
            inbox: &mut self.inbox,
        }
    }

    /// Builds the protocol message a thin tool call stands for.
    ///
    /// # Errors
    /// Unknown tool or non-object arguments (`-32602`).
    pub fn build_message(
        &mut self,
        name: &str,
        args: Value,
        request_id: Option<String>,
    ) -> Result<Value, RpcError> {
        let mut ctx = self.ctx(request_id);
        surfaces::thin::build_message(name, args, || ctx.request_id())
    }

    /// Calls a tool of the surface (or a common tool) and returns the MCP
    /// `CallToolResult` (for the thin surface: `structuredContent` = the protocol reply,
    /// `content` = the same JSON as text, `isError` for a protocol `error`).
    ///
    /// # Errors
    /// Unknown tool or malformed arguments (`-32602`).
    pub fn call_tool(
        &mut self,
        name: &str,
        args: Value,
        request_id: Option<String>,
    ) -> Result<Value, RpcError> {
        self.reconnect_if_reset();
        let own = self.surface.tools().iter().any(|t| t.name == name);
        let common = self.opts.common_tools && !own;
        let mut ctx = Ctx {
            hub: &self.hub,
            conn: &self.conn,
            principal: &self.config.principal,
            next_id: &mut self.next_id,
            request_id,
            inbox: &mut self.inbox,
        };
        if common {
            if let Some(r) = common::call(&mut ctx, name, &args) {
                return r;
            }
        }
        self.surface.call(&mut ctx, name, args)
    }
}

/// `CallToolResult` for a protocol reply.
#[must_use]
pub fn tool_result(reply: Value) -> Value {
    let is_error = reply.get("type").and_then(Value::as_str) == Some("error");
    json!({
        "resultType": "complete",
        "content": [{"type": "text", "text": reply.to_string()}],
        "structuredContent": reply,
        "isError": is_error,
    })
}

/// MCP `notifications/message` carrying a protocol `notify`.
#[must_use]
pub fn notification(q: Queued) -> Value {
    json!({
        "jsonrpc": "2.0",
        "method": "notifications/message",
        "params": {
            "level": "info",
            "logger": "qapsule",
            "data": q.msg,
            "_meta": {META_ORDER: q.order},
        }
    })
}

fn error_response(id: &Value, e: &RpcError) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": e.to_json()})
}

/// Per-request protocol version (`_meta`), when given, must be one we serve.
fn check_version(params: &Value) -> Result<(), RpcError> {
    let Some(v) = params
        .pointer("/_meta")
        .and_then(|m| m.get("io.modelcontextprotocol/protocolVersion"))
    else {
        return Ok(());
    };
    let ok = v
        .as_str()
        .is_some_and(|s| MODERN_VERSIONS.contains(&s) || LEGACY_VERSIONS.contains(&s));
    if ok {
        return Ok(());
    }
    let mut supported: Vec<&str> = MODERN_VERSIONS.to_vec();
    supported.extend(LEGACY_VERSIONS);
    Err(RpcError {
        code: -32022,
        message: "Unsupported protocol version".to_owned(),
        data: Some(json!({"supported": supported, "requested": v})),
    })
}
