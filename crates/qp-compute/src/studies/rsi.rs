//! Relative strength index (Wilder smoothing).

use std::collections::BTreeMap;

use super::pf;
use crate::{ComputeError, Core};

/// `rsi(period)`. Bar 0 has no change; the first value is at bar `period`, from the mean
/// gain and mean loss of the first `period` changes; after that Wilder smoothing
/// `(avg * (period - 1) + g) / period`. `rs = avg_gain / avg_loss`,
/// `rsi = 100 - 100 / (1 + rs)`; 100 when there is no loss, 50 when there is no move.
#[derive(Debug, Clone)]
pub struct Rsi {
    period: usize,
    pf: f64,
    pm1: f64,
    n: usize,
    prev_x: f64,
    gain_sum: f64,
    loss_sum: f64,
    avg_gain: f64,
    avg_loss: f64,
}

/// `(gain, loss)` of a change; written as two selects so it compiles without a
/// data-dependent branch (prices go up and down unpredictably).
#[inline]
fn split(change: f64) -> (f64, f64) {
    let up = change > 0.0;
    let gain = if up { change } else { 0.0 };
    let loss = if up { 0.0 } else { change.abs() };
    (gain, loss)
}

/// RSI from average gain and loss.
#[inline]
#[must_use]
pub(crate) fn rsi_from_avg(avg_gain: f64, avg_loss: f64) -> f64 {
    if avg_loss == 0.0 {
        if avg_gain == 0.0 {
            50.0
        } else {
            100.0
        }
    } else {
        let rs = avg_gain / avg_loss;
        100.0 - 100.0 / (1.0 + rs)
    }
}

impl Rsi {
    /// # Errors
    /// `period == 0`.
    pub fn new(period: usize) -> Result<Self, ComputeError> {
        if period == 0 {
            return Err(ComputeError::new("rsi: period must be >= 1"));
        }
        let p = pf(period);
        Ok(Self {
            period,
            pf: p,
            pm1: p - 1.0,
            n: 0,
            prev_x: 0.0,
            gain_sum: 0.0,
            loss_sum: 0.0,
            avg_gain: 0.0,
            avg_loss: 0.0,
        })
    }

    /// Period.
    #[must_use]
    pub fn period(&self) -> usize {
        self.period
    }

    /// Value of a bar with input `x` on top of the committed state.
    #[inline]
    #[must_use]
    pub fn peek1(&self, x: f64) -> Option<f64> {
        let i = self.n;
        if i < self.period {
            return None;
        }
        let (g, l) = split(x - self.prev_x);
        let (ag, al) = if i == self.period {
            ((self.gain_sum + g) / self.pf, (self.loss_sum + l) / self.pf)
        } else {
            (
                (self.avg_gain * self.pm1 + g) / self.pf,
                (self.avg_loss * self.pm1 + l) / self.pf,
            )
        };
        Some(rsi_from_avg(ag, al))
    }

    /// Commits a bar with input `x`.
    #[inline]
    pub fn commit1(&mut self, x: f64) {
        let i = self.n;
        if i > 0 {
            let (g, l) = split(x - self.prev_x);
            if i < self.period {
                self.gain_sum += g;
                self.loss_sum += l;
            } else if i == self.period {
                self.avg_gain = (self.gain_sum + g) / self.pf;
                self.avg_loss = (self.loss_sum + l) / self.pf;
            } else {
                self.avg_gain = (self.avg_gain * self.pm1 + g) / self.pf;
                self.avg_loss = (self.avg_loss * self.pm1 + l) / self.pf;
            }
        }
        self.prev_x = x;
        self.n = i + 1;
    }
}

impl Core for Rsi {
    const NAME: &'static str = "rsi";
    const OUTPUTS: &'static [&'static str] = &["rsi"];
    const N_IN: usize = 1;

    fn params(&self) -> BTreeMap<String, f64> {
        BTreeMap::from([("period".to_owned(), self.pf)])
    }
    fn warmup(&self) -> Vec<usize> {
        vec![self.period]
    }
    fn peek(&self, x: &[f64], out: &mut [Option<f64>]) {
        out[0] = self.peek1(x[0]);
    }
    fn commit(&mut self, x: &[f64]) {
        self.commit1(x[0]);
    }
    fn reset(&mut self) {
        self.n = 0;
        self.prev_x = 0.0;
        self.gain_sum = 0.0;
        self.loss_sum = 0.0;
        self.avg_gain = 0.0;
        self.avg_loss = 0.0;
    }
}
