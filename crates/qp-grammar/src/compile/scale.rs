//! Scale mappings of a compiled scene, both directions. Semantic reading (`hit`) and the
//! data → pixel → data round trip use the inverse functions.

use qp_schema::grammar::ScaleKind;
use qp_schema::scene::{XMap, YMap};

use qp_look::detmath;

/// Pixel x of the centre column of bar slot `slot`:
/// `floor(left + (slot - from_bar + 0.5) * slot_width)`.
#[must_use]
pub fn slot_x(x: &XMap, slot: u64) -> f64 {
    #[allow(clippy::cast_precision_loss)]
    let rel = slot as f64 - x.from_bar as f64;
    (x.left + (rel + 0.5) * x.slot).floor()
}

/// The bar slot under pixel x coordinate `px` (continuous; a pixel column `c` spans
/// `[c, c + 1)`), or `None` left of the plot.
#[must_use]
pub fn slot_at(x: &XMap, px: f64) -> Option<u64> {
    let rel = ((px - x.left) / x.slot).floor();
    if rel.is_nan() || rel < 0.0 {
        return None;
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let s = x.from_bar + rel as u64;
    Some(s)
}

/// `log10` of the domain ends of a log map (deterministic), or the ends of a linear map.
fn ends(y: &YMap) -> (f64, f64) {
    match y.kind {
        ScaleKind::Log => (
            detmath::log10(y.min).unwrap_or(0.0),
            detmath::log10(y.max).unwrap_or(1.0),
        ),
        _ => (y.min, y.max),
    }
}

/// Pixel y of value `v` (unrounded), or `None` when `v` cannot be drawn (non-finite, or
/// `<= 0` on a log scale).
#[must_use]
pub fn value_y(y: &YMap, v: f64) -> Option<f64> {
    if !v.is_finite() {
        return None;
    }
    let (lo, hi) = ends(y);
    let u = match y.kind {
        ScaleKind::Log => detmath::log10(v)?,
        _ => v,
    };
    Some(y.top + (hi - u) / (hi - lo) * (y.bottom - y.top))
}

/// Value at pixel y coordinate `py` (continuous).
#[must_use]
pub fn value_at(y: &YMap, py: f64) -> f64 {
    let (lo, hi) = ends(y);
    let u = hi - (py - y.top) / (y.bottom - y.top) * (hi - lo);
    match y.kind {
        ScaleKind::Log => detmath::pow10(u),
        _ => u,
    }
}

/// Value span of one pixel around value `v` (for round-trip tolerances).
#[must_use]
pub fn pixel_span(y: &YMap, v: f64) -> f64 {
    match value_y(y, v) {
        Some(py) => (value_at(y, py - 1.0) - value_at(y, py + 1.0)).abs() / 2.0,
        None => 0.0,
    }
}
