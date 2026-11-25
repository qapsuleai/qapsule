//! Document items: facts, findings, capsules, glyphs, intents, ledger, `Document`
//! and the legacy `Command`/`Op` draft.

use super::*;

// ---------------------------------------------------------------------------
// Fact, Finding, Capsule, Glyph
// ---------------------------------------------------------------------------

/// An indisputable numeric event detected by the engine. Carries its evidence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Fact {
    /// Unique id within the document.
    pub id: String,
    /// Event kind, e.g. `cross_above`.
    pub kind: String,
    /// Id of the series the event happened on.
    pub series: String,
    /// Bar index within the series.
    pub bar: u64,
    /// Bar time, epoch milliseconds UTC.
    pub t: TimestampMs,
    /// Short human-readable statement.
    pub text: String,
    /// Evidence values, e.g. `{"level": 70, "prev": 68.9, "value": 71.4}`.
    #[serde(default)]
    pub evidence: BTreeMap<String, f64>,
    /// Frozen evidence (RP2): the data this fact rests on, by reference and content
    /// digest; stamped by the authority (`docs/PROTOCOL.md` §18.5). Clients never send it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_ref: Option<crate::data::EvidenceRef>,
    /// Meaning (R3): kind, direction, family, time kind, detector, license, geometry.
    /// Absent on facts written before R3.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meaning: Option<crate::capsule::FactMeaning>,
    /// Who detected it.
    pub provenance: Provenance,
}

/// Comparison operator used in conditions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CompareOp {
    /// Strictly less than.
    Lt,
    /// Less than or equal.
    Le,
    /// Strictly greater than.
    Gt,
    /// Greater than or equal.
    Ge,
    /// Crosses the value upwards.
    CrossAbove,
    /// Crosses the value downwards.
    CrossBelow,
}

/// A condition evaluated over series (invalidation, intent trigger).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Condition {
    /// Last value of a series compared against a constant.
    Threshold {
        /// Series id.
        series: String,
        /// Comparison.
        op: CompareOp,
        /// Constant to compare against.
        value: f64,
    },
    /// One series compared against another, both at the last bar (and the bar before,
    /// for crossings). E.g. SMA(20) crosses above SMA(50):
    /// `{"kind":"compare","left":"sma20","op":"cross_above","right":"sma50"}`.
    /// For a bar series the compared value is the close.
    Compare {
        /// Left series id.
        left: String,
        /// Comparison.
        op: CompareOp,
        /// Right series id.
        right: String,
    },
    /// Free text; evaluated by a person.
    Text {
        /// Condition statement.
        text: String,
    },
}

/// A claim derived from facts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Finding {
    /// Unique id within the document.
    pub id: String,
    /// The claim, in one sentence.
    pub claim: String,
    /// Ids of the facts the claim rests on.
    pub facts: Vec<String>,
    /// Confidence in `[0, 1]`.
    #[schemars(range(min = 0.0, max = 1.0))]
    pub confidence: f64,
    /// When this condition holds, the finding is invalid.
    pub invalidation: Condition,
    /// Author and basis.
    pub provenance: Provenance,
}

/// Lifecycle state of a capsule:
/// `draft -> live -> (updated)* -> invalidated | expired | approved | rejected`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CapsuleState {
    /// Not yet shown as live.
    Draft,
    /// Updates itself as its series change.
    Live,
    /// The invalidation condition happened.
    Invalidated,
    /// Validity window elapsed.
    Expired,
    /// A person approved it.
    Approved,
    /// A person rejected it.
    Rejected,
}

/// Kind of meaning glyph (initial candidate set; finalized in R3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GlyphKind {
    /// Where is it going?
    Direction,
    /// How strong is the move?
    Strength,
    /// How sure are we?
    Uncertainty,
    /// Is it unusual?
    Extremity,
    /// Has it happened before, and what followed?
    Similarity,
    /// What changed since the last look?
    Change,
}

