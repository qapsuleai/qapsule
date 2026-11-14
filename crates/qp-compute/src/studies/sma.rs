//! Simple moving average.

use std::collections::BTreeMap;

use super::pf;
use crate::core::Ring;
use crate::{ComputeError, Core};

/// `sma(period)`: mean of the last `period` inputs.
///
/// Committed state: the last `period - 1` inputs (ring) and their rolling sum, updated as
/// `(sum + x) - evicted`. A bar's value is `(sum + x) / period`.
#[derive(Debug, Clone)]
pub struct Sma {
    period: usize,
    pf: f64,
    ring: Ring,
    sum: f64,
    n: usize,
}

impl Sma {
    /// # Errors
    /// `period == 0`.
    pub fn new(period: usize) -> Result<Self, ComputeError> {
        if period == 0 {
            return Err(ComputeError::new("sma: period must be >= 1"));
        }
        Ok(Self {
            period,
            pf: pf(period),
            ring: Ring::new(period - 1),
            sum: 0.0,
            n: 0,
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
        if self.n + 1 < self.period {
            None
        } else {
            Some((self.sum + x) / self.pf)
        }
    }

    /// Commits a bar with input `x`.
    #[inline]
    pub fn commit1(&mut self, x: f64) {
        self.sum += x;
        if let Some(old) = self.ring.push(x) {
            self.sum -= old;
        }
        self.n += 1;
    }
}

impl Core for Sma {
    const NAME: &'static str = "sma";
    const OUTPUTS: &'static [&'static str] = &["sma"];
    const N_IN: usize = 1;

    fn params(&self) -> BTreeMap<String, f64> {
        BTreeMap::from([("period".to_owned(), self.pf)])
    }
    fn warmup(&self) -> Vec<usize> {
        vec![self.period - 1]
    }
    fn peek(&self, x: &[f64], out: &mut [Option<f64>]) {
        out[0] = self.peek1(x[0]);
    }
    fn commit(&mut self, x: &[f64]) {
        self.commit1(x[0]);
    }
    fn reset(&mut self) {
        self.ring.clear();
        self.sum = 0.0;
        self.n = 0;
    }
}
