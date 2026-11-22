//! Changes, the replayable side: `ChangeOp` and the log entry.

use super::*;

// ---------------------------------------------------------------------------
// Changes (replayable side)
// ---------------------------------------------------------------------------

/// A primitive, replayable change to the document. Changes carry complete items
/// (with provenance stamped by the authority), so replaying them needs no presenter,
/// no study code and no clock: the same changes always give the same document bytes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum ChangeOp {
    /// Insert or replace a series.
    PutSeries {
        /// The series.
        series: Series,
        /// Position in its collection when inserting a new item (default: the end).
        /// Used by inverses so that undoing a delete restores the item in place;
        /// ignored when replacing an existing item.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        index: Option<u32>,
    },
    /// Insert or replace a view.
    PutView {
        /// The view.
        view: View,
        /// Position in its collection when inserting a new item (default: the end).
        /// Used by inverses so that undoing a delete restores the item in place;
        /// ignored when replacing an existing item.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        index: Option<u32>,
    },
    /// Insert a layer into a view at `index`.
    InsertLayer {
        /// View id.
        view: String,
        /// Position.
        index: u32,
        /// The layer.
        layer: Layer,
    },
    /// Delete the layer at `index`.
    DeleteLayer {
        /// View id.
        view: String,
        /// Position.
        index: u32,
        /// Id of the layer at `index` (R2; informational, checked on apply when present).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        layer: Option<String>,
    },
    /// Put a layer at `index`: a layer with the same id is removed first (so this both
    /// replaces and moves); otherwise it is inserted (R2).
    PutLayer {
        /// View id.
        view: String,
        /// Final position in the layer list.
        index: u32,
        /// The complete layer.
        layer: Layer,
    },
    /// Put a panel at `index` in the panel list: a panel with the same id is removed
    /// first; otherwise it is inserted (R2).
    PutPanel {
        /// View id.
        view: String,
        /// Final position in the panel list.
        index: u32,
        /// The complete panel.
        panel: crate::grammar::Panel,
    },
    /// Delete a panel by id (its layers are deleted by earlier `delete_layer` changes)
    /// (R2).
    DeletePanel {
        /// View id.
        view: String,
        /// Panel id.
        panel: u32,
    },
    /// Replace a view's emphasis list (R2).
    SetEmphasis {
        /// View id.
        view: String,
        /// The emphasis list.
        emphasis: Vec<crate::grammar::Emphasis>,
    },
    /// Set (or clear) a view's horizontal domain, epoch ms `[from, to]`.
    SetXDomain {
        /// View id.
        view: String,
        /// New domain; `null` = automatic.
        domain: Option<[f64; 2]>,
    },
    /// Insert or replace a fact.
    PutFact {
        /// The fact.
        fact: Fact,
        /// Position in its collection when inserting a new item (default: the end).
        /// Used by inverses so that undoing a delete restores the item in place;
        /// ignored when replacing an existing item.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        index: Option<u32>,
    },
    /// Insert or replace a finding.
    PutFinding {
        /// The finding.
        finding: Finding,
        /// Position in its collection when inserting a new item (default: the end).
        /// Used by inverses so that undoing a delete restores the item in place;
        /// ignored when replacing an existing item.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        index: Option<u32>,
    },
    /// Insert or replace a capsule.
    PutCapsule {
        /// The capsule.
        capsule: crate::types::Capsule,
        /// Position in its collection when inserting a new item (default: the end).
        /// Used by inverses so that undoing a delete restores the item in place;
        /// ignored when replacing an existing item.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        index: Option<u32>,
    },
    /// Set a capsule's state.
    SetCapsuleState {
        /// Capsule id.
        capsule: String,
        /// New state.
        state: CapsuleState,
    },
    /// Insert or replace an intent.
    PutIntent {
        /// The intent.
        intent: Intent,
        /// Position in its collection when inserting a new item (default: the end).
        /// Used by inverses so that undoing a delete restores the item in place;
        /// ignored when replacing an existing item.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        index: Option<u32>,
    },
    /// Append a ledger entry.
    PushLedger {
        /// The entry.
        entry: LedgerEntry,
    },
    /// Remove the last ledger entry (inverse of `push_ledger`).
    PopLedger {},
    /// Insert or replace a narrative.
    PutNarrative {
        /// The narrative.
        narrative: Narrative,
        /// Position in its collection when inserting a new item (default: the end).
        /// Used by inverses so that undoing a delete restores the item in place;
        /// ignored when replacing an existing item.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        index: Option<u32>,
    },
    /// Insert or replace a proposal.
    PutProposal {
        /// The proposal.
        proposal: Proposal,
        /// Position in its collection when inserting a new item (default: the end).
        /// Used by inverses so that undoing a delete restores the item in place;
        /// ignored when replacing an existing item.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        index: Option<u32>,
    },
    /// Delete an item (any collection) by id.
    Delete {
        /// Item id.
        id: String,
    },
    /// Append bars to a bar series.
    AppendBars {
        /// Series id.
        series: String,
        /// Bars.
        bars: Vec<Bar>,
    },
    /// Remove the last `count` bars (inverse of `append_bars`).
    TruncateBars {
        /// Series id.
        series: String,
        /// Number of bars removed from the end.
        count: u32,
    },
    /// Replace the last bar.
    SetLastBar {
        /// Series id.
        series: String,
        /// The bar.
        bar: Bar,
    },
    /// (R3) Set (or remove, `null`) a person's capsule preferences.
    PutAudience {
        /// Principal id.
        principal: String,
        /// The preferences; `null` removes them.
        audience: Option<Audience>,
    },
    /// (R5B, §25.4) Insert or replace a comment.
    PutComment {
        /// The comment.
        comment: Comment,
        /// Position in its collection when inserting a new item (default: the end).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        index: Option<u32>,
    },
}

/// One entry of the operation log.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LogEntry {
    /// Log position (1-based, gapless).
    pub seq: Seq,
    /// Time, from the last `clock`.
    pub at: TimestampMs,
    /// Principal id that wrote it (for an accepted proposal: the proposer).
    pub author: String,
    /// Principal id that approved it (accepted proposals only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approved_by: Option<String>,
    /// Request id of the command that caused it, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cause: Option<String>,
    /// The command's `reason`, if given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// The operations as requested.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub request: Vec<Op>,
    /// The replayable changes.
    pub changes: Vec<ChangeOp>,
    /// The changes that undo this entry.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inverse: Vec<ChangeOp>,
    /// If this entry is an undo: the entry it undoes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub undoes: Option<Seq>,
    /// Short deterministic summary, e.g. `"add_study rsi(period=7) -> rsi-3-0"`.
    pub summary: String,
    /// (RP2/T8) For an engine entry caused by data (a live capsule's invalidation after
    /// a data event or a bar write): the evidence of the claim (§19). Not replayed; the
    /// document does not depend on it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<ClaimEvidence>,
}
