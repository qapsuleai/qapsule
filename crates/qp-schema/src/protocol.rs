//! Qapsule domain protocol v0 (`qp/0`): the messages exchanged between the engine
//! (the document *authority*), user interfaces and agents. See `docs/PROTOCOL.md`.
//!
//! Single source of truth: these Rust types. The JSON Schema of every message is
//! generated from them ([`crate::json_schemas`]); nothing here is written by hand twice.
//!
//! Shape of the conversation:
//! - a client opens a connection and sends [`ClientMsg::Hello`] (who it is, what surface);
//! - it then sends commands, queries, subscriptions, input and attention directives;
//! - the authority answers each request (`ack`, `result`, `error`) and pushes
//!   notifications (`notify`) for the client's subscriptions.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::types::{
    Author, Bar, CapsuleState, Condition, Depth, Document, Fact, Finding, Glyph, Intent, Layer,
    LedgerEntry, Series, TimestampMs, View,
};

/// The protocol version string of this module.
pub const PROTOCOL_VERSION: &str = "qp/0";

/// Position in a document's operation log. The first entry is 1; 0 means "before any
/// entry" (the initial document).
pub type Seq = u64;

// ---------------------------------------------------------------------------
// Envelopes
// ---------------------------------------------------------------------------

/// A message from a client (UI, agent, data feed or host) to the authority.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ClientMsg {
    /// First message on every connection: who is speaking and from what surface.
    Hello {
        /// Protocol version the client speaks, `"qp/0"`.
        protocol: String,
        /// Principal id (a person, an agent, a feed), e.g. `"arkin"`, `"scout"`.
        principal: String,
        /// The surface this connection renders on (capability declaration).
        surface: Surface,
        /// Optional protocol features requested, e.g. `["attention", "narrative"]`.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        features: Vec<String>,
    },
    /// A request to change the document: one or more operations applied atomically.
    Command {
        /// Client-chosen request id, echoed in the reply.
        id: String,
        /// The operations, applied in order, all or nothing.
        ops: Vec<Op>,
        /// `auto` (default): write directly if permitted, otherwise create a proposal.
        /// `direct`: fail with `forbidden` instead of proposing. `propose`: always propose.
        #[serde(default, skip_serializing_if = "WriteMode::is_auto")]
        mode: WriteMode,
        /// Optimistic concurrency: the log position the client based this command on.
        /// If an item the command touches changed after `base`, the command fails with
        /// `conflict`. Omit to apply on top of whatever is current.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        base: Option<Seq>,
        /// Why (free text). Recorded in the log and shown on proposals.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
    /// A read. Never changes the document.
    Query {
        /// Client-chosen request id, echoed in the reply.
        id: String,
        /// What to read.
        query: Query,
    },
    /// Ask to be notified about something; answered with `ack`, then `notify` messages.
    Subscribe {
        /// Client-chosen request id; also the subscription id used in `notify.sub`.
        id: String,
        /// What to watch.
        topic: Topic,
    },
    /// Cancel a subscription.
    Unsubscribe {
        /// Client-chosen request id.
        id: String,
        /// The subscription id (the `id` of the `subscribe` message).
        sub: String,
    },
    /// Raw user input on a surface (pan, zoom, tap, what is visible). The engine
    /// interprets it; the reply is a `result` with the new viewport or the hit.
    Input {
        /// Client-chosen request id.
        id: String,
        /// The gesture.
        gesture: Gesture,
    },
    /// An attention directive: where the speaker wants a person to look, in order.
    /// Does not change the document; delivered to the target's `attention` subscribers.
    Attend {
        /// Client-chosen request id.
        id: String,
        /// The directive.
        directive: AttentionDirective,
    },
    /// Host only: the current time. The authority has no clock of its own; time is an
    /// input. Proposal expiry and log timestamps follow the last `clock`.
    Clock {
        /// Current time, epoch milliseconds UTC. Must not go backwards.
        now: TimestampMs,
    },
}

