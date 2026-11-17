//! Scene helpers for the reference renderer.
//!
//! Since R2 the renderer draws **only** from the scene description
//! ([`qp_schema::scene::SceneDesc`]) that the grammar compiler (`qp-grammar`) produces;
//! layout lives there, not here. This module keeps what is renderer-side:
//! - [`layout_glyphs`]: the reference text layout, reused by the spike backends so their
//!   text differs from the reference only by rasterizer;
//! - [`stress_scene`]: the hand-written determinism stress scene (golden
//!   `fixtures/golden/stress.png`);
//! - [`scene_of`]: wraps a bare primitive list in a `SceneDesc` (spikes, tests).
//!
//! This module is public only for the spike examples and the web binding; it is not a
//! stable API.

use qp_schema::scene::{SceneDesc, XMap};

pub use qp_schema::scene::{Align, Prim, Rgba};

use crate::RenderError;

/// Glyph layout identical to the reference renderer's (fontdue advances on the embedded
/// font, pen rounded to whole pixels). Returns `(glyph id, x)`; y is the baseline.
/// Used by the spike backends so their text differs from the reference only by rasterizer.
///
/// # Errors
/// Only if the embedded font is unusable.
pub fn layout_glyphs(
    text: &str,
    px: f32,
    x: i32,
    align: Align,
) -> Result<Vec<(u32, f32)>, RenderError> {
    let font = crate::cpu::font()?;
    let advance: f32 = text
        .chars()
        .map(|c| font.metrics(c, px).advance_width)
        .sum();
    let mut pen = match align {
        Align::Left => x as f32,
        Align::Right => x as f32 - advance,
    };
    let mut out = Vec::with_capacity(text.len());
    for c in text.chars() {
        out.push((u32::from(font.lookup_glyph_index(c)), pen.round()));
        pen += font.metrics(c, px).advance_width;
    }
    Ok(out)
}

/// A `SceneDesc` that draws exactly `prims` on `background` (no panels, no data): every
/// primitive goes into `under`. The mappings are placeholders spanning the frame.
#[must_use]
pub fn scene_of(width: u32, height: u32, background: Rgba, prims: Vec<Prim>) -> SceneDesc {
    SceneDesc {
        view: "prims".to_owned(),
        width,
        height,
        background,
        x: XMap {
            from_bar: 0,
            to_bar: 0,
            left: 0.0,
            slot: f64::from(width),
        },
        bars: 0,
        lod: None,
        panels: Vec::new(),
        emphasis: Vec::new(),
        under: prims,
        over: Vec::new(),
    }
}

fn rect(x: i32, y: i32, w: i32, h: i32, color: Rgba) -> Prim {
    Prim::Rect {
        x,
        y,
        w: w.max(0).unsigned_abs(),
        h: h.max(0).unsigned_abs(),
        color,
    }
}

/// Determinism stress scene (spike only): anti-aliased strokes at fractional coordinates and
/// widths, translucent colours, overlapping fills and text at many fractional sizes. Used by
/// the `stress` golden and the browser comparison to exercise paths the chart barely touches.
#[must_use]
pub fn stress_scene(width: u32, height: u32) -> SceneDesc {
    let mut rng = crate::bars::Lcg(0x5EED);
    let (wf, hf) = (f64::from(width), f64::from(height));
    let mut prims = Vec::new();
    let col = |rng: &mut crate::bars::Lcg, a: u8| -> Rgba {
        let v = rng.next_u32().to_le_bytes();
        [v[0], v[1], v[2], a]
    };
    for i in 0..40 {
        let a = col(&mut rng, 255);
        let x = (rng.unit() + 1.0) * 0.5 * wf;
        let y = (rng.unit() + 1.0) * 0.5 * hf;
        let side = (8.0 + 30.0 * (rng.unit() + 1.0)).floor() as i32;
        prims.push(rect(
            x as i32,
            y as i32,
            side,
            side,
            if i % 2 == 0 {
                a
            } else {
                [a[0], a[1], a[2], 128]
            },
        ));
    }
    for _ in 0..30 {
        let n = 2 + (rng.next_u32() % 12) as usize;
        let points = (0..n)
            .map(|_| {
                [
                    ((rng.unit() + 1.0) * 0.5 * wf) as f32,
                    ((rng.unit() + 1.0) * 0.5 * hf) as f32,
                ]
            })
            .collect();
        let width = (0.25 + 4.0 * (rng.unit() + 1.0)) as f32;
        let alpha = (64 + rng.next_u32() % 192) as u8;
        prims.push(Prim::Polyline {
            points,
            width,
            color: col(&mut rng, alpha),
        });
    }
    let mut y = 14;
    let mut size = 7.0_f32;
    while y < height.cast_signed() && size < 40.0 {
        let alpha = (128 + rng.next_u32() % 128) as u8;
        prims.push(Prim::Text {
            x: 4 + (rng.next_u32() % 40).cast_signed(),
            baseline: y,
            px: size,
            text: format!("Qapsule {size:.2}px 0123456789 +-.,:%$ AaBbGgQqWw"),
            color: col(&mut rng, alpha),
            align: Align::Left,
            font: None,
            role: None,
        });
        y += size as i32 + 4;
        size += 1.37;
    }
    scene_of(width, height, [250, 250, 248, 255], prims)
}
