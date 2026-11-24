//! What a subsystem registers: op handlers, queries, inputs, topics, condition kinds,
//! value sources and claim kinds. Each is keyed by its wire tag (`op` / `kind`).

use qp_ops::expand::Ctx;
use qp_plane::values::{Cache, Computed};
use qp_schema::data::FeedKey;
use qp_schema::protocol::{
    ChangeOp, ClaimRef, DisputeVerdict, Event, Gesture, Op, OutboundLine, Permissions,
    ProtocolError, Query, QueryResult, Seq, Topic,
};
use qp_schema::{Condition, Document};
use serde_json::Value;

use crate::{SessionCtx, Slots};

/// How an agent-origin command may carry an op (§25.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentOp {
    /// Applied directly (comments, watches).
    Direct,
    /// Only as a proposal a person decides (views and drawings).
    Proposal,
    /// Never.
    Deny,
}

/// The wire name of an op (its `op` tag).
pub fn op_name(op: &Op) -> String {
    serde_json::to_value(op)
        .ok()
        .and_then(|v| v.get("op").and_then(Value::as_str).map(str::to_owned))
        .unwrap_or_default()
}

/// An op kind a subsystem owns (`rounds/RF1/PLAN.md` §4.4): its expansion and policy.
pub trait OpHandler: Sync {
    /// The op's wire tag (`add_watch`, …).
    fn op(&self) -> &'static str;
    /// Whether `op` is of its kind (default: its wire tag is [`OpHandler::op`]; a handler
    /// overrides it with a variant match, so that a lookup never serializes the op).
    fn owns(&self, op: &Op) -> bool {
        op_name(op) == self.op()
    }
    /// Expands the op into changes against `doc` (`at`: its JSON pointer).
    ///
    /// # Errors
    /// The op is invalid here (the protocol error to send).
    fn expand(
        &self,
        doc: &Document,
        op: &Op,
        at: &str,
        ctx: &mut Ctx<'_>,
    ) -> Result<Vec<ChangeOp>, ProtocolError>;
    /// Whether a role with `perms` may write it directly (default: write permission).
    fn allowed(&self, perms: &Permissions) -> bool {
        perms.write
    }
    /// Whether it may become a proposal (default: yes).
    fn proposable(&self) -> bool {
        true
    }
    /// How an agent may carry it (default: never).
    fn agent(&self, _doc: &Document, _op: &Op) -> AgentOp {
        AgentOp::Deny
    }
    /// Its version in the public world (`None`: none; default: the same op).
    fn to_public(&self, op: &Op) -> Option<Op> {
        Some(op.clone())
    }
    /// Whether its requests reach only its author's person and owners (reader privacy).
    fn reader_private(&self) -> bool {
        false
    }
}

/// A query kind a subsystem answers.
pub trait QueryHandler: Sync {
    /// The query's wire tag (`kind`; [`crate::ANY`]: every kind no other handler takes).
    fn kind(&self) -> &'static str;
    /// Answers `q` of connection `conn` (inside the reader's projection).
    ///
    /// # Errors
    /// The query is invalid here.
    fn answer(
        &self,
        conn: &str,
        q: &Query,
        ctx: &mut SessionCtx<'_>,
    ) -> Result<QueryResult, ProtocolError>;
}

/// A gesture kind a subsystem handles (`input`).
pub trait InputHandler: Sync {
    /// The gesture's wire tag (`kind`; [`crate::ANY`]: every kind no other handler takes).
    fn kind(&self) -> &'static str;
    /// Handles gesture `g` of connection `conn`: the result and its notifications.
    ///
    /// # Errors
    /// The gesture is invalid here.
    fn input(
        &self,
        conn: &str,
        g: &Gesture,
        ctx: &mut SessionCtx<'_>,
    ) -> Result<(QueryResult, Vec<OutboundLine>), ProtocolError>;
}