/// A message from the authority to a client.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ServerMsg {
    /// Reply to `hello`.
    Welcome {
        /// Protocol version in use.
        protocol: String,
        /// Document id.
        doc: String,
        /// The principal, as authenticated.
        principal: String,
        /// The principal's role.
        role: Role,
        /// What the principal may do.
        permissions: Permissions,
        /// Current log position.
        seq: Seq,
        /// Authority epoch: increases every time the document is handed over to a new
        /// authority (server -> device or back). Log positions are only comparable
        /// within one epoch lineage; see `docs/PROTOCOL.md` §8.4.
        epoch: u64,
        /// Negotiated presentation profile for the declared surface.
        profile: Profile,
        /// Features granted (subset of those requested).
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        features: Vec<String>,
        /// Limits the authority enforces.
        limits: Limits,
    },
    /// Positive reply to `command`, `subscribe`, `unsubscribe`, `attend`.
    Ack {
        /// The request id.
        id: String,
        /// `applied`, `proposed`, `subscribed`, `unsubscribed` or `delivered`.
        status: AckStatus,
        /// Log position of the applied (or proposal-creating) entry.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        seq: Option<Seq>,
        /// Ids the authority assigned or created (in op order).
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        created: Vec<String>,
        /// The proposal id, when `status` is `proposed`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        proposal: Option<String>,
    },
    /// Reply to `query` and `input`.
    Result {
        /// The request id.
        id: String,
        /// Log position the answer reflects.
        seq: Seq,
        /// The answer.
        data: QueryResult,
    },
    /// A pushed event for a subscription.
    Notify {
        /// Subscription id.
        sub: String,
        /// What happened.
        event: Event,
    },
    /// Negative reply to any request (or a connection-level problem when `id` is absent).
    Error {
        /// The request id, if the request could be parsed far enough to read it.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        /// What went wrong and how to fix it.
        error: ProtocolError,
    },
}

/// How a command writes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WriteMode {
    /// Direct if permitted, otherwise a proposal (if the principal may propose).
    #[default]
    Auto,
    /// Direct or fail.
    Direct,
    /// Always a proposal.
    Propose,
}

impl WriteMode {
    /// Whether this is the default mode (used to omit it when serializing).
    #[must_use]
    pub fn is_auto(&self) -> bool {
        *self == Self::Auto
    }
}

/// Status carried by an `ack`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AckStatus {
    /// The command changed the document.
    Applied,
    /// The command became a pending proposal.
    Proposed,
    /// The subscription is active.
    Subscribed,
    /// The subscription is cancelled.
    Unsubscribed,
    /// The attention directive was delivered.
    Delivered,
}

// ---------------------------------------------------------------------------
// Capability
// ---------------------------------------------------------------------------

/// Kind of surface a connection presents on.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceKind {
    /// Large screen, pointer and keyboard.
    Desktop,
    /// Phone: small touch screen.
    Phone,
    /// Watch: glance-sized screen.
    Watch,
    /// Voice only: no screen; answers are spoken.
    Voice,
    /// No presentation at all (an agent, a batch job).
    Headless,
}

/// Input methods a surface offers.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum InputMethod {
    /// Mouse or trackpad.
    Pointer,
    /// Touch screen.
    Touch,
    /// Keyboard.
    Keyboard,
    /// Speech.
    Voice,
    /// Watch crown or similar rotary input.
    Crown,
}

/// The surface declaration sent in `hello`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Surface {
    /// Surface kind.
    pub kind: SurfaceKind,
    /// Screen size in logical pixels `[width, height]`, if there is a screen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub screen: Option<[u32; 2]>,
    /// Input methods.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub input: Vec<InputMethod>,
    /// Upper bound on presentation depth the surface wants, below what its kind allows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_depth: Option<Depth>,
}

/// The presentation profile the authority chose for a surface.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    /// Deepest presentation sent to this surface.
    pub depth: Depth,
    /// Whether presentations may carry series data.
    pub series_data: bool,
    /// Maximum data points per series in a presentation (0 when `series_data` is false).
    pub max_points: u32,
    /// Maximum glyphs per presentation.
    pub max_glyphs: u32,
    /// Whether text should be phrased for speech.
    pub speech: bool,
}

/// Limits enforced by the authority on incoming messages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    /// Maximum size of one incoming message, bytes.
    pub max_message_bytes: u64,
    /// Maximum operations in one command.
    pub max_ops: u32,
    /// Maximum bars in one `append_bars`.
    pub max_bars: u32,
    /// Maximum active subscriptions per connection.
    pub max_subscriptions: u32,
    /// Maximum length of any string, bytes.
    pub max_string: u32,
}

// ---------------------------------------------------------------------------
// Identity and authority
// ---------------------------------------------------------------------------

/// Role of a principal; decides what it may do (see `docs/PROTOCOL.md` §9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// Everything, including deciding proposals.
    Owner,
    /// Direct writes; may not decide proposals.
    Editor,
    /// Writes become proposals (the default for agents).
    Proposer,
    /// Read and subscribe only.
    Viewer,
    /// Market data only (`append_bars`, `update_last_bar`).
    Feed,
    /// The process hosting the authority: `clock` and data.
    Host,
}

