//! Reference CPU backend: tiny-skia for shapes, fontdue for glyph coverage, and an
//! integer-only compositor for text. Output is premultiplied RGBA8 in a tiny-skia `Pixmap`.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use tiny_skia::{Color, LineCap, LineJoin, Paint, PathBuilder, Pixmap, Rect, Stroke, Transform};

use crate::scene::{Align, Prim, Rgba, Scene};
use crate::RenderError;

/// IBM Plex Mono Regular, SIL OFL 1.1 (see `assets/fonts/OFL.txt`). Embedded so output never
/// depends on system fonts.
pub const FONT_BYTES: &[u8] = include_bytes!("../assets/fonts/IBMPlexMono-Regular.ttf");

/// Parsed embedded font, shared by all renders.
///
/// # Errors
/// Only if the embedded font is corrupt (a build-time invariant).
pub fn font() -> Result<&'static fontdue::Font, RenderError> {
    static FONT: OnceLock<Result<fontdue::Font, String>> = OnceLock::new();
    FONT.get_or_init(|| {
        fontdue::Font::from_bytes(FONT_BYTES, fontdue::FontSettings::default())
            .map_err(str::to_owned)
    })
    .as_ref()
    .map_err(|e| RenderError::new(format!("embedded font: {e}")))
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

type GlyphCache = BTreeMap<(char, u32), (fontdue::Metrics, Vec<u8>)>;

/// Draws one text line by compositing fontdue coverage masks with integer source-over.
/// Pen positions are rounded to whole pixels so glyph bitmaps are never resampled.
fn draw_text(
    pm: &mut Pixmap,
    font: &fontdue::Font,
    cache: &mut GlyphCache,
    t: (i32, i32, f32, &str, Rgba, Align),
) {
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
            .entry((ch, size.to_bits()))
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

/// Rasterizes `scene` into a premultiplied RGBA8 pixmap.
///
/// # Errors
/// Fails if the pixmap cannot be allocated or the embedded font is unusable.
pub fn rasterize(scene: &Scene) -> Result<Pixmap, RenderError> {
    let font = font()?;
    let mut pm = Pixmap::new(scene.width, scene.height)
        .ok_or_else(|| RenderError::new("cannot allocate pixmap"))?;
    pm.fill(color(scene.background));
    let mut cache = GlyphCache::new();
    let id = Transform::identity();
    for p in &scene.prims {
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
                let mut pb = PathBuilder::new();
                for (i, &(x, y)) in points.iter().enumerate() {
                    if i == 0 {
                        pb.move_to(x, y);
                    } else {
                        pb.line_to(x, y);
                    }
                }
                if let Some(path) = pb.finish() {
                    let stroke = Stroke {
                        width: *width,
                        line_cap: LineCap::Butt,
                        line_join: LineJoin::Miter,
                        ..Stroke::default()
                    };
                    pm.stroke_path(&path, &paint(*color, true), &stroke, id, None);
                }
            }
            Prim::Text {
                x,
                baseline,
                px,
                text,
                color,
                align,
            } => {
                draw_text(
                    &mut pm,
                    font,
                    &mut cache,
                    (*x, *baseline, *px, text, *color, *align),
                );
            }
        }
    }
    Ok(pm)
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
