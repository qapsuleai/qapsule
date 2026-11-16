//! MCP tools generated from the protocol JSON Schema (`qp_schema::json_schema("ClientMsg")`).
//!
//! Nothing here names an operation or a query: one tool per `Op` variant
//! (`op_<name>`), one per `Query` variant (`query_<kind>`), one per `ReplayControl`
//! variant (`replay_<kind>`, R4 §21), and the generic message tools `command`, `query`,
//! `subscribe`, `unsubscribe`, `attend`, `replay`. Input schemas are
//! the variants' schemas with the tag removed; descriptions are the schema's.

use std::collections::BTreeSet;
use std::sync::OnceLock;

use serde_json::{json, Map, Value};

/// Command envelope fields offered on every `op_<name>` tool.
pub const COMMAND_EXTRAS: [&str; 3] = ["mode", "base", "reason"];

/// Client message types that get a generic tool, in tool order.
pub const GENERIC: [&str; 6] = [
    "command",
    "query",
    "subscribe",
    "unsubscribe",
    "attend",
    "replay",
];

/// What a tool call becomes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolKind {
    /// A one-op `command`; the op name.
    Op(String),
    /// A `query`; the query kind.
    Query(String),
    /// (R4, §21) A `replay` message; the control kind.
    Replay(String),
    /// A whole client message of this `type` (`command`, `query`, …); arguments are the
    /// message without `type`.
    Message(String),
}

/// One generated tool.
#[derive(Debug, Clone)]
pub struct Tool {
    /// Tool name.
    pub name: String,
    /// Description, from the schema.
    pub description: String,
    /// JSON Schema of the arguments (self-contained: carries the `$defs` it uses).
    pub input_schema: Value,
    /// What a call becomes.
    pub kind: ToolKind,
}

impl Tool {
    /// The MCP `Tool` object for `tools/list`.
    #[must_use]
    pub fn to_mcp(&self) -> Value {
        json!({
            "name": self.name,
            "description": self.description,
            "inputSchema": self.input_schema,
            "outputSchema": output_schema(),
        })
    }
}

/// Output schema of every tool: `structuredContent` is the protocol reply (`ack`,
/// `result` or `error`, a `ServerMsg`). Kept slim on purpose; the full `ServerMsg`
/// schema is exported by `qp schema export`.
#[must_use]
pub fn output_schema() -> Value {
    json!({
        "type": "object",
        "description": "The qp/0 reply message (a ServerMsg): `ack`, `result` or `error`.",
        "properties": {"type": {"type": "string", "enum": ["ack", "result", "error"]}},
        "required": ["type"]
    })
}

/// All tools in deterministic order: ops (schema order), queries (schema order), replay
/// controls (schema order), then [`GENERIC`]. Generated once per process.
#[must_use]
pub fn tools() -> &'static [Tool] {
    static TOOLS: OnceLock<Vec<Tool>> = OnceLock::new();
    TOOLS.get_or_init(generate)
}

/// The tool named `name`.
#[must_use]
pub fn find(name: &str) -> Option<&'static Tool> {
    tools().iter().find(|t| t.name == name)
}

fn variants(schema: &Value) -> &[Value] {
    schema
        .get("oneOf")
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

fn tag_of<'a>(variant: &'a Value, tag: &str) -> Option<&'a str> {
    variant
        .pointer(&format!("/properties/{tag}/const"))?
        .as_str()
}

fn description(v: &Value, fallback: &str) -> String {
    v.get("description")
        .and_then(Value::as_str)
        .map_or_else(|| fallback.to_owned(), str::to_owned)
}

/// Removes property `name` (and its `required` entry) from an object schema.
fn drop_prop(schema: &mut Value, name: &str, keep_property: bool) {
    if !keep_property {
        if let Some(p) = schema.get_mut("properties").and_then(Value::as_object_mut) {
            p.remove(name);
        }
    }
    if let Some(r) = schema.get_mut("required").and_then(Value::as_array_mut) {
        r.retain(|x| x.as_str() != Some(name));
    }
}

fn collect_refs(v: &Value, out: &mut BTreeSet<String>) {
    match v {
        Value::Object(o) => {
            if let Some(r) = o.get("$ref").and_then(Value::as_str) {
                if let Some(name) = r.strip_prefix("#/$defs/") {
                    out.insert(name.to_owned());
                }
            }
            for x in o.values() {
                collect_refs(x, out);
            }
        }
        Value::Array(a) => a.iter().for_each(|x| collect_refs(x, out)),
        _ => {}
    }
}