/// What a principal may do.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[allow(clippy::struct_excessive_bools)]
pub struct Permissions {
    /// May read and subscribe.
    pub read: bool,
    /// May write directly.
    pub write: bool,
    /// May create proposals.
    pub propose: bool,
    /// May accept or reject proposals.
    pub decide: bool,
    /// May send attention directives.
    pub attend: bool,
    /// May append market data.
    pub feed: bool,
}

/// A principal known to the authority (host configuration, not sent by clients).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Principal {
    /// Principal id.
    pub id: String,
    /// How it appears in provenance.
    pub author: Author,
    /// Its role.
    pub role: Role,
}

// ---------------------------------------------------------------------------
// Operations (command side)
// ---------------------------------------------------------------------------

/// Where a new study is shown.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Show {
    /// View id.
    pub view: String,
    /// Put it in the panel where this series is drawn (e.g. `"rsi14"`). Takes precedence
    /// over `panel`. If neither is given, a new panel is added below the others.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub with: Option<String>,
    /// Panel index (0 = price panel).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub panel: Option<u32>,
}

/// A step of a narration or an attention directive points at something.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Target {
    /// View id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view: Option<String>,
    /// Panel index.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub panel: Option<u32>,
    /// Series id (e.g. the series whose value matters).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub series: Option<String>,
    /// Document item id (finding, fact, capsule...).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item: Option<String>,
    /// First bar index (0-based) of the region.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_bar: Option<u64>,
    /// Last bar index (inclusive) of the region; equal to `from_bar` for one bar.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_bar: Option<u64>,
}

/// One step of a narration timeline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NarrativeStep {
    /// Start of the step, milliseconds from the start of the narration.
    pub at_ms: u32,
    /// What to show.
    pub target: Target,
    /// What to say (one or two sentences).
    pub say: String,
}

/// A narration timeline about a finding: ordered steps, each pointing at something.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Narrative {
    /// Unique id within the document.
    pub id: String,
    /// The item it narrates (usually a finding id).
    pub about: String,
    /// Steps, ascending by `at_ms`.
    pub steps: Vec<NarrativeStep>,
    /// Who made it and when.
    pub provenance: crate::types::Provenance,
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

