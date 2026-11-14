//! Bollinger bands (four outputs).

use std::collections::BTreeMap;

use super::{pf, Sma};
use crate::core::Ring;
use crate::{ComputeError, Core};

/// `bollinger(period, k)`. `middle = sma(period)`; `std` is the population standard
/// deviation of the last `period` inputs; `upper/lower = middle ± k * std`;
/// `bandwidth = (2 * k * std) / middle` (0 when `middle` is 0).
///
/// The variance uses a windowed Welford update (mean and sum of squared deviations,
/// add-then-remove), which stays accurate when prices are far from zero; no O(period)
/// recomputation, so a new bar is O(1).
#[derive(Debug, Clone)]
pub struct Bollinger {
    sma: Sma,
    k: f64,
    period: usize,
    pf: f64,
    /// Last `period - 1` inputs.
    ring: Ring,
    /// Mean and M2 of the values in `ring`.
    mean: f64,
    m2: f64,
}

/// One bar of Bollinger output.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BollingerValue {
    /// Middle band (SMA).
    pub middle: f64,
    /// `middle + k * std`.
    pub upper: f64,
    /// `middle - k * std`.
    pub lower: f64,
    /// `(upper - lower) / middle`.
    pub bandwidth: f64,
}

impl Bollinger {
    /// # Errors
    /// `period == 0`, or `k` not finite and `> 0`.
    pub fn new(period: usize, k: f64) -> Result<Self, ComputeError> {
        if period == 0 {
            return Err(ComputeError::new("bollinger: period must be >= 1"));
        }
        if !(k.is_finite() && k > 0.0) {
            return Err(ComputeError::new(format!(
                "bollinger: k must be finite and > 0, got {k}"
            )));
        }
        Ok(Self {
            sma: Sma::new(period)?,
            k,
            period,
            pf: pf(period),
            ring: Ring::new(period - 1),
            mean: 0.0,
            m2: 0.0,
        })
    }

    /// Mean and M2 of the window `ring + [x]` (`n = ring.len() + 1` values).
    #[inline]
    fn add(&self, x: f64) -> (f64, f64) {
        let n = pf(self.ring.len() + 1);
        let delta = x - self.mean;
        let mean = self.mean + delta / n;
        (mean, self.m2 + delta * (x - mean))
    }

    /// Value of a bar with input `x` on top of the committed state.
    #[inline]
    #[must_use]
    pub fn peek1(&self, x: f64) -> Option<BollingerValue> {
        let middle = self.sma.peek1(x)?;
        let (_, m2) = self.add(x);
        let var = m2 / self.pf;
        let std = if var > 0.0 { var.sqrt() } else { 0.0 };
        let band = self.k * std;
        Some(BollingerValue {
            middle,
            upper: middle + band,
            lower: middle - band,
            bandwidth: if middle == 0.0 {
                0.0
            } else {
                (2.0 * band) / middle
            },
        })
    }

    /// Commits a bar with input `x`.
    #[inline]
    pub fn commit1(&mut self, x: f64) {
        self.sma.commit1(x);
        let (mean, m2) = self.add(x);
        match self.ring.push(x) {
            None => {
                self.mean = mean;
                self.m2 = m2;
            }
            Some(old) => {
                // Remove `old` from a window of `ring.len() + 1` values.
                let n_after = self.ring.len();
                if n_after == 0 {
                    self.mean = 0.0;
                    self.m2 = 0.0;
                } else {
                    let delta = old - mean;
                    let mean_after = mean - delta / pf(n_after);
                    self.m2 = m2 - delta * (old - mean_after);
                    self.mean = mean_after;
                }
            }
        }
    }
}

impl Core for Bollinger {
    const NAME: &'static str = "bollinger";
    const OUTPUTS: &'static [&'static str] = &["middle", "upper", "lower", "bandwidth"];
    const N_IN: usize = 1;

    fn params(&self) -> BTreeMap<String, f64> {
        BTreeMap::from([("period".to_owned(), self.pf), ("k".to_owned(), self.k)])
    }
    fn warmup(&self) -> Vec<usize> {
        vec![self.period - 1; 4]
    }
    fn peek(&self, x: &[f64], out: &mut [Option<f64>]) {
        let v = self.peek1(x[0]);
        out[0] = v.map(|v| v.middle);
        out[1] = v.map(|v| v.upper);
        out[2] = v.map(|v| v.lower);
        out[3] = v.map(|v| v.bandwidth);
    }
    fn commit(&mut self, x: &[f64]) {
        self.commit1(x[0]);
    }
    fn reset(&mut self) {
        self.sma.reset();
        self.ring.clear();
        self.mean = 0.0;
        self.m2 = 0.0;
    }
}
