//! Average true range (Wilder), R3/T3.

use std::collections::BTreeMap;

use super::pf;
use crate::{ComputeError, Core};

/// True range of a bar: `high - low` on the first bar (no previous close), then
/// `max(high - low, |high - prev_close|, |low - prev_close|)`.
#[inline]
#[must_use]
pub fn true_range(h: f64, l: f64, prev_close: Option<f64>) -> f64 {
    let hl = h - l;
    let Some(pc) = prev_close else {
        return hl;
    };
    let hc = (h - pc).abs();
    let lc = (l - pc).abs();
    let m = if hc > hl { hc } else { hl };
    if lc > m {
        lc
    } else {
        m
    }
}

/// `atr(period)`, inputs `[high, low, close]`. Bar 0's true range is `high - low`; the
/// first value is at bar `period - 1`: the mean of the first `period` true ranges
/// (summed in bar order, then `/ period`); after that Wilder smoothing
/// `(atr * (period - 1) + tr) / period`. Warmup `period - 1`.
#[derive(Debug, Clone)]
pub struct Atr {
    period: usize,
    pf: f64,
    pm1: f64,
    n: usize,
    prev_c: f64,
    sum: f64,
    atr: f64,
}

impl Atr {
    /// # Errors
    /// `period` of 0 or above [`crate::MAX_PERIOD`].
    pub fn new(period: usize) -> Result<Self, ComputeError> {
        crate::check_period("atr", "period", period)?;
        let p = pf(period);
        Ok(Self {
            period,
            pf: p,
            pm1: p - 1.0,
            n: 0,
            prev_c: 0.0,
            sum: 0.0,
            atr: 0.0,
        })
    }

    /// Period.
    #[must_use]
    pub fn period(&self) -> usize {
        self.period
    }

    #[inline]
    fn tr(&self, h: f64, l: f64) -> f64 {
        true_range(h, l, (self.n > 0).then_some(self.prev_c))
    }

    /// Value of a bar `(h, l)` on top of the committed state (the bar's close only
    /// matters to the next bar, see [`Atr::commit_hlc`]).
    #[inline]
    #[must_use]
    pub fn peek_hl(&self, h: f64, l: f64) -> Option<f64> {
        let tr = self.tr(h, l);
        let i = self.n;
        if i + 1 < self.period {
            None
        } else if i + 1 == self.period {
            Some((self.sum + tr) / self.pf)
        } else {
            Some((self.atr * self.pm1 + tr) / self.pf)
        }
    }

    /// Commits a bar `(h, l, c)`.
    #[inline]
    pub fn commit_hlc(&mut self, h: f64, l: f64, c: f64) {
        let tr = self.tr(h, l);
        let i = self.n;
        if i + 1 < self.period {
            self.sum += tr;
        } else if i + 1 == self.period {
            self.atr = (self.sum + tr) / self.pf;
        } else {
            self.atr = (self.atr * self.pm1 + tr) / self.pf;
        }
        self.prev_c = c;
        self.n = i + 1;
    }
}

impl Core for Atr {
    const NAME: &'static str = "atr";
    const OUTPUTS: &'static [&'static str] = &["atr"];
    const N_IN: usize = 3;

    fn params(&self) -> BTreeMap<String, f64> {
        BTreeMap::from([("period".to_owned(), self.pf)])
    }
    fn warmup(&self) -> Vec<usize> {
        vec![self.period - 1]
    }
    fn peek(&self, x: &[f64], out: &mut [Option<f64>]) {
        out[0] = self.peek_hl(x[0], x[1]);
    }
    fn commit(&mut self, x: &[f64]) {
        self.commit_hlc(x[0], x[1], x[2]);
    }
    fn reset(&mut self) {
        self.n = 0;
        self.prev_c = 0.0;
        self.sum = 0.0;
        self.atr = 0.0;
    }
}
