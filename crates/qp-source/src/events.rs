//! Field-level data events (`DataEvent`, K23): a deterministic synthetic stream and a
//! converter from the whole-bar [`BarEvent`] stream.

use std::collections::VecDeque;

use qp_schema::data::{BarPatch, DataChange, DataEvent, FeedKey};
use qp_schema::Bar;

use crate::synthetic::{round2, Lcg};
use crate::{synthetic_bars, BarEvent, SourceError};

/// The fields of `new` that differ from `old` (bitwise), as a patch. Empty if equal.
#[must_use]
pub fn bar_patch_diff(old: &Bar, new: &Bar) -> BarPatch {
    let d = |a: f64, b: f64| (a.to_bits() != b.to_bits()).then_some(b);
    BarPatch {
        o: d(old.o, new.o),
        h: d(old.h, new.h),
        l: d(old.l, new.l),
        c: d(old.c, new.c),
        v: d(old.v, new.v),
    }
}

/// Converts a whole-bar event stream into field-level data events for `feed`, numbered
/// from `first_seq`: `New` → `bar`; `UpdateLast` → `patch` with only the changed fields
/// (an update that changes nothing produces no event).
///
/// # Errors
/// An `UpdateLast` before any bar or for another `t` than the last bar.
pub fn data_events_from_bar_events(
    feed: &FeedKey,
    events: impl IntoIterator<Item = BarEvent>,
    first_seq: u64,
) -> Result<Vec<DataEvent>, SourceError> {
    let mut out = Vec::new();
    let mut last: Option<Bar> = None;
    let mut seq = first_seq;
    for ev in events {
        let change = match ev {
            BarEvent::New(b) => {
                last = Some(b.clone());
                DataChange::Bar { bar: b }
            }
            BarEvent::UpdateLast(b) => {
                let l = last.as_mut().filter(|l| l.t == b.t).ok_or_else(|| {
                    SourceError::new(format!("update for t={} without that last bar", b.t))
                })?;
                let fields = bar_patch_diff(l, &b);
                *l = b;
                if fields.is_empty() {
                    continue;
                }
                DataChange::Patch { t: l.t, fields }
            }
        };
        out.push(DataEvent {
            feed: feed.clone(),
            seq,
            change,
        });
        seq += 1;
    }
    Ok(out)
}

/// Settings of [`SyntheticEvents`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyntheticEventsConfig {
    /// Number of bars (the final bars are `synthetic_bars(n, seed)`).
    pub n: usize,
    /// Seed of the bars and of the intra-bar price path.
    pub seed: u64,
    /// Events per bar (at least 1): a `bar` event, then `ticks_per_bar - 1` patches.
    pub ticks_per_bar: u32,
    /// First bar to deliver; bars before it are the initial resolution
    /// (`synthetic_bars(start, seed)`).
    pub start: usize,
    /// Every `correct_every`-th delivered bar (0 = never) is first delivered with a wrong
    /// close and volume, then corrected by a patch `correct_after` bars later.
    pub correct_every: usize,
    /// Delay of a correction, in bars (at least 1).
    pub correct_after: usize,
    /// Sequence number of the first event.
    pub first_seq: u64,
}

impl SyntheticEventsConfig {
    /// `n` bars from `seed` in `ticks_per_bar` events each, from bar 0, no corrections,
    /// numbered from 1.
    #[must_use]
    pub fn new(n: usize, seed: u64, ticks_per_bar: u32) -> Self {
        Self {
            n,
            seed,
            ticks_per_bar,
            start: 0,
            correct_every: 0,
            correct_after: 3,
            first_seq: 1,
        }
    }
}

/// A deterministic field-level event stream for one feed.
///
/// Each bar opens with a `bar` event `{o, h: o, l: o, c: o, v: 0}`; each following tick
/// moves the price inside the final bar's `[l, h]` range (touching `h` and `l` at seeded
/// ticks) and sends a `patch` with **only the changed fields**: `c` when the price moved,
/// `h` / `l` only when exceeded, `v` when it grew. The last tick lands on the final bar,
/// so applying the whole stream yields exactly `synthetic_bars(n, seed)`, bit for bit —
/// also with corrections enabled, because a correction restores the true values of a bar
/// first delivered wrong.
#[derive(Debug, Clone)]
pub struct SyntheticEvents {
    feed: FeedKey,
    cfg: SyntheticEventsConfig,
    bars: Vec<Bar>,
    rng: Lcg,
    next_bar: usize,
    seq: u64,
    queue: VecDeque<DataChange>,
    /// (bar index after whose `bar` event it is sent, t, patch).
    corrections: VecDeque<(usize, i64, BarPatch)>,
}

