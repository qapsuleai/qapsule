//! (R5/M18) Story chart **`pavka`**: the story spine.
//!
//! The question the chart depth answers on a phone is "what happened on this instrument,
//! which of it held, what is live now" — so the stories, not the price, take most of the
//! frame. The open capsule's chart shrinks to a price **context strip** at the top (its
//! evidence geometry still drawn: K50 keeps the selected story's level, band and points
//! on the price). Below it every story of the visible range is a **lane** on the same
//! time axis, from its birth bar to its closing bar:
//!
//! - **Summary line** (one glance): how many stories, how many live, held, failed, coin
//!   flip — each count preceded by the shape it is drawn with (the legend is the summary).
//! - **Focus line:** the selected story in words (event, when, how it stands, ledger).
//! - **Live lanes first**, each on its own lane, reaching the last bar with an arrow head
//!   ("still going"); the word *canlı/live* in the right margin.
//! - **Closed lanes** packed below (several stories share a lane when they do not
//!   overlap), most recently closed on top, faint (`closed` role, ≥ 3:1), each ending in
//!   its outcome's shape: realized ● · invalidated ✕ · expired | · retracted ○.
//! - Birth marker: ▲ up, ▼ down, ◆ no direction. A coin-flip story (K53) is dashed.
//!   A story that no longer holds (`no_longer`) is thin, its arrow head hollow.
//! - The **selected** story: the focus colour, a thicker lane on a tinted row, and a
//!   guide from its birth bar up through the price strip (story ↔ price).
//! - When the lanes run out (zoomed out, many stories) they first get thinner (down to
//!   [`LANE_MIN`]); the closed stories that still do not fit are **clustered** by time in
//!   a bottom row (`+n` chips); a tap on a chip shows its bars. The selected story always
//!   keeps a lane, live ones before any closed one.
//! - A checkbox in the summary line's right margin shows or hides the closed stories
//!   (K54).
//!
//! Every story element is a layer whose data is the story's finding (`hit` names it); a
//! tap on another story moves the focus to it ([`tap`]). Colour never carries meaning
//! alone: direction, outcome, liveness and validity are shapes. Pure and deterministic
//! (`+ − × ÷`, `floor`, `min`, `max`, integer math).
//!
//! Values this design adds beyond the `r4` look (to be recorded in the look registry):
//! [`PRICE_SHARE`], [`PRICE_MIN_H`], [`STORY_MIN_H`], [`LANE_H`], [`LANE_MIN`], [`LANE_MAX`], [`CLUSTER_W`],
//! [`CLUSTER_H`], [`HIT_MIN`], [`FOCUS_ROW_ALPHA`], [`GUIDE_ALPHA`], the lane and marker
//! sizes ([`BAR_FOCUS`], [`BAR_LIVE`], [`BAR_CLOSED`], [`MARK`]), [`DASH`] and [`DASH_MAX`].

use std::collections::BTreeMap;

use qp_schema::capsule::{Direction, Validity};
use qp_schema::grammar::{Mark, Origin, ScaleKind};
use qp_schema::protocol::{HitItem, Language, StoryOutcome, StoryRow};
use qp_schema::scene::{
    Align, DataSource, Item, PixelRect, Prim, ResolvedConstant, Rgba, SceneDesc, SceneLayer,
    ScenePanel, YMap,
};

use super::{StoryDesign, StoryInput, StoryTap};
use qp_grammar::compile::MIN_HEIGHT;
use qp_grammar::scale::slot_x;
use qp_look::look::Look;
use qp_look::theme::Theme;

mod extend;

pub use extend::extend;

/// Design name.
pub const NAME: &str = "pavka";

/// The registry line.
pub const DESIGN: StoryDesign = StoryDesign {
    name: NAME,
    note: "story spine: lanes of every story on the time axis, price as a context strip",
    price_height,
    extend,
    tap,
    compile: None,
};

// (R5/S) The sizes below are `r4`'s values of the look's story record
// (`look::StoryLook`, `look::R4.story`); the design reads the look's.

