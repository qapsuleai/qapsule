//! The shared engine: one side's bars per binding, the reference RSI, and the reference
//! detector's claim set, maintained incrementally from source events.
//!
//! GOAL invariant: authority and replica run the same engine code. Every approach builds
//! its sides on this type (it may add its own bookkeeping on top, but it must not
//! re-implement bar application, RSI or the detector).
//!
//! Sequence handling ([`GapPolicy`]):
//! - `Strict` — `docs/PROTOCOL.md` §18.4 literally: `seq <= last` is a repeat (ignored);
//!   `seq > last + 1` is a gap: the feed becomes stale and nothing is applied until a
//!   snapshot (or settled bars) arrives.
//! - `SeqLww` (default) — the realistic receiver: repeats are ignored; gaps are recorded
//!   (the missing numbers are remembered) and events are still applied; every bar field
//!   remembers the `seq` that last wrote it and a field is only overwritten by a higher
//!   `seq` (last writer wins), so late (reordered) events never roll a field back. A patch
//!   for a bar the receiver does not have creates that bar from the patch (fields it
//!   does not carry are filled from its price and can be overwritten by any real event).

use std::collections::{BTreeMap, BTreeSet};

use qp_schema::data::{data_digest, DataChange, DataPosition, EvidenceRef, DATA_DIGEST_ALG};
use qp_schema::Bar;

use crate::adapter::{BindingSetup, Setup, SourceEvent};
use crate::detector::{Claim, ClaimDelta, ClaimKey, DetectorCfg, EvidenceRange};
use crate::series::SeriesState;

/// Sequence-number policy.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum GapPolicy {
    /// Stop at a gap until resynchronised (PROTOCOL §18.4).
    Strict,
    /// Apply despite gaps; per-field last-writer-wins by `seq`.
    #[default]
    SeqLww,
}

/// Outcome of applying one event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplyStatus {
    /// Applied (something may or may not have changed).
    Applied,
    /// A repeat (`seq` already seen): ignored.
    Duplicate,
    /// A gap was detected before this event (`SeqLww`: the event was still applied;
    /// `Strict`: not applied, the feed is stale).
    Gap {
        /// Expected `seq`.
        expected: u64,
        /// Received `seq`.
        got: u64,
    },
    /// Not applied: the feed is stale (`Strict`).
    Stale,
    /// Not applicable (unknown feed, bad bar).
    Rejected(String),
}

/// What applying an event did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Applied {
    /// Binding index (`usize::MAX` if unknown).
    pub binding: usize,
    /// Status.
    pub status: ApplyStatus,
    /// Earliest bar time whose data or RSI changed (`None`: nothing changed).
    pub changed_from_t: Option<i64>,
    /// A new last bar was added (the previous last bar closed).
    pub new_bar: bool,
}

#[derive(Debug)]
struct FeedState {
    series: SeriesState,
    field_seq: BTreeMap<i64, [u64; 5]>,
    last_seq: u64,
    missing: BTreeSet<u64>,
    gaps: u64,
    stale: bool,
}

/// One side's engine.
#[derive(Debug)]
pub struct Engine {
    bindings: Vec<BindingSetup>,
    ids: Vec<String>,
    cfg: DetectorCfg,
    policy: GapPolicy,
    feeds: Vec<FeedState>,
    claims: BTreeMap<ClaimKey, Claim>,
    deltas: Vec<ClaimDelta>,
    work: u64,
}

const MAX_MISSING: usize = 100_000;

fn patch_fields(p: &qp_schema::data::BarPatch) -> [Option<f64>; 5] {
    [p.o, p.h, p.l, p.c, p.v]
}

fn bar_fields(b: &Bar) -> [Option<f64>; 5] {
    [Some(b.o), Some(b.h), Some(b.l), Some(b.c), Some(b.v)]
}

fn set_field(b: &mut Bar, i: usize, x: f64) {
    match i {
        0 => b.o = x,
        1 => b.h = x,
        2 => b.l = x,
        3 => b.c = x,
        _ => b.v = x,
    }
}

impl Engine {
    /// An engine for `setup` with the default policy (`SeqLww`).
    #[must_use]
    pub fn new(setup: &Setup) -> Self {
        Self::with_policy(setup, GapPolicy::default())
    }

