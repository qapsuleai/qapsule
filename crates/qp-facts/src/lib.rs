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
mod extreme_rsi;
mod flow_institutional;
mod ma_stack;
mod rsi_divergence;
mod squeeze_bandwidth;
mod volume_surge;

pub use breakout_range::RangeBreakout;
pub use extreme_rsi::RsiExtreme;
pub use flow_institutional::InstitutionalFlow;
pub use ma_stack::MaStack;
pub use rsi_divergence::RsiDivergence;
pub use squeeze_bandwidth::BandSqueeze;
pub use volume_surge::VolumeSurge;

use std::collections::BTreeMap;

use qp_schema::capsule::{
    Direction, Edge, EvidenceFamily, FactMeaning, Geometry, License, MeaningCondition, MeaningKind,
    TimeKind,
};
use qp_schema::{Author, Bar, Fact, Provenance};

/// One observation of a licensed flow feed at a bar (server-only data, K33). Keys are
/// feed-defined (`net`, `buy`, `sell`, …), e.g. one broker's net buying on that bar.
#[derive(Debug, Clone, PartialEq)]
pub struct FlowPoint {
    /// Bar time it belongs to (same alignment as the bar series).
    pub t: i64,
    /// Values.
    pub values: BTreeMap<String, f64>,
}

/// What a detector found on a bar.
#[derive(Debug, Clone, PartialEq)]
pub struct Detection {
    /// Detector-local code, e.g. `rsi_above_70`; becomes part of the fact kind.
    pub code: &'static str,
    /// Meaning kind.
    pub kind: MeaningKind,
    /// Direction.
    pub direction: Direction,
    /// Time kind.
    pub time: TimeKind,
    /// Point, enter or exit.
    pub edge: Edge,
    /// Strength in `[0, 1]`.
    pub strength: f64,
    /// Evidence values (bit-identical to the engine's values).
    pub evidence: BTreeMap<String, f64>,
    /// Geometry (keys into `evidence`).
    pub geometry: Vec<Geometry>,
    /// Condition slot (expectation / state).
    pub condition: Option<MeaningCondition>,
}

/// A streaming fact detector.
pub trait Detector {
    /// Stable id, e.g. `breakout.range`.
    fn id(&self) -> &'static str;
    /// Version; bump when the definition changes (part of the ledger key).
    fn version(&self) -> u32;
    /// Evidence family of every detection.
    fn family(&self) -> EvidenceFamily;
    /// License of the data it reads.
    fn license(&self) -> License;
    /// Bars before the first possible detection.
    fn warmup(&self) -> usize;
    /// Commit the pending bar, make `bar` pending, return detections as if it closed.
    fn push(&mut self, bar: &Bar, flow: Option<&FlowPoint>) -> Vec<Detection>;
    /// Replace the pending bar, return detections as if it closed.
    fn update_last(&mut self, bar: &Bar, flow: Option<&FlowPoint>) -> Vec<Detection>;
}

/// A detector's committed state plus its per-bar rule (see the crate docs).
///
/// Contract: `eval` must not change observable state and must be a function of the
/// committed state and its arguments only; `commit(bar, step)` with the `step`
/// that `eval(bar, flow)` returned must leave the state exactly as if that bar had been
/// added. Then [`Staged`] gives batch = streaming = revised, bit for bit.
pub trait Engine {
    /// Everything `commit` needs from `eval` (study outputs of the bar, the new state).
    type Step: Clone + std::fmt::Debug;
    /// See [`Detector::id`].
    fn id(&self) -> &'static str;
    /// See [`Detector::version`].
    fn version(&self) -> u32;
    /// See [`Detector::family`].
    fn family(&self) -> EvidenceFamily;
    /// See [`Detector::license`]. Default: public.
    fn license(&self) -> License {
        License {
            class: qp_schema::capsule::LicenseClass::Public,
            entitlement: None,
        }
    }
    /// See [`Detector::warmup`].
    fn warmup(&self) -> usize;
    /// Detections of the next bar (index = number of committed bars) on top of the
    /// committed state, and the step to commit it with.
    fn eval(&self, bar: &Bar, flow: Option<&FlowPoint>) -> (Self::Step, Vec<Detection>);
    /// Folds the bar into the committed state.
    fn commit(&mut self, bar: &Bar, step: Self::Step);
}

/// An [`Engine`] with a pending (still revisable) last bar: a [`Detector`].
#[derive(Debug, Clone)]
pub struct Staged<E: Engine> {
    engine: E,
    pending: Option<(Bar, E::Step)>,
}

