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

use crate::grammar::{Layer, View};
use crate::types::{
    Author, Bar, CapsuleState, Condition, Depth, Document, Fact, Finding, Glyph, Intent,
    LedgerEntry, Series, TimestampMs,
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
        /// Ids inside also take selectors (§17).
        #[schemars(with = "AttentionDirectiveIn")]
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
// The `result` variant is large; boxing it would change the public API for no gain in
// this message-at-a-time protocol.
#[allow(clippy::large_enum_variant)]
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
        /// The authority's current time (from the last `clock`), epoch ms. Use it for
        /// relative questions ("in the last 30 minutes").
        now: TimestampMs,
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
        /// What the document contains, compactly, so the first message can already
        /// refer to it (R2).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        summary: Option<DocSummary>,
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
        /// For an applied `undo`: the log position that was undone.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        undoes: Option<Seq>,
        /// References by meaning resolved in this command (R2): JSON pointer in the
        /// command → the id it resolved to. The log stores the ids.
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        resolved: BTreeMap<String, String>,
    },
    /// Reply to `query` and `input`.
    Result {
        /// The request id.
        id: String,
        /// Log position the answer reflects.
        seq: Seq,
        /// The authority's current time, epoch ms.
        now: TimestampMs,
        /// The answer.
        data: QueryResult,
        /// Selectors resolved in the query (R2, §17): JSON pointer in the message → the
        /// id it resolved to (e.g. `{"/query/series": "rsi14"}`).
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        resolved: BTreeMap<String, String>,
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
    /// Also takes a selector (§17).
    #[schemars(with = "ViewRef")]
    pub view: String,
    /// Put it in the panel where this series is drawn (e.g. `"rsi14"`). Takes precedence
    /// over `panel`. If neither is given, a new panel is added below the others.
    /// Also takes a selector (§17).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Option<SeriesRef>")]
    pub with: Option<String>,
    /// Panel id (R2: the panel's stable id, not its position; in a view loaded from a
    /// `qp/0` document the ids equal the old positions).
    /// Also takes a selector (§17).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Option<PanelRef>")]
    pub panel: Option<u32>,
}

