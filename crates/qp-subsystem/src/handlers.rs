//! What a subsystem registers: op handlers, queries, inputs, topics, condition kinds,
//! value sources and claim kinds. Each is keyed by its wire tag (`op` / `kind`).

use qp_ops::expand::Ctx;
use qp_plane::values::{Cache, Computed};
use qp_schema::protocol::{
    ChangeOp, ClaimRef, DisputeVerdict, Gesture, Op, OutboundLine, Permissions, ProtocolError,
    Query, QueryResult, Topic,
};
use qp_schema::{Condition, Document};
use serde_json::Value;

use crate::SessionCtx;

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
    /// The query's wire tag (`kind`).
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
    /// The gesture's wire tag (`kind`).
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