/// An operation in a command: what a human or an agent asks for, at the level of
/// meaning. The authority expands it into [`ChangeOp`]s (the replayable form).
/// Ids are optional where marked; the authority then assigns one (returned in
/// `ack.created`). All ids in a document are unique across all collections.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Op {
    /// Add or replace a series binding (by id).
    PutSeries {
        /// The series.
        series: Series,
    },
    /// Add a study (computed series) on input series, optionally showing it in a view.
    /// E.g. `{"op":"add_study","study":"rsi","inputs":["px"],"params":{"period":7},
    /// "show":{"view":"main","with":"rsi14"}}`.
    AddStudy {
        /// Id of the new series; assigned if absent.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        /// Study name: `sma`, `ema`, `rsi`, `macd`, `bollinger`.
        study: String,
        /// Input series ids (one bar series for the built-in studies).
        inputs: Vec<String>,
        /// Parameters; missing ones take defaults (sma/ema/rsi: `period`; macd: `fast`,
        /// `slow`, `signal`; bollinger: `period`, `k`).
        #[serde(default)]
        params: BTreeMap<String, f64>,
        /// Show it in a view.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        show: Option<Show>,
    },
    /// Change parameters of an existing study series (merged into the current ones).
    SetParams {
        /// Study series id.
        series: String,
        /// Parameters to change.
        params: BTreeMap<String, f64>,
    },
    /// Add or replace a whole view (by id).
    PutView {
        /// The view.
        view: View,
    },
    /// Add a layer to a view.
    AddLayer {
        /// View id.
        view: String,
        /// The layer.
        layer: Layer,
        /// Position in the layer list; appended if absent.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        index: Option<u32>,
    },
    /// Remove a layer from a view by position.
    RemoveLayer {
        /// View id.
        view: String,
        /// Position in the layer list.
        index: u32,
    },
    /// Set the visible bar range of a view (persistent; for a transient viewport use
    /// `input`). Bars are 0-based indexes into the view's first bar series, inclusive.
    SetViewRange {
        /// View id.
        view: String,
        /// First visible bar index.
        from_bar: u64,
        /// Last visible bar index (inclusive).
        to_bar: u64,
    },
    /// Record a fact (normally the engine's job).
    AddFact {
        /// Id; assigned if absent.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        /// Event kind, e.g. `cross_above`.
        kind: String,
        /// Series id.
        series: String,
        /// Bar index (0-based).
        bar: u64,
        /// One-sentence statement.
        text: String,
        /// Evidence values.
        #[serde(default)]
        evidence: BTreeMap<String, f64>,
    },
    /// Write a finding (a claim resting on facts), optionally with its capsule.
    AddFinding {
        /// Id; assigned if absent.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        /// The claim, one sentence.
        claim: String,
        /// Fact ids it rests on (must exist).
        facts: Vec<String>,
        /// Confidence in `[0, 1]`.
        confidence: f64,
        /// When this holds, the finding is invalid.
        invalidation: Condition,
        /// Also create a live capsule with this headline.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        capsule: Option<String>,
    },
    /// Change fields of a finding; absent fields stay as they are.
    UpdateFinding {
        /// Finding id.
        finding: String,
        /// New claim.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        claim: Option<String>,
        /// New confidence.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        confidence: Option<f64>,
        /// New invalidation condition.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        invalidation: Option<Condition>,
    },
    /// A person approves a finding: its capsule becomes `approved` and the ledger
    /// records it.
    ApproveFinding {
        /// Finding id.
        finding: String,
        /// Optional note (recorded in the ledger).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        note: Option<String>,
    },
    /// A person rejects a finding: its capsule becomes `rejected` and the ledger
    /// records it with the note.
    RejectFinding {
        /// Finding id.
        finding: String,
        /// Why (recorded in the ledger).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        note: Option<String>,
    },
    /// Leave a persistent request ("tell me if ...").
    AddIntent {
        /// Id; assigned if absent.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        /// The request in words.
        text: String,
        /// Machine-checkable trigger. Series it names must exist.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        trigger: Option<Condition>,
    },
    /// Switch an intent on or off.
    SetIntentActive {
        /// Intent id.
        intent: String,
        /// Active or not.
        active: bool,
    },
    /// Add a narration timeline about an item.
    AddNarrative {
        /// Id; assigned if absent.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        /// Item id it narrates (usually a finding).
        about: String,
        /// Steps, ascending by `at_ms`.
        steps: Vec<NarrativeStep>,
    },
    /// Ask the engine's presenter to build a view for a finding (the agent speaks at the
    /// level of meaning; the presenter writes the grammar). The view is attached to the
    /// finding's capsule.
    ShowFinding {
        /// Finding id.
        finding: String,
        /// Id of the view to create or replace; assigned if absent.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        view: Option<String>,
    },
    /// Remove an item by id. Removing a series also removes the layers that draw it;
    /// it fails with `in_use` if other items (studies, facts, conditions) depend on it.
    Remove {
        /// Item id.
        id: String,
    },
    /// Append bars to a bar series (data feeds).
    AppendBars {
        /// Bar series id.
        series: String,
        /// Bars, strictly after the last bar.
        bars: Vec<Bar>,
    },
    /// Replace the last bar of a bar series (a live, still-forming bar).
    UpdateLastBar {
        /// Bar series id.
        series: String,
        /// The bar; same `t` as the current last bar.
        bar: Bar,
    },
    /// Undo a log entry by writing its inverse as a new entry. Without `seq`: the
    /// speaker's own latest entry that is not yet undone.
    Undo {
        /// Log position to undo.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        seq: Option<Seq>,
    },
    /// Accept a pending proposal: its operations are applied now, in the proposer's name
    /// with the decider recorded as approver.
    AcceptProposal {
        /// Proposal id.
        proposal: String,
        /// Optional note.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        note: Option<String>,
    },
    /// Reject a pending proposal.
    RejectProposal {
        /// Proposal id.
        proposal: String,
        /// Optional note.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        note: Option<String>,
    },
}

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
    },
    /// Insert or replace a view.
    PutView {
        /// The view.
        view: View,
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
    },
    /// Insert or replace a finding.
    PutFinding {
        /// The finding.
        finding: Finding,
    },
    /// Insert or replace a capsule.
    PutCapsule {
        /// The capsule.
        capsule: crate::types::Capsule,
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
    },
    /// Append a ledger entry.
    PushLedger {
        /// The entry.
        entry: LedgerEntry,
    },
    /// Remove the last ledger entry (inverse of `push_ledger`).
    PopLedger,
    /// Insert or replace a narrative.
    PutNarrative {
        /// The narrative.
        narrative: Narrative,
    },
    /// Insert or replace a proposal.
    PutProposal {
        /// The proposal.
        proposal: Proposal,
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
}

