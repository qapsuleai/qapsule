#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::cognitive_complexity
)]
#![allow(dead_code)]

use std::collections::BTreeMap;

use qp_compute::{create, run, Field, Study};

pub fn params(kv: &[(&str, f64)]) -> BTreeMap<String, f64> {
    kv.iter().map(|(k, v)| ((*k).to_owned(), *v)).collect()
}

pub fn study(name: &str, kv: &[(&str, f64)]) -> Box<dyn Study> {
    create(name, &params(kv), Field::Close).unwrap()
}

/// Batch outputs of a built-in study over one column.
pub fn batch(name: &str, kv: &[(&str, f64)], x: &[f64]) -> Vec<Vec<Option<f64>>> {
    let mut s = study(name, kv);
    run(s.as_mut(), &[x]).unwrap()
}

/// Relative tolerance for rolling sums (`sma`, the Bollinger middle band) against a
/// from-scratch reference (sum of the window per bar): `|a - b| <= tol * max(1, |a|, |b|)`.
///
/// Why relative: the rolling sum adds and subtracts every input once, each step off by
/// up to half an ulp of the running sum, so its rounding error scales with the price
/// level. An absolute bound such as 1e-9 is a statement about the price scale and fails
/// at ~1e5 prices (`tests/sweep.rs`). Why 1e-12: the deterministic worst case after `n`
/// bars is `2 * n * 2^-53` relative, 3.3e-13 for the 1 500-bar property test and 4.4e-13
/// for the 2 000-bar high-price test; measured errors are ~1e-14 (R1 audit finding 5;
/// <= 3.6e-14 even over 50 000 bars at 1e5). A real formula error is orders of magnitude
/// larger. `max(1, ..)` keeps the bound absolute (1e-12) for values near zero.
pub const REL_TOL: f64 = 1e-12;

/// Relative tolerance for the Bollinger upper/lower bands against a two-pass standard
/// deviation, for series of up to ~2 000 bars.
///
/// The bands carry the error of the rolled M2 (sum of squared deviations), which, unlike
/// the rolling sum, drifts with the series length: measured worst case relative to the
/// band (period 3, cent random walks) 1.5e-11 at 2 000 bars and 1e5 prices, 5.5e-10 at
/// 10 000 bars, 1.2e-9 at 50 000 bars. 1e-10 bounds the short-series case with a 6x
/// margin; longer series need a looser bound (`rolling_variance_accuracy_on_long_series`
/// uses 1e-9 for 10 000 bars at ~200).
pub const BAND_REL_TOL: f64 = 1e-10;

/// `|a - b| <= tol * max(1, |a|, |b|)`.
pub fn close(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol * a.abs().max(b.abs()).max(1.0)
}
