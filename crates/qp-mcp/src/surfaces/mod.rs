//! (R5B/T2, M28) Tool surfaces: what an MCP endpoint offers an agent, chosen with
//! `qp mcp serve --surface <name>` (`surfaces/README.md`).
//!
//! A surface is one file in this folder implementing [`ToolSurface`] and one line in
//! [`registry`]. It never talks to the session directly: every read and write goes
//! through [`Ctx`] as `qp/0` protocol messages on the endpoint's own connection, so the
//! authority checks the principal's permissions and entitlements exactly as for any
//! other client. Input schemas are generated from Rust types ([`input_schema`],
//! `schemars`), never written by hand.

use serde_json::{json, Value};

use crate::{Inbox, Queued, RpcError, SharedHub};

pub mod query;
pub mod task;
pub mod thin;

/// MCP tool annotations (hints only: the authority enforces permissions).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct Annotations {
    /// The tool does not change the document (`readOnlyHint`).
    pub read_only: bool,
    /// The tool may destroy data (`destructiveHint`); always false on our surfaces.
    pub destructive: bool,
    /// Calling twice with the same arguments has no further effect (`idempotentHint`).
    pub idempotent: bool,
    /// The tool reaches outside the session (`openWorldHint`); always false.
    pub open_world: bool,
}

impl Annotations {
    /// A query: read-only, idempotent, closed world.
    pub const READ: Self = Self {
        read_only: true,
        destructive: false,
        idempotent: true,
        open_world: false,
    };
    /// A write (a command, a proposal, a subscription): not read-only, not destructive.
    pub const WRITE: Self = Self {
        read_only: false,
        destructive: false,
        idempotent: false,
        open_world: false,
    };

    /// The MCP `annotations` object.
    #[must_use]
    pub fn to_mcp(self) -> Value {
        json!({
            "readOnlyHint": self.read_only,
            "destructiveHint": self.destructive,
            "idempotentHint": self.idempotent,
            "openWorldHint": self.open_world,
        })
    }
}

/// One tool a surface offers.
#[derive(Debug, Clone)]
pub struct ToolDef {
    /// Tool name (`[A-Za-z0-9_.-]`, at most 128 characters).
    pub name: String,
    /// What it does, for the model.
    pub description: String,
    /// JSON Schema of the arguments (self-contained; generate with [`input_schema`] or
    /// from the protocol schema).
    pub input_schema: Value,
    /// Hints.
    pub annotations: Annotations,
    /// Optional output schema (of `structuredContent`).
    pub output_schema: Option<Value>,
}

impl ToolDef {
    /// The MCP `Tool` object for `tools/list`.
    #[must_use]
    pub fn to_mcp(&self) -> Value {
        let mut t = json!({
            "name": self.name,
            "description": self.description,
            "inputSchema": self.input_schema,
            "annotations": self.annotations.to_mcp(),
        });
        if let Some(o) = &self.output_schema {
            t["outputSchema"] = o.clone();
        }
        t
    }
}

/// A tool surface (M28 alternative). Implementations live in `surfaces/<name>.rs`.
pub trait ToolSurface {
    /// The name `--surface` selects it by.
    fn name(&self) -> &'static str;
    /// The tools, in a fixed order (the same every call: clients cache the list).
    fn tools(&self) -> Vec<ToolDef>;
    /// Calls tool `name` (one of [`ToolSurface::tools`]) with `args`; returns the MCP
    /// `CallToolResult` (see [`crate::tool_result`] for a protocol reply). A protocol
    /// error is a tool result with `isError: true`, not an `Err`.
    ///
    /// # Errors
    /// Unknown tool or malformed arguments (`-32602`).
    fn call(&mut self, ctx: &mut Ctx<'_>, name: &str, args: Value) -> Result<Value, RpcError>;
    /// The `instructions` text of `initialize`/`server/discover`.
    fn instructions(&self, principal: &str) -> String {
        format!(
            "Qapsule chart document authority (protocol qp/0). You speak as principal \
             `{principal}`. Pending notifications: read the `qapsule://notifications` \
             resource (or the `notifications` tool when offered)."
        )
    }
}

/// Builds a surface.
pub type Factory = fn() -> Box<dyn ToolSurface>;

/// The surfaces, by name (the first is the default). Add a surface with one line here.
#[must_use]
pub fn registry() -> &'static [(&'static str, Factory)] {
    &[("thin", thin::new), ("task", task::new), ("query", query::new)]
}

