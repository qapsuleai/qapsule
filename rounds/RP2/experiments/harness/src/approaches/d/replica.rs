//! D, replica side: the per-claim display state machine.
//!
//! ```text
//!  authority holds it ──► evidence window not in my data ──────────────► Pending
//!                     └─► covered ─► predicate fails on my data ─┬─ provisional ─► Pending
//!                                                               └─ final ─► refetch own
//!                                                                  source ─► Disputed
//!                                 └─► predicate holds ─┬─ provisional (or my bar open) ─► Provisional
//!                                                      └─ final ─► window digest + anchor
//!                                                         + values equal ─► Final
//!                                                         else refetch own source ─►
//!                                                         Final | SourceDiffers
//!  authority retracts ─► not shown
//! ```
//!
//! Every change of the replica's own data re-evaluates the claims it touches in the same
//! callback, so an asserting label never outlives the predicate on the visible data.

use std::collections::{BTreeMap, BTreeSet};

use qp_schema::data::{DataChange, FeedKey};
use qp_schema::Bar;

use crate::adapter::{Ctx, FetchReply, ReplicaAdapter, Setup, SourceEvent};
use crate::detector::{Claim, ClaimKey, Direction, EvidenceRange, Maturity, CROSS, PAIR};
use crate::display::{DisplayItem, DisplayState, Label};
use crate::engine::{Applied, Engine};
use crate::spec::VTime;

use super::wire::{chunk_of, chunk_span, digest, hex, Batch, Envelope, Mat, Op, Report, CHUNK};
use super::{Cfg, DigestMode};

/// Coalescing delay before comparing own bars with data marks (lets an in-flight
/// correction on either side land first).
const CHECK_DELAY: VTime = 300_000;
/// A label already `Final` survives a digest mismatch this long (the other side's
/// update — new envelope or own feed's correction — is usually in flight).
const GRACE: VTime = 2_000_000;
/// Claim-level refetch attempts before giving up (`SourceDiffers` / `Disputed`).
const MAX_TRIES: u32 = 2;
/// How long one claim-level attempt waits for its refetch (or for the marks / alignment
/// it relies on) before the next attempt.
const WAIT: VTime = 3_000_000;
/// Bar-level refetch attempts per mark.
const MAX_BAR_TRIES: u32 = 2;

const TOKEN_CHECK: u64 = 1 << 56;
const TOKEN_EVAL: u64 = 2 << 56;

#[derive(Debug, Clone)]
struct View {
    s: u64,
    key: ClaimKey,
    mat: Mat,
    v: BTreeMap<String, f64>,
    ev: Vec<Envelope>,
    retracted: bool,
}

#[derive(Debug, Clone, Default)]
struct ClaimState {
    tries: u32,
    waiting_until: Option<VTime>,
    ranges: Vec<(usize, i64, i64)>,
    mismatch_since: Option<VTime>,
    reported: bool,
}

