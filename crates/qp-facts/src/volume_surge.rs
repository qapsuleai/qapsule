//! `flow.volume_surge` v1: unusually high traded volume (Volume family).
//!
//! Bar `i >= 20`: `vol_avg = sma(20)` of volume at bar `i - 1` (the previous 20 bars, the
//! current one excluded).
//!
//! - `vol_avg > 0 && volume > 2.5 * vol_avg` → point event, code `surge`; direction by
//!   `close - open`: `Up` if > 0, `Down` if < 0, `Neutral` if 0.
//! - `ratio = volume / vol_avg`; strength `(ratio - 2.5) / 2.5` clipped.
//! - Evidence `volume`, `vol_avg`, `ratio`.

use qp_compute::{ComputeError, Sma};
use qp_schema::capsule::{Direction, Edge, EvidenceFamily, MeaningKind, TimeKind};
use qp_schema::Bar;

use crate::{clip01, evidence, Detection, Engine, FlowPoint, Staged};

/// `flow.volume_surge` engine (see the module docs).
#[derive(Debug, Clone)]
pub struct VolumeSurge {
    sma: Sma,
    mult: f64,
    /// `sma` at the last committed bar.
    prev_avg: Option<f64>,
}

/// Values of one bar.
#[derive(Debug, Clone)]
pub struct VolumeSurgeStep {
    avg: Option<f64>,
}

impl VolumeSurge {
    /// Average window (bars before the current one).
    pub const WINDOW: usize = 20;
    /// Surge threshold as a multiple of the average.
    pub const MULT: f64 = 2.5;

    /// # Errors
    /// Invalid window.
    pub fn new(window: usize, mult: f64) -> Result<Self, ComputeError> {
        Ok(Self {
            sma: Sma::new(window)?,
            mult,
            prev_avg: None,
        })
    }

    /// As a [`crate::Detector`].
    #[must_use]
    pub fn detector(self) -> Staged<Self> {
        Staged::new(self)
    }
}

impl Default for VolumeSurge {
    fn default() -> Self {
        Self::new(Self::WINDOW, Self::MULT).expect("valid defaults")
    }
}

impl Engine for VolumeSurge {
    type Step = VolumeSurgeStep;

    fn id(&self) -> &'static str {
        "flow.volume_surge"
    }
    fn version(&self) -> u32 {
        1
    }
    fn family(&self) -> EvidenceFamily {
        EvidenceFamily::Volume
    }
    fn warmup(&self) -> usize {
        self.sma.period()
    }

    fn eval(&self, bar: &Bar, _flow: Option<&FlowPoint>) -> (VolumeSurgeStep, Vec<Detection>) {
        let step = VolumeSurgeStep {
            avg: self.sma.peek1(bar.v),
        };
        let mut out = Vec::new();
        if let Some(avg) = self.prev_avg {
            let v = bar.v;
            if avg > 0.0 && v > self.mult * avg {
                let ratio = v / avg;
                let d = bar.c - bar.o;
                out.push(Detection {
                    code: "surge",
                    kind: MeaningKind::Flow,
                    direction: if d > 0.0 {
                        Direction::Up
                    } else if d < 0.0 {
                        Direction::Down
                    } else {
                        Direction::Neutral
                    },
                    time: TimeKind::Event,
                    edge: Edge::Point,
                    strength: clip01((ratio - self.mult) / self.mult),
                    evidence: evidence(&[("volume", v), ("vol_avg", avg), ("ratio", ratio)]),
                    geometry: Vec::new(),
                    condition: None,
                });
            }
        }
        (step, out)
    }

    fn commit(&mut self, bar: &Bar, step: VolumeSurgeStep) {
        self.sma.commit1(bar.v);
        self.prev_avg = step.avg;
    }
}
