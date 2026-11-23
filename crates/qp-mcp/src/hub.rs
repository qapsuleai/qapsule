//! The [`Hub`]: one authority (a session or a replay host) shared by several adapters,
//! with one queue of pending `notify` messages per protocol connection.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use qp_session::{ReplayHost, Session};
use serde_json::Value;

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
pub(crate) enum Authority {
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
    ) -> Result<(), qp_plane::DataError> {
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
