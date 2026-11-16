//! `continuation.ma_stack` v1: moving-average alignment (trend) state.
//!
//! Bar `i`, `f = sma(20)` and `s = sma(50)` of close, `f5 = sma(20)` at bar `i - 5`,
//! `atr = atr(14)` (all `qp-compute` values):
//!
//! - up state: `close > f && f > s && f > f5`; down state: `close < f && f < s && f < f5`;
//!   otherwise (or not warm) no state.
//! - On a change: `Exit` of the old state (code `bull_end` / `bear_end`), then `Enter` of
//!   the new one (`bull` / `bear`); both may happen on one bar.
//! - Strength: `|f - s| / atr` clipped to `[0, 1]` (0 when `atr <= 0`).
//! - Evidence: `close`, `sma20`, `sma50`, `sma20_prev5`, `atr`. Condition: `close` `Gt`
//!   (`Lt` for down) `sma20`.

use std::collections::VecDeque;

use qp_compute::{Atr, ComputeError, Sma};
use qp_schema::capsule::{
    Direction, Edge, EvidenceFamily, MeaningCondition, MeaningKind, TimeKind,
};
use qp_schema::{Bar, CompareOp};

use crate::{clip01, evidence, Detection, Engine, FlowPoint, Staged};

/// Trend state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stack {
    None,
    Up,
    Down,
}

/// `continuation.ma_stack` engine (see the module docs).
#[derive(Debug, Clone)]
pub struct MaStack {
    fast: Sma,
    slow: Sma,
    atr: Atr,
    rise: usize,
    /// Last `rise` committed `fast` values (front = bar `i - rise`).
    hist: VecDeque<Option<f64>>,
    state: Stack,
}

/// Values of one bar.
#[derive(Debug, Clone)]
pub struct MaStackStep {
    fast: Option<f64>,
    state: Stack,
}

impl MaStack {
    /// Fast SMA period.
    pub const FAST: usize = 20;
    /// Slow SMA period.
    pub const SLOW: usize = 50;
    /// ATR period (strength normalization).
    pub const ATR: usize = 14;
    /// The fast SMA must exceed its value this many bars ago ("rising").
    pub const RISE: usize = 5;

    /// # Errors
    /// Invalid period.
    pub fn new(fast: usize, slow: usize, atr: usize, rise: usize) -> Result<Self, ComputeError> {
        Ok(Self {
            fast: Sma::new(fast)?,
            slow: Sma::new(slow)?,
            atr: Atr::new(atr)?,
            rise,
            hist: VecDeque::with_capacity(rise + 1),
            state: Stack::None,
        })
    }

    /// As a [`crate::Detector`].
    #[must_use]
    pub fn detector(self) -> Staged<Self> {
        Staged::new(self)
    }

    #[allow(clippy::too_many_arguments)]
    fn detection(
        dir: Stack,
        edge: Edge,
        c: f64,
        f: f64,
        s: f64,
        f5: f64,
        atr: Option<f64>,
    ) -> Detection {
        let (code, direction, op) = match (dir, edge) {
            (Stack::Up, Edge::Enter) => ("bull", Direction::Up, CompareOp::Gt),
            (Stack::Up, _) => ("bull_end", Direction::Up, CompareOp::Gt),
            (_, Edge::Enter) => ("bear", Direction::Down, CompareOp::Lt),
            _ => ("bear_end", Direction::Down, CompareOp::Lt),
        };
        let gap = (f - s).abs();
        let strength = match atr {
            Some(a) if a > 0.0 => clip01(gap / a),
            _ => 0.0,
        };
        let mut ev = evidence(&[
            ("close", c),
            ("sma20", f),
            ("sma50", s),
            ("sma20_prev5", f5),
        ]);
        if let Some(a) = atr {
            ev.insert("atr".to_owned(), a);
        }
        Detection {
            code,
            kind: MeaningKind::Continuation,
            direction,
            time: TimeKind::State,
            edge,
            strength,
            evidence: ev,
            geometry: Vec::new(),
            condition: Some(MeaningCondition {
                subject: "close".to_owned(),
                op,
                key: "sma20".to_owned(),
                within_bars: None,
            }),
        }
    }
}

impl Default for MaStack {
    fn default() -> Self {
        Self::new(Self::FAST, Self::SLOW, Self::ATR, Self::RISE).expect("valid defaults")
    }
}

impl Engine for MaStack {
    type Step = MaStackStep;

    fn id(&self) -> &'static str {
        "continuation.ma_stack"
    }
    fn version(&self) -> u32 {
        1
    }
    fn family(&self) -> EvidenceFamily {
        EvidenceFamily::Price
    }
    fn warmup(&self) -> usize {
        let f5 = self.fast.period() - 1 + self.rise;
        (self.slow.period() - 1).max(f5)
    }

    fn eval(&self, bar: &Bar, _flow: Option<&FlowPoint>) -> (MaStackStep, Vec<Detection>) {
        let c = bar.c;
        let fast = self.fast.peek1(c);
        let slow = self.slow.peek1(c);
        let atr = self.atr.peek_hl(bar.h, bar.l);
        let f5 = if self.hist.len() == self.rise {
            self.hist.front().copied().flatten()
        } else {
            None
        };
        let vals = match (fast, slow, f5) {
            (Some(f), Some(s), Some(f5)) => Some((f, s, f5)),
            _ => None,
        };
        let state = match vals {
            Some((f, s, f5)) if c > f && f > s && f > f5 => Stack::Up,
            Some((f, s, f5)) if c < f && f < s && f < f5 => Stack::Down,
            _ => Stack::None,
        };
        let mut out = Vec::new();
        if state != self.state {
            if let Some((f, s, f5)) = vals {
                if self.state != Stack::None {
                    out.push(Self::detection(self.state, Edge::Exit, c, f, s, f5, atr));
                }
                if state != Stack::None {
                    out.push(Self::detection(state, Edge::Enter, c, f, s, f5, atr));
                }
            }
        }
        (MaStackStep { fast, state }, out)
    }

    fn commit(&mut self, bar: &Bar, step: MaStackStep) {
        self.fast.commit1(bar.c);
        self.slow.commit1(bar.c);
        self.atr.commit_hlc(bar.h, bar.l, bar.c);
        self.hist.push_back(step.fast);
        if self.hist.len() > self.rise {
            self.hist.pop_front();
        }
        self.state = step.state;
    }
}
