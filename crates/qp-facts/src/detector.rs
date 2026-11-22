//! The [`Detector`] and [`Engine`] traits, their inputs and outputs, and the staging
//! wrapper [`Staged`] (out of `lib.rs`).

use std::collections::BTreeMap;

use qp_schema::capsule::{
    Direction, Edge, EvidenceFamily, Geometry, License, MeaningCondition, MeaningKind, TimeKind,
};
use qp_schema::Bar;

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
