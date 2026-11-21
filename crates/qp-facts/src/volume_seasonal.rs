//! `flow.volume_surge` v2 (RP4/M26 `mevsim`; in the `rp4` detector set): volume surge on
//! **deseasonalized** volume.
//!
//! Product v1 ([`crate::VolumeSurge`]): `volume > 2.5 * sma(20)` of the previous 20 bars'
//! volume. Intraday volume is U-shaped (opening and closing auction high, midday low), so
//! v1 fires on the day's last bar (closing auction merged) and first bars by construction.
//!
//! v2: `f` = the volume [`Season`] factor of the bar's slot (same time of day, past days
//! only); `u = volume / f`. Surge when `u > mult * avg`, `avg` = `sma(20)` of the previous
//! 20 bars' `u`. Equivalently `volume > mult * f * avg`: the threshold scales with the
//! slot's typical volume. Direction, code, strength as v1 (on `ratio = u / avg`).
//! Evidence `volume`, `vol_avg` (= `f * avg`, the expected volume of the slot),
//! `ratio`, and (when active) `season` (= `f`).
//!
//! Not active (daily bars, or bars shorter than the period floor) → `f = 1.0` exactly,
//! `u = volume`: the same detections as v1 (evidence and strength bit-identical; only
//! the version differs).
//!
//! Provenance: `rounds/RP4/experiments/M26/mevsim/src/volume.rs` without its trial-only
//! gate mode (`gate = false` there), bit-identical (cross-check in
//! `rounds/RP4/experiments/harness/tests/rp4_product.rs`).

use qp_compute::{ComputeError, Sma};
use qp_schema::capsule::{Direction, Edge, EvidenceFamily, MeaningKind, TimeKind};
use qp_schema::Bar;

use crate::season::{Pos, Season};
use crate::{clip01, evidence, Detection, Engine, FlowPoint, Staged};

/// Seasonal `flow.volume_surge` engine.
#[derive(Debug, Clone)]
pub struct SeasonalVolumeSurge {
    season: Season,
    sma: Sma,
    mult: f64,
    prev_avg: Option<f64>,
}

/// Values of one bar.
#[derive(Debug, Clone)]
pub struct SeasonalVolumeSurgeStep {
    pos: Pos,
    u: f64,
    avg: Option<f64>,
}

impl SeasonalVolumeSurge {
    /// Volume window `window`, multiple `mult`; seasonal profile over `days` past days,
    /// warm from `min_days`, active for bars of at least `min_len_ms`.
    ///
    /// # Errors
    /// Invalid window.
    pub fn new(
        window: usize,
        mult: f64,
        days: usize,
        min_days: usize,
        min_len_ms: i64,
    ) -> Result<Self, ComputeError> {
        Ok(Self {
            season: Season::new(days, min_days, min_len_ms),
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

impl Engine for SeasonalVolumeSurge {
    type Step = SeasonalVolumeSurgeStep;

    fn id(&self) -> &'static str {
        "flow.volume_surge"
    }
    fn version(&self) -> u32 {
        2
    }
    fn family(&self) -> EvidenceFamily {
        EvidenceFamily::Volume
    }
    fn warmup(&self) -> usize {
        self.sma.period()
    }

    fn eval(
        &self,
        bar: &Bar,
        _flow: Option<&FlowPoint>,
    ) -> (SeasonalVolumeSurgeStep, Vec<Detection>) {
        let pos = self.season.locate(bar.t);
        let f = self.season.factor(&pos);
        let v = bar.v;
        let u = v / f;
        let step = SeasonalVolumeSurgeStep {
            pos,
            u,
            avg: self.sma.peek1(u),
        };
        let mut out = Vec::new();
        if let Some(avg) = self.prev_avg {
            if avg > 0.0 && u > self.mult * avg {
                let ratio = u / avg;
                let d = bar.c - bar.o;
                let mut ev = evidence(&[("volume", v), ("vol_avg", f * avg), ("ratio", ratio)]);
                if pos.active() {
                    ev.insert("season".to_owned(), f);
                }
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
                    evidence: ev,
                    geometry: Vec::new(),
                    condition: None,
                });
            }
        }
        (step, out)
    }

    fn commit(&mut self, bar: &Bar, step: SeasonalVolumeSurgeStep) {
        self.sma.commit1(step.u);
        self.season.commit(&step.pos, bar.v);
        self.prev_avg = step.avg;
    }
}