/// Attaches the `$defs` that `schema` reaches (transitively), so it stands alone.
fn attach_defs(mut schema: Value, defs: &Map<String, Value>) -> Value {
    let mut seen = BTreeSet::new();
    let mut todo = BTreeSet::new();
    collect_refs(&schema, &mut todo);
    while let Some(name) = todo.pop_first() {
        if !seen.insert(name.clone()) {
            continue;
        }
        if let Some(d) = defs.get(&name) {
            let mut more = BTreeSet::new();
            collect_refs(d, &mut more);
            todo.extend(more.into_iter().filter(|n| !seen.contains(n)));
        }
    }
    if !seen.is_empty() {
        let picked: Map<String, Value> = seen
            .into_iter()
            .filter_map(|n| defs.get(&n).map(|d| (n, d.clone())))
            .collect();
        if let Some(o) = schema.as_object_mut() {
            o.insert("$defs".to_owned(), Value::Object(picked));
        }
    }
    schema
}

fn ensure_properties(schema: &mut Value) {
    if let Some(o) = schema.as_object_mut() {
        o.entry("properties")
            .or_insert_with(|| Value::Object(Map::new()));
    }
}

fn generate() -> Vec<Tool> {
    let root = qp_schema::json_schema("ClientMsg").expect("ClientMsg schema exists");
    let empty = Map::new();
    let defs = root
        .get("$defs")
        .and_then(Value::as_object)
        .unwrap_or(&empty);
    let client = |ty: &str| {
        variants(&root)
            .iter()
            .find(|v| tag_of(v, "type") == Some(ty))
            .cloned()
            .unwrap_or_else(|| panic!("ClientMsg has no `{ty}` variant"))
    };
    let command = client("command");
    let mut tools = Vec::new();

    for v in defs.get("Op").map(variants).unwrap_or_default() {
        let Some(name) = tag_of(v, "op") else {
            continue;
        };
        let mut s = v.clone();
        drop_prop(&mut s, "op", false);
        ensure_properties(&mut s);
        let props = s
            .get_mut("properties")
            .and_then(Value::as_object_mut)
            .expect("object schema");
        for extra in COMMAND_EXTRAS {
            // An op field of the same name would win; the tools test checks there is none.
            if props.contains_key(extra) {
                continue;
            }
            if let Some(p) = command.pointer(&format!("/properties/{extra}")) {
                props.insert(extra.to_owned(), p.clone());
            }
        }
        tools.push(Tool {
            name: format!("op_{name}"),
            description: description(v, name),
            input_schema: attach_defs(s, defs),
            kind: ToolKind::Op(name.to_owned()),
        });
    }

    for v in defs.get("Query").map(variants).unwrap_or_default() {
        let Some(kind) = tag_of(v, "kind") else {
            continue;
        };
        let mut s = v.clone();
        drop_prop(&mut s, "kind", false);
        ensure_properties(&mut s);
        tools.push(Tool {
            name: format!("query_{kind}"),
            description: description(v, kind),
            input_schema: attach_defs(s, defs),
            kind: ToolKind::Query(kind.to_owned()),
        });
    }

    for v in defs.get("ReplayControl").map(variants).unwrap_or_default() {
        let Some(kind) = tag_of(v, "kind") else {
            continue;
        };
        let mut s = v.clone();
        drop_prop(&mut s, "kind", false);
        ensure_properties(&mut s);
        tools.push(Tool {
            name: format!("replay_{kind}"),
            description: format!(
                "{} (A qp/0 `replay` message, docs/PROTOCOL.md §21.)",
                description(v, kind)
            ),
            input_schema: attach_defs(s, defs),
            kind: ToolKind::Replay(kind.to_owned()),
        });
    }

    for ty in GENERIC {
        let v = client(ty);
        let mut s = v.clone();
        drop_prop(&mut s, "type", false);
        // The request id is optional over MCP: generated if absent.
        drop_prop(&mut s, "id", true);
        tools.push(Tool {
            name: ty.to_owned(),
            description: format!(
                "{} (The whole qp/0 `{ty}` message without `type`; `id` is generated if absent.)",
                description(&v, ty)
            ),
            input_schema: attach_defs(s, defs),
            kind: ToolKind::Message(ty.to_owned()),
        });
    }
    tools
}
