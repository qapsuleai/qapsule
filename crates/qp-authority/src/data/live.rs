//! The incremental live path (RP2/T5, `docs/PROTOCOL.md` §18.8).
//!
//! A host data event on a bound series ([`crate::Session::data_event`]) or an inline bar
//! command (`append_bars`, `update_last_bar`) is applied **without copying the document**
//! and updates only what rests on the ticked bar series:
//!
//! - the document (inline) or the reading document (bound) takes the change in place;
//!   the log entry of an inline command is built exactly as the copying path builds it
//!   (same changes, same inverse);
//! - the values cache advances the streaming state of every study series on the ticked
//!   series (`push` for a new bar, `update_last` for a live revision; a correction or a
//!   truncation recomputes those studies) and leaves every other series alone
//!   ([`qp_plane::values::Cache::apply_delta`]);
//! - only the `condition` subscriptions, intents and live capsules whose condition reads
//!   a series resting on the ticked series are evaluated; only the `view` subscriptions
//!   of views reading such a series are told (`view_changed`).
//!
//! The result (outputs, log, document) is byte-identical to the from-scratch path
//! ([`crate::Session::set_incremental`]`(false)`), which a property test checks.
//!
//! Dependency index: study series → their input (one bar series; `values` computes a
//! study from its first input), kept by the cache per bar series; conditions and views →
//! the series they read, followed upstream through every study input
//! ([`rests_on`], the view index); the cache's touch stamps tell a watcher whether this
//! tick changed one of its series.

mod eval;

use std::collections::{BTreeMap, BTreeSet, HashMap};

use qp_schema::data::{BindingState, FeedKey};
use qp_schema::protocol::{
    AckStatus, ChangeOp, Event, LogEntry, Op, OutboundLine, Seq, ServerMsg, Topic,
};
use qp_schema::{Condition, Document, SeriesData, SeriesSource};

use crate::session::{out, permissions, Session};
use qp_ops::apply::apply_change;
use qp_ops::expand::{summarize_ops, Ctx};
use qp_ops::live::cond_ids;
use qp_plane::error::{ptr, PResult, E};
use qp_plane::plane::{inject, layer_reads, DataPlane, Reader};
use qp_plane::values::{find_series, Delta};

/// Bars of a series in `doc` (inline or injected bound bars).
fn bars_len(doc: &Document, id: &str) -> usize {
    find_series(doc, id)
        .and_then(qp_schema::Series::bars)
        .map_or(0, <[qp_schema::Bar]>::len)
}

/// How the entry's bar changes changed each bar series (`doc`: after the entry), in
/// order of first appearance. One change: its exact delta; several on one series: a
/// rebuild from the first bar any of them touches.
pub(crate) fn bar_deltas(doc: &Document, changes: &[ChangeOp]) -> Vec<(String, Delta)> {
    let mut per: Vec<(&str, Vec<&ChangeOp>)> = Vec::new();
    for c in changes {
        let id = match c {
            ChangeOp::AppendBars { series, .. }
            | ChangeOp::SetLastBar { series, .. }
            | ChangeOp::TruncateBars { series, .. } => series.as_str(),
            _ => continue,
        };
        match per.iter_mut().find(|(s, _)| *s == id) {
            Some((_, v)) => v.push(c),
            None => per.push((id, vec![c])),
        }
    }
    per.into_iter()
        .map(|(id, cs)| {
            let len = bars_len(doc, id);
            // Walk back from the length after the entry: each change's first touched bar.
            let mut after = len;
            let mut from = usize::MAX;
            for c in cs.iter().rev() {
                match c {
                    ChangeOp::AppendBars { bars, .. } => {
                        after = after.saturating_sub(bars.len());
                        from = from.min(after);
                    }
                    ChangeOp::SetLastBar { .. } => from = from.min(after.saturating_sub(1)),
                    ChangeOp::TruncateBars { count, .. } => {
                        from = from.min(after);
                        after += *count as usize;
                    }
                    _ => {}
                }
            }
            let d = match cs.as_slice() {
                [ChangeOp::AppendBars { .. }] => Delta::Appended { from },
                [ChangeOp::SetLastBar { .. }] => Delta::Revised { index: from },
                _ => Delta::Rebuild { from },
            };
            (id.to_owned(), d)
        })
        .collect()
}

/// True if a change cannot alter which series a view reads (the view index stays).
pub(crate) fn keeps_view_reads(c: &ChangeOp) -> bool {
    matches!(
        c,
        ChangeOp::AppendBars { .. }
            | ChangeOp::SetLastBar { .. }
            | ChangeOp::TruncateBars { .. }
            | ChangeOp::SetCapsuleState { .. }
            | ChangeOp::PutCapsule { .. }
            | ChangeOp::PutIntent { .. }
            | ChangeOp::PutNarrative { .. }
            | ChangeOp::PutProposal { .. }
            | ChangeOp::PushLedger { .. }
            | ChangeOp::PopLedger {}
            | ChangeOp::SetXDomain { .. }
    )
}