// ---------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------

/// Item collections of a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Collection {
    /// Series.
    Series,
    /// Views.
    Views,
    /// Facts.
    Facts,
    /// Findings.
    Findings,
    /// Capsules.
    Capsules,
    /// Intents.
    Intents,
    /// Narratives.
    Narratives,
    /// Proposals.
    Proposals,
}

/// A bar range. Exactly one form: `last` (the last n bars) or `from_bar`/`to_bar`
/// (0-based, inclusive) or `from_t`/`to_t` (epoch ms, inclusive).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Range {
    /// The last n bars.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last: Option<u64>,
    /// First bar index.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_bar: Option<u64>,
    /// Last bar index (inclusive).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_bar: Option<u64>,
    /// From time (inclusive).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_t: Option<TimestampMs>,
    /// To time (inclusive).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_t: Option<TimestampMs>,
}

/// Reduction applied over a range (warmup `null`s are skipped).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Reduce {
    /// Smallest value (and its bar).
    Min,
    /// Largest value (and its bar).
    Max,
    /// Arithmetic mean.
    Mean,
    /// First non-null value.
    First,
    /// Last non-null value.
    Last,
    /// Number of non-null values.
    Count,
}

/// A read request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Query {
    /// What this protocol endpoint offers: operations, queries, topics, principals,
    /// and the asker's own role and permissions.
    Describe,
    /// One item by id, from any collection.
    Get {
        /// Item id.
        id: String,
    },
    /// All items of a collection. For proposals, `state` filters (e.g. `pending`).
    List {
        /// Collection.
        collection: Collection,
        /// Proposal state filter (proposals only).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        state: Option<ProposalState>,
    },
    /// The whole document and its log position (a snapshot, e.g. for hand-over).
    Document,
    /// Log entries. Filters combine with AND.
    Log {
        /// Entries with `seq` greater than this.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        after_seq: Option<Seq>,
        /// Entries at or after this time (epoch ms).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        since_t: Option<TimestampMs>,
        /// Entries at or before this time.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        until_t: Option<TimestampMs>,
        /// Only entries by this principal.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        author: Option<String>,
        /// Return at most this many (the latest ones).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        limit: Option<u32>,
    },
    /// Values of a series over a range, aligned with its bars (K13/K20), optionally
    /// reduced to one number. For a bar series, outputs are `o`,`h`,`l`,`c`,`v`.
    Study {
        /// Series id (a study series or a bar series).
        series: String,
        /// Range; the whole series if absent.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        range: Option<Range>,
        /// Only this output (e.g. `"signal"` of MACD); all if absent.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        output: Option<String>,
        /// Reduce to one value per output.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reduce: Option<Reduce>,
    },
    /// Everything at one bar: OHLCV and the value of every study on that bar series.
    At {
        /// Bar series id.
        series: String,
        /// Bar index (0-based). Give `bar` or `t`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bar: Option<u64>,
        /// Bar time (epoch ms); the bar with exactly this `t`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        t: Option<TimestampMs>,
    },
    /// What a principal's surfaces show right now, semantically (views, panels, series,
    /// visible bar range, capsules). The asker's own if `principal` is absent.
    Screen {
        /// Principal id.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        principal: Option<String>,
    },
    /// Where a principal is looking (last viewport and focus) and the attention
    /// directives addressed to them.
    Attention {
        /// Principal id.
        principal: String,
    },
    /// A finding presented for a surface. Depth follows the asker's negotiated profile
    /// unless `surface` names another surface kind.
    Present {
        /// Finding id. If absent: the most important live finding (highest confidence).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        finding: Option<String>,
        /// Present for this surface kind instead of the asker's.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        surface: Option<SurfaceKind>,
    },
}

/// Values of a series over a range.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SeriesValues {
    /// Series id.
    pub series: String,
    /// First bar index in the answer.
    pub from_bar: u64,
    /// Bar times, one per bar in the range.
    pub t: Vec<TimestampMs>,
    /// Output name -> one value per bar (`null` during warmup).
    pub outputs: BTreeMap<String, Vec<Option<f64>>>,
}

/// One reduced value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Reduced {
    /// The value (`null` when the range has no non-null value).
    pub value: Option<f64>,
    /// Bar index where it occurs (min, max, first, last).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bar: Option<u64>,
    /// Bar time where it occurs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub t: Option<TimestampMs>,
}

/// Everything at one bar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AtBar {
    /// Bar series id.
    pub series: String,
    /// Bar index.
    pub bar: u64,
    /// The bar.
    pub ohlcv: Bar,
    /// Study series id -> output name -> value at this bar.
    pub studies: BTreeMap<String, BTreeMap<String, Option<f64>>>,
}

