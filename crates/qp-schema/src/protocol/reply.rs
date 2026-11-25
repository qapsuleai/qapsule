//! Query results (`QueryResult`) and the shapes they carry.

use super::*;

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
    /// (R5/m17-int, K49) A glossary term under the pixel (the scene's `terms`): a tap
    /// there opens its explanation instead of the capsule.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub term: Option<crate::scene::TermMark>,
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
        /// (R5/M19, K53) With `audience`: how many stories that pass every other filter of
        /// the query are kept out of the reader's feed by the host's coin-flip rule (absent:
        /// none). The empty feed says it (`docs/PROTOCOL.md` §23.2).
        #[serde(default, skip_serializing_if = "is_zero_u32")]
        coin_flip_hidden: u32,
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
    /// (R5B, §25.6) Answer to `preview`.
    Preview {
        /// Proposal id.
        proposal: String,
        /// The changes accepting it now would write.
        changes: Vec<ChangeOp>,
        /// The views it adds or changes, with their scene summary.
        views: Vec<PreviewView>,
    },
    /// (R5/M18, §23) Answer to `stories`.
    Stories {
        /// Instrument symbol.
        instrument: String,
        /// Bar series of the rows' bar indexes.
        series: String,
        /// The rows of this page, by birth (time, bar, id).
        stories: Vec<StoryRow>,
        /// Rows in the range (every page).
        total: u32,
        /// More rows follow: send this as `after` for the next page.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        next: Option<String>,
        /// For a reader without the licensed data entitlement: the server comments on
        /// that data in the range (instead of the licensed stories; K33, Ö37).
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        comments: Vec<ServerComment>,
    },
    /// (RL1, PS §3.2) Answer to `check_program` (fields flat).
    ProgramCheck(Box<crate::program::ProgramCheck>),
    /// (RL1, PS §3.2) Answer to `try_program` (fields flat).
    ProgramTry(Box<crate::program::ProgramTry>),
    /// (RL1, PS §3.2) Answer to `explain_program` (fields flat).
    ProgramExplain(Box<crate::program::ProgramExplain>),
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
