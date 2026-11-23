//! (R5/M18) `korda` — lifelines ("ömür şeritleri"). Under the price panel, a panel of
//! time-aligned horizontal lanes: one lane segment per story, from its birth bar to its
//! closing bar (an open story runs to the last bar). The birth carries a direction mark
//! (▲ up, ▼ down, ◆ none), the end its outcome (● realized, × invalidated, ○ expired,
//! □ retracted, ▸ still open). Lanes never overlap: stories are packed first-fit by birth,
//! in three groups from the price downwards — the selected story alone on the first lane
//! (focus colour, its label, a guide up to its birth bar on the price), the open stories,
//! then the closed ones (thin, in the closed colour, K54). The evidence geometry and the
//! glyph over the price are the selected story's only (the presenter's chart view).
//!
//! Crowding: when the lanes do not fit at full pitch (label + marks) they are drawn
//! compact (thinner, smaller marks, no labels); when even the compact pitch does not fit,
//! the closed stories (and the open ones if they too do not fit) are summarized as a
//! histogram of births per bucket of bars (stacked by outcome, each outcome its own fill
//! pattern), and a tap on a bucket zooms to its bars. The header row says how many open
//! and closed stories are in view and is the "hide / show closed" control (K54).
//!
//! Colour never carries meaning alone: direction is the birth mark's shape, the outcome
//! the end mark's shape (or the bucket segment's fill pattern), "no longer holds" a
//! double rail, "coin flip" (K53) a dotted lane.
//!
//! Values not in the look record yet (`qp_look::look`, R5/T3) are the `const`s below,
//! derived from the `r4` look's spacing and type scale; they should be moved into the
//! record (M18 report).

use std::collections::BTreeMap;

use qp_schema::capsule::{Direction, Validity};
use qp_schema::grammar::{Mark, Panel, Scale, ScaleKind, View};
use qp_schema::protocol::{HitItem, Language, StoryOutcome, StoryRow};
use qp_schema::scene::{
    Align, DataSource, Item, PixelRect, Prim, Rgba, SceneDesc, SceneLayer, XMap,
};

use super::{StoryDesign, StoryInput, StoryTap};
use crate::compile::{compile, CompileOptions};
use crate::provider::{DataProvider, SeriesKind};
use qp_look::error::GrammarError;
use qp_look::look::Look;
use qp_look::theme::{with_opacity, Theme};

mod draw;
mod scene;

pub use scene::scene;

/// Header row (counts and the closed-stories control), px.
pub const HEADER_PX: i32 = 16;
/// Lane pitch of the selected story (its label above its line), px.
pub const SELECTED_PITCH: i32 = 22;
/// Lane pitch with full-size marks and an inline label, px.
pub const FULL_PITCH: i32 = 13;
/// Largest compact pitch, px.
pub const COMPACT_MAX: i32 = 12;
/// Smallest compact pitch; below it the stories are summarized, px.
pub const MIN_PITCH: i32 = 6;
/// Pitch of open-story lanes beside a summary, px.
pub const SUMMARY_LIVE_PITCH: i32 = 8;
/// Gap between lane groups (selected, open, closed), px.
pub const GROUP_GAP: i32 = 3;
/// Smallest histogram height in a summary, px.
pub const MIN_HIST_PX: i32 = 24;
/// Largest share of the plot height the story panel may take.
pub const MAX_SHARE: f64 = 0.42;
/// Smallest bucket width of the summary histogram, px.
pub const BUCKET_PX: f64 = 12.0;
/// Free pixels between two stories sharing a lane.
pub const LANE_GAP_PX: f64 = 6.0;
/// Mark radius at full pitch / compact pitch, px.
pub const MARK_R: [f64; 2] = [4.5, 2.5];
/// Lane widths: selected, open, closed (closed is "soluk" by weight, not by alpha, so
/// it keeps the closed colour's contrast, A31), px.
pub const LANE_W: [f64; 3] = [3.0, 2.0, 1.5];
/// Opacity of the selected story's guide over the price.
pub const GUIDE_OPACITY: f64 = 0.55;

