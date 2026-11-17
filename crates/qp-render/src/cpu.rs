//! Reference CPU backend: tiny-skia for shapes, fontdue for glyph coverage, and an
//! integer-only compositor for text. Output is premultiplied RGBA8 in a tiny-skia `Pixmap`.

use std::collections::BTreeMap;

use crate::RenderError;
use qp_schema::scene::{Align, Prim, Rgba, SceneDesc};
use tiny_skia::{
    Color, FillRule, LineCap, LineJoin, Paint, PathBuilder, Pixmap, Rect, Stroke, Transform,
};

/// The default font: IBM Plex Mono Regular, SIL OFL 1.1 (see `assets/fonts/OFL.txt`).
/// Embedded so output never depends on system fonts. (R5/T3) Other files:
/// [`crate::fonts`].
pub const FONT_BYTES: &[u8] = crate::fonts::FILES[0].1;

/// Parsed default font, shared by all renders.
///
/// # Errors
/// Only if the embedded font is corrupt (a build-time invariant).
pub fn font() -> Result<&'static fontdue::Font, RenderError> {
    crate::fonts::by_index(0)
}

fn color(c: Rgba) -> Color {
    Color::from_rgba8(c[0], c[1], c[2], c[3])
}

fn paint(c: Rgba, anti_alias: bool) -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color(color(c));
    p.anti_alias = anti_alias;
    p
}

/// Exact `round(x / 255)` for `x <= 255 * 255`, integer-only.
#[inline]
fn div255(x: u32) -> u32 {
    let x = x + 128;
    (x + (x >> 8)) >> 8
}

/// Glyph coverage bitmaps by (font index, character, size bits); (R5/T3) the font index
/// is [`crate::fonts::index`] of the text's `font`.
pub type GlyphCache = BTreeMap<(usize, char, u32), (fontdue::Metrics, Vec<u8>)>;

/// Draws one text line by compositing fontdue coverage masks with integer source-over.
/// Pen positions are rounded to whole pixels so glyph bitmaps are never resampled.
fn draw_text(
    pm: &mut Pixmap,
    font: (usize, &fontdue::Font),
    cache: &mut GlyphCache,
    t: (i32, i32, f32, &str, Rgba, Align),
) {
    let (fi, font) = font;
    let (x, baseline, size, text, c, align) = t;
    let advance: f32 = text
        .chars()
        .map(|ch| font.metrics(ch, size).advance_width)
        .sum();
    let mut pen = match align {
        Align::Left => x as f32,
        Align::Right => x as f32 - advance,
    };
    let (w, h) = (pm.width().cast_signed(), pm.height().cast_signed());
    let stride = pm.width() as usize * 4;
    let data = pm.data_mut();
    for ch in text.chars() {
        let (m, cov) = cache
            .entry((fi, ch, size.to_bits()))
            .or_insert_with(|| font.rasterize(ch, size));
        let gx = pen.round() as i32 + m.xmin;
        let gy = baseline - m.height as i32 - m.ymin;
        for row in 0..m.height {
            let py = gy + row as i32;
            if py < 0 || py >= h {
                continue;
            }
            for col in 0..m.width {
                let pxx = gx + col as i32;
                if pxx < 0 || pxx >= w {
                    continue;
                }
                let cv = u32::from(cov[row * m.width + col]);
                if cv == 0 {
                    continue;
                }
                let sa = div255(u32::from(c[3]) * cv);
                let i = py.unsigned_abs() as usize * stride + pxx.unsigned_abs() as usize * 4;
                let inv = 255 - sa;
                for (k, &sc) in c[..3].iter().enumerate() {
                    let s = div255(u32::from(sc) * sa);
                    let d = u32::from(data[i + k]);
                    data[i + k] = (s + div255(d * inv)).min(255) as u8;
                }
                let d = u32::from(data[i + 3]);
                data[i + 3] = (sa + div255(d * inv)).min(255) as u8;
            }
        }
        pen += m.advance_width;
    }
}

