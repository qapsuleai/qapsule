//! `squeeze.bandwidth` v1: Bollinger bandwidth at its lowest (volatility squeeze) state.
//!
//! Bar `i`: `bollinger(20, 2)` of close (`qp-compute`): `middle`, `upper`, `lower`,
//! `bw` = its `bandwidth` output (`2 * k * std / middle`, i.e. `(upper - lower) / middle`
//! up to rounding: the engine's value is used as is). `bw_min = lowest(120)` and
//! `bw_max = highest(120)` of `bw` over the last 120 bars that have a `bw` (incl. bar `i`).
//!
//! - Not in a squeeze and `bw_min` warm and `bw <= bw_min` → `Enter` (code `squeeze`,
//!   `Neutral`); the entry's `bw` is remembered as `bw_entry`.
//! - In a squeeze and `bw > 1.5 * bw_entry` → `Exit` (code `squeeze_end`).
//! - Strength: `(bw_max - bw) / bw_max` clipped (0 when `bw_max <= 0`).
//! - Evidence `upper`, `lower`, `middle`, `bw`, `bw_min`, `bw_max`, `half_width =
//!   upper - middle` (+ `bw_entry` on exit). Geometry `Band { upper, lower,
//!   entry - 19 ..= i }`. Condition on enter: `close_dev = |close - middle|` `Gt`
//!   `half_width` within 20 bars (a close outside the band).

use qp_compute::{Bollinger, ComputeError, Highest, Lowest};
use qp_schema::capsule::{
    Direction, Edge, EvidenceFamily, Geometry, MeaningCondition, MeaningKind, TimeKind,
};
use qp_schema::{Bar, CompareOp};

use crate::{clip01, evidence, Detection, Engine, FlowPoint, Staged};

/// `squeeze.bandwidth` engine (see the module docs).
#[derive(Debug, Clone)]
pub struct BandSqueeze {
    bb: Bollinger,
    period: usize,
    bw_lo: Lowest,
    bw_hi: Highest,
    exit_mult: f64,
    /// `(entry bar, bw at entry)` while in a squeeze.
    active: Option<(u64, f64)>,
    n: u64,
}

/// Values of one bar.
#[derive(Debug, Clone)]
pub struct BandSqueezeStep {
    bw: Option<f64>,
    active: Option<(u64, f64)>,
}

impl BandSqueeze {
    /// Bollinger period.
    pub const PERIOD: usize = 20;
    /// Bollinger width (standard deviations).
    pub const K: f64 = 2.0;
    /// Lookback of the bandwidth minimum.
    pub const LOOKBACK: usize = 120;
    /// Exit when the bandwidth exceeds this multiple of the entry bandwidth.
    pub const EXIT_MULT: f64 = 1.5;
    /// Condition horizon (bars).
    pub const EXPECT_BARS: u32 = 20;

    /// # Errors
    /// Invalid period or `k`.
    pub fn new(
        period: usize,
        k: f64,
        lookback: usize,
        exit_mult: f64,
    ) -> Result<Self, ComputeError> {
        Ok(Self {
            bb: Bollinger::new(period, k)?,
            period,
            bw_lo: Lowest::new(lookback)?,
            bw_hi: Highest::new(lookback)?,
            exit_mult,
            active: None,
            n: 0,
        })
    }

    /// As a [`crate::Detector`].
    #[must_use]
    pub fn detector(self) -> Staged<Self> {
        Staged::new(self)
    }
}

impl Default for BandSqueeze {
    fn default() -> Self {
        Self::new(Self::PERIOD, Self::K, Self::LOOKBACK, Self::EXIT_MULT).expect("valid defaults")
    }
}

impl Engine for BandSqueeze {
    type Step = BandSqueezeStep;

    fn id(&self) -> &'static str {
        "squeeze.bandwidth"
    }
    fn version(&self) -> u32 {
        1
    }
    fn family(&self) -> EvidenceFamily {
        EvidenceFamily::Price
    }
    fn warmup(&self) -> usize {
        self.period - 1 + self.bw_lo.period() - 1
    }

    fn eval(&self, bar: &Bar, _flow: Option<&FlowPoint>) -> (BandSqueezeStep, Vec<Detection>) {
        let Some(v) = self.bb.peek1(bar.c) else {
            return (
                BandSqueezeStep {
                    bw: None,
                    active: self.active,
                },
                Vec::new(),
            );
        };
        let bw = v.bandwidth;
        let (Some(bw_min), Some(bw_max)) = (self.bw_lo.peek1(bw), self.bw_hi.peek1(bw)) else {
            return (
                BandSqueezeStep {
                    bw: Some(bw),
                    active: self.active,
                },
                Vec::new(),
            );
        };
        let i = self.n;
        let (edge, active) = match self.active {
            None if bw <= bw_min => (Some(Edge::Enter), Some((i, bw))),
            Some((_, e)) if bw > self.exit_mult * e => (Some(Edge::Exit), None),
            a => (None, a),
        };
        let mut out = Vec::new();
        if let Some(edge) = edge {
            let mut ev = evidence(&[
                ("upper", v.upper),
                ("lower", v.lower),
                ("middle", v.middle),
                ("bw", bw),
                ("bw_min", bw_min),
                ("bw_max", bw_max),
                ("half_width", v.upper - v.middle),
            ]);
            let entry = match (edge, self.active) {
                (Edge::Exit, Some((e, bwe))) => {
                    ev.insert("bw_entry".to_owned(), bwe);
                    e
                }
                _ => i,
            };
            let span = self.period as u64 - 1;
            out.push(Detection {
                code: if edge == Edge::Enter {
                    "squeeze"
                } else {
                    "squeeze_end"
                },
                kind: MeaningKind::Squeeze,
                direction: Direction::Neutral,
                time: TimeKind::State,
                edge,
                strength: if bw_max > 0.0 {
                    clip01((bw_max - bw) / bw_max)
                } else {
                    0.0
                },
                evidence: ev,
                geometry: vec![Geometry::Band {
                    upper: "upper".to_owned(),
                    lower: "lower".to_owned(),
                    from_bar: entry.saturating_sub(span),
                    to_bar: i,
                }],
                condition: (edge == Edge::Enter).then(|| MeaningCondition {
                    subject: "close_dev".to_owned(),
                    op: CompareOp::Gt,
                    key: "half_width".to_owned(),
                    within_bars: Some(Self::EXPECT_BARS),
                }),
            });
        }
        (
            BandSqueezeStep {
                bw: Some(bw),
                active,
            },
            out,
        )
    }

    fn commit(&mut self, bar: &Bar, step: BandSqueezeStep) {
        self.bb.commit1(bar.c);
        if let Some(bw) = step.bw {
            self.bw_lo.commit1(bw);
            self.bw_hi.commit1(bw);
        }
        self.active = step.active;
        self.n += 1;
    }
}