/// The words the chart writes itself (the rows carry their own).
struct Words {
    live: &'static str,
    closed: &'static str,
    hidden: &'static str,
    hide: &'static str,
    show: &'static str,
    coin: &'static str,
    stories: &'static str,
}

fn words(lang: Language) -> Words {
    match lang {
        Language::Tr => Words {
            live: "canlı",
            closed: "kapanmış",
            hidden: "gizli",
            hide: "gizle",
            show: "göster",
            coin: "yazı tura",
            stories: "hikâye",
        },
        Language::En => Words {
            live: "live",
            closed: "closed",
            hidden: "hidden",
            hide: "hide",
            show: "show",
            coin: "coin flip",
            stories: "stories",
        },
    }
}

/// One story's life in bars.
#[derive(Clone, Copy)]
struct Life<'a> {
    row: &'a StoryRow,
    start: u64,
    end: u64,
    open: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Mode {
    Full,
    Compact,
    Summary,
}

/// The plan of the story panel: lane groups and their pitches.
struct Plan<'a> {
    mode: Mode,
    sel: Option<Life<'a>>,
    /// Open stories on lanes, with their lane index.
    live: Vec<(Life<'a>, usize)>,
    live_lanes: usize,
    live_pitch: i32,
    /// Closed stories on lanes (not in a summary).
    closed: Vec<(Life<'a>, usize)>,
    closed_pitch: i32,
    /// Stories in the summary histogram.
    hist: Vec<Life<'a>>,
    /// Panel height wanted.
    height: i32,
}

/// The visible slot range of `view` over `bars` (the compile's rule without the future
/// slots): `opts.range`, else the bars inside `x.domain`, else all.
fn visible(view: &View, times: &[i64], opts: &CompileOptions) -> Option<(u64, u64)> {
    let n = times.len() as u64;
    if n == 0 {
        return None;
    }
    if let Some((a, b)) = opts.range {
        let (a, b) = (a.min(b), a.max(b));
        return (a < n).then_some((a, b));
    }
    if let Some([t0, t1]) = view.x.domain {
        #[allow(clippy::cast_precision_loss)]
        let a = times.iter().position(|t| *t as f64 >= t0)?;
        #[allow(clippy::cast_precision_loss)]
        let b = times.iter().rposition(|t| *t as f64 <= t1)?;
        return (a <= b).then_some((a as u64, b as u64));
    }
    Some((0, n - 1))
}

fn first_bar_series(view: &View, p: &dyn DataProvider) -> Option<String> {
    let bars = |s: &str| p.series_kind(s) == Some(SeriesKind::Bars);
    view.layers
        .iter()
        .filter(|l| l.mark == Mark::Candle)
        .find_map(|l| l.series.clone().filter(|s| bars(s)))
        .or_else(|| {
            view.layers
                .iter()
                .find_map(|l| l.series.clone().filter(|s| bars(s)))
        })
}

fn slot_x(x: &XMap, bar: u64) -> f64 {
    crate::scale::slot_x(x, bar)
}

/// First-fit lane packing by birth (pixel extents with marks of radius `r`).
fn pack<'a>(lives: &[Life<'a>], x: &XMap, r: f64) -> (Vec<(Life<'a>, usize)>, usize) {
    let mut ends: Vec<f64> = Vec::new();
    let mut out = Vec::with_capacity(lives.len());
    for l in lives {
        let x0 = slot_x(x, l.start) - r;
        let x1 = slot_x(x, l.end) + r;
        let lane = if let Some(i) = ends.iter().position(|e| *e + LANE_GAP_PX <= x0) {
            ends[i] = x1;
            i
        } else {
            ends.push(x1);
            ends.len() - 1
        };
        out.push((*l, lane));
    }
    (out, ends.len())
}

fn gaps(groups: &[usize]) -> i32 {
    let n = groups.iter().filter(|g| **g > 0).count();
    i32::try_from(n.saturating_sub(1)).unwrap_or(0) * GROUP_GAP
}

fn lanes_i(n: usize) -> i32 {
    i32::try_from(n).unwrap_or(i32::MAX / 64)
}

fn plan<'a>(
    sel: Option<Life<'a>>,
    live: &[Life<'a>],
    closed: &[Life<'a>],
    x: &XMap,
    max_h: i32,
) -> Plan<'a> {
    let s = usize::from(sel.is_some());
    // Full: every lane at full pitch.
    let (lf, nlf) = pack(live, x, MARK_R[0]);
    let (cf, ncf) = pack(closed, x, MARK_R[0]);
    let full_h = HEADER_PX
        + lanes_i(s) * SELECTED_PITCH
        + lanes_i(nlf + ncf) * FULL_PITCH
        + gaps(&[s, nlf, ncf]);
    if full_h <= max_h {
        return Plan {
            mode: Mode::Full,
            sel,
            live: lf,
            live_lanes: nlf,
            live_pitch: FULL_PITCH,
            closed: cf,
            closed_pitch: FULL_PITCH,
            hist: Vec::new(),
            height: full_h,
        };
    }
    // Compact: the selected story keeps its full lane; the others share what is left.
    let (lc, nlc) = pack(live, x, MARK_R[1]);
    let (cc, ncc) = pack(closed, x, MARK_R[1]);
    let room = max_h - HEADER_PX - lanes_i(s) * SELECTED_PITCH - gaps(&[s, nlc, ncc]);
    let n = lanes_i(nlc + ncc).max(1);
    let p = (room / n).min(COMPACT_MAX);
    if p >= MIN_PITCH {
        return Plan {
            mode: Mode::Compact,
            sel,
            live: lc,
            live_lanes: nlc,
            live_pitch: p,
            closed: cc,
            closed_pitch: p,
            hist: Vec::new(),
            height: max_h,
        };
    }
    // Summary: closed stories (and open ones that do not fit) in a histogram.
    let room = max_h - HEADER_PX - lanes_i(s) * SELECTED_PITCH - gaps(&[s, nlc, 1]) - MIN_HIST_PX;
    let live_fit = lanes_i(nlc) * SUMMARY_LIVE_PITCH <= room;
    let mut hist: Vec<Life<'a>> = closed.to_vec();
    let (live, live_lanes) = if live_fit {
        (lc, nlc)
    } else {
        hist.extend_from_slice(live);
        (Vec::new(), 0)
    };
    Plan {
        mode: Mode::Summary,
        sel,
        live,
        live_lanes,
        live_pitch: SUMMARY_LIVE_PITCH,
        closed: Vec::new(),
        closed_pitch: 0,
        hist,
        height: max_h,
    }
}

/// Layer id prefix of one story's lane (`story:<capsule id>`; data: its finding).
pub const STORY_LAYER: &str = "story:";
/// Layer id prefix of a cluster of stories (`story-cluster:<from bar>-<to bar>`).
pub const CLUSTER_LAYER: &str = "story-cluster:";
/// Layer id of the "closed stories shown / hidden" control.
pub const TOGGLE_LAYER: &str = "story-toggle";

/// (R5/S) The design's registry entry: it compiles the whole scene ([`scene`]).
pub const DESIGN: StoryDesign = StoryDesign {
    name: "korda",
    note: "lifelines: one time-aligned lane per story under the price, birth mark to outcome",
    price_height: |_, h| h,
    extend: |_, _, scene, _, _| scene,
    tap,
    compile: Some(scene),
};

/// What a tap on a story chart asks for, read from the point query's item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoryHit {
    /// Move the focus to this capsule (another story's lane or mark).
    Focus {
        /// Capsule id.
        capsule: String,
        /// Its finding.
        finding: String,
    },
    /// Zoom the chart to this bar range (a cluster).
    Zoom {
        /// First bar.
        from: u64,
        /// Last bar.
        to: u64,
    },
    /// Show or hide the closed stories.
    ToggleClosed,
}