/// A step of a narration or an attention directive points at something.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Target {
    /// View id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view: Option<String>,
    /// Panel id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub panel: Option<u32>,
    /// Layer id (R2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer: Option<String>,
    /// Series id (e.g. the series whose value matters).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub series: Option<String>,
    /// Output or field of the series, e.g. `"v"` (volume of a bar series), `"signal"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
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
        /// Also takes a selector (§17).
        #[schemars(with = "Vec<SeriesRef>")]
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
        /// Also takes a selector (§17).
        #[schemars(with = "SeriesRef")]
        series: String,
        /// Parameters to change.
        params: BTreeMap<String, f64>,
    },
    /// Add or replace a whole view (by id).
    PutView {
        /// The view.
        view: View,
    },
    /// Add a layer to a view. Prefer `insert_layer` (placement by id).
    AddLayer {
        /// View id.
        /// Also takes a selector (§17).
        #[schemars(with = "ViewRef")]
        view: String,
        /// The layer.
        /// Ids inside also take selectors (§17).
        #[schemars(with = "LayerIn")]
        layer: Layer,
        /// **Deprecated (`qp/0`).** Position in the layer list; appended if absent.
        /// Positions shift under concurrent writes; use `insert_layer` with
        /// `after`/`before`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("deprecated" = true))]
        index: Option<u32>,
    },
    /// **Deprecated (`qp/0`).** Remove a layer from a view by position; use
    /// `delete_layer` (by id).
    #[schemars(extend("deprecated" = true))]
    RemoveLayer {
        /// View id.
        /// Also takes a selector (§17).
        #[schemars(with = "ViewRef")]
        view: String,
        /// Position in the layer list.
        index: u32,
    },
    /// Set the visible bar range of a view (persistent; for a transient viewport use
    /// `input`). Bars are 0-based indexes into the view's first bar series, inclusive.
    SetViewRange {
        /// View id.
        /// Also takes a selector (§17).
        #[schemars(with = "ViewRef")]
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
        /// Also takes a selector (§17).
        #[schemars(with = "SeriesRef")]
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
        /// Also takes a selector (§17).
        #[schemars(with = "Vec<FactRef>")]
        facts: Vec<String>,
        /// Confidence in `[0, 1]`.
        confidence: f64,
        /// When this holds, the finding is invalid.
        /// Ids inside also take selectors (§17).
        #[schemars(with = "ConditionIn")]
        invalidation: Condition,
        /// Also create a live capsule with this headline.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        capsule: Option<String>,
    },
    /// Change fields of a finding; absent fields stay as they are.
    UpdateFinding {
        /// Finding id.
        /// Also takes a selector (§17).
        #[schemars(with = "FindingRef")]
        finding: String,
        /// New claim.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        claim: Option<String>,
        /// New confidence.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        confidence: Option<f64>,
        /// New invalidation condition.
        /// Ids inside also take selectors (§17).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(with = "Option<ConditionIn>")]
        invalidation: Option<Condition>,
    },
    /// A person approves a finding: its capsule becomes `approved` and the ledger
    /// records it.
    ApproveFinding {
        /// Finding id.
        /// Also takes a selector (§17).
        #[schemars(with = "FindingRef")]
        finding: String,
        /// Optional note (recorded in the ledger).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        note: Option<String>,
    },
    /// A person rejects a finding: its capsule becomes `rejected` and the ledger
    /// records it with the note.
    RejectFinding {
        /// Finding id.
        /// Also takes a selector (§17).
        #[schemars(with = "FindingRef")]
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
        /// Ids inside also take selectors (§17).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(with = "Option<ConditionIn>")]
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
        /// Also takes a selector (§17).
        #[schemars(with = "ItemRef")]
        about: String,
        /// Steps, ascending by `at_ms`.
        /// Ids inside also take selectors (§17).
        #[schemars(with = "Vec<NarrativeStepIn>")]
        steps: Vec<NarrativeStep>,
    },
    /// Ask the engine's presenter to build a view for a finding (the agent speaks at the
    /// level of meaning; the presenter writes the grammar). The view is attached to the
    /// finding's capsule.
    ShowFinding {
        /// Finding id.
        /// Also takes a selector (§17).
        #[schemars(with = "FindingRef")]
        finding: String,
        /// Id of the view to create or replace; assigned if absent.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        view: Option<String>,
    },
    /// Insert a layer into a view, placed by id: after layer `after`, before layer
    /// `before`, or at the end. The layer's `panel` must name an existing panel (R2).
    InsertLayer {
        /// View id.
        /// Also takes a selector (§17).
        #[schemars(with = "ViewRef")]
        view: String,
        /// The layer (id assigned if absent).
        /// Ids inside also take selectors (§17).
        #[schemars(with = "LayerIn")]
        layer: Layer,
        /// Place it right after this layer.
        /// Also takes a selector (§17).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(with = "Option<LayerRef>")]
        after: Option<String>,
        /// Place it right before this layer.
        /// Also takes a selector (§17).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(with = "Option<LayerRef>")]
        before: Option<String>,
    },
    /// Delete a layer by id.
    DeleteLayer {
        /// View id.
        /// Also takes a selector (§17).
        #[schemars(with = "ViewRef")]
        view: String,
        /// Layer id.
        /// Also takes a selector (§17).
        #[schemars(with = "LayerRef")]
        layer: String,
    },
    /// Move a layer: to another panel, and/or to another place in the paint order.
    MoveLayer {
        /// View id.
        /// Also takes a selector (§17).
        #[schemars(with = "ViewRef")]
        view: String,
        /// Layer id.
        /// Also takes a selector (§17).
        #[schemars(with = "LayerRef")]
        layer: String,
        /// Target panel id.
        /// Also takes a selector (§17).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(with = "Option<PanelRef>")]
        panel: Option<u32>,
        /// Place it right after this layer.
        /// Also takes a selector (§17).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(with = "Option<LayerRef>")]
        after: Option<String>,
        /// Place it right before this layer.
        /// Also takes a selector (§17).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(with = "Option<LayerRef>")]
        before: Option<String>,
    },
    /// Change a layer's encoding: the channels given replace the current ones, the
    /// channels in `unset` are removed; `origin` entries are merged. Other channels stay.
    SetEncoding {
        /// View id.
        /// Also takes a selector (§17).
        #[schemars(with = "ViewRef")]
        view: String,
        /// Layer id.
        /// Also takes a selector (§17).
        #[schemars(with = "LayerRef")]
        layer: String,
        /// Channels to set.
        /// Ids inside also take selectors (§17).
        #[serde(default)]
        #[schemars(with = "EncodingIn")]
        encoding: crate::grammar::Encoding,
        /// Channel names to remove (`x`, `y`, `y2`, `color`, `opacity`, `width`,
        /// `size`, `text`).
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        unset: Vec<String>,
        /// Origins to add or replace (JSON pointer inside the layer → origin).
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        origin: BTreeMap<String, crate::grammar::Origin>,
        /// New legend text.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<String>,
    },
    /// Insert a panel (with its layers, optionally) after/before another panel, or at
    /// the bottom. `ack.created`: the panel id (decimal), then the layer ids.
    InsertPanel {
        /// View id.
        /// Also takes a selector (§17).
        #[schemars(with = "ViewRef")]
        view: String,
        /// The panel (id assigned if absent: largest id + 1).
        panel: crate::grammar::Panel,
        /// Layers to create in it (their `panel` field is set to the new panel).
        /// Ids inside also take selectors (§17).
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        #[schemars(with = "Vec<LayerIn>")]
        layers: Vec<Layer>,
        /// Place it right below this panel.
        /// Also takes a selector (§17).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(with = "Option<PanelRef>")]
        after: Option<u32>,
        /// Place it right above this panel.
        /// Also takes a selector (§17).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(with = "Option<PanelRef>")]
        before: Option<u32>,
    },
    /// Delete a panel and every layer drawn in it.
    DeletePanel {
        /// View id.
        /// Also takes a selector (§17).
        #[schemars(with = "ViewRef")]
        view: String,
        /// Panel id.
        /// Also takes a selector (§17).
        #[schemars(with = "PanelRef")]
        panel: u32,
    },
    /// Move a panel up or down the stack.
    MovePanel {
        /// View id.
        /// Also takes a selector (§17).
        #[schemars(with = "ViewRef")]
        view: String,
        /// Panel id.
        /// Also takes a selector (§17).
        #[schemars(with = "PanelRef")]
        panel: u32,
        /// Place it right below this panel.
        /// Also takes a selector (§17).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(with = "Option<PanelRef>")]
        after: Option<u32>,
        /// Place it right above this panel.
        /// Also takes a selector (§17).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(with = "Option<PanelRef>")]
        before: Option<u32>,
    },
    /// Rename a panel or change its relative height.
    SetPanel {
        /// View id.
        /// Also takes a selector (§17).
        #[schemars(with = "ViewRef")]
        view: String,
        /// Panel id.
        /// Also takes a selector (§17).
        #[schemars(with = "PanelRef")]
        panel: u32,
        /// New name.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        /// New relative height (> 0).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        weight: Option<f64>,
    },
    /// Change a scale by id (`"x"`, `"y<panel>"`): kind and/or fixed domain.
    SetScale {
        /// View id.
        /// Also takes a selector (§17).
        #[schemars(with = "ViewRef")]
        view: String,
        /// Scale id.
        /// Also takes a selector (§17).
        #[schemars(with = "ScaleRef")]
        scale: String,
        /// New kind (`linear`/`log` for value scales; the time axis stays `time`).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        kind: Option<crate::grammar::ScaleKind>,
        /// New fixed domain `[min, max]` (on `x`: epoch ms).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        domain: Option<[f64; 2]>,
        /// Clear the fixed domain (automatic again).
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        auto_domain: bool,
    },
    /// Replace a view's persistent emphasis list (empty clears it).
    SetEmphasis {
        /// View id.
        /// Also takes a selector (§17).
        #[schemars(with = "ViewRef")]
        view: String,
        /// The emphasis list.
        /// Ids inside also take selectors (§17).
        #[schemars(with = "Vec<EmphasisIn>")]
        emphasis: Vec<crate::grammar::Emphasis>,
    },
    /// Remove an item by id. Removing a series also removes the layers that draw it;
    /// it fails with `in_use` if other items (studies, facts, conditions) depend on it.
    Remove {
        /// Item id.
        /// Also takes a selector (§17).
        #[schemars(with = "ItemRef")]
        id: String,
    },
    /// Append bars to a bar series (data feeds).
    AppendBars {
        /// Bar series id.
        /// Also takes a selector (§17).
        #[schemars(with = "SeriesRef")]
        series: String,
        /// Bars, strictly after the last bar.
        bars: Vec<Bar>,
    },
    /// Replace the last bar of a bar series (a live, still-forming bar).
    UpdateLastBar {
        /// Bar series id.
        /// Also takes a selector (§17).
        #[schemars(with = "SeriesRef")]
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
        /// Also takes a selector (§17).
        #[schemars(with = "ProposalRef")]
        proposal: String,
        /// Optional note.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        note: Option<String>,
    },
    /// Reject a pending proposal.
    RejectProposal {
        /// Proposal id.
        /// Also takes a selector (§17).
        #[schemars(with = "ProposalRef")]
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
    Describe {},
    /// One item by id, from any collection.
    Get {
        /// Item id.
        /// Also takes a selector (§17).
        #[schemars(with = "ItemRef")]
        id: String,
    },
    /// All items of a collection. For proposals, `state` filters (e.g. `pending`).
    List {
        /// Collection.
        collection: Collection,
        /// Proposal state filter (proposals only).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        state: Option<ProposalState>,
        /// Include bulky data (the bars of bar series). Default false: series are listed
        /// without `data`; read values with the `study` or `at` query, or `get` one item.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        full: Option<bool>,
    },
    /// The whole document and its log position (a snapshot, e.g. for hand-over).
    Document {},
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
        /// Entries from the last this-many milliseconds (relative to `now`), e.g.
        /// `1800000` for "the last 30 minutes".
        #[serde(default, skip_serializing_if = "Option::is_none")]
        within_ms: Option<u64>,
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
        /// Also takes a selector (§17).
        #[schemars(with = "SeriesRef")]
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
        /// Also takes a selector (§17).
        #[schemars(with = "SeriesRef")]
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
    /// Point query (R2): what is drawn at pixel (`x`, `y`) of a view rendered at
    /// `width` x `height`. Answered from the scene description, without rendering. The
    /// visible range is the asker's viewport of that view if it has one, otherwise the
    /// view's persistent range, otherwise all bars.
    Hit {
        /// View id; the asker's current view if absent.
        /// Also takes a selector (§17).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(with = "Option<ViewRef>")]
        view: Option<String>,
        /// Pixel x (0 = left edge).
        x: f64,
        /// Pixel y (0 = top edge).
        y: f64,
        /// Frame width in pixels (default 1200).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        width: Option<u32>,
        /// Frame height in pixels (default 600).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        height: Option<u32>,
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
        /// Also takes a selector (§17).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(with = "Option<FindingRef>")]
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
    /// Panel id (R2: stable id; equals the old position in `qp/0` documents).
    pub panel: u32,
    /// Series drawn in it, in layer order (e.g. `["px","bb20"]`).
    pub series: Vec<String>,
    /// Marks used, in layer order.
    pub marks: Vec<crate::grammar::Mark>,
    /// Panel name, if any (R2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Value scale kind and visible domain `[min, max]` (R2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale: Option<ScaleInfo>,
    /// Layers in paint order (R2).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub layers: Vec<LayerInfo>,
    /// A selector that resolves to this panel in its view now (R2, §17).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub select: Option<PanelSel>,
}

/// A value scale as drawn (R2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ScaleInfo {
    /// Scale id.
    pub id: String,
    /// Kind.
    pub kind: crate::grammar::ScaleKind,
    /// Visible domain `[min, max]`.
    pub domain: [f64; 2],
}

