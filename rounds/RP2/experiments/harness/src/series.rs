//! Bars of one binding plus the reference RSI column, kept up to date incrementally
//! with `qp_compute`'s streaming study (`push` for a new bar, `update_last` for the
//! live bar; anything else rebuilds from bar 0 — the computation always starts at the
//! first bar held, the anchor).
//!
//! Used by the shared [`crate::engine::Engine`] (approaches) and by the oracle (truth
//! and the replica's visible data). Tested against `qp_compute::run` in batch.

use qp_compute::{Field, Rsi, Streaming, Study};
use qp_schema::Bar;

/// Bars + RSI column (+ optional load studies).
pub struct SeriesState {
    bars: Vec<Bar>,
    rsi: Vec<Option<f64>>,
    study: Streaming<Rsi>,
    extra: Vec<Box<dyn Study>>,
    work: u64,
}

impl std::fmt::Debug for SeriesState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SeriesState")
            .field("bars", &self.bars.len())
            .field("extra", &self.extra.len())
            .field("work", &self.work)
            .finish_non_exhaustive()
    }
}

fn new_rsi(period: u32) -> Streaming<Rsi> {
    Streaming::new(
        Rsi::new(period as usize).expect("valid RSI period"),
        Field::Close,
    )
}

impl SeriesState {
    /// Empty, with RSI(`period`) and `extra` load studies (SMA of growing periods).
    #[must_use]
    pub fn new(period: u32, extra: u32) -> Self {
        let extra = (0..extra)
            .map(|k| {
                let mut p = std::collections::BTreeMap::new();
                p.insert("period".to_owned(), f64::from(5 + k % 50));
                qp_compute::create("sma", &p, Field::Close).expect("sma")
            })
            .collect();
        Self {
            bars: Vec::new(),
            rsi: Vec::new(),
            study: new_rsi(period),
            extra,
            work: 0,
        }
    }

    /// Bars held.
    #[must_use]
    pub fn bars(&self) -> &[Bar] {
        &self.bars
    }

    /// RSI column (one entry per bar; warmup `None`).
    #[must_use]
    pub fn rsi(&self) -> &[Option<f64>] {
        &self.rsi
    }

    /// Deterministic work counter: one unit per study step per bar.
    #[must_use]
    pub fn work(&self) -> u64 {
        self.work
    }

    /// Index of the bar with time `t`.
    #[must_use]
    pub fn index_of(&self, t: i64) -> Option<usize> {
        self.bars.binary_search_by_key(&t, |b| b.t).ok()
    }

    fn step(&mut self, bar: &Bar, revise: bool) -> Option<f64> {
        let x = [bar.c];
        let mut o = [None];
        if revise {
            self.study.update_last(&x, &mut o);
        } else {
            self.study.push(&x, &mut o);
        }
        let mut eo = [None; 1];
        for s in &mut self.extra {
            if revise {
                s.update_last(&x, &mut eo);
            } else {
                s.push(&x, &mut eo);
            }
        }
        self.work += 1 + self.extra.len() as u64;
        o[0]
    }

    fn rebuild(&mut self) {
        self.study.reset();
        for s in &mut self.extra {
            s.reset();
        }
        self.rsi.clear();
        let bars = std::mem::take(&mut self.bars);
        for b in &bars {
            let v = self.step(b, false);
            self.rsi.push(v);
        }
        self.bars = bars;
    }

    /// Replaces all bars. Returns the earliest bar time whose data or RSI may have
    /// changed (`i64::MIN`: everything).
    pub fn set_bars(&mut self, bars: Vec<Bar>) -> i64 {
        self.bars = bars;
        self.rebuild();
        i64::MIN
    }