impl SyntheticEvents {
    /// The stream for `feed` with `cfg`.
    #[must_use]
    pub fn new(feed: FeedKey, cfg: SyntheticEventsConfig) -> Self {
        let cfg = SyntheticEventsConfig {
            ticks_per_bar: cfg.ticks_per_bar.max(1),
            correct_after: cfg.correct_after.max(1),
            start: cfg.start.min(cfg.n),
            ..cfg
        };
        Self {
            feed,
            bars: synthetic_bars(cfg.n, cfg.seed),
            rng: Lcg(cfg.seed ^ 0x9e37_79b9_7f4a_7c15),
            next_bar: cfg.start,
            seq: cfg.first_seq,
            queue: VecDeque::new(),
            corrections: VecDeque::new(),
            cfg,
        }
    }

    /// The bars a receiver holds before the first event (`synthetic_bars(start, seed)`).
    #[must_use]
    pub fn initial_bars(&self) -> &[Bar] {
        &self.bars[..self.cfg.start]
    }

    /// The bars after the whole stream (`synthetic_bars(n, seed)`).
    #[must_use]
    pub fn final_bars(&self) -> &[Bar] {
        &self.bars
    }

    fn pick(&mut self, k: u32) -> u32 {
        self.rng.next_u32() % k
    }

    fn wrong(b: &Bar) -> Bar {
        let mut w = b.clone();
        let mid = round2((b.l + b.h) / 2.0).clamp(b.l, b.h);
        w.c = if mid.to_bits() != b.c.to_bits() {
            mid
        } else if b.h.to_bits() != b.c.to_bits() {
            b.h
        } else {
            b.l
        };
        w.v = if b.v > 0.0 { round2(b.v * 0.5) } else { 1.0 };
        w
    }

    fn fill(&mut self) {
        let i = self.next_bar;
        let b = self.bars[i].clone();
        self.next_bar += 1;
        let erroneous = self.cfg.correct_every > 0
            && (i - self.cfg.start) % self.cfg.correct_every == self.cfg.correct_every - 1;
        let target = if erroneous {
            let w = Self::wrong(&b);
            let fix = bar_patch_diff(&w, &b);
            if !fix.is_empty() {
                self.corrections
                    .push_back((i + self.cfg.correct_after, b.t, fix));
            }
            w
        } else {
            b.clone()
        };
        let k = self.cfg.ticks_per_bar;
        if k == 1 {
            self.queue.push_back(DataChange::Bar { bar: target });
        } else {
            let mut s = Bar {
                t: b.t,
                o: b.o,
                h: b.o,
                l: b.o,
                c: b.o,
                v: 0.0,
            };
            self.queue.push_back(DataChange::Bar { bar: s.clone() });
            // Intermediate ticks 1..=k-2 touch h and l once each (when there are any).
            let inner = k - 2;
            let (hi_at, lo_at) = if inner > 0 {
                (1 + self.pick(inner), 1 + self.pick(inner))
            } else {
                (0, 0)
            };
            for j in 1..k {
                let next = if j == k - 1 {
                    target.clone()
                } else {
                    let p = if j == hi_at {
                        b.h
                    } else if j == lo_at {
                        b.l
                    } else {
                        let u = (self.rng.unit() + 1.0) / 2.0;
                        round2(b.l + (b.h - b.l) * u).clamp(b.l, b.h)
                    };
                    let f = f64::from(j) / f64::from(k - 1);
                    Bar {
                        t: b.t,
                        o: b.o,
                        h: s.h.max(p),
                        l: s.l.min(p),
                        c: p,
                        v: round2(b.v * f).min(b.v),
                    }
                };
                let fields = bar_patch_diff(&s, &next);
                s = next;
                if !fields.is_empty() {
                    self.queue.push_back(DataChange::Patch { t: b.t, fields });
                }
            }
        }
        // Corrections due after this bar opened go right after its `bar` event.
        let mut at = 1;
        while self.corrections.front().is_some_and(|c| c.0 <= i) {
            if let Some((_, t, fields)) = self.corrections.pop_front() {
                self.queue.insert(at, DataChange::Patch { t, fields });
                at += 1;
            }
        }
    }
}

impl Iterator for SyntheticEvents {
    type Item = DataEvent;

    fn next(&mut self) -> Option<DataEvent> {
        while self.queue.is_empty() {
            if self.next_bar < self.cfg.n {
                self.fill();
            } else if let Some((_, t, fields)) = self.corrections.pop_front() {
                self.queue.push_back(DataChange::Patch { t, fields });
            } else {
                return None;
            }
        }
        let change = self.queue.pop_front()?;
        let ev = DataEvent {
            feed: self.feed.clone(),
            seq: self.seq,
            change,
        };
        self.seq += 1;
        Some(ev)
    }
}
