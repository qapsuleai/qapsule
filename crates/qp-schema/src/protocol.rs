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

use crate::capsule::{
    Audience, Direction, EvidenceFamily, GlyphSpec, LedgerStat, LicenseClass, MeaningKind,
    Sensitivity, Stage, Story, TimeKind, Transition, Trust, Validity,
};
use crate::data::{BindingState, DataBinding, DataEvent, DataPosition, EvidenceRef, FeedKey};
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
    /// (R3, §19.9) A replica disputes a claim the authority asserts as final: on its own
    /// clean, refetched data the claim's predicate does not hold at `t`. The authority
    /// re-checks the claim on its provider's settled bars; upheld → the claim is
    /// withdrawn (`claim` event, reason `dispute_upheld`; a capsule resting on it is
    /// retracted), rejected → its evidence goes back to the disputer. Answered with a
    /// `result` (`kind: "dispute"`). One dispute per (claim, `t`, `digest`).
    Dispute {
        /// Client-chosen request id.
        id: String,
        /// The claim (for a condition claim: the sender's own subscription).
        claim: ClaimRef,
        /// The claim's bar time.
        t: TimestampMs,
        /// The replica's own operand values at `t` (and the bar before).
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        values: Vec<EvidenceValue>,
        /// The anchor the replica computed from.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        anchor_t: Option<TimestampMs>,
        /// Digest of the replica's bars of the claim's bar series up to `t` (its data
        /// position; `data_digest`, 16 hex digits).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        digest: Option<String>,
    },
    /// (R4, §21) A control of the host's replay source: speed, pause, resume, step, seek,
    /// status. Answered by the replay host with a `result` (`kind: "replay"`); an
    /// authority without a replay source answers `unsupported`. Never written to the
    /// document or the log. `status` needs `read`; every other control the role `host` or
    /// `owner`.
    Replay {
        /// Client-chosen request id.
        id: String,
        /// What to do.
        control: ReplayControl,
    },
    /// (RP3/M15, K48, §22) What a viewer shows now: the capsules of its `viewer`
    /// subscription `sub` that are on screen (their faces come ready from the authority).
    /// The open capsule is the connection's view state (`input` `open`/`close`, §4.6).
    /// Replaces the previous visibility; presence, never written to the document or the
    /// log. Answered with a `result` (`kind: "viewer"`) that carries what changed.
    Visible {
        /// Client-chosen request id.
        id: String,
        /// The `viewer` subscription.
        sub: String,
        /// Capsule ids on screen (at most the subscription's `limit`).
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        visible: Vec<String>,
    },
}

/// (R4, §21) A replay control. A **step** is one distinct virtual time of the replay
/// (every feed's events at that time); step boundaries do not depend on the speed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ReplayControl {
    /// Where the replay is (no change).
    Status,
    /// Stop advancing with the wall clock (explicit `step` and `seek` still work).
    Pause,
    /// Advance with the wall clock again, at the current speed.
    Resume,
    /// Set the speed: steps per wall-clock second (`0 < steps_per_s <= 1000`).
    Speed {
        /// Steps per second.
        steps_per_s: f64,
    },
    /// Take `n` steps now (default 1, at most 100000); the notifications they cause
    /// follow the `result`.
    Step {
        /// Number of steps.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        n: Option<u32>,
    },
    /// Go to a position: exactly one of `t` (every step at or before `t` taken), `date`
    /// (`YYYY-MM-DD`: the start of that day in the exchange time zone, nothing of that
    /// day taken) or `step` (that many steps taken). Forward: the steps in between are
    /// taken and their notifications follow the `result`. Backward: the authority is
    /// rebuilt from its init and advanced silently to the position (a new authority,
    /// `epoch + 1`, §8.4): every connection is closed and must send `hello` again.
    Seek {
        /// Virtual time, epoch ms.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        t: Option<TimestampMs>,
        /// A date in the exchange time zone.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        date: Option<String>,
        /// A step count.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        step: Option<u64>,
    },
}

/// (R4, §21) Where the replay data comes from.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReplaySourceKind {
    /// The deterministic synthetic market (`qp_source::synthetic_market`).
    #[default]
    Synthetic,
    /// Real bars fetched to the host's machine (`fixtures/local/`, K23).
    Local,
}

/// (R4, §21) Replayed days by how a bar was built inside the day.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReplayDays {
    /// From finer bars (e.g. hourly bars of a daily feed).
    pub fine: u64,
    /// On the labelled synthetic path open → extreme → extreme → close (no fine data).
    pub ohlc: u64,
    /// As one whole bar.
    pub bar: u64,
}

/// (RP3/M15, K48, §22) The trust mark of a capsule on a viewer's screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TrustMark {
    /// The authority's settled claim, shown as "verified by the server": a feed capsule
    /// (the device did not check it; it holds none of its data).
    Server,
    /// Checked on the device's own data (the open capsule, after its check passed).
    Device,
}

/// (RP3/M15, §22) One feed row of a viewer: the capsule as listed for the reader.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ViewerItem {
    /// The capsule as the `capsules` query lists it for the reader.
    pub capsule: CapsuleItem,
    /// When the reader could first know its latest transition (the open time of the bar
    /// after the transition bar); absent: the transition is on the newest bar (now).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub known_at: Option<TimestampMs>,
    /// The authority sends a face scene for it when it is visible (the reader may see the
    /// series' data); else the device shows the sentence.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub face: bool,
}

/// (RP3/M15, §22) A ready face: the scene the authority compiled for the reader (its
/// face design, look, language and data), at the face frame.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ViewerFace {
    /// Capsule id.
    pub capsule: String,
    /// `view_digest` of the scene (§19.6).
    pub digest: String,
    /// The scene (`SceneDesc`).
    pub scene: Box<crate::scene::SceneDesc>,
    /// Always `server` from the authority.
    pub trust: TrustMark,
}

/// (RP3/M15, §22) The open capsule's slice: what the device needs to draw and check it
/// on its own data — the document's items about its instrument (its series and the
/// studies on them, their facts, findings, capsules, views and intents, their ledger
/// rows, the reader's audience) — and the feeds the host now serves this device.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ViewerOpen {
    /// Capsule id.
    pub capsule: String,
    /// Its finding.
    pub finding: String,
    /// Instrument symbol.
    pub instrument: String,
    /// Bar series.
    pub series: String,
    /// The slice (a document; `document_digest` in `digest`).
    pub document: Box<Document>,
    /// `document_digest` of the slice.
    pub digest: String,
    /// Bound feeds the slice rests on (the device's data subscriptions while it is open).
    pub feeds: Vec<FeedKey>,
}

