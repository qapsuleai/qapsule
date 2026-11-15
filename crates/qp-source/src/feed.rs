//! The bars of one feed and the rules for applying data events to them
//! (`docs/PROTOCOL.md` §18.4).
//!
//! A [`FeedStore`] is what each side (authority, replica, test provider) keeps per feed
//! ("Besleme", `FeedKey`): the bars, the last applied source sequence number and a lazily
//! maintained digest prefix so that a [`DataPosition`] can be published often without
//! rehashing every bar.

use std::fmt;

use qp_schema::data::{data_digest_u64, DataChange, DataEvent, DataPosition, FeedKey};
use qp_schema::{Bar, TimestampMs};

const FNV_PRIME: u64 = 0x0100_0000_01b3;

fn fnv_word(mut h: u64, v: u64) -> u64 {
    for b in v.to_le_bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(FNV_PRIME);
    }
    h
}

/// FNV-1a 64 offset basis, taken from `qp_schema` (digest of no bars).
fn fnv_offset() -> u64 {
    data_digest_u64(&[])
}

/// Folds one bar into a running [`qp_schema::data::data_digest_u64`] state. Folding all
/// bars in order from the offset basis gives exactly `data_digest_u64(bars)` (tested).
#[must_use]
pub fn digest_extend(h: u64, b: &Bar) -> u64 {
    let mut h = fnv_word(h, u64::from_le_bytes(b.t.to_le_bytes()));
    for v in [b.o, b.h, b.l, b.c, b.v] {
        h = fnv_word(h, v.to_bits());
    }
    h
}

/// What applying one event did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Applied {
    /// A `bar` event appended a bar at `index`.
    NewBar {
        /// Index of the new bar.
        index: usize,
    },
    /// A `patch` on the last bar (live update).
    LiveUpdate {
        /// Index of the last bar.
        index: usize,
    },
    /// A `patch` on an earlier bar (correction). Everything computed from bar `index` on
    /// is invalid.
    Correction {
        /// Index of the corrected bar.
        index: usize,
    },
    /// `seq` was not after the last applied one: ignored (idempotent).
    Duplicate,
    /// `seq` skipped numbers: not applied; the store is stale until [`FeedStore::reset`].
    /// While stale every further (non-duplicate) event is reported as the same gap.
    Gap {
        /// The sequence number the store waited for.
        expected: u64,
        /// The sequence number that arrived.
        got: u64,
    },
    /// The event concerns a time outside the store's window bounds (a closed window's
    /// live bars, a correction before the window's anchor): `seq` advances, no bar
    /// changes.
    OutsideWindow {
        /// Time the event concerns.
        t: TimestampMs,
    },
}

/// An event the rules reject (§18.4: not applied; reported to the host). A rejected
/// event does **not** advance `seq`: the store missed a step of the source, so the next
/// event is reported as a [`Applied::Gap`] and the host re-resolves.
#[derive(Debug, Clone, PartialEq)]
pub enum EventError {
    /// The event is for another feed.
    WrongFeed {
        /// The event's feed.
        got: Box<FeedKey>,
    },
    /// No store for the event's feed (a provider that does not hold it).
    UnknownFeed {
        /// The event's feed.
        feed: Box<FeedKey>,
    },
    /// A `bar` event whose `t` is not after the last bar.
    BarNotAfterLast {
        /// The new bar's time.
        t: TimestampMs,
        /// The last bar's time.
        last_t: TimestampMs,
    },
    /// A `patch` for a time the store has no bar for.
    UnknownT {
        /// The patch's time.
        t: TimestampMs,
    },
    /// A `patch` without fields.
    EmptyPatch {
        /// The patch's time.
        t: TimestampMs,
    },
    /// A non-finite value (NaN, ±inf).
    NonFinite {
        /// Time of the bar.
        t: TimestampMs,
        /// Field name (`o`, `h`, `l`, `c`, `v`).
        field: &'static str,
    },
    /// The resulting bar breaks `h >= max(o, c)`, `l <= min(o, c)`, `l <= h` (the rule of
    /// `qp_schema::validate_bars`, which every held bar satisfies).
    InconsistentOhlc {
        /// Time of the bar.
        t: TimestampMs,
    },
}

