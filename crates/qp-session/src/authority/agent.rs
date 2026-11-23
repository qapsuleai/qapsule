//! (R5B, `docs/PROTOCOL.md` §25) The agent surface of the authority: agent origin,
//! the human decision rule, what an agent may not write.
//!
//! - **Agent origin:** a command whose principal's author is `agent`, or that arrives on
//!   a connection the host marked ([`Session::mark_agent_connection`]; `qp mcp serve`
//!   marks every MCP connection; a client cannot unmark it).
//! - **Human decision** (unconditional): `accept_proposal`, `reject_proposal`,
//!   `approve_finding` and `reject_finding` only from a principal whose author is
//!   `human` **and** on an unmarked connection; otherwise `forbidden`,
//!   `reason: "human_only"`, whatever the role and the mode.
//! - **What an agent may write** (only on a marked connection, or in a session with
//!   `init.agent_policy = "r5b"`): an **allow-list** ([`agent_op`]). Comments and watches
//!   directly; view and drawing ops only as proposals (`mode: auto` becomes a proposal;
//!   `mode: direct` → `forbidden`, `reason: "proposal_only"`). Every other op — facts,
//!   findings, series, bar data, undo, audiences, intents, and any op added later — →
//!   `forbidden`, `reason: "agent_cannot_write"`, in every mode (not even as a proposal).

use std::collections::BTreeSet;

use qp_schema::protocol::WriteMode;
use qp_schema::protocol::{ErrorCode, Op, ProtocolError};
use qp_schema::{Author, Document};

use crate::session::Session;
use qp_plane::error::{ptr, PResult, E};

/// `forbidden` reason: a decision only a human may make (§25.2).
pub const HUMAN_ONLY: &str = "human_only";
/// `forbidden` reason: an op outside the agent allow-list (§25.3).
pub const AGENT_CANNOT_WRITE: &str = "agent_cannot_write";
/// `forbidden` reason: an op an agent may send only as a proposal, sent `direct` (§25.3).
pub const PROPOSAL_ONLY: &str = "proposal_only";

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
/// The agent policy name of `init.agent_policy` (§25.3).
pub const AGENT_POLICY_R5B: &str = "r5b";

/// The authority's agent state (host configuration and marks; never in the document).
#[derive(Debug, Default)]
pub(crate) struct AgentState {
    /// Connections the host marked as agent connections (by connection id).
    pub marked: BTreeSet<String>,
    /// `init.agent_policy = "r5b"`: the agent write rule holds for agent principals on
    /// every connection.
    pub policy: bool,
}

/// True for the four decision ops (§25.2).
pub(crate) fn is_decision(op: &Op) -> bool {
    matches!(
        op,
        Op::AcceptProposal { .. }
            | Op::RejectProposal { .. }
            | Op::ApproveFinding { .. }
            | Op::RejectFinding { .. }
    )
}

impl Session {
    /// (R5B, §25.1) Marks connection `conn` as an agent connection: every command on it
    /// is of agent origin (the human decision rule refuses its decisions; the agent write
    /// rule holds). The mark is the host's: a client cannot remove it. It may be set
    /// before or after the connection's `hello`.
    pub fn mark_agent_connection(&mut self, conn: &str) {
        self.agent.marked.insert(conn.to_owned());
    }

    /// (R5B) Whether connection `conn` is marked as an agent connection.
    #[must_use]
    pub fn is_agent_connection(&self, conn: &str) -> bool {
        self.agent.marked.contains(conn)
    }

    /// (R5B) Whether the session runs the R5B agent policy (`init.agent_policy`).
    #[must_use]
    pub fn agent_policy(&self) -> bool {
        self.agent.policy
    }

    /// The author of principal `id`, if known.
    fn author_of(&self, id: &str) -> Option<&Author> {
        self.principal(id).map(|p| &p.author)
    }

    /// Whether a command of connection `ci` is of agent origin (§25.1).
    pub(crate) fn agent_origin(&self, ci: usize) -> bool {
        let c = &self.conns[ci];
        self.agent.marked.contains(&c.id)
            || matches!(self.author_of(&c.principal), Some(Author::Agent { .. }))
    }

    /// Whether a command of connection `ci` is of human origin: its principal's author
    /// is `human` and the connection is not marked (§25.2).
    pub(crate) fn human_origin(&self, ci: usize) -> bool {
        let c = &self.conns[ci];
        !self.agent.marked.contains(&c.id)
            && matches!(self.author_of(&c.principal), Some(Author::Human { .. }))
    }