/// (RP3/M15, §22) What changed for a viewer (`viewer` event and `visible` answer).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ViewerUpdate {
    /// The feed, top to bottom (when it changed).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feed: Option<Vec<ViewerItem>>,
    /// Faces of visible capsules that are new on screen or changed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub faces: Vec<ViewerFace>,
    /// The open capsule's slice (when it opened or its slice changed).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open: Option<ViewerOpen>,
    /// The open capsule was closed.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub closed: bool,
    /// The reader's audience (when it changed; `null` in JSON is not sent).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audience: Option<Audience>,
    /// The device's data subscriptions now: the feeds the host serves this connection
    /// (empty on the feed screen).
    pub data: Vec<FeedKey>,
}

impl ViewerUpdate {
    /// Nothing changed (only the data subscriptions, repeated).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.feed.is_none()
            && self.faces.is_empty()
            && self.open.is_none()
            && !self.closed
            && self.audience.is_none()
    }
}

/// (R4, §21) The state of a replay (answer to every `replay` control).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReplayState {
    /// Virtual time (the authority's clock): the time of the last step taken.
    pub t: TimestampMs,
    /// Steps taken.
    pub step: u64,
    /// Steps of the whole replay.
    pub steps: u64,
    /// Data events delivered so far.
    pub events: u64,
    /// Time of the first step.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_t: Option<TimestampMs>,
    /// Time of the last step.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_t: Option<TimestampMs>,
    /// Not advancing with the wall clock.
    pub paused: bool,
    /// Every step taken.
    pub done: bool,
    /// Steps per wall-clock second.
    pub speed: f64,
    /// True when the data is synthetic (a page must say so on screen).
    pub synthetic: bool,
    /// Where the data comes from.
    pub source: ReplaySourceKind,
    /// Why the source differs from the one asked for (e.g. no local data).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Number of instruments (feeds).
    pub instruments: u32,
    /// Bar interval of the feeds.
    pub interval: String,
    /// Interval of the finer bars that move a bar inside its day, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fine: Option<String>,
    /// Replayed days by how they were built.
    pub days: ReplayDays,
    /// Authority epoch (grows with every backward seek).
    pub epoch: u64,
    /// True when this control rebuilt the authority (a backward seek): every connection
    /// was closed.
    #[serde(default, skip_serializing_if = "is_false")]
    pub reset: bool,
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
    /// (R3, §20) Language of capsule sentences for this connection; Turkish if absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lang: Option<Language>,
}

/// Language of capsule sentences (R3, §20).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    /// Turkish (default).
    #[default]
    Tr,
    /// English.
    En,
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
    /// Maximum bars one data binding may ask for (`window.last`) and hold (RP2, §18).
    #[serde(default = "default_max_bind_bars")]
    pub max_bind_bars: u32,
}

