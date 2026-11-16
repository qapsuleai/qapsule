//! Pixel work: turns a prepared layer into primitives and items for one panel.

use std::collections::BTreeMap;

use qp_schema::scene::{Item, PixelRect, Prim, Rgba, SceneLayer, XMap, YMap};

use qp_schema::grammar::PanelPos;

use crate::detmath::px;
use crate::glyph::{alphabet, GlyphBox};
use crate::layer::{step_index, Geom, PRow, Prepared, Split};
use crate::scale::value_y;
use crate::theme::Theme;

/// Pixel-aligned rectangle primitive (negative sizes clamp to 0), as R0 builds them.
pub(crate) fn rect(x: i32, y: i32, w: i32, h: i32, color: Rgba) -> Prim {
    Prim::Rect {
        x,
        y,
        w: w.max(0).unsigned_abs(),
        h: h.max(0).unsigned_abs(),
        color,
    }
}

/// Estimated advance width of `text` at `px` (no font in the compiler): `0.6 · px` per
/// character, rounded up. Used only to place legends and size text hit boxes.
pub(crate) fn est_width(text: &str, size: f64) -> i32 {
    let n = text.chars().count();
    #[allow(clippy::cast_precision_loss)]
    let w = (n as f64 * size * 0.6).ceil();
    #[allow(clippy::cast_possible_truncation)]
    let w = w as i32;
    w.max(1)
}

/// Candle body width for a slot (bucket) width: 70 %, at least 1, odd above 2 (R0).
pub(crate) fn body_width(slot: f64) -> i32 {
    #[allow(clippy::cast_possible_truncation)]
    let mut w = ((slot * 0.7).floor() as i32).max(1);
    if w > 2 && w % 2 == 0 {
        w -= 1;
    }
    w
}

/// What `draw` needs about the frame.
pub(crate) struct Geo<'a> {
    pub x: &'a XMap,
    pub y: &'a YMap,
    pub rect: PixelRect,
    /// Bars per bucket.
    pub k: u64,
    /// Highlighted layer: strokes 1 px thicker.
    pub bold: bool,
    /// Theme (glyph colours).
    pub theme: &'a Theme,
    /// (R4/M11) Outcome style of glyphs (`CompileOptions::outcome`).
    pub outcome: &'a dyn crate::glyph::outcome::OutcomeStyle,
}

impl Geo<'_> {
    /// Pixel column of the centre of `n` slots starting at `slot` (R0's `cx_of` for n = 1).
    fn cx(&self, slot: u64, n: u64) -> f64 {
        #[allow(clippy::cast_precision_loss)]
        let rel = (slot as f64 - self.x.from_bar as f64) + n as f64 / 2.0;
        (self.x.left + rel * self.x.slot).floor()
    }

    fn y(&self, v: Option<f64>) -> Option<f64> {
        let lim = crate::detmath::PX_LIMIT;
        value_y(self.y, v?)
            .filter(|y| !y.is_nan())
            .map(|y| y.clamp(-lim, lim))
    }

    /// Half width of a slot column (bucket of `k`) for hit boxes.
    fn half_slot(&self) -> i32 {
        #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
        let h = (self.x.slot * self.k as f64 / 2.0).floor() as i32;
        h.max(0)
    }

    fn bars(&self, n: u64) -> Option<u64> {
        (self.k > 1).then_some(n)
    }
}

fn hit_x(cx: i32, half: i32) -> (i32, i32) {
    (cx - half, cx + half + 1)
}

/// Colour breaks between two consecutive drawn rows: `(s, colour after)` with `s` in
/// `[0, 1]` along the segment (`docs/GRAMMAR.md` §7).
fn breaks(split: &Split, a: &PRow, b: &PRow) -> Vec<(f64, Rgba)> {
    if a.color == b.color {
        return Vec::new();
    }
    let mid = vec![(0.5, b.color)];
    match (split, a.ckey, b.ckey) {
        (Split::When, Some(ka), Some(kb)) if ka != kb => {
            vec![((ka / (ka - kb)).clamp(0.0, 1.0), b.color)]
        }
        (Split::Steps { thresholds, colors }, Some(ka), Some(kb)) if ka != kb => {
            let mut out = Vec::new();
            if kb > ka {
                for t in thresholds.iter().filter(|t| ka < **t && **t <= kb) {
                    out.push(((t - ka) / (kb - ka), colors[step_index(thresholds, *t)]));
                }
            } else {
                for t in thresholds.iter().rev().filter(|t| kb < **t && **t <= ka) {
                    let i = step_index(thresholds, *t);
                    out.push(((t - ka) / (kb - ka), colors[i.saturating_sub(1)]));
                }
            }
            if out.is_empty() {
                mid
            } else {
                out
            }
        }
        _ => mid,
    }
}

