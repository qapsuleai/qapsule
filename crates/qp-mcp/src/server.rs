//! [`McpServer`]: one MCP endpoint bound to one principal, one device surface and one tool
//! surface; answers JSON-RPC 2.0 over a protocol connection.

use serde_json::{json, Map, Value};

use crate::*;

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

pub(crate) fn hello_reply(hub: &SharedHub, conn: &str, config: &Config) -> Result<Value, Value> {
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
    /// the thin surface with the decision tools, RD1 notification delivery, no common
    /// tools, an unmarked connection. In-process only (the conformance runner, tests);
    /// RD2/A62 removed the command that served it (`qp mcp --init --principal`).
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
        mut surface: Box<dyn ToolSurface>,
        opts: Options,
    ) -> Result<Self, Value> {
        if let Some(f) = &opts.capsule_format {
            surface.set_capsule_format(f);
        }
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
                    server_tools::notifications_json(&mut ctx)
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
            for c in server_tools::tools() {
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
            if let Some(r) = server_tools::call(&mut ctx, name, &args) {
                return r;
            }
        }
        let mut args = args;
        if let (Some(f), Value::Object(m)) = (&self.opts.capsule_format, &mut args) {
            let takes_format =
                self.surface.tools().iter().any(|t| {
                    t.name == name && t.input_schema.pointer("/properties/format").is_some()
                });
            if takes_format && !m.contains_key("format") {
                m.insert("format".into(), json!(f));
            }
        }
        self.surface.call(&mut ctx, name, args)
    }
}