fn default_max_bind_bars() -> u32 {
    100_000
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
    /// Split a multi-output `line` layer (e.g. one Bollinger line drawing `upper`,
    /// `middle`, `lower`) into one layer per drawn output, each with an explicit `y`, at
    /// the same place in the paint order, in the same panel, with the same encoding.
    /// `ack.created`: the new layer ids, in output order. Then each output can be styled
    /// alone (`set_encoding` with `layer: {"output": "upper"}`).
    SplitLayer {
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
        /// Channel names to remove (`x`, `x2`, `y`, `y2`, `color`, `opacity`,
        /// `width`, `size`, `text`).
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
    /// (R3, §20) Set a person's capsule preferences: user group, sensitivity, watch list
    /// and positions (replaces them as a whole). The speaker's own if `principal` is
    /// absent; another person's only by an `owner`. Any principal that may read may set
    /// its own. The system never writes it by itself (K31).
    SetAudience {
        /// Principal id; the speaker if absent.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        principal: Option<String>,
        /// The preferences.
        audience: Audience,
    },
    /// (R3, §20) Set only the sensitivity level of a person (the speaker's own if
    /// `principal` is absent); `null` returns to the group's default. A person without
    /// preferences gets the `habituated` group.
    SetSensitivity {
        /// Principal id; the speaker if absent.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        principal: Option<String>,
        /// The level; `null` = the group's default.
        sensitivity: Option<Sensitivity>,
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
    /// (R3) Set (or remove, `null`) a person's capsule preferences.
    PutAudience {
        /// Principal id.
        principal: String,
        /// The preferences; `null` removes them.
        audience: Option<Audience>,
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
        /// Finding id. If absent: the only live finding; with several live findings the
        /// authority does not pick one (`ambiguous`, every live finding as a candidate;
        /// K25). Also takes a selector (§17), e.g. `{"latest":true}`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(with = "Option<FindingRef>")]
        finding: Option<String>,
        /// Present for this surface kind instead of the asker's.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        surface: Option<SurfaceKind>,
    },
    /// (RP2, §18.3) Data bindings and their state for the asker: resolved or not, why
    /// not, the anchor (first bar time) and the data position. All bound series if
    /// `series` is absent.
    Binding {
        /// A bound series.
        /// Also takes a selector (§17).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(with = "Option<SeriesRef>")]
        series: Option<String>,
    },
    /// (RP2, §18.5) Re-checks the frozen evidence of a fact, or of every fact of a
    /// finding, against today's data: content digest of the referenced bar range and the
    /// fact's evidence values recomputed.
    VerifyEvidence {
        /// A fact or a finding.
        /// Also takes a selector (§17).
        #[schemars(with = "ItemRef")]
        item: String,
    },
    /// (R3, §20) The capsules (stories) the asker may see, most important first
    /// (importance descending, then latest transition, then id). Filters combine with
    /// AND. Sentences are written for the asker (its language and data entitlement).
    Capsules {
        /// Only this instrument (symbol).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        instrument: Option<String>,
        /// Only these stages.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        stages: Vec<Stage>,
        /// `true`: only open stories; `false`: only closed ones.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        open: Option<bool>,
        /// Only this meaning kind.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        meaning: Option<MeaningKind>,
        /// Only stories with a transition at or after this bar.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        since_bar: Option<u64>,
        /// Only stories with a transition at or after this time (epoch ms).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        since_t: Option<TimestampMs>,
        /// (R4) Only stories born at or after this bar (of their own bar series).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        born_since_bar: Option<u64>,
        /// (R4) Only stories born at or after this time (epoch ms; the birth bar's time).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        born_since_t: Option<TimestampMs>,
        /// (R4) Only the most recently born of the matching stories ("the latest
        /// capsule"): the greatest birth time, then the greatest birth bar, then the
        /// greatest id. Applied after every other filter, before `limit`.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        latest: bool,
        /// Apply the asker's audience: watch list (and positions, intents) and
        /// sensitivity, with the position/intent override (§9.8). Default false: every
        /// story the asker may see.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        audience: bool,
        /// Return at most this many.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        limit: Option<u32>,
    },
    /// (R3, §20) One capsule opened to its evidence (K34): story history with
    /// transitions and reasons, facts, sentences, ledger, glyph and, per fact, the
    /// evidence values if the asker is entitled, otherwise the entitlement it requires
    /// and what it rests on.
    Capsule {
        /// Capsule id, or its finding's id.
        /// (R4) Also takes a capsule selector (§17, [`CapsuleSel`]).
        #[schemars(with = "CapsuleRef")]
        capsule: String,
    },
    /// (R3, §20) The ledger: closed capsules and realized ones per ledger key
    /// (`kind/direction/detector@version`, R4), or summed per kind. Retracted capsules
    /// never count.
    /// The ledger is license-independent (K33): every principal that may read gets it.
    Ledger {
        /// Only this ledger key.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        key: Option<String>,
        /// Only this meaning kind.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        meaning: Option<MeaningKind>,
        /// Only this detector.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        detector: Option<String>,
        /// (R4, K42/A23) Only this direction.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        direction: Option<Direction>,
        /// Group rows per kind instead of per key (every direction summed; with
        /// `direction`, that direction only).
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        by_kind: bool,
    },
}

/// Both sentences of a capsule for one reader (R3, §20).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CapsuleSentence {
    /// The sentence in the reader's language.
    pub text: String,
    /// Turkish.
    pub tr: String,
    /// English.
    pub en: String,
}

/// One capsule as listed for a reader (R3, §20).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CapsuleItem {
    /// Capsule id.
    pub id: String,
    /// Finding id.
    pub finding: String,
    /// Instrument symbol.
    pub instrument: String,
    /// Bar series.
    pub series: String,
    /// Meaning kind.
    pub kind: MeaningKind,
    /// Direction.
    pub direction: Direction,
    /// Time kind.
    pub time: TimeKind,
    /// Story stage.
    pub stage: Stage,
    /// Capsule state.
    pub state: CapsuleState,
    /// Importance tier (1 most important).
    pub tier: u8,
    /// Importance score.
    pub importance: f64,
    /// Independent evidence families supporting it.
    pub confirmations: u8,
    /// Trust path.
    pub trust: Trust,
    /// Birth bar.
    pub born_bar: u64,
    /// Bar of the latest transition.
    pub last_bar: u64,
    /// Time of the latest transition.
    pub t: TimestampMs,
    /// Ledger of its key at birth (no look-ahead).
    pub ledger_at_birth: LedgerStat,
    /// The sentence as of the series' last closed bar, for this reader (K26).
    pub sentence: CapsuleSentence,
    /// (R4, K42/A28) Whether the claim still holds as of the same bar: `valid`,
    /// `no_longer` (the state condition broke, the story is still open) or `closed`.
    /// The sentence and the glyph say the same.
    pub validity: Validity,
    /// About the reader's position or active intent. (R4, K42/A22) Its **negative
    /// transitions** (`weakened`, `invalidated`, `retracted`) pass any sensitivity; its
    /// birth and other transitions go through the sensitivity filter like any story.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub relevant: bool,
    /// The sentence carries no digits for this reader (K33).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub redacted: bool,
}

/// One fact's evidence as a reader may open it (R3, §20; K34).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceAccess {
    /// Fact id.
    pub fact: String,
    /// Evidence family.
    pub family: EvidenceFamily,
    /// License class of the data.
    pub license: LicenseClass,
    /// The reader may see the values.
    pub open: bool,
    /// The evidence values (only when `open`).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub values: BTreeMap<String, f64>,
    /// The frozen evidence (only when `open`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_ref: Option<EvidenceRef>,
    /// The data entitlement the values require (when not `open`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requires: Option<String>,
    /// What the evidence rests on, in words (always).
    pub rests_on: String,
}

/// A capsule opened to its evidence (R3, §20).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CapsuleDetail {
    /// The capsule as listed.
    pub item: CapsuleItem,
    /// The story with every transition (bar, time, stage, reason, facts).
    pub story: Story,
    /// The facts as the reader may receive them.
    pub facts: Vec<Fact>,
    /// The ledger of its key now (closes before the series' last closed bar).
    pub ledger: LedgerStat,
    /// The glyph's parameters (engine `glyph_spec` at the last closed bar).
    pub glyph: GlyphSpec,
    /// Per fact: the evidence values or what they require (K34).
    pub evidence: Vec<EvidenceAccess>,
    /// (R4/S1, §20.2) Only when `capsule` was a selector, the reader may not read the
    /// licensed flow of the instrument and the authority already told this reader of
    /// licensed activity newer than the resolved capsule's latest transition: the
    /// entitlement those stories require and the server comments that told it (the same
    /// comments the `capsules` query gives). Absent otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub withheld: Option<Withheld>,
}

/// (R4/S1, §20.2) What a capsule selector may also have matched that the reader cannot
/// see: newer licensed stories, told only by the server comments the reader already gets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Withheld {
    /// Entitlements they require (the comments' `entitlement`, e.g. `flow/*`).
    pub requires: Vec<String>,
    /// The server comments newer than the resolved capsule's latest transition.
    pub comments: Vec<ServerComment>,
}

