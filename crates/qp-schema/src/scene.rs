//! Scene description (R2): the renderer-independent intermediate form that the grammar
//! compiler (`qp-grammar`) produces from a view and its data.
//!
//! It is the **single source of truth** for everything drawn: the reference renderer
//! (`qp-render`) rasterizes its primitives and nothing else, and semantic reading
//! (`screen`, the point query `hit`) reads its panels, layers and items, so it never
//! needs a renderer. Layout is done in `f64` with IEEE basic operations only (see
//! `docs/GRAMMAR.md` §Belirleyicilik); pixel geometry is final here.
//!
//! Coordinates: pixels, origin top-left, `x` right, `y` down.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::grammar::{EmphasisKind, Mark, Origin, ScaleKind};
use crate::TimestampMs;

/// Straight (non-premultiplied) RGBA8 colour.
pub type Rgba = [u8; 4];

/// Horizontal text alignment relative to the anchor `x`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Align {
    /// Text starts at `x`.
    Left,
    /// Text ends at `x`.
    Right,
}

/// One drawing primitive.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "prim", rename_all = "snake_case", deny_unknown_fields)]
pub enum Prim {
    /// Pixel-aligned filled rectangle, drawn without anti-aliasing.
    Rect {
        /// Left.
        x: i32,
        /// Top.
        y: i32,
        /// Width.
        w: u32,
        /// Height.
        h: u32,
        /// Colour.
        color: Rgba,
    },
    /// Anti-aliased polyline stroke.
    Polyline {
        /// Points in pixels.
        points: Vec<[f32; 2]>,
        /// Stroke width in pixels.
        width: f32,
        /// Colour.
        color: Rgba,
    },
    /// Anti-aliased filled polygon (non-zero winding), e.g. an area or band segment.
    Fill {
        /// Closed outline in pixels (the last point connects to the first).
        points: Vec<[f32; 2]>,
        /// Colour.
        color: Rgba,
    },
    /// Anti-aliased filled circle.
    Circle {
        /// Centre x.
        cx: f32,
        /// Centre y.
        cy: f32,
        /// Radius.
        r: f32,
        /// Colour.
        color: Rgba,
    },
    /// One line of text. `x` is the anchor, `baseline` the baseline y, `px` the size.
    Text {
        /// Anchor x.
        x: i32,
        /// Baseline y.
        baseline: i32,
        /// Font size in pixels.
        px: f32,
        /// The text.
        text: String,
        /// Colour.
        color: Rgba,
        /// Alignment.
        align: Align,
        /// (R5/T3) Font asset the text is drawn with (`qp_grammar::look::FONTS`, the
        /// renderers' embedded files); absent: the default asset (`plex-mono`, IBM Plex Mono
        /// Regular). Set only when a look's type role uses another asset, so `r4` scenes
        /// are unchanged.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        font: Option<String>,
        /// (R5/T3) The look's type role the text is drawn with (`headline`, `label`,
        /// `figure`...), written only when the compile asks for it
        /// (`CompileOptions::roles`, the measurement harness); absent otherwise.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        role: Option<String>,
    },
}

/// A pixel rectangle; `right` and `bottom` are exclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PixelRect {
    /// Left edge.
    pub left: i32,
    /// Top edge.
    pub top: i32,
    /// Right edge (exclusive).
    pub right: i32,
    /// Bottom edge (exclusive).
    pub bottom: i32,
}

/// The horizontal mapping shared by all panels: bar slot `i` is centred at
/// `floor(left + (i - from_bar + 0.5) * slot)`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct XMap {
    /// First visible slot (bar index; may be negative-free only, so `u64`).
    pub from_bar: u64,
    /// Last visible slot, inclusive. May lie beyond the last bar (future points).
    pub to_bar: u64,
    /// Left edge of the plot in pixels.
    pub left: f64,
    /// Slot width in pixels.
    pub slot: f64,
}

/// A panel's vertical mapping. Linear: `y = top + (max - v) / (max - min) * (bottom -
/// top)`. Log: the same on `log10(v)`, computed with the deterministic logarithm.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct YMap {
    /// Scale kind (`linear` or `log`).
    pub kind: ScaleKind,
    /// Domain minimum (data units).
    pub min: f64,
    /// Domain maximum (data units).
    pub max: f64,
    /// Pixel y of `max`.
    pub top: f64,
    /// Pixel y of `min`.
    pub bottom: f64,
}

/// Where a layer's data comes from, as resolved by the compiler.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DataSource {
    /// A series.
    Series {
        /// Series id.
        series: String,
    },
    /// A fact.
    Fact {
        /// Fact id.
        fact: String,
    },
    /// The facts of a finding.
    Finding {
        /// Finding id.
        finding: String,
    },
    /// Constants only (each traced by an origin).
    Constant {},
}

