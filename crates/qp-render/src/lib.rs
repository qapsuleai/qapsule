//! Qapsule render core.
//!
//! R0 state: a deterministic headless *reference renderer* (tiny-skia + fontdue, scalar
//! paths only) that draws axes, candlesticks, one line series and text from a bars file.
//! The same code is compiled to wasm32 by `qp-wasm`. See `SPIKE.md` for the render-stack
//! experiment and measurements.
//!
//! Output pixel format of [`render_rgba`]: **straight (non-premultiplied) RGBA8**,
//! row-major, top-left origin, `width * height * 4` bytes — exactly what
//! `ImageData`/`putImageData` expects. (The background is opaque, so every pixel currently
//! has alpha 255 and straight == premultiplied, but callers must not rely on that.)

use std::fmt;

mod bars;
pub mod cpu;
pub mod scene;

pub use bars::{parse_bars, synthetic_bars, Bar, BarsFile};

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

/// Renders `bars` and returns straight (non-premultiplied) RGBA8 pixels.
///
/// # Errors
/// Fails for sizes outside `200x120 ..= 16384x16384` or an empty bar list.
pub fn render_rgba(bars: &BarsFile, width: u32, height: u32) -> Result<Vec<u8>, RenderError> {
    let scene = scene::build_scene(bars, width, height)?;
    let pm = cpu::rasterize(&scene)?;
    Ok(cpu::demultiply(pm.data()))
}

/// Renders `bars` and returns deterministic PNG bytes (8-bit RGBA, no ancillary chunks,
/// fixed filter and compression settings).
///
/// # Errors
/// Same as [`render_rgba`], plus PNG encoding failures.
pub fn render_png(bars: &BarsFile, width: u32, height: u32) -> Result<Vec<u8>, RenderError> {
    let rgba = render_rgba(bars, width, height)?;
    encode_png(&rgba, width, height)
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
    let pm = cpu::rasterize(&scene::stress_scene(width, height))?;
    Ok(cpu::demultiply(pm.data()))
}