    /// An engine with an explicit sequence policy.
    #[must_use]
    pub fn with_policy(setup: &Setup, policy: GapPolicy) -> Self {
        let feeds = setup
            .bindings
            .iter()
            .map(|_| FeedState {
                series: SeriesState::new(setup.detector.spec.period, setup.extra_studies),
                field_seq: BTreeMap::new(),
                last_seq: 0,
                missing: BTreeSet::new(),
                gaps: 0,
                stale: false,
            })
            .collect();
        Self {
            bindings: setup.bindings.clone(),
            ids: setup.bindings.iter().map(|b| b.id.clone()).collect(),
            cfg: setup.detector.clone(),
            policy,
            feeds,
            claims: BTreeMap::new(),
            deltas: Vec::new(),
            work: 0,
        }
    }

    /// Binding index of an id.
    #[must_use]
    pub fn index(&self, id: &str) -> Option<usize> {
        self.ids.iter().position(|x| x == id)
    }

    /// Binding ids.
    #[must_use]
    pub fn ids(&self) -> &[String] {
        &self.ids
    }

    /// The detector configuration.
    #[must_use]
    pub fn detector(&self) -> &DetectorCfg {
        &self.cfg
    }

    /// Bars of binding `b`.
    #[must_use]
    pub fn bars(&self, b: usize) -> &[Bar] {
        self.feeds[b].series.bars()
    }

    /// Bars of a binding by id (empty if unknown).
    #[must_use]
    pub fn bars_of(&self, id: &str) -> &[Bar] {
        self.index(id).map_or(&[], |b| self.bars(b))
    }

    /// Bars + RSI of binding `b`.
    #[must_use]
    pub fn series(&self, b: usize) -> &SeriesState {
        &self.feeds[b].series
    }

    /// Current claim set on this side's data.
    #[must_use]
    pub fn claims(&self) -> &BTreeMap<ClaimKey, Claim> {
        &self.claims
    }

    /// The claim with this key if it holds on this side's data.
    #[must_use]
    pub fn claim(&self, key: &ClaimKey) -> Option<&Claim> {
        self.claims.get(key)
    }

    /// Claim-set changes since the last call.
    pub fn take_deltas(&mut self) -> Vec<ClaimDelta> {
        std::mem::take(&mut self.deltas)
    }

    /// Source sequence number of the last applied event of binding `b`.
    #[must_use]
    pub fn last_seq(&self, b: usize) -> u64 {
        self.feeds[b].last_seq
    }

    /// Gaps detected on binding `b`.
    #[must_use]
    pub fn gaps(&self, b: usize) -> u64 {
        self.feeds[b].gaps
    }

    /// Missing sequence numbers (SeqLww) of binding `b`.
    #[must_use]
    pub fn missing(&self, b: usize) -> usize {
        self.feeds[b].missing.len()
    }

    /// True if binding `b` is stale (Strict policy after a gap).
    #[must_use]
    pub fn is_stale(&self, b: usize) -> bool {
        self.feeds[b].stale
    }

    /// True if binding `b` holds a bar at `t`.
    #[must_use]
    pub fn has_bar(&self, b: usize, t: i64) -> bool {
        self.feeds[b].series.index_of(t).is_some()
    }

    /// True if the bar at `t` exists and is closed (a later bar exists).
    #[must_use]
    pub fn is_closed(&self, b: usize, t: i64) -> bool {
        let s = &self.feeds[b].series;
        s.index_of(t).is_some_and(|i| i + 1 < s.bars().len())
    }

    /// True if every evidence range's first and last bar are held.
    #[must_use]
    pub fn covers(&self, ev: &[EvidenceRange]) -> bool {
        ev.iter().all(|r| {
            self.index(&r.binding)
                .is_some_and(|b| self.has_bar(b, r.from_t) && self.has_bar(b, r.to_t))
        })
    }

    /// Bars of binding `b` with `from_t <= t <= to_t`.
    #[must_use]
    pub fn range(&self, b: usize, from_t: i64, to_t: i64) -> &[Bar] {
        let bars = self.bars(b);
        let lo = bars.partition_point(|x| x.t < from_t);
        let hi = bars.partition_point(|x| x.t <= to_t);
        &bars[lo..hi.max(lo)]
    }

