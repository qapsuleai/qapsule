//! Qapsule web binding.
//!
//! A thin wasm-bindgen wrapper: every function calls the same `qp-render` / `qp-compute`
//! code that the native CLI and the tests use. No drawing or compute logic lives here.

use wasm_bindgen::prelude::*;

fn js_err(e: qp_render::RenderError) -> JsError {
    JsError::new(&e.to_string())
}

fn compute_err(e: qp_compute::ComputeError) -> JsError {
    JsError::new(&e.to_string())
}

/// Runs a built-in study (`sma`, `ema`, `rsi`, `macd`, `bollinger`) over a bar series
/// document and returns a `StudyOutput` JSON string (one value per bar, warmup `null`).
/// `params_json` is an object of numeric parameters (`{}` for defaults); `field` is the
/// bar field to read (`close`, `open`, ...).
///
/// # Errors
/// Invalid JSON, bars, study name, parameter or field.
#[wasm_bindgen]
pub fn compute(
    study: &str,
    params_json: &str,
    field: &str,
    bars_json: &str,
) -> Result<String, JsError> {
    let params: std::collections::BTreeMap<String, f64> = serde_json::from_str(params_json)
        .map_err(|e| JsError::new(&format!("invalid params JSON: {e}")))?;
    let field = qp_compute::Field::parse(field).map_err(compute_err)?;
    let series = qp_schema::parse_bar_series(bars_json).map_err(|e| JsError::new(&e))?;
    let bars = series.bars().unwrap_or_default();
    let out = qp_compute::compute_bars(study, &params, field, bars).map_err(compute_err)?;
    serde_json::to_string(&out).map_err(|e| JsError::new(&e.to_string()))
}

/// Benchmark helper: one sweep of the R1 benchmark grid (1,000 parameter sets) of
/// `study` over `bars` deterministic closes (`qp_compute::sweep::bench_closes`). Returns
/// the sum of every variant's last value so the work cannot be optimized away; time it
/// from JavaScript.
///
/// # Errors
/// Unknown study.
#[wasm_bindgen]
pub fn sweep_bench(study: &str, bars: u32, seed: u32) -> Result<f64, JsError> {
    let x = qp_compute::sweep::bench_closes(bars as usize, u64::from(seed));
    let grid = qp_compute::sweep::bench_grid(study);
    let mut sink = 0.0;
    let last = x.len().saturating_sub(1);
    qp_compute::sweep::sweep_each(study, &grid, &x, |_, c| sink += c.columns[0][last])
        .map_err(compute_err)?;
    Ok(sink)
}

/// FNV-1a digest (hex) of all outputs of the R1 benchmark grid sweep, as printed by
/// `cargo run --release -p qp-compute --example sweep_bench`.
///
/// # Errors
/// Unknown study.
#[wasm_bindgen]
pub fn sweep_digest(study: &str, bars: u32, seed: u32) -> Result<String, JsError> {
    let x = qp_compute::sweep::bench_closes(bars as usize, u64::from(seed));
    let grid = qp_compute::sweep::bench_grid(study);
    let mut h = qp_compute::sweep::FNV_OFFSET;
    qp_compute::sweep::sweep_each(study, &grid, &x, |_, c| {
        h = qp_compute::sweep::fnv1a_columns(h, c);
    })
    .map_err(compute_err)?;
    Ok(format!("{h:016x}"))
}

