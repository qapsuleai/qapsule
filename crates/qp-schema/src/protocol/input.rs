//! Input and attention: gestures, view state, openings and attention directives.

use super::*;

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
        /// (R5/m17-int) Capsule voice: the language of the face's headline and ledger,
        /// the open capsule's detail and ledger, and the glossary
        /// (`qp_capsule::voice::VOICES`: `r4`, `marla`, `selvi`; default `r4`, today's
        /// text).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        voice: Option<String>,
        /// (R5/M18) Story chart design drawn at chart depth (`qp_grammar::stories::STORIES`;
        /// `r4`: none, the chart of the open capsule alone — the default).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        story: Option<String>,
        /// (R5/M18, K54) Hide the closed stories in the story chart (`false`: shown, the
        /// default). The selected story stays drawn.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        hide_closed: Option<bool>,
        /// (R5/M20) Open capsule design: what the capsule depth shows
        /// (`qp_grammar::open::OPENS`: `r4`, `tesir`, `nobra`; default `r4`, today's card).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        open: Option<String>,
        /// (RP4/T7) Row time: the capsule rows (the voice's headline) and the story chart's
        /// rows carry the event's time — `14:35` on today's intraday bars, else the date
        /// (`12 Kas` / `Nov 12`), in the exchange's time zone (`false`: none, the default;
        /// `qp_capsule::realize::stamp`).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        clock: Option<bool>,
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
    /// (R5/m17-int) Capsule voice its capsules speak in (absent: the default, `r4`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voice: Option<String>,
    /// (R5/M18) Story chart design drawn at chart depth (absent: none, the default).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub story: Option<String>,
    /// (R5/M18, K54) The story chart hides closed stories (absent/`false`: shown).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub hide_closed: bool,
    /// (R5/M20) Open capsule design of its capsule depth (absent: the default, `r4`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open: Option<String>,
    /// (R5/M20 `nobra`) The question the open capsule's design shows opened (a design with
    /// questions: `what`, `basis`, `who`, `before`, `why`; absent: none). Reset when the
    /// capsule closes or another one opens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ask: Option<String>,
    /// (RP4/T7) Rows carry the event's time (the `style` gesture's `clock`; absent/`false`:
    /// none, every surface as before).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub clock: bool,
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
