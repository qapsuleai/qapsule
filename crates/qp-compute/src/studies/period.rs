//! [`pf`]: the period conversion shared by the studies (out of `mod.rs`).

/// `usize -> f64` for periods (validated `<= MAX_PERIOD`, so exact).
#[allow(clippy::cast_precision_loss)]
pub(crate) fn pf(p: usize) -> f64 {
    p as f64
}