    /// Digest (`qp_schema::data::data_digest`) and bar count of a range. Counts one work
    /// unit per bar.
    pub fn digest(&mut self, b: usize, from_t: i64, to_t: i64) -> (u64, String) {
        let (n, d) = {
            let r = self.range(b, from_t, to_t);
            (r.len() as u64, data_digest(r))
        };
        self.work += n;
        (n, d)
    }

    /// Frozen evidence reference of a range (`qp_schema::data::EvidenceRef`).
    pub fn evidence_ref(&mut self, b: usize, from_t: i64, to_t: i64) -> EvidenceRef {
        let (bars, digest) = self.digest(b, from_t, to_t);
        EvidenceRef {
            series: self.ids[b].clone(),
            binding: Some(self.bindings[b].binding.clone()),
            from_t,
            to_t,
            bars,
            alg: DATA_DIGEST_ALG.to_owned(),
            digest,
        }
    }

    /// Data position of binding `b` (digest of all bars; one work unit per bar).
    pub fn position(&mut self, b: usize) -> DataPosition {
        let f = &self.feeds[b];
        let bars = f.series.bars();
        let pos = DataPosition {
            feed: self.bindings[b].feed.clone(),
            bars: bars.len() as u64,
            last_t: bars.last().map(|x| x.t),
            seq: f.last_seq,
            digest: data_digest(bars),
        };
        self.work += bars.len() as u64;
        pos
    }

    /// Deterministic work units so far (study steps + detector evaluations + digests).
    #[must_use]
    pub fn work(&self) -> u64 {
        self.work + self.feeds.iter().map(|f| f.series.work()).sum::<u64>()
    }

    /// Adds work units of the caller's own (e.g. a state machine step).
    pub fn add_work(&mut self, units: u64) {
        self.work += units;
    }

    fn refresh_claims(&mut self, b: usize, from_t: i64) {
        let series: Vec<&SeriesState> = self.feeds.iter().map(|f| &f.series).collect();
        let w = self.cfg.recompute(
            &self.ids,
            &series,
            b,
            from_t,
            &mut self.claims,
            &mut self.deltas,
        );
        self.work += w;
    }

    /// Applies a source event.
    pub fn apply(&mut self, ev: &SourceEvent) -> Applied {
        let Some(b) = self.bindings.iter().position(|x| &x.feed == ev.feed()) else {
            return Applied {
                binding: usize::MAX,
                status: ApplyStatus::Rejected("unknown feed".into()),
                changed_from_t: None,
                new_bar: false,
            };
        };
        match ev {
            SourceEvent::Snapshot { seq, bars, .. } => {
                let f = &mut self.feeds[b];
                f.series.set_bars(bars.clone());
                f.field_seq = bars.iter().map(|x| (x.t, [*seq; 5])).collect();
                f.last_seq = *seq;
                f.missing.clear();
                f.stale = false;
                self.refresh_claims(b, i64::MIN);
                Applied {
                    binding: b,
                    status: ApplyStatus::Applied,
                    changed_from_t: Some(i64::MIN),
                    new_bar: false,
                }
            }
            SourceEvent::Data(d) => self.apply_data(b, d.seq, &d.change),
        }
    }

