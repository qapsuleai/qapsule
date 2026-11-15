//! Visual grammar v0 (R2). Names follow `docs/CONCEPTS.md`; the design and its rejected
//! alternatives are in `docs/GRAMMAR.md`.
//!
//! A [`View`] is a stack of [`Panel`]s sharing one bar-ordered time axis. Each panel has
//! its own value [`Scale`]. A [`Layer`] is one [`Mark`] bound to data (a series, a fact or
//! a finding) through a closed, typed set of [`Encoding`] channels. Every numeric data
//! constant in a layer (a rule's level, a condition's threshold) must resolve to an
//! [`Origin`]: a fact, a named threshold or a series value. Layers, panels and scales
//! carry stable ids; they are addressed by id, never by position.
//!
//! Compatibility with `qp/0` (RD1): the wire shape of v0 views is a subset of this one.
//! `View::y` (per-panel scales by position) is the deprecated v0 form; the authority
//! moves it into `View::panels` when it loads a document. `Layer::panel` keeps its name
//! and number type, but it is now the panel's **id**, not its position.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Marks and scales
// ---------------------------------------------------------------------------

/// Visual mark of a layer.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Mark {
    /// Candlesticks; needs a bar series (`o`, `h`, `l`, `c`).
    Candle,
    /// Polyline through `y` (one line per default output when `y` is absent).
    Line,
    /// Fill from `y` down to `y2`, or to the bottom of the panel when `y2` is absent.
    Area,
    /// Vertical columns from `y2` (default 0, clamped to the panel) to `y`.
    Bar,
    /// Horizontal (`y`) or vertical (`x`) reference line across the panel.
    Rule,
    /// Points at (`x`, `y`).
    Point,
    /// Text label at (`x`, `y`); the text comes from the `text` channel.
    Text,
    /// Fill between two values `y` and `y2` (both required): bands, envelopes, fans.
    Band,
    /// Meaning glyph (R3, `docs/GRAMMAR.md` §18) of the capsule that presents the layer's
    /// `finding`; drawn by the product alphabet from the capsule's glyph parameters.
    Glyph,
}

/// Scale type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ScaleKind {
    /// Bar-ordered time (horizontal only): one slot per bar of the view's first bar
    /// series; session gaps (nights, weekends, holidays) take no space. `domain` is in
    /// epoch milliseconds.
    Time,
    /// Linear numeric.
    Linear,
    /// Base-10 logarithmic numeric; values `<= 0` are not drawn.
    Log,
}

/// A scale mapping data values to a visual axis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Scale {
    /// Scale type.
    pub kind: ScaleKind,
    /// Fixed domain `[min, max]`; automatic (fit to the visible data, 5 % padding) when
    /// absent. On the time axis this is the view's **persistent range** in epoch ms
    /// (`set_view_range`); a connection's transient viewport is presence, not this.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain: Option<[f64; 2]>,
    /// Stable id within the view: `"x"` for the time axis, `"y<panel id>"` for a panel's
    /// value scale. Filled by the authority; if given it must match.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

impl Scale {
    /// A scale of `kind` with an automatic domain and no id.
    #[must_use]
    pub fn new(kind: ScaleKind) -> Self {
        Self {
            kind,
            domain: None,
            id: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Panels
// ---------------------------------------------------------------------------

/// One horizontal strip of a view. Panels are stacked top to bottom in list order and
/// share the view's time axis; each has its own value scale.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Panel {
    /// Stable panel id (a number, unique within the view). It is **not** the position:
    /// moving or inserting panels never changes it. Layers refer to it (`Layer::panel`).
    /// Assigned (largest id + 1) when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<u32>,
    /// Optional name for people and for reference by meaning, e.g. `"price"`, `"rsi"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Relative height (> 0). Default: 3 for the first panel, 1 for the others.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,
    /// The panel's value scale (`linear` or `log`).
    pub y: Scale,
}

// ---------------------------------------------------------------------------
// Encoding channels (closed and typed)
// ---------------------------------------------------------------------------

/// A reference to a field of a series: `{"series": "sma50", "field": "sma"}`. Within a
/// layer, `series` may be omitted (the layer's own data); `field` may be omitted for the
/// series' main output (`c` for bars).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DataRef {
    /// Series id; the layer's own series when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub series: Option<String>,
    /// Field name: a bar field (`o`, `h`, `l`, `c`, `v`), a study output (`rsi`,
    /// `upper`, `histogram`...), `v` for a point series, or a fact/finding field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
}

/// A position channel (`x`, `x2`, `y`, `y2`).
///
/// - a string: a field of the layer's data (`"c"`, `"upper"`, `"level"`); a string that
///   parses as a number (`"70"`) is a numeric constant (the `qp/0` form);
/// - a number: a numeric constant (needs an [`Origin`]);
/// - an object: a field of another series ([`DataRef`]), e.g. the other edge of a band;
/// - `{"panel": f, "px": d}`: a panel-relative layout position ([`PanelPos`], R3; `text`
///   and `glyph` only).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Position {
    /// Numeric constant.
    Number(f64),
    /// Field name, or a numeric constant written as a string.
    Name(String),
    /// Field of a (possibly other) series.
    Ref(DataRef),
    /// Panel-relative layout position (not data; needs no origin).
    Panel(PanelPos),
}

