//! `compile`: view + data provider + options → scene description (`docs/GRAMMAR.md` §14).

use std::borrow::Cow;
use std::collections::BTreeSet;

use qp_schema::grammar::{
    ColorSpec, Emphasis, EmphasisKind, Layer, Mark, Operand, Panel, Position, Scale, ScaleKind,
    View,
};
use qp_schema::scene::{
    Align, DataSource, Lod, PixelRect, Prim, Rgba, SceneDesc, SceneEmphasis, ScenePanel, XMap, YMap,
};

use crate::detmath::{self, nice_step, px, year_month};
use crate::draw::{draw, est_width, rect, Geo};
use crate::error::{ErrorKind, GrammarError};
use crate::frame::Frame;
use crate::layer::{prepare, Geom, Prepared};
use crate::provider::{DataProvider, SeriesKind};
use crate::scale::value_y;
use crate::theme::with_opacity;

mod alphabet;
mod axis;
pub(crate) mod draw;
mod emphasis;
pub(crate) mod frame;
pub(crate) mod layer;
mod panels;
pub mod scale;
mod scene;
mod series;
pub mod terms;

pub use alphabet::ALPHABET_FIXED;
use axis::{domain, domain_values, ticks};
use emphasis::resolve_emphasis;
use panels::{canonical_panels, layer_ids, panel_rects};
pub use scene::compile;
pub use series::future_times;
use series::{ext_times, first_bar_series};

/// Smallest and largest accepted frame sizes (R0's limits).
pub const MIN_WIDTH: u32 = 200;
/// Smallest accepted height.
pub const MIN_HEIGHT: u32 = 120;
/// Largest accepted width or height.
pub const MAX_DIM: u32 = 16_384;

// (R5/T3) Plot margins, panel gap, the title and label sizes and the strip and veil alphas
// are the look's (`crate::look`, `r4`: R0's values).
/// Smallest panel height.
const MIN_PANEL_H: i32 = 10;
/// Hard cap on grid lines per panel, and the largest tick index that is still exact.
const MAX_TICKS: u32 = 64;
const MAX_EXACT_TICK: f64 = 4_503_599_627_370_496.0; // 2^52

/// Transient inputs of one compile: frame size, a connection's viewport, transient
/// emphasis (attention directives, narration steps) and a theme override.
#[derive(Debug, Clone, PartialEq)]
pub struct CompileOptions {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Visible slot range `(from_bar, to_bar)`, inclusive (a connection's transient
    /// viewport). Overrides `x.domain`; it is clamped to the existing slots.
    pub range: Option<(u64, u64)>,
    /// Transient emphasis, drawn after the view's persistent emphasis.
    pub emphasis: Vec<Emphasis>,
    /// Theme override (`dark`, `light`); the view's theme otherwise.
    pub theme: Option<String>,
    /// (R4/M11) Outcome style of glyphs by name (`qp_grammar::glyph::outcome::STYLES`);
    /// absent = `r3`, the R3 drawing. A view setting like the theme override.
    pub outcome: Option<String>,
    /// Level of detail when slots are narrower than one pixel (default `true`).
    pub lod: bool,
    /// Surface reduction: aggregate when more than this many slots are visible
    /// (`docs/GRAMMAR.md` §13, `profile.max_points`).
    pub max_points: Option<u64>,
    /// (R4/T5) Glyph alphabet by name (`crate::glyph::ALPHABETS`); the product's if
    /// `None`.
    pub alphabet: Option<String>,
    /// (R4/H-FP) The face design (`crate::faces`) whose feed face this scene is, set only
    /// when compiling a face (open depth `sentence`). With a headline template
    /// ([`crate::faces::FaceDesign::headline`]) the finding's `headline` text field is
    /// the template's output; `None` (every other scene) or a design without one: the
    /// capsule's sentence. An unknown name is `invalid`.
    pub face: Option<String>,
    /// (R5/T3) The look (`crate::look::LOOKS`) the scene is drawn in; `None`: `r4`. An
    /// unknown name is `invalid`.
    pub look: Option<String>,
    /// (R5/T3) Write each text's type role into the scene (`Prim::Text::role`; the
    /// measurement harness). Off by default: scenes and their digests stay as they were.
    pub roles: bool,
    /// (RD2/C2a) The pointer's place: a look with chart conventions (`ChartLook`) draws
    /// the crosshair, its axis pills and the legend of that bar. Transient, like `range`.
    pub cursor: Option<Cursor>,
    /// (RD2/C2a) A look record that is not registered (design trials, tests); overrides
    /// `look`.
    pub look_record: Option<&'static crate::look::Look>,
}

/// (RD2/C2a) The pointer's place in data terms: a slot and, optionally, a price on the
/// first panel's scale (absent: that bar's close).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cursor {
    /// Slot (bar index); clamped to the visible range.
    pub bar: u64,
    /// Value on the first panel's scale.
    pub price: Option<f64>,
}

