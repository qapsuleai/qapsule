//! `reversal.rsi_divergence` v1: price/RSI divergence at confirmed swing points.
//!
//! - Swing high at bar `j`: `high[j]` strictly greater than the highs of bars `j-2`,
//!   `j-1`, `j+1`, `j+2`; swing low mirror on lows (strictly lower). A swing at `j` is
//!   *confirmed* on bar `j + 2` and the detection is emitted there (no look-ahead).
//! - Bearish (`Down`, code `bearish`): a newly confirmed swing high `p2` and the
//!   previous confirmed swing high `p1` (the immediately previous one, no search further
//!   back) with `5 <= p2.bar - p1.bar <= 40`, `p2.price > p1.price` and
//!   `p2.rsi < p1.rsi` (`rsi(14)` of close at those bars, both warm).
//! - Bullish (`Up`, `bullish`): mirror on swing lows (`p2.price < p1.price`,
//!   `p2.rsi > p1.rsi`).
//! - Strength: `|p1.rsi - p2.rsi| / 20` clipped. Point event.
//! - Evidence `p1_price`, `p2_price`, `p1_rsi`, `p2_rsi`, `p1_bar`, `p2_bar`; geometry
//!   points `(p1_bar, p1_price)`, `(p2_bar, p2_price)`.

use std::collections::VecDeque;

use qp_compute::{ComputeError, Rsi};
use qp_schema::capsule::{
    Direction, Edge, EvidenceFamily, GeoPoint, Geometry, MeaningKind, TimeKind,
};
use qp_schema::Bar;

use crate::{bar_f64, clip01, evidence, Detection, Engine, FlowPoint, Staged};

/// A confirmed swing point.
#[derive(Debug, Clone, Copy)]
pub struct Swing {
    bar: u64,
    price: f64,
    rsi: Option<f64>,
}

/// One committed bar as the swing finder needs it.
#[derive(Debug, Clone, Copy)]
struct Seen {
    h: f64,
    l: f64,
    rsi: Option<f64>,
}

/// `reversal.rsi_divergence` engine (see the module docs).
#[derive(Debug, Clone)]
pub struct RsiDivergence {
    rsi: Rsi,
    min_gap: u64,
    max_gap: u64,
    /// Last 4 committed bars (front = oldest).
    hist: VecDeque<Seen>,
    last_high: Option<Swing>,
    last_low: Option<Swing>,
    n: u64,
}

/// Values of one bar.
#[derive(Debug, Clone)]
pub struct RsiDivergenceStep {
    rsi: Option<f64>,
    high: Option<Swing>,
    low: Option<Swing>,
}

impl RsiDivergence {
    /// RSI period.
    pub const RSI: usize = 14;
    /// Fractal half-width (bars each side).
    pub const K: usize = 2;
    /// Smallest distance between the two swings, in bars.
    pub const MIN_GAP: u64 = 5;
    /// Largest distance between the two swings, in bars.
    pub const MAX_GAP: u64 = 40;
    /// RSI difference that maps to strength 1.
    pub const FULL_STRENGTH: f64 = 20.0;

    /// # Errors
    /// Invalid RSI period.
    pub fn new(rsi: usize, min_gap: u64, max_gap: u64) -> Result<Self, ComputeError> {
        Ok(Self {
            rsi: Rsi::new(rsi)?,
            min_gap,
            max_gap,
            hist: VecDeque::with_capacity(2 * Self::K + 1),
            last_high: None,
            last_low: None,
            n: 0,
        })
    }

    /// As a [`crate::Detector`].
    #[must_use]
    pub fn detector(self) -> Staged<Self> {
        Staged::new(self)
    }

    fn divergence(&self, p1: Option<Swing>, p2: Swing, bearish: bool) -> Option<Detection> {
        let p1 = p1?;
        let gap = p2.bar - p1.bar;
        if gap < self.min_gap || gap > self.max_gap {
            return None;
        }
        let (r1, r2) = (p1.rsi?, p2.rsi?);
        let hit = if bearish {
            p2.price > p1.price && r2 < r1
        } else {
            p2.price < p1.price && r2 > r1
        };
        if !hit {
            return None;
        }
        Some(Detection {
            code: if bearish { "bearish" } else { "bullish" },
            kind: MeaningKind::Reversal,
            direction: if bearish {
                Direction::Down
            } else {
                Direction::Up
            },
            time: TimeKind::Event,
            edge: Edge::Point,
            strength: clip01((r1 - r2).abs() / Self::FULL_STRENGTH),
            evidence: evidence(&[
                ("p1_price", p1.price),
                ("p2_price", p2.price),
                ("p1_rsi", r1),
                ("p2_rsi", r2),
                ("p1_bar", bar_f64(p1.bar)),
                ("p2_bar", bar_f64(p2.bar)),
            ]),
            geometry: vec![Geometry::Points {
                points: vec![
                    GeoPoint {
                        bar: p1.bar,
                        key: "p1_price".to_owned(),
                    },
                    GeoPoint {
                        bar: p2.bar,
                        key: "p2_price".to_owned(),
                    },
                ],
            }],
            condition: None,
        })
    }
}

impl Default for RsiDivergence {
    fn default() -> Self {
        Self::new(Self::RSI, Self::MIN_GAP, Self::MAX_GAP).expect("valid defaults")
    }
}

impl Engine for RsiDivergence {
    type Step = RsiDivergenceStep;

    fn id(&self) -> &'static str {
        "reversal.rsi_divergence"
    }
    fn version(&self) -> u32 {
        1
    }
    fn family(&self) -> EvidenceFamily {
        EvidenceFamily::Price
    }
    fn warmup(&self) -> usize {
        // First swing with an RSI value at bar `rsi`, the second `min_gap` later,
        // confirmed `K` bars after that.
        let gap = usize::try_from(self.min_gap).unwrap_or(usize::MAX);
        self.rsi
            .period()
            .saturating_add(gap)
            .saturating_add(Self::K)
    }

    fn eval(&self, bar: &Bar, _flow: Option<&FlowPoint>) -> (RsiDivergenceStep, Vec<Detection>) {
        let rsi = self.rsi.peek1(bar.c);
        let mut step = RsiDivergenceStep {
            rsi,
            high: self.last_high,
            low: self.last_low,
        };
        let mut out = Vec::new();
        if self.hist.len() == 2 * Self::K {
            // Candidate swing at j = i - 2 = hist[2]; neighbours hist[0], hist[1],
            // hist[3] and this bar.
            let (a, b, m, d) = (self.hist[0], self.hist[1], self.hist[2], self.hist[3]);
            let j = self.n - 2;
            if m.h > a.h && m.h > b.h && m.h > d.h && m.h > bar.h {
                let p2 = Swing {
                    bar: j,
                    price: m.h,
                    rsi: m.rsi,
                };
                out.extend(self.divergence(self.last_high, p2, true));
                step.high = Some(p2);
            }
            if m.l < a.l && m.l < b.l && m.l < d.l && m.l < bar.l {
                let p2 = Swing {
                    bar: j,
                    price: m.l,
                    rsi: m.rsi,
                };
                out.extend(self.divergence(self.last_low, p2, false));
                step.low = Some(p2);
            }
        }
        (step, out)
    }

    fn commit(&mut self, bar: &Bar, step: RsiDivergenceStep) {
        self.rsi.commit1(bar.c);
        self.hist.push_back(Seen {
            h: bar.h,
            l: bar.l,
            rsi: step.rsi,
        });
        if self.hist.len() > 2 * Self::K {
            self.hist.pop_front();
        }
        self.last_high = step.high;
        self.last_low = step.low;
        self.n += 1;
    }
}
