//! Named detector sets: [`default_detectors`] (R3), [`rp4_detectors`] and
//! [`detectors_named`] (out of `lib.rs`).

use crate::{
    BandSqueeze, Detector, InstitutionalFlow, MaStack, RangeBreakout, RsiDivergence, RsiExtreme,
    SeasonalBreakout, SeasonalVolumeSurge, VolumeSurge,
};

/// The R3 detectors with their default parameters, in a fixed order (continuation,
/// reversal, breakout, squeeze, extreme, volume flow, institutional flow).
#[must_use]
pub fn default_detectors() -> Vec<Box<dyn Detector>> {
    vec![
        Box::new(MaStack::default().detector()),
        Box::new(RsiDivergence::default().detector()),
        Box::new(RangeBreakout::default().detector()),
        Box::new(BandSqueeze::default().detector()),
        Box::new(RsiExtreme::default().detector()),
        Box::new(VolumeSurge::default().detector()),
        Box::new(InstitutionalFlow::default().detector()),
    ]
}

/// (RP4) Seasonal profile length of the `rp4` set: past trading days per slot.
pub const SEASON_DAYS: usize = 20;
/// (RP4) A slot's seasonal factor is used from this many past days on.
pub const SEASON_MIN_DAYS: usize = 5;
/// (RP4) Period floor of the `rp4` set: the seasonal adjustment is active for bars of at
/// least one hour; shorter bars (and daily bars) get exactly the `r3` detections.
pub const SEASON_MIN_LEN_MS: i64 = 3_600_000;

/// (RP4/M26, `docs/CAPSULE.md` §9.9) The `rp4` detector set: the `r3` set
/// ([`default_detectors`]) in the same order and with the same ids, with
/// `breakout.range` and `flow.volume_surge` replaced by their seasonal version 2
/// ([`SeasonalBreakout`], [`SeasonalVolumeSurge`]; intraday seasonality of the XIST
/// calendar, active only on bars of at least [`SEASON_MIN_LEN_MS`]). On daily bars and
/// on bars shorter than one hour the detections are bit-identical to `r3`'s; only the
/// version (part of the ledger key) differs, so the two detectors keep ledger rows of
/// their own. Productized from `rounds/RP4/experiments/M26/mevsim` (choice in
/// `M26/RESULTS.md`).
///
/// # Panics
/// Never (valid constants).
#[must_use]
pub fn rp4_detectors() -> Vec<Box<dyn Detector>> {
    vec![
        Box::new(MaStack::default().detector()),
        Box::new(RsiDivergence::default().detector()),
        Box::new(
            SeasonalBreakout::new(
                RangeBreakout::WINDOW,
                RangeBreakout::ATR,
                SEASON_DAYS,
                SEASON_MIN_DAYS,
                SEASON_MIN_LEN_MS,
            )
            .expect("valid")
            .detector(),
        ),
        Box::new(BandSqueeze::default().detector()),
        Box::new(RsiExtreme::default().detector()),
        Box::new(
            SeasonalVolumeSurge::new(
                VolumeSurge::WINDOW,
                VolumeSurge::MULT,
                SEASON_DAYS,
                SEASON_MIN_DAYS,
                SEASON_MIN_LEN_MS,
            )
            .expect("valid")
            .detector(),
        ),
        Box::new(InstitutionalFlow::default().detector()),
    ]
}

/// (RP4) Named detector sets (`docs/CAPSULE.md` §9.9): `r3` ([`default_detectors`]) and
/// `rp4` ([`rp4_detectors`]). The protocol default is `r3`.
pub const DETECTOR_SETS: &[&str] = &["r3", "rp4"];

/// (RP4) A fresh detector set by name ([`DETECTOR_SETS`]); `None` for an unknown name.
#[must_use]
pub fn detectors_named(name: &str) -> Option<Vec<Box<dyn Detector>>> {
    match name {
        "r3" => Some(default_detectors()),
        "rp4" => Some(rp4_detectors()),
        _ => None,
    }
}