/// Share of the frame height the price context strip takes.
pub const PRICE_SHARE: f64 = qp_look::look::R4.story.price_share;
/// Smallest price strip (px).
pub const PRICE_MIN_H: u32 = qp_look::look::R4.story.price_min_h;
/// Smallest story part (px).
pub const STORY_MIN_H: u32 = qp_look::look::R4.story.story_min_h;
/// Height of one lane (px; `r4` `space.scale[5]` − 4).
pub const LANE_H: i32 = qp_look::look::R4.story.lane_h;
/// Smallest lane height before stories are clustered (px).
pub const LANE_MIN: i32 = qp_look::look::R4.story.lane_min;
/// Largest lane height, when few lanes leave room (px; a larger touch target).
pub const LANE_MAX: i32 = qp_look::look::R4.story.lane_max;
/// Width of a cluster's time bucket (px).
pub const CLUSTER_W: i32 = qp_look::look::R4.story.cluster_w;
/// Height of the cluster row (px).
pub const CLUSTER_H: i32 = qp_look::look::R4.story.cluster_h;
/// Smallest width of a story's hit box (px), centred on its span.
pub const HIT_MIN: i32 = qp_look::look::R4.story.hit_min;
/// Alpha of the selected story's row tint (`focus` role).
pub const FOCUS_ROW_ALPHA: u8 = qp_look::look::R4.story.focus_row_alpha;
/// Alpha of the selected story's guide through the price strip (`focus` role).
pub const GUIDE_ALPHA: u8 = qp_look::look::R4.story.guide_alpha;
/// Lane thickness of the selected story (px).
pub const BAR_FOCUS: i32 = qp_look::look::R4.story.bar_focus;
/// Lane thickness of a live story (px).
pub const BAR_LIVE: i32 = qp_look::look::R4.story.bar_live;
/// Lane thickness of a closed story, or a live one that no longer holds (px).
pub const BAR_CLOSED: i32 = qp_look::look::R4.story.bar_closed;
/// Half size of a birth or outcome marker (px).
pub const MARK: f32 = qp_look::look::R4.story.mark;
/// Dash and gap of a coin-flip lane (px).
pub const DASH: (i32, i32) = qp_look::look::R4.story.dash;
/// Most dashes in one coin-flip lane (a longer lane gets longer dashes).
pub const DASH_MAX: i32 = qp_look::look::R4.story.dash_max;

/// Layer id prefix of a story's lane (followed by the capsule id).
pub const STORY_LAYER: &str = "story:";
/// Layer id prefix of a cluster chip (followed by its index).
pub const CLUSTER_LAYER: &str = "story-cluster:";
/// Layer id of the closed-stories checkbox.
pub const TOGGLE_LAYER: &str = "story-toggle";
/// Layer id of the summary and focus lines.
pub const SUMMARY_LAYER: &str = "story-summary";
/// Name of the story panel.
pub const PANEL_NAME: &str = "stories";

/// The price strip's height for a frame `h` high.
#[must_use]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
pub fn price_height(look: &Look, h: u32) -> u32 {
    let sl = &look.story;
    let share = (f64::from(h) * sl.price_share).floor() as u32;
    share
        .max(sl.price_min_h)
        .min(h.saturating_sub(sl.story_min_h))
        .max(MIN_HEIGHT)
}

/// A story's tap: another story's lane moves the focus; a cluster chip shows its bars;
/// the checkbox shows/hides the closed stories. The selected story's own lane is not a
/// story tap (the opening rule opens its evidence).
#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn tap(item: Option<&HitItem>, focus: Option<&str>) -> Option<StoryTap> {
    let i = item?;
    if i.layer == TOGGLE_LAYER {
        return Some(StoryTap::ToggleClosed);
    }
    if i.layer.starts_with(CLUSTER_LAYER) {
        let a = i.values.get("from_bar").copied()?;
        let b = i.values.get("to_bar").copied()?;
        return Some(StoryTap::Zoom(a.max(0.0) as u64, b.max(0.0) as u64));
    }
    if i.layer.starts_with(STORY_LAYER) {
        if let DataSource::Finding { finding } = &i.data {
            if Some(finding.as_str()) != focus {
                return Some(StoryTap::Focus(finding.clone()));
            }
        }
    }
    None
}

/// The words of this design, by language.
struct Words {
    stories: fn(usize) -> String,
    live: &'static str,
    held: &'static str,
    failed: &'static str,
    coin: &'static str,
    closed: &'static str,
    hidden: &'static str,
}

const TR: Words = Words {
    stories: |n| format!("{n} hikâye"),
    live: "canlı",
    held: "tuttu",
    failed: "tutmadı",
    coin: "yazı tura",
    closed: "kapanmış",
    hidden: "kapanmışlar gizli",
};

const EN: Words = Words {
    stories: |n| {
        if n == 1 {
            "1 story".to_owned()
        } else {
            format!("{n} stories")
        }
    },
    live: "live",
    held: "held",
    failed: "failed",
    coin: "coin flip",
    closed: "closed",
    hidden: "closed hidden",
};

fn words(l: Language) -> &'static Words {
    match l {
        Language::En => &EN,
        Language::Tr => &TR,
    }
}

/// How a story is drawn.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Shape {
    Live,
    Realized,
    Failed,
    Expired,
    Retracted,
    Coin,
}

/// A visible story, placed.
struct Placed<'r> {
    row: &'r StoryRow,
    end: u64,
    /// Pixel span (packing box).
    span: (i32, i32),
    /// Lane ends (birth column, closing column; `ends` of the extend).
    ends: (i32, i32),
    live: bool,
    focus: bool,
}

/// `parts` joined (what `format!` of them gives, without its machinery: a story chart
/// names every lane this way, and on the JIT-less phone setting that machinery costs).
fn join(parts: &[&str]) -> String {
    let mut s = String::with_capacity(parts.iter().map(|p| p.len()).sum());
    for p in parts {
        s.push_str(p);
    }
    s
}

