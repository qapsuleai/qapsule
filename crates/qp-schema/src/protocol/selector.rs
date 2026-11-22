//! Reference by meaning (R2, §17): selectors and id-or-selector references.

use super::*;

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
