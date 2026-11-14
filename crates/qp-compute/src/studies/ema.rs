//! Exponential moving average, seeded with the simple average.

use std::collections::BTreeMap;

use super::pf;
use crate::{ComputeError, Core};

/// `ema(period)`: `a = 2 / (period + 1)`. The first value (bar `period - 1`) is the mean
/// of the first `period` inputs; after that `x * a + prev * (1 - a)`.
#[derive(Debug, Clone)]
pub struct Ema {
    period: usize,
    pf: f64,
    a: f64,
    n: usize,
    seed_sum: f64,
    prev: f64,
}

impl Ema {
    /// # Errors
    /// `period == 0`.
    pub fn new(period: usize) -> Result<Self, ComputeError> {
        if period == 0 {
            return Err(ComputeError::new("ema: period must be >= 1"));
        }
        let p = pf(period);
        Ok(Self {
            period,
            pf: p,
            a: 2.0 / (p + 1.0),
            n: 0,
            seed_sum: 0.0,
            prev: 0.0,
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
        let k = self.n + 1;
        if k < self.period {
            None
        } else if k == self.period {
            Some((self.seed_sum + x) / self.pf)
        } else {
            Some(x * self.a + self.prev * (1.0 - self.a))
        }
    }

    /// `(a, last value)` once the seed bar is committed (every later bar is
    /// [`Ema::steady_next`]); `None` before.
    #[inline]
    pub(crate) fn steady(&self) -> Option<(f64, f64)> {
        (self.n >= self.period).then_some((self.a, self.prev))
    }

    /// A bar after the seed: the expression of `peek1`/`commit1`, with the state
    /// `(a, prev)` held by the caller (e.g. in registers across several lanes).
    #[inline]
    #[must_use]
    pub(crate) fn steady_next(a: f64, prev: f64, x: f64) -> f64 {
        x * a + prev * (1.0 - a)
    }

    /// Commits a bar with input `x`.
    #[inline]
    pub fn commit1(&mut self, x: f64) {
        let k = self.n + 1;
        if k < self.period {
            self.seed_sum += x;
        } else if k == self.period {
            self.prev = (self.seed_sum + x) / self.pf;
        } else {
            self.prev = x * self.a + self.prev * (1.0 - self.a);
        }
        self.n = k;
    }
}

impl Core for Ema {
    const NAME: &'static str = "ema";
    const OUTPUTS: &'static [&'static str] = &["ema"];
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
        self.n = 0;
        self.seed_sum = 0.0;
        self.prev = 0.0;
    }
}
