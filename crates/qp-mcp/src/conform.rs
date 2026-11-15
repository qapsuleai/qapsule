//! Conformance transcripts over MCP (`qp conform … --transport mcp`).
//!
//! Every transcript connection gets its own [`McpServer`] over the same in-process
//! [`Hub`], so multi-party transcripts work. Translation of a transcript step:
//!
//! - `hello` creates the adapter for that connection; the adapter sends its own
//!   `hello` and the step's output is its `welcome` (or the authority's error, and no
//!   adapter is kept).
//! - `command` with exactly one op → `tools/call op_<name>`; any other `command` →
//!   `tools/call command`; `query` → `tools/call query_<kind>` (or `query` for an
//!   unknown kind); `subscribe`, `unsubscribe`, `attend` → their tools. The request id
//!   travels in `_meta["dev.qapsule/id"]` (op/query tools) or the `id` argument.
//! - Not carried over MCP, handed to the session directly: `input` (stays in the core),
//!   `clock` (host only), `send_raw` (not a JSON message), any message before the
//!   connection's `hello`, and messages whose `id` is not a string (an MCP tool call
//!   always has a request id).
//!
//! Tool results are turned back into protocol messages (`structuredContent`, checked
//! against `content[0].text` and `isError`), notifications back into `notify` messages
//! (merged across connections by their `dev.qapsule/order`), and the step is matched
//! with the same expectations as every other transport.

use std::collections::BTreeMap;
use std::path::Path;

use qp_schema::protocol::ClientMsg;
use qp_session::conform::{
    load_init, read_transcript, run_steps, FinalState, TranscriptOutcome, Transport,
};
use qp_session::Session;
use serde_json::{json, Map, Value};

use crate::tools::{self, COMMAND_EXTRAS};
use crate::{Config, Hub, McpServer, SharedHub, META_ORDER, META_REQUEST_ID, MODERN_VERSIONS};

/// A notification turned back into a protocol line: (hub order, conn, `notify`).
type Note = (u64, String, Value);

/// How the steps of a transcript were carried.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Stats {
    /// `hello` steps turned into adapters.
    pub hellos: usize,
    /// Steps sent as `tools/call`.
    pub tool_calls: usize,
    /// Steps handed to the session directly (not carried over MCP).
    pub direct: usize,
}

/// MCP transport for the conformance runner.
pub struct McpTransport {
    hub: SharedHub,
    adapters: BTreeMap<String, McpServer>,
    rpc_id: u64,
    /// How steps were carried so far.
    pub stats: Stats,
}

impl McpTransport {
    /// A transport over `session`.
    #[must_use]
    pub fn new(session: Session) -> Self {
        Self {
            hub: Hub::shared(session),
            adapters: BTreeMap::new(),
            rpc_id: 0,
            stats: Stats::default(),
        }
    }

    fn direct(&mut self, conn: &str, text: &str) -> Vec<Value> {
        self.stats.direct += 1;
        self.hub.borrow_mut().send(conn, text)
    }

    fn hello(&mut self, conn: &str, msg: &Value) -> Option<Value> {
        let Ok(ClientMsg::Hello {
            protocol,
            principal,
            surface,
            features,
        }) = serde_json::from_value::<ClientMsg>(msg.clone())
        else {
            return None;
        };
        self.stats.hellos += 1;
        let config = Config {
            principal,
            surface,
            protocol,
            features,
        };
        Some(match McpServer::connect(self.hub.clone(), conn, config) {
            Ok(server) => {
                let w = server.welcome().clone();
                self.adapters.insert(conn.to_owned(), server);
                w
            }
            Err(e) => e,
        })
    }