/// One layer as drawn, semantically (R2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LayerInfo {
    /// Layer id.
    pub id: String,
    /// Mark.
    pub mark: crate::grammar::Mark,
    /// Data source.
    pub data: crate::scene::DataSource,
    /// Fields drawn.
    pub fields: Vec<String>,
    /// Legend text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Constants and their origins.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub constants: BTreeMap<String, crate::scene::ResolvedConstant>,
    /// Number of drawn elements in the visible range.
    pub items: u64,
    /// Last drawn values (field → value) in the visible range, if any.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub last: BTreeMap<String, f64>,
    /// A selector that resolves to this layer in its view now (R2, §17).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub select: Option<LayerSel>,
}

/// Answer to the point query (R2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HitInfo {
    /// View id.
    pub view: String,
    /// Panel under the pixel, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub panel: Option<u32>,
    /// Bar slot under the pixel, if inside the plot.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bar: Option<u64>,
    /// Time of that bar, if it exists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub t: Option<TimestampMs>,
    /// The panel's scale value at the pixel's y.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
    /// The topmost element at the pixel, if any ("no item" when absent).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item: Option<HitItem>,
}

/// The element found by a point query (R2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HitItem {
    /// Layer id.
    pub layer: String,
    /// Mark.
    pub mark: crate::grammar::Mark,
    /// Data source.
    pub data: crate::scene::DataSource,
    /// Bar of the element, if it has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bar: Option<u64>,
    /// Time of the element, if it has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub t: Option<TimestampMs>,
    /// Values drawn by the element.
    pub values: BTreeMap<String, f64>,
    /// Origins of constants the element draws, if any.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub constants: BTreeMap<String, crate::scene::ResolvedConstant>,
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
    /// The capsule view (R2; depth `capsule`, only if the profile allows series data):
    /// one panel with the main output of the fact's series over the last `max_points`
    /// bars, a rule at the fact's level and a point at the fact bar
    /// (`docs/GRAMMAR.md` §13). Not written to the document.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capsule_view: Option<Box<View>>,
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
    /// Named thresholds usable as origins (R2), e.g. `rsi.overbought` → 70.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub thresholds: BTreeMap<String, f64>,
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
    /// Answer to the point query `hit` (R2).
    Hit {
        /// What is at the pixel.
        hit: HitInfo,
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
        /// Also takes a selector (§17).
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        #[schemars(with = "Vec<ItemRef>")]
        ids: Vec<String>,
    },
    /// State changes of one finding's capsule (live -> invalidated, approved, ...).
    Finding {
        /// Finding id.
        /// Also takes a selector (§17).
        #[schemars(with = "FindingRef")]
        finding: String,
    },
    /// A condition becoming true (edge-triggered: notified when it turns true).
    Condition {
        /// The condition.
        /// Ids inside also take selectors (§17).
        #[schemars(with = "ConditionIn")]
        condition: Condition,
    },
    /// Proposals created and decided.
    Proposals {},
    /// Attention directives addressed to a principal (default: the subscriber) and
    /// that principal's reported viewport.
    Attention {
        /// Principal id.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        principal: Option<String>,
    },
    /// Active intents whose trigger becomes true.
    Intents {},
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
    /// A numeric constant of a layer has no origin (fact, named threshold or series
    /// value) (R2). `path` points at the constant; `candidates` lists origins to use.
    NoOrigin,
    /// A reference by meaning matches more than one item (R2). `candidates` lists them,
    /// ranked (the kind's order, §17); the authority never picks one at random.
    Ambiguous,
}