/// A one-glance visual of meaning, not data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Glyph {
    /// Which glyph.
    pub kind: GlyphKind,
    /// Normalized magnitude; `[-1, 1]` for direction, `[0, 1]` otherwise.
    #[schemars(range(min = -1.0, max = 1.0))]
    pub value: f64,
}

/// Semantic zoom level: 0 sentence, 1 capsule, 2 marked chart, 3 full workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Depth {
    /// Depth 0: one sentence.
    Sentence,
    /// Depth 1: the capsule.
    Capsule,
    /// Depth 2: chart with marks.
    Chart,
    /// Depth 3: full workspace.
    Workspace,
}

/// The live, openable, portable presentation of a single finding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Capsule {
    /// Unique id within the document.
    pub id: String,
    /// Id of the finding it presents.
    pub finding: String,
    /// Lifecycle state.
    pub state: CapsuleState,
    /// Number of live updates so far.
    #[serde(default)]
    pub revision: u32,
    /// Depth-0 sentence.
    pub headline: String,
    /// Glyphs shown at a glance.
    #[serde(default)]
    pub glyphs: Vec<Glyph>,
    /// Id of the view used when the capsule is opened.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view: Option<String>,
    /// Depth the capsule is shown at by default.
    pub depth: Depth,
    /// Expiry time, epoch milliseconds UTC.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<TimestampMs>,
    /// The story (R3): theme, stage, families, ledger at birth, history. Absent on
    /// capsules written before R3.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub story: Option<crate::capsule::Story>,
    /// Who made it and when.
    pub provenance: Provenance,
}

// View: see `crate::grammar` (R2).

// ---------------------------------------------------------------------------
// Intent, Ledger
// ---------------------------------------------------------------------------

/// A persistent request left by the user, e.g. "tell me if the uptrend breaks".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Intent {
    /// Unique id within the document.
    pub id: String,
    /// The request as the user said it.
    pub text: String,
    /// Machine-checkable trigger, if one could be derived.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger: Option<Condition>,
    /// Whether the intent is still active.
    pub active: bool,
    /// Who left it and when.
    pub provenance: Provenance,
    /// (R5B, §25.5) A structured watch on story transitions, matched by the engine.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub watch: Option<Watch>,
    /// (R5B, §25.5) Lifecycle state of a watch (absent: an intent before R5B).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<IntentState>,
    /// (R5B, §25.5) How many times the watch matched.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub matches: u32,
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_zero(n: &u32) -> bool {
    *n == 0
}

/// (R5B, §25.5) Lifecycle state of a watch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum IntentState {
    /// Matching.
    Active,
    /// Reached `max_matches`.
    Matched,
    /// The clock reached `expires_t`.
    Expired,
    /// Removed by `remove_watch`.
    Removed,
}

/// (R5B, §25.5) A structured watch: whom it is for, what story transitions match, until
/// when and how often.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Watch {
    /// The person it is for (principal id).
    #[serde(rename = "for")]
    pub for_: String,
    /// What matches.
    #[serde(rename = "match")]
    pub match_: WatchMatch,
    /// The watch expires when the clock reaches this time (epoch ms); never if absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_t: Option<TimestampMs>,
    /// The watch ends after this many matches; never if absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_matches: Option<u32>,
}

/// (R5B, §25.5) What story transitions a watch matches. Every given field must hold.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WatchMatch {
    /// Only this instrument.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instrument: Option<String>,
    /// Only instruments the person holds (non-zero position in their audience).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub holdings: bool,
    /// Only these meaning kinds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meaning: Option<Vec<crate::capsule::MeaningKind>>,
    /// Only this direction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direction: Option<crate::capsule::Direction>,
    /// Only transitions to these stages.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stages: Option<Vec<crate::capsule::Stage>>,
    /// Only negative transitions (`weakened`, `invalidated`, `retracted`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub negative: bool,
}