#[derive(Debug, Clone, Copy, Default)]
struct Dwell {
    since: Option<VTime>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FetchKind {
    Repair,
    Anchor,
}

#[derive(Debug, Clone, Copy)]
struct Fetch {
    b: usize,
    from: i64,
    to: i64,
    req_seq: u64,
    kind: FetchKind,
}

/// The replica.
#[derive(Debug)]
pub struct Replica {
    cfg: Cfg,
    engine: Engine,
    feeds: Vec<FeedKey>,
    display: DisplayState,
    interval_ms: i64,
    period: usize,
    views: BTreeMap<u64, View>,
    by_key: BTreeMap<ClaimKey, u64>,
    /// `(binding, bar_t, id)` for every viewed claim (pair claims under both bindings).
    index: BTreeSet<(usize, i64, u64)>,
    state: BTreeMap<u64, ClaimState>,
    anchors: BTreeMap<String, i64>,
    aligning: Vec<bool>,
    mark_bar: Vec<BTreeMap<i64, (u64, u64)>>,
    mark_chunk: Vec<BTreeMap<i64, (u64, u64)>>,
    bar_tries: BTreeMap<(usize, i64), u32>,
    check_bars: Vec<BTreeSet<i64>>,
    check_chunks: Vec<BTreeSet<i64>>,
    check_set: bool,
    head: Vec<i64>,
    floor: Vec<i64>,
    queue: Vec<Vec<(i64, i64)>>,
    /// Feed writes to bars that were already closed in the feed's own time
    /// (corrections, late ticks): `bar t -> seq`.
    corr: Vec<BTreeMap<i64, u64>>,
    inflight: BTreeMap<u64, Fetch>,
    eval_timers: BTreeSet<(VTime, u64)>,
    dwell: BTreeMap<u64, Dwell>,
}

fn is_closed_in(bars: &[Bar], t: i64) -> bool {
    bars.binary_search_by_key(&t, |b| b.t)
        .is_ok_and(|i| i + 1 < bars.len())
}

fn values_equal(a: &BTreeMap<String, f64>, b: &BTreeMap<String, f64>) -> bool {
    a.len() == b.len()
        && a
            .iter()
            .zip(b)
            .all(|((ka, va), (kb, vb))| ka == kb && va.to_bits() == vb.to_bits())
}

impl Replica {
    /// New replica side.
    #[must_use]
    pub fn new(setup: &Setup, cfg: Cfg) -> Self {
        let n = setup.bindings.len();
        Self {
            cfg,
            engine: Engine::new(setup),
            feeds: setup.bindings.iter().map(|b| b.feed.clone()).collect(),
            display: DisplayState::default(),
            interval_ms: setup.bindings.first().map_or(60_000, |b| b.interval_ms),
            period: setup.detector.spec.period as usize,
            views: BTreeMap::new(),
            by_key: BTreeMap::new(),
            index: BTreeSet::new(),
            state: BTreeMap::new(),
            anchors: BTreeMap::new(),
            aligning: vec![false; n],
            mark_bar: vec![BTreeMap::new(); n],
            mark_chunk: vec![BTreeMap::new(); n],
            bar_tries: BTreeMap::new(),
            check_bars: vec![BTreeSet::new(); n],
            check_chunks: vec![BTreeSet::new(); n],
            check_set: false,
            head: vec![i64::MIN; n],
            floor: vec![i64::MIN; n],
            queue: vec![Vec::new(); n],
            corr: vec![BTreeMap::new(); n],
            inflight: BTreeMap::new(),
            eval_timers: BTreeSet::new(),
            dwell: BTreeMap::new(),
        }
    }

    fn binding_of(&self, feed: &FeedKey) -> Option<usize> {
        self.feeds.iter().position(|f| f == feed)
    }

    /// Same rule as the oracle's M1: the claim's bar is held with at least `period` bars
    /// before it (and, for `closed`, a later bar exists) on every binding.
    fn covered(&self, key: &ClaimKey, closed: bool) -> bool {
        key.bindings().iter().all(|id| {
            let Some(b) = self.engine.index(id) else {
                return false;
            };
            let s = self.engine.series(b);
            s.index_of(key.bar_t)
                .is_some_and(|i| i >= self.period && (!closed || i + 1 < s.bars().len()))
        })
    }

    /// Provisional guard (margin on the replica's own live values).
    fn guard(&self, c: &Claim) -> bool {
        if c.maturity == Maturity::Final || self.cfg.prov_margin <= 0.0 {
            return true;
        }
        let g = |k: &str| c.values.get(k).copied().unwrap_or(f64::NAN);
        let level = g("level");
        let m = if c.key.detector == PAIR {
            g("a").min(g("b")) - level
        } else if c.key.detector == CROSS && c.key.direction == Direction::Up {
            g("value") - level
        } else {
            level - g("value")
        };
        m >= self.cfg.prov_margin
    }