/// What a failed in-place application falls back on when an inverse does not fit (it
/// never should): the document rebuilt from the initial document and the log (the
/// entries already committed; the failed command is not among them).
#[derive(Clone, Copy)]
pub(crate) struct Restore<'a> {
    pub initial: &'a Document,
    pub log: &'a [LogEntry],
}

impl Restore<'_> {
    /// The document as the committed log leaves it (`None` if that fails as well).
    fn rebuild(self) -> Option<Document> {
        let mut doc = self.initial.clone();
        for e in self.log {
            qp_ops::apply::apply_all(&mut doc, &e.changes).ok()?;
        }
        Some(doc)
    }
}

/// Undoes `inverses` (in the order given). An inverse that does not fit would leave the
/// document half rolled back: debug builds stop; release builds rebuild the document
/// from the log (RP2 T5 audit D4).
fn undo_in_place<'a>(
    doc: &mut Document,
    inverses: impl Iterator<Item = &'a ChangeOp>,
    restore: Restore<'_>,
) {
    for inv in inverses {
        if let Err(e) = apply_change(doc, inv) {
            debug_assert!(false, "an inverse does not fit: {e}");
            if let Some(d) = restore.rebuild() {
                *doc = d;
            }
            return;
        }
    }
}

/// Applies `changes` to `doc` in place and returns their inverse (reversed, as
/// [`crate::apply_all`]). If one does not fit, the ones applied before it are undone
/// with their inverses, so `doc` is left as it was (the same bytes, as undo guarantees),
/// and the error reads as [`crate::apply_all`]'s. Should an inverse not fit, the
/// document is rebuilt from `restore`.
///
/// # Errors
/// The first change that does not fit.
pub(crate) fn apply_in_place(
    doc: &mut Document,
    changes: &[ChangeOp],
    restore: Restore<'_>,
) -> Result<Vec<ChangeOp>, String> {
    let mut inverse = Vec::with_capacity(changes.len());
    for (i, c) in changes.iter().enumerate() {
        match apply_change(doc, c) {
            Ok(inv) => inverse.push(inv),
            Err(e) => {
                undo_in_place(doc, inverse.iter().rev(), restore);
                return Err(format!("change {i}: {e}"));
            }
        }
    }
    inverse.reverse();
    Ok(inverse)
}

/// Undoes groups of inverses (each already reversed) in reverse order.
fn roll_back(doc: &mut Document, groups: &[Vec<ChangeOp>], restore: Restore<'_>) {
    undo_in_place(doc, groups.iter().rev().flatten(), restore);
}

/// Mirrors a data plane change of bound series `id` into the reading document in place:
/// new bars are pushed, a revised bar replaced; anything else (a state change) re-injects
/// the bars (or removes them when the binding is no longer ready).
pub(crate) fn mirror(rd: &mut Document, plane: &DataPlane, id: &str, d: Delta) {
    let Some(src) = plane.bars(id) else {
        inject(rd, plane, id);
        return;
    };
    let Some(s) = rd.series.iter_mut().find(|s| s.id == id) else {
        return;
    };
    let done = match (&mut s.data, d) {
        (Some(SeriesData::Bars { bars }), Delta::Appended { from }) if bars.len() == from => {
            bars.extend_from_slice(&src[from..]);
            true
        }
        (Some(SeriesData::Bars { bars }), Delta::Revised { index })
            if bars.len() == src.len() && index < bars.len() =>
        {
            bars[index] = src[index].clone();
            true
        }
        _ => false,
    };
    if !done {
        inject(rd, plane, id);
    }
}

/// True if `c` reads a series resting on a bound series that is not ready for the
/// authority (pending, no data, stale): the condition is not evaluated (§18.3).
pub(crate) fn cond_blocked(plane: &DataPlane, doc: &Document, c: &Condition) -> bool {
    if plane.series.is_empty() || all_ready(plane) {
        return false;
    }
    qp_plane::values::condition_series(c)
        .iter()
        .any(|(_, s)| plane.gate(doc, None, s).is_some())
}

/// True if `principal` may hear the outcome of `c` (`condition_met`, `intent_triggered`,
/// a capsule invalidated by its author's condition): entitled to every bound feed the
/// condition reads, through study inputs (§18.3). The outcome of a condition is data:
/// edge by edge it tells the values.
pub(crate) fn hears(plane: &DataPlane, doc: &Document, principal: &str, c: &Condition) -> bool {
    plane.series.is_empty()
        || qp_plane::values::condition_series(c)
            .iter()
            .all(|(_, s)| plane.unentitled(doc, principal, s).is_none())
}

