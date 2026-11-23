//! (R5/m18-pavka-perf) The story chart `pavka`'s `extend` exactly as it was before the
//! performance change (`r5/m18-pavka` d62207a, `crates/qp-grammar/src/stories/pavka.rs`),
//! kept verbatim (only the paths are the crate's public ones and the unused registry
//! items are left out) so that `story_pavka_equal.rs` can compare the scenes it draws with
//! the current code's byte for byte. Never edited.
#![allow(dead_code, clippy::all, clippy::pedantic)]

use std::collections::BTreeMap;

use qp_schema::capsule::{Direction, Validity};
use qp_schema::grammar::{Mark, Origin, ScaleKind};
use qp_schema::protocol::{Language, StoryOutcome, StoryRow};
use qp_schema::scene::{
    Align, DataSource, Item, PixelRect, Prim, ResolvedConstant, Rgba, SceneDesc, SceneLayer,
    ScenePanel, YMap,
};

use qp_designs::stories::StoryInput;
use qp_grammar::scale::slot_x;
use qp_look::look::Look;
use qp_look::theme::Theme;

/// Share of the frame height the price context strip takes.
pub const PRICE_SHARE: f64 = 0.38;
/// Smallest price strip (px).
pub const PRICE_MIN_H: u32 = 150;
/// Smallest story part (px).
pub const STORY_MIN_H: u32 = 120;
/// Height of one lane (px; `r4` `space.scale[5]` − 4).
pub const LANE_H: i32 = 20;
/// Smallest lane height before stories are clustered (px).
pub const LANE_MIN: i32 = 14;
/// Largest lane height, when few lanes leave room (px; a larger touch target).
pub const LANE_MAX: i32 = 28;
/// Width of a cluster's time bucket (px).
pub const CLUSTER_W: i32 = 36;
/// Height of the cluster row (px).
pub const CLUSTER_H: i32 = 24;
/// Smallest width of a story's hit box (px), centred on its span.
pub const HIT_MIN: i32 = 24;
/// Alpha of the selected story's row tint (`focus` role).
pub const FOCUS_ROW_ALPHA: u8 = 40;
/// Alpha of the selected story's guide through the price strip (`focus` role).
pub const GUIDE_ALPHA: u8 = 110;
/// Lane thickness of the selected story (px).
pub const BAR_FOCUS: i32 = 6;
/// Lane thickness of a live story (px).
pub const BAR_LIVE: i32 = 4;
/// Lane thickness of a closed story, or a live one that no longer holds (px).
pub const BAR_CLOSED: i32 = 2;
/// Half size of a birth or outcome marker (px).
pub const MARK: f32 = 5.0;
/// Dash and gap of a coin-flip lane (px).
pub const DASH: (i32, i32) = (5, 3);
/// Most dashes in one coin-flip lane (a longer lane gets longer dashes).
pub const DASH_MAX: i32 = 12;

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
    live: bool,
    focus: bool,
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