impl fmt::Display for EventError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongFeed { got } => write!(
                f,
                "event for feed {}/{} given to another feed's store",
                got.symbol, got.interval
            ),
            Self::UnknownFeed { feed } => {
                write!(f, "no feed {}/{}", feed.symbol, feed.interval)
            }
            Self::BarNotAfterLast { t, last_t } => {
                write!(f, "new bar at t={t} is not after the last bar (t={last_t})")
            }
            Self::UnknownT { t } => write!(f, "patch for t={t}: no bar at that time"),
            Self::EmptyPatch { t } => write!(f, "patch for t={t} has no fields"),
            Self::NonFinite { t, field } => write!(f, "bar t={t}: `{field}` is not finite"),
            Self::InconsistentOhlc { t } => write!(f, "bar t={t}: inconsistent OHLC"),
        }
    }
}

impl std::error::Error for EventError {}

/// Optional time bounds of a store holding a resolved window (§18.2): events concerning
/// `t < from` or `t > to` are [`Applied::OutsideWindow`]. Without bounds such events are
/// errors (`UnknownT`) or new bars.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TimeBounds {
    /// Earliest time held (the anchor or `from_t`).
    pub from: Option<TimestampMs>,
    /// Latest time held (`to_t` of a closed window).
    pub to: Option<TimestampMs>,
}

impl TimeBounds {
    /// True if `t` lies inside the bounds.
    #[must_use]
    pub fn contains(&self, t: TimestampMs) -> bool {
        self.from.is_none_or(|a| t >= a) && self.to.is_none_or(|b| t <= b)
    }
}

/// The bars of one feed, the last applied source `seq` and the gap state.
///
/// Digest maintenance: `prefix[k]` is the digest state after bars `0..k`
/// (`prefix[0]` = offset basis). The vector is kept valid only up to the first changed
/// bar and extended lazily when a digest is asked for, so:
///
/// | change | cost when applied | cost of the next [`FeedStore::position`] |
/// |--------|-------------------|-------------------------------------------|
/// | new bar | O(1) | O(new bars since last position) |
/// | live update (last bar) | O(1) | O(1) |
/// | correction at bar `i` | O(1) (truncate) | O(n − i) |
///
/// Memory: 8 bytes per bar.
#[derive(Debug, Clone)]
pub struct FeedStore {
    feed: FeedKey,
    bars: Vec<Bar>,
    seq: u64,
    stale: Option<(u64, u64)>,
    bounds: TimeBounds,
    prefix: Vec<u64>,
}

fn check_bar(b: &Bar) -> Result<(), EventError> {
    for (field, v) in [("o", b.o), ("h", b.h), ("l", b.l), ("c", b.c), ("v", b.v)] {
        if !v.is_finite() {
            return Err(EventError::NonFinite { t: b.t, field });
        }
    }
    if b.h < b.o.max(b.c) || b.l > b.o.min(b.c) || b.l > b.h {
        return Err(EventError::InconsistentOhlc { t: b.t });
    }
    Ok(())
}

impl FeedStore {
    /// An empty store at `seq` 0.
    #[must_use]
    pub fn new(feed: FeedKey) -> Self {
        Self {
            feed,
            bars: Vec::new(),
            seq: 0,
            stale: None,
            bounds: TimeBounds::default(),
            prefix: vec![fnv_offset()],
        }
    }

    /// A store holding `bars` (initial resolution) at source position `seq`.
    ///
    /// # Errors
    /// Bars that are not strictly increasing in `t`, non-finite or inconsistent.
    pub fn with_bars(feed: FeedKey, bars: Vec<Bar>, seq: u64) -> Result<Self, EventError> {
        let mut s = Self::new(feed);
        s.reset(bars, seq)?;
        Ok(s)
    }

    /// Sets window bounds (see [`TimeBounds`]).
    #[must_use]
    pub fn with_bounds(mut self, bounds: TimeBounds) -> Self {
        self.bounds = bounds;
        self
    }

    /// Replaces all bars and the position (re-resolution after a gap); clears staleness.
    ///
    /// # Errors
    /// As [`FeedStore::with_bars`]; the store is unchanged on error.
    pub fn reset(&mut self, bars: Vec<Bar>, seq: u64) -> Result<(), EventError> {
        for (i, b) in bars.iter().enumerate() {
            check_bar(b)?;
            if i > 0 && b.t <= bars[i - 1].t {
                return Err(EventError::BarNotAfterLast {
                    t: b.t,
                    last_t: bars[i - 1].t,
                });
            }
        }
        self.bars = bars;
        self.seq = seq;
        self.stale = None;
        self.prefix.truncate(1);
        Ok(())
    }

    /// The feed.
    #[must_use]
    pub fn feed(&self) -> &FeedKey {
        &self.feed
    }

    /// The bars held.
    #[must_use]
    pub fn bars(&self) -> &[Bar] {
        &self.bars
    }

    /// Number of bars.
    #[must_use]
    pub fn len(&self) -> usize {
        self.bars.len()
    }

