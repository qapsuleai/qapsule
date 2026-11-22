//! Live bars against REST bars: [`compare_bars`], [`BarDiff`] (split out of
//! `livebar.rs` for size).

use super::*;

/// Tolerances for [`compare_bars_with`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DiffTolerance {
    /// Absolute price tolerance (e.g. half the price step).
    pub price_abs: f64,
    /// Relative volume tolerance.
    pub volume_rel: f64,
}

impl Default for DiffTolerance {
    fn default() -> Self {
        Self {
            price_abs: 1e-6,
            volume_rel: 1e-6,
        }
    }
}

/// Agreement of one bar field over the matched bars.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FieldDiff {
    /// Bit-equal values.
    pub exact: u64,
    /// Values within tolerance (includes `exact`).
    pub within_tol: u64,
    /// Largest absolute difference.
    pub max_abs: f64,
    /// Largest relative difference (`|live − rest| / |rest|`; infinite if `rest` is 0
    /// and `live` is not).
    pub max_rel: f64,
}

impl FieldDiff {
    fn add(&mut self, live: f64, rest: f64, tol_ok: bool) {
        if live.to_bits() == rest.to_bits() {
            self.exact += 1;
        }
        if tol_ok {
            self.within_tol += 1;
        }
        let abs = (live - rest).abs();
        let rel = if rest == 0.0 {
            if abs == 0.0 {
                0.0
            } else {
                f64::INFINITY
            }
        } else {
            abs / rest.abs()
        };
        self.max_abs = self.max_abs.max(abs);
        self.max_rel = self.max_rel.max(rel);
    }
}

/// Live versus REST bars (M22 table): counts and extreme differences only.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BarDiff {
    /// Bars with the same `t` on both sides.
    pub matched: u64,
    /// Bars only in the live series.
    pub live_only: u64,
    /// Bars only in the REST series.
    pub rest_only: u64,
    /// Matched bars equal in all five fields (bit-equal).
    pub all_exact: u64,
    /// Matched bars equal in all five fields within tolerance.
    pub all_within_tol: u64,
    /// Open.
    pub o: FieldDiff,
    /// High.
    pub h: FieldDiff,
    /// Low.
    pub l: FieldDiff,
    /// Close.
    pub c: FieldDiff,
    /// Volume.
    pub v: FieldDiff,
}

impl BarDiff {
    /// `n / matched` (0 when nothing matched).
    #[must_use]
    pub fn ratio(&self, n: u64) -> f64 {
        if self.matched == 0 {
            0.0
        } else {
            n as f64 / self.matched as f64
        }
    }
}

/// [`compare_bars_with`] at the default tolerance.
#[must_use]
pub fn compare_bars(live: &[Bar], rest: &[Bar]) -> BarDiff {
    compare_bars_with(live, rest, &DiffTolerance::default())
}

/// Compares live bars with REST bars by `t` (any order; a repeated `t` keeps the last).
#[must_use]
pub fn compare_bars_with(live: &[Bar], rest: &[Bar], tol: &DiffTolerance) -> BarDiff {
    let live: BTreeMap<TimestampMs, &Bar> = live.iter().map(|b| (b.t, b)).collect();
    let rest: BTreeMap<TimestampMs, &Bar> = rest.iter().map(|b| (b.t, b)).collect();
    let mut d = BarDiff::default();
    for (t, a) in &live {
        let Some(b) = rest.get(t) else {
            d.live_only += 1;
            continue;
        };
        d.matched += 1;
        let p = |x: f64, y: f64| (x - y).abs() <= tol.price_abs;
        let ok = [
            p(a.o, b.o),
            p(a.h, b.h),
            p(a.l, b.l),
            p(a.c, b.c),
            (a.v - b.v).abs() <= tol.volume_rel * b.v.abs(),
        ];
        d.o.add(a.o, b.o, ok[0]);
        d.h.add(a.h, b.h, ok[1]);
        d.l.add(a.l, b.l, ok[2]);
        d.c.add(a.c, b.c, ok[3]);
        d.v.add(a.v, b.v, ok[4]);
        let bits = |x: f64, y: f64| x.to_bits() == y.to_bits();
        if bits(a.o, b.o) && bits(a.h, b.h) && bits(a.l, b.l) && bits(a.c, b.c) && bits(a.v, b.v) {
            d.all_exact += 1;
        }
        if ok.iter().all(|&x| x) {
            d.all_within_tol += 1;
        }
    }
    d.rest_only = rest.keys().filter(|t| !live.contains_key(t)).count() as u64;
    d
}
