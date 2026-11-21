//! `breakout.range` v2 (RP4/M26 `mevsim`; in the `rp4` detector set): range breakout with
//! a seasonal volatility margin.
//!
//! Provenance: `rounds/RP4/experiments/M26/mevsim/src/breakout.rs`, bit-identical
//! (cross-check in `rounds/RP4/experiments/harness/tests/rp4_product.rs`).
//!
//! Product v1: `close > hh` (`close < ll`), `hh`/`ll` the previous 20 bars' high/low.
//! Intraday volatility is U-shaped: the first bars of the day (overnight gap, opening
//! auction) move more than the previous 20 bars did, so v1 fires there by construction.
//!
//! v2: `g` = the true-range [`Season`] factor of the bar's slot (past days only);
//! margin `m = max(0, g - 1) * atr(14)`. Up when `close > hh + m`, down when
//! `close < ll - m`. In slots no more volatile than the day's average (`g <= 1`) this is
//! v1. Level, strength, evidence, geometry and condition as v1 (`level` = `hh` / `ll`),
//! plus (when active) evidence `season` (= `g`) and `margin`.
//!
//! Not active (daily bars, or bars shorter than the period floor) → `g = 1.0`, `m = 0`:
//! the same detections as v1.

use qp_compute::{Atr, ComputeError, Highest, Lowest};
use qp_schema::capsule::{
    Direction, Edge, EvidenceFamily, Geometry, MeaningCondition, MeaningKind, TimeKind,
};
use qp_schema::{Bar, CompareOp};

use crate::season::{Pos, Season};
use crate::{clip01, evidence, Detection, Engine, FlowPoint, Staged};

/// Seasonal `breakout.range` engine.
#[derive(Debug, Clone)]
pub struct SeasonalBreakout {
    season: Season,
    hi: Highest,
    lo: Lowest,
    atr: Atr,
    prev_hh: Option<f64>,
    prev_ll: Option<f64>,
    prev_c: Option<f64>,
    n: u64,
}

/// Values of one bar.
#[derive(Debug, Clone)]
pub struct SeasonalBreakoutStep {
    pos: Pos,
    tr: f64,
    hh: Option<f64>,
    ll: Option<f64>,
}

/// Condition horizon (as v1).
const HOLD_BARS: u32 = 3;

impl SeasonalBreakout {
    /// # Errors
    /// Invalid period.
    pub fn new(
        window: usize,
        atr: usize,
        days: usize,
        min_days: usize,
        min_len_ms: i64,
    ) -> Result<Self, ComputeError> {
        Ok(Self {
            season: Season::new(days, min_days, min_len_ms),
            hi: Highest::new(window)?,
            lo: Lowest::new(window)?,
            atr: Atr::new(atr)?,
            prev_hh: None,
            prev_ll: None,
            prev_c: None,
            n: 0,
        })
    }

    /// As a [`crate::Detector`].
    #[must_use]
    pub fn detector(self) -> Staged<Self> {
        Staged::new(self)
    }

    #[allow(clippy::too_many_arguments)]
    fn detection(
        &self,
        up: bool,
        level: f64,
        c: f64,
        atr: Option<f64>,
        pos: &Pos,
        g: f64,
        margin: f64,
    ) -> Detection {
        let strength = match atr {
            Some(a) if a > 0.0 => clip01((c - level).abs() / a),
            _ => 0.0,
        };
        let mut ev = evidence(&[("level", level), ("close", c)]);
        if let Some(a) = atr {
            ev.insert("atr".to_owned(), a);
        }
        if pos.active() {
            ev.insert("season".to_owned(), g);
            ev.insert("margin".to_owned(), margin);
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
                within_bars: Some(HOLD_BARS),
            }),
        }
    }
}

impl Engine for SeasonalBreakout {
    type Step = SeasonalBreakoutStep;

    fn id(&self) -> &'static str {
        "breakout.range"
    }
    fn version(&self) -> u32 {
        2
    }
    fn family(&self) -> EvidenceFamily {
        EvidenceFamily::Price
    }
    fn warmup(&self) -> usize {
        self.hi.period()
    }

    fn eval(&self, bar: &Bar, _flow: Option<&FlowPoint>) -> (SeasonalBreakoutStep, Vec<Detection>) {
        let pos = self.season.locate(bar.t);
        let g = self.season.factor(&pos);
        let tr = match self.prev_c {
            Some(pc) => bar.h.max(pc) - bar.l.min(pc),
            None => bar.h - bar.l,
        };
        let step = SeasonalBreakoutStep {
            pos,
            tr,
            hh: self.hi.peek1(bar.h),
            ll: self.lo.peek1(bar.l),
        };
        let atr = self.atr.peek_hl(bar.h, bar.l);
        let margin = match atr {
            Some(a) if g > 1.0 => (g - 1.0) * a,
            _ => 0.0,
        };
        let c = bar.c;
        let mut out = Vec::new();
        if let Some(hh) = self.prev_hh {
            if c > hh + margin {
                out.push(self.detection(true, hh, c, atr, &pos, g, margin));
            }
        }
        if let Some(ll) = self.prev_ll {
            if c < ll - margin {
                out.push(self.detection(false, ll, c, atr, &pos, g, margin));
            }
        }
        (step, out)
    }

    fn commit(&mut self, bar: &Bar, step: SeasonalBreakoutStep) {
        self.hi.commit1(bar.h);
        self.lo.commit1(bar.l);
        self.atr.commit_hlc(bar.h, bar.l, bar.c);
        self.season.commit(&step.pos, step.tr);
        self.prev_hh = step.hh;
        self.prev_ll = step.ll;
        self.prev_c = Some(bar.c);
        self.n += 1;
    }
}