    /// Provisional path with hysteresis: once shown, a claim stays while its predicate
    /// holds; (re)showing needs the margin and the dwell time.
    fn provisional(&mut self, ctx: &mut Ctx, id: u64, c: &Claim) -> Label {
        let shown = self
            .display
            .items
            .get(&c.key)
            .is_some_and(|x| x.label.asserts());
        if shown || c.maturity == Maturity::Final {
            self.dwell.remove(&id);
            return Label::Provisional;
        }
        if !self.guard(c) {
            self.dwell.remove(&id);
            return Label::Pending;
        }
        if self.cfg.prov_dwell <= 0 {
            return Label::Provisional;
        }
        let now = ctx.now();
        let since = *self.dwell.entry(id).or_default().since.get_or_insert(now);
        if now - since >= self.cfg.prov_dwell {
            Label::Provisional
        } else {
            ctx.set_timer(since + self.cfg.prov_dwell, TOKEN_EVAL | id);
            Label::Pending
        }
    }

    fn queue_fetch(&mut self, b: usize, from: i64, to: i64) {
        if !self.cfg.repair || to < from {
            return;
        }
        // Merge only overlapping / nearly adjacent ranges: far-apart bars are fetched
        // separately (a merged range would refetch everything in between).
        let gap = 2 * self.interval_ms;
        let (mut f, mut t) = (from, to);
        self.queue[b].retain(|&(a, z)| {
            if a <= t.saturating_add(gap) && f <= z.saturating_add(gap) {
                f = f.min(a);
                t = t.max(z);
                false
            } else {
                true
            }
        });
        self.queue[b].push((f, t));
    }

    fn pump(&mut self, ctx: &mut Ctx) {
        for b in 0..self.queue.len() {
            for (from, to) in std::mem::take(&mut self.queue[b]) {
                let covered = self.inflight.values().any(|x| {
                    x.b == b && x.kind == FetchKind::Repair && x.from <= from && x.to >= to
                });
                if covered {
                    continue;
                }
                let id = ctx.fetch_settled(b, from, to);
                self.inflight.insert(
                    id,
                    Fetch {
                        b,
                        from,
                        to,
                        req_seq: self.engine.last_seq(b),
                        kind: FetchKind::Repair,
                    },
                );
            }
        }
    }

    fn schedule_check(&mut self, ctx: &mut Ctx) {
        if !self.check_set {
            self.check_set = true;
            ctx.set_timer(ctx.now() + CHECK_DELAY, TOKEN_CHECK);
        }
    }

    /// Queues the marked bars / chunks of `b` touching `[lo, hi]` for comparison.
    fn recheck(&mut self, b: usize, lo: i64, hi: i64) {
        if !self.cfg.marks {
            return;
        }
        let ts: Vec<i64> = self.mark_bar[b].range(lo..=hi).map(|(t, _)| *t).collect();
        self.check_bars[b].extend(ts);
        let iv = self.interval_ms;
        let (c0, c1) = (chunk_of(lo.max(i64::MIN / 4), iv), chunk_of(hi, iv));
        let cs: Vec<i64> = self.mark_chunk[b].range(c0..=c1).map(|(c, _)| *c).collect();
        self.check_chunks[b].extend(cs);
    }

    /// Compares own bars with the data marks (E) and queues refetches of the bars that
    /// differ.
    fn run_checks(&mut self) {
        let iv = self.interval_ms;
        for b in 0..self.check_bars.len() {
            let bars = std::mem::take(&mut self.check_bars[b]);
            let chunks = std::mem::take(&mut self.check_chunks[b]);
            let mut work = 0;
            let mut want: Vec<(i64, i64)> = Vec::new();
            {
                let own = self.engine.bars(b);
                for t in bars {
                    let Some(&(_, d)) = self.mark_bar[b].get(&t) else {
                        continue;
                    };
                    let Ok(i) = own.binary_search_by_key(&t, |x| x.t) else {
                        continue;
                    };
                    if i + 1 >= own.len() {
                        continue; // not closed here yet; re-checked when it closes
                    }
                    work += 1;
                    if digest(&own[i..=i]) != d {
                        want.push((t, t));
                    }
                }
                for c in chunks {
                    let Some(&(_, d)) = self.mark_chunk[b].get(&c) else {
                        continue;
                    };
                    let (cs, ce) = chunk_span(c, iv);
                    let lo = own.partition_point(|x| x.t < cs);
                    let hi = own.partition_point(|x| x.t <= ce);
                    if (hi - lo) as i64 != CHUNK || hi >= own.len() {
                        continue; // incomplete or not closed here
                    }
                    work += CHUNK as u64;
                    if digest(&own[lo..hi]) != d {
                        want.push((cs, ce));
                    }
                }
            }
            self.engine.add_work(work);
            for (from, to) in want {
                let tries = self.bar_tries.entry((b, from)).or_insert(0);
                if *tries < MAX_BAR_TRIES {
                    *tries += 1;
                    self.queue_fetch(b, from, to);
                }
            }
        }
    }

