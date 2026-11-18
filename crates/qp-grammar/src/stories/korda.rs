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
//! Values not in the look record yet (`crate::look`, R5/T3) are the `const`s below,
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
use crate::error::GrammarError;
use crate::look::Look;
use crate::provider::{DataProvider, SeriesKind};
use crate::theme::{with_opacity, Theme};

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

/// The `korda` story chart (`super::StoryDesign::compile`).
///
/// # Errors
/// The compile's errors (unknown look or theme, a view that does not compile).
#[allow(clippy::too_many_lines)]
pub fn scene(
    view: &View,
    p: &dyn DataProvider,
    opts: &CompileOptions,
    inp: &StoryInput<'_>,
) -> Result<SceneDesc, GrammarError> {
    let look = crate::look::by_name(opts.look.as_deref())?;
    let theme = look.theme(opts.theme.as_deref().or(view.theme.as_deref()))?;
    let Some(bs) = first_bar_series(view, p) else {
        return compile(view, p, opts);
    };
    let times: Vec<i64> = p
        .bars(&bs)
        .map(|b| b.iter().map(|x| x.t).collect())
        .unwrap_or_default();
    let Some((a, b)) = visible(view, &times, opts) else {
        return compile(view, p, opts);
    };
    let now = times.len() as u64 - 1;
    let b_last = b;
    // The horizontal mapping the compile will use (same formula, `compile.rs`).
    let (wi, hi) = (opts.width.cast_signed(), opts.height.cast_signed());
    let [m_left, m_right, m_top, m_bottom] = look.space.plot;
    let plot_w = wi - m_right - m_left;
    #[allow(clippy::cast_precision_loss)]
    let xm = XMap {
        from_bar: a,
        to_bar: b_last,
        left: f64::from(m_left),
        slot: f64::from(plot_w) / (b_last - a + 1) as f64,
    };
    // The stories in view, by group.
    let mut sel: Option<Life<'_>> = None;
    let mut live: Vec<Life<'_>> = Vec::new();
    let mut closed: Vec<Life<'_>> = Vec::new();
    let mut hidden = 0usize;
    let mut rows: Vec<&StoryRow> = inp.rows.iter().filter(|r| r.series == bs).collect();
    rows.sort_by(|x, y| {
        x.born_bar
            .cmp(&y.born_bar)
            .then(x.born_t.cmp(&y.born_t))
            .then(x.id.cmp(&y.id))
    });
    for r in rows {
        let open = r.closed_bar.is_none();
        let end = r.closed_bar.unwrap_or_else(|| r.last_bar.max(now));
        if r.born_bar > b || end < a {
            continue;
        }
        let l = Life {
            row: r,
            start: r.born_bar,
            end,
            open,
        };
        if r.finding == inp.focus.unwrap_or("") {
            sel = Some(l);
        } else if open {
            live.push(l);
        } else if !inp.hide_closed {
            closed.push(l);
        } else {
            hidden += 1;
        }
    }
    let closed_n = closed.len() + hidden + usize::from(sel.is_some_and(|s| !s.open));
    let live_n = live.len() + usize::from(sel.is_some_and(|s| s.open));
    if sel.is_none() && live.is_empty() && closed.is_empty() && hidden == 0 {
        // Nothing to tell (or the list has not arrived yet): the chart alone.
        return compile(view, p, opts);
    }
    // The story panel's height: what the plan wants, at most a share of the plot.
    let base_panels = if view.panels.is_empty() {
        1
    } else {
        view.panels.len()
    };
    let gap = look.space.panel_gap;
    let avail = hi - m_top - m_bottom - gap * lanes_i(base_panels);
    #[allow(clippy::cast_possible_truncation)]
    let max_h = ((f64::from(avail) * MAX_SHARE).floor() as i32).max(HEADER_PX + SELECTED_PITCH);
    let plan = plan(sel, &live, &closed, &xm, max_h);
    let want = plan.height.min(avail - 10);
    // The view with the story panel appended (an empty panel: the compile draws its
    // frame and the time grid; its value grid is removed below).
    let mut v = view.clone();
    let mut panels: Vec<Panel> = if v.panels.is_empty() {
        vec![Panel {
            id: Some(0),
            name: None,
            weight: None,
            y: Scale::new(ScaleKind::Linear),
        }]
    } else {
        v.panels.clone()
    };
    let total: f64 = panels
        .iter()
        .enumerate()
        .map(|(i, q)| q.weight.unwrap_or(if i == 0 { 3.0 } else { 1.0 }))
        .sum();
    for (i, q) in panels.iter_mut().enumerate() {
        q.weight = Some(q.weight.unwrap_or(if i == 0 { 3.0 } else { 1.0 }));
    }
    let id = panels
        .iter()
        .filter_map(|q| q.id)
        .max()
        .map_or(0, |m| m + 1);
    panels.push(Panel {
        id: Some(id),
        name: Some("stories".to_owned()),
        weight: Some(total * f64::from(want) / f64::from((avail - want).max(1))),
        y: Scale {
            kind: ScaleKind::Linear,
            domain: Some([0.0, 1.0]),
            id: None,
        },
    });
    v.panels = panels;
    v.y.clear();
    let mut o = opts.clone();
    o.range = Some((a, b));
    let mut sc = compile(&v, p, &o)?;
    let Some(pi) = sc.panels.iter().position(|q| q.id == id) else {
        return Ok(sc);
    };
    let rc = sc.panels[pi].rect;
    strip_value_grid(&mut sc.under, rc);
    let price_top = sc.panels.first().map_or(rc.top, |q| q.rect.top);
    let d = Draw {
        x: sc.x,
        rc,
        theme,
        look,
        roles: opts.roles,
        a,
        b: sc.x.to_bar,
        price_top,
        lang: inp.lang,
    };
    let mut layers: Vec<SceneLayer> = Vec::new();
    // Lanes, top to bottom: selected, open, closed (closed painted first).
    let mut y = rc.top + HEADER_PX;
    let mut sel_layer = None;
    if let Some(s) = plan.sel {
        sel_layer = Some(d.lane(&s, y, SELECTED_PITCH, Role::Selected, plan.mode, None));
        y += SELECTED_PITCH + GROUP_GAP;
    }
    let live_y = y;
    if plan.live_lanes > 0 {
        y += lanes_i(plan.live_lanes) * plan.live_pitch + GROUP_GAP;
    }
    let closed_y = y;
    let small = plan.mode != Mode::Full;
    for (l, lane) in &plan.closed {
        let next = next_start(&plan.closed, l, *lane, &d);
        let ly = closed_y + lanes_i(*lane) * plan.closed_pitch;
        layers.push(d.lane(l, ly, plan.closed_pitch, Role::Closed, mode_of(small), next));
    }
    if plan.mode == Mode::Summary && !plan.hist.is_empty() {
        let hy = if plan.live_lanes > 0 { y } else { live_y };
        layers.extend(d.histogram(&plan.hist, hy, rc.bottom - 2, inp.focus.unwrap_or("")));
    }
    for (l, lane) in &plan.live {
        let next = next_start(&plan.live, l, *lane, &d);
        let ly = live_y + lanes_i(*lane) * plan.live_pitch;
        layers.push(d.lane(l, ly, plan.live_pitch, Role::Live, mode_of(small), next));
    }
    if let Some(l) = sel_layer {
        layers.push(l);
    }
    layers.push(d.header(live_n, closed_n, hidden, !inp.hide_closed));
    sc.panels[pi].layers = layers;
    Ok(sc)
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

impl Draw<'_> {
    fn text(&self, x: f64, baseline: i32, s: String, color: Rgba) -> Prim {
        #[allow(clippy::cast_possible_truncation)]
        Prim::Text {
            x: x.floor() as i32,
            baseline,
            px: f(self.look.types.label.px),
            text: s,
            color,
            align: Align::Left,
            font: self.look.scene_font("label"),
            role: self.roles.then(|| "label".to_owned()),
        }
    }

    fn label_px(&self) -> f64 {
        self.look.types.label.px
    }

    fn adv(&self) -> f64 {
        self.look.role_font("label").advance
    }

    fn dir_color(&self, d: Direction) -> Rgba {
        match d {
            Direction::Up => self.theme.up,
            Direction::Down => self.theme.down,
            Direction::Neutral => self.theme.neutral,
        }
    }

    fn inside(&self, bar: u64) -> bool {
        bar >= self.a && bar <= self.b
    }

    /// The birth mark: ▲ up, ▼ down, ◆ none.
    fn birth(&self, out: &mut Vec<Prim>, cx: f64, cy: f64, r: f64, d: Direction, c: Rgba) {
        let pts = match d {
            Direction::Up => vec![[cx, cy - r], [cx + r, cy + r * 0.8], [cx - r, cy + r * 0.8]],
            Direction::Down => vec![[cx, cy + r], [cx + r, cy - r * 0.8], [cx - r, cy - r * 0.8]],
            Direction::Neutral => vec![[cx, cy - r], [cx + r, cy], [cx, cy + r], [cx - r, cy]],
        };
        out.push(fill(&pts, c));
    }

    /// The end mark: ● realized, × invalidated, ○ expired, □ retracted, ▸ open.
    fn end(&self, out: &mut Vec<Prim>, cx: f64, cy: f64, r: f64, o: Option<StoryOutcome>, c: Rgba) {
        match o {
            Some(StoryOutcome::Realized) => out.push(Prim::Circle {
                cx: f(cx),
                cy: f(cy),
                r: f(r),
                color: c,
            }),
            Some(StoryOutcome::Invalidated) => {
                let w = (r * 0.35).max(1.2);
                out.push(poly(&[[cx - r, cy - r], [cx + r, cy + r]], w, c));
                out.push(poly(&[[cx - r, cy + r], [cx + r, cy - r]], w, c));
            }
            Some(StoryOutcome::Expired) => {
                out.push(Prim::Circle {
                    cx: f(cx),
                    cy: f(cy),
                    r: f(r),
                    color: c,
                });
                out.push(Prim::Circle {
                    cx: f(cx),
                    cy: f(cy),
                    r: f((r - (r * 0.4).max(1.2)).max(0.5)),
                    color: self.theme.bg,
                });
            }
            Some(StoryOutcome::Retracted) => {
                let w = (r * 0.35).max(1.2);
                let pts = [
                    [cx - r, cy - r],
                    [cx + r, cy - r],
                    [cx + r, cy + r],
                    [cx - r, cy + r],
                    [cx - r, cy - r],
                ];
                out.push(poly(&pts, w, c));
            }
            None => out.push(fill(
                &[
                    [cx - r * 0.6, cy - r],
                    [cx + r * 0.9, cy],
                    [cx - r * 0.6, cy + r],
                ],
                c,
            )),
        }
    }

    /// One story's lane in a layer `story:<capsule>` (data: its finding; one item, its
    /// lane box).
    #[allow(clippy::too_many_lines)]
    fn lane(
        &self,
        l: &Life<'_>,
        top: i32,
        pitch: i32,
        role: Role,
        mode: Mode,
        next: Option<f64>,
    ) -> SceneLayer {
        let r = l.row;
        let full = mode == Mode::Full;
        let rad = if full || role == Role::Selected {
            MARK_R[0]
        } else {
            MARK_R[1]
        };
        // The selected story's label sits above its line; a full lane's beside it.
        let above = role == Role::Selected;
        let cy = if above {
            f64::from(top) + f64::from(pitch) - rad - 1.5
        } else {
            f64::from(top) + f64::from(pitch) / 2.0
        };
        let (left, right) = (f64::from(self.rc.left), f64::from(self.rc.right));
        let xb = slot_x(&self.x, l.start);
        let xe = slot_x(&self.x, l.end.min(self.b.max(l.start)));
        let x0 = xb.max(left);
        let x1 = if l.end > self.b { right } else { xe.min(right) };
        let (lw, line_c) = match role {
            Role::Selected => (LANE_W[0], self.theme.focus),
            Role::Live => (
                LANE_W[1],
                if r.validity == Validity::NoLonger {
                    self.theme.invalid
                } else {
                    self.theme.valid
                },
            ),
            Role::Closed => (LANE_W[2], self.theme.closed),
        };
        let mark_c = match role {
            Role::Selected => self.theme.focus,
            Role::Live => self.dir_color(r.direction),
            Role::Closed => self.theme.closed,
        };
        let mut prims = Vec::new();
        // The selected story's guide: from its birth up through the price.
        if role == Role::Selected && self.inside(l.start) {
            prims.push(poly(
                &[[xb + 0.5, f64::from(self.price_top)], [xb + 0.5, cy]],
                1.0,
                with_opacity(self.theme.focus, GUIDE_OPACITY),
            ));
        }
        if x1 > x0 {
            if r.coin_flip {
                // Dotted (K53).
                let mut xx = x0;
                let d = (lw + 1.0).max(2.0);
                while xx < x1 {
                    prims.push(rect(xx, cy - d / 2.0, d, d, line_c));
                    xx += d * 2.5;
                }
            } else if l.open && r.validity == Validity::NoLonger {
                // Double rail: it no longer holds.
                prims.push(poly(&[[x0, cy - 1.5], [x1, cy - 1.5]], 1.0, line_c));
                prims.push(poly(&[[x0, cy + 1.5], [x1, cy + 1.5]], 1.0, line_c));
            } else {
                prims.push(poly(&[[x0, cy], [x1, cy]], lw, line_c));
            }
        }
        if self.inside(l.start) {
            self.birth(&mut prims, xb, cy, rad, r.direction, mark_c);
        }
        if l.end <= self.b {
            let ec = if l.open { mark_c } else { line_c };
            self.end(&mut prims, xe, cy, rad, r.outcome, ec);
        }
        // The label: what happened and how it stands. The selected story's above its
        // line; a full lane's after its end mark or, when there is more room on its life,
        // over the line (on a background patch).
        if above || full {
            let mut s = format!("{} · {}", r.face.event, r.face.validity_phrase);
            if r.coin_flip {
                s.push_str(" · ");
                s.push_str(words(self.lang).coin);
            }
            let c = if role == Role::Closed {
                self.theme.closed
            } else {
                self.theme.text
            };
            let on_x = if self.inside(l.start) {
                xb + rad + 3.0
            } else {
                left + 2.0
            };
            let stop = next.unwrap_or(right).min(right) - 4.0;
            let lp = self.label_px();
            #[allow(clippy::cast_possible_truncation)]
            if above {
                if let Some(t) = fit(&s, stop - on_x, lp, self.adv()) {
                    let base = top + (lp.ceil() as i32) + 1;
                    prims.push(self.text(on_x, base, t, c));
                }
            } else {
                let on_room = if l.end <= self.b {
                    xe - rad - 3.0
                } else {
                    right - 2.0
                } - on_x;
                let after_x = xe + rad + 3.0;
                let after_room = if l.end <= self.b { stop - after_x } else { 0.0 };
                let base = (cy + lp * 0.36).round() as i32;
                if after_room >= on_room {
                    if let Some(t) = fit(&s, after_room, lp, self.adv()) {
                        prims.push(self.text(after_x, base, t, c));
                    }
                } else if let Some(t) = fit(&s, on_room, lp, self.adv()) {
                    #[allow(clippy::cast_precision_loss)]
                    let tw = (t.chars().count() as f64 * lp * self.adv()).ceil();
                    prims.push(rect(on_x - 1.0, cy - lp * 0.5, tw + 3.0, lp, self.theme.bg));
                    prims.push(self.text(on_x, base, t, c));
                }
            }
        }
        #[allow(clippy::cast_possible_truncation)]
        let hit = PixelRect {
            left: ((x0 - rad - 2.0).floor() as i32).max(self.rc.left),
            top,
            right: ((x1 + rad + 3.0).ceil() as i32).min(self.rc.right),
            bottom: (top + pitch).min(self.rc.bottom),
        };
        let lane_ix = f64::from(top - self.rc.top - HEADER_PX) / f64::from(pitch.max(1));
        #[allow(clippy::cast_precision_loss)]
        let values = BTreeMap::from([
            ("born_bar".to_owned(), l.start as f64),
            ("end_bar".to_owned(), l.end as f64),
            ("lane".to_owned(), lane_ix.floor()),
            ("open".to_owned(), if l.open { 1.0 } else { 0.0 }),
            ("tier".to_owned(), f64::from(r.tier)),
        ]);
        let mut label = format!("{} · {}", r.face.event, r.face.validity_phrase);
        if r.coin_flip {
            label.push_str(" · ");
            label.push_str(words(self.lang).coin);
        }
        SceneLayer {
            id: format!("{STORY_LAYER}{}", r.id),
            mark: Mark::Rule,
            data: DataSource::Finding {
                finding: r.finding.clone(),
            },
            fields: vec!["born_bar".to_owned(), "end_bar".to_owned()],
            constants: BTreeMap::new(),
            label: Some(label),
            items: vec![Item {
                bar: Some(l.start),
                t: Some(r.born_t),
                values,
                hit,
                bars: None,
            }],
            prims,
            skipped: 0,
        }
    }

    /// The summary: births per bucket of bars, stacked bottom-up by outcome (realized:
    /// solid; invalidated: outline; expired or retracted: hatched; open: solid in the
    /// "holds" colour). One layer per bucket (`story-cluster:<from>-<to>`), data: its
    /// newest story (the selected one if it is there).
    #[allow(clippy::too_many_lines)]
    fn histogram(
        &self,
        lives: &[Life<'_>],
        top: i32,
        bottom: i32,
        selected: &str,
    ) -> Vec<SceneLayer> {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let k = ((BUCKET_PX / self.x.slot).ceil().max(1.0)) as u64;
        // bucket index → lives
        let mut buckets: BTreeMap<u64, Vec<&Life<'_>>> = BTreeMap::new();
        for l in lives {
            buckets.entry(l.start.max(self.a) / k).or_default().push(l);
        }
        let max = buckets.values().map(Vec::len).max().unwrap_or(1).max(1);
        let h = f64::from((bottom - top).max(4));
        let mut out = Vec::with_capacity(buckets.len());
        let (left, right) = (f64::from(self.rc.left), f64::from(self.rc.right));
        for (j, ls) in &buckets {
            let (from, to) = (j * k, j * k + k - 1);
            let (va, vb) = (from.max(self.a), to.min(self.b));
            #[allow(clippy::cast_precision_loss)]
            let xa = (self.x.left + (va - self.x.from_bar) as f64 * self.x.slot).max(left);
            #[allow(clippy::cast_precision_loss)]
            let xb = (self.x.left + (vb - self.x.from_bar + 1) as f64 * self.x.slot).min(right);
            let (bx, bw) = (xa.floor() + 1.0, (xb.floor() - xa.floor() - 2.0).max(1.0));
            let count = |pred: &dyn Fn(&Life<'_>) -> bool| ls.iter().filter(|l| pred(l)).count();
            let real = count(&|l| l.row.outcome == Some(StoryOutcome::Realized));
            let inv = count(&|l| l.row.outcome == Some(StoryOutcome::Invalidated));
            let exp = count(&|l| {
                matches!(
                    l.row.outcome,
                    Some(StoryOutcome::Expired | StoryOutcome::Retracted)
                )
            });
            let open = count(&|l| l.open);
            let n = ls.len();
            #[allow(clippy::cast_precision_loss)]
            let bh = (h * n as f64 / max as f64).round().max(2.0);
            let base = f64::from(bottom);
            let mut prims = Vec::new();
            let mut cum = 0usize;
            let seg = |c: usize, cum: usize| {
                #[allow(clippy::cast_precision_loss)]
                let y1 = base - (bh * cum as f64 / n as f64).round();
                #[allow(clippy::cast_precision_loss)]
                let y0 = base - (bh * (cum + c) as f64 / n as f64).round();
                (y0, (y1 - y0).max(1.0))
            };
            let closed = self.theme.closed;
            if real > 0 {
                let (y0, sh) = seg(real, cum);
                prims.push(rect(bx, y0, bw, sh, closed));
                cum += real;
            }
            if inv > 0 {
                let (y0, sh) = seg(inv, cum);
                prims.push(rect(bx, y0, bw, 1.0, closed));
                prims.push(rect(bx, y0 + sh - 1.0, bw, 1.0, closed));
                prims.push(rect(bx, y0, 1.0, sh, closed));
                prims.push(rect(bx + bw - 1.0, y0, 1.0, sh, closed));
                cum += inv;
            }
            if exp > 0 {
                let (y0, sh) = seg(exp, cum);
                let mut yy = y0;
                while yy < y0 + sh {
                    prims.push(rect(bx, yy, bw, 1.0, closed));
                    yy += 2.0;
                }
                cum += exp;
            }
            if open > 0 {
                let (y0, sh) = seg(open, cum);
                prims.push(rect(bx, y0, bw, sh, self.theme.valid));
            }
            let rep = ls
                .iter()
                .find(|l| l.row.finding == selected)
                .or_else(|| {
                    ls.iter()
                        .max_by(|x, y| (x.start, &x.row.id).cmp(&(y.start, &y.row.id)))
                })
                .map_or_else(String::new, |l| l.row.finding.clone());
            #[allow(clippy::cast_precision_loss)]
            let values = BTreeMap::from([
                ("count".to_owned(), n as f64),
                ("realized".to_owned(), real as f64),
                ("invalidated".to_owned(), inv as f64),
                ("expired".to_owned(), exp as f64),
                ("open".to_owned(), open as f64),
                ("from_bar".to_owned(), from as f64),
                ("to_bar".to_owned(), to as f64),
            ]);
            #[allow(clippy::cast_possible_truncation)]
            let hit = PixelRect {
                left: xa.floor() as i32,
                top,
                right: (xb.floor() as i32).max(xa.floor() as i32 + 1),
                bottom: bottom + 2,
            };
            out.push(SceneLayer {
                id: format!("{CLUSTER_LAYER}{from}-{to}"),
                mark: Mark::Bar,
                data: DataSource::Finding { finding: rep },
                fields: vec!["count".to_owned()],
                constants: BTreeMap::new(),
                label: Some(format!("{n} {}", words(self.lang).stories)),
                items: vec![Item {
                    bar: Some(va),
                    t: None,
                    values,
                    hit,
                    bars: Some(vb - va + 1),
                }],
                prims,
                skipped: 0,
            });
        }
        out
    }

    /// The header row: open and closed counts, and the control that hides or shows the
    /// closed stories (the whole row is its target).
    fn header(&self, live: usize, closed: usize, hidden: usize, shown: bool) -> SceneLayer {
        let w = words(self.lang);
        let mut s = format!("{} {live} · {} {closed}", w.live, w.closed);
        if hidden > 0 {
            s.push(' ');
            s.push_str(w.hidden);
        }
        let base = self.rc.top + 12;
        let mut prims = vec![self.text(f64::from(self.rc.left) + 2.0, base, s, self.theme.muted)];
        let mut items = Vec::new();
        if closed > 0 {
            let t = if shown { w.hide } else { w.show };
            #[allow(clippy::cast_precision_loss)]
            let tw = (t.chars().count() as f64 * self.label_px() * self.adv()).ceil();
            let x1 = f64::from(self.rc.right) - 3.0;
            let x0 = x1 - tw - 10.0;
            let (y0, ch) = (f64::from(self.rc.top) + 1.0, f64::from(HEADER_PX) - 2.0);
            let c = self.theme.muted;
            prims.push(rect(x0, y0, x1 - x0, 1.0, c));
            prims.push(rect(x0, y0 + ch - 1.0, x1 - x0, 1.0, c));
            prims.push(rect(x0, y0, 1.0, ch, c));
            prims.push(rect(x1 - 1.0, y0, 1.0, ch, c));
            prims.push(self.text(x0 + 5.0, base, t.to_owned(), self.theme.text));
            #[allow(clippy::cast_precision_loss)]
            items.push(Item {
                bar: None,
                t: None,
                values: BTreeMap::from([
                    ("closed".to_owned(), closed as f64),
                    ("shown".to_owned(), if shown { 1.0 } else { 0.0 }),
                ]),
                hit: PixelRect {
                    left: self.rc.left,
                    top: self.rc.top,
                    right: self.rc.right,
                    bottom: self.rc.top + HEADER_PX,
                },
                bars: None,
            });
        }
        SceneLayer {
            id: TOGGLE_LAYER.to_owned(),
            mark: Mark::Text,
            data: DataSource::Constant {},
            fields: vec!["closed".to_owned(), "shown".to_owned()],
            constants: BTreeMap::new(),
            label: None,
            items,
            prims,
            skipped: 0,
        }
    }
}