/// The cross-target compute digest (hex), same as
/// `cargo run --release -p qp-compute --example digest`.
#[wasm_bindgen]
#[must_use]
pub fn compute_digest() -> String {
    format!("{:016x}", qp_compute::sweep::cross_target_digest())
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

/// Renders the determinism stress scene (spike only; golden `fixtures/golden/stress.png`).
///
/// # Errors
/// Allocation failure.
#[wasm_bindgen]
pub fn render_stress_rgba(width: u32, height: u32) -> Result<Vec<u8>, JsError> {
    qp_render::render_stress_rgba(width, height).map_err(js_err)
}

/// Spike (feature `spike-vello-cpu`): render a bars JSON document with the vello_cpu backend.
///
/// # Errors
/// Invalid JSON/bars or an out-of-range size.
#[cfg(feature = "spike-vello-cpu")]
#[wasm_bindgen]
pub fn render_rgba_vello_cpu(
    json: &str,
    width: u32,
    height: u32,
    quality: bool,
) -> Result<Vec<u8>, JsError> {
    use qp_render::spikes::vello_cpu::{rasterize, Config};
    let bars = qp_render::parse_bars(json).map_err(js_err)?;
    let scene = qp_render::scene::build_scene(&bars, width, height).map_err(js_err)?;
    rasterize(
        &scene,
        Config {
            quality,
            ..Config::default()
        },
    )
    .map_err(js_err)
}

/// Spike (feature `spike-vello-cpu`): the stress scene through vello_cpu.
///
/// # Errors
/// Out-of-range size.
#[cfg(feature = "spike-vello-cpu")]
#[wasm_bindgen]
pub fn render_stress_rgba_vello_cpu(
    width: u32,
    height: u32,
    quality: bool,
) -> Result<Vec<u8>, JsError> {
    use qp_render::spikes::vello_cpu::{rasterize, Config};
    let scene = qp_render::scene::stress_scene(width, height);
    rasterize(
        &scene,
        Config {
            quality,
            ..Config::default()
        },
    )
    .map_err(js_err)
}

/// Spike (feature `spike-vello-cpu`): `n` synthetic bars through vello_cpu (u8 pipeline).
///
/// # Errors
/// Out-of-range size or `n == 0`.
#[cfg(feature = "spike-vello-cpu")]
#[wasm_bindgen]
pub fn render_synthetic_rgba_vello_cpu(
    n: u32,
    seed: u32,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, JsError> {
    use qp_render::spikes::vello_cpu::{rasterize, Config};
    let bars = qp_render::synthetic_bars(n as usize, u64::from(seed));
    let scene = qp_render::scene::build_scene(&bars, width, height).map_err(js_err)?;
    rasterize(&scene, Config::default()).map_err(js_err)
}

#[cfg(feature = "spike-vello-gpu")]
mod vello_gpu_spike {
    use std::cell::RefCell;

    use qp_render::spikes::vello_gpu::{AaConfig, GpuRenderer};
    use wasm_bindgen::prelude::*;

    thread_local! {
        static GPU: RefCell<Option<GpuRenderer>> = const { RefCell::new(None) };
    }

    fn aa(mode: u8) -> AaConfig {
        match mode {
            1 => AaConfig::Msaa8,
            2 => AaConfig::Msaa16,
            _ => AaConfig::Area,
        }
    }

    /// Spike (feature `spike-vello-gpu`): create the WebGPU device and vello pipelines.
    ///
    /// # Errors
    /// No WebGPU adapter/device.
    #[wasm_bindgen]
    pub async fn vello_gpu_init() -> Result<(), JsError> {
        let r = GpuRenderer::new_async().await.map_err(super::js_err)?;
        GPU.with(|g| *g.borrow_mut() = Some(r));
        Ok(())
    }

    async fn with_gpu(scene: qp_render::scene::Scene, mode: u8) -> Result<Vec<u8>, JsError> {
        // Take the renderer out for the duration of the await (no RefCell borrow held).
        let mut r = GPU
            .with(|g| g.borrow_mut().take())
            .ok_or_else(|| JsError::new("call vello_gpu_init first"))?;
        let out = r.render_async(&scene, aa(mode)).await;
        GPU.with(|g| *g.borrow_mut() = Some(r));
        out.map_err(super::js_err)
    }

    /// Spike: render a bars JSON document with vello on WebGPU; `aa`: 0 area, 1 msaa8, 2 msaa16.
    ///
    /// # Errors
    /// Invalid input, missing init, or GPU failure.
    #[wasm_bindgen]
    pub async fn vello_gpu_render_rgba(
        json: String,
        width: u32,
        height: u32,
        aa: u8,
    ) -> Result<Vec<u8>, JsError> {
        let bars = qp_render::parse_bars(&json).map_err(super::js_err)?;
        let scene = qp_render::scene::build_scene(&bars, width, height).map_err(super::js_err)?;
        with_gpu(scene, aa).await
    }

    /// Spike: the stress scene with vello on WebGPU.
    ///
    /// # Errors
    /// Missing init or GPU failure.
    #[wasm_bindgen]
    pub async fn vello_gpu_render_stress_rgba(
        width: u32,
        height: u32,
        aa: u8,
    ) -> Result<Vec<u8>, JsError> {
        with_gpu(qp_render::scene::stress_scene(width, height), aa).await
    }

    /// Spike: `n` synthetic bars with vello on WebGPU.
    ///
    /// # Errors
    /// Invalid size, missing init or GPU failure.
    #[wasm_bindgen]
    pub async fn vello_gpu_render_synthetic_rgba(
        n: u32,
        seed: u32,
        width: u32,
        height: u32,
        aa: u8,
    ) -> Result<Vec<u8>, JsError> {
        let bars = qp_render::synthetic_bars(n as usize, u64::from(seed));
        let scene = qp_render::scene::build_scene(&bars, width, height).map_err(super::js_err)?;
        with_gpu(scene, aa).await
    }
}