impl<E: Engine> Staged<E> {
    /// Wraps an engine with no bars.
    #[must_use]
    pub fn new(engine: E) -> Self {
        Self {
            engine,
            pending: None,
        }
    }

    /// The engine (committed state only).
    #[must_use]
    pub fn engine(&self) -> &E {
        &self.engine
    }

    fn stage(&mut self, bar: &Bar, flow: Option<&FlowPoint>) -> Vec<Detection> {
        let (step, out) = self.engine.eval(bar, flow);
        self.pending = Some((bar.clone(), step));
        out
    }
}

impl<E: Engine> Detector for Staged<E> {
    fn id(&self) -> &'static str {
        self.engine.id()
    }
    fn version(&self) -> u32 {
        self.engine.version()
    }
    fn family(&self) -> EvidenceFamily {
        self.engine.family()
    }
    fn license(&self) -> License {
        self.engine.license()
    }
    fn warmup(&self) -> usize {
        self.engine.warmup()
    }
    fn push(&mut self, bar: &Bar, flow: Option<&FlowPoint>) -> Vec<Detection> {
        if let Some((b, step)) = self.pending.take() {
            self.engine.commit(&b, step);
        }
        self.stage(bar, flow)
    }
    fn update_last(&mut self, bar: &Bar, flow: Option<&FlowPoint>) -> Vec<Detection> {
        // Without a pending bar this is a push of the first bar.
        self.stage(bar, flow)
    }
}

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

/// Evidence map from `(key, value)` pairs.
pub(crate) fn evidence(kv: &[(&str, f64)]) -> BTreeMap<String, f64> {
    kv.iter().map(|(k, v)| ((*k).to_owned(), *v)).collect()
}

/// `x` clipped to `[0, 1]`; `NaN` → 0.
pub(crate) fn clip01(x: f64) -> f64 {
    if x > 1.0 {
        1.0
    } else if x > 0.0 {
        x
    } else {
        0.0
    }
}

/// Bar index as `f64` (evidence); exact below 2^53.
#[allow(clippy::cast_precision_loss)]
pub(crate) fn bar_f64(i: u64) -> f64 {
    i as f64
}

/// Run a detector over all bars: `(bar index, detection)` in bar order. Defined as
/// streaming every bar, so batch = streaming by construction.
pub fn run_batch(
    det: &mut dyn Detector,
    bars: &[Bar],
    flow: Option<&[FlowPoint]>,
) -> Vec<(u64, Detection)> {
    let mut out = Vec::new();
    for (i, b) in bars.iter().enumerate() {
        let f = flow.and_then(|fl| fl.get(i));
        for d in det.push(b, f) {
            out.push((i as u64, d));
        }
    }
    out
}

/// Deterministic fact id: `fx.<instrument>.<detector>.<code>.<bar>`.
#[must_use]
pub fn fact_id(instrument: &str, detector: &str, code: &str, bar: u64) -> String {
    format!("fx.{instrument}.{detector}.{code}.{bar}")
}

/// Build the typed fact for a detection. `series` is the bar series id; `t` the bar time;
/// `engine_version` goes into the provenance. The `evidence_ref` is left empty: the
/// authority stamps it (RP2 §18.5).
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn to_fact(
    det: &dyn Detector,
    d: &Detection,
    instrument: &str,
    series: &str,
    bar: u64,
    t: i64,
    engine_version: &str,
) -> Fact {
    let license = det.license();
    let server = matches!(license.class, qp_schema::capsule::LicenseClass::Licensed);
    Fact {
        id: fact_id(instrument, det.id(), d.code, bar),
        kind: format!("{}.{}", det.id(), d.code),
        series: series.to_owned(),
        bar,
        t,
        text: format!("{} {} at bar {bar}", det.id(), d.code),
        evidence: d.evidence.clone(),
        evidence_ref: None,
        meaning: Some(FactMeaning {
            kind: d.kind,
            direction: d.direction,
            family: det.family(),
            time: d.time,
            edge: d.edge,
            detector: det.id().to_owned(),
            detector_version: det.version(),
            instrument: instrument.to_owned(),
            strength: d.strength,
            license,
            geometry: d.geometry.clone(),
            condition: d.condition.clone(),
            server,
        }),
        provenance: Provenance {
            author: Author::Engine {
                name: "qp-facts".to_owned(),
                version: engine_version.to_owned(),
            },
            at: t,
            based_on: vec![series.to_owned()],
            approved_by: None,
        },
    }
}