/// One ledger row (R3, §20).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LedgerRow {
    /// Ledger key (`kind/direction/detector@version`), or the kind when grouped by kind.
    pub key: String,
    /// Meaning kind.
    pub kind: MeaningKind,
    /// (R4, K42/A23) Direction (absent when grouped by kind without a direction filter).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direction: Option<Direction>,
    /// Detector (absent when grouped by kind).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detector: Option<String>,
    /// Closed capsules (realized + invalidated + expired; never retracted).
    pub n: u32,
    /// Of which realized.
    pub realized: u32,
}

/// A server-verified, digit-free comment on licensed data for a reader without the
/// entitlement (R3, §20; the chosen trust path mühür, M6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ServerComment {
    /// Instrument symbol.
    pub instrument: String,
    /// Bar series it belongs to.
    pub series: String,
    /// Bar index.
    pub bar: u64,
    /// Bar time.
    pub t: TimestampMs,
    /// Machine codes of what is said.
    pub codes: Vec<String>,
    /// Turkish.
    pub tr: String,
    /// English.
    pub en: String,
    /// Always `server` (K38).
    pub trust: Trust,
    /// What it requires and rests on, the rule and the receipt (no digits).
    pub attachments: BTreeMap<String, String>,
}

/// One data binding and its state for the asker (§18.3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BindingInfo {
    /// The bound series.
    pub series: String,
    /// Its binding.
    pub binding: DataBinding,
    /// State for the asker.
    pub state: BindingState,
    /// Why not ready, in words.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Time of bar 0 (the anchor of bar numbers), when ready.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor_t: Option<TimestampMs>,
    /// Where the authority stands on the feed, when ready.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<DataPosition>,
}

/// Outcome of re-checking one fact's frozen evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceStatus {
    /// Same bars (digest equal) and same evidence values.
    Verified,
    /// The data or the recomputed evidence values differ.
    Changed,
    /// The fact has no evidence reference (written before RP2, or on a points series).
    NoRef,
    /// The referenced data is not available now (binding not ready, range outside).
    Unavailable,
    /// The asker may not read the referenced feed.
    NoEntitlement,
}

/// An evidence value then and now.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ValueCheck {
    /// Value stored in the fact.
    pub then: f64,
    /// Value recomputed today; `null` if not computable.
    pub now: Option<f64>,
}

/// One fact's evidence check.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FactCheck {
    /// Fact id.
    pub fact: String,
    /// Outcome.
    pub status: EvidenceStatus,
    /// What happened, in words (first difference when changed).
    pub message: String,
    /// The stored reference.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stored: Option<EvidenceRef>,
    /// The same range over today's data (bars and digest), when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current: Option<EvidenceRef>,
    /// Evidence values (`value`, `prev`…) stored vs recomputed, for a fact on a study.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub values: BTreeMap<String, ValueCheck>,
}

/// Answer to `verify_evidence`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceCheck {
    /// The fact or finding checked.
    pub item: String,
    /// True iff every fact is `verified`.
    pub verified: bool,
    /// One check per fact, in the finding's fact order.
    pub facts: Vec<FactCheck>,
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
    /// (RP2) If the layer's data rests on a bound series that is not ready for the asker:
    /// that state; the layer is then not drawn (§18.3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub binding: Option<BindingState>,
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
    /// (R4, §7) The connection's view state, once a pixel gesture or open/close set it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view_state: Option<ViewState>,
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
    /// (R4, §4.6) Answer to a pixel gesture (`tap_at`, `drag`, `pinch`) or `open` /
    /// `close`.
    Interaction {
        /// What the gesture did.
        interaction: Box<Interaction>,
    },
    /// Answer to `binding` (RP2).
    Bindings {
        /// Bound series, document order.
        bindings: Vec<BindingInfo>,
    },
    /// Answer to `verify_evidence` (RP2).
    EvidenceCheck {
        /// The check.
        check: EvidenceCheck,
    },
    /// Answer to `capsules` (R3, §20).
    Capsules {
        /// The capsules, most important first.
        capsules: Vec<CapsuleItem>,
        /// For a reader without the licensed data entitlement: the server comments on
        /// that data (instead of the licensed stories).
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        comments: Vec<ServerComment>,
    },
    /// Answer to `capsule` (R3, §20).
    CapsuleDetail {
        /// The capsule opened to its evidence.
        detail: Box<CapsuleDetail>,
    },
    /// Answer to `ledger` (R3, §20).
    Ledger {
        /// Rows, by key (or kind).
        rows: Vec<LedgerRow>,
        /// Sum over the rows.
        total: LedgerStat,
    },
    /// Answer to a `dispute` message (R3, §19.9).
    Dispute {
        /// The verdict so far.
        verdict: DisputeVerdict,
        /// For `rejected`: the authority's evidence on settled data.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        evidence: Option<ClaimEvidence>,
    },
    /// Answer to a `replay` message (R4, §21).
    Replay {
        /// The replay's state after the control.
        replay: ReplayState,
    },
    /// (RP3/M15, §22) Answer to a `visible` message: what changed for the viewer.
    Viewer {
        /// The changes.
        viewer: Box<ViewerUpdate>,
    },
}

