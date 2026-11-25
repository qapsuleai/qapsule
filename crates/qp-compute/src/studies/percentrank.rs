//! Percent rank of the current input among the previous ones (RL1, `docs/PROGRAMS.md` §4.2).

use std::collections::BTreeMap;

use super::pf;
use crate::core::Ring;
use crate::{ComputeError, Core};

/// `percentrank(period)`: `100 * #{k in 1..=p : x[j-k] <= x[j]} / p`, the share of the
/// previous `period` inputs that are at most the bar's own input.
///
/// Definition (`docs/PROGRAMS.md` §4.2, RX4 §4.2): the count is exact; the value is
/// `(100 * count) / p`. The bar's own input is not counted, so the first value needs
/// `period` previous inputs: warmup `period` (first value at bar `period`). A bar costs
/// O(`period`) (its program cost weight is `period`).
///
/// Committed state: the last `period` inputs (ring).
#[derive(Debug, Clone)]
pub struct PercentRank {
    period: usize,
    pf: f64,
    ring: Ring,
    n: usize,
}

impl PercentRank {
    /// # Errors
    /// `period` of 0 or above [`crate::MAX_PERIOD`].
    pub fn new(period: usize) -> Result<Self, ComputeError> {
        crate::check_period("percentrank", "period", period)?;
        Ok(Self {
            period,
            pf: pf(period),
            ring: Ring::new(period),
            n: 0,
        })
    }

    /// Period.
    #[must_use]
    pub fn period(&self) -> usize {
        self.period
    }

    /// Value of a bar with input `x` on top of the committed state.
    #[must_use]
    pub fn peek1(&self, x: f64) -> Option<f64> {
        if self.n < self.period {
            return None;
        }
        let count = self.ring.iter().filter(|&v| v <= x).count();
        Some(100.0 * pf(count) / self.pf)
    }

    /// Commits a bar with input `x`.
    #[inline]
    pub fn commit1(&mut self, x: f64) {
        self.ring.push(x);
        self.n += 1;
    }
}

impl Core for PercentRank {
    const NAME: &'static str = "percentrank";
    const OUTPUTS: &'static [&'static str] = &["percentrank"];
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
        self.ring.clear();
        self.n = 0;
    }
}
