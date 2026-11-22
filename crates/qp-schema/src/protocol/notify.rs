//! Subscriptions and notifications (topics, events) and the consistency claims
//! they carry (RP2/T8, §19).

use super::*;

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
    /// (R5/M18, §23) Changes of one instrument's story list: a `story` event with the
    /// row whenever a story is born, moves, closes, or its validity or feed place for
    /// the subscriber changes; server comments on that instrument for a subscriber
    /// without the licensed data entitlement. The state at subscription is the `stories`
    /// answer (no event for it).
    Stories {
        /// Instrument symbol.
        instrument: String,
        /// Its bar series (needed only when the instrument has more than one).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        series: Option<String>,
    },
    /// (R5B, §25.4) New comments the subscriber may receive (optionally about one
    /// capsule or one instrument): a `comment` event each.
    Comments {
        /// Only comments about this capsule (or finding) id.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        capsule: Option<String>,
        /// Only comments about this instrument (or a capsule of it).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        instrument: Option<String>,
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
    /// (R5/M18, §23) A row of the story list changed or is new (topic `stories`).
    Story {
        /// The row as the subscriber may receive it now.
        story: Box<StoryRow>,
        /// Log position it reflects.
        seq: Seq,
    },
    /// (R5B, §25.5) A watch matched a story transition (topic `intents`), as the
    /// subscriber may receive it.
    IntentMatched {
        /// Intent id.
        intent: String,
        /// The capsule whose story moved.
        capsule: String,
        /// The stage it moved to.
        stage: crate::capsule::Stage,
        /// Bar of the transition.
        bar: u64,
        /// Time of the transition.
        t: TimestampMs,
        /// Log position of the engine entry.
        seq: Seq,
    },
    /// (R5B, §25.4) A validated comment (topic `comments`).
    Comment {
        /// The comment.
        comment: Box<Comment>,
        /// Log position of the entry that stored it (the subscriber's world).
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
