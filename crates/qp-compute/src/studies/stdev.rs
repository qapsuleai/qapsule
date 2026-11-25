//! Rolling population standard deviation, two passes (RL1, `docs/PROGRAMS.md` §4.2).

use std::collections::BTreeMap;

use super::pf;
use crate::core::Ring;
use crate::{ComputeError, Core};

/// `stdev(period)`: population standard deviation of the last `period` inputs.
///
/// Definition (`docs/PROGRAMS.md` §4.2, RX4 §4.2): over the window `w` (oldest first, the
/// bar's own input last) `m = Σw / p`, then `sqrt(Σ(w - m)² / p)`, both sums in window
/// order starting from `0.0`, each square as `d * d`. Two passes over the window on every
/// bar, so a bar costs O(`period`) (its program cost weight is `period`), not O(1); in
/// exchange there is no rolled state and no drift. Not the same kernel as
/// [`crate::Bollinger`]'s rolled deviation; the bits differ.
///
/// Committed state: the last `period - 1` inputs (ring). Warmup `period - 1`.
#[derive(Debug, Clone)]
pub struct Stdev {
    period: usize,
    pf: f64,
    ring: Ring,
    n: usize,
}

impl Stdev {
    /// # Errors
    /// `period` of 0 or above [`crate::MAX_PERIOD`].
    pub fn new(period: usize) -> Result<Self, ComputeError> {
        crate::check_period("stdev", "period", period)?;
        Ok(Self {
            period,
            pf: pf(period),
            ring: Ring::new(period - 1),
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
        if self.n + 1 < self.period {
            return None;
        }
        let mut s = 0.0;
        for v in self.ring.iter() {
            s += v;
        }
        s += x;
        let m = s / self.pf;
        let mut q = 0.0;
        for v in self.ring.iter() {
            let d = v - m;
            q += d * d;
        }
        let d = x - m;
        q += d * d;
        Some((q / self.pf).sqrt())
    }

    /// Commits a bar with input `x`.
    #[inline]
    pub fn commit1(&mut self, x: f64) {
        self.ring.push(x);
        self.n += 1;
    }
}

impl Core for Stdev {
    const NAME: &'static str = "stdev";
    const OUTPUTS: &'static [&'static str] = &["stdev"];
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
        self.n = 0;
    }
}