    /// Calls a tool on `conn`'s adapter through JSON-RPC; returns the reply message and
    /// the notifications that came with it.
    fn call(
        &mut self,
        conn: &str,
        name: &str,
        args: Value,
        rid: &str,
    ) -> Result<(Value, Vec<Note>), String> {
        self.rpc_id += 1;
        let req = json!({
            "jsonrpc": "2.0",
            "id": self.rpc_id,
            "method": "tools/call",
            "params": {
                "name": name,
                "arguments": args,
                "_meta": {
                    META_REQUEST_ID: rid,
                    "io.modelcontextprotocol/protocolVersion": MODERN_VERSIONS[0],
                    "io.modelcontextprotocol/clientCapabilities": {},
                },
            },
        });
        let server = self.adapters.get_mut(conn).ok_or("no adapter")?;
        let mut reply = None;
        let mut notes = Vec::new();
        for m in server.handle_message(&req) {
            if m.get("method").is_some() {
                notes.push(unwrap_notification(conn, &m)?);
            } else if m.get("id") == Some(&json!(self.rpc_id)) {
                reply = Some(unwrap_result(&m)?);
            } else {
                return Err(format!("unexpected MCP message: {m}"));
            }
        }
        self.stats.tool_calls += 1;
        Ok((reply.ok_or("no tools/call response")?, notes))
    }

    fn drain(&mut self) -> Result<Vec<Note>, String> {
        let mut out = Vec::new();
        for (conn, server) in &mut self.adapters {
            for m in server.take_notifications() {
                out.push(unwrap_notification(conn, &m)?);
            }
        }
        let rest = self.hub.borrow().pending_conns();
        for conn in rest {
            for q in self.hub.borrow_mut().take(&conn) {
                out.push((q.order, conn.clone(), q.msg));
            }
        }
        Ok(out)
    }
}

fn unwrap_notification(conn: &str, m: &Value) -> Result<Note, String> {
    if m.get("method").and_then(Value::as_str) != Some("notifications/message") {
        return Err(format!("unexpected MCP notification: {m}"));
    }
    let data = m
        .pointer("/params/data")
        .cloned()
        .ok_or_else(|| format!("notification without data: {m}"))?;
    let order = m
        .pointer("/params/_meta")
        .and_then(|x| x.get(META_ORDER))
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("notification without order: {m}"))?;
    Ok((order, conn.to_owned(), data))
}

fn unwrap_result(m: &Value) -> Result<Value, String> {
    if let Some(e) = m.get("error") {
        return Err(format!("JSON-RPC error: {e}"));
    }
    let r = m.get("result").ok_or("response without result")?;
    let sc = r
        .get("structuredContent")
        .cloned()
        .ok_or("tool result without structuredContent")?;
    let text = r
        .pointer("/content/0/text")
        .and_then(Value::as_str)
        .ok_or("tool result without text content")?;
    let parsed: Value = serde_json::from_str(text).map_err(|e| format!("text content: {e}"))?;
    if parsed != sc {
        return Err("text content differs from structuredContent".to_owned());
    }
    let is_error = sc.get("type").and_then(Value::as_str) == Some("error");
    if r.get("isError").and_then(Value::as_bool) != Some(is_error) {
        return Err(format!("isError does not match the reply type: {r}"));
    }
    Ok(sc)
}