/// A visible region.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Viewport {
    /// View id.
    pub view: String,
    /// First visible bar index.
    pub from_bar: u64,
    /// Last visible bar index (inclusive).
    pub to_bar: u64,
}

/// One panel as seen on screen.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PanelInfo {
    /// Panel index.
    pub panel: u32,
    /// Series drawn in it, in layer order (e.g. `["px","bb20"]`).
    pub series: Vec<String>,
    /// Marks used, in layer order.
    pub marks: Vec<crate::types::Mark>,
}

/// What one surface shows.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ScreenInfo {
    /// Principal id.
    pub principal: String,
    /// Surface kind.
    pub surface: SurfaceKind,
    /// The visible region.
    pub viewport: Viewport,
    /// Instrument symbol of the view's main series, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    /// Bar interval, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interval: Option<String>,
    /// Panels.
    pub panels: Vec<PanelInfo>,
    /// Capsules whose fact bar is in the visible range.
    pub capsules: Vec<String>,
    /// Last close in the visible range.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_close: Option<f64>,
}

/// Where a principal looks, and what they are asked to look at.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttentionState {
    /// Principal id.
    pub principal: String,
    /// Last reported viewport, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub viewport: Option<Viewport>,
    /// Last focused point (tap or hover), if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focus: Option<Target>,
    /// Time of the last report.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<TimestampMs>,
    /// Directives addressed to this principal, latest last.
    #[serde(default)]
    pub directives: Vec<AttentionDirective>,
}

/// A finding presented at a depth.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Presentation {
    /// Finding id.
    pub finding: String,
    /// Depth chosen.
    pub depth: Depth,
    /// Surface kind it was made for.
    pub surface: SurfaceKind,
    /// One sentence (always present).
    pub text: String,
    /// Capsule state, if the finding has a capsule.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<CapsuleState>,
    /// Glyphs (depth >= capsule; at most `profile.max_glyphs`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub glyphs: Vec<Glyph>,
    /// Confidence of the finding (depth >= capsule).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    /// Small series excerpt (depth >= capsule, if the profile allows series data).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spark: Option<SeriesValues>,
    /// The view (depth >= chart).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view: Option<View>,
    /// Evidence: the facts (depth >= chart).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub facts: Vec<Fact>,
}

/// A protocol self-description (`describe`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Description {
    /// Protocol version.
    pub protocol: String,
    /// Document id.
    pub doc: String,
    /// Operation names accepted in commands.
    pub ops: Vec<String>,
    /// Query kinds.
    pub queries: Vec<String>,
    /// Topic kinds.
    pub topics: Vec<String>,
    /// Study names and their default parameters.
    pub studies: BTreeMap<String, BTreeMap<String, f64>>,
    /// Known principals and roles.
    pub principals: Vec<Principal>,
    /// The asker's role.
    pub role: Role,
    /// The asker's permissions.
    pub permissions: Permissions,
}

/// A log entry as returned by the `log` query.
pub type LogView = LogEntry;

/// The answer to a query or input.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum QueryResult {
    /// Answer to `describe`.
    Description {
        /// The description.
        description: Description,
    },
    /// Answer to `get`: the item, tagged with its collection.
    Item {
        /// Collection the item belongs to.
        collection: Collection,
        /// The item.
        item: serde_json::Value,
    },
    /// Answer to `list`.
    Items {
        /// Collection.
        collection: Collection,
        /// Items.
        items: Vec<serde_json::Value>,
    },
    /// Answer to `document`.
    Document {
        /// The whole document.
        document: Box<Document>,
    },
    /// Answer to `log`.
    Log {
        /// Entries, ascending by seq.
        entries: Vec<LogEntry>,
    },
    /// Answer to `study` without `reduce`.
    Values {
        /// The values.
        values: SeriesValues,
    },
    /// Answer to `study` with `reduce`: output name -> reduced value.
    Reduced {
        /// Series id.
        series: String,
        /// The reduction.
        reduce: Reduce,
        /// Number of bars in the range.
        bars: u64,
        /// Output name -> value.
        values: BTreeMap<String, Reduced>,
    },
    /// Answer to `at` (and to a `tap`).
    At {
        /// The bar and studies.
        at: AtBar,
    },
    /// Answer to `screen`.
    Screen {
        /// One entry per connected surface of the principal.
        screens: Vec<ScreenInfo>,
    },
    /// Answer to `attention`.
    Attention {
        /// The state.
        attention: AttentionState,
    },
    /// Answer to `present`.
    Presentation {
        /// The presentation.
        presentation: Presentation,
    },
    /// Answer to a viewport-changing input.
    Viewport {
        /// The new viewport.
        viewport: Viewport,
    },
}