/// Largest accepted frame side in pixels (same bound as the grammar compiler).
pub const MAX_DIM: u32 = 16_384;
/// Largest accepted text size in pixels (a bound on the glyph bitmaps fontdue allocates).
pub const MAX_TEXT_PX: f32 = 512.0;

/// Builds an open path through `points` (`None` for fewer than two usable points).
fn polyline_path(points: &[[f32; 2]], close: bool) -> Option<tiny_skia::Path> {
    let mut pb = PathBuilder::new();
    for (i, &[x, y]) in points.iter().enumerate() {
        if i == 0 {
            pb.move_to(x, y);
        } else {
            pb.line_to(x, y);
        }
    }
    if close {
        pb.close();
    }
    pb.finish()
}

/// Rasterizes the primitives of `scene` (`SceneDesc::prims()`, in paint order) into a
/// premultiplied RGBA8 pixmap. Nothing else of the scene is read: layout is final there.
///
/// Primitives tiny-skia cannot build (non-finite coordinates, degenerate paths, a negative
/// or non-finite stroke width or radius) draw nothing.
///
/// # Errors
/// Fails for a size outside `1x1 ..= 16384x16384`, a text size that is not finite or not
/// in `(0, 512]`, an allocation failure or an unusable embedded font.
pub fn rasterize(scene: &SceneDesc) -> Result<Pixmap, RenderError> {
    rasterize_prims(scene.width, scene.height, scene.background, scene.prims())
}

/// One primitive scaled by the integer factor `s` (R4/H-M8910, HiDPI): every coordinate,
/// size, stroke width, radius and text size times `s`. `s = 1` is the identity.
#[must_use]
pub fn scale_prim(p: &Prim, s: u32) -> Prim {
    #[allow(clippy::cast_possible_wrap, clippy::cast_precision_loss)]
    let (si, sf) = (s as i32, s as f32);
    let pts = |v: &[[f32; 2]]| v.iter().map(|[x, y]| [x * sf, y * sf]).collect();
    match p {
        Prim::Rect { x, y, w, h, color } => Prim::Rect {
            x: x * si,
            y: y * si,
            w: w * s,
            h: h * s,
            color: *color,
        },
        Prim::Polyline {
            points,
            width,
            color,
        } => Prim::Polyline {
            points: pts(points),
            width: width * sf,
            color: *color,
        },
        Prim::Fill { points, color } => Prim::Fill {
            points: pts(points),
            color: *color,
        },
        Prim::Circle { cx, cy, r, color } => Prim::Circle {
            cx: cx * sf,
            cy: cy * sf,
            r: r * sf,
            color: *color,
        },
        Prim::Text {
            x,
            baseline,
            px,
            text,
            color,
            align,
            font,
            role,
        } => Prim::Text {
            x: x * si,
            baseline: baseline * si,
            px: px * sf,
            text: text.clone(),
            color: *color,
            align: *align,
            font: font.clone(),
            role: role.clone(),
        },
    }
}

/// [`rasterize`] at an integer device scale `s` (1..=4; R4/H-M8910, HiDPI): a pixmap of
/// `width·s × height·s` with every primitive scaled ([`scale_prim`]). The scene and its
/// hit boxes stay in CSS px; `s = 1` is exactly [`rasterize`].
///
/// # Errors
/// As [`rasterize`], or `s` outside `1..=4`.
pub fn rasterize_scaled(scene: &SceneDesc, s: u32) -> Result<Pixmap, RenderError> {
    if s == 1 {
        return rasterize(scene);
    }
    if !(1..=4).contains(&s) {
        return Err(RenderError::new(format!("scale {s} out of range (1..=4)")));
    }
    let prims: Vec<Prim> = scene.prims().map(|p| scale_prim(p, s)).collect();
    rasterize_prims(
        scene.width.saturating_mul(s),
        scene.height.saturating_mul(s),
        scene.background,
        prims.iter(),
    )
}

