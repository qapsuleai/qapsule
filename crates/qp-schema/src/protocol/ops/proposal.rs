//! Proposals: writes waiting for approval and their state.

use super::*;

/// State of a proposal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProposalState {
    /// Waiting for a decision.
    Pending,
    /// Accepted and applied.
    Accepted,
    /// Rejected.
    Rejected,
    /// Not decided before `expires_at`.
    Expired,
}

/// A write waiting for approval.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Proposal {
    /// Unique id within the document.
    pub id: String,
    /// The operations, exactly as proposed.
    pub ops: Vec<Op>,
    /// Why, as given by the proposer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Current state.
    pub state: ProposalState,
    /// Proposer and time.
    pub provenance: crate::types::Provenance,
    /// Expiry time; absent means no expiry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<TimestampMs>,
    /// Who decided, once decided.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decided_by: Option<Author>,
    /// Decision note.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Log position at which it was decided or expired.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decided_seq: Option<Seq>,
}