    fn apply_data(&mut self, b: usize, seq: u64, change: &DataChange) -> Applied {
        let policy = self.policy;
        let f = &mut self.feeds[b];
        let mut status = ApplyStatus::Applied;
        let done = |status| Applied {
            binding: b,
            status,
            changed_from_t: None,
            new_bar: false,
        };
        match policy {
            GapPolicy::Strict => {
                if f.stale {
                    return done(ApplyStatus::Stale);
                }
                if seq <= f.last_seq {
                    return done(ApplyStatus::Duplicate);
                }
                if seq > f.last_seq + 1 {
                    f.stale = true;
                    f.gaps += 1;
                    return done(ApplyStatus::Gap {
                        expected: f.last_seq + 1,
                        got: seq,
                    });
                }
                f.last_seq = seq;
            }
            GapPolicy::SeqLww => {
                if seq <= f.last_seq {
                    if !f.missing.remove(&seq) {
                        return done(ApplyStatus::Duplicate);
                    }
                } else {
                    if seq > f.last_seq + 1 {
                        status = ApplyStatus::Gap {
                            expected: f.last_seq + 1,
                            got: seq,
                        };
                        f.gaps += 1;
                        for s in (f.last_seq + 1)..seq {
                            if f.missing.len() >= MAX_MISSING {
                                break;
                            }
                            f.missing.insert(s);
                        }
                    }
                    f.last_seq = seq;
                }
            }
        }
        let old_last = f.series.bars().last().map(|x| x.t);
        let old_len = f.series.bars().len();
        let (t, fields) = match change {
            DataChange::Bar { bar } => (bar.t, bar_fields(bar)),
            DataChange::Patch { t, fields } => (*t, patch_fields(fields)),
        };
        if fields.iter().flatten().any(|x| !x.is_finite()) {
            return done(ApplyStatus::Rejected("non-finite value".into()));
        }
        let existing = f.series.index_of(t).map(|i| f.series.bars()[i].clone());
        let strict = policy == GapPolicy::Strict;
        let new = match existing {
            Some(mut bar) => {
                if strict && matches!(change, DataChange::Bar { .. }) {
                    return done(ApplyStatus::Rejected("bar not after the last bar".into()));
                }
                let fs = f.field_seq.entry(t).or_insert([0; 5]);
                let mut changed = false;
                for (i, x) in fields.iter().enumerate() {
                    if let Some(x) = x {
                        if strict || seq > fs[i] {
                            fs[i] = seq;
                            set_field(&mut bar, i, *x);
                            changed = true;
                        }
                    }
                }
                changed.then_some(bar)
            }
            None => {
                if strict {
                    match change {
                        DataChange::Bar { bar } if old_last.is_none_or(|l| bar.t > l) => {
                            f.field_seq.insert(t, [seq; 5]);
                            Some(bar.clone())
                        }
                        _ => return done(ApplyStatus::Rejected("no bar at this time".into())),
                    }
                } else {
                    let base = fields[3]
                        .or(fields[0])
                        .or(fields[1])
                        .or(fields[2])
                        .or_else(|| {
                            let bars = f.series.bars();
                            let i = bars.partition_point(|x| x.t < t);
                            i.checked_sub(1).map(|j| bars[j].c)
                        });
                    let Some(base) = base else {
                        return done(ApplyStatus::Rejected("no price for a missing bar".into()));
                    };
                    let mut bar = Bar {
                        t,
                        o: base,
                        h: base,
                        l: base,
                        c: base,
                        v: 0.0,
                    };
                    let mut fs = [0u64; 5];
                    for (i, x) in fields.iter().enumerate() {
                        if let Some(x) = x {
                            set_field(&mut bar, i, *x);
                            fs[i] = seq;
                        }
                    }
                    f.field_seq.insert(t, fs);
                    Some(bar)
                }
            }
        };
        let Some(bar) = new else {
            return done(status);
        };
        let from = f.series.upsert(bar);
        let new_bar = f.series.bars().len() > old_len && old_last.is_some_and(|l| t > l);
        let from = if new_bar {
            old_last.map_or(from, |l| l.min(from))
        } else {
            from
        };
        self.refresh_claims(b, from);
        Applied {
            binding: b,
            status,
            changed_from_t: Some(from),
            new_bar,
        }
    }

    /// Applies settled (provider-finalised) bars of binding `b`, e.g. from a
    /// [`crate::adapter::FetchReply`]. Each bar replaces the bar at its time or is
    /// inserted; its fields count as written at the current `seq` (a later event still
    /// overrides them).
    pub fn apply_settled(&mut self, b: usize, bars: &[Bar]) -> Applied {
        let f = &mut self.feeds[b];
        let mut from: Option<i64> = None;
        let old_last = f.series.bars().last().map(|x| x.t);
        for bar in bars {
            let same = f.series.index_of(bar.t).map(|i| &f.series.bars()[i]) == Some(bar);
            if same {
                continue;
            }
            f.field_seq.insert(bar.t, [f.last_seq; 5]);
            let t = f.series.upsert(bar.clone());
            from = Some(from.map_or(t, |x: i64| x.min(t)));
        }
        let new_last = f.series.bars().last().map(|x| x.t);
        let new_bar = new_last > old_last && old_last.is_some();
        if let Some(mut t) = from {
            if new_bar {
                t = t.min(old_last.unwrap_or(t));
            }
            self.refresh_claims(b, t);
        }
        Applied {
            binding: b,
            status: ApplyStatus::Applied,
            changed_from_t: from,
            new_bar,
        }
    }
}