/// The story chart's meaning of a hit item (`None`: not a story chart element).
#[must_use]
pub fn story_hit(item: &HitItem) -> Option<StoryHit> {
    if item.layer == TOGGLE_LAYER {
        return Some(StoryHit::ToggleClosed);
    }
    if let Some(rest) = item.layer.strip_prefix(CLUSTER_LAYER) {
        let (a, b) = rest.split_once('-')?;
        return Some(StoryHit::Zoom {
            from: a.parse().ok()?,
            to: b.parse().ok()?,
        });
    }
    let capsule = item.layer.strip_prefix(STORY_LAYER)?;
    match &item.data {
        DataSource::Finding { finding } => Some(StoryHit::Focus {
            capsule: capsule.to_owned(),
            finding: finding.clone(),
        }),
        _ => None,
    }
}

/// (R5/S) The registry's tap rule: [`story_hit`]; the open story's own lane is not the
/// story chart's (the opening rule opens its evidence).
#[must_use]
pub fn tap(item: Option<&HitItem>, focus: Option<&str>) -> Option<StoryTap> {
    match story_hit(item?)? {
        StoryHit::ToggleClosed => Some(StoryTap::ToggleClosed),
        StoryHit::Zoom { from, to } => Some(StoryTap::Zoom(from, to)),
        StoryHit::Focus { finding, .. } => {
            (Some(finding.as_str()) != focus).then_some(StoryTap::Focus(finding))
        }
    }
}