/// The verdict on a `dispute` (R3, §19.9).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum DisputeVerdict {
    /// The authority waits for its settled data (the verdict follows as a `claim` event:
    /// `withdrawn` with reason `dispute_upheld`, or `asserted` with reason
    /// `dispute_rejected` to the disputer).
    Pending,
    /// The claim does not hold on settled data: withdrawn.
    Upheld,
    /// The claim holds on settled data: it stands.
    Rejected,
    /// The same dispute (claim, `t`, digest) was already made: nothing new.
    Duplicate,
    /// The authority does not assert this claim (never, or no longer).
    Unknown,
    /// Too many open disputes on this connection.
    Limited,
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
    /// (RP2) Data changes a view draws: bars of a series it reads (directly, through a
    /// study's input, a fact, a channel reference or an origin) were added, revised or
    /// corrected, by a data event or a bar write, or its binding state changed. Fires
    /// only for views that read the changed series; structure changes of the view come
    /// through `changes`.
    View {
        /// View id.
        /// Also takes a selector (§17).
        #[schemars(with = "ViewRef")]
        view: String,
    },
    /// (RP2/T8) The authority's data-derived claims with their evidence (§19): every
    /// capsule invalidation and intent trigger of the document and this connection's own
    /// condition subscriptions, as they are asserted (provisional), become final at the
    /// bar's close, are withdrawn, or change cleanliness. The topic a replica listens to.
    Claims {},
    /// (R3, §20) Capsule story transitions for the subscriber: filtered by its audience
    /// (watch list, sensitivity; stories about its positions and active intents always
    /// pass) unless `all`; licensed stories reach a subscriber without the entitlement
    /// only as server comments (`trust: server`).
    Capsules {
        /// Only this instrument.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        instrument: Option<String>,
        /// Every story the subscriber may see, without the audience filter.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        all: bool,
    },
    /// (RP3/M15, K48, §22) The visibility subscription of a viewer: the authority
    /// pushes what the device shows — the feed (the reader's audience, at most `limit`),
    /// the ready faces of the capsules on screen (`visible`), the open capsule's slice
    /// and claims — and names the data subscriptions the host serves this connection
    /// (none on the feed screen; only the open instrument's feeds). The device runs no
    /// capsule engine for the feed. One per connection.
    Viewer {
        /// Feed length (default 40).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        limit: Option<u32>,
        /// Face frame width in CSS px (default 390); the height is the face design's.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        width: Option<u32>,
    },
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
        /// (RP2/T8) Evidence of the claim (§19), when data caused this entry.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        evidence: Option<ClaimEvidence>,
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
        /// (RP2/T8) Evidence of the claim (§19), when data caused the change.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        evidence: Option<ClaimEvidence>,
    },
    /// A watched condition became true at a bar.
    ConditionMet {
        /// Bar index.
        bar: u64,
        /// Bar time.
        t: TimestampMs,
        /// Log position.
        seq: Seq,
        /// (RP2/T8) Evidence of the claim (§19): the operands' values at `t` and the bar before, maturity, cleanliness.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        evidence: Option<ClaimEvidence>,
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
        /// (RP2/T8) Evidence of the claim (§19): the trigger's operand values, maturity, cleanliness.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        evidence: Option<ClaimEvidence>,
    },
    /// (RP2) What a watched view draws changed from bar `from_bar` on: redraw from there.
    /// One event per changed series the view reads.
    ViewChanged {
        /// View id.
        view: String,
        /// The series whose bars changed (a bar series: inline or bound).
        series: String,
        /// First bar index (of `series`) whose values may differ: the new bar, the revised
        /// last bar, the corrected bar (everything after it may change too), 0 after a
        /// binding state change.
        from_bar: u64,
        /// Last bar index of `series` now; absent when it holds no bars (e.g. stale).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        to_bar: Option<u64>,
        /// Log position the change is visible at (data events write no entry: the
        /// current position).
        seq: Seq,
    },
    /// (RP2/T8) A claim's lifecycle step (topic `claims`, §19): asserted (provisional at
    /// the live bar, final when the bar closed and the claim still holds there, again
    /// when a correction changed its values or its cleanliness changed) or withdrawn.
    Claim {
        /// What the claim is about.
        claim: ClaimRef,
        /// Asserted or withdrawn.
        state: ClaimState,
        /// The evidence (for a withdrawal: the values that no longer hold).
        evidence: ClaimEvidence,
        /// Log position.
        seq: Seq,
        /// (R3, §19.9) Why, when the step comes from settled data or a dispute.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reason: Option<ClaimReason>,
    },
    /// (R3, §20) A capsule story transition (topic `capsules`).
    Capsule {
        /// The capsule as listed for the subscriber after the transition.
        capsule: Box<CapsuleItem>,
        /// The transition.
        transition: Transition,
        /// Log position of the engine entry.
        seq: Seq,
    },
    /// (R3, §20) A server comment on licensed data (topic `capsules`, subscribers without
    /// the entitlement).
    ServerComment {
        /// The comment.
        comment: ServerComment,
        /// Log position it was made at.
        seq: Seq,
    },
    /// (RP3/M15, §22) What changed for a viewer (topic `viewer`).
    Viewer {
        /// The changes.
        viewer: Box<ViewerUpdate>,
        /// Log position it reflects.
        seq: Seq,
    },
}

/// Why a claim step happened (R3, §19.9; absent for the ordinary lifecycle of §19.3).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ClaimReason {
    /// The authority's data proved wrong against its provider's settled bars.
    DataCorrected,
    /// A replica's dispute was upheld on settled data.
    DisputeUpheld,
    /// A replica's dispute was rejected: the claim holds on settled data (sent to the
    /// disputer only, with the authority's evidence).
    DisputeRejected,
}

// ---------------------------------------------------------------------------
// Consistency (RP2/T8, `docs/PROTOCOL.md` §19)
// ---------------------------------------------------------------------------

/// Maturity of a claim's bar on one side's data.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Maturity {
    /// The bar is the live (last, not yet closed) bar.
    Provisional,
    /// The bar is closed: a later bar exists, or the window is closed (`to_t`).
    Final,
}

/// One operand value a claim rests on: the main output of a series the condition reads,
/// at the claim's bar and at the bar before it (`null` in warmup).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceValue {
    /// Series id (as in the condition).
    pub series: String,
    /// Bar time of `value` (the claim's bar on that series' bar series).
    pub t: TimestampMs,
    /// Value at `t` (absent in warmup, and for a recipient without the data entitlement
    /// of the feed it rests on: §18.3, §19.7).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
    /// Value at the bar before `t` (absent as `value`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prev: Option<f64>,
}

/// The evidence an authority attaches to a data-derived claim (§19): a few numbers, never
/// bars (K23).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ClaimEvidence {
    /// The bar series the claim rests on (of its first operand; bound or inline).
    pub series: String,
    /// The claim's bar time: the identity of the bar across the planes (§18.3).
    pub t: TimestampMs,
    /// Each operand's value at `t` and at the bar before, in condition order.
    pub values: Vec<EvidenceValue>,
    /// `final` if the bar is closed on the authority's data, else `provisional`.
    pub maturity: Maturity,
    /// The authority's data up to `t` has no unrepaired sequence gap.
    pub clean: bool,
    /// Calculation start: the bound series' anchor (bar 0 time, §18.3) the values were
    /// computed from; absent for inline series.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor_t: Option<TimestampMs>,
    /// The bound feeds the evidence rests on (its bar series and every operand, through
    /// study inputs), recorded when the claim is made; empty if it rests on inline data
    /// only. Values go only to recipients entitled to all of them (§18.3, §19.7).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub feeds: Vec<FeedKey>,
}