/// One option offered by an error (R2): an id (or an origin, as JSON) and what it is.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Candidate {
    /// The id to use, or the value to write (e.g. an origin object as JSON text).
    pub id: String,
    /// What it is, in a few words.
    pub about: String,
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
    /// For `no_origin` and `ambiguous` (R2): the options, best first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub candidates: Vec<Candidate>,
}

// ---------------------------------------------------------------------------
// Document summary (R2)
// ---------------------------------------------------------------------------

/// A compact summary of a document: enough to refer to its items in the first message.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DocSummary {
    /// Series.
    pub series: Vec<SeriesSummary>,
    /// Views.
    pub views: Vec<ViewSummary>,
    /// Facts, latest bar last.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub facts: Vec<FactSummary>,
    /// Findings, latest fact last.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub findings: Vec<FindingSummary>,
    /// Pending proposals.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_proposals: Option<u32>,
    /// True when lists were cut to fit the summary limit.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub truncated: bool,
    /// How to refer to the listed items by meaning (R2, §17): for each item, a selector
    /// that resolves to it now. An agent can copy these instead of ids.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub select: Option<SelectHints>,
}

/// Selectors that resolve to the items of a summary (R2, §17), keyed by item id
/// (panels: `"<view>/<panel id>"`, layers: `"<view>/<layer id>"`; resolved within that
/// view). Items for which no selector of the vocabulary is unique are left out.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SelectHints {
    /// Series id → selector.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub series: BTreeMap<String, SeriesSel>,
    /// View id → selector.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub views: BTreeMap<String, ViewSel>,
    /// `"<view>/<panel id>"` → selector (within that view).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub panels: BTreeMap<String, PanelSel>,
    /// `"<view>/<layer id>"` → selector (within that view).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub layers: BTreeMap<String, LayerSel>,
    /// Fact id → selector.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub facts: BTreeMap<String, FactSel>,
    /// Finding id → selector.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub findings: BTreeMap<String, FindingSel>,
}