/// A panel-relative layout position (R3, `docs/GRAMMAR.md` §18): `panel` is the fraction
/// of the panel's width (`x`) or height (`y`) from its left/top edge, `px` a pixel offset
/// added to it. A layout constant, like a size: it claims nothing about the data, so it
/// needs no [`Origin`]. Used by `text` and `glyph` (a capsule card's sentence, ledger line
/// and glyph).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PanelPos {
    /// Fraction `0..=1` of the panel's extent from its left (`x`) or top (`y`) edge.
    pub panel: f64,
    /// Pixel offset added to it (may be negative).
    #[serde(default, skip_serializing_if = "is_zero_px")]
    pub px: f64,
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_zero_px(v: &f64) -> bool {
    *v == 0.0
}

/// Comparison in a colour condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cmp {
    /// `<`
    Lt,
    /// `<=`
    Le,
    /// `>`
    Gt,
    /// `>=`
    Ge,
}

/// Right-hand side of a condition: a constant (needs an [`Origin`]) or a field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Operand {
    /// Numeric constant.
    Number(f64),
    /// A field of the layer's data or of another series.
    Ref(DataRef),
}

/// A per-bar condition: `field op than`, e.g. `{"field":"c","op":"ge","than":{"field":"o"}}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Predicate {
    /// Left-hand field of the layer's data.
    pub field: String,
    /// Comparison.
    pub op: Cmp,
    /// Right-hand side.
    pub than: Operand,
}

/// Colour chosen per bar by a condition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ColorWhen {
    /// The condition.
    pub when: Predicate,
    /// Colour where it holds.
    pub then: String,
    /// Colour where it does not (and where a side is `null`).
    #[serde(rename = "else")]
    pub otherwise: String,
}

/// Colour chosen per bar by the value of a field, in steps:
/// `colors[i]` for `thresholds[i-1] <= value < thresholds[i]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ColorSteps {
    /// Field of the layer's data.
    pub field: String,
    /// Ascending thresholds (each needs an [`Origin`]).
    pub thresholds: Vec<f64>,
    /// `thresholds.len() + 1` colours.
    pub colors: Vec<String>,
}

/// The colour channel: constant, conditional or bound to a value.
///
/// A colour is `#rrggbb`, `#rrggbbaa` or a theme token (`up`, `down`, `accent`, `text`,
/// `muted`, `warn`, `s1`…`s8`; see `docs/GRAMMAR.md` §Tema).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ColorSpec {
    /// One colour.
    Constant(String),
    /// Colour by condition.
    When(ColorWhen),
    /// Colour by value steps.
    Steps(ColorSteps),
}

/// Size bound to a field, mapped linearly onto `range` pixels over the visible values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SizeByField {
    /// Field of the layer's data.
    pub field: String,
    /// `[min, max]` size in pixels.
    pub range: [f64; 2],
}

/// Size channel (point diameter, text size), in pixels.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum SizeSpec {
    /// Constant size in pixels.
    Constant(f64),
    /// Size by value.
    ByField(SizeByField),
}

/// The closed set of encoding channels. Unknown channel names are rejected
/// (`schema`, with `did_you_mean`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Encoding {
    /// Horizontal position. Default: the row's bar. A constant makes a vertical rule.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<Position>,
    /// Second horizontal position (R3): the last bar of a bar extent. With `x`, a
    /// horizontal `rule` becomes a segment from bar `x` to bar `x2` (sloped when `y2` is
    /// also given), and a `band`/`area` covers only bars `x..=x2` (evidence geometry,
    /// `docs/GRAMMAR.md` §18).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x2: Option<Position>,
    /// Vertical position. Default: the main output of the layer's series (`c` for bars,
    /// `v` for points, `level` for facts); for `line` on a multi-output study, one line
    /// per default output.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<Position>,
    /// Second vertical position: the lower edge of `band`/`area`, the base of `bar`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y2: Option<Position>,
    /// Colour (stroke for lines and rules, fill for areas, bands, bars and points).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<ColorSpec>,
    /// Opacity `0..=1` (multiplies the colour's alpha).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opacity: Option<f64>,
    /// Stroke width in pixels (lines, rules, band edges).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<f64>,
    /// Size in pixels (point diameter, text size).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<SizeSpec>,
    /// Text of a `text` mark: a field of the layer's data (`text` of a fact, `claim` of a
    /// finding).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

impl Encoding {
    /// True when no channel is set.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

// ---------------------------------------------------------------------------
// Origins of constants
// ---------------------------------------------------------------------------

/// Which value of a series an [`Origin::Series`] takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SeriesAt {
    /// The last bar (follows live updates).
    Last,
}