/// Every bound series is ready for the authority (then no condition is blocked: the
/// plane holds every bound series of the document after a sync).
fn all_ready(plane: &DataPlane) -> bool {
    plane
        .series
        .values()
        .all(|bs| bs.bound.current_state() == BindingState::Ready)
}

/// [`eval_condition`] from known cache slots (no lookups); `None` when a slot is
/// missing (then the caller evaluates by id).
#[allow(clippy::option_option)]
#[inline]
fn eval_slots(
    cache: &qp_plane::values::Cache,
    c: &Condition,
    slots: [Option<usize>; 2],
) -> Option<Option<(bool, u64, i64)>> {
    match c {
        Condition::Threshold { op, value, .. } => {
            let l = cache.val(slots[0]?)?;
            Some(qp_plane::values::eval_on(*op, l, None, *value))
        }
        Condition::Compare { op, .. } => {
            let l = cache.val(slots[0]?)?;
            let r = cache.val(slots[1]?)?;
            Some(qp_plane::values::eval_on(*op, l, Some(r), 0.0))
        }
        Condition::Text { .. } => Some(None),
    }
}

/// The view index: view id → every series its layers read (`plane::layer_reads`) and,
/// upstream of studies among them, their inputs (transitively).
fn build_view_reads(doc: &Document) -> BTreeMap<String, BTreeSet<String>> {
    let mut out = BTreeMap::new();
    for v in &doc.views {
        let mut set = BTreeSet::new();
        let mut stack: Vec<String> = v.layers.iter().flat_map(|l| layer_reads(doc, l)).collect();
        while let Some(id) = stack.pop() {
            if set.len() > 4096 || !set.insert(id.clone()) {
                continue;
            }
            if let Some(SeriesSource::Study { inputs, .. }) =
                find_series(doc, &id).map(|s| &s.source)
            {
                stack.extend(inputs.iter().cloned());
            }
        }
        out.insert(v.id.clone(), set);
    }
    out
}

/// Where one condition sits in the document's structure (RP2 T5 fixes): every series
/// its series rest on (themselves included, through every study input), the bound
/// series among them and their feeds. Derived from the document; rebuilt when the
/// structure changes ([`LiveIndex`]).
#[derive(Debug, Default, Clone)]
pub(crate) struct CondInfo {
    /// Series ids the condition rests on (the condition → bar series index: a tick on bar
    /// series `X` affects the condition iff `X` is here).
    closure: Vec<String>,
    /// Bound series among them (readiness, §18.3).
    bound: Vec<String>,
    /// Their feeds, as positions in [`LiveIndex::feeds`] (entitlement, §18.3).
    feeds: Vec<u32>,
}

impl CondInfo {
    /// Series the condition rests on.
    pub(crate) fn closure(&self) -> &[String] {
        &self.closure
    }

    /// True if one of `changed` (bar series) is among the series the condition rests on.
    #[inline]
    fn rests_on_any(&self, changed: &[&str]) -> bool {
        changed.iter().any(|b| self.closure.iter().any(|s| s == b))
    }

    /// [`cond_blocked`] from the index (`all_ready`: [`all_ready`] of `plane`).
    #[inline]
    fn blocked(&self, plane: &DataPlane, all_ready: bool) -> bool {
        if plane.series.is_empty() || all_ready {
            return false;
        }
        self.bound.iter().any(|id| {
            plane
                .series
                .get(id)
                .is_none_or(|bs| bs.bound.current_state() != BindingState::Ready)
        })
    }
}

/// An active intent with a value trigger, as the live path evaluates it.
#[derive(Debug)]
pub(crate) struct IntentWatch {
    id: String,
    trigger: Condition,
    info: CondInfo,
    /// Cache slots of the trigger's series, with their cache generation.
    memo: Option<(u64, [Option<usize>; 2])>,
    /// The intent's edge state as last written by the live path, valid while the
    /// session's `intent_epoch` is `stamp` (then it equals `intent_state[id]`).
    last: Option<bool>,
    stamp: u64,
}

/// A live capsule whose finding exists, as the live path evaluates it.
#[derive(Debug)]
pub(crate) struct CapsuleWatch {
    capsule: String,
    finding: String,
    invalidation: Condition,
    info: CondInfo,
    /// Principals of this session (positions in `Session::principals`) who must be
    /// entitled to every feed the invalidation reads for it to be evaluated
    /// ([`Session::responsible`], §18.3).
    who: Vec<usize>,
    memo: Option<(u64, [Option<usize>; 2])>,
}