/// One data-bound element of a layer (a candle, a line vertex, a bar, a point, a rule):
/// what the point query returns and what the data → pixel → data round trip checks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Item {
    /// Bar index (slot) of the element; absent for a horizontal rule.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bar: Option<u64>,
    /// Time of the bar (or point), epoch ms; absent for a horizontal rule.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub t: Option<TimestampMs>,
    /// Data values drawn by this element, by field (`o`,`h`,`l`,`c` for a candle; the
    /// `y` field for a line vertex; `y` and `y2` fields for a band slice...).
    pub values: BTreeMap<String, f64>,
    /// Pixel hit box (inclusive-exclusive like [`PixelRect`]).
    pub hit: PixelRect,
    /// For an item drawn from several values of an aggregated slot (LOD), how many bars
    /// it stands for; 1 when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bars: Option<u64>,
}

/// A compiled layer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SceneLayer {
    /// Layer id.
    pub id: String,
    /// Mark.
    pub mark: Mark,
    /// Data source.
    pub data: DataSource,
    /// Fields drawn, in channel order (`y` first), e.g. `["upper","lower"]`.
    pub fields: Vec<String>,
    /// Resolved constants: JSON pointer inside the layer → (origin, value).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub constants: BTreeMap<String, ResolvedConstant>,
    /// Legend text, if the layer has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Data-bound elements, in bar order.
    pub items: Vec<Item>,
    /// Primitives that draw this layer, in paint order.
    pub prims: Vec<Prim>,
    /// Values not drawn (e.g. `<= 0` on a log scale, `null` warmup), counted.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub skipped: u64,
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_zero(v: &u64) -> bool {
    *v == 0
}

/// A constant with the origin that justifies it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResolvedConstant {
    /// Where it comes from.
    pub origin: Origin,
    /// Its value.
    pub value: f64,
}

/// A compiled panel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ScenePanel {
    /// Panel id.
    pub id: u32,
    /// Panel name, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Plot rectangle.
    pub rect: PixelRect,
    /// Vertical mapping.
    pub y: YMap,
    /// Layers in paint order.
    pub layers: Vec<SceneLayer>,
}

/// An emphasis as drawn.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SceneEmphasis {
    /// Kind.
    pub kind: EmphasisKind,
    /// Panels it covers (all when empty).
    pub panels: Vec<u32>,
    /// Layer it covers, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer: Option<String>,
    /// First bar.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_bar: Option<u64>,
    /// Last bar (inclusive).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_bar: Option<u64>,
}

/// Level-of-detail information when the visible bars outnumber the pixel columns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Lod {
    /// Bars aggregated per column bucket (>= 2).
    pub bars_per_bucket: u64,
    /// Number of buckets drawn.
    pub buckets: u64,
}

/// A complete compiled frame.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SceneDesc {
    /// View id.
    pub view: String,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Background colour.
    pub background: Rgba,
    /// Horizontal mapping.
    pub x: XMap,
    /// Number of bars in the view's first bar series.
    pub bars: u64,
    /// Level of detail, when aggregated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lod: Option<Lod>,
    /// Panels, top to bottom.
    pub panels: Vec<ScenePanel>,
    /// Emphasis drawn.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub emphasis: Vec<SceneEmphasis>,
    /// Paint order: primitives drawn **before** the panels' layers (emphasis bands,
    /// grid, axis lines, axis labels).
    pub under: Vec<Prim>,
    /// Paint order: primitives drawn **after** the layers (last-value tags, title,
    /// legends, dimming veils).
    pub over: Vec<Prim>,
    /// (R5/m17-int, K49) Glossary terms drawn in this scene's texts, with their boxes
    /// (each also underlined in its layer's primitives): a tap inside a box opens the
    /// explanation. Only a capsule scene under a voice has them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub terms: Vec<TermMark>,
}

/// (R5/m17-int, K49) A glossary term as drawn: where it is in the scene, its text and its
/// explanation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TermMark {
    /// The glossary term.
    pub term: String,
    /// Its explanation.
    pub text: String,
    /// The layer whose text holds it (`headline`, `ledger`, `detail`).
    pub layer: String,
    /// The drawn words' box (inclusive-exclusive like [`PixelRect`]).
    pub hit: PixelRect,
}

impl SceneDesc {
    /// All primitives in paint order: `under`, then each panel's layers, then `over`.
    pub fn prims(&self) -> impl Iterator<Item = &Prim> {
        self.under
            .iter()
            .chain(
                self.panels
                    .iter()
                    .flat_map(|p| p.layers.iter().flat_map(|l| l.prims.iter())),
            )
            .chain(self.over.iter())
    }
}
