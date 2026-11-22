//! Agent comments and proposals (R5B, §25): what an agent writes for approval.

use super::*;

/// (R5B, §25.4) What a comment is about: exactly one of a capsule (or its finding) and
/// an instrument symbol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CommentAbout {
    /// Capsule id (or its finding's id).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capsule: Option<String>,
    /// Instrument symbol.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instrument: Option<String>,
}

/// (R5B, §25.4) A validated comment, as stored in the document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Comment {
    /// Unique id within the document.
    pub id: String,
    /// What it is about (a capsule is stored by its capsule id).
    pub about: CommentAbout,
    /// Language of the text.
    pub lang: Language,
    /// The text.
    pub text: String,
    /// Ids it rests on.
    pub cites: Vec<String>,
    /// External quotes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub external: Vec<crate::comment::ExternalQuote>,
    /// The strictness it was validated with.
    pub strictness: crate::comment::Strictness,
    /// Who wrote it (an agent: `author.kind = "agent"`) and when.
    pub provenance: crate::types::Provenance,
    /// (R5B) The person the writing agent works for, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_behalf: Option<String>,
}

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

/// (R5B, §25.6) One view a previewed proposal adds or changes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PreviewView {
    /// View id.
    pub view: String,
    /// Whether the view is new.
    pub new: bool,
    /// The scene summary (`explain` text: panels, layers, marks, ranges).
    pub explain: String,
}
