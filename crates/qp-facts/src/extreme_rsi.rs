//! `extreme.rsi` v1: RSI overbought / oversold state.
//!
//! Bar `i`: `v = rsi(14)` of close at `i`, `prev` = at `i - 1` (both needed; nothing
//! happens before bar 15).
//!
//! - Not in a zone: `prev <= 70 && v > 70` → `Enter` overbought (`Up`, code
//!   `overbought`, `level = 70`); `prev >= 30 && v < 30` → `Enter` oversold (`Down`,
//!   `oversold`, `level = 30`).
//! - Overbought and `v < 70` → `Exit` (`overbought_end`); oversold and `v > 30` → `Exit`
//!   (`oversold_end`). After an exit the enter rule of the other zone is checked on the
//!   same bar (a jump from > 70 to < 30 exits and enters).
//! - Strength: `|v - level| / 30` clipped.
//! - Evidence `level`, `value`, `prev`. Geometry `Level { level, i ..= i }`. Condition:
//!   `rsi` `Gt` (overbought) / `Lt` (oversold) `level` (does the state still hold, K26).

use qp_compute::{ComputeError, Rsi};
use qp_schema::capsule::{
    Direction, Edge, EvidenceFamily, Geometry, MeaningCondition, MeaningKind, TimeKind,
};
use qp_schema::{Bar, CompareOp};

use crate::{clip01, evidence, Detection, Engine, FlowPoint, Staged};

/// RSI zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Zone {
    None,
    Over,
    Under,
}

/// `extreme.rsi` engine (see the module docs).
#[derive(Debug, Clone)]
pub struct RsiExtreme {
    rsi: Rsi,
    upper: f64,
    lower: f64,
    prev: Option<f64>,
    zone: Zone,
    n: u64,
}

/// Values of one bar.
#[derive(Debug, Clone)]
pub struct RsiExtremeStep {
    value: Option<f64>,
    zone: Zone,
}

impl RsiExtreme {
    /// RSI period.
    pub const RSI: usize = 14;
    /// Overbought level.
    pub const UPPER: f64 = 70.0;
    /// Oversold level.
    pub const LOWER: f64 = 30.0;
    /// RSI distance from the level that maps to strength 1.
    pub const FULL_STRENGTH: f64 = 30.0;

    /// # Errors
    /// Invalid RSI period.
    pub fn new(rsi: usize, upper: f64, lower: f64) -> Result<Self, ComputeError> {
        Ok(Self {
            rsi: Rsi::new(rsi)?,
            upper,
            lower,
            prev: None,
            zone: Zone::None,
            n: 0,
        })
    }

    /// As a [`crate::Detector`].
    #[must_use]
    pub fn detector(self) -> Staged<Self> {
        Staged::new(self)
    }

    fn detection(&self, zone: Zone, edge: Edge, v: f64, prev: f64) -> Detection {
        let over = zone == Zone::Over;
        let level = if over { self.upper } else { self.lower };
        let code = match (over, edge) {
            (true, Edge::Enter) => "overbought",
            (true, _) => "overbought_end",
            (false, Edge::Enter) => "oversold",
            (false, _) => "oversold_end",
        };
        let i = self.n;
        Detection {
            code,
            kind: MeaningKind::Extreme,
            direction: if over { Direction::Up } else { Direction::Down },
            time: TimeKind::State,
            edge,
            strength: clip01((v - level).abs() / Self::FULL_STRENGTH),
            evidence: evidence(&[("level", level), ("value", v), ("prev", prev)]),
            geometry: vec![Geometry::Level {
                key: "level".to_owned(),
                from_bar: i,
                to_bar: i,
            }],
            condition: Some(MeaningCondition {
                subject: "rsi".to_owned(),
                op: if over { CompareOp::Gt } else { CompareOp::Lt },
                key: "level".to_owned(),
                within_bars: None,
            }),
        }
    }
}

impl Default for RsiExtreme {
    fn default() -> Self {
        Self::new(Self::RSI, Self::UPPER, Self::LOWER).expect("valid defaults")
    }
}

impl Engine for RsiExtreme {
    type Step = RsiExtremeStep;

    fn id(&self) -> &'static str {
        "extreme.rsi"
    }
    fn version(&self) -> u32 {
        1
    }
    fn family(&self) -> EvidenceFamily {
        EvidenceFamily::Price
    }
    fn warmup(&self) -> usize {
        self.rsi.period() + 1
    }

    fn eval(&self, bar: &Bar, _flow: Option<&FlowPoint>) -> (RsiExtremeStep, Vec<Detection>) {
        let value = self.rsi.peek1(bar.c);
        let mut zone = self.zone;
        let mut out = Vec::new();
        if let (Some(v), Some(p)) = (value, self.prev) {
            let exited = match zone {
                Zone::Over => v < self.upper,
                Zone::Under => v > self.lower,
                Zone::None => false,
            };
            if exited {
                out.push(self.detection(zone, Edge::Exit, v, p));
                zone = Zone::None;
            }
            if zone == Zone::None {
                if p <= self.upper && v > self.upper {
                    zone = Zone::Over;
                } else if p >= self.lower && v < self.lower {
                    zone = Zone::Under;
                }
                if zone != Zone::None {
                    out.push(self.detection(zone, Edge::Enter, v, p));
                }
            }
        }
        (RsiExtremeStep { value, zone }, out)
    }

    fn commit(&mut self, bar: &Bar, step: RsiExtremeStep) {
        self.rsi.commit1(bar.c);
        self.prev = step.value;
        self.zone = step.zone;
        self.n += 1;
    }
}