fn lerp(a: [f64; 2], b: [f64; 2], s: f64) -> [f64; 2] {
    [a[0] + (b[0] - a[0]) * s, a[1] + (b[1] - a[1]) * s]
}

#[allow(clippy::cast_possible_truncation)]
fn pt(p: [f64; 2]) -> [f32; 2] {
    [p[0] as f32, p[1] as f32]
}

/// Draws a prepared layer.
pub(crate) fn draw(pl: &Prepared, g: &Geo<'_>) -> SceneLayer {
    let mut out = SceneLayer {
        id: pl.id.clone(),
        mark: pl.mark,
        data: pl.data.clone(),
        fields: pl.fields.clone(),
        constants: pl.constants.clone(),
        label: pl.label.clone(),
        items: Vec::new(),
        prims: Vec::new(),
        skipped: pl.skipped,
    };
    match pl.geom {
        Geom::Candle => candles(pl, g, &mut out),
        Geom::Line => lines(pl, g, &mut out),
        Geom::Area | Geom::Band => areas(pl, g, &mut out),
        Geom::Bar => bars(pl, g, &mut out),
        Geom::RuleH | Geom::RuleV => rules(pl, g, &mut out),
        Geom::Text if pl.anchor.x.is_some() || pl.anchor.y.is_some() => {
            anchored_text(pl, g, &mut out);
        }
        Geom::Point | Geom::Text => points(pl, g, &mut out),
        Geom::Segment => segments(pl, g, &mut out),
        Geom::Glyph => glyphs(pl, g, &mut out),
    }
    out
}

fn values(keys: &[String], vals: &[f64]) -> BTreeMap<String, f64> {
    keys.iter().cloned().zip(vals.iter().copied()).collect()
}

fn candles(pl: &Prepared, g: &Geo<'_>, out: &mut SceneLayer) {
    #[allow(clippy::cast_precision_loss)]
    let body_w = body_width(g.x.slot * g.k as f64);
    for r in &pl.traces[0].rows {
        let (Some(yo), Some(yh), Some(yl), Some(yc)) =
            (g.y(r.v[0]), g.y(r.v[1]), g.y(r.v[2]), g.y(r.v[3]))
        else {
            out.skipped += 1;
            continue;
        };
        #[allow(clippy::cast_possible_truncation)]
        let cx = g.cx(r.slot, r.n) as i32;
        let (yh, yl) = (px(yh), px(yl));
        out.prims.push(rect(cx, yh, 1, (yl - yh).max(1), r.color));
        let (yo, yc) = (px(yo), px(yc));
        let top = yo.min(yc);
        let bh = (yo - yc).abs().max(1);
        out.prims
            .push(rect(cx - body_w / 2, top, body_w, bh, r.color));
        let (left, right) = hit_x(cx, (body_w / 2).max(1));
        let vals: Vec<f64> = r.v.iter().map(|v| v.unwrap_or(0.0)).collect();
        out.items.push(Item {
            bar: Some(r.slot),
            t: r.t,
            values: values(&pl.keys, &vals),
            hit: PixelRect {
                left,
                top: yh,
                right,
                bottom: (yh + (yl - yh).max(1)).max(top + bh),
            },
            bars: g.bars(r.n),
        });
    }
}

/// A row's drawn points for a line (1 point, or 4 for a LOD bucket), or `None` (gap).
fn line_points(g: &Geo<'_>, r: &PRow, lod: bool) -> Option<Vec<[f64; 2]>> {
    let x = g.cx(r.slot, r.n) + 0.5;
    if lod {
        let ys: Vec<f64> = r.v.iter().map(|v| g.y(*v)).collect::<Option<_>>()?;
        Some(ys.into_iter().map(|y| [x, y]).collect())
    } else {
        Some(vec![[x, g.y(r.v[0])?]])
    }
}