/// The surface named `name`.
#[must_use]
pub fn by_name(name: &str) -> Option<Box<dyn ToolSurface>> {
    registry()
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, f)| f())
}

/// The registered names, in order.
#[must_use]
pub fn names() -> Vec<&'static str> {
    registry().iter().map(|(n, _)| *n).collect()
}

/// The input schema of tool arguments of type `T` (a `qp-schema` type or a small
/// argument struct deriving `JsonSchema`), self-contained (`$defs` inside), with the
/// meta-schema key removed.
#[must_use]
pub fn input_schema<T: schemars::JsonSchema>() -> Value {
    let mut v = serde_json::to_value(schemars::schema_for!(T)).unwrap_or(Value::Null);
    if let Some(o) = v.as_object_mut() {
        o.remove("$schema");
        o.entry("type").or_insert_with(|| json!("object"));
        o.entry("properties").or_insert_with(|| json!({}));
    }
    v
}

/// The `_meta` of a `tools/call` and what it says.
#[derive(Debug, Clone, Default)]
pub struct CallMeta {
    /// `_meta["dev.qapsule/id"]`: the `qp/0` request id to use for the call's message.
    pub request_id: Option<String>,
}

/// What a surface may do during one call: speak `qp/0` on the endpoint's connection and
/// read its pending notifications.
pub struct Ctx<'a> {
    pub(crate) hub: &'a SharedHub,
    pub(crate) conn: &'a str,
    pub(crate) principal: &'a str,
    pub(crate) next_id: &'a mut u64,
    pub(crate) request_id: Option<String>,
    pub(crate) inbox: &'a mut Inbox,
}

impl Ctx<'_> {
    /// The principal the endpoint speaks as.
    #[must_use]
    pub fn principal(&self) -> &str {
        self.principal
    }

    /// The protocol connection id.
    #[must_use]
    pub fn conn(&self) -> &str {
        self.conn
    }

    /// The hub (host-side reads such as compiling a scene; never for writes).
    #[must_use]
    pub fn hub(&self) -> &SharedHub {
        self.hub
    }

    /// A fresh request id (`mcp-<n>`).
    pub fn fresh_id(&mut self) -> String {
        *self.next_id += 1;
        format!("mcp-{}", self.next_id)
    }

    /// The call's request id (`_meta["dev.qapsule/id"]`) the first time, then fresh ids.
    pub fn request_id(&mut self) -> String {
        match self.request_id.take() {
            Some(id) => id,
            None => self.fresh_id(),
        }
    }

    /// Sends one protocol message on the connection and returns the reply (the first
    /// message for this connection that is not a `notify`; notifications are queued).
    pub fn send(&mut self, msg: &Value) -> Value {
        let reply = self.hub.borrow_mut().send(self.conn, &msg.to_string());
        reply.into_iter().next().unwrap_or_else(
            || json!({"type":"error","code":"internal","message":"the authority sent no reply"}),
        )
    }

    /// Sends a `query` (`query` is the query object with its `kind`).
    pub fn query(&mut self, query: Value) -> Value {
        let id = self.request_id();
        self.send(&json!({"type": "query", "id": id, "query": query}))
    }

    /// Sends a `command` with `ops` (atomic).
    pub fn command(&mut self, ops: Vec<Value>) -> Value {
        let id = self.request_id();
        self.send(&json!({"type": "command", "id": id, "ops": ops}))
    }

    /// Takes the pending notifications (`notify` messages) of the endpoint.
    pub fn take_notifications(&mut self) -> Vec<Queued> {
        let q = self.hub.borrow_mut().take(self.conn);
        self.inbox.extend(q);
        self.inbox.drain()
    }

    /// How many notifications were dropped because nobody read them (bounded inbox).
    #[must_use]
    pub fn dropped_notifications(&self) -> u64 {
        self.inbox.dropped()
    }
}
