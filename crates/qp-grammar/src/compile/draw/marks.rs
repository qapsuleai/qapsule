//! Annotation marks: rules, points, anchored texts, segments and glyphs.

use super::*;

pub(super) fn rules(pl: &Prepared, g: &Geo<'_>, out: &mut SceneLayer) {
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

pub(super) fn points(pl: &Prepared, g: &Geo<'_>, out: &mut SceneLayer) {
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
            let mut w = est_width(&text, r.size, text_metrics(g.look, r.size).0);
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
                font: None,
                role: None,
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

#[allow(clippy::too_many_lines)]
/// A `text` layer with a panel-relative position (R3, `docs/GRAMMAR.md` §18): left-aligned
/// at a panel-relative `x` (else centred on the row's bar), its top at a panel-relative
/// `y` (else on the value); with a panel-relative `x` the text wraps at the panel's right
/// edge, and lines below the panel's bottom are dropped (the last kept one ends in "…").
pub(super) fn anchored_text(pl: &Prepared, g: &Geo<'_>, out: &mut SceneLayer) {
    let rc = g.rect;
    for r in &pl.traces[0].rows {
        let size = r.size;
        let (adv, line, asc) = text_metrics(g.look, size);
        #[allow(clippy::cast_possible_truncation)]
        let line_h = (size * line).round() as i32;
        #[allow(clippy::cast_possible_truncation)]
        let ascent = (size * asc).round() as i32;
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
                // (R5/M20) A panel-relative `x2` sets the wrap edge.
                #[allow(clippy::cast_possible_truncation)]
                let edge = pl
                    .anchor
                    .x2
                    .map_or(rc.right, |e| panel_px(e, rc.left, rc.right) as i32);
                (x, wrap(&text, edge - x, size, adv), true)
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
            let w = est_width(l, size, adv);
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
                font: None,
                role: None,
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
pub(super) fn segments(pl: &Prepared, g: &Geo<'_>, out: &mut SceneLayer) {
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
pub(super) fn glyphs(pl: &Prepared, g: &Geo<'_>, out: &mut SceneLayer) {
    let rc = g.rect;
    for (r, spec) in pl.traces[0].rows.iter().zip(&pl.glyphs) {
        let s = g.look.mark_px(r.size).floor().max(1.0);
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
            .extend(g.outcome.draw(g.alphabet, spec, bx, g.theme));
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
