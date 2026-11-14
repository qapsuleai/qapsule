//! Moving average convergence/divergence (three outputs).

use std::collections::BTreeMap;

use super::{pf, Ema};
use crate::{ComputeError, Core};

/// `macd(fast, slow, signal)`, `fast < slow`. Outputs `macd = ema(fast) - ema(slow)`,
/// `signal = ema(signal)` of the macd line (seeded with its simple mean) and
/// `histogram = macd - signal`.
#[derive(Debug, Clone)]
pub struct Macd {
    fast: Ema,
    slow: Ema,
    signal: Ema,
}

/// One bar of MACD output.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MacdValue {
    /// `ema(fast) - ema(slow)`.
    pub macd: Option<f64>,
    /// Signal line.
    pub signal: Option<f64>,
    /// `macd - signal`.
    pub histogram: Option<f64>,
}

impl Macd {
    /// # Errors
    /// A zero period or `fast >= slow`.
    pub fn new(fast: usize, slow: usize, signal: usize) -> Result<Self, ComputeError> {
        if fast == 0 || slow == 0 || signal == 0 {
            return Err(ComputeError::new("macd: periods must be >= 1"));
        }
        if fast >= slow {
            return Err(ComputeError::new(format!(
                "macd: fast ({fast}) must be < slow ({slow})"
            )));
        }
        Ok(Self {
            fast: Ema::new(fast)?,
            slow: Ema::new(slow)?,
            signal: Ema::new(signal)?,
        })
    }

    /// Value of a bar with input `x` on top of the committed state.
    #[inline]
    #[must_use]
    pub fn peek1(&self, x: f64) -> MacdValue {
        let macd = match (self.fast.peek1(x), self.slow.peek1(x)) {
            (Some(f), Some(s)) => Some(f - s),
            _ => None,
        };
        let signal = macd.and_then(|m| self.signal.peek1(m));
        let histogram = match (macd, signal) {
            (Some(m), Some(s)) => Some(m - s),
            _ => None,
        };
        MacdValue {
            macd,
            signal,
            histogram,
        }
    }

    /// Commits a bar with input `x`.
    #[inline]
    pub fn commit1(&mut self, x: f64) {
        let macd = match (self.fast.peek1(x), self.slow.peek1(x)) {
            (Some(f), Some(s)) => Some(f - s),
            _ => None,
        };
        self.fast.commit1(x);
        self.slow.commit1(x);
        if let Some(m) = macd {
            self.signal.commit1(m);
        }
    }
}

impl Core for Macd {
    const NAME: &'static str = "macd";
    const OUTPUTS: &'static [&'static str] = &["macd", "signal", "histogram"];
    const N_IN: usize = 1;

    fn params(&self) -> BTreeMap<String, f64> {
        BTreeMap::from([
            ("fast".to_owned(), pf(self.fast.period())),
            ("slow".to_owned(), pf(self.slow.period())),
            ("signal".to_owned(), pf(self.signal.period())),
        ])
    }
    fn warmup(&self) -> Vec<usize> {
        let m = self.slow.period() - 1;
        let s = m + self.signal.period() - 1;
        vec![m, s, s]
    }
    fn peek(&self, x: &[f64], out: &mut [Option<f64>]) {
        let v = self.peek1(x[0]);
        out[0] = v.macd;
        out[1] = v.signal;
        out[2] = v.histogram;
    }
    fn commit(&mut self, x: &[f64]) {
        self.commit1(x[0]);
    }
    fn reset(&mut self) {
        self.fast.reset();
        self.slow.reset();
        self.signal.reset();
    }
}
