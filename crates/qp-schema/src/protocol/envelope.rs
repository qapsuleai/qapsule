//! The envelopes (`ClientMsg`, `ServerMsg`, ack), what a connection presents
//! (capability, identity) and errors: the outer shape of every `qp/0` message.

use super::*;

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
    /// (R5B, §25.5) Maximum active watches per person (sent with the agent policy).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_watches: Option<u32>,
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
    /// (R5B, §25) For an agent: the person it works for. The agent reads with that
    /// person's data entitlements (licence, audience), never more; when the agent has an
    /// entitlement row of its own, the intersection of the two. Absent: nobody.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_behalf: Option<String>,
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
    /// (R5B, §25.4) The comment validator rejected a comment draft; `reasons` says why.
    CommentRejected,
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
    /// (R5B, §25) A machine-readable sub-reason of the code, e.g. `human_only` or
    /// `agent_cannot_write` for `forbidden`. Absent on every error before R5B.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// (R5B, §25.4) For `comment_rejected`: every reason, machine-readable.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reasons: Vec<crate::comment::Reason>,
    /// (RL1, PS §3.1) For a program operation: every program error (the top-level fields
    /// carry the first one).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<ProgramError>,
}

/// (RL1) A position in program text (PR §11.1): 1-based line and column, column and
/// length in Unicode characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Span {
    /// Line, 1-based.
    pub line: u32,
    /// Column, 1-based.
    pub col: u32,
    /// Length.
    pub len: u32,
}

/// (RL1) One error or warning of a program (PR §11.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProgramError {
    /// The closed protocol code; absent on a warning.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<ErrorCode>,
    /// Sub-code (`E_...`; `W_...` for a warning).
    pub reason: String,
    /// What went wrong, in one sentence (`lang`).
    pub message: String,
    /// JSON pointer into the IR (relative to the program root).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,
    /// Position in the text, for text input.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span: Option<Span>,
    /// Accepted values or forms, when a short list.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub expected: Vec<String>,
    /// Close names (at most 5).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub did_you_mean: Vec<String>,
    /// How to fix it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}