/// Where a numeric constant of a layer comes from (principle 6). The authority rejects a
/// constant it cannot trace (`no_origin`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Origin {
    /// A field of a fact: an evidence key (`level`), or `bar` / `t` / `value`, or (R3) a
    /// bar of the fact's evidence geometry: `geometry.<i>.from_bar`,
    /// `geometry.<i>.to_bar`, `geometry.<i>.points.<j>.bar` (`docs/GRAMMAR.md` §18).
    Fact {
        /// Fact id.
        fact: String,
        /// Field name.
        field: String,
    },
    /// A named threshold of a study, e.g. `rsi.overbought` (70). The list is the
    /// engine's (`describe` → `thresholds`).
    Threshold {
        /// Threshold name `<study>.<name>`.
        name: String,
    },
    /// A value of a series (live: no constant is written, the value follows the data).
    Series {
        /// Series id.
        series: String,
        /// Field; main output when absent.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        field: Option<String>,
        /// Which bar.
        at: SeriesAt,
    },
}

// ---------------------------------------------------------------------------
// Layers and emphasis
// ---------------------------------------------------------------------------

/// One layer of a view: a mark bound to data through encoding channels.
///
/// Data is at most one of `series`, `fact`, `finding`. A layer with none of them draws
/// only constants, and every constant needs an origin.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Layer {
    /// Stable id, unique within the view. Assigned when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Visual mark.
    pub mark: Mark,
    /// Data: a series (bars, study, points).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub series: Option<String>,
    /// Data: one fact (one row: `bar`, `t`, `value`, `text` and its evidence keys).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fact: Option<String>,
    /// Data: the facts of a finding (one row per fact, plus `claim`, `confidence`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finding: Option<String>,
    /// Id of the panel the layer is drawn in (not a position).
    #[serde(default)]
    pub panel: u32,
    /// Encoding channels.
    #[serde(default)]
    pub encoding: Encoding,
    /// Origins of the layer's numeric constants, keyed by JSON pointer inside the layer
    /// (`"/encoding/y"`, `"/encoding/color/when/than"`,
    /// `"/encoding/color/thresholds/0"`). Filled by the authority when it can trace a
    /// constant (see `docs/GRAMMAR.md` §Köken); a constant may also be omitted when an
    /// origin is given, and the authority writes its value.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub origin: BTreeMap<String, Origin>,
    /// Legend text; derived from the data when absent (e.g. `SMA20`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

impl Layer {
    /// A layer of `mark` in panel `panel`, with no data, id or channels.
    #[must_use]
    pub fn new(mark: Mark, panel: u32) -> Self {
        Self {
            id: None,
            mark,
            series: None,
            fact: None,
            finding: None,
            panel,
            encoding: Encoding::default(),
            origin: BTreeMap::new(),
            label: None,
        }
    }
}

/// How an emphasis changes the drawing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EmphasisKind {
    /// Mark the target (accent band behind its bars, thicker stroke for its layer).
    Highlight,
    /// Dim everything except the target.
    Focus,
    /// Dim the target.
    Dim,
}

/// A persistent emphasis on part of a view (a finding, a layer, a bar range). Attention
/// directives and narration steps use the same target and are drawn the same way, but
/// they are presence, not document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Emphasis {
    /// What is emphasised. `item` (a fact or finding) resolves to its bars.
    pub target: crate::protocol::Target,
    /// How.
    pub kind: EmphasisKind,
}

// ---------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------

/// A presentation recipe written in the visual grammar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct View {
    /// Unique id within the document.
    pub id: String,
    /// Optional title; derived from the first bar series (symbol and interval) when
    /// absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Display time zone override (IANA); display only, see `Document::display_tz`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_tz: Option<String>,
    /// Theme name (`dark`, the default, or `light`). The theme lives in the engine, not
    /// in the view: the same view can be drawn with another theme.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
    /// Horizontal scale (always `time`).
    pub x: Scale,
    /// **Deprecated (`qp/0`).** Value scales by panel position. The authority converts
    /// it into `panels` (ids `0..n`) when loading; do not write both.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(extend("deprecated" = true))]
    pub y: Vec<Scale>,
    /// Panels, top to bottom.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub panels: Vec<Panel>,
    /// Layers, drawn in order (later layers on top).
    pub layers: Vec<Layer>,
    /// Persistent emphasis.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub emphasis: Vec<Emphasis>,
}

impl View {
    /// The panel with id `id`.
    #[must_use]
    pub fn panel(&self, id: u32) -> Option<&Panel> {
        self.panels.iter().find(|p| p.id == Some(id))
    }

    /// The layer with id `id`.
    #[must_use]
    pub fn layer(&self, id: &str) -> Option<&Layer> {
        self.layers.iter().find(|l| l.id.as_deref() == Some(id))
    }
}
