//! The compact document summary (R2): enough to refer to items in the first
//! message.

use super::*;

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