    /// Anchor alignment: the replica's compute start must be the authority's.
    fn align(&mut self, ctx: &mut Ctx, b: usize) {
        if !self.cfg.anchor_align || self.aligning[b] {
            return;
        }
        let Some(&a) = self.anchors.get(&self.engine.ids()[b]) else {
            return;
        };
        let Some(first) = self.engine.bars(b).first().map(|x| x.t) else {
            return;
        };
        if first > a {
            let id = ctx.fetch_settled(b, a, first - 1);
            self.inflight.insert(
                id,
                Fetch {
                    b,
                    from: a,
                    to: first - 1,
                    req_seq: self.engine.last_seq(b),
                    kind: FetchKind::Anchor,
                },
            );
            self.aligning[b] = true;
        } else if first < a {
            // More history than the authority: drop the bars before its anchor.
            let bars: Vec<Bar> = self
                .engine
                .bars(b)
                .iter()
                .filter(|x| x.t >= a)
                .cloned()
                .collect();
            let ev = SourceEvent::Snapshot {
                feed: self.feeds[b].clone(),
                seq: self.engine.last_seq(b),
                bars,
            };
            let ap = self.engine.apply(&ev);
            self.after_apply(b, &ap);
        }
    }

    fn view_ranges(v: &View) -> Vec<EvidenceRange> {
        v.ev.iter()
            .map(|e| EvidenceRange {
                binding: e.series.clone(),
                from_t: e.from_t,
                to_t: e.to_t,
            })
            .collect()
    }

    /// Final-path verification (T2): window digest, anchor, values. Returns the ranges
    /// to refetch if anything differs (`Err(None)` when refetching cannot help).
    #[allow(clippy::type_complexity)]
    fn verify(
        &mut self,
        v: &View,
        c: &Claim,
        tries: u32,
    ) -> Result<(), Option<Vec<(usize, i64, i64)>>> {
        if self.cfg.digest == DigestMode::None {
            return Ok(());
        }
        let mut ranges = Vec::new();
        let mut anchor_diff = false;
        let mut work = 0;
        for e in &v.ev {
            let Some(b) = self.engine.index(&e.series) else {
                return Err(None);
            };
            let own_anchor = self.engine.bars(b).first().map_or(i64::MIN, |x| x.t);
            if own_anchor != e.anchor_t {
                anchor_diff = true;
            }
            let w = self.engine.range(b, e.from_t, e.to_t);
            work += w.len() as u64;
            let wd = hex(digest(w));
            if w.len() as u64 != e.bars || e.digest.as_deref().is_some_and(|d| d != wd) {
                ranges.push((b, e.from_t, e.to_t));
            }
            if let Some(p) = &e.prefix {
                let r = self.engine.range(b, e.anchor_t, e.to_t);
                work += r.len() as u64;
                if own_anchor != e.anchor_t || hex(digest(r)) != *p {
                    ranges.push((b, own_anchor.max(e.anchor_t), e.to_t));
                }
            }
        }
        self.engine.add_work(work);
        let values_ok = values_equal(&v.v, &c.values);
        if ranges.is_empty() && values_ok {
            return Ok(());
        }
        if !ranges.is_empty() {
            return Err(Some(ranges));
        }
        if anchor_diff {
            // Values differ because the computations start at different bars; a refetch of
            // the window cannot fix that; anchor alignment (if enabled) is what fixes it.
            return Err(self.cfg.anchor_align.then(Vec::new));
        }
        // Window equal, values differ: a difference outside the window. With E the marks
        // locate it (give them one round); otherwise refetch everything up to the claim.
        if self.cfg.marks && tries == 0 {
            return Err(Some(Vec::new()));
        }
        let mut out = Vec::new();
        for e in &v.ev {
            if let Some(b) = self.engine.index(&e.series) {
                let first = self.engine.bars(b).first().map_or(e.from_t, |x| x.t);
                out.push((b, first, e.to_t));
            }
        }
        Err(Some(out))
    }

