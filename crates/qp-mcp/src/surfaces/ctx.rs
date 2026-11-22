//! [`Ctx`]: a surface's only door to the authority during one call (`qp/0` messages on
//! the endpoint's connection and its pending notifications).

use serde_json::{json, Value};

use crate::{Inbox, Queued, SharedHub};

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