    /// True if no bars are held.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bars.is_empty()
    }

    /// Source sequence number of the last applied event (0 = initial bars only).
    #[must_use]
    pub fn seq(&self) -> u64 {
        self.seq
    }

    /// The window bounds.
    #[must_use]
    pub fn bounds(&self) -> TimeBounds {
        self.bounds
    }

    /// `Some((expected, got))` of the first gap if the store is stale.
    #[must_use]
    pub fn stale(&self) -> Option<(u64, u64)> {
        self.stale
    }

    /// Index of the bar at time `t`. O(log n).
    #[must_use]
    pub fn index_of(&self, t: TimestampMs) -> Option<usize> {
        self.bars.binary_search_by_key(&t, |b| b.t).ok()
    }

    /// Applies one event by the §18.4 rules. Order of checks: feed, duplicate, stale/gap,
    /// then the change's own rules.
    ///
    /// # Errors
    /// See [`EventError`]; on error nothing changes (not even `seq`).
    pub fn apply(&mut self, ev: &DataEvent) -> Result<Applied, EventError> {
        if ev.feed != self.feed {
            return Err(EventError::WrongFeed {
                got: Box::new(ev.feed.clone()),
            });
        }
        if ev.seq <= self.seq {
            return Ok(Applied::Duplicate);
        }
        if let Some((expected, _)) = self.stale {
            return Ok(Applied::Gap {
                expected,
                got: ev.seq,
            });
        }
        let expected = self.seq + 1;
        if ev.seq != expected {
            self.stale = Some((expected, ev.seq));
            return Ok(Applied::Gap {
                expected,
                got: ev.seq,
            });
        }
        let applied = match &ev.change {
            DataChange::Bar { bar } => self.apply_bar(bar)?,
            DataChange::Patch { t, fields } => {
                if fields.is_empty() {
                    return Err(EventError::EmptyPatch { t: *t });
                }
                for (field, v) in [
                    ("o", fields.o),
                    ("h", fields.h),
                    ("l", fields.l),
                    ("c", fields.c),
                    ("v", fields.v),
                ] {
                    if v.is_some_and(|x| !x.is_finite()) {
                        return Err(EventError::NonFinite { t: *t, field });
                    }
                }
                match self.index_of(*t) {
                    Some(index) => {
                        let mut b = self.bars[index].clone();
                        fields.apply_to(&mut b);
                        check_bar(&b)?;
                        self.bars[index] = b;
                        self.prefix.truncate(index + 1);
                        if index + 1 == self.bars.len() {
                            Applied::LiveUpdate { index }
                        } else {
                            Applied::Correction { index }
                        }
                    }
                    None if !self.bounds.contains(*t) => Applied::OutsideWindow { t: *t },
                    None => return Err(EventError::UnknownT { t: *t }),
                }
            }
        };
        self.seq = ev.seq;
        Ok(applied)
    }

    fn apply_bar(&mut self, bar: &Bar) -> Result<Applied, EventError> {
        check_bar(bar)?;
        if let Some(last) = self.bars.last() {
            if bar.t <= last.t {
                return Err(EventError::BarNotAfterLast {
                    t: bar.t,
                    last_t: last.t,
                });
            }
        }
        if !self.bounds.contains(bar.t) {
            return Ok(Applied::OutsideWindow { t: bar.t });
        }
        self.bars.push(bar.clone());
        Ok(Applied::NewBar {
            index: self.bars.len() - 1,
        })
    }

    /// Digest state of bars `0..k` (`k <= len`), extending the prefix cache as needed.
    ///
    /// # Panics
    /// `k > len`.
    pub fn prefix_digest_u64(&mut self, k: usize) -> u64 {
        assert!(k <= self.bars.len(), "prefix longer than the bars");
        while self.prefix.len() <= k {
            let i = self.prefix.len() - 1;
            let h = digest_extend(self.prefix[i], &self.bars[i]);
            self.prefix.push(h);
        }
        self.prefix[k]
    }

    /// [`qp_schema::data::data_digest_u64`] of all bars, incrementally.
    pub fn digest_u64(&mut self) -> u64 {
        self.prefix_digest_u64(self.bars.len())
    }

    /// Where this store stands (§18.3 `DataPosition`).
    pub fn position(&mut self) -> DataPosition {
        DataPosition {
            feed: self.feed.clone(),
            bars: self.bars.len() as u64,
            last_t: self.bars.last().map(|b| b.t),
            seq: self.seq,
            digest: format!("{:016x}", self.digest_u64()),
        }
    }
}