    /// Label when the final path finds a difference: refetch from the replica's own
    /// source (at most `MAX_TRIES` attempts, each waiting up to `WAIT`), then give up
    /// with `SourceDiffers` (predicate holds) or `Disputed` (it does not).
    fn differ(
        &mut self,
        ctx: &mut Ctx,
        id: u64,
        repair: Option<Vec<(usize, i64, i64)>>,
        holds: bool,
        what: &str,
    ) -> Label {
        let now = ctx.now();
        let cur = self
            .views
            .get(&id)
            .and_then(|v| self.display.items.get(&v.key))
            .map(|x| x.label);
        let can_repair = self.cfg.repair && repair.is_some();
        // Refetches already in flight on the claim's bindings (marks, catch-up, other
        // claims) may fix it: waiting for them does not use up an attempt.
        let bindings: Vec<usize> = self
            .views
            .get(&id)
            .map(|v| {
                v.key
                    .bindings()
                    .iter()
                    .filter_map(|x| self.engine.index(x))
                    .collect()
            })
            .unwrap_or_default();
        let busy = self
            .inflight
            .values()
            .any(|f| bindings.contains(&f.b))
            || bindings.iter().any(|&b| !self.queue[b].is_empty());
        let st = self.state.entry(id).or_default();
        let since = *st.mismatch_since.get_or_insert(now);
        let waiting = st.waiting_until.is_some_and(|t| now < t);
        if can_repair && (waiting || busy || st.tries < MAX_TRIES) {
            if !waiting {
                if !busy {
                    st.tries += 1;
                }
                st.waiting_until = Some(now + WAIT);
                st.ranges = repair.unwrap_or_default();
                let ranges = st.ranges.clone();
                ctx.set_timer(now + WAIT, TOKEN_EVAL | id);
                for (b, from, to) in ranges {
                    self.queue_fetch(b, from, to);
                }
            }
            if holds {
                if cur == Some(Label::Final) && now - since < GRACE {
                    self.eval_timers.insert((since + GRACE, id));
                    ctx.set_timer(since + GRACE, TOKEN_EVAL | id);
                    return Label::Final;
                }
                return Label::Provisional;
            }
            return Label::Pending;
        }
        if !st.reported {
            st.reported = true;
            let r = Report::DataDispute {
                id,
                what: what.to_owned(),
            };
            ctx.send(serde_json::to_vec(&r).expect("json"));
        }
        if holds {
            Label::SourceDiffers
        } else {
            Label::Disputed
        }
    }

    fn evaluate(&mut self, ctx: &mut Ctx, id: u64) {
        self.engine.add_work(1);
        let Some(v) = self.views.get(&id).cloned() else {
            return;
        };
        if v.retracted {
            self.display.remove(&v.key);
            self.state.remove(&id);
            return;
        }
        let local = self.engine.claim(&v.key).cloned();
        if local.is_none() {
            self.dwell.remove(&id);
        }
        let covered = self.covered(&v.key, false);
        let closed = covered && self.covered(&v.key, true);
        let (label, values, evidence) = match (&local, covered) {
            (_, false) => (Label::Pending, v.v.clone(), Self::view_ranges(&v)),
            (Some(c), true) => {
                let label = match v.mat {
                    Mat::Provisional => self.provisional(ctx, id, c),
                    Mat::Final if !closed => Label::Provisional,
                    Mat::Final => {
                        let tries = self.state.get(&id).map_or(0, |s| s.tries);
                        match self.verify(&v, c, tries) {
                            Ok(()) => {
                                self.state.remove(&id);
                                Label::Final
                            }
                            Err(r) => self.differ(ctx, id, r, true, "digest_or_values"),
                        }
                    }
                };
                (label, c.values.clone(), c.evidence.clone())
            }
            (None, true) => {
                let label = if v.mat == Mat::Final && closed {
                    let ranges: Vec<(usize, i64, i64)> = v
                        .ev
                        .iter()
                        .filter_map(|e| {
                            let b = self.engine.index(&e.series)?;
                            let first = self.engine.bars(b).first().map_or(e.from_t, |x| x.t);
                            let tries = self.state.get(&id).map_or(0, |s| s.tries);
                            Some(if tries == 0 || self.cfg.marks {
                                (b, e.from_t, e.to_t)
                            } else {
                                (b, first, e.to_t)
                            })
                        })
                        .collect();
                    self.differ(ctx, id, Some(ranges), false, "predicate")
                } else {
                    Label::Pending
                };
                (label, v.v.clone(), Self::view_ranges(&v))
            }
        };
        let item = DisplayItem {
            key: v.key.clone(),
            label,
            shown_values: values,
            evidence,
        };
        if self.display.items.get(&v.key) != Some(&item) {
            self.display.put(item);
        }
    }