fn with_alpha(c: Rgba, a: u8) -> Rgba {
    [c[0], c[1], c[2], a]
}

fn rect(x0: i32, y0: i32, x1: i32, y1: i32, color: Rgba) -> Option<Prim> {
    (x1 > x0 && y1 > y0).then(|| Prim::Rect {
        x: x0,
        y: y0,
        w: (x1 - x0).unsigned_abs(),
        h: (y1 - y0).unsigned_abs(),
        color,
    })
}

fn fill(points: &[(f32, f32)], color: Rgba) -> Prim {
    Prim::Fill {
        points: points.iter().map(|(x, y)| [*x, *y]).collect(),
        color,
    }
}

fn text(look: &Look, role: &str, x: i32, baseline: i32, s: String, color: Rgba) -> Prim {
    #[allow(clippy::cast_possible_truncation)]
    let px = look.types.role(role).map_or(11.0, |r| r.px) as f32;
    Prim::Text {
        x,
        baseline,
        px,
        text: s,
        color,
        align: Align::Left,
        font: look.scene_font(role),
        role: None,
    }
}

/// Width of `s` in `role`: the compiler's text measure (`FontAsset::width`; a monospaced
/// file: characters × advance × size, as before; (R5/S) a proportional one: its advance
/// table, not the mean advance, so a word measured to fit does fit).
fn text_w(look: &Look, role: &str, s: &str) -> i32 {
    if s.is_empty() {
        return 0;
    }
    let px = look.types.role(role).map_or(11.0, |r| r.px);
    look.role_font(role).width(s, px)
}

/// `s` cut to at most `w` px in `role`, with `…` when cut.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
fn fit(look: &Look, role: &str, s: &str, w: i32) -> String {
    if text_w(look, role, s) <= w {
        return s.to_owned();
    }
    let px = look.types.role(role).map_or(11.0, |r| r.px);
    let font = look.role_font(role);
    if !font.advances.mono {
        // (R5/S) A proportional file: the longest prefix that fits with its `…`.
        let chars: Vec<char> = s.chars().collect();
        for n in (0..chars.len()).rev() {
            let mut out: String = chars[..n].iter().collect();
            out.push('…');
            if text_w(look, role, &out) <= w || n == 0 {
                return out;
            }
        }
    }
    let adv = font.advance;
    let n = (f64::from(w.max(0)) / (adv * px)).floor() as usize;
    let mut out: String = s.chars().take(n.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// The small shape of `s` centred at (`cx`, `cy`), half size `r`, in `color` (the lane's
/// end cap, and the summary's legend).
#[allow(clippy::too_many_arguments)]
fn cap(
    out: &mut Vec<Prim>,
    s: Shape,
    cx: f32,
    cy: f32,
    r: f32,
    color: Rgba,
    bg: Rgba,
    dash: (i32, i32),
) {
    match s {
        Shape::Live => out.push(fill(
            &[(cx + r, cy), (cx - r * 0.6, cy - r), (cx - r * 0.6, cy + r)],
            color,
        )),
        Shape::Realized => out.push(Prim::Circle {
            cx,
            cy,
            r: r * 0.9,
            color,
        }),
        Shape::Failed => {
            let d = r * 0.8;
            out.push(Prim::Polyline {
                points: vec![[cx - d, cy - d], [cx + d, cy + d]],
                width: 2.0,
                color,
            });
            out.push(Prim::Polyline {
                points: vec![[cx - d, cy + d], [cx + d, cy - d]],
                width: 2.0,
                color,
            });
        }
        Shape::Expired => {
            #[allow(clippy::cast_possible_truncation)]
            let (x, y, h) = (
                (cx - 1.0).floor() as i32,
                (cy - r).floor() as i32,
                (2.0 * r) as u32,
            );
            out.push(Prim::Rect {
                x,
                y,
                w: 2,
                h,
                color,
            });
        }
        Shape::Retracted => {
            out.push(Prim::Circle { cx, cy, r, color });
            out.push(Prim::Circle {
                cx,
                cy,
                r: r - 2.0,
                color: bg,
            });
        }
        Shape::Coin => {
            #[allow(clippy::cast_possible_truncation)]
            let (x, y) = ((cx - r).floor() as i32, (cy - 1.0).floor() as i32);
            let mut a = x;
            #[allow(clippy::cast_possible_truncation)]
            let end = (cx + r).floor() as i32;
            while a < end {
                let b = (a + dash.0).min(end);
                out.extend(rect(a, y, b, y + 2, color));
                a = b + dash.1;
            }
        }
    }
}

fn outcome_shape(r: &StoryRow) -> Shape {
    match r.outcome {
        Some(StoryOutcome::Realized) => Shape::Realized,
        Some(StoryOutcome::Invalidated) => Shape::Failed,
        Some(StoryOutcome::Expired) => Shape::Expired,
        Some(StoryOutcome::Retracted) => Shape::Retracted,
        None => Shape::Live,
    }
}
