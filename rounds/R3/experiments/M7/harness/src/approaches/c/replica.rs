//! The replica side of approach C. See the module documentation of `approaches::c`.

use std::collections::{BTreeMap, BTreeSet};

use qp_schema::data::DataChange;
use qp_schema::Bar;

use super::{apply_settled_bulk, parse_key, AuthMsg, Divergence, ReportMsg};
use crate::adapter::{Ctx, FetchReply, ReplicaAdapter, Setup, SourceEvent};
use crate::detector::{Claim, ClaimKey, Direction, EvidenceRange, Maturity};
use crate::display::{DisplayItem, DisplayState, Label};
use crate::engine::Engine;
use crate::spec::VTime;

/// Whether claims on the replica's live, unsettled bar are shown as provisional. `true`
/// is C's latency promise; `false` (variant "C-final", measured in T7c.md) shows only
/// confirmed claims and claims on settled bars.
const LIVE_PROVISIONAL: bool = true;
/// Disagreement with the authority must last this long before a shown claim is
/// relabelled (absorbs the different arrival times of the same correction on the two
/// feeds and on the channel).
const GRACE: VTime = 2_000_000;
/// Housekeeping period (feed-silence check).
const HOUSEKEEP: VTime = 1_000_000;
/// Minimum feed silence before the live bar is treated as frozen.
const SILENCE_MIN: VTime = 5_000_000;
/// Silence threshold in units of the mean inter-event gap.
const SILENCE_GAPS: f64 = 8.0;
/// While settle timing is loosely known, polls probe at this pace (one request in
/// flight per binding, so the effective pace is this plus the source's reply delay).
const POLL_PROBE: VTime = 250_000;
/// Settle timing counts as loosely known while the bracket is wider than this.
const POLL_LOOSE: i64 = 5_000_000;
/// Live-bar events a binding must have seen before a new provisional claim is shown on
/// its unsettled data (the margin estimate needs samples).
const MIN_JUMP_SAMPLES: u32 = 8;
/// Extra RSI margin for provisional claims on a feed whose prices are known to differ
/// from the provider's (audit mismatch: f32, glitches).
const INEXACT_MARGIN: f64 = 0.05;
/// Bisection stops when the settle-time estimate is this tight.
const POLL_TIGHT: VTime = 20_000;
/// Feed latency the provisional margin is sized for (µs).
const LATENCY_BUDGET: VTime = 250_000;
/// Weight of a new sample in the gap and jump averages.
const EWMA_A: f64 = 0.2;
/// Safety factor of the provisional margin.
const MARGIN_K: f64 = 1.5;
/// A gap larger than this is tracked as one taint that only settled bars resolve.
const MAX_TRACKED_GAP: u64 = 4096;

const TK_HOUSEKEEP: u64 = 1 << 56;
const TK_POLL: u64 = 2 << 56;
const TK_GRACE: u64 = 3 << 56;
const TK_MASK: u64 = 0xff << 56;

/// Whether this replica's feed reproduces the provider's settled bars bit for bit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Exactness {
    /// Not audited yet.
    Unknown,
    /// Audited: closed bars without missing events equal the settled bars.
    Exact,
    /// A complete closed bar differed from the settled bar (f32 prices, glitches):
    /// only settled bars are trusted.
    Inexact,
}

/// When a bar becomes fetchable from the settled source, learned from replies.
/// Normalised as `request_time_us - (bar_t + interval) * 1000`: the source makes bars
/// available a fixed time after their close, so the normalised value is the same for
/// every bar and binding.
#[derive(Debug, Default)]
struct Tracker {
    /// Largest normalised request time that missed.
    lo: Option<i64>,
    /// Smallest normalised request time that hit.
    hi: Option<i64>,
}

impl Tracker {
    fn norm(q: VTime, t: i64, iv: i64) -> i64 {
        q - (t + iv) * 1000
    }

    fn hit(&mut self, q: VTime, t: i64, iv: i64) {
        let n = Self::norm(q, t, iv);
        self.hi = Some(self.hi.map_or(n, |h| h.min(n)));
        if self.lo.is_some_and(|l| l >= n) {
            self.lo = None;
        }
    }

    fn miss(&mut self, q: VTime, t: i64, iv: i64) {
        let n = Self::norm(q, t, iv);
        self.lo = Some(self.lo.map_or(n, |l| l.max(n)));
        if self.hi.is_some_and(|h| h <= n) {
            self.hi = None;
        }
    }