fn lines(pl: &Prepared, g: &Geo<'_>, out: &mut SceneLayer) {
    let width = pl.width + if g.bold { 1.0 } else { 0.0 };
    #[allow(clippy::cast_possible_truncation)]
    let w32 = width as f32;
    let flush = |pts: &mut Vec<[f64; 2]>, color: Rgba, prims: &mut Vec<Prim>| {
        match pts.len() {
            0 => {}
            1 => {
                let [x, y] = pt(pts[0]);
                prims.push(Prim::Circle {
                    cx: x,
                    cy: y,
                    r: (w32 / 2.0).max(1.0),
                    color,
                });
            }
            _ => prims.push(Prim::Polyline {
                points: pts.iter().map(|p| pt(*p)).collect(),
                width: w32,
                color,
            }),
        }
        pts.clear();
    };
    for tr in &pl.traces {
        let mut pts: Vec<[f64; 2]> = Vec::new();
        let mut color = [0u8; 4];
        let mut prev: Option<(&PRow, [f64; 2])> = None;
        for r in &tr.rows {
            let Some(rp) = line_points(g, r, tr.lod_line) else {
                // A present value that cannot be drawn on this scale (log, <= 0).
                if r.v[0].is_some() {
                    out.skipped += 1;
                }
                flush(&mut pts, color, &mut out.prims);
                prev = None;
                continue;
            };
            match prev {
                None => {
                    color = r.color;
                }
                Some((pr, last)) => {
                    for (s, c) in breaks(&tr.split, pr, r) {
                        let p = lerp(last, rp[0], s);
                        pts.push(p);
                        flush(&mut pts, color, &mut out.prims);
                        pts.push(p);
                        color = c;
                    }
                }
            }
            pts.extend_from_slice(&rp);
            prev = Some((r, rp[rp.len() - 1]));
        }
        flush(&mut pts, color, &mut out.prims);
    }
    // Items: one per (slot, output); rows of all traces are aligned by slot.
    let aligned = pl
        .traces
        .windows(2)
        .all(|w| w[0].rows.len() == w[1].rows.len());
    let n = if aligned {
        pl.traces.first().map_or(0, |t| t.rows.len())
    } else {
        0
    };
    let half = g.half_slot();
    let item_for = |rows: &[(&String, &PRow, bool)], out: &mut SceneLayer| {
        let mut vals = BTreeMap::new();
        let (mut ytop, mut ybot) = (f64::INFINITY, f64::NEG_INFINITY);
        let mut first = None;
        for (key, r, lod) in rows {
            let Some(ps) = line_points(g, r, *lod) else {
                continue;
            };
            for p in &ps {
                ytop = ytop.min(p[1]);
                ybot = ybot.max(p[1]);
            }
            if *lod {
                let lo = r.v.iter().flatten().copied().fold(f64::INFINITY, f64::min);
                let hi =
                    r.v.iter()
                        .flatten()
                        .copied()
                        .fold(f64::NEG_INFINITY, f64::max);
                vals.insert((*key).clone(), r.v[3].unwrap_or(0.0));
                vals.insert(format!("{key}.min"), lo);
                vals.insert(format!("{key}.max"), hi);
            } else {
                vals.insert((*key).clone(), r.v[0].unwrap_or(0.0));
            }
            first.get_or_insert(*r);
        }
        let Some(r) = first else { return };
        #[allow(clippy::cast_possible_truncation)]
        let cx = g.cx(r.slot, r.n) as i32;
        let (left, right) = hit_x(cx, half);
        out.items.push(Item {
            bar: Some(r.slot),
            t: r.t,
            values: vals,
            hit: PixelRect {
                left,
                top: px(ytop) - 4,
                right,
                bottom: px(ybot) + 5,
            },
            bars: g.bars(r.n),
        });
    };
    if aligned {
        // Slot-major, then output order: a multi-output line (e.g. bands drawn as one
        // line layer) yields one item per (slot, output), each with its own ±4 px box
        // (GRAMMAR §16), so a pixel between two traces hits nothing.
        for i in 0..n {
            for (t, k) in pl.traces.iter().zip(&pl.keys) {
                item_for(&[(k, &t.rows[i], t.lod_line)], out);
            }
        }
    } else {
        for (t, k) in pl.traces.iter().zip(&pl.keys) {
            for r in &t.rows {
                item_for(&[(k, r, t.lod_line)], out);
            }
        }
    }
}