/// One series in a summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SeriesSummary {
    /// Series id.
    pub id: String,
    /// `bars`, `points` or `study`.
    pub kind: String,
    /// Study name, for a study series.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub study: Option<String>,
    /// Study parameters (complete, with defaults).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub params: BTreeMap<String, f64>,
    /// Input series ids.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inputs: Vec<String>,
    /// Fields / outputs (`o h l c v`, `upper middle lower bandwidth`...).
    pub fields: Vec<String>,
    /// Instrument symbol.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    /// Interval.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interval: Option<String>,
    /// Number of bars (or points).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub len: Option<u64>,
    /// Legend text (e.g. `RSI14`).
    pub label: String,
}

/// One view in a summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ViewSummary {
    /// View id.
    pub id: String,
    /// Title, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Panels, top to bottom.
    pub panels: Vec<PanelSummary>,
    /// Number of emphasis entries.
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    pub emphasis: u32,
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_zero_u32(v: &u32) -> bool {
    *v == 0
}

/// One panel in a summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PanelSummary {
    /// Panel id.
    pub id: u32,
    /// Name, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Scale id and kind, e.g. `y0 linear`.
    pub scale: String,
    /// Layers in paint order.
    pub layers: Vec<LayerSummary>,
}

/// One layer in a summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LayerSummary {
    /// Layer id.
    pub id: String,
    /// Mark.
    pub mark: crate::grammar::Mark,
    /// What it draws, in a few words: `px candles`, `rsi14.rsi line`, `rule 70
    /// (rsi.overbought)`.
    pub about: String,
}

/// One fact in a summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FactSummary {
    /// Fact id.
    pub id: String,
    /// Kind.
    pub kind: String,
    /// Series id.
    pub series: String,
    /// Bar index.
    pub bar: u64,
    /// Text.
    pub text: String,
}

/// One finding in a summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FindingSummary {
    /// Finding id.
    pub id: String,
    /// Claim.
    pub claim: String,
    /// Fact ids.
    pub facts: Vec<String>,
    /// Latest bar among its facts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bar: Option<u64>,
    /// Capsule id and state, if any (e.g. `cap-9 live`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capsule: Option<String>,
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

// ---------------------------------------------------------------------------
// Reference by meaning (R2, `docs/PROTOCOL.md` §17): selectors
// ---------------------------------------------------------------------------
//
// Every message field that takes an item id also accepts a *selector*: an object that
// describes the item by its attributes. The authority resolves each selector against
// the current document (deterministically, `docs/PROTOCOL.md` §17), substitutes the id,
// returns `{pointer -> id}` in `ack.resolved` / `result.resolved`, and logs the request
// with ids only. The types below are the closed vocabulary; message fields name them
// through `#[schemars(with = ...)]`, so the generated JSON Schema of those fields is
// "id or selector" while the stored form (log, proposals, documents) is an id.
//
// Common to every selector: all given keys must hold (AND); `{}` matches every item of
// the kind; `first` / `latest` pick the first / last match in the kind's order (the same
// order in which an `ambiguous` error lists its candidates). Without them, more than one
// match is `ambiguous` and none is `not_found`.

#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_false(b: &bool) -> bool {
    !*b
}

/// Kind of a series, as in `welcome.summary` (`SeriesSummary::kind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SeriesKind {
    /// A bar series (OHLCV).
    Bars,
    /// A computed (study) series.
    Study,
    /// A point series (e.g. a forecast).
    Points,
}

/// Selects a series by meaning, e.g. `{"study":"rsi","params":{"period":14}}` ("RSI(14)"),
/// `{"kind":"bars"}` ("the price series"), `{"study":"sma","params":{"period":50}}`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SeriesSel {
    /// `bars`, `study` or `points`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<SeriesKind>,
    /// Study name (`sma`, `ema`, `rsi`, `macd`, `bollinger`), exact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub study: Option<String>,
    /// Study parameters that must be equal (each one given; others free). Parameters
    /// are complete with defaults in the document, so `{"period":14}` matches a default
    /// RSI.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub params: BTreeMap<String, f64>,
    /// A study computed on this series (one of its inputs matches).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on: Option<Box<SeriesRef>>,
    /// Instrument symbol, case-insensitive (e.g. `"THYAO"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    /// Legend text, case-insensitive (as in `welcome.summary`: `RSI14`, `MACD(12,26,9)`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Pick the first match (document order).
    #[serde(default, skip_serializing_if = "is_false")]
    pub first: bool,
    /// Pick the last match (document order: the most recently added).
    #[serde(default, skip_serializing_if = "is_false")]
    pub latest: bool,
}

