//! Qapsule fact detectors (R3, `Detector` = "Olgu dedektörü" in `docs/CONCEPTS.md`).
//!
//! A detector turns a bar series (and, for server facts, a licensed flow feed) into typed
//! facts in the six-kind meaning vocabulary (`qp_schema::capsule::MeaningKind`).
//!
//! Rules (same as `qp-compute`):
//!
//! - **Deterministic**: `f64`, IEEE basic operations in a fixed order; no clock, network,
//!   randomness or LLM.
//! - **Batch = streaming, bit-identical**: [`run_batch`] is defined as pushing every bar;
//!   the stream API follows `qp-compute`'s pending-bar model:
//!   - [`Detector::push`]: commit the pending bar (if any), make `bar` pending, return the
//!     detections *as if `bar` closed now*.
//!   - [`Detector::update_last`]: replace the pending bar (live revision), return the
//!     detections as if it closed now.
//!
//!   The detections of bar `i` depend only on bars `0..=i` (final inputs), whichever way
//!   they got there. A detection on a pending (open) bar is a **candidate**; it becomes a
//!   fact only when the bar is final (the authority decides finality; RP2 `Maturity`).
//! - **Edges, not levels**: detectors emit on change (a point event, a state's start or
//!   end), never "still true" on every bar.
//!
//! Contract version: R3/T1. Implementations: R3/T3 ([`default_detectors`]; formulas in
//! each module and in `rounds/R3/experiments/T3.md`).
//!
//! The implementations follow `qp-compute`'s `Core` model one level up: an [`Engine`]
//! holds the committed state (bars `0..i`), `eval` computes the pending bar's
//! detections and its would-be state change without changing anything, `commit` folds
//! the final bar in. [`Staged`] wraps an engine into a [`Detector`] with a pending bar.

mod breakout_range;
mod breakout_seasonal;
mod detector;
mod extreme_rsi;
mod fact;
mod flow_institutional;
mod ma_stack;
mod rsi_divergence;
pub mod season;
mod sets;
mod squeeze_bandwidth;
mod volume_seasonal;
mod volume_surge;

pub use breakout_range::RangeBreakout;
pub use breakout_seasonal::SeasonalBreakout;
pub use detector::{Detection, Detector, Engine, FlowPoint, Staged};
pub use extreme_rsi::RsiExtreme;
pub(crate) use fact::{bar_f64, clip01, evidence};
pub use fact::{fact_id, run_batch, to_fact};
pub use flow_institutional::InstitutionalFlow;
pub use ma_stack::MaStack;
pub use rsi_divergence::RsiDivergence;
pub use season::Season;
pub use sets::{
    default_detectors, detectors_named, rp4_detectors, DETECTOR_SETS, SEASON_DAYS, SEASON_MIN_DAYS,
    SEASON_MIN_LEN_MS,
};
pub use squeeze_bandwidth::BandSqueeze;
pub use volume_seasonal::SeasonalVolumeSurge;
pub use volume_surge::VolumeSurge;
