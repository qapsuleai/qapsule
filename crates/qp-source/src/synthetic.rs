//! Deterministic synthetic bars (integer LCG + IEEE basic operations only, so identical
//! on every target). The generator is the one that produced `fixtures/bars/sample.json`
//! (`synthetic_series(200, 20_251_114)`), moved here from `qp-render` in R1.

use qp_schema::{Bar, Instrument, Series, SeriesData, SeriesSource as Binding};

use crate::{BarEvent, SeriesMeta, SeriesSource, SourceError};

/// Minimal 64-bit LCG (Knuth MMIX constants).
#[derive(Debug, Clone)]
pub(crate) struct Lcg(pub(crate) u64);

impl Lcg {
    pub(crate) fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        // Truncation intended: the high 32 bits.
        u32::try_from(self.0 >> 32).unwrap_or(u32::MAX)
    }

    /// Uniform in [-1, 1): built from an integer, then one exact division.
    pub(crate) fn unit(&mut self) -> f64 {
        f64::from(self.next_u32()) / 2_147_483_648.0 - 1.0
    }
}

pub(crate) fn round2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

const DAY_MS: i64 = 86_400_000;
/// 2025-01-02 00:00:00 UTC (a Thursday).
const START_MS: i64 = 1_735_776_000_000;

/// `n` deterministic daily bars (weekdays only) from `seed`.
#[must_use]
pub fn synthetic_bars(n: usize, seed: u64) -> Vec<Bar> {
    let mut rng = Lcg(seed);
    let mut bars = Vec::with_capacity(n);
    let mut close = 100.0_f64;
    let mut day = 0_i64;
    for _ in 0..n {
        // Skip weekends: day 0 is a Thursday, so (day + 3) % 7 gives Mon=0..Sun=6.
        while (day + 3).rem_euclid(7) >= 5 {
            day += 1;
        }
        let open = round2(close * (1.0 + 0.004 * rng.unit()));
        let drift = 0.0004;
        let c = round2(open * (1.0 + drift + 0.018 * rng.unit()));
        let hi = round2(open.max(c) * (1.0 + 0.008 * (rng.unit() + 1.0)));
        let lo = round2(open.min(c) * (1.0 - 0.008 * (rng.unit() + 1.0)));
        let vol = round2(1_000_000.0 * (1.5 + rng.unit()));
        bars.push(Bar {
            t: START_MS + day * DAY_MS,
            o: open,
            h: hi,
            l: lo,
            c,
            v: vol,
        });
        close = c;
        day += 1;
    }
    bars
}

/// The synthetic bars as a `Series` document (`id` `sample`, symbol `SAMPLE`, `1d`).
#[must_use]
pub fn synthetic_series(n: usize, seed: u64) -> Series {
    Series {
        id: "sample".to_owned(),
        instrument: Some(Instrument {
            symbol: "SAMPLE".to_owned(),
            exchange: None,
            calendar: None,
        }),
        interval: Some("1d".to_owned()),
        source: Binding::Inline {},
        data: Some(SeriesData::Bars {
            bars: synthetic_bars(n, seed),
        }),
    }
}

/// A synthetic source. With `ticks_per_bar = 1` it emits one `New` per bar; with `k > 1`
/// every bar arrives as a live bar: `New` with the opening state, then `k - 1`
/// `UpdateLast` revisions whose close walks from open towards the final close, the last
/// one being the final bar. Draining it always yields exactly [`synthetic_bars`].
#[derive(Debug, Clone)]
pub struct SyntheticSource {
    meta: SeriesMeta,
    bars: Vec<Bar>,
    ticks_per_bar: u32,
    bar: usize,
    tick: u32,
}

impl SyntheticSource {
    /// `n` bars from `seed`, each delivered in `ticks_per_bar` events (at least 1).
    #[must_use]
    pub fn new(n: usize, seed: u64, ticks_per_bar: u32) -> Self {
        let s = synthetic_series(n, seed);
        Self {
            meta: SeriesMeta {
                id: s.id,
                instrument: s.instrument,
                interval: s.interval,
            },
            bars: synthetic_bars(n, seed),
            ticks_per_bar: ticks_per_bar.max(1),
            bar: 0,
            tick: 0,
        }
    }

    /// Intermediate state of bar `b` after `k` of `n` ticks (`k < n`).
    fn partial(b: &Bar, k: u32, n: u32) -> Bar {
        let f = f64::from(k) / f64::from(n);
        let c = round2(b.o + (b.c - b.o) * f);
        Bar {
            t: b.t,
            o: b.o,
            h: b.o.max(c),
            l: b.o.min(c),
            c,
            v: round2(b.v * f),
        }
    }
}

impl SeriesSource for SyntheticSource {
    fn meta(&self) -> &SeriesMeta {
        &self.meta
    }

    fn next_event(&mut self) -> Result<Option<BarEvent>, SourceError> {
        let Some(b) = self.bars.get(self.bar) else {
            return Ok(None);
        };
        let n = self.ticks_per_bar;
        let k = self.tick;
        let state = if k + 1 == n {
            b.clone()
        } else {
            Self::partial(b, k, n)
        };
        let ev = if k == 0 {
            BarEvent::New(state)
        } else {
            BarEvent::UpdateLast(state)
        };
        if k + 1 == n {
            self.bar += 1;
            self.tick = 0;
        } else {
            self.tick += 1;
        }
        Ok(Some(ev))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_is_valid_and_stable() {
        let a = synthetic_series(500, 7);
        let b = synthetic_series(500, 7);
        assert_eq!(a, b);
        qp_schema::validate_bars(a.bars().unwrap()).unwrap();
    }
}