    /// Evaluates every viewed claim of binding `b` with bar time `>= from`.
    fn evaluate_from(&mut self, ctx: &mut Ctx, b: usize, from: i64) {
        let ids: Vec<u64> = self
            .index
            .range((b, from, 0)..=(b, i64::MAX, u64::MAX))
            .map(|x| x.2)
            .collect();
        for id in ids {
            self.evaluate(ctx, id);
        }
    }

    fn after_apply(&mut self, b: usize, ap: &Applied) {
        let Some(from) = ap.changed_from_t else {
            return;
        };
        // Newly closed / changed closed bars: compare with marks.
        let bars = self.engine.bars(b);
        let last = bars.last().map_or(i64::MIN, |x| x.t);
        if from < last || ap.new_bar {
            self.recheck(b, from, last);
        }
    }

    /// Takes the engine's claim deltas and re-evaluates what they touch.
    fn settle_display(&mut self, ctx: &mut Ctx, touched: &[(usize, i64)]) {
        let deltas = self.engine.take_deltas();
        let ids: BTreeSet<u64> = deltas
            .iter()
            .filter_map(|d| self.by_key.get(&d.key).copied())
            .collect();
        for id in ids {
            self.evaluate(ctx, id);
        }
        for &(b, from) in touched {
            // Closedness and coverage can change without a claim delta.
            let bars = self.engine.bars(b);
            let from = if bars.len() >= 2 {
                from.min(bars[bars.len() - 2].t)
            } else {
                from
            };
            self.evaluate_from(ctx, b, from);
        }
    }

    fn apply_view(&mut self, s: u64, id: u64, nv: View) {
        if self.views.get(&id).is_some_and(|v| v.s >= s) {
            return;
        }
        for bid in nv.key.bindings() {
            if let Some(b) = self.engine.index(bid) {
                self.index.insert((b, nv.key.bar_t, id));
            }
        }
        for e in &nv.ev {
            self.anchors.insert(e.series.clone(), e.anchor_t);
        }
        self.by_key.insert(nv.key.clone(), id);
        // A new final envelope resets the claim's repair budget.
        if nv.mat == Mat::Final {
            self.state.remove(&id);
        }
        self.views.insert(id, nv);
    }
}

impl ReplicaAdapter for Replica {
    fn on_source(&mut self, ctx: &mut Ctx, ev: &SourceEvent) {
        let Some(b) = self.binding_of(ev.feed()) else {
            return;
        };
        let ap = match ev {
            SourceEvent::Data(d) => {
                let t = match &d.change {
                    DataChange::Bar { bar } => bar.t,
                    DataChange::Patch { t, .. } => *t,
                };
                // Catch-up filter: a live tick (in the feed's own time) for a bar this
                // replica already holds in its settled form is older than what it shows.
                if self.cfg.catch_up && t <= self.floor[b] && t >= self.head[b] {
                    self.head[b] = self.head[b].max(t);
                    self.engine.add_work(1);
                    return;
                }
                if t < self.head[b] {
                    self.corr[b].insert(t, d.seq);
                }
                self.head[b] = self.head[b].max(t);
                self.engine.apply(ev)
            }
            SourceEvent::Snapshot { bars, .. } => {
                self.head[b] = bars.last().map_or(self.head[b], |x| x.t);
                let ap = self.engine.apply(ev);
                self.align(ctx, b);
                ap
            }
        };
        self.after_apply(b, &ap);
        let touched: Vec<(usize, i64)> = ap.changed_from_t.map(|f| (b, f)).into_iter().collect();
        self.settle_display(ctx, &touched);
        if self.check_bars.iter().any(|x| !x.is_empty())
            || self.check_chunks.iter().any(|x| !x.is_empty())
        {
            self.schedule_check(ctx);
        }
        self.pump(ctx);
    }