/// The tool call a transcript message maps to: `(tool, arguments, request id)`.
/// `None`: not carried over MCP.
#[must_use]
pub fn to_tool(msg: &Value) -> Option<(String, Value, String)> {
    let obj = msg.as_object()?;
    let ty = obj.get("type")?.as_str()?;
    let id = obj.get("id")?.as_str()?.to_owned();
    let rest = |o: &Map<String, Value>| {
        let mut a = o.clone();
        a.remove("type");
        Value::Object(a)
    };
    match ty {
        "command" => {
            let known = obj.keys().all(|k| {
                ["type", "id", "ops"].contains(&k.as_str()) || COMMAND_EXTRAS.contains(&k.as_str())
            });
            if let (true, Some([Value::Object(op)])) = (
                known,
                obj.get("ops").and_then(Value::as_array).map(Vec::as_slice),
            ) {
                let name = op
                    .get("op")
                    .and_then(Value::as_str)
                    .map(|n| format!("op_{n}"));
                let clean = !COMMAND_EXTRAS.iter().any(|k| op.contains_key(*k));
                if let (Some(name), true) = (name, clean) {
                    if tools::find(&name).is_some() {
                        let mut args = op.clone();
                        args.remove("op");
                        for k in COMMAND_EXTRAS {
                            if let Some(v) = obj.get(k) {
                                args.insert(k.to_owned(), v.clone());
                            }
                        }
                        return Some((name, Value::Object(args), id));
                    }
                }
            }
            Some(("command".to_owned(), rest(obj), id))
        }
        "query" => {
            let known = obj
                .keys()
                .all(|k| ["type", "id", "query"].contains(&k.as_str()));
            if let (true, Some(q)) = (known, obj.get("query").and_then(Value::as_object)) {
                if let Some(name) = q
                    .get("kind")
                    .and_then(Value::as_str)
                    .map(|k| format!("query_{k}"))
                {
                    if tools::find(&name).is_some() {
                        let mut args = q.clone();
                        args.remove("kind");
                        return Some((name, Value::Object(args), id));
                    }
                }
            }
            Some(("query".to_owned(), rest(obj), id))
        }
        "subscribe" | "unsubscribe" | "attend" => Some((ty.to_owned(), rest(obj), id)),
        _ => None,
    }
}

impl Transport for McpTransport {
    fn send(&mut self, conn: &str, text: &str, raw: bool) -> Result<Vec<Value>, String> {
        let parsed: Option<Value> = if raw {
            None
        } else {
            Some(serde_json::from_str(text).map_err(|e| e.to_string())?)
        };
        let mut replies: Vec<Value> = Vec::new();
        let mut notes = Vec::new();
        let is_hello = parsed
            .as_ref()
            .and_then(|m| m.get("type"))
            .and_then(Value::as_str)
            == Some("hello");
        let hello = if is_hello {
            parsed.as_ref().and_then(|m| self.hello(conn, m))
        } else {
            None
        };
        if let Some(h) = hello {
            replies.push(h);
        } else {
            let tool = parsed
                .as_ref()
                .filter(|_| self.adapters.contains_key(conn))
                .and_then(to_tool);
            match tool {
                Some((name, args, rid)) => {
                    let (r, n) = self.call(conn, &name, args, &rid)?;
                    replies.push(r);
                    notes = n;
                }
                None => replies = self.direct(conn, text),
            }
        }
        notes.extend(self.drain()?);
        notes.sort_by_key(|(o, _, _)| *o);
        let mut out: Vec<Value> = replies
            .into_iter()
            .map(|m| json!({"conn": conn, "msg": m}))
            .collect();
        out.extend(
            notes
                .into_iter()
                .map(|(_, c, m)| json!({"conn": c, "msg": m})),
        );
        Ok(out)
    }

    fn finish(&mut self) -> Result<FinalState, String> {
        let hub = self.hub.borrow();
        Ok(FinalState {
            document: hub.session().document().clone(),
            log: hub.session().log().to_vec(),
        })
    }
}

/// Loads and runs one transcript file over MCP.
#[must_use]
pub fn run_file(file: &Path, root: &Path) -> (TranscriptOutcome, Stats) {
    let mut outcome = TranscriptOutcome {
        file: file.to_owned(),
        name: file
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default(),
        failure: None,
    };
    let mut stats = Stats::default();
    let r = (|| -> Result<(), String> {
        let t = read_transcript(file)?;
        outcome.name.clone_from(&t.name);
        let loaded = load_init(&t.init, root)?;
        let session = Session::new(loaded.init.clone(), root)?;
        let initial = session.initial_document().clone();
        let mut tr = McpTransport::new(session);
        let r = run_steps(&t, &mut tr, &initial);
        stats = tr.stats;
        r
    })();
    outcome.failure = r.err();
    (outcome, stats)
}
