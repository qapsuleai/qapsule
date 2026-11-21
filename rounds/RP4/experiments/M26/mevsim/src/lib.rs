//! RP4 M26 alternative `mevsim`: detector thresholds adjusted for intraday seasonality.
//!
//! Intraday volume and volatility are U-shaped (opening and closing high, midday low);
//! thresholds of the form "k times the mean of the last N bars" therefore fire at the
//! open and at the close by construction. This set scales such thresholds by the typical
//! value of the **same time of day on past days** ([`season::Season`]; XIST calendar,
//! slot from the bar time; no lookahead).
//!
//! The set has the product's detector ids and order (`qp_facts::default_detectors`);
//! only the detectors chosen by measurement (`NOTES.md`) are replaced
//! (`flow.volume_surge`, `breakout.range`), and only on bars of at least one hour. On
//! daily bars, and on 5 / 15 minute bars, every replaced detector behaves exactly as the
//! product's (factor `1.0`).
//!
//! Pure and deterministic: no clock, network, randomness or file access; the seasonal
//! state lives in each detector's own streaming state.

pub mod breakout;
pub mod season;
pub mod volume;

use std::collections::BTreeMap;

use qp_facts::{
    BandSqueeze, Detector, InstitutionalFlow, MaStack, RangeBreakout, RsiDivergence, RsiExtreme,
};

pub use breakout::SeasonalBreakout;
pub use season::Season;
pub use volume::SeasonalVolumeSurge;

/// Profile length: past trading days per slot.
pub const DAYS: usize = 20;
/// A slot's factor is used from this many past days on.
pub const MIN_DAYS: usize = 5;
/// Period floor: the adjustment is active for bars of at least one hour (NOTES.md,
/// trial 3); shorter bars get the product detectors' behaviour exactly.
pub const MIN_LEN_MS: i64 = 3_600_000;
/// Volume window and multiple (product values, unchanged).
pub const VOL_WINDOW: usize = 20;
/// See [`VOL_WINDOW`].
pub const VOL_MULT: f64 = 2.5;

/// Seasonal volume surge (`gate`: only removes product births) active from `min_len_ms`.
///
/// # Panics
/// Never (valid constants).
#[must_use]
pub fn volume_surge(min_len_ms: i64, gate: bool) -> Box<dyn Detector> {
    Box::new(
        SeasonalVolumeSurge::new(VOL_WINDOW, VOL_MULT, DAYS, MIN_DAYS, min_len_ms, gate)
            .expect("valid")
            .detector(),
    )
}

/// Seasonal range breakout active from `min_len_ms`.
///
/// # Panics
/// Never (valid constants).
#[must_use]
pub fn range_breakout(min_len_ms: i64) -> Box<dyn Detector> {
    Box::new(
        SeasonalBreakout::new(
            RangeBreakout::WINDOW,
            RangeBreakout::ATR,
            DAYS,
            MIN_DAYS,
            min_len_ms,
        )
        .expect("valid")
        .detector(),
    )
}

/// The product set in its order, with `breakout` and `volume` in their slots.
fn set(breakout: Box<dyn Detector>, volume: Box<dyn Detector>) -> Vec<Box<dyn Detector>> {
    vec![
        Box::new(MaStack::default().detector()),
        Box::new(RsiDivergence::default().detector()),
        breakout,
        Box::new(BandSqueeze::default().detector()),
        Box::new(RsiExtreme::default().detector()),
        volume,
        Box::new(InstitutionalFlow::default().detector()),
    ]
}

/// The `mevsim` set (final, NOTES.md trial 3): seasonal `flow.volume_surge` and
/// `breakout.range`, active for bars of at least one hour (fresh per call).
#[must_use]
pub fn detectors() -> Vec<Box<dyn Detector>> {
    set(range_breakout(MIN_LEN_MS), volume_surge(MIN_LEN_MS, false))
}

/// Trial 1a: seasonal volume only, every intraday period.
#[must_use]
pub fn trial_volume() -> Vec<Box<dyn Detector>> {
    set(
        Box::new(RangeBreakout::default().detector()),
        volume_surge(0, false),
    )
}

/// Trial 1b: seasonal volume and range breakout, every intraday period.
#[must_use]
pub fn trial_volume_breakout() -> Vec<Box<dyn Detector>> {
    set(range_breakout(0), volume_surge(0, false))
}

/// Trial 2: gated seasonal volume (only removes) and range breakout, every intraday period.
#[must_use]
pub fn trial_gate() -> Vec<Box<dyn Detector>> {
    set(range_breakout(0), volume_surge(0, true))
}

/// Evidence map from `(key, value)` pairs.
pub(crate) fn evidence(kv: &[(&str, f64)]) -> BTreeMap<String, f64> {
    kv.iter().map(|(k, v)| ((*k).to_owned(), *v)).collect()
}

/// `x` clipped to `[0, 1]`; `NaN` → 0 (as `qp_facts`).
pub(crate) fn clip01(x: f64) -> f64 {
    if x > 1.0 {
        1.0
    } else if x > 0.0 {
        x
    } else {
        0.0
    }
}