/// Selects a view, e.g. `{}` (the only view), `{"shows":{"study":"macd"}}`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ViewSel {
    /// Title, case-insensitive (the given title, or the derived `SYMBOL interval`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// A layer of the view draws this series.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shows: Option<Box<SeriesRef>>,
    /// Pick the first match (document order).
    #[serde(default, skip_serializing_if = "is_false")]
    pub first: bool,
    /// Pick the last match (document order).
    #[serde(default, skip_serializing_if = "is_false")]
    pub latest: bool,
}

/// Selects a panel of the view named by the same op (or target), e.g.
/// `{"shows":{"study":"rsi"}}` ("RSI's panel"), `{"mark":"candle"}` ("the price panel"),
/// `{"name":"macd"}`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PanelSel {
    /// Panel name, case-insensitive.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// A layer in the panel draws this series.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shows: Option<Box<SeriesRef>>,
    /// A layer in the panel has this mark.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mark: Option<crate::grammar::Mark>,
    /// Pick the first match (stack order: the top panel).
    #[serde(default, skip_serializing_if = "is_false")]
    pub first: bool,
    /// Pick the last match (stack order: the bottom panel).
    #[serde(default, skip_serializing_if = "is_false")]
    pub latest: bool,
}

/// Selects a layer of the view named by the same op (or target), e.g.
/// `{"shows":{"study":"bollinger"},"output":"upper"}` ("the upper band"),
/// `{"mark":"rule","value":70}` ("the 70 line").
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LayerSel {
    /// The layer's data series matches.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shows: Option<Box<SeriesRef>>,
    /// The layer draws this field: its `y` or `y2` names it, or `y` is absent and the
    /// field is one the layer draws by default (a `line` on Bollinger draws `upper`,
    /// `middle`, `lower`; otherwise the main output).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    /// Mark.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mark: Option<crate::grammar::Mark>,
    /// The layer is in this panel.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub panel: Option<Box<PanelRef>>,
    /// A constant of the layer (`y` or `x`) equals this number exactly (a rule's level).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
    /// The layer's data is this fact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fact: Option<Box<FactRef>>,
    /// The layer's data is this finding.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finding: Option<Box<FindingRef>>,
    /// Legend text, case-insensitive (the given label or the derived one, `RSI14`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Pick the first match (paint order).
    #[serde(default, skip_serializing_if = "is_false")]
    pub first: bool,
    /// Pick the last match (paint order: the topmost).
    #[serde(default, skip_serializing_if = "is_false")]
    pub latest: bool,
}

/// Selects a fact, e.g. `{"kind":"breakout","latest":true}`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FactSel {
    /// Fact kind, exact (`cross_above`, `breakout`...).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// The fact is about this series.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on: Option<Box<SeriesRef>>,
    /// Bar index.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bar: Option<u64>,
    /// Pick the first match (by bar: the oldest).
    #[serde(default, skip_serializing_if = "is_false")]
    pub first: bool,
    /// Pick the last match (by bar: the most recent).
    #[serde(default, skip_serializing_if = "is_false")]
    pub latest: bool,
}

/// Selects a finding, e.g. `{"kind":"breakout","latest":true}` ("the latest breakout").
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FindingSel {
    /// One of its facts has this kind.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// One of its facts is about this series.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on: Option<Box<SeriesRef>>,
    /// It rests on this fact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fact: Option<Box<FactRef>>,
    /// State of its capsule (a finding without a capsule has none).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<CapsuleState>,
    /// Pick the first match (by its latest fact bar: the oldest).
    #[serde(default, skip_serializing_if = "is_false")]
    pub first: bool,
    /// Pick the last match (by its latest fact bar: the most recent).
    #[serde(default, skip_serializing_if = "is_false")]
    pub latest: bool,
}

/// Selects a proposal, e.g. `{"state":"pending","latest":true}`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProposalSel {
    /// State.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<ProposalState>,
    /// Pick the first match (document order).
    #[serde(default, skip_serializing_if = "is_false")]
    pub first: bool,
    /// Pick the last match (document order: the most recent).
    #[serde(default, skip_serializing_if = "is_false")]
    pub latest: bool,
}

/// Selects a value scale of the op's view by its panel: `{"panel":{"mark":"candle"}}`
/// is `"y<id of the price panel>"`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ScaleSel {
    /// The panel whose value scale is meant.
    pub panel: PanelRef,
}

/// Selects any item, naming its kind: `{"finding":{"kind":"breakout","latest":true}}`,
/// `{"fact":{"bar":120}}`. Used where a field takes an id of any collection
/// (`target.item`, `add_narrative.about`, `remove.id`, `get.id`, `changes.ids`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ItemSel {
    /// A series.
    Series(SeriesSel),
    /// A view.
    View(ViewSel),
    /// A fact.
    Fact(FactSel),
    /// A finding.
    Finding(FindingSel),
    /// A proposal.
    Proposal(ProposalSel),
}

