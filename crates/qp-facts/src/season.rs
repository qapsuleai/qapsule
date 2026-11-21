//! Intraday seasonal profile (RP4/M26 `mevsim`, productized for the `rp4` detector set): the mean of a per-bar quantity (volume, true range) at the
//! same time of day over the last `days` **past** trading days.
//!
//! - **Slot** of a bar: minutes from the day's session open (XIST, `qp_calendar`), except
//!   the day's last bar (its span reaches the session close, closing auction merged;
//!   half days included), which is the [`CLOSE`] slot.
//! - **Bar length** is inferred from the series itself: the smallest gap between two
//!   consecutive bars of the same local day seen so far. A series with no two bars on one
//!   day (daily bars) never gets a length: it is not intraday and every factor is exactly
//!   `1.0` (product behaviour).
//! - **Period floor**: the adjustment is *active* only when the inferred bar length is at
//!   least `min_len_ms` (the set uses one hour, [`crate::SEASON_MIN_LEN_MS`]); below it every factor is
//!   exactly `1.0` and the detectors are the product's.
//! - **Factor** of a slot = `mean(slot) / overall`, `mean(slot)` the mean of its last
//!   `days` values (at least `min_days`), `overall` the mean of all warm slot means,
//!   frozen at the first bar of each day (so it holds past days only). A slot's own
//!   values come from past days only because a slot occurs once per day. Not warm → `1.0`.
//!
//! Pure and deterministic: the state is a function of the committed bars; `locate` and
//! `factor` do not change it (the [`crate::Engine`] eval/commit split).
//!
//! Provenance: `rounds/RP4/experiments/M26/mevsim/src/season.rs` (measured; choice in
//! `rounds/RP4/experiments/M26/RESULTS.md`). Bit-identical to it (cross-check in
//! `rounds/RP4/experiments/harness/tests/rp4_product.rs`).

use std::collections::{BTreeMap, VecDeque};

use qp_calendar::{builtin, ExchangeCalendar};

/// Slot key of the day's last bar.
pub const CLOSE: i64 = -1;

const MS_PER_MIN: i64 = 60_000;

/// Where a bar sits in its day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pos {
    t: i64,
    day: i64,
    /// Inferred bar length (ms) including this bar; `None` = not intraday (yet).
    len: Option<i64>,
    /// Slot; `None` outside any session or before the bar length is known.
    slot: Option<i64>,
    /// Intraday with a bar length at or above the floor.
    active: bool,
}

impl Pos {
    /// Whether the seasonal adjustment is active at this bar (intraday, bar length at or
    /// above the floor).
    #[must_use]
    pub fn active(&self) -> bool {
        self.active
    }

    /// The slot, if known.
    #[must_use]
    pub fn slot(&self) -> Option<i64> {
        self.slot
    }
}

/// The seasonal profile of one quantity (see the module docs).
#[derive(Debug, Clone)]
pub struct Season {
    cal: ExchangeCalendar,
    days: usize,
    min_days: usize,
    min_len_ms: i64,
    hist: BTreeMap<i64, VecDeque<f64>>,
    mean: BTreeMap<i64, f64>,
    overall: Option<f64>,
    prev: Option<(i64, i64)>,
    len: Option<i64>,
}

impl Season {
    /// A profile over the last `days` past days, warm from `min_days`, active for bar
    /// lengths of at least `min_len_ms` (XIST calendar).
    ///
    /// # Panics
    /// Never (XIST is built in).
    #[must_use]
    pub fn new(days: usize, min_days: usize, min_len_ms: i64) -> Self {
        let cal = ExchangeCalendar::new(&builtin("XIST").expect("XIST")).expect("XIST valid");
        Self {
            cal,
            days: days.max(1),
            min_days: min_days.clamp(1, days.max(1)),
            min_len_ms,
            hist: BTreeMap::new(),
            mean: BTreeMap::new(),
            overall: None,
            prev: None,
            len: None,
        }
    }

    /// Position of a bar opening at `t` on top of the committed state.
    #[must_use]
    pub fn locate(&self, t: i64) -> Pos {
        let day = self.cal.local_day(t);
        let len = match self.prev {
            Some((pt, pd)) if pd == day && t > pt => {
                Some(self.len.map_or(t - pt, |l| l.min(t - pt)))
            }
            _ => self.len,
        };
        let slot = match (len, self.cal.day_span(day)) {
            (Some(l), Some(span)) if t >= span.open && t < span.close => {
                if t + l >= span.close {
                    Some(CLOSE)
                } else {
                    Some((t - span.open) / MS_PER_MIN)
                }
            }
            _ => None,
        };
        Pos {
            t,
            day,
            len,
            slot,
            active: len.is_some_and(|l| l >= self.min_len_ms),
        }
    }

    /// Seasonal factor at `pos` (`1.0` when not active or not warm).
    #[must_use]
    pub fn factor(&self, pos: &Pos) -> f64 {
        if !pos.active {
            return 1.0;
        }
        let (Some(slot), Some(overall)) = (pos.slot, self.overall_at(pos)) else {
            return 1.0;
        };
        match self.mean.get(&slot) {
            Some(&m) if m > 0.0 && overall > 0.0 && m.is_finite() && overall.is_finite() => {
                m / overall
            }
            _ => 1.0,
        }
    }

    /// The overall mean in force for `pos`: frozen at the start of its day.
    fn overall_at(&self, pos: &Pos) -> Option<f64> {
        match self.prev {
            Some((_, pd)) if pd == pos.day => self.overall,
            // First bar of a new day: what the freeze at its commit will produce.
            _ => self.overall_now(),
        }
    }

    fn overall_now(&self) -> Option<f64> {
        if self.mean.is_empty() {
            return None;
        }
        let mut s = 0.0;
        for m in self.mean.values() {
            s += m;
        }
        #[allow(clippy::cast_precision_loss)]
        Some(s / self.mean.len() as f64)
    }

    /// Folds the bar at `pos` with value `x` into the profile.
    pub fn commit(&mut self, pos: &Pos, x: f64) {
        if self.prev.is_none_or(|(_, pd)| pd != pos.day) {
            self.overall = self.overall_now();
        }
        if let (Some(slot), true) = (pos.slot, x.is_finite()) {
            let h = self.hist.entry(slot).or_default();
            h.push_back(x);
            while h.len() > self.days {
                h.pop_front();
            }
            if h.len() >= self.min_days {
                let mut s = 0.0;
                for v in h.iter() {
                    s += v;
                }
                #[allow(clippy::cast_precision_loss)]
                self.mean.insert(slot, s / h.len() as f64);
            }
        }
        self.prev = Some((pos.t, pos.day));
        self.len = pos.len;
    }
}