fn areas(pl: &Prepared, g: &Geo<'_>, out: &mut SceneLayer) {
    let tr = &pl.traces[0];
    let edge_w = pl.width + if g.bold { 1.0 } else { 0.0 };
    #[allow(clippy::cast_possible_truncation)]
    let edge_w = edge_w as f32;
    let floor_y = f64::from(g.rect.bottom);
    let edge = pl.edge || g.bold;
    let flush =
        |top: &mut Vec<[f64; 2]>, bot: &mut Vec<[f64; 2]>, color: Rgba, prims: &mut Vec<Prim>| {
            if top.len() >= 2 {
                let mut poly: Vec<[f32; 2]> = top.iter().map(|p| pt(*p)).collect();
                poly.extend(bot.iter().rev().map(|p| pt(*p)));
                prims.push(Prim::Fill {
                    points: poly,
                    color,
                });
                if edge {
                    let ec = [color[0], color[1], color[2], 255];
                    prims.push(Prim::Polyline {
                        points: top.iter().map(|p| pt(*p)).collect(),
                        width: edge_w,
                        color: ec,
                    });
                    if pl.geom == Geom::Band {
                        prims.push(Prim::Polyline {
                            points: bot.iter().map(|p| pt(*p)).collect(),
                            width: edge_w,
                            color: ec,
                        });
                    }
                }
            }
            top.clear();
            bot.clear();
        };
    let half = g.half_slot();
    let (mut top, mut bot) = (Vec::new(), Vec::new());
    let mut color = [0u8; 4];
    let mut prev: Option<(&PRow, [f64; 2], [f64; 2])> = None;
    for r in &tr.rows {
        let x = g.cx(r.slot, r.n) + 0.5;
        let ya = g.y(r.v[0]);
        let yb = match r.v[1] {
            Some(_) => g.y(r.v[1]),
            None if pl.geom == Geom::Area && pl.keys.len() == 1 => Some(floor_y),
            None => None,
        };
        let (Some(ya), Some(yb)) = (ya, yb) else {
            if r.v[0].is_some() {
                out.skipped += 1;
            }
            flush(&mut top, &mut bot, color, &mut out.prims);
            prev = None;
            continue;
        };
        let (pa, pb) = ([x, ya], [x, yb]);
        match prev {
            None => color = r.color,
            Some((pr, la, lb)) => {
                for (s, c) in breaks(&tr.split, pr, r) {
                    let (sa, sb) = (lerp(la, pa, s), lerp(lb, pb, s));
                    top.push(sa);
                    bot.push(sb);
                    flush(&mut top, &mut bot, color, &mut out.prims);
                    top.push(sa);
                    bot.push(sb);
                    color = c;
                }
            }
        }
        top.push(pa);
        bot.push(pb);
        prev = Some((r, pa, pb));
        #[allow(clippy::cast_possible_truncation)]
        let cx = (x - 0.5) as i32;
        let (left, right) = hit_x(cx, half);
        let (ry, ry2) = (px(ya), px(yb));
        let mut vals = vec![r.v[0].unwrap_or(0.0)];
        if let Some(b) = r.v[1] {
            vals.push(b);
        }
        out.items.push(Item {
            bar: Some(r.slot),
            t: r.t,
            values: values(&pl.keys, &vals),
            hit: PixelRect {
                left,
                top: ry.min(ry2),
                right,
                bottom: ry.max(ry2) + 1,
            },
            bars: g.bars(r.n),
        });
    }
    flush(&mut top, &mut bot, color, &mut out.prims);
}

fn bars(pl: &Prepared, g: &Geo<'_>, out: &mut SceneLayer) {
    #[allow(clippy::cast_precision_loss)]
    let bw = body_width(g.x.slot * g.k as f64);
    for r in &pl.traces[0].rows {
        let Some(yv) = g.y(r.v[0]) else {
            out.skipped += 1;
            continue;
        };
        let yv = px(yv);
        let yb = g
            .y(r.v[1])
            .map_or(g.rect.bottom, px)
            .clamp(g.rect.top, g.rect.bottom);
        #[allow(clippy::cast_possible_truncation)]
        let cx = g.cx(r.slot, r.n) as i32;
        let top = yv.min(yb);
        let h = (yv - yb).abs().max(1);
        out.prims.push(rect(cx - bw / 2, top, bw, h, r.color));
        let (left, right) = hit_x(cx, (bw / 2).max(1));
        let vals = [r.v[0].unwrap_or(0.0), r.v[1].unwrap_or(0.0)];
        out.items.push(Item {
            bar: Some(r.slot),
            t: r.t,
            values: values(&pl.keys, &vals),
            hit: PixelRect {
                left,
                top,
                right,
                bottom: top + h,
            },
            bars: g.bars(r.n),
        });
    }
}