fn rasterize_prims<'a>(
    width: u32,
    height: u32,
    background: Rgba,
    prims: impl Iterator<Item = &'a Prim>,
) -> Result<Pixmap, RenderError> {
    if !(1..=MAX_DIM).contains(&width) || !(1..=MAX_DIM).contains(&height) {
        return Err(RenderError::new(format!(
            "size {width}x{height} out of range (1x1 ..= {MAX_DIM}x{MAX_DIM})"
        )));
    }
    let font = font()?;
    let mut pm =
        Pixmap::new(width, height).ok_or_else(|| RenderError::new("cannot allocate pixmap"))?;
    pm.fill(color(background));
    let mut cache = GlyphCache::new();
    for p in prims {
        draw_prim(&mut pm, font, &mut cache, p)?;
    }
    Ok(pm)
}

/// Draws one primitive onto `pm` exactly as [`rasterize`] does (RP3/M12a: the retained
/// painter, [`crate::retained`], draws with this same function). `cache` holds glyph
/// coverage bitmaps; it may live across calls (a bitmap depends only on the character and
/// the size).
///
/// # Errors
/// A text size that is not finite or not in `(0, 512]`.
pub fn draw_prim(
    pm: &mut Pixmap,
    font: &fontdue::Font,
    cache: &mut GlyphCache,
    p: &Prim,
) -> Result<(), RenderError> {
    let id = Transform::identity();
    match p {
        Prim::Rect { x, y, w, h, color } => {
            if let Some(r) = Rect::from_xywh(*x as f32, *y as f32, *w as f32, *h as f32) {
                pm.fill_rect(r, &paint(*color, false), id, None);
            }
        }
        Prim::Polyline {
            points,
            width,
            color,
        } => {
            if !(width.is_finite() && *width >= 0.0) {
                return Ok(());
            }
            if let Some(path) = polyline_path(points, false) {
                let stroke = Stroke {
                    width: *width,
                    line_cap: LineCap::Butt,
                    line_join: LineJoin::Miter,
                    ..Stroke::default()
                };
                pm.stroke_path(&path, &paint(*color, true), &stroke, id, None);
            }
        }
        Prim::Fill { points, color } => {
            if let Some(path) = polyline_path(points, true) {
                pm.fill_path(&path, &paint(*color, true), FillRule::Winding, id, None);
            }
        }
        Prim::Circle { cx, cy, r, color } => {
            if let Some(path) = PathBuilder::from_circle(*cx, *cy, *r) {
                pm.fill_path(&path, &paint(*color, true), FillRule::Winding, id, None);
            }
        }
        Prim::Text {
            x,
            baseline,
            px,
            text,
            color,
            align,
            font: face,
            ..
        } => {
            if !(px.is_finite() && *px > 0.0 && *px <= MAX_TEXT_PX) {
                return Err(RenderError::new(format!(
                    "text size {px} out of range (0, {MAX_TEXT_PX}]"
                )));
            }
            let f = crate::fonts::for_text(font, face.as_deref())?;
            draw_text(pm, f, cache, (*x, *baseline, *px, text, *color, *align));
        }
    }
    Ok(())
}