/// What a claim is about.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ClaimRef {
    /// A condition subscription of the receiving connection became true.
    Condition {
        /// The subscription id.
        sub: String,
    },
    /// An intent's trigger became true.
    Intent {
        /// Intent id.
        intent: String,
    },
    /// A live capsule's invalidation held (the capsule was invalidated).
    Capsule {
        /// Capsule id.
        capsule: String,
        /// Its finding.
        finding: String,
    },
}

/// The authority's latest word on a claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ClaimState {
    /// The authority asserts the claim (see the evidence's maturity and cleanliness).
    Asserted,
    /// The authority no longer asserts it (it did not hold at the bar's close, or a
    /// correction broke it).
    Withdrawn,
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
    /// (R4, §4.6) Tap at pixel (`x`, `y`) of a target as the surface draws it. The engine
    /// hit-tests the target's scene (the `hit` code) and decides what the tap means: a
    /// bar's values (`at`), or opening a capsule one depth further (§4.6 "Açılma").
    /// Answered with an `interaction`.
    TapAt {
        /// What was tapped; the connection's current scene if absent.
        #[serde(default, skip_serializing_if = "SurfaceTarget::is_current")]
        target: SurfaceTarget,
        /// Pixel x inside the target's frame (0 = left edge).
        x: f64,
        /// Pixel y inside the target's frame (0 = top edge).
        y: f64,
        /// Frame width the surface draws the target at; remembered per target (view
        /// state). Omit to use the remembered or the surface's default size.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        width: Option<u32>,
        /// Frame height, as `width`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        height: Option<u32>,
    },
    /// (R4, §4.6) Horizontal drag by `dx` pixels on a chart (positive: the finger moves
    /// right, older bars come into view). The engine turns pixels into bars with the
    /// scene's slot width. Answered with an `interaction`.
    Drag {
        /// The chart dragged; the connection's current scene if absent.
        #[serde(default, skip_serializing_if = "SurfaceTarget::is_current")]
        target: SurfaceTarget,
        /// Pixels moved (positive = right).
        dx: f64,
        /// Frame width (see `tap_at`).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        width: Option<u32>,
        /// Frame height (see `tap_at`).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        height: Option<u32>,
    },
    /// (R4, §4.6) Pinch on a chart by `factor` (> 1: the fingers move apart, fewer bars)
    /// around pixel column `x` (the bar under it stays in place; default: the last visible
    /// bar). Answered with an `interaction`.
    Pinch {
        /// The chart pinched; the connection's current scene if absent.
        #[serde(default, skip_serializing_if = "SurfaceTarget::is_current")]
        target: SurfaceTarget,
        /// Factor, > 0.
        factor: f64,
        /// Pixel x of the pinch centre.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        x: Option<f64>,
        /// Frame width (see `tap_at`).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        width: Option<u32>,
        /// Frame height (see `tap_at`).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        height: Option<u32>,
    },
    /// (R4, §4.6) Open a capsule one depth further (sentence → capsule → chart →
    /// evidence), or straight to `to`. Answered with an `interaction` carrying the
    /// `opening`: what to show at the new depth and the elements both depths share.
    Open {
        /// Capsule id (or the id of the finding it presents); (R4/S1) also a capsule
        /// selector (§17, [`CapsuleSel`]).
        #[schemars(with = "CapsuleRef")]
        capsule: String,
        /// Depth to open to; one deeper than now if absent.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        to: Option<OpenDepth>,
        /// Frame width the new depth will be drawn at (see `tap_at`).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        width: Option<u32>,
        /// Frame height, as `width`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        height: Option<u32>,
    },
    /// (R4/T5, §4.6.2) The connection's presentation choices (view state, presence):
    /// the capsule face design, the glyph alphabet and the theme the scenes of this
    /// connection are compiled with. Each field given replaces the remembered one (an
    /// absent field keeps it); unknown names are `invalid` with the known ones.
    /// Answered with an `interaction` (its `state`).
    Style {
        /// Face design name (`qp_grammar::faces::FACES`, `docs/PROTOCOL.md` §4.6.2; default `r3`,
        /// alias `card`).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        face: Option<String>,
        /// Glyph alphabet name (`siluet`, `rozet`; default `siluet`).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        alphabet: Option<String>,
        /// Theme (`light`, `dark`; `view`: the view's own, the default).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        theme: Option<String>,
        /// (R4/H-M8910) Feed layout name (`qp_grammar::feeds::LAYOUTS`; default `r3`, the
        /// `capsules` order).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        feed: Option<String>,
        /// (R4/H-M8910) Depth transition name (`qp_grammar::transitions::TRANSITIONS`;
        /// default `r3`).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        transition: Option<String>,
        /// (R4/H-M8910) Outcome mark style of glyphs (`qp_grammar::glyph::outcome::STYLES`;
        /// default `r3`).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        outcome: Option<String>,
        /// (R4/H-M8910) The reader's last look ("son bakış"): a time (ms) the feed layout
        /// may mark what changed since; `0` clears it.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        seen: Option<TimestampMs>,
        /// (R5/T3) Look (design record: type, colour roles, spacing, surface, glyph,
        /// motion; `qp_grammar::look::LOOKS`; default `r4`).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        look: Option<String>,
    },
    /// (R4, §4.6) Close the open capsule one depth back, or to `to` (`sentence`: back to
    /// the feed). Answered with an `interaction` carrying the `opening` (the way back).
    Close {
        /// Depth to go back to; one shallower than now if absent.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        to: Option<OpenDepth>,
        /// Frame width the depth closed to is drawn at (see `tap_at`).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        width: Option<u32>,
        /// Frame height, as `width`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        height: Option<u32>,
    },
}

/// (R4, §4.6) What a pixel gesture lands on: a view of the document (a chart) or a
/// capsule (its face in the feed, or the open capsule at its depth). At most one field;
/// neither means the connection's current scene (the open capsule if there is one,
/// otherwise its current view).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SurfaceTarget {
    /// A view id of the document.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view: Option<String>,
    /// A capsule id (or the id of the finding it presents).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capsule: Option<String>,
}

impl SurfaceTarget {
    /// No field set: the connection's current scene.
    #[must_use]
    pub fn is_current(&self) -> bool {
        self.view.is_none() && self.capsule.is_none()
    }
}