    /// Whether the agent write rule holds for a command of connection `ci` (§25.3).
    pub(crate) fn agent_rule_holds(&self, ci: usize) -> bool {
        let c = &self.conns[ci];
        self.agent.marked.contains(&c.id) || (self.agent.policy && self.agent_origin(ci))
    }

    /// (R5B, §25.2–§25.3) Refuses `ops` of connection `ci` that its origin may not
    /// send, before any permission or mode check (so neither the role nor the mode
    /// changes the answer); the write mode to use (an agent's view ops become proposals).
    pub(crate) fn agent_check(&self, ci: usize, ops: &[Op], mode: WriteMode) -> PResult<WriteMode> {
        if let Some(i) = ops.iter().position(is_decision) {
            if !self.human_origin(ci) {
                return Err(E::new(
                    ErrorCode::Forbidden,
                    format!(
                        "`{}` is a human decision; this command is not of human origin",
                        op_name(&ops[i])
                    ),
                )
                .at(ptr("/ops", i))
                .reason(HUMAN_ONLY)
                .hint("Only a person decides proposals and findings, on a connection of their own; leave the proposal pending for them.")
                .build());
            }
        }
        if !self.agent_rule_holds(ci) {
            return Ok(mode);
        }
        let kinds: Vec<AgentOp> = ops.iter().map(|o| agent_op(&self.doc, o)).collect();
        if let Some(i) = kinds.iter().position(|k| *k == AgentOp::Deny) {
            return Err(cannot_write(i, &ops[i]));
        }
        let direct = kinds.iter().position(|k| *k == AgentOp::Direct);
        let proposal = kinds.iter().position(|k| *k == AgentOp::Proposal);
        match (direct, proposal) {
            (Some(d), Some(_)) => Err(qp_plane::error::invalid(
                "an agent command mixes ops applied directly (comments, watches) with ops that are only proposed (views, drawings)",
                ptr("/ops", d),
            )),
            (None, Some(i)) => {
                if mode == WriteMode::Direct {
                    return Err(E::new(
                        ErrorCode::Forbidden,
                        format!("an agent may `{}` only as a proposal", op_name(&ops[i])),
                    )
                    .at(ptr("/ops", i))
                    .reason(PROPOSAL_ONLY)
                    .hint("Send it with mode auto or propose; a person accepts or rejects it.")
                    .build());
                }
                Ok(WriteMode::Propose)
            }
            _ => Ok(mode),
        }
    }
}

/// (R5B, §25.3) The agent allow-list: how an agent-origin command may carry `op` in
/// document `doc`. Anything not named here — including every op added later — is
/// [`AgentOp::Deny`].
#[must_use]
pub fn agent_op(doc: &Document, op: &Op) -> AgentOp {
    match op {
        Op::PutView { .. }
        | Op::InsertLayer { .. }
        | Op::DeleteLayer { .. }
        | Op::MoveLayer { .. }
        | Op::SplitLayer { .. }
        | Op::SetEncoding { .. }
        | Op::InsertPanel { .. }
        | Op::DeletePanel { .. }
        | Op::MovePanel { .. }
        | Op::SetPanel { .. }
        | Op::SetScale { .. }
        | Op::SetEmphasis { .. }
        | Op::SetViewRange { .. }
        | Op::AddNarrative { .. }
        | Op::ShowFinding { .. } => AgentOp::Proposal,
        Op::AddComment { .. } | Op::AddWatch { .. } | Op::RemoveWatch { .. } => AgentOp::Direct,
        // A new study series only: an existing series is never replaced.
        Op::AddStudy { id, .. } => {
            if id
                .as_ref()
                .is_some_and(|id| doc.series.iter().any(|s| &s.id == id))
            {
                AgentOp::Deny
            } else {
                AgentOp::Proposal
            }
        }
        _ => AgentOp::Deny,
    }
}

fn op_name(op: &Op) -> String {
    serde_json::to_value(op)
        .ok()
        .and_then(|v| v.get("op").and_then(|o| o.as_str()).map(str::to_owned))
        .unwrap_or_default()
}

fn cannot_write(i: usize, op: &Op) -> ProtocolError {
    E::new(
        ErrorCode::Forbidden,
        format!(
            "an agent may not `{}`: facts, findings, series, capsules, the ledger and stories come from the engine",
            op_name(op)
        ),
    )
    .at(ptr("/ops", i))
    .reason(AGENT_CANNOT_WRITE)
    .hint("Agents write comments (`add_comment`), watches (`add_watch`) and view proposals; facts, findings, capsules, the ledger and stories come from the engine.")
    .build()
}
