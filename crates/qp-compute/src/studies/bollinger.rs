//! Bollinger bands (four outputs).

use std::collections::BTreeMap;

use super::{pf, Sma};
use crate::{ComputeError, Core};

/// `bollinger(period, k)`. `middle = sma(period)`; `std` is the population standard
/// deviation of the last `period` inputs; `upper/lower = middle ± k * std`;
/// `bandwidth = (2 * k * std) / middle` (0 when `middle` is 0).
///
/// Variance: a windowed Welford update of `M2` (sum of squared deviations), add-then-
/// remove, which stays accurate far from zero. The window means it needs come from the
/// SMA's rolling sum (`sum / count`), so the only per-bar dependency chain is the two
/// additions on `M2`; a new bar is O(1) with no O(period) recomputation. With a full
/// window, divisions by the window size are multiplications by precomputed reciprocals.
///
/// Flat windows are exact: a counter of trailing equal inputs (O(1)) detects a committed
/// window whose values are all equal; its M2 is then exactly 0 (a rolling M2 would
/// otherwise keep a ~1e-15 residue that `sqrt` turns into a visible ~1e-7 band width).
/// This also clears any rounding residue accumulated before the flat stretch.
#[derive(Debug, Clone)]
pub struct Bollinger {
    /// Holds the committed window (last `period - 1` inputs) and its sum.
    pub(crate) sma: Sma,
    pub(crate) k: f64,
    pub(crate) period: usize,
    pub(crate) pf: f64,
    pub(crate) inv_p: f64,
    pub(crate) inv_pm1: f64,
    /// M2 of the committed window.
    pub(crate) m2: f64,
    /// Last committed input and how many trailing committed inputs equal it.
    pub(crate) last: f64,
    pub(crate) run: usize,
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
            inv_p: 1.0 / pf(period),
            inv_pm1: if period > 1 {
                1.0 / pf(period - 1)
            } else {
                0.0
            },
            m2: 0.0,
            last: 0.0,
            run: 0,
        })
    }

    /// Mean of the committed window (`w` values summing to `s`).
    #[inline]
    fn mean_of(&self, s: f64, w: usize) -> f64 {
        if w == 0 {
            0.0
        } else if w + 1 == self.period {
            s * self.inv_pm1
        } else {
            s / pf(w)
        }
    }

    /// `(mean, M2)` of the committed window plus `x`.
    #[inline]
    fn add(&self, x: f64) -> (f64, f64) {
        let w = self.sma.window_len();
        let s = self.sma.window_sum();
        let mean = if w + 1 == self.period {
            (s + x) * self.inv_p
        } else {
            (s + x) / pf(w + 1)
        };
        if self.run >= w {
            // Committed window is flat (all equal to `last`): exact M2 = 0.
            let delta = x - self.last;
            return (mean, delta * (x - mean));
        }
        let delta = x - self.mean_of(s, w);
        (mean, self.m2 + delta * (x - mean))
    }

    /// Updates the trailing-equal counter with a committed input.
    #[inline]
    fn track_run(&mut self, x: f64) {
        if self.run > 0 && x == self.last {
            self.run += 1;
        } else {
            self.run = 1;
            self.last = x;
        }
    }

    #[inline]
    fn value(&self, middle: f64, m2: f64) -> BollingerValue {
        let var = m2 * self.inv_p;
        let std = if var > 0.0 { var.sqrt() } else { 0.0 };
        let band = self.k * std;
        BollingerValue {
            middle,
            upper: middle + band,
            lower: middle - band,
            bandwidth: if middle == 0.0 {
                0.0
            } else {
                (2.0 * band) / middle
            },
        }
    }

    /// Value of a bar with input `x` on top of the committed state.
    #[inline]
    #[must_use]
    pub fn peek1(&self, x: f64) -> Option<BollingerValue> {
        let middle = self.sma.peek1(x)?;
        let (_, m2) = self.add(x);
        Some(self.value(middle, m2))
    }

    /// Commits a bar with input `x`.
    #[inline]
    pub fn commit1(&mut self, x: f64) {
        let (mean, m2) = self.add(x);
        self.track_run(x);
        self.m2 = match self.sma.commit_evict(x) {
            None => m2,
            Some(_) if self.period == 1 => 0.0,
            Some(old) => {
                // Remove `old` from the full window: M2 -= (old - mean) * (old - mean_after).
                let mean_after = self.sma.window_sum() * self.inv_pm1;
                m2 - (old - mean) * (old - mean_after)
            }
        };
    }

    /// Peek + commit in one step. With a full window this is a fused path performing
    /// exactly the operations of `peek1(x)` followed by `commit1(x)`, once each and
    /// without warmup branches; results are bit-identical (tested).
    #[inline]
    pub(crate) fn step1(&mut self, x: f64) -> Option<BollingerValue> {
        if !self.sma.is_steady() || self.run >= self.period - 1 {
            let v = self.peek1(x);
            self.commit1(x);
            return v;
        }
        let s = self.sma.window_sum();
        let mean_c = s * self.inv_pm1;
        let mean = (s + x) * self.inv_p;
        let m2 = self.m2 + (x - mean_c) * (x - mean);
        self.track_run(x);
        let (middle, old) = self.sma.step_steady(x);
        let v = self.value(middle, m2);
        let mean_after = self.sma.window_sum() * self.inv_pm1;
        self.m2 = m2 - (old - mean) * (old - mean_after);
        Some(v)
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
        self.m2 = 0.0;
        self.last = 0.0;
        self.run = 0;
    }
}