/// A series id or a [`SeriesSel`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum SeriesRef {
    /// Series id.
    Id(String),
    /// Selector.
    Select(SeriesSel),
}

/// A view id or a [`ViewSel`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ViewRef {
    /// View id.
    Id(String),
    /// Selector.
    Select(ViewSel),
}

/// A panel id (number) or a [`PanelSel`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum PanelRef {
    /// Panel id.
    Id(u32),
    /// Selector.
    Select(PanelSel),
}

impl Default for PanelRef {
    fn default() -> Self {
        Self::Id(0)
    }
}

/// A layer id or a [`LayerSel`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum LayerRef {
    /// Layer id.
    Id(String),
    /// Selector.
    Select(LayerSel),
}

/// A fact id or a [`FactSel`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum FactRef {
    /// Fact id.
    Id(String),
    /// Selector.
    Select(FactSel),
}

/// A finding id or a [`FindingSel`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum FindingRef {
    /// Finding id.
    Id(String),
    /// Selector.
    Select(FindingSel),
}

/// A proposal id or a [`ProposalSel`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ProposalRef {
    /// Proposal id.
    Id(String),
    /// Selector.
    Select(ProposalSel),
}

/// A scale id (`"x"`, `"y<panel>"`) or a [`ScaleSel`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ScaleRef {
    /// Scale id.
    Id(String),
    /// Selector.
    Select(ScaleSel),
}

/// An item id (any collection) or an [`ItemSel`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ItemRef {
    /// Item id.
    Id(String),
    /// Selector.
    Select(ItemSel),
}

// Input shapes of composite fields that accept selectors inside. They describe the
// *wire* form of these fields (schema only, through `#[schemars(with = ...)]`); the
// authority resolves the selectors and then parses the stored types (`Target`,
// `grammar::Layer`, `Condition`, ...). A test keeps their field names equal to the
// stored types' (unit test `wire_forms_mirror_stored_types` below).

/// Wire form of [`Target`]: every id field also takes a selector.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TargetIn {
    /// View id or selector.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view: Option<ViewRef>,
    /// Panel id or selector (in `view`, or in the op's view).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub panel: Option<PanelRef>,
    /// Layer id or selector (in `view`, the op's view, or any view).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer: Option<LayerRef>,
    /// Series id or selector.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub series: Option<SeriesRef>,
    /// Output or field of the series, e.g. `"v"`, `"signal"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    /// Item id or selector (`{"finding":{...}}`, `{"fact":{...}}`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item: Option<ItemRef>,
    /// First bar index (0-based) of the region.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_bar: Option<u64>,
    /// Last bar index (inclusive) of the region.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_bar: Option<u64>,
}

/// Wire form of [`crate::grammar::Emphasis`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EmphasisIn {
    /// What is emphasised.
    pub target: TargetIn,
    /// How.
    pub kind: crate::grammar::EmphasisKind,
}

/// Wire form of [`NarrativeStep`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NarrativeStepIn {
    /// Start of the step, milliseconds from the start of the narration.
    pub at_ms: u32,
    /// What to show.
    pub target: TargetIn,
    /// What to say.
    pub say: String,
}

/// Wire form of [`AttentionDirective`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttentionDirectiveIn {
    /// Id; assigned if absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Principal who should look.
    pub to: String,
    /// Steps, in the order to show them.
    pub steps: Vec<AttentionStepIn>,
    /// Filled in by the authority; ignored if sent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    /// Filled in by the authority.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<TimestampMs>,
}

/// Wire form of [`AttentionStep`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttentionStepIn {
    /// What to look at.
    pub target: TargetIn,
    /// Optional short caption.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub say: Option<String>,
}

/// Wire form of [`Condition`]: series fields also take selectors.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ConditionIn {
    /// Last value of a series compared against a constant.
    Threshold {
        /// Series id or selector.
        series: SeriesRef,
        /// Comparison.
        op: crate::types::CompareOp,
        /// Constant to compare against.
        value: f64,
    },
    /// One series compared against another.
    Compare {
        /// Left series id or selector.
        left: SeriesRef,
        /// Comparison.
        op: crate::types::CompareOp,
        /// Right series id or selector.
        right: SeriesRef,
    },
    /// Free text; evaluated by a person.
    Text {
        /// Condition statement.
        text: String,
    },
}

/// Wire form of [`crate::grammar::DataRef`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DataRefIn {
    /// Series id or selector; the layer's own series when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub series: Option<SeriesRef>,
    /// Field name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
}

/// Wire form of [`crate::grammar::Position`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum PositionIn {
    /// Numeric constant.
    Number(f64),
    /// Field name, or a numeric constant written as a string.
    Name(String),
    /// Field of a (possibly other) series.
    Ref(DataRefIn),
}

/// Wire form of [`crate::grammar::Operand`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum OperandIn {
    /// Numeric constant.
    Number(f64),
    /// A field of the layer's data or of another series.
    Ref(DataRefIn),
}

/// Wire form of [`crate::grammar::Predicate`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PredicateIn {
    /// Left-hand field of the layer's data.
    pub field: String,
    /// Comparison.
    pub op: crate::grammar::Cmp,
    /// Right-hand side.
    pub than: OperandIn,
}

