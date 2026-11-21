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
//! - **What an agent may not write** (only on a marked connection, or in a session with
//!   `init.agent_policy = "r5b"`): facts, findings, bar data, undo, another person's
//!   audience and every op that changes capsules, the ledger or stories → `forbidden`,
//!   `reason: "agent_cannot_write"`, in every mode (not even as a proposal).

use std::collections::BTreeSet;

use qp_schema::protocol::{ErrorCode, Op, ProtocolError};
use qp_schema::{Author, SeriesData};

use crate::error::{ptr, PResult, E};
use crate::session::Session;

/// `forbidden` reason: a decision only a human may make (§25.2).
pub const HUMAN_ONLY: &str = "human_only";
/// `forbidden` reason: an item only the engine (or a person) may write (§25.3).
pub const AGENT_CANNOT_WRITE: &str = "agent_cannot_write";
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
    /// changes the answer).
    pub(crate) fn agent_check(&self, ci: usize, ops: &[Op]) -> PResult<()> {
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
            return Ok(());
        }
        let speaker = &self.conns[ci].principal;
        for (i, op) in ops.iter().enumerate() {
            if let Some(why) = self.agent_cannot_write(speaker, op) {
                return Err(cannot_write(i, op, &why));
            }
        }
        Ok(())
    }

    /// Why an agent may not send `op` (§25.3), if it may not.
    fn agent_cannot_write(&self, speaker: &str, op: &Op) -> Option<String> {
        let what = match op {
            Op::AddFact { .. } => "facts come from the engine only",
            Op::AddFinding { .. } | Op::UpdateFinding { .. } => {
                "findings come from the engine or a person only"
            }
            Op::AppendBars { .. } | Op::UpdateLastBar { .. } => {
                "bar data comes from the host's feeds only"
            }
            Op::PutSeries { series } if matches!(series.data, Some(SeriesData::Bars { .. })) => {
                "bar data comes from the host's feeds only"
            }
            Op::Undo { .. } => "an agent does not undo log entries",
            Op::ShowFinding { .. } => "it changes a capsule",
            Op::SetAudience {
                principal: Some(p), ..
            }
            | Op::SetSensitivity {
                principal: Some(p), ..
            } if p != speaker => "another person's audience is theirs",
            Op::Remove { id } => {
                let d = &self.doc;
                if d.facts.iter().any(|f| &f.id == id)
                    || d.findings.iter().any(|f| &f.id == id)
                    || d.capsules.iter().any(|c| &c.id == id)
                {
                    "facts, findings and capsules are the engine's"
                } else {
                    return None;
                }
            }
            _ => return None,
        };
        Some(what.to_owned())
    }
}

fn op_name(op: &Op) -> String {
    serde_json::to_value(op)
        .ok()
        .and_then(|v| v.get("op").and_then(|o| o.as_str()).map(str::to_owned))
        .unwrap_or_default()
}

fn cannot_write(i: usize, op: &Op, why: &str) -> ProtocolError {
    E::new(
        ErrorCode::Forbidden,
        format!("an agent may not `{}`: {why}", op_name(op)),
    )
    .at(ptr("/ops", i))
    .reason(AGENT_CANNOT_WRITE)
    .hint("Agents write comments (`add_comment`), watches (`add_watch`) and view proposals; facts, findings, capsules, the ledger and stories come from the engine.")
    .build()
}