/// Outcome of a finding compared against what actually happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    /// Not yet decided.
    Pending,
    /// Reality agreed with the claim.
    Confirmed,
    /// Reality contradicted the claim.
    Refuted,
}

/// One record in the ledger.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LedgerEntry {
    /// Finding id.
    pub finding: String,
    /// Final capsule state, if any (e.g. approved / rejected by a person).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<CapsuleState>,
    /// Outcome against reality.
    pub outcome: Outcome,
    /// Record time, epoch milliseconds UTC.
    pub recorded_at: TimestampMs,
    /// Optional note.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Capsule id (R3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capsule: Option<String>,
    /// Ledger key `kind/detector@version` (R3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    /// Closing stage (R3): `realized`, `invalidated` or `expired` (`retracted` is never
    /// recorded, K37).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage: Option<crate::capsule::Stage>,
    /// Bar at which the outcome was decided (R3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bar: Option<u64>,
}

/// The record of findings compared with reality afterwards.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Ledger {
    /// Entries, in record order.
    #[serde(default)]
    pub entries: Vec<LedgerEntry>,
}

// ---------------------------------------------------------------------------
// Document, Command
// ---------------------------------------------------------------------------

/// Everything: series bindings, views, facts, findings, capsules, intents and the ledger.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Document {
    /// Schema version; `"0"` for this schema.
    pub schema_version: String,
    /// Document id.
    pub id: String,
    /// Optional title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Time zone (IANA) in which times are *shown* to the user. Display only: it never
    /// affects bar alignment or computation (K14); views may override it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_tz: Option<String>,
    /// Series bindings.
    #[serde(default)]
    pub series: Vec<Series>,
    /// Views.
    #[serde(default)]
    pub views: Vec<View>,
    /// Facts.
    #[serde(default)]
    pub facts: Vec<Fact>,
    /// Findings.
    #[serde(default)]
    pub findings: Vec<Finding>,
    /// Capsules.
    #[serde(default)]
    pub capsules: Vec<Capsule>,
    /// Intents.
    #[serde(default)]
    pub intents: Vec<Intent>,
    /// Ledger.
    #[serde(default)]
    pub ledger: Ledger,
    /// Narration timelines (protocol v0).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub narratives: Vec<crate::protocol::Narrative>,
    /// Proposals: writes waiting for approval, and their outcome (protocol v0).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub proposals: Vec<crate::protocol::Proposal>,
    /// Capsule preferences per person (principal id → audience), R3.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub audiences: BTreeMap<String, crate::capsule::Audience>,
    /// (R5B) Validated comments (agents' and people's narratives about capsules and
    /// instruments, with their citations).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub comments: Vec<crate::protocol::Comment>,
}

/// The operation a command performs on a document.
// Legacy draft type (docs/LEGACY.md); `Fact` grew with `evidence_ref` (RP2). Not on a hot path.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Op {
    /// Add or replace a series binding (by id).
    PutSeries {
        /// The series.
        series: Series,
    },
    /// Add or replace a view (by id).
    PutView {
        /// The view.
        view: View,
    },
    /// Add a fact.
    AddFact {
        /// The fact.
        fact: Fact,
    },
    /// Add a finding.
    AddFinding {
        /// The finding.
        finding: Finding,
    },
    /// Add a capsule.
    AddCapsule {
        /// The capsule.
        capsule: Capsule,
    },
    /// Move a capsule to a new lifecycle state.
    SetCapsuleState {
        /// Capsule id.
        capsule: String,
        /// New state.
        state: CapsuleState,
    },
    /// Add an intent.
    AddIntent {
        /// The intent.
        intent: Intent,
    },
    /// Append a ledger entry.
    RecordLedger {
        /// The entry.
        entry: LedgerEntry,
    },
    /// Remove an item by id.
    Remove {
        /// Item id.
        id: String,
    },
}

/// The only way to change a document. Humans and agents use the same commands.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Command {
    /// Who issued the command and when.
    pub provenance: Provenance,
    /// What to do.
    pub op: Op,
}