fn rules(pl: &Prepared, g: &Geo<'_>, out: &mut SceneLayer) {
    let w = pl.width + if g.bold { 1.0 } else { 0.0 };
    let w = px(w).max(1);
    let rc = g.rect;
    for r in &pl.traces[0].rows {
        if pl.geom == Geom::RuleH {
            let Some(y) = g.y(r.v[0]) else {
                out.skipped += 1;
                continue;
            };
            let ry = px(y);
            out.prims.push(rect(
                rc.left,
                ry - (w - 1) / 2,
                rc.right - rc.left,
                w,
                r.color,
            ));
            out.items.push(Item {
                bar: None,
                t: None,
                values: values(&pl.keys, &[r.v[0].unwrap_or(0.0)]),
                hit: PixelRect {
                    left: rc.left,
                    top: ry - 3,
                    right: rc.right,
                    bottom: ry + 4,
                },
                bars: None,
            });
        } else {
            #[allow(clippy::cast_possible_truncation)]
            let cx = g.cx(r.slot, 1) as i32;
            out.prims.push(rect(
                cx - (w - 1) / 2,
                rc.top,
                w,
                rc.bottom - rc.top,
                r.color,
            ));
            let (left, right) = hit_x(cx, 3);
            out.items.push(Item {
                bar: Some(r.slot),
                t: r.t,
                values: r.v[0].map(|v| values(&pl.keys, &[v])).unwrap_or_default(),
                hit: PixelRect {
                    left,
                    top: rc.top,
                    right,
                    bottom: rc.bottom,
                },
                bars: None,
            });
        }
    }
}

fn points(pl: &Prepared, g: &Geo<'_>, out: &mut SceneLayer) {
    for r in &pl.traces[0].rows {
        let Some(y) = g.y(r.v[0]) else {
            out.skipped += 1;
            continue;
        };
        let cxf = g.cx(r.slot, r.n);
        #[allow(clippy::cast_possible_truncation)]
        let cx = cxf as i32;
        let ry = px(y);
        let (half_w, half_h) = if pl.geom == Geom::Point {
            #[allow(clippy::cast_possible_truncation)]
            let r2 = px((r.size / 2.0).ceil()) + 1;
            #[allow(clippy::cast_possible_truncation)]
            out.prims.push(Prim::Circle {
                cx: (cxf + 0.5) as f32,
                cy: y as f32,
                r: (r.size / 2.0) as f32,
                color: r.color,
            });
            (r2, r2)
        } else {
            let text = r.text.clone().unwrap_or_default();
            let mut w = est_width(&text, r.size);
            if w % 2 == 0 {
                w += 1;
            }
            #[allow(clippy::cast_possible_truncation)]
            out.prims.push(Prim::Text {
                x: cx - w / 2,
                baseline: ry + px(r.size * 0.35),
                px: r.size as f32,
                text,
                color: r.color,
                align: qp_schema::scene::Align::Left,
            });
            (w / 2, px((r.size / 2.0).ceil()))
        };
        let (left, right) = hit_x(cx, half_w);
        out.items.push(Item {
            bar: Some(r.slot),
            t: r.t,
            values: values(&pl.keys[..1], &[r.v[0].unwrap_or(0.0)]),
            hit: PixelRect {
                left,
                top: ry - half_h,
                right,
                bottom: ry + half_h + 1,
            },
            bars: g.bars(r.n),
        });
    }
}

/// Pixel position of a panel-relative coordinate along `[lo, hi)`.
fn panel_px(pp: PanelPos, lo: i32, hi: i32) -> f64 {
    (f64::from(lo) + pp.panel * f64::from(hi - lo) + pp.px).floor()
}

