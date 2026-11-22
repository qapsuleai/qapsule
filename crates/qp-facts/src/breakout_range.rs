//! `breakout.range` v1: close beyond the previous 20 bars' range.
//!
//! Bar `i >= 20`: `hh = highest(20)` of high at bar `i - 1` (= max of `high[i-20..=i-1]`,
//! the current bar excluded), `ll = lowest(20)` of low at bar `i - 1`, `atr = atr(14)` at
//! bar `i`.
//!
//! - `close > hh` → `Up`, code `up`, `level = hh`; `close < ll` → `Down`, code `down`,
//!   `level = ll`. Point event; it fires on every bar that closes beyond its own window.
//! - Strength: `|close - level| / atr` clipped (0 when `atr <= 0`).
//! - Evidence `level`, `close`, `atr`. Geometry `Level { level, i - 20 ..= i }`.
//!   Condition: `close` `Gt` (`Lt`) `level` within 3 bars (the breakout holds).

use qp_compute::{Atr, ComputeError, Highest, Lowest};
use qp_schema::capsule::{
    Direction, Edge, EvidenceFamily, Geometry, MeaningCondition, MeaningKind, TimeKind,
};
use qp_schema::{Bar, CompareOp};

use crate::{clip01, evidence, Detection, Engine, FlowPoint, Staged};

/// `breakout.range` engine (see the module docs).
#[derive(Debug, Clone)]
pub struct RangeBreakout {
    hi: Highest,
    lo: Lowest,
    atr: Atr,
    /// `highest` / `lowest` at the last committed bar.
    prev_hh: Option<f64>,
    prev_ll: Option<f64>,
    n: u64,
}

/// Values of one bar.
#[derive(Debug, Clone)]
pub struct RangeBreakoutStep {
    hh: Option<f64>,
    ll: Option<f64>,
}

impl RangeBreakout {
    /// Range window (bars before the current one).
    pub const WINDOW: usize = 20;
    /// ATR period.
    pub const ATR: usize = 14;
    /// Condition horizon.
    pub const HOLD_BARS: u32 = 3;

    /// # Errors
    /// Invalid period.
    pub fn new(window: usize, atr: usize) -> Result<Self, ComputeError> {
        Ok(Self {
            hi: Highest::new(window)?,
            lo: Lowest::new(window)?,
            atr: Atr::new(atr)?,
            prev_hh: None,
            prev_ll: None,
            n: 0,
        })
    }

    /// As a [`crate::Detector`].
    #[must_use]
    pub fn detector(self) -> Staged<Self> {
        Staged::new(self)
    }

    fn detection(&self, up: bool, level: f64, c: f64, atr: Option<f64>) -> Detection {
        let strength = match atr {
            Some(a) if a > 0.0 => clip01((c - level).abs() / a),
            _ => 0.0,
        };
        let mut ev = evidence(&[("level", level), ("close", c)]);
        if let Some(a) = atr {
            ev.insert("atr".to_owned(), a);
        }
        let i = self.n;
        let w = self.hi.period() as u64;
        Detection {
            code: if up { "up" } else { "down" },
            kind: MeaningKind::Breakout,
            direction: if up { Direction::Up } else { Direction::Down },
            time: TimeKind::Event,
            edge: Edge::Point,
            strength,
            evidence: ev,
            geometry: vec![Geometry::Level {
                key: "level".to_owned(),
                from_bar: i - w,
                to_bar: i,
            }],
            condition: Some(MeaningCondition {
                subject: "close".to_owned(),
                op: if up { CompareOp::Gt } else { CompareOp::Lt },
                key: "level".to_owned(),
                within_bars: Some(Self::HOLD_BARS),
            }),
        }
    }
}

impl Default for RangeBreakout {
    #[allow(clippy::expect_used)]
    fn default() -> Self {
        Self::new(Self::WINDOW, Self::ATR).expect("valid defaults")
    }
}

impl Engine for RangeBreakout {
    type Step = RangeBreakoutStep;

    fn id(&self) -> &'static str {
        "breakout.range"
    }
    fn version(&self) -> u32 {
        1
    }
    fn family(&self) -> EvidenceFamily {
        EvidenceFamily::Price
    }
    fn warmup(&self) -> usize {
        self.hi.period()
    }

    fn eval(&self, bar: &Bar, _flow: Option<&FlowPoint>) -> (RangeBreakoutStep, Vec<Detection>) {
        let step = RangeBreakoutStep {
            hh: self.hi.peek1(bar.h),
            ll: self.lo.peek1(bar.l),
        };
        let atr = self.atr.peek_hl(bar.h, bar.l);
        let c = bar.c;
        let mut out = Vec::new();
        if let Some(hh) = self.prev_hh {
            if c > hh {
                out.push(self.detection(true, hh, c, atr));
            }
        }
        if let Some(ll) = self.prev_ll {
            if c < ll {
                out.push(self.detection(false, ll, c, atr));
            }
        }
        (step, out)
    }

    fn commit(&mut self, bar: &Bar, step: RangeBreakoutStep) {
        self.hi.commit1(bar.h);
        self.lo.commit1(bar.l);
        self.atr.commit_hlc(bar.h, bar.l, bar.c);
        self.prev_hh = step.hh;
        self.prev_ll = step.ll;
        self.n += 1;
    }
}
