//! The [`ToolSurface`] trait and what a surface describes its tools with ([`ToolDef`],
//! [`Annotations`], [`input_schema`]).

use serde_json::{json, Value};

use super::Ctx;
use crate::RpcError;

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
    /// (R5B/M29) The server's capsule presentation (`card`, `json`, `layered`;
    /// `qp mcp serve --capsule-format`). Surfaces whose tools take a `format` argument
    /// get it as that argument's default instead (see `McpServer::call_tool`); a surface
    /// whose tools do not (thin) may lay out its capsule replies in it. Default: ignored.
    fn set_capsule_format(&mut self, _format: &str) {}
    /// The `instructions` text of `initialize`/`server/discover`.
    fn instructions(&self, principal: &str) -> String {
        format!(
            "Qapsule chart document authority (protocol qp/0). You speak as principal \
             `{principal}`. Pending notifications: read the `qapsule://notifications` \
             resource (or the `notifications` tool when offered)."
        )
    }
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
