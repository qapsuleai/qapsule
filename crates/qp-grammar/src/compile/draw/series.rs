//! Series marks: candles, lines, areas and bars.

use super::*;

pub(super) fn candles(pl: &Prepared, g: &Geo<'_>, out: &mut SceneLayer) {
    #[allow(clippy::cast_precision_loss)]
    let body_w = body_width(g.x.slot * g.k as f64, g.look.chart.body_ratio);
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

#[allow(clippy::too_many_lines)]
pub(super) fn lines(pl: &Prepared, g: &Geo<'_>, out: &mut SceneLayer) {
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

#[allow(clippy::too_many_lines)]
pub(super) fn areas(pl: &Prepared, g: &Geo<'_>, out: &mut SceneLayer) {
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

pub(super) fn bars(pl: &Prepared, g: &Geo<'_>, out: &mut SceneLayer) {
    #[allow(clippy::cast_precision_loss)]
    let bw = body_width(g.x.slot * g.k as f64, g.look.chart.body_ratio);
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