// ---------------------------------------------------------------------------
// Subscriptions
// ---------------------------------------------------------------------------

/// What a subscription watches.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Topic {
    /// Every applied log entry (optionally only those touching these item ids).
    Changes {
        /// Only entries touching any of these ids.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        ids: Vec<String>,
    },
    /// State changes of one finding's capsule (live -> invalidated, approved, ...).
    Finding {
        /// Finding id.
        finding: String,
    },
    /// A condition becoming true (edge-triggered: notified when it turns true).
    Condition {
        /// The condition.
        condition: Condition,
    },
    /// Proposals created and decided.
    Proposals,
    /// Attention directives addressed to a principal (default: the subscriber) and
    /// that principal's reported viewport.
    Attention {
        /// Principal id.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        principal: Option<String>,
    },
    /// Active intents whose trigger becomes true.
    Intents,
}

/// A pushed event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Event {
    /// A log entry was applied (the Change message).
    Change {
        /// Log position.
        seq: Seq,
        /// Time.
        at: TimestampMs,
        /// Author principal.
        author: String,
        /// Approver principal (accepted proposals).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        approved_by: Option<String>,
        /// Causing request id.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cause: Option<String>,
        /// The changes.
        changes: Vec<ChangeOp>,
    },
    /// A finding's capsule changed state.
    FindingState {
        /// Finding id.
        finding: String,
        /// Capsule id.
        capsule: String,
        /// New state.
        state: CapsuleState,
        /// Log position.
        seq: Seq,
    },
    /// A watched condition became true at a bar.
    ConditionMet {
        /// Bar index.
        bar: u64,
        /// Bar time.
        t: TimestampMs,
        /// Log position.
        seq: Seq,
    },
    /// A proposal was created, accepted, rejected or expired.
    Proposal {
        /// Proposal id.
        proposal: String,
        /// New state.
        state: ProposalState,
        /// Proposer principal.
        author: String,
        /// Log position.
        seq: Seq,
    },
    /// An attention directive for the watched principal.
    Attention {
        /// The directive.
        directive: AttentionDirective,
    },
    /// The watched principal's viewport changed.
    Viewport {
        /// Principal id.
        principal: String,
        /// New viewport.
        viewport: Viewport,
    },
    /// An intent's trigger became true.
    IntentTriggered {
        /// Intent id.
        intent: String,
        /// Bar index.
        bar: u64,
        /// Bar time.
        t: TimestampMs,
        /// Log position.
        seq: Seq,
    },
}

// ---------------------------------------------------------------------------
// Input and attention
// ---------------------------------------------------------------------------

/// A user gesture on a surface. Bars are 0-based indexes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Gesture {
    /// The surface reports what is visible (e.g. after layout or a native scroll).
    Look {
        /// View id.
        view: String,
        /// First visible bar.
        from_bar: u64,
        /// Last visible bar (inclusive).
        to_bar: u64,
    },
    /// Scroll by a number of bars (positive = towards newer bars).
    Pan {
        /// View id.
        view: String,
        /// Bars to move.
        bars: i64,
    },
    /// Zoom by a factor (> 1 shows fewer bars), keeping `anchor_bar` in place
    /// (default: the last visible bar).
    Zoom {
        /// View id.
        view: String,
        /// Factor, > 0.
        factor: f64,
        /// Bar that stays in place.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        anchor_bar: Option<u64>,
    },
    /// Tap or click on a bar in a panel; answered with what is there (`at`).
    Tap {
        /// View id.
        view: String,
        /// Panel index.
        panel: u32,
        /// Bar index.
        bar: u64,
    },
}

/// Where someone wants a person to look, in order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttentionDirective {
    /// Id; assigned if absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Principal who should look.
    pub to: String,
    /// Steps, in the order to show them.
    pub steps: Vec<AttentionStep>,
    /// Who sent it (filled in by the authority; ignored if sent).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    /// Time sent (filled in by the authority).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<TimestampMs>,
}