/// (R4, §4.6) How far a capsule is opened on one connection: `sentence` (nothing open:
/// the feed of faces or sentences), `capsule` (the capsule opened), `chart` (its marked
/// chart), `evidence` (the chart with the evidence opened).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum OpenDepth {
    /// Nothing open.
    #[default]
    Sentence,
    /// The capsule opened.
    Capsule,
    /// Its marked chart.
    Chart,
    /// The chart with its evidence opened.
    Evidence,
}

/// (R4, §4.6, §7) A connection's view state (presence; never logged, never in the
/// document): how far which capsule is open, the chart range shown at chart depth and the
/// frame sizes the surface reported per target.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ViewState {
    /// Depth of the open capsule (`sentence`: none open).
    pub depth: OpenDepth,
    /// The open capsule.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capsule: Option<String>,
    /// The finding it presents.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finding: Option<String>,
    /// Id of the view shown at this depth (`capsule-<finding>` at capsule depth; the
    /// capsule's stored view or `present-<finding>` at chart and evidence depth).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view: Option<String>,
    /// Visible range of that chart (chart and evidence depth).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range: Option<Viewport>,
    /// Frame sizes `[width, height]` reported by the surface, by frame key: `view:<id>`,
    /// `face` (a capsule face in the feed), `capsule`, `chart`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub frames: BTreeMap<String, [u32; 2]>,
    /// (R4/T5) Face design the connection's capsule faces are built with (absent: the
    /// default, `r3`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub face: Option<String>,
    /// (R4/T5) Glyph alphabet its scenes draw with (absent: the product's, `siluet`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alphabet: Option<String>,
    /// (R4/T5) Theme its scenes are compiled with (absent: the view's own).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
    /// (R4/H-M8910) Feed layout (absent: the default, `r3`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feed: Option<String>,
    /// (R4/H-M8910) Depth transition (absent: the default, `r3`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transition: Option<String>,
    /// (R4/H-M8910) Outcome mark style (absent: the default, `r3`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<String>,
    /// (R4/H-M8910) The reader's last look (ms; absent: none).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seen: Option<TimestampMs>,
    /// (R5/T3) Look its scenes are compiled with (absent: the default, `r4`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub look: Option<String>,
}

/// (R4, §4.6) One element both sides of a depth transition show (e.g. the glyph, the
/// headline, the marked bar), with its box in each scene: the shell animates between the
/// two boxes. A side without a scene (the sentence depth) has no box.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SharedElement {
    /// Element id: `glyph`, `headline`, `ledger`, `price`, `bar:<i>`, `fact:<id>`,
    /// `evidence:<fact>` (`docs/GRAMMAR.md` §18.6).
    pub id: String,
    /// Box in the scene before the transition.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before: Option<crate::scene::PixelRect>,
    /// Box in the scene after it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<crate::scene::PixelRect>,
}

/// (R4, §4.6) A depth transition of a capsule (open or close).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Opening {
    /// Capsule id.
    pub capsule: String,
    /// The finding it presents.
    pub finding: String,
    /// Depth before.
    pub from: OpenDepth,
    /// Depth after.
    pub to: OpenDepth,
    /// What to show at `to`: the presenter's output at that depth (`text`; the card at
    /// capsule depth; the view and facts at chart and evidence depth).
    pub presentation: Presentation,
    /// Frame `[width, height]` the new depth's scene was laid out at, if it has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame: Option<[u32; 2]>,
    /// Elements both depths show, by id, with their boxes in each scene.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shared: Vec<SharedElement>,
    /// At evidence depth: per fact, the values or the entitlement they require (the
    /// `capsule` query's `evidence`, K34).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<EvidenceAccess>,
}

/// (R4, §4.6) The answer to a pixel gesture or an open/close.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Interaction {
    /// What was under the finger (`tap_at`), from the same code as the `hit` query.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hit: Option<HitInfo>,
    /// The bar tapped on a chart, with its values (as for `tap`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<AtBar>,
    /// The new visible range (`drag`, `pinch`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub viewport: Option<Viewport>,
    /// The depth transition, if the gesture opened or closed a capsule.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opening: Option<Opening>,
    /// The connection's view state after the gesture.
    pub state: ViewState,
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
    /// (RP2, §18) The asker has no data entitlement for a bound series' feed (data
    /// licence; not a role permission). `path` points at the series reference.
    NoEntitlement,
    /// (RP2, §18) A bound series has no data: the source does not know the binding, its
    /// window is empty, or resolution is still pending.
    NoData,
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
    /// (RP2) For a bound series: its binding state for the asker.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub binding: Option<BindingState>,
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
    /// (RP2) The host's data plane for bound series: feeds and entitlements, served by
    /// a test source provider (`docs/PROTOCOL.md` §18.6).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<InitData>,
    /// (R3, §20) The capsule engine the authority runs: on which bar series, with which
    /// licensed flow. Absent: no engine (documents and transcripts before R3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capsules: Option<CapsuleInit>,
    /// (R4, §21) A replay source: the authority runs under a replay host that serves
    /// `init.data` from the replay's initial bars, gives the clock and the data events,
    /// and answers `replay` messages. Absent: no replay (`replay` → `unsupported`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replay: Option<ReplayInit>,
}

