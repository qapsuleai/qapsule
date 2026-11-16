//! Qapsule render core.
//!
//! A deterministic headless *reference renderer* (tiny-skia + fontdue, scalar paths only)
//! that rasterizes a scene description ([`SceneDesc`], produced by the grammar compiler
//! `qp-grammar`) and nothing else: no layout, no data access, no study computation here.
//! The same code is compiled to wasm32 by `qp-wasm`. See `SPIKE.md` for the render-stack
//! experiment and its determinism rules.
//!
//! Output pixel format of [`render_scene_rgba`]: **straight (non-premultiplied) RGBA8**,
//! row-major, top-left origin, `width * height * 4` bytes — exactly what
//! `ImageData`/`putImageData` expects. (The background is opaque, so every pixel currently
//! has alpha 255 and straight == premultiplied, but callers must not rely on that.)

use std::fmt;

mod bars;
pub mod cpu;
pub mod scene;
#[cfg(any(feature = "spike-vello-cpu", feature = "spike-vello-gpu"))]
pub mod spikes;

pub use bars::{parse_bars, synthetic_bars};
pub use qp_schema::scene::SceneDesc;
pub use qp_schema::{Bar, Series};

/// Default output width in pixels.
pub const DEFAULT_WIDTH: u32 = 1200;
/// Default output height in pixels.
pub const DEFAULT_HEIGHT: u32 = 600;

/// Any failure while parsing input or rendering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderError(String);

impl RenderError {
    pub(crate) fn new(msg: impl Into<String>) -> Self {
        Self(msg.into())
    }
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for RenderError {}

/// Rasterizes `scene` (its primitives, in paint order: `SceneDesc::prims()`) and returns
/// straight (non-premultiplied) RGBA8 pixels, `width * height * 4` bytes.
///
/// # Errors
/// Fails for a size outside `1x1 ..= 16384x16384`, a text size outside `(0, 512]`, or an
/// allocation failure (see [`cpu::rasterize`]).
pub fn render_scene_rgba(scene: &SceneDesc) -> Result<Vec<u8>, RenderError> {
    let pm = cpu::rasterize(scene)?;
    Ok(cpu::demultiply(pm.data()))
}

/// [`render_scene_rgba`] at an integer device scale `s` (R4/H-M8910, HiDPI): straight
/// RGBA8 of `width·s × height·s` (see [`cpu::rasterize_scaled`]); `s = 1` is
/// [`render_scene_rgba`].
///
/// # Errors
/// As [`render_scene_rgba`], or `s` outside `1..=4`.
pub fn render_scene_rgba_scaled(scene: &SceneDesc, s: u32) -> Result<Vec<u8>, RenderError> {
    let pm = cpu::rasterize_scaled(scene, s)?;
    Ok(cpu::demultiply(pm.data()))
}

/// Rasterizes `scene` and returns deterministic PNG bytes (8-bit RGBA, no ancillary
/// chunks, fixed filter and compression settings; see [`encode_png`]).
///
/// # Errors
/// Same as [`render_scene_rgba`], plus PNG encoding failures.
pub fn render_scene_png(scene: &SceneDesc) -> Result<Vec<u8>, RenderError> {
    let rgba = render_scene_rgba(scene)?;
    encode_png(&rgba, scene.width, scene.height)
}

/// Encodes straight RGBA8 pixels as PNG with fixed settings.
///
/// Byte-level stability relies on the pinned `png`/`fdeflate`/`flate2` versions in
/// `Cargo.lock`; pixel-level stability does not.
///
/// # Errors
/// Fails if `rgba.len() != width * height * 4` or the encoder errors.
pub fn encode_png(rgba: &[u8], width: u32, height: u32) -> Result<Vec<u8>, RenderError> {
    let enc_err = |e: png::EncodingError| RenderError::new(format!("png: {e}"));
    let mut out = Vec::new();
    let mut enc = png::Encoder::new(&mut out, width, height);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.set_compression(png::Compression::Balanced);
    enc.set_filter(png::Filter::Paeth);
    let mut w = enc.write_header().map_err(enc_err)?;
    w.write_image_data(rgba).map_err(enc_err)?;
    w.finish().map_err(enc_err)?;
    Ok(out)
}

/// Renders the determinism stress scene (spike only, see [`scene::stress_scene`]) as
/// straight RGBA8. Golden: `fixtures/golden/stress.png`.
///
/// # Errors
/// Fails if the pixmap cannot be allocated.
#[doc(hidden)]
pub fn render_stress_rgba(width: u32, height: u32) -> Result<Vec<u8>, RenderError> {
    render_scene_rgba(&scene::stress_scene(width, height))
}

#[cfg(test)]
mod scale_tests {
    use super::*;

    /// R4/H-M8910: scale 1 is the reference path byte for byte; scale 2 doubles the sides.
    #[test]
    fn scaled_render() {
        let sc = scene::stress_scene(120, 80);
        assert_eq!(
            render_scene_rgba_scaled(&sc, 1).unwrap(),
            render_scene_rgba(&sc).unwrap()
        );
        assert_eq!(
            render_scene_rgba_scaled(&sc, 2).unwrap().len(),
            240 * 160 * 4
        );
        assert!(render_scene_rgba_scaled(&sc, 5).is_err());
        assert!(render_scene_rgba_scaled(&sc, 0).is_err());
    }
}