/// One step of an attention directive.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttentionStep {
    /// What to look at.
    pub target: Target,
    /// Optional short caption.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub say: Option<String>,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Error codes. Each has a fixed meaning; see `docs/PROTOCOL.md` §13.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    /// The message is not valid JSON.
    Parse,
    /// The message does not match the schema (unknown field, wrong type, missing field).
    Schema,
    /// `hello` missing, repeated, or unknown principal / protocol version.
    Handshake,
    /// The principal may not do this.
    Forbidden,
    /// A referenced id does not exist.
    NotFound,
    /// An id to be created already exists.
    Duplicate,
    /// A value is out of range or inconsistent (bad parameter, bad bar, bad range).
    Invalid,
    /// The item is used by other items and cannot be removed.
    InUse,
    /// An item touched by the command changed after `base`.
    Conflict,
    /// Undo is not possible (nothing to undo, or later entries touched the same items).
    UndoConflict,
    /// The proposal is not pending.
    NotPending,
    /// A size limit was exceeded.
    Limit,
    /// Recognized but not supported here (e.g. a feature not negotiated).
    Unsupported,
}

/// A protocol error. Written so that an agent can correct its message without help.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProtocolError {
    /// Error code.
    pub code: ErrorCode,
    /// What went wrong, in one sentence.
    pub message: String,
    /// JSON pointer into the offending message, e.g. `/ops/0/params/period`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Accepted values or forms at `path`, when they are a short list.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub expected: Vec<String>,
    /// Existing ids or names close to the one given (edit distance).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub did_you_mean: Vec<String>,
    /// For `conflict` / `undo_conflict`: the log positions that conflict.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conflicts: Vec<Seq>,
    /// How to fix it, in one sentence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

// ---------------------------------------------------------------------------
// Line transport framing
// ---------------------------------------------------------------------------

/// A line of the line-based transport (`qp serve`), client to authority: one JSON
/// object per line. `conn` names the logical connection, so several clients can share
/// one stream. The first message on a connection must be `hello`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InboundLine {
    /// Logical connection id.
    pub conn: String,
    /// The message.
    pub msg: ClientMsg,
}

/// A line of the line-based transport, authority to client.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OutboundLine {
    /// Logical connection id.
    pub conn: String,
    /// The message.
    pub msg: ServerMsg,
}

// ---------------------------------------------------------------------------
// Host configuration and conformance transcripts
// ---------------------------------------------------------------------------

/// A document given inline or as a path (relative to the repository root).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum DocumentRef {
    /// Path to a `Document` JSON file.
    Path(String),
    /// The document itself.
    Inline(Box<Document>),
}

/// How an authority starts (host configuration; not a protocol message).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SessionInit {
    /// The initial document.
    pub document: DocumentRef,
    /// Known principals.
    pub principals: Vec<Principal>,
    /// Initial time, epoch ms.
    pub clock: TimestampMs,
    /// Authority epoch.
    #[serde(default)]
    pub epoch: u64,
    /// Proposals expire this long after creation; never if absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proposal_ttl_ms: Option<u64>,
}

/// A session init given inline or as a path.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum InitRef {
    /// Path to a `SessionInit` JSON file.
    Path(String),
    /// The init itself.
    Inline(Box<SessionInit>),
}

/// A raw (possibly invalid) line sent on a connection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RawLine {
    /// Logical connection id.
    pub conn: String,
    /// The text of the message, sent as is.
    pub line: String,
}

/// An expected outbound message: `msg` is a pattern matched as a subset
/// (see `docs/PROTOCOL.md` §15).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Expect {
    /// Connection the message goes to.
    pub conn: String,
    /// Pattern of the message.
    pub msg: serde_json::Value,
}

/// One step of a transcript: send one message, expect exactly these outputs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Step {
    /// A well-formed message to send.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub send: Option<InboundLine>,
    /// A raw line to send instead (for parse and schema errors).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub send_raw: Option<RawLine>,
    /// Expected outputs, complete and in order.
    pub expect: Vec<Expect>,
    /// Optional comment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Checks after the last step.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FinalCheck {
    /// Pattern the final document must match (subset).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document: Option<serde_json::Value>,
    /// Exact number of log entries expected, if given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub log_len: Option<u64>,
}

/// A conformance transcript: language- and transport-independent
/// "input messages -> expected output messages" scenario.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Transcript {
    /// Short name (file stem).
    pub name: String,
    /// What it shows.
    pub description: String,
    /// Message kinds and features it covers.
    #[serde(default)]
    pub covers: Vec<String>,
    /// How the authority starts.
    pub init: InitRef,
    /// Steps, in order.
    pub steps: Vec<Step>,
    /// Final checks. Replay byte-equality is always checked.
    #[serde(default, rename = "final")]
    pub final_check: FinalCheck,
}
