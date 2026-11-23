//! The R0 view bindings (RG1: moved out of `lib.rs`): compute -> grammar -> render of a
//! bar series, the synthetic and stress scenes.

use std::collections::BTreeMap;

use qp_grammar::{compile, MemProvider};
use qp_schema::scene::SceneDesc;
use qp_schema::Series;
use wasm_bindgen::prelude::*;

use super::compute::compute_err;

pub(super) fn js_err(e: qp_render::RenderError) -> JsError {
    JsError::new(&e.to_string())
}

/// The R0 view (`rounds/R2/tasks/VIEWS.md` V0): candles of `px` and the SMA(20) line of
/// `sma20` in one linear panel.
const R0_VIEW: &str = r#"{"id":"r0","x":{"kind":"time"},
  "panels":[{"id":0,"y":{"kind":"linear"}}],
  "layers":[{"id":"candles","mark":"candle","series":"px","panel":0},
            {"id":"sma","mark":"line","series":"sma20","panel":0}]}"#;

/// Compiles the R0 view over `series` at `width` x `height` (level of detail on).
///
/// # Errors
/// A series without bars, a study failure, or a compile error (e.g. size out of range).
pub(super) fn r0_scene(series: &Series, width: u32, height: u32) -> Result<SceneDesc, JsError> {
    let bars = series
        .bars()
        .ok_or_else(|| JsError::new("series has no embedded bars"))?
        .to_vec();
    let params = BTreeMap::from([("period".to_owned(), 20.0)]);
    let line = qp_compute::compute_bars("sma", &params, qp_compute::Field::Close, &bars)
        .map_err(compute_err)?
        .outputs
        .into_values()
        .next()
        .unwrap_or_default();
    let provider = MemProvider::new()
        .with_bars("px", bars, series.symbol(), series.interval.as_deref())
        .with_values("sma20", "px", vec![("sma", line)], "sma", &["sma"])
        .with_label("sma20", "SMA20");
    let view = serde_json::from_str(R0_VIEW).map_err(|e| JsError::new(&e.to_string()))?;
    compile(&view, &provider, &qp_designs::options(width, height))
        .map_err(|e| JsError::new(&e.to_string()))
}

pub(super) fn r0_scene_json(json: &str, width: u32, height: u32) -> Result<SceneDesc, JsError> {
    let series = qp_render::parse_bars(json).map_err(js_err)?;
    r0_scene(&series, width, height)
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

/// Renders a bars JSON document (the R0 view: candles + SMA(20)) to straight (non-premultiplied) RGBA8 pixels, ready for
/// `new ImageData(new Uint8ClampedArray(bytes), width, height)`.
///
/// # Errors
/// Invalid JSON/bars or an out-of-range size.
#[wasm_bindgen]
pub fn render_rgba(json: &str, width: u32, height: u32) -> Result<Vec<u8>, JsError> {
    qp_render::render_scene_rgba(&r0_scene_json(json, width, height)?).map_err(js_err)
}

/// Renders a bars JSON document (the R0 view) to PNG bytes (same encoder settings as native).
///
/// # Errors
/// Invalid JSON/bars or an out-of-range size.
#[wasm_bindgen]
pub fn render_png(json: &str, width: u32, height: u32) -> Result<Vec<u8>, JsError> {
    qp_render::render_scene_png(&r0_scene_json(json, width, height)?).map_err(js_err)
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
    let bars = qp_source::synthetic_series(n as usize, u64::from(seed));
    qp_render::render_scene_rgba(&r0_scene(&bars, width, height)?).map_err(js_err)
}

/// Renders the determinism stress scene (spike only; golden `fixtures/golden/stress.png`).
///
/// # Errors
/// Allocation failure.
#[wasm_bindgen]
pub fn render_stress_rgba(width: u32, height: u32) -> Result<Vec<u8>, JsError> {
    qp_render::render_stress_rgba(width, height).map_err(js_err)
}

#[cfg(test)]
mod tests {
    /// The binding's R0 wiring (compute -> grammar -> render) reproduces the golden PNG
    /// natively; the browser step checks the same in wasm32.
    #[test]
    fn render_png_matches_golden() {
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
        let json = std::fs::read_to_string(format!("{root}/fixtures/bars/sample.json")).unwrap();
        let golden = std::fs::read(format!("{root}/fixtures/golden/sample.png")).unwrap();
        let png = super::render_png(&json, super::default_width(), super::default_height())
            .unwrap_or_else(|_| panic!("render failed"));
        assert!(
            png == golden,
            "binding render differs from fixtures/golden/sample.png"
        );
    }
}