/// Wire form of [`crate::grammar::ColorWhen`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ColorWhenIn {
    /// The condition.
    pub when: PredicateIn,
    /// Colour where it holds.
    pub then: String,
    /// Colour where it does not.
    #[serde(rename = "else")]
    pub otherwise: String,
}

/// Wire form of [`crate::grammar::ColorSpec`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ColorSpecIn {
    /// One colour.
    Constant(String),
    /// Colour by condition.
    When(ColorWhenIn),
    /// Colour by value steps.
    Steps(crate::grammar::ColorSteps),
}

/// Wire form of [`crate::grammar::Encoding`]: series references inside channels also
/// take selectors.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EncodingIn {
    /// Horizontal position.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<PositionIn>,
    /// Vertical position.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<PositionIn>,
    /// Second vertical position.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y2: Option<PositionIn>,
    /// Colour.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<ColorSpecIn>,
    /// Opacity `0..=1`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opacity: Option<f64>,
    /// Stroke width in pixels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<f64>,
    /// Size in pixels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<crate::grammar::SizeSpec>,
    /// Text field of a `text` mark.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// Wire form of [`crate::grammar::Layer`]: `series`, `fact`, `finding`, `panel` and
/// series references in the encoding also take selectors.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LayerIn {
    /// Stable id, unique within the view. Assigned when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Visual mark.
    pub mark: crate::grammar::Mark,
    /// Data: a series id or selector.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub series: Option<SeriesRef>,
    /// Data: a fact id or selector.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fact: Option<FactRef>,
    /// Data: a finding id or selector.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finding: Option<FindingRef>,
    /// Panel id or selector (in the op's view).
    #[serde(default)]
    pub panel: PanelRef,
    /// Encoding channels.
    #[serde(default)]
    pub encoding: EncodingIn,
    /// Origins of the layer's numeric constants (JSON pointer inside the layer).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub origin: BTreeMap<String, crate::grammar::Origin>,
    /// Legend text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[cfg(test)]
mod selector_tests {
    use super::*;

    /// Property names of a type's generated schema (object variants merged).
    fn props<T: JsonSchema>() -> Vec<String> {
        fn collect(v: &serde_json::Value, out: &mut Vec<String>) {
            if let Some(p) = v.get("properties").and_then(serde_json::Value::as_object) {
                out.extend(p.keys().cloned());
            }
            for k in ["oneOf", "anyOf"] {
                if let Some(bs) = v.get(k).and_then(serde_json::Value::as_array) {
                    bs.iter().for_each(|b| collect(b, out));
                }
            }
        }
        let mut out = Vec::new();
        collect(&schemars::schema_for!(T).to_value(), &mut out);
        out.sort();
        out.dedup();
        out
    }

    #[test]
    fn wire_forms_mirror_stored_types() {
        use crate::grammar as g;
        assert_eq!(props::<TargetIn>(), props::<Target>());
        assert_eq!(props::<EmphasisIn>(), props::<g::Emphasis>());
        assert_eq!(props::<NarrativeStepIn>(), props::<NarrativeStep>());
        assert_eq!(
            props::<AttentionDirectiveIn>(),
            props::<AttentionDirective>()
        );
        assert_eq!(props::<AttentionStepIn>(), props::<AttentionStep>());
        assert_eq!(props::<ConditionIn>(), props::<Condition>());
        assert_eq!(props::<LayerIn>(), props::<g::Layer>());
        assert_eq!(props::<EncodingIn>(), props::<g::Encoding>());
        assert_eq!(props::<DataRefIn>(), props::<g::DataRef>());
        assert_eq!(props::<PredicateIn>(), props::<g::Predicate>());
        assert_eq!(props::<ColorWhenIn>(), props::<g::ColorWhen>());
    }

    #[test]
    fn id_fields_accept_selectors_in_the_schema() {
        let s = schemars::schema_for!(ClientMsg).to_value();
        let text = s.to_string();
        for def in [
            "SeriesSel",
            "ViewSel",
            "PanelSel",
            "LayerSel",
            "FactSel",
            "FindingSel",
            "ProposalSel",
            "ScaleSel",
            "ItemSel",
            "LayerIn",
            "TargetIn",
            "ConditionIn",
        ] {
            assert!(
                s.pointer(&format!("/$defs/{def}")).is_some(),
                "{def} missing from the ClientMsg schema"
            );
        }
        assert!(text.contains("\"deprecated\":true"));
        // Stored views keep plain ids. (The stored `Op`s of proposals and log requests
        // share the input schema; the authority only ever stores them resolved.)
        let d = schemars::schema_for!(View).to_value();
        assert!(d.pointer("/$defs/SeriesSel").is_none());
        assert!(d.pointer("/$defs/LayerIn").is_none());
        assert!(d.pointer("/$defs/TargetIn").is_none());
    }

    #[test]
    fn selectors_reject_unknown_keys() {
        let r: Result<SeriesSel, _> = serde_json::from_str(r#"{"stdy":"rsi"}"#);
        assert!(r.is_err());
        let r: Result<ItemSel, _> = serde_json::from_str(r#"{"finding":{"kind":"x"}}"#);
        assert!(r.is_ok());
    }
}