/// Width estimate of `s` in `role` (the compiler's rule: characters × advance × size).
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap
)]
fn text_w(look: &Look, role: &str, s: &str) -> i32 {
    let px = look.types.role(role).map_or(11.0, |r| r.px);
    let adv = look.role_font(role).advance;
    (s.chars().count() as f64 * adv * px).ceil() as i32
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
    let adv = look.role_font(role).advance;
    let n = (f64::from(w.max(0)) / (adv * px)).floor() as usize;
    let mut out: String = s.chars().take(n.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// The small shape of `s` centred at (`cx`, `cy`), half size `r`, in `color` (the lane's
/// end cap, and the summary's legend).
fn cap(out: &mut Vec<Prim>, s: Shape, cx: f32, cy: f32, r: f32, color: Rgba, bg: Rgba) {
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
                let b = (a + DASH.0).min(end);
                out.extend(rect(a, y, b, y + 2, color));
                a = b + DASH.1;
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

/// The story part of the scene; see the module documentation.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn extend(
    look: &Look,
    theme_name: Option<&str>,
    mut scene: SceneDesc,
    h: u32,
    input: &StoryInput<'_>,
) -> SceneDesc {
    if h <= scene.height {
        return scene;
    }
    let theme: Theme = look.theme(theme_name).unwrap_or(look.dark);
    let w = scene.width.cast_signed();
    let full_h = h.cast_signed();
    let [m_left, m_right, _, _] = look.space.plot;
    let (left, right) = (m_left, w - m_right);
    let top0 = scene.height.cast_signed();
    let x = scene.x;
    let last = scene.bars.saturating_sub(1);
    let wd = words(input.lang);
    let label_px = look.types.label.px;
    let body_px = look.types.body.px;
    #[allow(clippy::cast_possible_truncation)]
    let (lp, bp) = (label_px.ceil() as i32, body_px.ceil() as i32);
    let line1 = top0 + 6 + lp;
    let line2 = line1 + 6 + bp;
    let line3 = line2 + 4 + lp;
    let lanes_top = line3 + 8;
    // A lane's pixel ends: birth column to closing column, at least two markers apart (a
    // story closed a bar after its birth still shows both shapes).
    #[allow(clippy::cast_possible_truncation)]
    let gap = 2 * (MARK.ceil() as i32) + 3;
    let ends = |born: u64, end: u64| -> (i32, i32) {
        #[allow(clippy::cast_possible_truncation)]
        let (a, b) = (
            slot_x(&x, born.max(x.from_bar)) as i32,
            slot_x(&x, end.min(x.to_bar)) as i32,
        );
        if born >= x.from_bar && end <= x.to_bar {
            (a, b.max(a + gap).min(right))
        } else {
            (a, b)
        }
    };
    let bottom = full_h - 6;

    // Visible stories (life meets the range), counted before the K54 filter.
    let mut vis: Vec<Placed<'_>> = Vec::new();
    let (mut n_live, mut n_held, mut n_failed, mut n_coin, mut n_all) = (0, 0, 0, 0, 0);
    for r in input.rows {
        let live = r.closed_bar.is_none();
        let end = r.closed_bar.unwrap_or_else(|| r.last_bar.max(last));
        if r.born_bar > x.to_bar || end < x.from_bar {
            continue;
        }
        n_all += 1;
        match r.outcome {
            None => n_live += 1,
            Some(StoryOutcome::Realized) => n_held += 1,
            Some(StoryOutcome::Invalidated | StoryOutcome::Expired) => n_failed += 1,
            Some(StoryOutcome::Retracted) => {}
        }
        if r.coin_flip {
            n_coin += 1;
        }
        let focus = input.focus == Some(r.finding.as_str());
        if input.hide_closed && !live && !focus {
            continue;
        }
        let (a, b) = ends(r.born_bar, end);
        #[allow(clippy::cast_possible_truncation)]
        let m = MARK.ceil() as i32;
        let tail = if live { m + 4 } else { m + 1 };
        vis.push(Placed {
            row: r,
            end,
            span: (a - m - 1, b + tail),
            live,
            focus,
        });
    }
    // Placement order: the selected story, live ones (newest first), closed ones (most
    // recently closed first).
    let mut order: Vec<usize> = (0..vis.len()).collect();
    order.sort_by(|&i, &j| {
        let (a, b) = (&vis[i], &vis[j]);
        b.focus
            .cmp(&a.focus)
            .then(b.live.cmp(&a.live))
            .then(b.end.cmp(&a.end))
            .then(b.row.born_bar.cmp(&a.row.born_bar))
            .then(a.row.id.cmp(&b.row.id))
    });
    // First fit, uncapped: lanes are opened in placement order and a story never moves
    // into a later lane, so a capacity of `c` lanes is the first `c` lanes of this result
    // (the rest are clustered). Each lane keeps its spans sorted by start (overlap test
    // by binary search). Live stories keep a lane each.
    let mut lanes: Vec<(bool, Vec<usize>)> = Vec::new();
    let mut spans: Vec<Vec<(i32, i32)>> = Vec::new();
    for &i in &order {
        let p = &vis[i];
        let (s0, s1) = p.span;
        let mut placed = false;
        if !p.live {
            for (l, sp) in lanes.iter_mut().zip(spans.iter_mut()) {
                if l.0 {
                    continue;
                }
                let at = sp.partition_point(|q| q.0 < s0);
                let left_ok = at == 0 || sp[at - 1].1 + 4 <= s0;
                let right_ok = at == sp.len() || s1 + 4 <= sp[at].0;
                if left_ok && right_ok {
                    sp.insert(at, (s0, s1));
                    l.1.push(i);
                    placed = true;
                    break;
                }
            }
        }
        if !placed {
            lanes.push((p.live, vec![i]));
            spans.push(vec![(s0, s1)]);
        }
    }
    let avail = bottom - lanes_top;
    // Thin out first (lanes down to LANE_MIN), then cluster what still does not fit.
    let n = i32::try_from(lanes.len()).unwrap_or(i32::MAX).max(1);
    let mut over: Vec<usize> = Vec::new();
    let lane_h = if n <= avail / LANE_H {
        (avail / n).clamp(LANE_H, LANE_MAX)
    } else if n <= avail / LANE_MIN {
        (avail / n).clamp(LANE_MIN, LANE_H)
    } else {
        let cap = usize::try_from(((avail - CLUSTER_H) / LANE_MIN).max(1)).unwrap_or(1);
        for l in lanes.drain(cap.min(lanes.len())..) {
            over.extend(l.1);
        }
        LANE_MIN
    };
    // Live lanes on top, then closed (each group in placement order).
    lanes.sort_by_key(|l| !l.0);
    let n_lanes = i32::try_from(lanes.len()).unwrap_or(0);

    let mut layers: Vec<SceneLayer> = Vec::new();
    let mut under: Vec<Prim> = Vec::new();

    // Summary line: counts, each with its shape (the legend).
    let mut sum: Vec<Prim> = Vec::new();
    let toggle_x = right + 4;
    let limit = toggle_x - 8;
    let mut cx = left;
    let first = (wd.stories)(n_all);
    sum.push(text(look, "label", cx, line1, first.clone(), theme.text));
    cx += text_w(look, "label", &first);
    #[allow(clippy::cast_precision_loss)]
    let ly = (line1 - lp / 2 + 1) as f32;
    let mut segs: Vec<(Option<Shape>, String)> =
        vec![(Some(Shape::Live), format!("{n_live} {}", wd.live))];
    if input.hide_closed {
        segs.push((None, wd.hidden.to_owned()));
    } else {
        segs.push((Some(Shape::Realized), format!("{n_held} {}", wd.held)));
        segs.push((Some(Shape::Failed), format!("{n_failed} {}", wd.failed)));
    }
    if n_coin > 0 {
        segs.push((Some(Shape::Coin), format!("{n_coin} {}", wd.coin)));
    }
    for (s, t) in segs {
        let need = 8 + if s.is_some() { 11 } else { 0 } + text_w(look, "label", &t);
        if cx + need > limit {
            break;
        }
        cx += 8;
        if let Some(s) = s {
            #[allow(clippy::cast_precision_loss)]
            cap(&mut sum, s, (cx + 4) as f32, ly, 4.0, theme.text, theme.bg);
            cx += 11;
        }
        sum.push(text(look, "label", cx, line1, t.clone(), theme.muted));
        cx += text_w(look, "label", &t);
    }
    // Focus line: the selected story in words.
    if let Some(f) = vis.iter().find(|p| p.focus) {
        let fc = &f.row.face;
        let head = format!("{} · {}", fc.event, fc.when);
        let mut more = format!("{} · {}", fc.validity_phrase, fc.ledger_short);
        if f.row.coin_flip {
            more.push_str(" · ");
            more.push_str(wd.coin);
        }
        sum.extend(rect(left, line2 - bp + 2, left + 6, line3, theme.focus));
        let room = w - left - 8 - 10;
        sum.push(text(
            look,
            "body",
            left + 10,
            line2,
            fit(look, "body", &head, room),
            theme.text,
        ));
        sum.push(text(
            look,
            "label",
            left + 10,
            line3,
            fit(look, "label", &more, room),
            theme.muted,
        ));
    }
    layers.push(SceneLayer {
        id: SUMMARY_LAYER.to_owned(),
        mark: Mark::Text,
        data: DataSource::Constant {},
        fields: Vec::new(),
        constants: BTreeMap::new(),
        label: None,
        items: Vec::new(),
        prims: sum,
        skipped: 0,
    });

    // The closed-stories checkbox (K54), in the right margin of the summary line.
    {
        let mut prims = Vec::new();
        let (bx, by) = (toggle_x, line1 - lp + 1);
        prims.extend(rect(bx, by, bx + 10, by + 10, theme.muted));
        if input.hide_closed {
            prims.extend(rect(bx + 2, by + 2, bx + 8, by + 8, theme.bg));
        }
        prims.push(text(
            look,
            "label",
            bx + 14,
            line1,
            fit(look, "label", wd.closed, w - bx - 14),
            theme.muted,
        ));
        layers.push(SceneLayer {
            id: TOGGLE_LAYER.to_owned(),
            mark: Mark::Text,
            data: DataSource::Constant {},
            fields: Vec::new(),
            constants: BTreeMap::new(),
            label: Some(wd.closed.to_owned()),
            items: vec![Item {
                bar: None,
                t: None,
                values: BTreeMap::from([(
                    "hide_closed".to_owned(),
                    if input.hide_closed { 1.0 } else { 0.0 },
                )]),
                hit: PixelRect {
                    left: toggle_x - 4,
                    top: top0,
                    right: w,
                    bottom: line1 + 8,
                },
                bars: None,
            }],
            prims,
            skipped: 0,
        });
    }

    // Lanes.
    let mut focus_layer: Option<SceneLayer> = None;
    let mut live_end = None;
    for (li, (is_live, members)) in lanes.iter().enumerate() {
        let lt = lanes_top + i32::try_from(li).unwrap_or(0) * lane_h;
        let cy_i = lt + lane_h / 2;
        #[allow(clippy::cast_precision_loss)]
        let cy = cy_i as f32;
        if *is_live {
            live_end = Some(lt + lane_h);
            let t = fit(look, "label", wd.live, w - right - 6);
            layers.push(SceneLayer {
                id: format!("story-live:{li}"),
                mark: Mark::Text,
                data: DataSource::Constant {},
                fields: Vec::new(),
                constants: BTreeMap::new(),
                label: None,
                items: Vec::new(),
                prims: vec![text(
                    look,
                    "label",
                    right + 6,
                    cy_i + lp / 2 - 1,
                    t,
                    theme.valid,
                )],
                skipped: 0,
            });
        }
        for &i in members {
            let p = &vis[i];
            let r = p.row;
            let no_longer = p.live && r.validity == Validity::NoLonger;
            let color = if p.focus {
                theme.focus
            } else if p.live && !no_longer {
                theme.valid
            } else if p.live {
                theme.invalid
            } else {
                theme.closed
            };
            let thick = if p.focus {
                if p.live {
                    BAR_FOCUS
                } else {
                    BAR_LIVE
                }
            } else if p.live && !no_longer {
                BAR_LIVE
            } else {
                BAR_CLOSED
            };
            let mut prims = Vec::new();
            let (a, b) = ends(r.born_bar, p.end);
            let (y0, y1) = (cy_i - thick / 2, cy_i - thick / 2 + thick);
            if r.coin_flip {
                // At most DASH_MAX dashes: a long lane gets longer dashes, same gap.
                let len = b.max(a + 1) - a;
                let dash = DASH.0.max(len / DASH_MAX - DASH.1);
                let mut s = a;
                while s < b.max(a + 1) {
                    let e = (s + dash).min(b.max(a + 1));
                    prims.extend(rect(s, y0, e, y1, color));
                    s = e + DASH.1;
                }
            } else {
                prims.extend(rect(a, y0, b.max(a + 1), y1, color));
            }
            let m = if p.focus { MARK * 1.3 } else { MARK };
            #[allow(clippy::cast_precision_loss)]
            let (fa, fb) = (a as f32, b as f32);
            if r.born_bar >= x.from_bar {
                let pts = match r.direction {
                    Direction::Up => {
                        vec![(fa, cy - m), (fa + m, cy + m * 0.8), (fa - m, cy + m * 0.8)]
                    }
                    Direction::Down => {
                        vec![(fa, cy + m), (fa + m, cy - m * 0.8), (fa - m, cy - m * 0.8)]
                    }
                    Direction::Neutral => {
                        vec![(fa, cy - m), (fa + m, cy), (fa, cy + m), (fa - m, cy)]
                    }
                };
                prims.push(fill(&pts, color));
            } else {
                prims.push(fill(
                    &[(fa - 4.0, cy), (fa + 2.0, cy - 4.0), (fa + 2.0, cy + 4.0)],
                    color,
                ));
            }
            if p.end <= x.to_bar {
                if p.live {
                    cap(&mut prims, Shape::Live, fb + 4.0, cy, m, color, theme.bg);
                    if no_longer {
                        prims.push(Prim::Circle {
                            cx: fb + 3.0,
                            cy,
                            r: 1.5,
                            color: theme.bg,
                        });
                    }
                } else {
                    cap(&mut prims, outcome_shape(r), fb, cy, m, color, theme.bg);
                }
            } else {
                prims.push(fill(
                    &[(fb + 4.0, cy), (fb - 2.0, cy - 4.0), (fb - 2.0, cy + 4.0)],
                    color,
                ));
            }
            // Hit box: the lane over the span, at least HIT_MIN wide.
            let (mut h0, mut h1) = p.span;
            if h1 - h0 < HIT_MIN {
                let c = (h0 + h1) / 2;
                h0 = c - HIT_MIN / 2;
                h1 = h0 + HIT_MIN;
            }
            let layer = SceneLayer {
                id: format!("{STORY_LAYER}{}", r.id),
                mark: Mark::Glyph,
                data: DataSource::Finding {
                    finding: r.finding.clone(),
                },
                fields: Vec::new(),
                constants: BTreeMap::new(),
                label: Some(format!("{} · {}", r.face.event, r.face.validity_phrase)),
                items: vec![Item {
                    bar: Some(r.born_bar),
                    t: Some(r.born_t),
                    #[allow(clippy::cast_precision_loss)]
                    values: BTreeMap::from([
                        ("end_bar".to_owned(), p.end as f64),
                        ("lane".to_owned(), li as f64),
                    ]),
                    hit: PixelRect {
                        left: h0.max(left - 4),
                        top: lt,
                        right: h1.min(w),
                        bottom: lt + lane_h,
                    },
                    bars: None,
                }],
                prims,
                skipped: 0,
            };
            if p.focus {
                under.extend(rect(
                    left,
                    lt,
                    right,
                    lt + lane_h,
                    with_alpha(theme.focus, FOCUS_ROW_ALPHA),
                ));
                if r.born_bar >= x.from_bar {
                    let price_top = scene.panels.first().map_or(0, |pp| pp.rect.top);
                    under.extend(rect(
                        a,
                        price_top,
                        a + 1,
                        cy_i,
                        with_alpha(theme.focus, GUIDE_ALPHA),
                    ));
                }
                focus_layer = Some(layer);
            } else {
                layers.push(layer);
            }
        }
    }
    // A hairline between the live lanes and the closed ones.
    if let Some(y) = live_end {
        if lanes.iter().any(|l| !l.0) {
            under.extend(rect(left, y, right, y + 1, theme.line));
        }
    }

    // Clusters: the closed stories without a lane, by time bucket.
    if !over.is_empty() {
        let ct = bottom - CLUSTER_H;
        let mut buckets: BTreeMap<i32, Vec<usize>> = BTreeMap::new();
        for &i in &over {
            let b = (vis[i].span.0.max(left) - left) / CLUSTER_W;
            buckets.entry(b).or_default().push(i);
        }
        for (k, (b, members)) in buckets.iter().enumerate() {
            let x0 = left + b * CLUSTER_W + 2;
            let x1 = (x0 + CLUSTER_W - 4).min(right);
            let from = members
                .iter()
                .map(|&i| vis[i].row.born_bar)
                .min()
                .unwrap_or(x.from_bar)
                .max(x.from_bar);
            let to = members
                .iter()
                .map(|&i| vis[i].end)
                .max()
                .unwrap_or(x.to_bar)
                .min(x.to_bar);
            let t = format!("+{}", members.len());
            let mut prims = Vec::new();
            prims.extend(rect(x0, ct + 3, x1, ct + CLUSTER_H - 3, theme.line));
            prims.extend(rect(x0, ct + 3, x0 + 2, ct + CLUSTER_H - 3, theme.closed));
            prims.push(text(
                look,
                "label",
                x0 + 5,
                ct + CLUSTER_H / 2 + lp / 2 - 1,
                t,
                theme.text,
            ));
            let mut constants = BTreeMap::new();
            for &i in members {
                let r = vis[i].row;
                if let Some(fact) = (input.fact_of)(&r.finding) {
                    #[allow(clippy::cast_precision_loss)]
                    constants.insert(
                        format!("/stories/{}", r.id),
                        ResolvedConstant {
                            origin: Origin::Fact {
                                fact,
                                field: "bar".to_owned(),
                            },
                            value: r.born_bar as f64,
                        },
                    );
                }
            }
            #[allow(clippy::cast_precision_loss)]
            layers.push(SceneLayer {
                id: format!("{CLUSTER_LAYER}{k}"),
                mark: Mark::Point,
                data: DataSource::Constant {},
                fields: Vec::new(),
                constants,
                label: Some(format!("+{}", members.len())),
                items: vec![Item {
                    bar: Some(from),
                    t: None,
                    values: BTreeMap::from([
                        ("from_bar".to_owned(), from as f64),
                        ("to_bar".to_owned(), to as f64),
                        ("stories".to_owned(), members.len() as f64),
                    ]),
                    hit: PixelRect {
                        left: x0 - 2,
                        top: ct,
                        right: x1 + 2,
                        bottom,
                    },
                    bars: None,
                }],
                prims,
                skipped: 0,
            });
        }
    }
    layers.extend(focus_layer);

    let id = scene.panels.iter().map(|p| p.id).max().map_or(0, |m| m + 1);
    #[allow(clippy::cast_precision_loss)]
    scene.panels.push(ScenePanel {
        id,
        name: Some(PANEL_NAME.to_owned()),
        // The whole width: the right margin holds the checkbox and the live words.
        rect: PixelRect {
            left,
            top: top0,
            right: w,
            bottom: full_h,
        },
        y: YMap {
            kind: ScaleKind::Linear,
            min: 0.0,
            max: f64::from(n_lanes.max(1)),
            top: f64::from(lanes_top),
            bottom: f64::from(lanes_top + n_lanes.max(1) * lane_h),
        },
        layers,
    });
    scene.under.extend(under);
    scene.height = h;
    scene
}
