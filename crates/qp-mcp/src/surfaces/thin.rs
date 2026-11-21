//! (M28 a) The thin surface: the protocol mirrored one to one (RD1). The tools are the
//! ones [`crate::tools`] generates from the protocol schema (`op_<name>`, `query_<kind>`,
//! `replay_<kind>`, the generic message tools); a call becomes exactly one `qp/0`
//! message and its reply is the result. Behaviour as before R5B.

use serde_json::{json, Map, Value};

use super::{Annotations, Ctx, ToolDef, ToolSurface};
use crate::tools::{self, ToolKind};
use crate::{tool_result, RpcError};

/// The thin surface.
#[derive(Debug, Default)]
pub struct Thin {
    /// (R5B, §25.2) An agent's endpoint: the human decision tools are not offered (a
    /// call to one is an unknown tool; the authority refuses the op on any other path
    /// too, `human_only`).
    agent: bool,
}

/// The tools only a person may use: deciding proposals and findings (§25.2).
pub const HUMAN_DECISION_TOOLS: [&str; 4] = [
    "op_accept_proposal",
    "op_reject_proposal",
    "op_approve_finding",
    "op_reject_finding",
];

/// Registry constructor: the surface an agent's endpoint serves (`qp mcp serve`), with
/// no human decision tool.
#[must_use]
pub fn new() -> Box<dyn ToolSurface> {
    Box::new(Thin { agent: true })
}

/// The RD1 surface with every generated tool, for adapters a person speaks through
/// ([`crate::McpServer::connect`], the conformance runner).
#[must_use]
pub fn with_decisions() -> Box<dyn ToolSurface> {
    Box::new(Thin { agent: false })
}

/// The annotations of a generated tool: queries and status reads are read-only.
#[must_use]
pub fn annotations(t: &tools::Tool) -> Annotations {
    let read = match &t.kind {
        ToolKind::Query(_) => true,
        ToolKind::Replay(k) => k == "status",
        ToolKind::Message(ty) => ty == "query",
        ToolKind::Op(_) => false,
    };
    if read {
        Annotations::READ
    } else {
        Annotations::WRITE
    }
}

/// The protocol message a thin tool call stands for (`ids` gives the request id).
///
/// # Errors
/// Unknown tool or non-object arguments (`-32602`).
pub fn build_message(
    name: &str,
    args: Value,
    mut id: impl FnMut() -> String,
) -> Result<Value, RpcError> {
    let tool =
        tools::find(name).ok_or_else(|| RpcError::new(-32602, format!("Unknown tool: {name}")))?;
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
            cmd.insert("id".into(), json!(id()));
            Value::Object(cmd)
        }
        ToolKind::Query(kind) => {
            args.insert("kind".into(), json!(kind));
            json!({"type": "query", "id": id(), "query": args})
        }
        ToolKind::Replay(kind) => {
            args.insert("kind".into(), json!(kind));
            json!({"type": "replay", "id": id(), "control": args})
        }
        ToolKind::Message(ty) => {
            if !args.contains_key("id") {
                args.insert("id".into(), json!(id()));
            }
            args.insert("type".into(), json!(ty));
            Value::Object(args)
        }
    };
    Ok(msg)
}

impl ToolSurface for Thin {
    fn name(&self) -> &'static str {
        "thin"
    }

    fn tools(&self) -> Vec<ToolDef> {
        tools::tools()
            .iter()
            .filter(|t| !(self.agent && HUMAN_DECISION_TOOLS.contains(&t.name.as_str())))
            .map(|t| ToolDef {
                name: t.name.clone(),
                description: t.description.clone(),
                input_schema: t.input_schema.clone(),
                annotations: annotations(t),
                output_schema: Some(tools::output_schema()),
            })
            .collect()
    }

    fn call(&mut self, ctx: &mut Ctx<'_>, name: &str, args: Value) -> Result<Value, RpcError> {
        if self.agent && HUMAN_DECISION_TOOLS.contains(&name) {
            return Err(RpcError::new(-32602, format!("Unknown tool: {name}")));
        }
        let msg = build_message(name, args, || ctx.request_id())?;
        Ok(tool_result(ctx.send(&msg)))
    }

    fn instructions(&self, principal: &str) -> String {
        format!(
            "Qapsule chart document authority (protocol qp/0). You speak as principal `{principal}`. \
             op_<name> tools change the document with one operation (use `command` for \
             several operations applied atomically); query_<kind> tools read it. \
             structuredContent is the qp/0 reply (`ack`, `result` or `error`); errors carry \
             `path`, `expected`, `did_you_mean` and `hint` for self-correction. \
             Subscriptions deliver qp/0 `notify` messages as notifications/message (params.data)."
        )
    }
}