    /// Puts `bar` at its time: appends after the last bar, revises the last bar,
    /// replaces an earlier bar or inserts a missing one. Returns the earliest bar time
    /// whose RSI may have changed.
    pub fn upsert(&mut self, bar: Bar) -> i64 {
        let t = bar.t;
        match self.bars.last().map(|b| b.t) {
            None => {
                let v = self.step(&bar, false);
                self.bars.push(bar);
                self.rsi.push(v);
            }
            Some(last) if t > last => {
                let v = self.step(&bar, false);
                self.bars.push(bar);
                self.rsi.push(v);
            }
            Some(last) if t == last => {
                let v = self.step(&bar, true);
                let n = self.bars.len();
                self.bars[n - 1] = bar;
                self.rsi[n - 1] = v;
            }
            Some(_) => {
                match self.bars.binary_search_by_key(&t, |b| b.t) {
                    Ok(i) => self.bars[i] = bar,
                    Err(i) => self.bars.insert(i, bar),
                }
                self.rebuild();
            }
        }
        t
    }

    /// Makes this state hold exactly `bars` (diffing against what it holds). Returns the
    /// earliest bar time whose data, RSI or closedness changed (when bars are appended,
    /// the previous last bar), or `None` if nothing changed.
    pub fn sync_from(&mut self, bars: &[Bar]) -> Option<i64> {
        let n = self.bars.len();
        let m = bars.len();
        let common = n.min(m);
        let first_diff = (0..common).find(|&i| self.bars[i] != bars[i]);
        match first_diff {
            None if n == m => None,
            None if m > n => {
                // The previous last bar closes: its claims change maturity.
                let from = if n > 0 { self.bars[n - 1].t } else { bars[n].t };
                for b in &bars[n..] {
                    self.upsert(b.clone());
                }
                Some(from)
            }
            Some(i) if i + 1 == n && m >= n && n > 0 && bars[i].t == self.bars[i].t => {
                // Only the last bar changed (live update), maybe plus new bars.
                let from = bars[i].t;
                self.upsert(bars[i].clone());
                for b in &bars[n..] {
                    self.upsert(b.clone());
                }
                Some(from)
            }
            Some(i) => {
                let from = self.bars[i].t.min(bars[i].t);
                self.bars = bars.to_vec();
                self.rebuild();
                Some(from)
            }
            None => {
                // Shorter now.
                let from = self.bars[m].t;
                self.bars.truncate(m);
                self.rebuild();
                Some(from)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::Rng;

    fn batch(bars: &[Bar]) -> Vec<Option<f64>> {
        let mut s = new_rsi(14);
        let col: Vec<f64> = bars.iter().map(|b| b.c).collect();
        qp_compute::run(&mut s, &[&col]).unwrap().remove(0)
    }

    fn bar(t: i64, c: f64) -> Bar {
        Bar {
            t,
            o: c,
            h: c,
            l: c,
            c,
            v: 1.0,
        }
    }

    #[test]
    fn incremental_equals_batch() {
        let mut rng = Rng::new(3);
        let mut s = SeriesState::new(14, 2);
        let mut truth: Vec<Bar> = Vec::new();
        let mut p = 100.0;
        for step in 0..3000 {
            let r = rng.f64();
            if truth.is_empty() || r < 0.1 {
                let t = truth.last().map_or(0, |b| b.t + 60);
                p += rng.normal();
                truth.push(bar(t, p));
                s.upsert(truth.last().unwrap().clone());
            } else if r < 0.9 {
                p += rng.normal() * 0.1;
                let n = truth.len();
                truth[n - 1].c = p;
                s.upsert(truth[n - 1].clone());
            } else if r < 0.95 {
                let i = rng.below(truth.len() as u64) as usize;
                truth[i].c += 0.5;
                s.upsert(truth[i].clone());
            } else {
                let mut other = truth.clone();
                if !other.is_empty() && step % 2 == 0 {
                    let i = rng.below(other.len() as u64) as usize;
                    other[i].c -= 0.25;
                }
                s.sync_from(&other);
                truth = other;
            }
            let b = batch(&truth);
            assert_eq!(s.rsi().len(), b.len());
            for (x, y) in s.rsi().iter().zip(&b) {
                assert_eq!(x.map(f64::to_bits), y.map(f64::to_bits));
            }
        }
    }
}
