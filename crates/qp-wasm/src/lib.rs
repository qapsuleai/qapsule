//! Qapsule web binding.
//!
//! A thin wasm-bindgen wrapper: every function calls the same `qp-render` code that the
//! native CLI and the golden test use. No drawing logic lives here.

use wasm_bindgen::prelude::*;

fn js_err(e: qp_render::RenderError) -> JsError {
    JsError::new(&e.to_string())
}

/// Default output width (mirrors `qp_render::DEFAULT_WIDTH`).
#[wasm_bindgen]
#[must_use]
pub fn default_width() -> u32 {
    qp_render::DEFAULT_WIDTH
}

/// Default output height (mirrors `qp_render::DEFAULT_HEIGHT`).
#[wasm_bindgen]
#[must_use]
pub fn default_height() -> u32 {
    qp_render::DEFAULT_HEIGHT
}

/// Renders a bars JSON document to straight (non-premultiplied) RGBA8 pixels, ready for
/// `new ImageData(new Uint8ClampedArray(bytes), width, height)`.
///
/// # Errors
/// Invalid JSON/bars or an out-of-range size.
#[wasm_bindgen]
pub fn render_rgba(json: &str, width: u32, height: u32) -> Result<Vec<u8>, JsError> {
    let bars = qp_render::parse_bars(json).map_err(js_err)?;
    qp_render::render_rgba(&bars, width, height).map_err(js_err)
}

/// Renders a bars JSON document to PNG bytes (same encoder settings as native).
///
/// # Errors
/// Invalid JSON/bars or an out-of-range size.
#[wasm_bindgen]
pub fn render_png(json: &str, width: u32, height: u32) -> Result<Vec<u8>, JsError> {
    let bars = qp_render::parse_bars(json).map_err(js_err)?;
    qp_render::render_png(&bars, width, height).map_err(js_err)
}

/// Renders `n` deterministic synthetic bars (benchmark helper; see `SPIKE.md`).
///
/// # Errors
/// An out-of-range size or `n == 0`.
#[wasm_bindgen]
pub fn render_synthetic_rgba(
    n: u32,
    seed: u32,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, JsError> {
    let bars = qp_render::synthetic_bars(n as usize, u64::from(seed));
    qp_render::render_rgba(&bars, width, height).map_err(js_err)
}
