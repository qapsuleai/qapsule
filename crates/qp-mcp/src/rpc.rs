//! JSON-RPC and MCP wire pieces: served revisions, `_meta` keys, [`RpcError`], tool
//! results and notification messages.

use serde_json::{json, Value};

use crate::Queued;

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

/// `ttlMs` of list results (the lists do not change while the server runs).
pub const LIST_TTL_MS: u64 = 300_000;

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

pub(crate) fn error_response(id: &Value, e: &RpcError) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": e.to_json()})
}

/// Per-request protocol version (`_meta`), when given, must be one we serve.
pub(crate) fn check_version(params: &Value) -> Result<(), RpcError> {
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