fn mode_of(small: bool) -> Mode {
    if small {
        Mode::Compact
    } else {
        Mode::Full
    }
}

/// The pixel x where the next story on the same lane starts (the room for a label).
fn next_start(on: &[(Life<'_>, usize)], l: &Life<'_>, lane: usize, d: &Draw<'_>) -> Option<f64> {
    on.iter()
        .filter(|(o, ln)| *ln == lane && o.start > l.end)
        .map(|(o, _)| slot_x(&d.x, o.start))
        .reduce(f64::min)
}

/// Removes the compile's value grid and value labels inside the story panel `rc` (it
/// has no value scale); the time grid and the frame stay.
fn strip_value_grid(under: &mut Vec<Prim>, rc: PixelRect) {
    under.retain(|p| match p {
        Prim::Rect { x, y, w, h, .. } => {
            let full = *x == rc.left && i64::from(*w) == i64::from(rc.right - rc.left);
            !(*h == 1 && full && *y > rc.top && *y < rc.bottom)
        }
        Prim::Text { x, baseline, .. } => {
            !(*x == rc.right + 6 && *baseline - 4 > rc.top && *baseline - 4 < rc.bottom)
        }
        _ => true,
    });
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Role {
    Selected,
    Live,
    Closed,
}

struct Draw<'a> {
    x: XMap,
    rc: PixelRect,
    theme: Theme,
    look: &'a Look,
    roles: bool,
    a: u64,
    b: u64,
    price_top: i32,
    lang: Language,
    /// (RP4/T7) The rows' time stamps ([`StoryInput::stamp_of`]).
    stamp_of: Option<&'a super::StampFn<'a>>,
}

#[allow(clippy::cast_possible_truncation)]
fn f(v: f64) -> f32 {
    v as f32
}

fn poly(points: &[[f64; 2]], width: f64, color: Rgba) -> Prim {
    Prim::Polyline {
        points: points.iter().map(|q| [f(q[0]), f(q[1])]).collect(),
        width: f(width),
        color,
    }
}

fn fill(points: &[[f64; 2]], color: Rgba) -> Prim {
    Prim::Fill {
        points: points.iter().map(|q| [f(q[0]), f(q[1])]).collect(),
        color,
    }
}

#[allow(clippy::cast_possible_truncation)]
fn rect(x: f64, y: f64, w: f64, h: f64, color: Rgba) -> Prim {
    crate::draw::rect(
        x.floor() as i32,
        y.floor() as i32,
        w.round().max(1.0) as i32,
        h.round().max(1.0) as i32,
        color,
    )
}

/// `text` cut to `room` px at `px` (advance `adv` em), with "…" when cut; `None` when not
/// even three characters fit.
fn fit(text: &str, room: f64, px: f64, adv: f64) -> Option<String> {
    let per = px * adv;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let n = (room / per).floor().max(0.0) as usize;
    let len = text.chars().count();
    if n >= len {
        return Some(text.to_owned());
    }
    if n < 4 {
        return None;
    }
    let mut s: String = text.chars().take(n - 1).collect();
    s = s.trim_end().to_owned();
    s.push('…');
    Some(s)
}
