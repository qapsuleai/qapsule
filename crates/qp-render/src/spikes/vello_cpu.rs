//! Spike backend (feature `spike-vello-cpu`): draws a [`Scene`] with vello_cpu.
//! Comparison-only; the reference renderer is [`crate::cpu`].
//!
//! Text uses vello_cpu's outline rasterizer (glifo/skrifa) on the same embedded font, with
//! glyph positions from [`crate::scene::layout_glyphs`], so text differences come from the
//! rasterizer only.

use std::sync::{Arc, OnceLock};

use vello_cpu::color::{AlphaColor, Srgb};
use vello_cpu::kurbo::{BezPath, Rect, Stroke};
use vello_cpu::peniko::{Blob, FontData};
use vello_cpu::{Glyph, Level, Pixmap, RasterizerSettings, RenderContext, RenderMode};
use vello_cpu::{RenderSettings, Resources};

use crate::scene::{Prim, Rgba, Scene};
use crate::RenderError;

/// vello_cpu configuration knobs measured by the spike.
#[derive(Debug, Clone, Copy, Default)]
pub struct Config {
    /// `true`: f32 pipeline (`OptimizeQuality`); `false`: u8 pipeline (`OptimizeSpeed`).
    pub quality: bool,
    /// Worker threads (0 = single-threaded; >0 needs feature `spike-vello-cpu-mt`).
    pub threads: u16,
    /// Force the baseline SIMD level instead of runtime detection.
    pub baseline_simd: bool,
}

fn font() -> &'static FontData {
    static F: OnceLock<FontData> = OnceLock::new();
    F.get_or_init(|| FontData::new(Blob::new(Arc::new(crate::cpu::FONT_BYTES.to_vec())), 0))
}

fn rgba(c: Rgba) -> AlphaColor<Srgb> {
    AlphaColor::from_rgba8(c[0], c[1], c[2], c[3])
}

/// Rasterizes `scene`; returns straight RGBA8.
///
/// # Errors
/// Size beyond `u16`, or a glyph-run failure.
pub fn rasterize(scene: &Scene, cfg: Config) -> Result<Vec<u8>, RenderError> {
    let to16 = |v: u32| u16::try_from(v).map_err(|_| RenderError::new("size exceeds u16"));
    let (w, h) = (to16(scene.width)?, to16(scene.height)?);
    let settings = RenderSettings {
        level: if cfg.baseline_simd {
            Level::baseline()
        } else {
            Level::try_detect().unwrap_or(Level::baseline())
        },
        num_threads: cfg.threads,
    };
    let mut ctx = RenderContext::new_with(w, h, settings);
    let mut res = Resources::new();
    ctx.set_paint(rgba(scene.background));
    ctx.fill_rect(&Rect::new(0.0, 0.0, f64::from(w), f64::from(h)));
    for p in &scene.prims {
        match p {
            Prim::Rect { x, y, w, h, color } => {
                ctx.set_paint(rgba(*color));
                let (x, y) = (f64::from(*x), f64::from(*y));
                ctx.fill_rect(&Rect::new(x, y, x + f64::from(*w), y + f64::from(*h)));
            }
            Prim::Polyline {
                points,
                width,
                color,
            } => {
                let mut bp = BezPath::new();
                for (i, &(x, y)) in points.iter().enumerate() {
                    let pt = (f64::from(x), f64::from(y));
                    if i == 0 {
                        bp.move_to(pt);
                    } else {
                        bp.line_to(pt);
                    }
                }
                ctx.set_paint(rgba(*color));
                ctx.set_stroke(Stroke::new(f64::from(*width)));
                ctx.stroke_path(&bp);
            }
            Prim::Text {
                x,
                baseline,
                px,
                text,
                color,
                align,
            } => {
                ctx.set_paint(rgba(*color));
                let glyphs: Vec<Glyph> = crate::scene::layout_glyphs(text, *px, *x, *align)?
                    .into_iter()
                    .map(|(id, gx)| Glyph {
                        id,
                        x: gx,
                        y: *baseline as f32,
                    })
                    .collect();
                ctx.glyph_run(&mut res, font())
                    .font_size(*px)
                    .hint(false)
                    .fill_glyphs(glyphs.into_iter())
                    .map_err(|e| RenderError::new(format!("glyph run: {e:?}")))?;
            }
        }
    }
    let mut pm = Pixmap::new(w, h);
    ctx.flush();
    let mode = if cfg.quality {
        RenderMode::OptimizeQuality
    } else {
        RenderMode::OptimizeSpeed
    };
    let rs = RasterizerSettings {
        render_mode: mode,
        ..RasterizerSettings::default()
    };
    ctx.render_with(&mut pm, &mut res, rs);
    Ok(crate::cpu::demultiply(pm.data_as_u8_slice()))
}