/// The pixel box `[x0, y0, x1, y1)` (device px, unclipped) that drawing `p` with
/// [`draw_prim`] can touch: a conservative bound (RP3/M12a, the retained painter's dirty
/// rectangles). Exact glyph boxes for text (the same pen walk as `draw_text`, metrics
/// only); the points' bounds widened by the stroke's miter reach and an antialiasing
/// margin for lines, fills and circles. `None`: unbounded (non-finite numbers), to be
/// treated as the whole surface.
#[must_use]
pub fn prim_bounds(font: &fontdue::Font, p: &Prim) -> Option<[i64; 4]> {
    // Antialiasing reaches at most one pixel past a shape's geometry; one more for safety.
    const AA: f32 = 2.0;
    match p {
        Prim::Rect { x, y, w, h, .. } => {
            let (x, y) = (i64::from(*x), i64::from(*y));
            Some([x - 1, y - 1, x + i64::from(*w) + 1, y + i64::from(*h) + 1])
        }
        Prim::Polyline { points, width, .. } => {
            if !width.is_finite() {
                return None;
            }
            // A miter join reaches `limit * width / 2` (limit 4, tiny-skia's default); a
            // butt cap `width / 2`.
            points_box(points, width.max(0.0) * 2.0 + AA)
        }
        Prim::Fill { points, .. } => points_box(points, AA),
        Prim::Circle { cx, cy, r, .. } => {
            if !(cx.is_finite() && cy.is_finite() && r.is_finite()) {
                return None;
            }
            let r = r.abs() + AA;
            Some(float_box(cx - r, cy - r, cx + r, cy + r))
        }
        Prim::Text {
            x,
            baseline,
            px,
            text,
            align,
            font: face,
            ..
        } => {
            if !(px.is_finite() && *px > 0.0 && *px <= MAX_TEXT_PX) {
                return None;
            }
            let (_, font) = crate::fonts::for_text(font, face.as_deref()).ok()?;
            let advance: f32 = text
                .chars()
                .map(|ch| font.metrics(ch, *px).advance_width)
                .sum();
            let mut pen = match align {
                Align::Left => *x as f32,
                Align::Right => *x as f32 - advance,
            };
            let mut b = [i64::MAX, i64::MAX, i64::MIN, i64::MIN];
            for ch in text.chars() {
                let m = font.metrics(ch, *px);
                if m.width > 0 && m.height > 0 {
                    let gx = i64::from(pen.round() as i32 + m.xmin);
                    let gy = i64::from(*baseline - m.height as i32 - m.ymin);
                    b[0] = b[0].min(gx);
                    b[1] = b[1].min(gy);
                    b[2] = b[2].max(gx + m.width as i64);
                    b[3] = b[3].max(gy + m.height as i64);
                }
                pen += m.advance_width;
            }
            if b[0] > b[2] {
                return Some([0, 0, 0, 0]);
            }
            Some([b[0] - 1, b[1] - 1, b[2] + 1, b[3] + 1])
        }
    }
}

/// The bounds of `pts` widened by `pad`, rounded outwards (`None`: a non-finite point).
fn points_box(pts: &[[f32; 2]], pad: f32) -> Option<[i64; 4]> {
    if pts.is_empty() {
        return Some([0, 0, 0, 0]);
    }
    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for &[x, y] in pts {
        if !(x.is_finite() && y.is_finite()) {
            return None;
        }
        x0 = x0.min(x);
        y0 = y0.min(y);
        x1 = x1.max(x);
        y1 = y1.max(y);
    }
    Some(float_box(x0 - pad, y0 - pad, x1 + pad, y1 + pad))
}

/// A float box rounded outwards to whole pixels (one more pixel on the far side).
fn float_box(x0: f32, y0: f32, x1: f32, y1: f32) -> [i64; 4] {
    [
        x0.floor() as i64,
        y0.floor() as i64,
        x1.ceil() as i64 + 1,
        y1.ceil() as i64 + 1,
    ]
}

/// Converts premultiplied RGBA8 to straight RGBA8 (integer-only, round-to-nearest).
#[must_use]
pub fn demultiply(premul: &[u8]) -> Vec<u8> {
    let mut out = premul.to_vec();
    for px in out.chunks_exact_mut(4) {
        let a = u32::from(px[3]);
        if a == 0 {
            px[..3].fill(0);
        } else if a < 255 {
            for c in &mut px[..3] {
                *c = ((u32::from(*c) * 255 + a / 2) / a).min(255) as u8;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn div255_is_exact_rounding() {
        for x in 0..=(255 * 255) {
            let exact = (f64::from(x) / 255.0).round() as u32;
            assert_eq!(div255(x), exact, "x = {x}");
        }
    }

    #[test]
    fn font_loads() {
        let f = font().unwrap();
        assert!(f.metrics('0', 11.0).advance_width > 0.0);
    }
}