impl CompileOptions {
    /// Options for a `width` x `height` frame, everything else default.
    #[must_use]
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            range: None,
            emphasis: Vec::new(),
            theme: None,
            outcome: None,
            lod: true,
            max_points: None,
            alphabet: None,
            face: None,
            look: None,
            roles: false,
            cursor: None,
            look_record: None,
        }
    }
}

struct Resolved {
    kind: EmphasisKind,
    panels: Vec<u32>,
    layer: Option<String>,
    bars: Option<(u64, u64)>,
}

/// (R5/T3) A text primitive's font and role: the role it was made with (compiler texts),
/// else for a text mark the look's role of that size, for a glyph's text `glyph`; the
/// font is the role's asset (`None` for the default); the role is kept only if `keep`.
fn settle_text(p: &mut Prim, look: &crate::look::Look, mark: Option<Mark>, keep: bool) {
    let Prim::Text { px, font, role, .. } = p else {
        return;
    };
    if role.is_none() {
        *role = match mark {
            Some(Mark::Glyph) => Some("glyph".to_owned()),
            Some(_) => look.types.mark_role(f64::from(*px)).map(str::to_owned),
            None => None,
        };
    }
    if font.is_none() {
        *font = role.as_deref().and_then(|r| look.scene_font(r));
    }
    if !keep {
        *role = None;
    }
}

/// `text` cut to fit `room` pixels at `size` in font `adv` ([`est_width`]), ending in
/// "…" when cut.
fn fit_text(text: &str, room: i32, size: f64, adv: &crate::look::FontAsset) -> String {
    if est_width(text, size, adv) <= room {
        return text.to_owned();
    }
    let chars: Vec<char> = text.chars().collect();
    let mut n = chars.len();
    while n > 0 {
        let mut t: String = chars[..n].iter().collect::<String>().trim_end().to_owned();
        t.push('…');
        if est_width(&t, size, adv) <= room {
            return t;
        }
        n -= 1;
    }
    String::new()
}

/// Last drawn value of a layer and its colour (candle: close and direction colour).
fn last_value(pl: &Prepared) -> Option<(f64, Rgba)> {
    let t = pl.traces.first()?;
    let r = t
        .rows
        .iter()
        .rev()
        .find(|r| r.v.iter().any(Option::is_some))?;
    let v = match (pl.geom, t.lod_line) {
        (Geom::Candle, _) | (_, true) => r.v[3],
        _ => r.v[0],
    }?;
    let color = if pl.geom == Geom::Candle {
        r.color
    } else {
        t.base
    };
    Some((v, color))
}

fn recolor(p: Prim, c: Rgba) -> Prim {
    match p {
        Prim::Rect { x, y, w, h, .. } => Prim::Rect {
            x,
            y,
            w,
            h,
            color: c,
        },
        Prim::Polyline { points, width, .. } => Prim::Polyline {
            points,
            width,
            color: c,
        },
        Prim::Fill { points, .. } => Prim::Fill { points, color: c },
        Prim::Circle { cx, cy, r, .. } => Prim::Circle {
            cx,
            cy,
            r,
            color: c,
        },
        Prim::Text {
            x,
            baseline,
            px,
            text,
            align,
            font,
            role,
            ..
        } => Prim::Text {
            x,
            baseline,
            px,
            text,
            color: c,
            align,
            font,
            role,
        },
    }
}