/// The live path's view of the document structure: series positions, bound feeds,
/// the watched intents and capsules with their [`CondInfo`]. Built for one structure
/// generation of the session (bumped by every log entry that is not only bar writes);
/// condition subscriptions keep their own [`CondInfo`] with the generation.
#[derive(Debug, Default)]
pub(crate) struct LiveIndex {
    pub(crate) gen: u64,
    /// Series id → position in `doc.series` (the first one, as `find_series`).
    pos: HashMap<String, usize>,
    /// Position in `doc.series` → feed position, for bound series.
    feed_of: Vec<Option<u32>>,
    /// Distinct feeds of the document's bound series.
    feeds: Vec<FeedKey>,
    intents: Vec<IntentWatch>,
    capsules: Vec<CapsuleWatch>,
}

impl LiveIndex {
    /// The index of `doc`'s series and feeds (no watchers yet).
    fn of_series(doc: &Document, gen: u64) -> Self {
        let mut idx = Self {
            gen,
            ..Self::default()
        };
        for (i, s) in doc.series.iter().enumerate() {
            idx.pos.entry(s.id.clone()).or_insert(i);
            let f = s.binding().map(|b| {
                let k = b.feed_key();
                match idx.feeds.iter().position(|x| *x == k) {
                    Some(j) => j as u32,
                    None => {
                        idx.feeds.push(k);
                        (idx.feeds.len() - 1) as u32
                    }
                }
            });
            idx.feed_of.push(f);
        }
        idx
    }

    /// Where `c` sits in `doc` (the document this index was built from).
    pub(crate) fn cond_info(&self, doc: &Document, c: &Condition) -> CondInfo {
        let mut info = CondInfo::default();
        for id in cond_ids(c).into_iter().flatten() {
            let mut stack = vec![id];
            while let Some(cur) = stack.pop() {
                if info.closure.len() > 4096 || info.closure.iter().any(|x| x == cur) {
                    continue;
                }
                info.closure.push(cur.to_owned());
                let Some(&i) = self.pos.get(cur) else {
                    continue;
                };
                match &doc.series[i].source {
                    SeriesSource::Bind { .. } => {
                        info.bound.push(cur.to_owned());
                        if let Some(f) = self.feed_of[i] {
                            if !info.feeds.contains(&f) {
                                info.feeds.push(f);
                            }
                        }
                    }
                    SeriesSource::Study { inputs, .. } => {
                        stack.extend(inputs.iter().rev().map(String::as_str));
                    }
                    _ => {}
                }
            }
        }
        info
    }
}

/// Data entitlements asked during one live change, per principal of the session (by
/// position) and feed: the provider is asked once per pair.
struct Ents<'a> {
    plane: &'a DataPlane,
    feeds: &'a [FeedKey],
    principals: &'a [qp_schema::protocol::Principal],
    asked: Vec<Vec<Option<bool>>>,
}

impl<'a> Ents<'a> {
    fn new(
        plane: &'a DataPlane,
        feeds: &'a [FeedKey],
        principals: &'a [qp_schema::protocol::Principal],
    ) -> Self {
        Self {
            plane,
            feeds,
            principals,
            asked: vec![Vec::new(); principals.len()],
        }
    }

    /// [`hears`] from the index, for principal `who` (its position in the session's
    /// principals, or its id when it is not one).
    #[inline]
    fn hears(&mut self, who: Result<usize, &str>, info: &CondInfo) -> bool {
        if info.feeds.is_empty() || self.plane.series.is_empty() {
            return true;
        }
        let (plane, feeds) = (self.plane, self.feeds);
        let k = match who {
            Ok(k) => k,
            Err(p) => {
                return info
                    .feeds
                    .iter()
                    .all(|&f| plane.entitled(p, &feeds[f as usize]))
            }
        };
        let principal = self.principals[k].id.as_str();
        let row = &mut self.asked[k];
        if row.is_empty() {
            row.resize(feeds.len(), None);
        }
        info.feeds.iter().all(|&f| {
            let f = f as usize;
            *row[f].get_or_insert_with(|| plane.entitled(principal, &feeds[f]))
        })
    }
}

/// [`eval_condition`] through a slot memo: from the known cache slots when they are of
/// the current cache generation, else by id (and the slots are noted).
#[inline]
fn eval_memo(
    rd: &Document,
    cache: &mut qp_plane::values::Cache,
    c: &Condition,
    memo: &mut Option<(u64, [Option<usize>; 2])>,
    reg: &qp_subsystem::Registry,
) -> Option<(bool, u64, i64)> {
    if let Some(r) = memo
        .filter(|(g, _)| *g == cache.gen())
        .and_then(|(_, sl)| eval_slots(cache, c, sl))
    {
        return r;
    }
    let r = reg.eval_condition(rd, cache, c);
    let ids = cond_ids(c);
    *memo = Some((
        cache.gen(),
        [
            ids[0].and_then(|i| cache.slot_of(i)),
            ids[1].and_then(|i| cache.slot_of(i)),
        ],
    ));
    r
}