/// A topic kind a subsystem serves (`subscribe`).
pub trait TopicHandler: Sync {
    /// The topic's wire tag (`kind`).
    fn kind(&self) -> &'static str;
    /// Accepts or refuses subscription `sub` of `conn` to `t` (state goes to the slots).
    ///
    /// # Errors
    /// The subscription is refused.
    fn subscribe(
        &self,
        conn: &str,
        sub: &str,
        t: &Topic,
        ctx: &mut SessionCtx<'_>,
    ) -> Result<(), ProtocolError>;

    // ---- A pushed topic (RF1 S3.3; the viewer, §22): after every input the authority
    // asks every connection it pushes to what changed. Everything has a default (a topic
    // that only accepts subscriptions pushes nothing).

    /// Whether it pushes to any connection (cheap; its state in `slots`).
    fn pushing(&self, _slots: &Slots) -> bool {
        false
    }
    /// Whether it pushes to connection `conn`.
    fn pushes(&self, _conn: &str, _slots: &Slots) -> bool {
        false
    }
    /// What changed for connection `conn` since it was last told (`all`: its first event,
    /// right after the subscription's ack), as one event on one of its subscriptions:
    /// `(sub, event)`; `None`: nothing to tell. Called in the connection's reader world
    /// with the read view (`SessionCtx::read`); `seq`: the log position as that reader
    /// receives it.
    fn push(
        &self,
        _conn: &str,
        _all: bool,
        _seq: Seq,
        _ctx: &mut SessionCtx<'_>,
    ) -> Option<(String, Event)> {
        None
    }
    /// A `visible` message of connection `conn` for subscription `sub` (§22.2): records
    /// what is on screen. `None`: not its message (the authority asks the next handler).
    ///
    /// # Errors
    /// `sub` is not its subscription of `conn`.
    fn visible(
        &self,
        _conn: &str,
        _sub: &str,
        _visible: &[String],
        _slots: &mut Slots,
    ) -> Option<Result<(), ProtocolError>> {
        None
    }
    /// The answer to the `visible` message it took: what changed for `conn` (in its reader
    /// world, as [`TopicHandler::push`]).
    fn visible_result(&self, _conn: &str, _ctx: &mut SessionCtx<'_>) -> Option<QueryResult> {
        None
    }
    /// The subscription of `conn` that also gets a claim resting on `feeds` (besides
    /// `claims`).
    fn claim_sub<'s>(&self, _conn: &str, _feeds: &[FeedKey], _slots: &'s Slots) -> Option<&'s str> {
        None
    }
    /// The feeds a host serves connection `conn` (its data subscriptions, §22.5; `None`:
    /// the whole data plane).
    fn data<'s>(&self, _conn: &str, _slots: &'s Slots) -> Option<&'s [FeedKey]> {
        None
    }
}

/// A condition kind the live path evaluates (RL1's programs plug in here).
pub trait ConditionKind: Sync {
    /// The condition's wire tag (`kind`).
    fn kind(&self) -> &'static str;
    /// Evaluates `c` at the last bar: `(true now, bar index, its time)`; `None`: cannot.
    fn eval(&self, rd: &Document, cache: &mut Cache, c: &Condition) -> Option<(bool, u64, i64)>;
}

/// A value source of the data plane (D3): the values of the series ids it owns.
pub trait ValueSource: Sync {
    /// Whether it gives the values of series `id`.
    fn owns(&self, id: &str) -> bool;
    /// The values of series `id` over the reading document.
    fn values(&self, rd: &Document, id: &str) -> Option<Computed>;
}

/// A claim kind (D1): its dispute verdict.
pub trait ClaimKind: Sync {
    /// The claim's wire tag (`kind`).
    fn kind(&self) -> &'static str;
    /// The verdict on a dispute of `claim` at bar time `t` by connection `conn`.
    fn verdict(
        &self,
        conn: &str,
        claim: &ClaimRef,
        t: i64,
        ctx: &mut SessionCtx<'_>,
    ) -> DisputeVerdict;
}
