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
    pub(crate) pf: f64,
    ring: Ring,
    sum: f64,
    n: usize,
}

impl Sma {
    /// # Errors
    /// `period` of 0 or above [`crate::MAX_PERIOD`].
    pub fn new(period: usize) -> Result<Self, ComputeError> {
        crate::check_period("sma", "period", period)?;
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
        self.commit_evict(x);
    }

    /// Commits `x` and returns the value that left the `period - 1` window, if any.
    #[inline]
    pub(crate) fn commit_evict(&mut self, x: f64) -> Option<f64> {
        self.sum += x;
        let old = self.ring.push(x);
        if let Some(old) = old {
            self.sum -= old;
        }
        self.n += 1;
        old
    }

    /// Whether the committed window is full and non-empty (`period >= 2`, at least
    /// `period - 1` bars committed).
    #[inline]
    pub(crate) fn is_steady(&self) -> bool {
        self.ring.is_full_nonempty()
    }

    /// Steady-state peek + commit in one step (requires [`Sma::is_steady`]): returns the
    /// bar's value and the evicted input. Same operations, same order as
    /// `peek1(x)` followed by `commit1(x)`.
    #[inline]
    pub(crate) fn step_steady(&mut self, x: f64) -> (f64, f64) {
        let s = self.sum + x;
        let value = s / self.pf;
        let old = self.ring.push_full(x);
        self.sum = s - old;
        self.n += 1;
        (value, old)
    }

    /// Sum of the committed window (the last `window_len()` inputs).
    #[inline]
    pub(crate) fn window_sum(&self) -> f64 {
        self.sum
    }

    /// Number of inputs in the committed window (`<= period - 1`).
    #[inline]
    pub(crate) fn window_len(&self) -> usize {
        self.ring.len()
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
