//! Operations, the command side: `Op` and the shapes it carries (show, target,
//! narration).

use super::*;

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
    /// (R5B, §25.4) A comment: a short narrative about a capsule or an instrument that
    /// cites the facts, findings or capsules it rests on. The authority validates it
    /// deterministically against the evidence the writer may read (citations, numbers,
    /// advice language, licence) before it enters the document; a rejected draft →
    /// `comment_rejected` with `reasons`. Applied directly (never a proposal); needs the
    /// `propose` or `write` permission.
    AddComment {
        /// Id; assigned if absent (`cmt-<seq>-<n>`).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        /// What it is about.
        about: CommentAbout,
        /// Language of the text.
        lang: Language,
        /// The text (plain; `[n]` marks the n-th entry of `cites`).
        text: String,
        /// Ids of the facts, findings or capsules it rests on (at least one).
        cites: Vec<String>,
        /// External sources quoted (named, with the numbers they give).
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        external: Vec<crate::comment::ExternalQuote>,
    },
    /// (R5B, §25.5) Leave a watch: the engine matches story transitions against it and
    /// sends `intent_matched` on the `intents` topic. For the speaker (an agent: the
    /// person it works for) unless `for` names another person (an owner only). At most
    /// `limits.max_watches` (32) active watches per person. The authority assigns the id
    /// (`watch-<seq>-<n>`; ids starting with `watch-` are reserved).
    AddWatch {
        /// The person it is for; default: the speaker's person.
        #[serde(default, rename = "for", skip_serializing_if = "Option::is_none")]
        for_: Option<String>,
        /// The request in words.
        text: String,
        /// What matches.
        #[serde(rename = "match")]
        match_: crate::types::WatchMatch,
        /// Expiry time (epoch ms).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        expires_t: Option<TimestampMs>,
        /// Matches after which it ends.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max_matches: Option<u32>,
    },
    /// (R5B, §25.5) Remove a watch (state `removed`).
    RemoveWatch {
        /// Intent id of the watch.
        intent: String,
    },
}