    fn on_authority_msg(&mut self, ctx: &mut Ctx, msg: &[u8]) {
        let Ok(batch) = serde_json::from_slice::<Batch>(msg) else {
            return;
        };
        self.engine.add_work(batch.ops.len() as u64);
        let s = batch.s;
        let mut touched = BTreeSet::new();
        let mut align = BTreeSet::new();
        for op in batch.ops {
            match op {
                Op::Hello { anchors, .. } => {
                    for (series, a) in anchors {
                        if let Some(b) = self.engine.index(&series) {
                            align.insert(b);
                        }
                        self.anchors.insert(series, a);
                    }
                }
                Op::Fact { id, key, m, v, ev } => {
                    self.apply_view(
                        s,
                        id,
                        View {
                            s,
                            key,
                            mat: m,
                            v,
                            ev,
                            retracted: false,
                        },
                    );
                    touched.insert(id);
                }
                Op::FactFinal { id, key, v, ev } => {
                    self.apply_view(
                        s,
                        id,
                        View {
                            s,
                            key,
                            mat: Mat::Final,
                            v,
                            ev,
                            retracted: false,
                        },
                    );
                    touched.insert(id);
                }
                Op::FactRetracted { id, .. } => {
                    match self.views.get_mut(&id) {
                        Some(v) if v.s < s => {
                            v.s = s;
                            v.retracted = true;
                        }
                        Some(_) => {}
                        None => {
                            // Retraction before the fact (reordering): a tombstone.
                            self.views.insert(
                                id,
                                View {
                                    s,
                                    key: ClaimKey {
                                        detector: String::new(),
                                        params: String::new(),
                                        binding: String::new(),
                                        bar_t: i64::MIN,
                                        direction: Direction::Up,
                                    },
                                    mat: Mat::Provisional,
                                    v: BTreeMap::new(),
                                    ev: Vec::new(),
                                    retracted: true,
                                },
                            );
                        }
                    }
                    touched.insert(id);
                }
                Op::DataMark {
                    series,
                    bars,
                    chunks,
                    ..
                } => {
                    let Some(b) = self.engine.index(&series) else {
                        continue;
                    };
                    let mut tmax = i64::MIN;
                    for (t, d) in bars {
                        let d = u64::from_str_radix(&d, 16).unwrap_or(0);
                        let e = self.mark_bar[b].entry(t).or_insert((0, d));
                        if s >= e.0 {
                            if e.1 != d {
                                self.bar_tries.remove(&(b, t));
                            }
                            *e = (s, d);
                            self.check_bars[b].insert(t);
                        }
                        tmax = tmax.max(t);
                    }
                    for (c, d) in chunks {
                        let d = u64::from_str_radix(&d, 16).unwrap_or(0);
                        let e = self.mark_chunk[b].entry(c).or_insert((0, d));
                        if s >= e.0 {
                            if e.1 != d {
                                self.bar_tries.remove(&(b, chunk_span(c, self.interval_ms).0));
                            }
                            *e = (s, d);
                            self.check_chunks[b].insert(c);
                        }
                    }
                    for (c, _) in self.mark_chunk[b].iter().filter(|(_, x)| x.0 == s) {
                        tmax = tmax.max(chunk_span(*c, self.interval_ms).1);
                    }
                    // Catch-up: the authority has settled a bar my feed has not closed yet.
                    if self.cfg.catch_up && tmax > i64::MIN {
                        let bars = self.engine.bars(b);
                        if bars.is_empty() {
                            // My feed has not even resolved the binding: start from my own
                            // settled source at the authority's anchor.
                            let from = self
                                .anchors
                                .get(&series)
                                .copied()
                                .or_else(|| self.mark_bar[b].keys().next().copied())
                                .unwrap_or(tmax);
                            self.queue_fetch(b, from, tmax);
                        } else if !is_closed_in(bars, tmax) {
                            let last_closed = if bars.len() >= 2 {
                                bars[bars.len() - 2].t
                            } else {
                                bars[0].t
                            };
                            let from = last_closed.max(self.floor[b]).saturating_add(1);
                            self.queue_fetch(b, from, tmax);
                        }
                    }
                }
            }
        }
        for b in align {
            self.align(ctx, b);
        }
        for id in touched {
            self.evaluate(ctx, id);
        }
        // Envelopes carry anchors too.
        for b in 0..self.feeds.len() {
            if !self.aligning[b] {
                self.align(ctx, b);
            }
        }
        self.schedule_check(ctx);
        self.pump(ctx);
    }

