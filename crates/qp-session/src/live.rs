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
//!   ([`crate::values::Cache::apply_delta`]);
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

use std::collections::{BTreeMap, BTreeSet, HashMap};

use qp_schema::data::{BindingState, FeedKey};
use qp_schema::protocol::{
    AckStatus, ChangeOp, Event, LogEntry, Op, OutboundLine, Seq, ServerMsg, Topic,
};
use qp_schema::{Condition, Document, SeriesData, SeriesSource};

use crate::apply::apply_change;
use crate::error::{ptr, PResult, E};
use crate::expand::{expand_op, summarize_ops, Ctx};
use crate::plane::{inject, layer_reads, DataPlane, Reader};
use crate::session::{out, permissions, Session};
use crate::values::{eval_condition, find_series, Delta};

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
            crate::apply::apply_all(&mut doc, &e.changes).ok()?;
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
    crate::values::condition_series(c)
        .iter()
        .any(|(_, s)| plane.gate(doc, None, s).is_some())
}

/// True if `principal` may hear the outcome of `c` (`condition_met`, `intent_triggered`,
/// a capsule invalidated by its author's condition): entitled to every bound feed the
/// condition reads, through study inputs (§18.3). The outcome of a condition is data:
/// edge by edge it tells the values.
pub(crate) fn hears(plane: &DataPlane, doc: &Document, principal: &str, c: &Condition) -> bool {
    plane.series.is_empty()
        || crate::values::condition_series(c)
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

/// Every series `c` rests on (its series and, through every study input, theirs), as
/// [`LiveIndex::cond_info`] finds them, by lookup in `doc`.
pub(crate) fn closure_of(doc: &Document, c: &Condition) -> Vec<String> {
    let mut closure: Vec<String> = Vec::new();
    for id in cond_ids(c).into_iter().flatten() {
        let mut stack = vec![id];
        while let Some(cur) = stack.pop() {
            if closure.len() > 4096 || closure.iter().any(|x| x == cur) {
                continue;
            }
            closure.push(cur.to_owned());
            if let Some(SeriesSource::Study { inputs, .. }) =
                find_series(doc, cur).map(|s| &s.source)
            {
                stack.extend(inputs.iter().rev().map(String::as_str));
            }
        }
    }
    closure
}

/// The series ids a condition reads (at most two).
fn cond_ids(c: &Condition) -> [Option<&str>; 2] {
    match c {
        Condition::Threshold { series, .. } => [Some(series), None],
        Condition::Compare { left, right, .. } => [Some(left), Some(right)],
        Condition::Text { .. } => [None, None],
    }
}

/// [`eval_condition`] from known cache slots (no lookups); `None` when a slot is
/// missing (then the caller evaluates by id).
#[allow(clippy::option_option)]
#[inline]
fn eval_slots(
    cache: &crate::values::Cache,
    c: &Condition,
    slots: [Option<usize>; 2],
) -> Option<Option<(bool, u64, i64)>> {
    match c {
        Condition::Threshold { op, value, .. } => {
            let l = cache.val(slots[0]?)?;
            Some(crate::values::eval_on(*op, l, None, *value))
        }
        Condition::Compare { op, .. } => {
            let l = cache.val(slots[0]?)?;
            let r = cache.val(slots[1]?)?;
            Some(crate::values::eval_on(*op, l, Some(r), 0.0))
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
    cache: &mut crate::values::Cache,
    c: &Condition,
    memo: &mut Option<(u64, [Option<usize>; 2])>,
) -> Option<(bool, u64, i64)> {
    if let Some(r) = memo
        .filter(|(g, _)| *g == cache.gen())
        .and_then(|(_, sl)| eval_slots(cache, c, sl))
    {
        return r;
    }
    let r = eval_condition(rd, cache, c);
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

impl Session {
    /// A direct command whose ops are all bar writes (`append_bars`, `update_last_bar`),
    /// applied to the document in place (no copy). Each op is expanded against the
    /// document as the earlier ops leave it and applied; a failing op (or `base`) rolls
    /// the applied ones back. The log entry is the one the copying path writes.
    pub(crate) fn bar_command(
        &mut self,
        ci: usize,
        id: &str,
        ops: &[Op],
        (base, reason): (Option<Seq>, Option<String>),
        resolved: BTreeMap<String, String>,
    ) -> PResult<Vec<OutboundLine>> {
        let role = self.conns[ci].role;
        let perms = permissions(role);
        let principal = self.conns[ci].principal.clone();
        let conn = self.conns[ci].id.clone();
        let author = self
            .principal(&principal)
            .map(|p| p.author.clone())
            .ok_or_else(|| {
                E::new(
                    qp_schema::protocol::ErrorCode::Handshake,
                    "principal vanished",
                )
                .build()
            })?;
        let seq = self.seq() + 1;
        let mut changes = Vec::new();
        let mut groups: Vec<Vec<ChangeOp>> = Vec::new();
        let mut origin = Vec::new();
        let (created, undoes) = {
            let mut ctx = Ctx {
                author,
                approved_by: None,
                at: self.clock,
                seq,
                id_seq: seq,
                n: 0,
                created: Vec::new(),
                speaker: &principal,
                is_owner: role == qp_schema::protocol::Role::Owner,
                can_decide: perms.decide,
                log: &self.log,
                undoes: None,
                data: self.rd.as_ref().map(|rd| Reader {
                    rd,
                    plane: &self.plane,
                }),
                plane: &self.plane,
            };
            let restore = Restore {
                initial: &self.initial,
                log: &self.log,
            };
            let doc = &mut self.doc;
            for (i, op) in ops.iter().enumerate() {
                let p = ptr("/ops", i);
                let cs = match expand_op(doc, op, &p, &mut ctx) {
                    Ok(cs) => cs,
                    Err(e) => {
                        roll_back(doc, &groups, restore);
                        return Err(e);
                    }
                };
                match apply_in_place(doc, &cs, restore) {
                    Ok(inv) => groups.push(inv),
                    Err(m) => {
                        roll_back(doc, &groups, restore);
                        return Err(E::new(
                            qp_schema::protocol::ErrorCode::Invalid,
                            format!("cannot apply: {m}"),
                        )
                        .at(p)
                        .build());
                    }
                }
                origin.extend(std::iter::repeat_n(i, cs.len()));
                changes.extend(cs);
            }
            (ctx.created, ctx.undoes)
        };
        if let Err(e) = self.base_check(base, &changes, &origin) {
            let restore = Restore {
                initial: &self.initial,
                log: &self.log,
            };
            roll_back(&mut self.doc, &groups, restore);
            return Err(e);
        }
        let inverse = groups.into_iter().rev().flatten().collect();
        self.committed(LogEntry {
            reason,
            seq,
            at: self.clock,
            author: principal,
            approved_by: None,
            cause: Some(id.to_owned()),
            request: ops.to_vec(),
            changes,
            inverse,
            undoes,
            summary: summarize_ops(ops, &created),
            evidence: None,
        });
        let mut lines = vec![out(
            &conn,
            ServerMsg::Ack {
                resolved,
                id: id.to_owned(),
                status: AckStatus::Applied,
                seq: Some(seq),
                created,
                proposal: None,
                undoes,
            },
        )];
        lines.extend(self.settle(seq));
        Ok(lines)
    }

    /// True if entry `seq` only writes bars.
    pub(crate) fn bars_only(&self, seq: Seq) -> bool {
        let e = &self.log[usize::try_from(seq - 1).unwrap_or(0)];
        !e.changes.is_empty()
            && e.changes.iter().all(|c| {
                matches!(
                    c,
                    ChangeOp::AppendBars { .. }
                        | ChangeOp::SetLastBar { .. }
                        | ChangeOp::TruncateBars { .. }
                )
            })
    }

    /// After a bar change (entry `entry`, or a data event: `None`): only what rests on
    /// the changed bar series — condition subscriptions and intents (at the current
    /// position), then live capsules' invalidations (engine entries) — and the
    /// notifications in the order of §15. Watchers not resting on a changed series keep
    /// their value (their series did not change), so skipping them changes no output.
    pub(crate) fn settle_live(&mut self, entry: Option<Seq>) -> Vec<OutboundLine> {
        let seq = self.seq();
        let mut changed: Vec<String> = Vec::new();
        for (id, _, _) in &self.pending_views {
            if !changed.contains(id) {
                changed.push(id.clone());
            }
        }
        let changed_ref: Vec<&str> = changed.iter().map(String::as_str).collect();
        let conds = self.eval_conditions_live(seq, &changed_ref);
        let fired = self.eval_intents_live(seq, &changed_ref);
        let mut entries: Vec<Seq> = entry.into_iter().collect();
        entries.extend(self.invalidate_live(&changed_ref));
        entries.extend(self.invalidate_public_only());
        let mut lines = Vec::new();
        for s in entries {
            lines.extend(self.entry_events(s));
        }
        lines.extend(self.run_capsules());
        lines.extend(self.view_events());
        lines.extend(conds);
        let claims = self.claim_events();
        lines.extend(self.intent_lines(fired));
        lines.extend(claims);
        lines
    }

    /// The live index of the current structure generation (built if needed).
    pub(crate) fn take_index(&mut self) -> LiveIndex {
        match self.live_index.take() {
            Some(i) if i.gen == self.struct_gen => i,
            _ => self.build_index(),
        }
    }

    /// Builds the live index of the document: series, feeds, active intents with a
    /// value trigger and live capsules (with the principals who must hear them).
    fn build_index(&self) -> LiveIndex {
        let doc = &self.doc;
        let mut idx = LiveIndex::of_series(doc, self.struct_gen);
        for i in doc.intents.iter().filter(|i| i.active) {
            let Some(trig) = &i.trigger else { continue };
            if matches!(trig, Condition::Text { .. }) {
                continue;
            }
            let info = idx.cond_info(doc, trig);
            idx.intents.push(IntentWatch {
                id: i.id.clone(),
                trigger: trig.clone(),
                info,
                memo: None,
                last: None,
                stamp: 0,
            });
        }
        // (R3/perf) Findings by id (first occurrence, as `iter().find` gives it).
        let mut finding_at: std::collections::HashMap<&str, usize> =
            std::collections::HashMap::with_capacity(doc.findings.len());
        for (i, f) in doc.findings.iter().enumerate() {
            finding_at.entry(f.id.as_str()).or_insert(i);
        }
        for c in doc
            .capsules
            .iter()
            .filter(|c| c.state == qp_schema::CapsuleState::Live)
        {
            let Some(f) = finding_at
                .get(c.finding.as_str())
                .map(|&i| &doc.findings[i])
            else {
                continue;
            };
            let info = idx.cond_info(doc, &f.invalidation);
            // (R3/perf) A condition resting on no series (a text condition, as every
            // engine finding's) is never evaluated by `invalidate_live`: no watch.
            if info.closure.is_empty() {
                continue;
            }
            let who = self
                .responsible(f, info.closure())
                .iter()
                .filter_map(|p| self.principals.iter().position(|x| x.id == *p))
                .collect();
            idx.capsules.push(CapsuleWatch {
                capsule: c.id.clone(),
                finding: c.finding.clone(),
                invalidation: f.invalidation.clone(),
                info,
                who,
                memo: None,
            });
        }
        idx
    }

    /// Edge-triggered condition subscriptions resting on a changed series. A watcher is
    /// evaluated when a cache slot of its series changed in this tick, when its slots are
    /// not known (first time, or after the cache dropped slots), or when a series it
    /// reads is not computable and rests on a changed series.
    fn eval_conditions_live(&mut self, seq: Seq, changed: &[&str]) -> Vec<OutboundLine> {
        let idx = self.take_index();
        let rd = self.rd.as_ref().unwrap_or(&self.doc);
        let cache = &mut self.cache;
        let ready = all_ready(&self.plane);
        let mut ents = Ents::new(&self.plane, &idx.feeds, &self.principals);
        let mut lines = Vec::new();
        for c in &mut self.conns {
            let mut row = None;
            for s in &mut c.subs {
                let Topic::Condition { condition } = &s.topic else {
                    continue;
                };
                if s.watch.as_ref().is_none_or(|(g, _)| *g != idx.gen) {
                    s.watch = Some((idx.gen, idx.cond_info(&self.doc, condition)));
                }
                let info = &s.watch.as_ref().expect("set above").1;
                let ids = cond_ids(condition);
                let affected = match s.memo.filter(|(g, _)| *g == cache.gen()) {
                    None => true,
                    Some((_, slots)) => ids.iter().zip(slots).any(|(id, slot)| match (id, slot) {
                        (Some(_), Some(i)) => cache.touched_now(i),
                        (Some(_), None) => info.rests_on_any(changed),
                        (None, _) => false,
                    }),
                };
                if !affected {
                    continue;
                }
                let who = *row.get_or_insert_with(|| {
                    self.principals
                        .iter()
                        .position(|p| p.id == c.principal)
                        .ok_or(c.principal.as_str())
                });
                if !ents.hears(who, info) {
                    // Not evaluated for a subscriber without the data entitlement (§18.3).
                    continue;
                }
                if info.blocked(&self.plane, ready) {
                    // Not evaluated; the edge state is kept (§18.3).
                    continue;
                }
                let r = eval_memo(rd, cache, condition, &mut s.memo);
                let now = r.is_some_and(|x| x.0);
                let prev = s.last.replace(now);
                if prev == Some(false) && now {
                    if let Some((_, bar, t)) = r {
                        let evidence = crate::claims::evidence(
                            rd,
                            cache,
                            &self.claims,
                            &self.plane,
                            condition,
                        );
                        if let Some(ev) = &evidence {
                            self.claims.assert(
                                crate::claims::Owner::Conn(c.id.clone()),
                                qp_schema::protocol::ClaimRef::Condition { sub: s.id.clone() },
                                condition.clone(),
                                ev.clone(),
                            );
                        }
                        lines.push(out(
                            &c.id,
                            ServerMsg::Notify {
                                sub: s.id.clone(),
                                event: Event::ConditionMet {
                                    bar,
                                    t,
                                    seq,
                                    evidence: evidence.map(|ev| {
                                        crate::claims::for_principal(
                                            &self.doc,
                                            &self.plane,
                                            &c.principal,
                                            &ev,
                                        )
                                    }),
                                },
                            },
                        ));
                    }
                }
            }
        }
        drop(ents);
        self.live_index = Some(idx);
        lines
    }

    /// Active intents whose trigger rests on a changed series; the others keep their
    /// state.
    fn eval_intents_live(&mut self, seq: Seq, changed: &[&str]) -> Vec<Event> {
        let mut idx = self.take_index();
        let rd = self.rd.as_ref().unwrap_or(&self.doc);
        let ready = all_ready(&self.plane);
        let mut fired = Vec::new();
        for w in &mut idx.intents {
            if !w.info.rests_on_any(changed) || w.info.blocked(&self.plane, ready) {
                continue;
            }
            let r = eval_memo(rd, &mut self.cache, &w.trigger, &mut w.memo);
            let now = r.is_some_and(|x| x.0);
            let prev = if w.stamp == self.intent_epoch {
                w.last
            } else {
                self.intent_state.get(&w.id).copied()
            };
            if prev == Some(false) && now {
                if let Some((_, bar, t)) = r {
                    let evidence = crate::claims::evidence(
                        rd,
                        &mut self.cache,
                        &self.claims,
                        &self.plane,
                        &w.trigger,
                    );
                    if let Some(ev) = &evidence {
                        self.claims.assert(
                            crate::claims::Owner::Doc,
                            qp_schema::protocol::ClaimRef::Intent {
                                intent: w.id.clone(),
                            },
                            w.trigger.clone(),
                            ev.clone(),
                        );
                    }
                    fired.push(Event::IntentTriggered {
                        intent: w.id.clone(),
                        bar,
                        t,
                        seq,
                        evidence,
                    });
                }
            }
            if prev != Some(now) {
                self.intent_state.insert(w.id.clone(), now);
            }
            w.last = Some(now);
            w.stamp = self.intent_epoch;
        }
        self.live_index = Some(idx);
        fired
    }

    /// Engine entries invalidating live capsules whose invalidation rests on a changed
    /// series and holds now (evaluated from the index; the ones that hold go through
    /// [`Session::invalidate_these`], which checks them again and writes the entries).
    fn invalidate_live(&mut self, changed: &[&str]) -> Vec<Seq> {
        let mut idx = self.take_index();
        let rd = self.rd.as_ref().unwrap_or(&self.doc);
        let ready = all_ready(&self.plane);
        let mut ents = Ents::new(&self.plane, &idx.feeds, &self.principals);
        let mut holding = Vec::new();
        for w in &mut idx.capsules {
            if !w.info.rests_on_any(changed) || w.info.blocked(&self.plane, ready) {
                continue;
            }
            if !w.who.iter().all(|&k| ents.hears(Ok(k), &w.info)) {
                continue;
            }
            if eval_memo(rd, &mut self.cache, &w.invalidation, &mut w.memo).is_some_and(|r| r.0) {
                holding.push((w.capsule.clone(), w.finding.clone()));
            }
        }
        drop(ents);
        self.live_index = Some(idx);
        if holding.is_empty() {
            return Vec::new();
        }
        self.invalidate_these(holding)
    }

    /// `view_changed` for every pending bar change, to the `view` subscribers whose view
    /// reads the changed series (a bound series only to principals entitled to its feed),
    /// per change in order, then by connection and subscription order (§15).
    pub(crate) fn view_events(&mut self) -> Vec<OutboundLine> {
        let pend = std::mem::take(&mut self.pending_views);
        if pend.is_empty()
            || !self
                .conns
                .iter()
                .any(|c| c.subs.iter().any(|s| matches!(s.topic, Topic::View { .. })))
        {
            return Vec::new();
        }
        let rd = self.rd.as_ref().unwrap_or(&self.doc);
        if self.view_reads.is_none() || !self.incremental {
            self.view_reads = Some(build_view_reads(rd));
        }
        let reads = self.view_reads.as_ref().expect("built above");
        let mut lines = Vec::new();
        for (series, d, seq) in &pend {
            let n = bars_len(rd, series);
            let feed = self
                .plane
                .series
                .get(series)
                .map(|bs| bs.binding.feed_key());
            for c in &self.conns {
                if let Some(k) = &feed {
                    if !self.plane.entitled(&c.principal, k) {
                        continue;
                    }
                }
                for s in &c.subs {
                    let Topic::View { view } = &s.topic else {
                        continue;
                    };
                    if !reads.get(view).is_some_and(|r| r.contains(series)) {
                        continue;
                    }
                    lines.push(out(
                        &c.id,
                        ServerMsg::Notify {
                            sub: s.id.clone(),
                            event: Event::ViewChanged {
                                view: view.clone(),
                                series: series.clone(),
                                from_bar: d.first_bar() as u64,
                                to_bar: n.checked_sub(1).map(|x| x as u64),
                                seq: *seq,
                            },
                        },
                    ));
                }
            }
        }
        lines
    }
}