/// The replay source of a session (R4, §21; host configuration, not a protocol
/// message). Paths never come from here: local data is read from the host's own
/// directory (`qp_source::local_bars_dir`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReplayInit {
    /// `synthetic` (default) or `local`; `local` without data falls back to the
    /// synthetic market and says so (`ReplayState::note`).
    #[serde(default)]
    pub source: ReplaySourceKind,
    /// Number of instruments (default 30).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instruments: Option<u32>,
    /// Synthetic market: trading days (default 260).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub days: Option<u32>,
    /// Synthetic market: seed (default 1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
    /// Feed interval (default `1d`; the synthetic market has `1d` only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interval: Option<String>,
    /// Finer interval that moves a bar inside its day (e.g. `1h`); absent: none (the
    /// labelled OHLC path, or whole bars with `path: false`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fine: Option<String>,
    /// Bars of each feed resolved before the first replayed bar (default 120).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub history: Option<u32>,
    /// First replayed bar: the first with `t >= from_t` (overrides `history`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_t: Option<TimestampMs>,
    /// Last replayed bar: `t <= to_t`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_t: Option<TimestampMs>,
    /// Without fine data: the OHLC path (default true) or one whole bar per day.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<bool>,
    /// Named feed of the replay's feed keys (default: none, the host's default feed).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feed: Option<String>,
    /// Initial speed, steps per wall-clock second (default 4).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speed: Option<f64>,
    /// Start running with the wall clock (default false: paused).
    #[serde(default, skip_serializing_if = "is_false")]
    pub running: bool,
    /// Principal id -> data entitlements on the replay's feeds (as `InitData`);
    /// empty: every principal is entitled to every feed (`*`).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub entitlements: BTreeMap<String, Vec<String>>,
    /// Add a bound series (id: the symbol in lower case, window: the history) for every
    /// replay feed no series of the document binds.
    #[serde(default, skip_serializing_if = "is_false")]
    pub bind_all: bool,
    /// Run the capsule engine (§20) on every bound series of a replay feed (added to
    /// `capsules.series`).
    #[serde(default, skip_serializing_if = "is_false")]
    pub capsules_all: bool,
}

/// The authority's capsule engine (R3, §20; host configuration).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CapsuleInit {
    /// Bar series (bound or inline, with an instrument symbol) the detectors and the
    /// capsule engine run on, at every bar that becomes final.
    pub series: Vec<String>,
    /// Licensed flow feeds the authority alone reads (server facts).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub flow: Vec<FlowInit>,
}

/// A licensed flow feed of the test host (R3, §20): `qp_source::synthetic_flow` over the
/// bars of a watched series.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FlowInit {
    /// The watched bar series the flow follows (one record per bar).
    pub series: String,
    /// Seed of the synthetic flow.
    pub seed: u64,
    /// Feed name; entitlement `feed/SYMBOL` (default `flow`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feed: Option<String>,
}

/// Test data plane of a session init (RP2): the feeds a test source provider serves and
/// who is entitled to which symbols.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InitData {
    /// Feeds, each with its initial bars.
    #[serde(default)]
    pub feeds: Vec<InitFeed>,
    /// Principal id -> entitlements: `"SYMBOL"` (that symbol on every feed),
    /// `"feed/SYMBOL"` (that symbol on that named feed only), `"feed/*"`, or `"*"` (all).
    /// Principals not listed have no entitlement. The authority's own engine work uses the
    /// entitlement of the first principal whose role is `host` (§18.3).
    #[serde(default)]
    pub entitlements: BTreeMap<String, Vec<String>>,
    /// Feeds that answer `pending` (never resolved) instead of data, for tests.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pending: Vec<FeedKey>,
    /// (R3, §19.9) The test provider's settled source: answers the authority's settled
    /// requests synchronously with the provider's bars, these bars replacing theirs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settled: Option<InitSettled>,
}

/// The test settled source of a session init (R3, §19.9).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InitSettled {
    /// Per feed, settled bars that differ from what the feed delivered (by `t`).
    #[serde(default)]
    pub feeds: Vec<InitSettledFeed>,
    /// Verify every closed bar of a bound series against the settled source (the
    /// authority's settled-bar hygiene); off: only disputes ask for settled bars.
    #[serde(default)]
    pub verify_closed: bool,
}

/// Settled bars of one feed (R3, §19.9).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InitSettledFeed {
    /// The feed.
    pub feed: FeedKey,
    /// The settled bars (replacing the provider's bars with the same `t`).
    pub bars: Vec<Bar>,
}

/// One test feed: give exactly one of `bars`, `path`, `synthetic`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InitFeed {
    /// The feed.
    pub feed: FeedKey,
    /// Bars inline.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bars: Option<Vec<Bar>>,
    /// A bar series JSON file (a `Series` with bars), relative to the repository root and
    /// confined to it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Deterministic synthetic bars (`qp_source::synthetic_bars(n, seed)`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub synthetic: Option<SyntheticFeed>,
}

/// Parameters of a synthetic test feed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SyntheticFeed {
    /// Number of bars.
    pub n: u32,
    /// Seed.
    pub seed: u64,
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
    /// (RP2) A data event the host delivers instead (data plane, not a protocol message;
    /// §18.4). Outputs are the notifications it causes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<DataEvent>,
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
    /// (R4) Its capsule's story is about this instrument (symbol).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instrument: Option<String>,
    /// (R4) Its capsule's story has this meaning kind (`breakout`; `kind` is a fact kind
    /// such as `breakout.range.up`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meaning: Option<MeaningKind>,
    /// (R4) Its capsule's story is at this stage.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage: Option<Stage>,
    /// (R4) Its capsule's story is open (`true`) or closed (`false`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open: Option<bool>,
    /// Pick the first match (by its latest fact bar: the oldest).
    #[serde(default, skip_serializing_if = "is_false")]
    pub first: bool,
    /// Pick the last match (by its latest fact bar: the most recent).
    #[serde(default, skip_serializing_if = "is_false")]
    pub latest: bool,
}

/// (R4) Selects a capsule with a story, e.g.
/// `{"instrument":"THYAO","meaning":"breakout","open":true}` ("THYAO's open breakout
/// capsule"). Order: the latest transition's bar, then the birth bar, then document
/// order; so `latest` is the one that moved last.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CapsuleSel {
    /// Instrument symbol.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instrument: Option<String>,
    /// Meaning kind.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meaning: Option<MeaningKind>,
    /// Direction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direction: Option<Direction>,
    /// Story stage.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage: Option<Stage>,
    /// Open (`true`) or closed (`false`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open: Option<bool>,
    /// Birth bar.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub born_bar: Option<u64>,
    /// Bar series of the story.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub series: Option<String>,
    /// Pick the first match (the one that moved earliest).
    #[serde(default, skip_serializing_if = "is_false")]
    pub first: bool,
    /// Pick the last match (the one that moved last).
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

/// (R4) A capsule id (or its finding's id) or a [`CapsuleSel`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum CapsuleRef {
    /// Capsule id or finding id.
    Id(String),
    /// Selector.
    Select(CapsuleSel),
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
    /// Panel-relative layout position (R3).
    Panel(crate::grammar::PanelPos),
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
    /// Second horizontal position (R3: last bar of an extent).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x2: Option<PositionIn>,
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