    /// When to ask for bar `t`.
    fn when(&self, t: i64, iv: i64) -> Option<VTime> {
        let base = (t + iv) * 1000;
        match (self.lo, self.hi) {
            (Some(l), Some(h)) if h - l > POLL_LOOSE => None,
            (Some(l), Some(h)) if h - l > POLL_TIGHT => Some(base + l + (h - l) / 2),
            (_, Some(h)) => Some(base + h),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReqKind {
    Poll,
    Backfill,
}

#[derive(Debug, Clone, Copy)]
struct Req {
    b: usize,
    kind: ReqKind,
    q: VTime,
    from_t: i64,
    to_t: i64,
    frontier: Option<i64>,
    epoch: u32,
}

#[derive(Debug)]
struct Bind {
    iv: i64,
    exact: Exactness,
    ready: bool,
    anchor: Option<i64>,
    backfill_pending: bool,
    /// Latest bar the feed itself has opened.
    frontier: Option<i64>,
    last_seq: u64,
    /// Missing `seq` → bar range `(lo, hi)` the missing event may belong to.
    missing: BTreeMap<u64, (i64, i64)>,
    /// Gaps too large to track per `seq`.
    big_gaps: Vec<(i64, i64)>,
    /// Settled frontier: every bar from the first held bar up to here was settled or
    /// verified against a settled bar.
    set_t: Option<i64>,
    /// Confirmed frontier (cached).
    conf_t: Option<i64>,
    /// The feed opens bars only after the settled source already has the previous
    /// bar: its live view is older than the settled data.
    behind: bool,
    /// No feed event for longer than the silence threshold.
    silent: bool,
    last_feed_at: Option<VTime>,
    gap_ewma: f64,
    /// Mean absolute RSI change per live-bar event.
    jump_ewma: f64,
    jump_n: u32,
    /// Time of the last data event (gap estimate).
    last_data_at: Option<VTime>,
    /// RSI of the live bar after the last applied event.
    last_rsi: Option<(i64, f64)>,
    /// Last time a feed event changed a bar (bar t → time).
    touched: BTreeMap<i64, VTime>,
    poll_inflight: bool,
    poll_at: Option<VTime>,
    started: bool,
    /// Data epoch: bumped when a snapshot replaces the data wholesale (split); replies
    /// requested in an older epoch are dropped.
    epoch: u32,
}

impl Bind {
    fn taint_floor(&self) -> Option<i64> {
        self.missing
            .values()
            .chain(self.big_gaps.iter())
            .map(|&(lo, _)| lo)
            .min()
    }

    /// RSI margin a new provisional claim on unsettled live data needs: the move the
    /// live RSI can make within the latency budget if it trends,
    /// `MARGIN_K × jump × LATENCY_BUDGET / mean gap`.
    fn live_margin(&self) -> f64 {
        if self.jump_n < MIN_JUMP_SAMPLES {
            return f64::INFINITY;
        }
        let floor = if self.exact == Exactness::Inexact {
            INEXACT_MARGIN
        } else {
            0.0
        };
        #[allow(clippy::cast_precision_loss)]
        let n = if self.gap_ewma > 0.0 {
            LATENCY_BUDGET as f64 / self.gap_ewma
        } else {
            0.0
        };
        floor + MARGIN_K * self.jump_ewma * n
    }

    fn fresh(&self, t: i64) -> bool {
        self.set_t.is_some_and(|s| t <= s) || (!self.behind && !self.silent)
    }
}

/// Distance of a claim's evidence value beyond its level, in its direction.
fn margin(c: &Claim) -> f64 {
    let v = |k: &str| c.values.get(k).copied().unwrap_or(f64::NAN);
    let level = v("level");
    let d = if c.values.contains_key("a") {
        v("a").min(v("b")) - level
    } else {
        match c.key.direction {
            Direction::Up => v("value") - level,
            Direction::Down => level - v("value"),
        }
    };
    if d.is_nan() {
        0.0
    } else {
        d
    }
}

fn close_enough(a: &Bar, b: &Bar) -> bool {
    let near = |x: f64, y: f64| (x - y).abs() <= 1e-6 * y.abs().max(1.0);
    a.t == b.t && near(a.o, b.o) && near(a.h, b.h) && near(a.l, b.l) && near(a.c, b.c)
}

/// The replica.
pub(crate) struct Replica {
    setup: Setup,
    engine: Engine,
    period: i64,
    binds: Vec<Bind>,
    display: DisplayState,
    /// Keys of interest (engine claims, authority finals, shown) by (binding, bar t).
    index: BTreeMap<(usize, i64), BTreeSet<ClaimKey>>,
    tracker: Tracker,
    auth_next: u64,
    auth_buf: BTreeMap<u64, AuthMsg>,
    auth_finals: BTreeSet<ClaimKey>,
    auth_w: Vec<Option<i64>>,
    disagree_since: BTreeMap<ClaimKey, VTime>,
    reported: BTreeMap<ClaimKey, (bool, bool)>,
    out: Vec<Divergence>,
    timers: Vec<(VTime, u64)>,
    reqs: BTreeMap<u64, Req>,
    housekeeping: bool,
    /// Show provisional claims on the live (unsettled) bar ([`LIVE_PROVISIONAL`]).
    live_provisional: bool,
}

impl Replica {
    pub(crate) fn new(setup: &Setup) -> Self {
        let binds = setup
            .bindings
            .iter()
            .map(|bs| Bind {
                iv: bs.interval_ms,
                exact: Exactness::Unknown,
                ready: false,
                anchor: None,
                backfill_pending: false,
                frontier: None,
                last_seq: 0,
                missing: BTreeMap::new(),
                big_gaps: Vec::new(),
                set_t: None,
                conf_t: None,
                behind: false,
                silent: false,
                last_feed_at: None,
                gap_ewma: 0.0,
                jump_ewma: 0.0,
                jump_n: 0,
                last_data_at: None,
                last_rsi: None,
                touched: BTreeMap::new(),
                poll_inflight: false,
                poll_at: None,
                started: false,
                epoch: 0,
            })
            .collect();
        Self {
            setup: setup.clone(),
            engine: Engine::new(setup),
            period: i64::from(setup.detector.spec.period),
            binds,
            display: DisplayState::default(),
            index: BTreeMap::new(),
            tracker: Tracker::default(),
            auth_next: 1,
            auth_buf: BTreeMap::new(),
            auth_finals: BTreeSet::new(),
            auth_w: vec![None; setup.bindings.len()],
            disagree_since: BTreeMap::new(),
            reported: BTreeMap::new(),
            out: Vec::new(),
            timers: Vec::new(),
            reqs: BTreeMap::new(),
            housekeeping: false,
            live_provisional: LIVE_PROVISIONAL,
        }
    }

    // ----- keys, index, labels -------------------------------------------------------

    fn key_bindings(&self, k: &ClaimKey) -> Option<Vec<usize>> {
        k.bindings().iter().map(|id| self.engine.index(id)).collect()
    }

    fn index_key(&mut self, k: &ClaimKey) {
        if let Some(bs) = self.key_bindings(k) {
            for b in bs {
                self.index.entry((b, k.bar_t)).or_default().insert(k.clone());
            }
        }
    }

    /// Relabels every key of interest on binding `b` with `lo < bar_t <= hi`.
    fn relabel_range(&mut self, now: VTime, b: usize, lo: Option<i64>, hi: Option<i64>) {
        let lo = lo.unwrap_or(i64::MIN);
        let hi = hi.unwrap_or(i64::MAX);
        if lo >= hi {
            return;
        }
        let keys: Vec<ClaimKey> = self
            .index
            .range((b, lo.saturating_add(1))..=(b, hi))
            .flat_map(|(_, ks)| ks.iter().cloned())
            .collect();
        for k in keys {
            self.relabel(now, &k);
        }
    }

    fn evidence_of(&self, k: &ClaimKey, bs: &[usize]) -> Vec<EvidenceRange> {
        bs.iter()
            .map(|&b| EvidenceRange {
                binding: self.setup.bindings[b].id.clone(),
                from_t: k.bar_t - self.period * self.binds[b].iv,
                to_t: k.bar_t,
            })
            .collect()
    }

    /// Recomputes the label of one key from the replica's own state; the authority's
    /// stream only turns an agreed `Final` into `SourceDiffers` / adds `Disputed`.
    fn relabel(&mut self, now: VTime, k: &ClaimKey) {
        self.engine.add_work(1);
        let Some(bs) = self.key_bindings(k) else {
            self.display.remove(k);
            return;
        };
        let current = self.display.items.get(k).map(|x| x.label);
        if !bs.iter().all(|&b| self.binds[b].ready) {
            self.display.remove(k);
            return;
        }
        let sealed = bs
            .iter()
            .all(|&b| self.binds[b].conf_t.is_some_and(|c| k.bar_t <= c));
        // A provisional claim rests on the closed bars before it too (RSI is a running
        // value): those must be confirmed, and the live bar must come from a fresh feed.
        let fresh = bs.iter().all(|&b| {
            let s = &self.binds[b];
            s.fresh(k.bar_t) && s.conf_t.is_some_and(|c| c >= k.bar_t - s.iv)
        });
        let comparable = bs
            .iter()
            .all(|&b| self.auth_w[b].is_some_and(|w| k.bar_t <= w));
        let auth_has = self.auth_finals.contains(k);
        let claim = self.engine.claim(k).cloned();
        let local_final = claim
            .as_ref()
            .is_some_and(|c| c.maturity == Maturity::Final);
        // Disagreement after the fact (both sides past the bar, the replica confirmed).
        let disagree = comparable && sealed && local_final != auth_has && (local_final || claim.is_none());
        let settled_disagreement = if disagree {
            let since = *self.disagree_since.entry(k.clone()).or_insert(now);
            if since + GRACE > now {
                self.timers.push((since + GRACE, TK_GRACE));
                false
            } else {
                true
            }
        } else {
            self.disagree_since.remove(k);
            false
        };
        let label = match &claim {
            Some(c) if c.maturity == Maturity::Final && sealed => Some(if settled_disagreement {
                Label::SourceDiffers
            } else {
                Label::Final
            }),
            Some(_) if !fresh => None,
            // First provisional show only with a margin over the level that the live
            // RSI is unlikely to give back within the feed-latency budget (the truth is
            // ahead of the replica's feed); once shown it stays while the claim holds.
            // On settled bars (the engine's last bar while the feed lags or is down) the
            // claim is true data, merely not closed on screen yet.
            Some(c) if c.maturity == Maturity::Provisional => {
                let on_settled = bs
                    .iter()
                    .all(|&b| self.binds[b].set_t.is_some_and(|s| k.bar_t <= s));
                let live_ok = self.live_provisional
                    && (current.is_some_and(Label::asserts)
                        || margin(c)
                            >= bs
                                .iter()
                                .map(|&b| self.binds[b].live_margin())
                                .fold(0.0, f64::max));
                (on_settled || live_ok).then_some(Label::Provisional)
            }
            // Closed but not confirmed: keep a claim already asserted as provisional
            // ("not confirmed yet"); a new one waits for confirmation.
            Some(_) => current
                .is_some_and(Label::asserts)
                .then_some(Label::Provisional),
            None if settled_disagreement => Some(Label::Disputed),
            None => None,
        };
        if matches!(label, Some(Label::SourceDiffers | Label::Disputed)) {
            let pair = (local_final, auth_has);
            if self.reported.get(k) != Some(&pair) {
                self.reported.insert(k.clone(), pair);
                self.out.push(Divergence {
                    claim: k.text(),
                    local: local_final,
                    authority: auth_has,
                    sealed_t: self.binds[bs[0]].conf_t.unwrap_or(i64::MIN),
                });
            }
        }
        match label {
            None => {
                self.display.remove(k);
            }
            Some(label) => {
                let (shown_values, evidence) = match claim {
                    Some(c) => (c.values, c.evidence),
                    None => (BTreeMap::new(), self.evidence_of(k, &bs)),
                };
                let item = DisplayItem {
                    key: k.clone(),
                    label,
                    shown_values,
                    evidence,
                };
                if self.display.items.get(k) != Some(&item) {
                    self.display.put(item);
                }
            }
        }
    }

    // ----- confirmation ---------------------------------------------------------------

    fn compute_conf(&self, b: usize) -> Option<i64> {
        let s = &self.binds[b];
        if !s.ready {
            return None;
        }
        let bars = self.engine.bars(b);
        if bars.len() < 2 {
            return None;
        }
        let last_closed = bars[bars.len() - 2].t;
        let fast = (s.exact == Exactness::Exact).then(|| match s.taint_floor() {
            None => last_closed,
            Some(f) => last_closed.min(f - s.iv),
        });
        s.set_t.max(fast).map(|c| c.min(last_closed))
    }

    /// Updates the confirmed and settled frontiers of `b` (after any change of its data
    /// or bookkeeping) and relabels what moved.
    fn refresh_binding(&mut self, now: VTime, b: usize, old_set: Option<i64>) {
        let new = self.compute_conf(b);
        let old = self.binds[b].conf_t;
        self.binds[b].conf_t = new;
        let new_set = self.binds[b].set_t;
        let lo = [old, new, old_set, new_set].into_iter().flatten().min();
        let hi = [old, new, old_set, new_set].into_iter().flatten().max();
        if old != new || old_set != new_set {
            if let (Some(lo), Some(hi)) = (lo, hi) {
                // Every key whose label may depend on the moved frontiers.
                let from = if old.is_none() || new.is_none() || old_set.is_none() || new_set.is_none() {
                    None
                } else {
                    Some(lo)
                };
                self.relabel_range(now, b, from, Some(hi));
            }
        }
    }

    fn set_flag(&mut self, now: VTime, b: usize, f: impl FnOnce(&mut Bind) -> bool) {
        if f(&mut self.binds[b]) {
            let s = self.binds[b].set_t;
            self.relabel_range(now, b, s, None);
        }
    }

    fn maybe_ready(&mut self, now: VTime, b: usize) {
        let s = &self.binds[b];
        // Exactness need not be known: until it is, only settled bars are confirmed.
        let ready = s.anchor.is_some() && !s.backfill_pending;
        if ready && !s.ready {
            self.binds[b].ready = true;
            self.binds[b].conf_t = self.compute_conf(b);
            self.relabel_range(now, b, None, None);
        }
    }

    // ----- engine -----------------------------------------------------------------------

    fn after_engine(&mut self, now: VTime) {
        let deltas = self.engine.take_deltas();
        for d in &deltas {
            self.index_key(&d.key);
        }
        for d in deltas {
            self.relabel(now, &d.key);
        }
    }

    fn apply_snapshot_bars(&mut self, b: usize, seq: u64, bars: Vec<Bar>) {
        self.engine.apply(&SourceEvent::Snapshot {
            feed: self.setup.bindings[b].feed.clone(),
            seq,
            bars,
        });
    }

    /// Merges a feed snapshot (REFRESH, split) with what the replica holds: the backfill
    /// before the feed's window is kept if the data epoch is the same; settled bars
    /// newer than the snapshot's live bar are kept; an inexact feed's values do not
    /// replace settled bars. A different epoch (split) replaces everything.
    fn merge_snapshot(&mut self, b: usize, snap: &[Bar]) -> (Vec<Bar>, bool) {
        let cur = self.engine.bars(b);
        let s = &self.binds[b];
        let (Some(first), Some(last)) = (snap.first(), snap.last()) else {
            return (snap.to_vec(), true);
        };
        let common = cur.first().map_or(first.t, |c| c.t.max(first.t));
        let same = match (
            cur.binary_search_by_key(&common, |x| x.t),
            snap.binary_search_by_key(&common, |x| x.t),
        ) {
            (Ok(i), Ok(j)) => close_enough(&cur[i], &snap[j]),
            _ => true,
        };
        let anchor = s.anchor.unwrap_or(i64::MIN);
        if !same {
            return (snap.iter().filter(|x| x.t >= anchor).cloned().collect(), false);
        }
        let set_t = s.set_t.unwrap_or(i64::MIN);
        let get = |t: i64| cur.binary_search_by_key(&t, |x| x.t).ok().map(|i| &cur[i]);
        let mut out: Vec<Bar> = cur
            .iter()
            .filter(|x| x.t >= anchor && x.t < first.t)
            .cloned()
            .collect();
        for x in snap.iter().filter(|x| x.t >= anchor) {
            let prefer_settled = x.t <= set_t && (s.exact != Exactness::Exact || x.t == last.t);
            match get(x.t) {
                Some(c) if prefer_settled => out.push(c.clone()),
                _ => out.push(x.clone()),
            }
        }
        out.extend(cur.iter().filter(|x| x.t > last.t && x.t <= set_t).cloned());
        (out, true)
    }

    // ----- settled polling ------------------------------------------------------------

    fn schedule_poll(&mut self, now: VTime, b: usize) {
        let s = &self.binds[b];
        let target = match s.set_t {
            Some(t) => t + s.iv,
            None => self.engine.bars(b).first().map_or(0, |x| x.t),
        };
        let at = self
            .tracker
            .when(target, s.iv)
            .unwrap_or(now + POLL_PROBE)
            .max(now);
        if self.binds[b].poll_at.is_some_and(|p| p <= at && p >= now) {
            return;
        }
        self.binds[b].poll_at = Some(at);
        self.timers.push((at, TK_POLL | b as u64));
    }

    fn poll(&mut self, ctx: &mut Ctx, b: usize) {
        let s = &self.binds[b];
        if s.poll_inflight {
            return;
        }
        // Before the feed's first snapshot, bootstrap from the authority's anchor.
        let Some(first) = self.engine.bars(b).first().map(|x| x.t).or(s.anchor) else {
            return;
        };
        let from = s.set_t.map_or(first, |t| t + s.iv);
        let id = ctx.fetch_settled(b, from, i64::MAX);
        self.reqs.insert(
            id,
            Req {
                b,
                kind: ReqKind::Poll,
                q: ctx.now(),
                from_t: from,
                to_t: i64::MAX,
                frontier: s.frontier,
                epoch: s.epoch,
            },
        );
        self.binds[b].poll_inflight = true;
        self.binds[b].poll_at = None;
    }

    fn on_poll_reply(&mut self, now: VTime, req: Req, bars: &[Bar]) {
        let b = req.b;
        let iv = self.binds[b].iv;
        self.binds[b].poll_inflight = false;
        let anchor = self.binds[b].anchor.unwrap_or(i64::MIN);
        let floor = self.binds[b].taint_floor();
        let old_set = self.binds[b].set_t;
        let mut set_t = old_set;
        let mut contiguous = true;
        let mut diffs: Vec<Bar> = Vec::new();
        // Audit: compare only complete feed bars (closed at the request, no missing event).
        let mut clean_compared = 0usize;
        let mut mismatch_clean = false;
        for bar in bars.iter().filter(|x| x.t >= anchor) {
            let touched_after = self.binds[b]
                .touched
                .get(&bar.t)
                .is_some_and(|&at| at > req.q);
            if touched_after {
                // The feed changed this bar after the request (a correction): its data
                // is newer than the reply.
                contiguous = false;
                continue;
            }
            let cur = self.engine.bars(b);
            let clean = old_set.is_none_or(|s| bar.t > s)
                && floor.is_none_or(|f| bar.t < f)
                && req.frontier.is_some_and(|f| bar.t < f);
            match cur.binary_search_by_key(&bar.t, |x| x.t) {
                Ok(i) => {
                    clean_compared += usize::from(clean);
                    if cur[i] != *bar {
                        mismatch_clean |= clean;
                        diffs.push(bar.clone());
                    }
                }
                Err(_) => diffs.push(bar.clone()),
            }
            if contiguous {
                if set_t.is_none_or(|s| bar.t <= s + iv) {
                    set_t = Some(set_t.map_or(bar.t, |s| s.max(bar.t)));
                } else {
                    contiguous = false;
                }
            }
        }
        match self.binds[b].exact {
            Exactness::Unknown | Exactness::Exact if mismatch_clean => {
                self.binds[b].exact = Exactness::Inexact;
            }
            Exactness::Unknown if clean_compared > 0 => self.binds[b].exact = Exactness::Exact,
            _ => {}
        }
        if !diffs.is_empty() {
            apply_settled_bulk(&mut self.engine, &self.setup, b, &diffs);
        }
        self.binds[b].set_t = set_t;
        if let Some(s) = set_t {
            let st = &mut self.binds[b];
            st.missing.retain(|_, &mut (_, hi)| hi > s);
            st.big_gaps.retain(|&(_, hi)| hi > s);
        }
        if req.to_t == i64::MAX {
            match bars.last() {
                Some(last) => {
                    self.tracker.hit(req.q, last.t, iv);
                    self.tracker.miss(req.q, last.t + iv, iv);
                }
                None => self.tracker.miss(req.q, req.from_t, iv),
            }
        }
        self.after_engine(now);
        self.refresh_binding(now, b, old_set);
        self.maybe_ready(now, b);
        self.schedule_poll(now, b);
    }

    fn on_backfill_reply(&mut self, now: VTime, req: Req, bars: &[Bar]) {
        let b = req.b;
        self.binds[b].backfill_pending = false;
        let cur = self.engine.bars(b);
        let first = cur.first().map_or(i64::MAX, |x| x.t);
        let mut merged: Vec<Bar> = bars.iter().filter(|x| x.t < first).cloned().collect();
        if !merged.is_empty() {
            merged.extend(cur.iter().cloned());
            let seq = self.engine.last_seq(b);
            self.apply_snapshot_bars(b, seq, merged);
        }
        self.after_engine(now);
        let old_set = self.binds[b].set_t;
        self.refresh_binding(now, b, old_set);
        self.maybe_ready(now, b);
    }

    fn request_backfill(&mut self, ctx: &mut Ctx, b: usize) {
        let (Some(anchor), Some(first)) = (
            self.binds[b].anchor,
            self.engine.bars(b).first().map(|x| x.t),
        ) else {
            return;
        };
        if anchor >= first || self.binds[b].backfill_pending {
            return;
        }
        let to = first - self.binds[b].iv;
        let id = ctx.fetch_settled(b, anchor, to);
        self.reqs.insert(
            id,
            Req {
                b,
                kind: ReqKind::Backfill,
                q: ctx.now(),
                from_t: anchor,
                to_t: to,
                frontier: self.binds[b].frontier,
                epoch: self.binds[b].epoch,
            },
        );
        self.binds[b].backfill_pending = true;
    }

    fn apply_anchor(&mut self, ctx: &mut Ctx, b: usize) {
        let Some(anchor) = self.binds[b].anchor else {
            return;
        };
        let bars = self.engine.bars(b);
        let Some(first) = bars.first().map(|x| x.t) else {
            return;
        };
        if anchor > first {
            let kept: Vec<Bar> = bars.iter().filter(|x| x.t >= anchor).cloned().collect();
            let seq = self.engine.last_seq(b);
            self.apply_snapshot_bars(b, seq, kept);
        } else if anchor < first {
            self.request_backfill(ctx, b);
        }
    }

    // ----- feed -------------------------------------------------------------------------

    fn on_feed_data(&mut self, ctx: &mut Ctx, b: usize, ev: &SourceEvent, seq: u64, change: &DataChange) {
        let now = ctx.now();
        let (t, is_bar) = match change {
            DataChange::Bar { bar } => (bar.t, true),
            DataChange::Patch { t, .. } => (*t, false),
        };
        let st = &mut self.binds[b];
        let frontier_before = st.frontier;
        let is_new_bar = frontier_before.is_none_or(|f| t > f);
        // Sequence bookkeeping (mirrors the engine's SeqLww view, plus which bars a
        // missing event may belong to).
        if seq <= st.last_seq {
            st.missing.remove(&seq);
        } else {
            if seq > st.last_seq + 1 {
                let lo = frontier_before.unwrap_or(t).min(t);
                let hi = if is_bar && is_new_bar { t - st.iv } else { t }.max(lo);
                let n = seq - st.last_seq - 1;
                if n > MAX_TRACKED_GAP {
                    st.big_gaps.push((lo, hi));
                } else {
                    for m in (st.last_seq + 1)..seq {
                        st.missing.insert(m, (lo, hi));
                    }
                }
            }
            st.last_seq = seq;
        }
        // Behind: the feed opens a bar only after the settled source already gave the
        // previous one.
        let mut behind_changed = false;
        if is_new_bar {
            if let Some(p) = frontier_before {
                let behind = st.set_t.is_some_and(|s| s >= p);
                behind_changed = behind != st.behind;
                st.behind = behind;
            }
            st.frontier = Some(t);
        }
        // A feed event for a bar the settled source already gave, at the feed's own
        // frontier, is a stale replay (the feed lags): skip it. Older bars (behind the
        // frontier) are corrections and apply. Newer bars apply (a lagging feed's live
        // bar closes the settled bar on screen; its provisional claims stay hidden).
        let skip = st.set_t.is_some_and(|s| t <= s) && frontier_before.is_some_and(|f| t >= f);
        if !skip {
            self.engine.apply(ev);
            self.binds[b].touched.insert(t, now);
            let rsi = self.engine.series(b).rsi();
            let last_t = self.engine.bars(b).last().map(|x| x.t);
            if let (Some(Some(r)), Some(lt)) = (rsi.last().copied(), last_t) {
                let st = &mut self.binds[b];
                if let Some((pt, pr)) = st.last_rsi {
                    if pt == lt && lt == t {
                        let d = (r - pr).abs();
                        st.jump_ewma = if st.jump_n == 0 {
                            d
                        } else {
                            (1.0 - EWMA_A) * st.jump_ewma + EWMA_A * d
                        };
                        st.jump_n = st.jump_n.saturating_add(1);
                    }
                }
                st.last_rsi = Some((lt, r));
            }
        }
        if behind_changed {
            let s = self.binds[b].set_t;
            self.relabel_range(now, b, s, None);
        }
        self.after_engine(now);
        let old_set = self.binds[b].set_t;
        self.refresh_binding(now, b, old_set);
    }

    fn on_feed_snapshot(&mut self, ctx: &mut Ctx, b: usize, seq: u64, bars: &[Bar]) {
        let now = ctx.now();
        let first_time = !self.binds[b].started;
        self.binds[b].started = true;
        let old_set = self.binds[b].set_t;
        if self.engine.bars(b).is_empty() {
            self.apply_snapshot_bars(b, seq, bars.to_vec());
            self.apply_anchor(ctx, b);
        } else {
            let (merged, same) = self.merge_snapshot(b, bars);
            self.apply_snapshot_bars(b, seq, merged);
            if !same {
                self.binds[b].set_t = None;
                self.binds[b].epoch += 1;
                self.binds[b].poll_inflight = false;
                self.binds[b].backfill_pending = false;
                self.request_backfill(ctx, b);
            }
        }
        let st = &mut self.binds[b];
        st.missing.clear();
        st.big_gaps.clear();
        st.last_seq = seq;
        st.frontier = bars.last().map(|x| x.t).or(st.frontier);
        st.behind = false;
        self.after_engine(now);
        self.refresh_binding(now, b, old_set);
        if first_time || self.binds[b].set_t.is_none() {
            self.poll(ctx, b);
        }
    }

    fn flush(&mut self, ctx: &mut Ctx) {
        if !self.out.is_empty() {
            let msg = ReportMsg {
                ty: "report".into(),
                kind: "divergence".into(),
                items: std::mem::take(&mut self.out),
            };
            ctx.send(serde_json::to_vec(&msg).expect("json"));
        }
        let mut timers = std::mem::take(&mut self.timers);
        timers.sort_unstable();
        timers.dedup();
        for (at, tk) in timers {
            ctx.set_timer(at, tk);
        }
    }
}

impl ReplicaAdapter for Replica {
    fn on_source(&mut self, ctx: &mut Ctx, ev: &SourceEvent) {
        let Some(b) = self.setup.binding_of(ev.feed()) else {
            return;
        };
        let now = ctx.now();
        if !self.housekeeping {
            self.housekeeping = true;
            self.timers.push((now + HOUSEKEEP, TK_HOUSEKEEP));
        }
        {
            let st = &mut self.binds[b];
            if matches!(ev, SourceEvent::Data(_)) {
                if let Some(last) = st.last_data_at {
                    #[allow(clippy::cast_precision_loss)]
                    let gap = (now - last) as f64;
                    st.gap_ewma = if st.gap_ewma == 0.0 {
                        gap
                    } else {
                        (1.0 - EWMA_A) * st.gap_ewma + EWMA_A * gap
                    };
                }
                st.last_data_at = Some(now);
            }
            st.last_feed_at = Some(now);
        }
        self.set_flag(now, b, |s| std::mem::replace(&mut s.silent, false));
        match ev {
            SourceEvent::Snapshot { seq, bars, .. } => self.on_feed_snapshot(ctx, b, *seq, bars),
            SourceEvent::Data(d) => self.on_feed_data(ctx, b, ev, d.seq, &d.change),
        }
        self.flush(ctx);
    }

    fn on_authority_msg(&mut self, ctx: &mut Ctx, msg: &[u8]) {
        let Ok(m) = serde_json::from_slice::<AuthMsg>(msg) else {
            return;
        };
        if m.n < self.auth_next {
            return;
        }
        self.auth_buf.insert(m.n, m);
        let now = ctx.now();
        while let Some(m) = self.auth_buf.remove(&self.auth_next) {
            self.auth_next += 1;
            for (id, a) in &m.a {
                if let Some(b) = self.engine.index(id) {
                    if self.binds[b].anchor.is_none() {
                        self.binds[b].anchor = Some(*a);
                        self.apply_anchor(ctx, b);
                        if self.engine.bars(b).is_empty() {
                            self.poll(ctx, b);
                        }
                        self.after_engine(now);
                        self.maybe_ready(now, b);
                    }
                }
            }
            for s in &m.add {
                if let Some(k) = parse_key(s) {
                    self.index_key(&k);
                    self.auth_finals.insert(k.clone());
                    self.relabel(now, &k);
                }
            }
            for s in &m.del {
                if let Some(k) = parse_key(s) {
                    self.auth_finals.remove(&k);
                    self.relabel(now, &k);
                }
            }
            for (id, w) in &m.w {
                if let Some(b) = self.engine.index(id) {
                    let old = self.auth_w[b];
                    if old.is_none_or(|o| *w > o) {
                        self.auth_w[b] = Some(*w);
                        self.relabel_range(now, b, old, Some(*w));
                    }
                }
            }
        }
        self.flush(ctx);
    }

    fn on_timer(&mut self, ctx: &mut Ctx, token: u64) {
        let now = ctx.now();
        match token & TK_MASK {
            TK_HOUSEKEEP => {
                for b in 0..self.binds.len() {
                    let st = &self.binds[b];
                    #[allow(clippy::cast_possible_truncation)]
                    let thr = SILENCE_MIN.max((SILENCE_GAPS * st.gap_ewma) as VTime);
                    let silent = st.last_feed_at.is_some_and(|l| now - l > thr);
                    if silent {
                        self.set_flag(now, b, |s| !std::mem::replace(&mut s.silent, true));
                    }
                }
                self.timers.push((now + HOUSEKEEP, TK_HOUSEKEEP));
            }
            TK_POLL => {
                #[allow(clippy::cast_possible_truncation)]
                let b = (token & !TK_MASK) as usize;
                if b < self.binds.len() && self.binds[b].poll_at == Some(now) {
                    self.poll(ctx, b);
                }
            }
            TK_GRACE => {
                let due: Vec<ClaimKey> = self
                    .disagree_since
                    .iter()
                    .filter(|(_, &s)| s + GRACE <= now)
                    .map(|(k, _)| k.clone())
                    .collect();
                for k in due {
                    self.relabel(now, &k);
                }
            }
            _ => {}
        }
        self.flush(ctx);
    }

    fn on_fetch(&mut self, ctx: &mut Ctx, reply: &FetchReply) {
        let Some(req) = self.reqs.remove(&reply.id) else {
            return;
        };
        let now = ctx.now();
        if req.epoch != self.binds[req.b].epoch {
            return;
        }
        match req.kind {
            ReqKind::Poll => self.on_poll_reply(now, req, &reply.bars),
            ReqKind::Backfill => self.on_backfill_reply(now, req, &reply.bars),
        }
        self.flush(ctx);
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