/// Words of `text` wrapped into lines of at most `room` pixels at `size` (estimate as in
/// [`est_width`]); a word longer than a line stands alone.
fn wrap(text: &str, room: i32, size: f64) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut cur = String::new();
    for w in text.split_whitespace() {
        let cand = if cur.is_empty() {
            w.to_owned()
        } else {
            format!("{cur} {w}")
        };
        if cur.is_empty() || est_width(&cand, size) <= room {
            cur = cand;
        } else {
            lines.push(std::mem::take(&mut cur));
            cur = w.to_owned();
        }
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines
}

/// A `text` layer with a panel-relative position (R3, `docs/GRAMMAR.md` §18): left-aligned
/// at a panel-relative `x` (else centred on the row's bar), its top at a panel-relative
/// `y` (else on the value); with a panel-relative `x` the text wraps at the panel's right
/// edge, and lines below the panel's bottom are dropped (the last kept one ends in "…").
fn anchored_text(pl: &Prepared, g: &Geo<'_>, out: &mut SceneLayer) {
    let rc = g.rect;
    for r in &pl.traces[0].rows {
        let size = r.size;
        #[allow(clippy::cast_possible_truncation)]
        let line_h = (size * 1.3).round() as i32;
        #[allow(clippy::cast_possible_truncation)]
        let ascent = (size * 0.8).round() as i32;
        let top = match pl.anchor.y {
            Some(pp) => {
                #[allow(clippy::cast_possible_truncation)]
                let t = panel_px(pp, rc.top, rc.bottom) as i32;
                t
            }
            None => {
                let Some(y) = g.y(r.v[0]) else {
                    out.skipped += 1;
                    continue;
                };
                px(y) - ascent / 2
            }
        };
        let text = r.text.clone().unwrap_or_default();
        let (left, lines, align_left) = match pl.anchor.x {
            Some(pp) => {
                #[allow(clippy::cast_possible_truncation)]
                let x = panel_px(pp, rc.left, rc.right) as i32;
                (x, wrap(&text, rc.right - x, size), true)
            }
            None => {
                #[allow(clippy::cast_possible_truncation)]
                let cx = g.cx(r.slot, r.n) as i32;
                (cx, vec![text], false)
            }
        };
        let mut kept: Vec<String> = Vec::new();
        for (i, l) in lines.iter().enumerate() {
            let bottom = top + line_h * i32::try_from(i + 1).unwrap_or(i32::MAX);
            if align_left && i > 0 && bottom > rc.bottom + (line_h - ascent) {
                if let Some(last) = kept.last_mut() {
                    *last = format!("{} …", last.trim_end());
                }
                break;
            }
            kept.push(l.clone());
        }
        let mut right = left;
        let mut x0 = left;
        for (i, l) in kept.iter().enumerate() {
            let w = est_width(l, size);
            let x = if align_left { left } else { left - w / 2 };
            x0 = x0.min(x);
            right = right.max(x + w);
            #[allow(clippy::cast_possible_truncation)]
            out.prims.push(Prim::Text {
                x,
                baseline: top + ascent + line_h * i32::try_from(i).unwrap_or(0),
                px: size as f32,
                text: l.clone(),
                color: r.color,
                align: qp_schema::scene::Align::Left,
            });
        }
        let n = i32::try_from(kept.len()).unwrap_or(1).max(1);
        let vals: Vec<f64> = r.v[0].into_iter().collect();
        out.items.push(Item {
            bar: pl.anchor.x.is_none().then_some(r.slot),
            t: if pl.anchor.x.is_none() { r.t } else { None },
            values: values(&pl.keys, &vals),
            hit: PixelRect {
                left: x0,
                top,
                right: right.max(x0 + 1),
                bottom: top + line_h * n,
            },
            bars: None,
        });
    }
}

