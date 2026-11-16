//! MCP server adapter over the `qp/0` reference authority (`rounds/RD1/EMBEDDING.md` §1).
//!
//! - [`Hub`]: one [`qp_session::Session`] shared by any number of adapters, with one
//!   queue of pending `notify` messages per protocol connection.
//! - [`McpServer`]: one MCP endpoint bound to one principal and surface. It sends the
//!   `hello` itself at [`McpServer::connect`], then answers JSON-RPC 2.0: `initialize`,
//!   `server/discover`, `ping`, `tools/list`, `tools/call` (and ignores notifications).
//! - [`tools`]: the tool list, generated from the protocol JSON Schema.
//! - [`conform`]: the conformance transcript runner over MCP.
//!
//! The adapter adds no state to the protocol: every tool call becomes exactly one
//! `qp/0` client message handed to [`qp_session::Session::handle_raw`], and the reply
//! message is the tool result.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use qp_schema::protocol::Surface;
use qp_session::{ReplayHost, Session};
use serde_json::{json, Map, Value};

pub mod conform;
pub mod tools;

use tools::ToolKind;

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
    fn new(code: i64, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    fn to_json(&self) -> Value {
        let mut e = json!({"code": self.code, "message": self.message});
        if let Some(d) = &self.data {
            e["data"] = d.clone();
        }
        e
    }
}

/// One MCP endpoint over a protocol connection.
pub struct McpServer {
    hub: SharedHub,
    conn: String,
    config: Config,
    welcome: Value,
    next_id: u64,
}

impl McpServer {
    /// Opens protocol connection `conn` on the hub and sends the configured `hello`.
    ///
    /// # Errors
    /// The authority's reply when it is not `welcome` (normally a `handshake` error).
    pub fn connect(hub: SharedHub, conn: &str, config: Config) -> Result<Self, Value> {
        let reply = hub.borrow_mut().send(conn, &config.hello().to_string());
        let first = reply.into_iter().next().unwrap_or_else(
            || json!({"type":"error","code":"internal","message":"no reply to hello"}),
        );
        if first.get("type").and_then(Value::as_str) != Some("welcome") {
            return Err(first);
        }
        Ok(Self {
            hub,
            conn: conn.to_owned(),
            config,
            welcome: first,
            next_id: 0,
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

    /// Takes the pending `notify` messages of this connection (in-process API).
    pub fn take_notifies(&mut self) -> Vec<Queued> {
        self.hub.borrow_mut().take(&self.conn)
    }

    /// Takes the pending `notify` messages as MCP `notifications/message`.
    pub fn take_notifications(&mut self) -> Vec<Value> {
        self.take_notifies().into_iter().map(notification).collect()
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
    /// notifications first, then the response to a request).
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
        let Some(id) = id.filter(|i| i.is_string() || i.is_i64() || i.is_u64()) else {
            // Notifications (`notifications/initialized`, `notifications/cancelled`, …)
            // need no answer; a null id is not a valid request id.
            if obj.contains_key("id") {
                return vec![error_response(
                    &Value::Null,
                    &RpcError::new(-32600, "request id must be a string or an integer"),
                )];
            }
            return Vec::new();
        };
        let empty = Value::Object(Map::new());
        let params = obj.get("params").unwrap_or(&empty);
        let result = self.request(method, params);
        let mut out = self.take_notifications();
        out.push(match result {
            Ok(r) => json!({"jsonrpc": "2.0", "id": id, "result": r}),
            Err(e) => error_response(&id, &e),
        });
        out
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
                "tools": tools::tools().iter().map(tools::Tool::to_mcp).collect::<Vec<_>>(),
            })),
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

    fn capabilities() -> Value {
        json!({"tools": {"listChanged": false}, "logging": {}})
    }

    fn server_info() -> Value {
        json!({"name": "qapsule", "title": "Qapsule qp/0", "version": env!("CARGO_PKG_VERSION")})
    }

    fn instructions(&self) -> String {
        format!(
            "Qapsule chart document authority (protocol qp/0). You speak as principal `{}`. \
             op_<name> tools change the document with one operation (use `command` for \
             several operations applied atomically); query_<kind> tools read it. \
             structuredContent is the qp/0 reply (`ack`, `result` or `error`); errors carry \
             `path`, `expected`, `did_you_mean` and `hint` for self-correction. \
             Subscriptions deliver qp/0 `notify` messages as notifications/message (params.data).",
            self.config.principal
        )
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

    fn fresh_id(&mut self) -> String {
        self.next_id += 1;
        format!("mcp-{}", self.next_id)
    }

    /// Builds the protocol message a tool call stands for.
    ///
    /// # Errors
    /// Unknown tool or non-object arguments (`-32602`).
    pub fn build_message(
        &mut self,
        name: &str,
        args: Value,
        request_id: Option<String>,
    ) -> Result<Value, RpcError> {
        let tool = tools::find(name)
            .ok_or_else(|| RpcError::new(-32602, format!("Unknown tool: {name}")))?;
        let Value::Object(mut args) = args else {
            return Err(RpcError::new(
                -32602,
                format!("tool `{name}`: `arguments` must be an object"),
            ));
        };
        let msg = match &tool.kind {
            ToolKind::Op(op) => {
                let mut cmd = Map::new();
                cmd.insert("type".into(), json!("command"));
                for extra in tools::COMMAND_EXTRAS {
                    if let Some(v) = args.remove(extra) {
                        cmd.insert(extra.into(), v);
                    }
                }
                args.insert("op".into(), json!(op));
                cmd.insert("ops".into(), json!([args]));
                let id = request_id.unwrap_or_else(|| self.fresh_id());
                cmd.insert("id".into(), json!(id));
                Value::Object(cmd)
            }
            ToolKind::Query(kind) => {
                args.insert("kind".into(), json!(kind));
                let id = request_id.unwrap_or_else(|| self.fresh_id());
                json!({"type": "query", "id": id, "query": args})
            }
            ToolKind::Replay(kind) => {
                args.insert("kind".into(), json!(kind));
                let id = request_id.unwrap_or_else(|| self.fresh_id());
                json!({"type": "replay", "id": id, "control": args})
            }
            ToolKind::Message(ty) => {
                if !args.contains_key("id") {
                    let id = request_id.unwrap_or_else(|| self.fresh_id());
                    args.insert("id".into(), json!(id));
                }
                args.insert("type".into(), json!(ty));
                Value::Object(args)
            }
        };
        Ok(msg)
    }

    /// Calls a tool: builds the protocol message, hands it to the session and returns the
    /// MCP `CallToolResult` (`structuredContent` = the protocol reply, `content` = the
    /// same JSON as text, `isError` for a protocol `error`).
    ///
    /// # Errors
    /// Unknown tool or non-object arguments (`-32602`).
    pub fn call_tool(
        &mut self,
        name: &str,
        args: Value,
        request_id: Option<String>,
    ) -> Result<Value, RpcError> {
        let msg = self.build_message(name, args, request_id)?;
        let reply = self.hub.borrow_mut().send(&self.conn, &msg.to_string());
        let first = reply.into_iter().next().unwrap_or_else(
            || json!({"type":"error","code":"internal","message":"the authority sent no reply"}),
        );
        Ok(tool_result(first))
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