    fn on_timer(&mut self, ctx: &mut Ctx, token: u64) {
        if token == TOKEN_CHECK {
            self.check_set = false;
            self.run_checks();
        } else if token & TOKEN_EVAL != 0 {
            let id = token & !TOKEN_EVAL;
            let now = ctx.now();
            self.eval_timers.retain(|&(t, _)| t > now);
            self.evaluate(ctx, id);
        }
        self.pump(ctx);
    }

    fn on_fetch(&mut self, ctx: &mut Ctx, reply: &FetchReply) {
        let Some(f) = self.inflight.remove(&reply.id) else {
            return;
        };
        let b = f.b;
        let ap = match f.kind {
            FetchKind::Anchor => {
                let first = self.engine.bars(b).first().map_or(i64::MAX, |x| x.t);
                let mut bars: Vec<Bar> = reply
                    .bars
                    .iter()
                    .filter(|x| x.t < first)
                    .cloned()
                    .collect();
                bars.extend_from_slice(self.engine.bars(b));
                let ev = SourceEvent::Snapshot {
                    feed: self.feeds[b].clone(),
                    seq: self.engine.last_seq(b),
                    bars,
                };
                self.aligning[b] = false;
                self.engine.apply(&ev)
            }
            FetchKind::Repair => {
                if let Some(last) = reply.bars.last() {
                    self.floor[b] = self.floor[b].max(last.t);
                }
                // A correction my feed delivered after the request is newer than the
                // answer: keep it.
                let corr = &self.corr[b];
                let bars: Vec<Bar> = reply
                    .bars
                    .iter()
                    .filter(|x| corr.get(&x.t).is_none_or(|s| *s <= f.req_seq))
                    .cloned()
                    .collect();
                self.engine.apply_settled(b, &bars)
            }
        };
        let hi = reply.bars.last().map_or(f.from, |x| x.t);
        self.recheck(b, f.from, hi);
        self.after_apply(b, &ap);
        // Claims waiting on a repair of this binding: evaluate again.
        let bid = self.engine.ids()[b].clone();
        let waiting: Vec<u64> = self
            .state
            .iter()
            .filter(|(id, st)| {
                st.waiting_until.is_some()
                    && self
                        .views
                        .get(id)
                        .is_some_and(|v| v.key.bindings().contains(&bid.as_str()))
                    && (st.ranges.is_empty()
                        || st
                            .ranges
                            .iter()
                            .any(|&(rb, a, z)| rb == b && a <= f.to && f.from <= z))
            })
            .map(|(id, _)| *id)
            .collect();
        for id in &waiting {
            if let Some(st) = self.state.get_mut(id) {
                st.waiting_until = None;
            }
        }
        let from = ap.changed_from_t.unwrap_or(f.from).min(f.from);
        self.settle_display(ctx, &[(b, from)]);
        for id in waiting {
            self.evaluate(ctx, id);
        }
        self.schedule_check(ctx);
        self.pump(ctx);
    }

    fn display(&self) -> &DisplayState {
        &self.display
    }

    fn visible_data(&self, binding: usize) -> &[Bar] {
        self.engine.bars(binding)
    }

    fn work_units(&self) -> u64 {
        self.engine.work()
    }
}