/// Horizontal or sloped rule segments from bar `x` to bar `x2` (R3 evidence geometry).
fn segments(pl: &Prepared, g: &Geo<'_>, out: &mut SceneLayer) {
    let Some((a, b)) = pl.extent else { return };
    let w = pl.width + if g.bold { 1.0 } else { 0.0 };
    let rc = g.rect;
    for r in &pl.traces[0].rows {
        let (Some(y0), Some(y1)) = (g.y(r.v[0]), g.y(r.v[1].or(r.v[0]))) else {
            out.skipped += 1;
            continue;
        };
        if b < g.x.from_bar || a > g.x.to_bar {
            continue;
        }
        let (xa, xb) = (g.cx(a, 1) + 0.5, g.cx(b, 1) + 0.5);
        let (l, rr) = (f64::from(rc.left), f64::from(rc.right));
        // Clip to the panel horizontally (linear interpolation on a sloped segment).
        let at = |x: f64| {
            if xb == xa {
                y0
            } else {
                y0 + (y1 - y0) * (x - xa) / (xb - xa)
            }
        };
        let (ca, cb) = (xa.max(l), xb.min(rr));
        if cb < ca {
            continue;
        }
        let (ya, yb) = (at(ca), at(cb));
        if r.v[1].is_none() {
            let ry = px(y0);
            let wi = px(w).max(1);
            #[allow(clippy::cast_possible_truncation)]
            let (ia, ib) = (ca as i32, cb as i32);
            out.prims
                .push(rect(ia, ry - (wi - 1) / 2, (ib - ia).max(1), wi, r.color));
            out.items.push(Item {
                bar: None,
                t: None,
                values: values(&pl.keys, &[r.v[0].unwrap_or(0.0)]),
                hit: PixelRect {
                    left: ia,
                    top: ry - 3,
                    right: ib.max(ia + 1),
                    bottom: ry + 4,
                },
                bars: None,
            });
        } else {
            #[allow(clippy::cast_possible_truncation)]
            out.prims.push(Prim::Polyline {
                points: vec![pt([ca, ya]), pt([cb, yb])],
                width: w as f32,
                color: r.color,
            });
            #[allow(clippy::cast_possible_truncation)]
            let (ia, ib) = (ca as i32, cb as i32);
            let vals = [r.v[0].unwrap_or(0.0), r.v[1].unwrap_or(0.0)];
            out.items.push(Item {
                bar: None,
                t: None,
                values: values(&pl.keys, &vals),
                hit: PixelRect {
                    left: ia,
                    top: px(ya.min(yb)) - 3,
                    right: ib.max(ia + 1),
                    bottom: px(ya.max(yb)) + 4,
                },
                bars: None,
            });
        }
    }
}

/// Meaning glyphs (R3): the product alphabet draws each row's glyph into a square box —
/// top-left at the panel-relative position, else centred on the row's bar / value — kept
/// inside the panel.
fn glyphs(pl: &Prepared, g: &Geo<'_>, out: &mut SceneLayer) {
    let rc = g.rect;
    for (r, spec) in pl.traces[0].rows.iter().zip(&pl.glyphs) {
        let s = r.size.floor().max(1.0);
        let left = match pl.anchor.x {
            Some(pp) => panel_px(pp, rc.left, rc.right),
            None => (g.cx(r.slot, r.n) + 0.5 - s / 2.0).floor(),
        };
        let top = match pl.anchor.y {
            Some(pp) => panel_px(pp, rc.top, rc.bottom),
            None => {
                let Some(y) = g.y(r.v[0]) else {
                    out.skipped += 1;
                    continue;
                };
                (y - s / 2.0).floor()
            }
        };
        let left = left.min(f64::from(rc.right) - s).max(f64::from(rc.left));
        let top = top.min(f64::from(rc.bottom) - s).max(f64::from(rc.top));
        #[allow(clippy::cast_possible_truncation)]
        let bx = GlyphBox {
            x: left as f32,
            y: top as f32,
            size: s as f32,
        };
        out.prims
            .extend(g.outcome.draw(alphabet(), spec, bx, g.theme));
        let mut vals: BTreeMap<String, f64> = BTreeMap::new();
        vals.insert("strength".to_owned(), spec.strength);
        vals.insert("freshness".to_owned(), spec.freshness);
        vals.insert("confirmations".to_owned(), f64::from(spec.confirmations));
        if let Some(c) = spec.confidence {
            vals.insert("confidence".to_owned(), c);
        }
        if let (Some(k), Some(v)) = (pl.keys.first(), r.v[0]) {
            vals.insert(k.clone(), v);
        }
        #[allow(clippy::cast_possible_truncation)]
        let (l, t, side) = (left as i32, top as i32, s as i32);
        out.items.push(Item {
            bar: Some(r.slot),
            t: r.t,
            values: vals,
            hit: PixelRect {
                left: l,
                top: t,
                right: l + side,
                bottom: t + side,
            },
            bars: None,
        });
    }
}
