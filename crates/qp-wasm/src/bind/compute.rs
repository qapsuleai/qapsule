//! The compute bindings (RG1: moved out of `lib.rs`): studies, the benchmark sweep and
//! the cross-target digest.

use wasm_bindgen::prelude::*;

pub(super) fn compute_err(e: qp_compute::ComputeError) -> JsError {
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
