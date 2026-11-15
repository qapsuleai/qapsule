//! Approach A — data position stamp (RESEARCH §3.A), at its strongest, with the data
//! mark of §3.E folded in. See `rounds/RP2/experiments/T7a.md`.
//!
//! **Authority.** Runs the shared engine on its own feed. For every binding it announces
//! a data position `[b, W, h, e, a, s]`: the last *certified* bar `W`, the digest `h` of
//! every bar from the compute anchor `a` through `W` (FNV-1a 64 over canonical bar
//! bytes — exactly `qp_schema::data::data_digest` of that prefix, kept as a streaming
//! chain), the data epoch `e` (bumped when a correction or snapshot changes a
//! certified bar) and `s` = 1 if every bar through `W` has been checked against the
//! provider's settled bars (`ctx.fetch_settled`), 0 if the tail is certified only by
//! the feed's sequence numbers (closed bars whose events arrived without a `seq` gap
//! are the source's bars). Every closed bar is later checked against the settled source
//! and replaced if it differs. With the position it lists the claims that became
//! certified (`f`: every binding of the claim is certified at or past its bar) — a
//! "facts through `W`" mark (§3.E). Claims that leave the authority's claim set are
//! retracted at once (`x`); hiding is always safe. Uncertified claims (live bar) are
//! only hinted (`l`, coalesced) so the replica can show a non-asserting `Pending`.
//!
//! **Replica.** Runs the shared engine on its own feed. It shows a claim `Final` only
//! when the authority certified it, the replica's own data has reached every position
//! the claim rests on (bar `W` present) and its own prefix digest at `W` equals the
//! authority's. Same data + same anchor + same engine ⇒ same claims, so the claim then
//! holds on the replica's visible data; and the digest equals that of data the
//! authority certified as the source's. A shown claim stays while it is still
//! certified and still holds on the replica's data; otherwise it is hidden at once. On
//! a digest mismatch the replica refetches settled bars from its own provider (first
//! the span since its last verified position, then the whole prefix from the
//! authority's anchor — which also extends a shorter window to the authority's
//! anchor) and marks the binding `Disputed` if that does not reconcile. Market data
//! never crosses the protocol channel: only positions, digests and claim keys (K23).
//!
//! Channel ordering is not trusted: per binding the newest position wins (by message
//! `s`), per claim the newest add/retract wins; duplicates are idempotent.

use std::collections::{BTreeMap, BTreeSet};

use qp_schema::data::{DataChange, FeedKey};
use qp_schema::Bar;
use serde::{Deserialize, Serialize};

use crate::adapter::{
    Approach, AuthorityAdapter, ChannelEvent, Ctx, FetchReply, ReplicaAdapter, Setup, SourceEvent,
};
use crate::detector::{ClaimKey, DetectorCfg, Direction, Maturity, CROSS, INVALIDATION, PAIR};
use crate::display::{DisplayItem, DisplayState, Label};
use crate::engine::{Applied, ApplyStatus, Engine};
use crate::spec::{VTime, BASE_T, LIVE0};

/// Assumed settle lag of the provider (ms). A fetch that comes back short is retried
/// after [`RETRY_US`]; the value only shifts latency, never safety.
const SETTLE_GUESS_MS: i64 = 2000;
/// Retry delay of a short settled fetch (µs).
const RETRY_US: VTime = 250_000;
/// Coalescing window of the authority's `Pending` hints (µs).
const LIVE_COALESCE_US: VTime = if LIVE_PROVISIONAL { 0 } else { 500_000 };
/// Experiment switch (off in the measured approach; T7a.md "A-canlı"): show the
/// authority's live hints as `Provisional` when the replica's own engine holds the claim
/// on its live bar and the binding's prefix is verified at the authority's position.
const LIVE_PROVISIONAL: bool = false;
/// After the replica's own closed bars change (a correction reached it first), it waits
/// this long for the authority's new position before refetching (µs).
const RESEAL_WAIT_US: VTime = 1_500_000;
/// Mismatch refetches against a settled position before the binding is marked
/// `Disputed` (until a newer position arrives).
const MAX_LEVEL: u8 = 2;
/// A settled answer with more bars than this is merged as one snapshot of the replica's
/// own data (one recompute instead of one per bar).
const MERGE_AS_SNAPSHOT: usize = 32;

const TIMER_FETCH: u64 = 1 << 32;
const TIMER_LIVE: u64 = 2 << 32;
const TIMER_HOLD: u64 = 3 << 32;
const TIMER_PREFETCH: u64 = 4 << 32;
const TIMER_MASK: u64 = 0xffff_ffff;

// ---------------------------------------------------------------------------------
// Digest chain.

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0100_0000_01b3;

fn fnv_word(mut h: u64, v: u64) -> u64 {
    for b in v.to_le_bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(FNV_PRIME);
    }
    h
}

/// Folds one bar into the running FNV-1a state (same byte order as
/// `qp_schema::data::data_digest`, so the chain value after bar `i` is the
/// `data_digest` of bars `0..=i`).
fn fold_bar(h: u64, b: &Bar) -> u64 {
    let mut h = fnv_word(h, u64::from_le_bytes(b.t.to_le_bytes()));
    for v in [b.o, b.h, b.l, b.c, b.v] {
        h = fnv_word(h, v.to_bits());
    }
    h
}

/// Prefix digests of one binding's bars, cached and invalidated from the first changed
/// index.
#[derive(Debug, Default)]
struct Chain {
    vals: Vec<u64>,
}

impl Chain {
    fn invalidate(&mut self, idx: usize) {
        self.vals.truncate(idx);
    }

    /// Digest of `bars[0..=idx]`; returns (digest, bars hashed now).
    fn at(&mut self, bars: &[Bar], idx: usize) -> (u64, u64) {
        let mut work = 0;
        while self.vals.len() <= idx {
            let prev = self.vals.last().copied().unwrap_or(FNV_OFFSET);
            let i = self.vals.len();
            self.vals.push(fold_bar(prev, &bars[i]));
            work += 1;
        }
        (self.vals[idx], work)
    }
}

/// First bar time at which two bar vectors differ (bitwise), if any.
fn first_diff(old: &[Bar], new: &[Bar]) -> Option<i64> {
    let same = |a: &Bar, b: &Bar| {
        a.t == b.t
            && a.o.to_bits() == b.o.to_bits()
            && a.h.to_bits() == b.h.to_bits()
            && a.l.to_bits() == b.l.to_bits()
            && a.c.to_bits() == b.c.to_bits()
            && a.v.to_bits() == b.v.to_bits()
    };
    let n = old.len().min(new.len());
    for i in 0..n {
        if !same(&old[i], &new[i]) {
            return Some(old[i].t.min(new[i].t));
        }
    }
    if old.len() == new.len() {
        None
    } else if old.len() > n {
        Some(old[n].t)
    } else {
        Some(new[n].t)
    }
}

fn idx_from(bars: &[Bar], t: i64) -> usize {
    bars.partition_point(|x| x.t < t)
}

/// Virtual time a bar closes (bars are calendar-aligned, K14; this is the harness's
/// mapping of bar time to its clock — a device knows the wall clock the same way).
fn close_vtime(t: i64, interval_ms: i64) -> VTime {
    LIVE0 + (t - BASE_T + interval_ms) * 1000
}

/// Earliest virtual time the settled source can answer for bar `t`.
fn settled_at(t: i64, interval_ms: i64) -> VTime {
    close_vtime(t, interval_ms) + SETTLE_GUESS_MS * 1000 + 1000
}

/// Was an applied source event a change to an already closed bar?
fn closed_change(applied: &Applied, bars: &[Bar]) -> Option<i64> {
    let from = applied.changed_from_t?;
    let live = bars.last()?.t;
    (!applied.new_bar && from < live).then_some(from)
}

fn event_t(ev: &SourceEvent) -> Option<i64> {
    match ev {
        SourceEvent::Data(d) => Some(match &d.change {
            DataChange::Bar { bar } => bar.t,
            DataChange::Patch { t, .. } => *t,
        }),
        SourceEvent::Snapshot { .. } => None,
    }
}

// ---------------------------------------------------------------------------------
// Claim key codec: `[kind, binding index, bar t]`, kind ∈ u (cross up), d (cross
// down), i (invalidation), p (pair; binding index 0).

#[derive(Debug, Clone)]
struct Codec {
    ids: Vec<String>,
    up: String,
    lo: String,
    inval: String,
    pair: String,
}

fn num(x: f64) -> String {
    format!("{x}")
}

impl Codec {
    fn new(ids: Vec<String>, cfg: &DetectorCfg) -> Self {
        let s = &cfg.spec;
        let inv = s.invalidation.clone().unwrap_or_default();
        Self {
            ids,
            up: format!("period={},level={}", s.period, num(s.upper)),
            lo: format!("period={},level={}", s.period, num(s.lower)),
            inval: format!(
                "period={},level={},upper={},lookback={}",
                s.period,
                num(inv.level),
                num(s.upper),
                inv.lookback
            ),
            pair: format!("period={},level={}", s.period, num(s.upper)),
        }
    }

    fn kind(k: &ClaimKey) -> &'static str {
        match (k.detector.as_str(), k.direction) {
            (CROSS, Direction::Up) => "u",
            (CROSS, Direction::Down) => "d",
            (INVALIDATION, _) => "i",
            (PAIR, _) => "p",
            _ => "?",
        }
    }

    /// Binding indices a key rests on.
    fn bindings(&self, k: &ClaimKey) -> Vec<usize> {
        k.bindings()
            .iter()
            .filter_map(|id| self.ids.iter().position(|x| x == id))
            .collect()
    }

    fn encode(&self, k: &ClaimKey) -> WireKey {
        let b = self.bindings(k).first().copied().unwrap_or(0);
        (Self::kind(k).to_owned(), b, k.bar_t)
    }

    fn decode(&self, w: &WireKey) -> Option<ClaimKey> {
        let (kind, b, t) = w;
        let id = self.ids.get(*b)?.clone();
        let (detector, params, binding, direction) = match kind.as_str() {
            "u" => (CROSS, self.up.clone(), id, Direction::Up),
            "d" => (CROSS, self.lo.clone(), id, Direction::Down),
            "i" => (INVALIDATION, self.inval.clone(), id, Direction::Down),
            "p" => (
                PAIR,
                self.pair.clone(),
                format!("{}+{}", self.ids.first()?, self.ids.get(1)?),
                Direction::Up,
            ),
            _ => return None,
        };
        Some(ClaimKey {
            detector: detector.to_owned(),
            params,
            binding,
            bar_t: *t,
            direction,
        })
    }
}

// ---------------------------------------------------------------------------------
// Wire format.

/// `[kind, binding, bar_t]`.
type WireKey = (String, usize, i64);
/// Data position: `[binding, W, digest of anchor..=W (16 hex), epoch, anchor t,
/// settled (1) or feed-certified (0)]`.
type WirePos = (usize, i64, String, u32, i64, u8);

/// Authority → replica.
#[derive(Debug, Default, Serialize, Deserialize)]
struct AMsg {
    /// Message sequence number (per authority, from 1).
    s: u64,
    /// Data positions (data marks).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    p: Vec<WirePos>,
    /// Claims certified at these positions (final).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    f: Vec<WireKey>,
    /// Claims retracted.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    x: Vec<WireKey>,
    /// Pending hints: binding → its current uncertified claims `[kind, bar_t]`
    /// (replaces the previous hint of that binding).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    l: Vec<(usize, Vec<(String, i64)>)>,
}

impl AMsg {
    fn is_empty(&self) -> bool {
        self.p.is_empty() && self.f.is_empty() && self.x.is_empty() && self.l.is_empty()
    }
}

/// Replica → authority: data disputes `[binding, W]` (for the authority's records).
#[derive(Debug, Default, Serialize, Deserialize)]
struct RMsg {
    d: Vec<(usize, i64)>,
}

// ---------------------------------------------------------------------------------
// Authority.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Pos {
    w: i64,
    h: u64,
    e: u32,
    settled: bool,
}

#[derive(Debug, Default)]
struct ABind {
    chain: Chain,
    /// Every bar from the anchor through this time has been checked against the
    /// provider's settled bars.
    settled_upto: Option<i64>,
    /// Bars that may have missed events (a `seq` gap was seen around them).
    taint: BTreeSet<i64>,
    /// Bar time of the last applied data event.
    last_ev_t: Option<i64>,
    /// Last announced position.
    pos: Option<Pos>,
    epoch: u32,
    /// Closed-bar changes from the feed (a fetch issued before one is discarded).
    gen: u64,
    /// In-flight fetch: (id, gen at request, requested from_t).
    fetch: Option<(u64, u64, i64)>,
    fetch_timer: Option<VTime>,
    live_sent: Vec<(String, i64)>,
}

struct Authority {
    engine: Engine,
    feeds: Vec<FeedKey>,
    codec: Codec,
    interval_ms: Vec<i64>,
    binds: Vec<ABind>,
    /// Claims announced as certified.
    sent: BTreeSet<ClaimKey>,
    seq: u64,
    out: AMsg,
    live_dirty: BTreeSet<usize>,
    live_timer: bool,
}

impl Authority {
    fn new(s: &Setup) -> Self {
        let engine = Engine::new(s);
        let codec = Codec::new(engine.ids().to_vec(), engine.detector());
        Self {
            engine,
            feeds: s.bindings.iter().map(|b| b.feed.clone()).collect(),
            codec,
            interval_ms: s.bindings.iter().map(|b| b.interval_ms).collect(),
            binds: s.bindings.iter().map(|_| ABind::default()).collect(),
            sent: BTreeSet::new(),
            seq: 0,
            out: AMsg::default(),
            live_dirty: BTreeSet::new(),
            live_timer: false,
        }
    }

    fn flush(&mut self, ctx: &mut Ctx) {
        if self.out.is_empty() {
            return;
        }
        self.seq += 1;
        let mut m = std::mem::take(&mut self.out);
        m.s = self.seq;
        ctx.send(serde_json::to_vec(&m).expect("json"));
    }

    fn last_closed(&self, b: usize) -> Option<i64> {
        let bars = self.engine.bars(b);
        (bars.len() >= 2).then(|| bars[bars.len() - 2].t)
    }

    /// A certified bar changed through the feed (correction, snapshot): it is no longer
    /// provider-checked from `t` on; refetch and bump the epoch.
    fn unsettle(&mut self, ctx: &mut Ctx, b: usize, t: i64) {
        let bars = self.engine.bars(b);
        let i = idx_from(bars, t);
        let prev = i.checked_sub(1).map(|j| bars[j].t);
        let bb = &mut self.binds[b];
        if bb.settled_upto.is_some_and(|s| s >= t) {
            bb.settled_upto = prev;
        }
        if bb.pos.is_some_and(|p| p.w >= t) {
            bb.epoch += 1;
        }
        bb.fetch = None;
        self.try_fetch(ctx, b);
    }

    fn handle_deltas(&mut self) {
        for d in self.engine.take_deltas() {
            for b in self.codec.bindings(&d.key) {
                self.live_dirty.insert(b);
            }
            if d.after.is_none() && self.sent.remove(&d.key) {
                self.out.x.push(self.codec.encode(&d.key));
            }
        }
    }

    fn try_fetch(&mut self, ctx: &mut Ctx, b: usize) {
        if self.binds[b].fetch.is_some() {
            return;
        }
        let Some(to) = self.last_closed(b) else {
            return;
        };
        let bars = self.engine.bars(b);
        let from = match self.binds[b].settled_upto {
            Some(s) => match bars.get(idx_from(bars, s + 1)) {
                Some(x) => x.t,
                None => return,
            },
            None => match bars.first() {
                Some(x) => x.t,
                None => return,
            },
        };
        if from > to {
            return;
        }
        let ready = settled_at(to, self.interval_ms[b]);
        if ready > ctx.now() {
            if self.binds[b].fetch_timer.is_none_or(|at| at > ready) {
                self.binds[b].fetch_timer = Some(ready);
                ctx.set_timer(ready, TIMER_FETCH | b as u64);
            }
            return;
        }
        let id = ctx.fetch_settled(b, from, to);
        let gen = self.binds[b].gen;
        self.binds[b].fetch = Some((id, gen, from));
    }

    /// Last certified bar: provider-checked through `settled_upto`, then closed bars
    /// received without a `seq` gap. Returns (W, all settled).
    fn certified(&self, b: usize) -> Option<(i64, bool)> {
        let bars = self.engine.bars(b);
        let lc = self.last_closed(b)?;
        let bb = &self.binds[b];
        let mut w = bb.settled_upto.map(|s| s.min(lc));
        let start = bb.settled_upto.map_or(0, |s| idx_from(bars, s + 1));
        for x in bars.iter().skip(start) {
            if x.t > lc || bb.taint.contains(&x.t) {
                break;
            }
            w = Some(x.t);
        }
        let w = w?;
        Some((w, bb.settled_upto.is_some_and(|s| s >= w)))
    }

    /// Announces the position of `b` if it moved, and the claims it certifies.
    fn advance(&mut self, b: usize) {
        let Some((w, settled)) = self.certified(b) else {
            return;
        };
        let bars = self.engine.bars(b);
        let i = idx_from(bars, w);
        if i >= bars.len() || bars[i].t != w {
            return;
        }
        let anchor = bars[0].t;
        let (h, work) = self.binds[b].chain.at(self.engine.bars(b), i);
        self.engine.add_work(work);
        let pos = Pos {
            w,
            h,
            e: self.binds[b].epoch,
            settled,
        };
        if self.binds[b].pos == Some(pos) {
            return;
        }
        if let Some(old) = self.binds[b].pos {
            // The data under an announced position changed without a feed correction
            // (the settled check replaced a bar): new epoch.
            if old.w <= w && old.e == pos.e && old.h != self.digest_at(b, old.w) {
                self.binds[b].epoch += 1;
            }
        }
        let pos = Pos {
            e: self.binds[b].epoch,
            ..pos
        };
        self.binds[b].pos = Some(pos);
        self.out.p.retain(|p| p.0 != b);
        self.out.p.push((
            b,
            pos.w,
            format!("{:016x}", pos.h),
            pos.e,
            anchor,
            u8::from(pos.settled),
        ));
        self.certify_claims(Some(b));
    }

    fn digest_at(&mut self, b: usize, t: i64) -> u64 {
        let bars = self.engine.bars(b);
        let i = idx_from(bars, t);
        if i >= bars.len() {
            return 0;
        }
        let (h, work) = self.binds[b].chain.at(self.engine.bars(b), i);
        self.engine.add_work(work);
        h
    }

    /// Announces claims whose every binding is certified at or past their bar
    /// (`only`: claims involving this binding).
    fn certify_claims(&mut self, only: Option<usize>) {
        let mut add = Vec::new();
        for (k, c) in self.engine.claims() {
            if c.maturity != Maturity::Final || self.sent.contains(k) {
                continue;
            }
            let bs = self.codec.bindings(k);
            if only.is_some_and(|b| !bs.contains(&b)) {
                continue;
            }
            if !bs.is_empty()
                && bs
                    .iter()
                    .all(|&x| self.binds[x].pos.is_some_and(|p| p.w >= k.bar_t))
            {
                add.push(k.clone());
            }
        }
        self.engine.add_work(self.engine.claims().len() as u64);
        for k in add {
            self.out.f.push(self.codec.encode(&k));
            for x in self.codec.bindings(&k) {
                self.live_dirty.insert(x);
            }
            self.sent.insert(k);
        }
    }

    fn arm_live(&mut self, ctx: &mut Ctx) {
        if !self.live_dirty.is_empty() && !self.live_timer {
            self.live_timer = true;
            ctx.set_timer(ctx.now() + LIVE_COALESCE_US, TIMER_LIVE);
        }
    }

    fn flush_live(&mut self) {
        let dirty = std::mem::take(&mut self.live_dirty);
        for b in dirty {
            let bars = self.engine.bars(b);
            let cutoff = if bars.len() >= 2 {
                bars[bars.len() - 2].t
            } else {
                i64::MIN
            };
            let cur: Vec<(String, i64)> = self
                .engine
                .claims()
                .keys()
                .filter(|k| {
                    k.bar_t >= cutoff
                        && !self.sent.contains(*k)
                        && self.codec.bindings(k).first() == Some(&b)
                })
                .map(|k| (Codec::kind(k).to_owned(), k.bar_t))
                .collect();
            if cur != self.binds[b].live_sent {
                self.binds[b].live_sent.clone_from(&cur);
                self.out.l.push((b, cur));
            }
        }
    }

    fn taint_gap(&mut self, b: usize, t_ev: i64) {
        let lo = self.binds[b].last_ev_t.map_or(t_ev, |x| x.min(t_ev));
        let hi = self.binds[b].last_ev_t.map_or(t_ev, |x| x.max(t_ev));
        let ts: Vec<i64> = self
            .engine
            .range(b, lo, hi)
            .iter()
            .map(|x| x.t)
            .chain([lo, hi])
            .collect();
        self.binds[b].taint.extend(ts);
    }
}

impl AuthorityAdapter for Authority {
    fn on_source(&mut self, ctx: &mut Ctx, ev: &SourceEvent) {
        let Some(b) = self.feeds.iter().position(|f| f == ev.feed()) else {
            return;
        };
        let old = match ev {
            SourceEvent::Snapshot { .. } => Some(self.engine.bars(b).to_vec()),
            SourceEvent::Data(_) => None,
        };
        let applied = self.engine.apply(ev);
        match (&old, event_t(ev)) {
            (Some(_), _) => {
                // A snapshot is the source's state: nothing in it is missing.
                self.binds[b].taint.clear();
                self.binds[b].last_ev_t = None;
            }
            (None, Some(t)) => match applied.status {
                ApplyStatus::Gap { .. } => {
                    self.taint_gap(b, t);
                    self.binds[b].last_ev_t = Some(t);
                }
                ApplyStatus::Applied => self.binds[b].last_ev_t = Some(t),
                _ => {}
            },
            (None, None) => {}
        }
        let changed = match &old {
            Some(old) => first_diff(old, self.engine.bars(b)),
            None => applied.changed_from_t,
        };
        if let Some(t) = changed {
            let idx = idx_from(self.engine.bars(b), t);
            self.binds[b].chain.invalidate(idx);
        }
        let closed = match &old {
            Some(_) => changed,
            None => closed_change(&applied, self.engine.bars(b)),
        };
        if let Some(t) = closed {
            self.binds[b].gen += 1;
            self.unsettle(ctx, b, t);
        }
        self.handle_deltas();
        if applied.new_bar || old.is_some() || closed.is_some() {
            self.try_fetch(ctx, b);
            self.advance(b);
            if self.engine.detector().spec.pair {
                self.certify_claims(None);
            }
        }
        self.arm_live(ctx);
        self.flush(ctx);
    }

    fn on_replica_msg(&mut self, _ctx: &mut Ctx, msg: &[u8]) {
        // Disputes are only recorded: the authority's data is provider-checked.
        let _ = serde_json::from_slice::<RMsg>(msg);
    }

    fn on_timer(&mut self, ctx: &mut Ctx, token: u64) {
        match token & !TIMER_MASK {
            TIMER_FETCH => {
                let b = (token & TIMER_MASK) as usize;
                if self.binds[b].fetch_timer.is_some_and(|at| at <= ctx.now()) {
                    self.binds[b].fetch_timer = None;
                }
                self.try_fetch(ctx, b);
            }
            TIMER_LIVE => {
                self.live_timer = false;
                self.flush_live();
            }
            _ => {}
        }
        self.arm_live(ctx);
        self.flush(ctx);
    }

    fn on_fetch(&mut self, ctx: &mut Ctx, reply: &FetchReply) {
        let b = reply.binding;
        let Some((id, gen, from)) = self.binds[b].fetch else {
            return;
        };
        if id != reply.id {
            return;
        }
        self.binds[b].fetch = None;
        if gen != self.binds[b].gen {
            // Closed bars changed while the request was in flight: the answer may predate
            // a correction. Ask again.
            self.try_fetch(ctx, b);
            self.flush(ctx);
            return;
        }
        let applied = self.engine.apply_settled(b, &reply.bars);
        if let Some(t) = applied.changed_from_t {
            let idx = idx_from(self.engine.bars(b), t);
            self.binds[b].chain.invalidate(idx);
        }
        for x in &reply.bars {
            self.binds[b].taint.remove(&x.t);
        }
        self.handle_deltas();
        if reply.bars.first().is_some_and(|x| x.t == from) {
            if let Some(last) = reply.bars.last() {
                let s = &mut self.binds[b].settled_upto;
                *s = Some(s.map_or(last.t, |x| x.max(last.t)));
            }
        }
        self.advance(b);
        if self.engine.detector().spec.pair {
            self.certify_claims(None);
        }
        // More closed bars than this answer covered (short answer or new bars)?
        if let Some(lc) = self.last_closed(b) {
            if self.binds[b].settled_upto.is_none_or(|s| s < lc) {
                let at = settled_at(lc, self.interval_ms[b]).max(ctx.now() + RETRY_US);
                if self.binds[b].fetch_timer.is_none_or(|x| x > at || x < ctx.now()) {
                    self.binds[b].fetch_timer = Some(at);
                    ctx.set_timer(at, TIMER_FETCH | b as u64);
                }
            }
        }
        self.arm_live(ctx);
        self.flush(ctx);
    }

    fn on_channel(&mut self, _ctx: &mut Ctx, _ev: ChannelEvent) {}

    fn work_units(&self) -> u64 {
        self.engine.work()
    }
}

// ---------------------------------------------------------------------------------
// Replica.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
    /// No position yet, or the replica's data has not reached it.
    Waiting,
    /// Prefix digest at the authority's position equals the replica's.
    Verified,
    /// Digests differ; refetching settled bars.
    Fetching,
    /// Digests still differ after refetching (or the anchor cannot be aligned).
    Disputed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Seal {
    w: i64,
    h: u64,
    e: u32,
    a: i64,
    settled: bool,
}

#[derive(Debug)]
struct RBind {
    chain: Chain,
    seal: Option<Seal>,
    seal_seq: u64,
    status: Status,
    last_verified: Option<i64>,
    /// In-flight fetch: (id, gen at request, counts as a mismatch attempt).
    fetch: Option<(u64, u64, bool)>,
    level: u8,
    gen: u64,
    /// Hold refetching until this time (a correction reached us before the authority's
    /// new position).
    hold_until: Option<VTime>,
    prefetch_at: Option<VTime>,
    caught_up: bool,
    disputed_sent: Option<i64>,
    live_seq: u64,
    live: BTreeSet<ClaimKey>,
}

impl Default for RBind {
    fn default() -> Self {
        Self {
            chain: Chain::default(),
            seal: None,
            seal_seq: 0,
            status: Status::Waiting,
            last_verified: None,
            fetch: None,
            level: 0,
            gen: 0,
            hold_until: None,
            prefetch_at: None,
            caught_up: false,
            disputed_sent: None,
            live_seq: 0,
            live: BTreeSet::new(),
        }
    }
}

struct Replica {
    engine: Engine,
    feeds: Vec<FeedKey>,
    codec: Codec,
    interval_ms: Vec<i64>,
    binds: Vec<RBind>,
    /// Authority's certified set: key → (message seq of the last op, present).
    sealed: BTreeMap<ClaimKey, (u64, bool)>,
    display: DisplayState,
    touched: BTreeSet<ClaimKey>,
    touched_bindings: BTreeSet<usize>,
    disputes: Vec<(usize, i64)>,
}

impl Replica {
    fn new(s: &Setup) -> Self {
        let engine = Engine::new(s);
        let codec = Codec::new(engine.ids().to_vec(), engine.detector());
        Self {
            engine,
            feeds: s.bindings.iter().map(|b| b.feed.clone()).collect(),
            codec,
            interval_ms: s.bindings.iter().map(|b| b.interval_ms).collect(),
            binds: s.bindings.iter().map(|_| RBind::default()).collect(),
            sealed: BTreeMap::new(),
            display: DisplayState::default(),
            touched: BTreeSet::new(),
            touched_bindings: BTreeSet::new(),
            disputes: Vec::new(),
        }
    }

    fn set_status(&mut self, b: usize, s: Status) {
        if self.binds[b].status != s {
            self.binds[b].status = s;
            self.touched_bindings.insert(b);
        }
    }

    /// Our own data changed at `t`; `closed`: a closed bar changed; `own`: by our own
    /// settled fetch.
    fn on_data_change(&mut self, ctx: &mut Ctx, b: usize, t: i64, closed: bool, own: bool) {
        let idx = idx_from(self.engine.bars(b), t);
        self.binds[b].chain.invalidate(idx);
        let bars = self.engine.bars(b);
        let prev = idx.checked_sub(1).map(|j| bars[j].t);
        let rb = &mut self.binds[b];
        if rb.last_verified.is_some_and(|v| v >= t) {
            rb.last_verified = prev;
        }
        if closed && !own {
            rb.gen += 1;
            if rb.seal.is_some_and(|s| s.w >= t) {
                let until = ctx.now() + RESEAL_WAIT_US;
                rb.hold_until = Some(until);
                ctx.set_timer(until, TIMER_HOLD | b as u64);
            }
        }
    }

    /// Compares our prefix digest at the authority's position.
    fn verify(&mut self, ctx: &mut Ctx, b: usize) {
        let Some(seal) = self.binds[b].seal else {
            self.set_status(b, Status::Waiting);
            return;
        };
        let bars = self.engine.bars(b);
        if bars.is_empty() {
            // No data from our feed yet: catch up from our own settled source.
            if !self.binds[b].caught_up && self.binds[b].fetch.is_none() {
                self.binds[b].caught_up = true;
                let id = ctx.fetch_settled(b, seal.a, seal.w);
                let gen = self.binds[b].gen;
                self.binds[b].fetch = Some((id, gen, false));
            }
            self.set_status(b, Status::Waiting);
            return;
        }
        let Ok(i) = bars.binary_search_by_key(&seal.w, |x| x.t) else {
            // Not there yet (feed behind) — unless our window starts after the
            // authority's anchor and the bar is older than our first bar.
            if seal.a < bars[0].t && seal.w < bars[0].t {
                self.mismatch(ctx, b, seal);
            } else {
                self.set_status(b, Status::Waiting);
            }
            return;
        };
        if bars[0].t == seal.a {
            let (h, work) = self.binds[b].chain.at(self.engine.bars(b), i);
            self.engine.add_work(work + 1);
            if h == seal.h {
                let rb = &mut self.binds[b];
                rb.last_verified = Some(seal.w);
                rb.level = 0;
                rb.hold_until = None;
                self.set_status(b, Status::Verified);
                return;
            }
        }
        self.mismatch(ctx, b, seal);
    }

    fn fetch_range(&self, b: usize, seal: Seal) -> (i64, i64) {
        let rb = &self.binds[b];
        let anchor = self.engine.bars(b).first().map_or(i64::MAX, |x| x.t);
        let from = match rb.last_verified {
            Some(v) if rb.level == 0 && anchor == seal.a && v < seal.w => v + 1,
            _ => seal.a,
        };
        (from, seal.w)
    }

    fn mismatch(&mut self, ctx: &mut Ctx, b: usize, seal: Seal) {
        let anchor = self.engine.bars(b).first().map_or(i64::MAX, |x| x.t);
        if seal.a > anchor {
            // We hold history before the authority's anchor; we cannot un-hold it.
            self.dispute(b, seal);
            return;
        }
        if self.binds[b].hold_until.is_some_and(|u| u > ctx.now()) {
            self.set_status(b, Status::Waiting);
            return;
        }
        if self.binds[b].fetch.is_some() {
            self.set_status(b, Status::Fetching);
            return;
        }
        if !seal.settled {
            // A feed-certified position: our bars differ from the authority's feed bars.
            // Wait for its settled position, but have our settled bars ready by then.
            let at = settled_at(seal.w, self.interval_ms[b]).max(ctx.now());
            if self.binds[b].prefetch_at.is_none_or(|x| x < ctx.now()) {
                self.binds[b].prefetch_at = Some(at);
                ctx.set_timer(at, TIMER_PREFETCH | b as u64);
            }
            self.set_status(b, Status::Waiting);
            return;
        }
        if self.binds[b].level >= MAX_LEVEL {
            self.dispute(b, seal);
            return;
        }
        let (from, to) = self.fetch_range(b, seal);
        let id = ctx.fetch_settled(b, from, to);
        let rb = &mut self.binds[b];
        rb.fetch = Some((id, rb.gen, true));
        rb.level += 1;
        self.set_status(b, Status::Fetching);
    }

    fn dispute(&mut self, b: usize, seal: Seal) {
        if self.binds[b].disputed_sent != Some(seal.w) {
            self.binds[b].disputed_sent = Some(seal.w);
            self.disputes.push((b, seal.w));
        }
        self.set_status(b, Status::Disputed);
    }

    fn keys_of_binding(&self, b: usize) -> Vec<ClaimKey> {
        let mut v: BTreeSet<ClaimKey> = BTreeSet::new();
        for (k, (_, present)) in &self.sealed {
            if *present && self.codec.bindings(k).contains(&b) {
                v.insert(k.clone());
            }
        }
        for k in self.display.items.keys() {
            if self.codec.bindings(k).contains(&b) {
                v.insert(k.clone());
            }
        }
        v.extend(self.binds[b].live.iter().cloned());
        v.into_iter().collect()
    }

    fn evaluate(&mut self, k: &ClaimKey) {
        self.engine.add_work(1);
        let present = self.sealed.get(k).is_some_and(|(_, p)| *p);
        let bs = self.codec.bindings(k);
        let held = self
            .engine
            .claim(k)
            .filter(|c| c.maturity == Maturity::Final)
            .cloned();
        let label = if present {
            let verified = !bs.is_empty()
                && bs.iter().all(|&b| {
                    let rb = &self.binds[b];
                    rb.status == Status::Verified && rb.seal.is_some_and(|s| s.w >= k.bar_t)
                });
            let shown = self
                .display
                .items
                .get(k)
                .is_some_and(|i| i.label == Label::Final);
            if held.is_some() && (verified || shown) {
                Some(Label::Final)
            } else if bs.iter().any(|&b| self.binds[b].status == Status::Disputed) {
                Some(Label::Disputed)
            } else {
                Some(Label::Pending)
            }
        } else if bs.iter().any(|&b| self.binds[b].live.contains(k)) {
            let live_ok = LIVE_PROVISIONAL
                && self
                    .engine
                    .claim(k)
                    .is_some_and(|c| c.maturity == Maturity::Provisional)
                && bs.iter().all(|&b| self.binds[b].status == Status::Verified);
            Some(if live_ok {
                Label::Provisional
            } else {
                Label::Pending
            })
        } else {
            None
        };
        match label {
            None => {
                self.display.remove(k);
            }
            Some(label) => {
                let (values, evidence) = match (&held, label) {
                    (Some(c), Label::Final) => (c.values.clone(), c.evidence.clone()),
                    _ => (BTreeMap::new(), Vec::new()),
                };
                let item = DisplayItem {
                    key: k.clone(),
                    label,
                    shown_values: values,
                    evidence,
                };
                if self.display.items.get(k) != Some(&item) {
                    self.display.put(item);
                }
            }
        }
    }

    /// Re-evaluates everything touched in this callback and sends disputes.
    fn settle_display(&mut self, ctx: &mut Ctx) {
        let bs = std::mem::take(&mut self.touched_bindings);
        for b in bs {
            for k in self.keys_of_binding(b) {
                self.touched.insert(k);
            }
        }
        let ks = std::mem::take(&mut self.touched);
        for k in &ks {
            self.evaluate(k);
        }
        if !self.disputes.is_empty() {
            let m = RMsg {
                d: std::mem::take(&mut self.disputes),
            };
            ctx.send(serde_json::to_vec(&m).expect("json"));
        }
    }

    fn take_engine_deltas(&mut self) {
        for d in self.engine.take_deltas() {
            self.touched.insert(d.key);
        }
    }
}

impl ReplicaAdapter for Replica {
    fn on_source(&mut self, ctx: &mut Ctx, ev: &SourceEvent) {
        let Some(b) = self.feeds.iter().position(|f| f == ev.feed()) else {
            return;
        };
        let old = match ev {
            SourceEvent::Snapshot { .. } => Some(self.engine.bars(b).to_vec()),
            SourceEvent::Data(_) => None,
        };
        let applied = self.engine.apply(ev);
        let (changed, closed) = match &old {
            Some(old) => {
                let d = first_diff(old, self.engine.bars(b));
                (d, d.is_some())
            }
            None => (
                applied.changed_from_t,
                closed_change(&applied, self.engine.bars(b)).is_some(),
            ),
        };
        if let Some(t) = changed {
            self.on_data_change(ctx, b, t, closed, false);
            let rb = &self.binds[b];
            let relevant = applied.new_bar
                || old.is_some()
                || rb.status != Status::Verified
                || rb.seal.is_some_and(|s| s.w >= t);
            if relevant && rb.seal.is_some() {
                self.verify(ctx, b);
                self.touched_bindings.insert(b);
            }
        }
        self.take_engine_deltas();
        self.settle_display(ctx);
    }

    fn on_authority_msg(&mut self, ctx: &mut Ctx, msg: &[u8]) {
        let Ok(m) = serde_json::from_slice::<AMsg>(msg) else {
            return;
        };
        for (b, w, h, e, a, s) in &m.p {
            let Some(rb) = self.binds.get_mut(*b) else {
                continue;
            };
            if m.s <= rb.seal_seq {
                continue;
            }
            let Ok(h) = u64::from_str_radix(h, 16) else {
                continue;
            };
            let seal = Seal {
                w: *w,
                h,
                e: *e,
                a: *a,
                settled: *s == 1,
            };
            let moved = rb
                .seal
                .is_none_or(|x| x.e != seal.e || x.w != seal.w || x.settled != seal.settled);
            rb.seal_seq = m.s;
            rb.seal = Some(seal);
            if moved {
                rb.level = 0;
                rb.hold_until = None;
            }
            self.touched_bindings.insert(*b);
            self.verify(ctx, *b);
        }
        for (wk, present) in m
            .f
            .iter()
            .map(|k| (k, true))
            .chain(m.x.iter().map(|k| (k, false)))
        {
            let Some(k) = self.codec.decode(wk) else {
                continue;
            };
            let e = self.sealed.entry(k.clone()).or_insert((0, false));
            if m.s > e.0 {
                *e = (m.s, present);
                self.touched.insert(k);
            }
        }
        for (b, keys) in &m.l {
            let Some(rb) = self.binds.get(*b) else {
                continue;
            };
            if m.s <= rb.live_seq {
                continue;
            }
            let new: BTreeSet<ClaimKey> = keys
                .iter()
                .filter_map(|(kind, t)| self.codec.decode(&(kind.clone(), *b, *t)))
                .collect();
            let rb = &mut self.binds[*b];
            rb.live_seq = m.s;
            let old = std::mem::replace(&mut rb.live, new);
            self.touched.extend(old);
            self.touched.extend(self.binds[*b].live.iter().cloned());
        }
        self.settle_display(ctx);
    }

    fn on_timer(&mut self, ctx: &mut Ctx, token: u64) {
        let b = (token & TIMER_MASK) as usize;
        match token & !TIMER_MASK {
            TIMER_HOLD => {
                if self.binds[b].hold_until.is_some_and(|u| u <= ctx.now()) {
                    self.binds[b].hold_until = None;
                    if self.binds[b].status != Status::Verified {
                        self.verify(ctx, b);
                    }
                }
            }
            TIMER_PREFETCH => {
                if self.binds[b].prefetch_at.is_some_and(|x| x <= ctx.now()) {
                    self.binds[b].prefetch_at = None;
                    if self.binds[b].status != Status::Verified && self.binds[b].fetch.is_none()
                    {
                        if let Some(seal) = self.binds[b].seal {
                            let (from, to) = self.fetch_range(b, seal);
                            let id = ctx.fetch_settled(b, from, to);
                            let gen = self.binds[b].gen;
                            self.binds[b].fetch = Some((id, gen, false));
                        }
                    }
                }
            }
            _ => {}
        }
        self.settle_display(ctx);
    }

    fn on_fetch(&mut self, ctx: &mut Ctx, reply: &FetchReply) {
        let b = reply.binding;
        let Some((id, gen, counted)) = self.binds[b].fetch else {
            return;
        };
        if id != reply.id {
            return;
        }
        self.binds[b].fetch = None;
        if gen == self.binds[b].gen {
            let first = self.engine.bars(b).first().map(|x| x.t);
            let extends = reply.bars.first().is_some_and(|x| first.is_none_or(|f| x.t < f));
            if extends || reply.bars.len() > MERGE_AS_SNAPSHOT {
                // Many bars or older history (anchor alignment): merge into one snapshot
                // of our own data so the engine recomputes once, not once per bar.
                let mut merged: BTreeMap<i64, Bar> = self
                    .engine
                    .bars(b)
                    .iter()
                    .map(|x| (x.t, x.clone()))
                    .collect();
                for x in &reply.bars {
                    merged.insert(x.t, x.clone());
                }
                let old = self.engine.bars(b).to_vec();
                let snap = SourceEvent::Snapshot {
                    feed: self.feeds[b].clone(),
                    seq: self.engine.last_seq(b),
                    bars: merged.into_values().collect(),
                };
                self.engine.apply(&snap);
                if let Some(t) = first_diff(&old, self.engine.bars(b)) {
                    self.on_data_change(ctx, b, t, true, true);
                }
            } else {
                let applied = self.engine.apply_settled(b, &reply.bars);
                if let Some(t) = applied.changed_from_t {
                    self.on_data_change(ctx, b, t, true, true);
                }
            }
            self.take_engine_deltas();
        } else if counted {
            // A correction reached our feed while the request was in flight; the answer
            // may predate it. Do not count the attempt.
            self.binds[b].level = self.binds[b].level.saturating_sub(1);
        }
        self.verify(ctx, b);
        self.touched_bindings.insert(b);
        self.settle_display(ctx);
    }

    fn on_channel(&mut self, _ctx: &mut Ctx, _ev: ChannelEvent) {}

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

/// Registry entry.
#[must_use]
pub fn register() -> Approach {
    Approach {
        name: "a",
        summary: "data position stamp: prefix digest at a certified position; replica shows only when its data reaches it with the same digest",
        authority: |s: &Setup| Box::new(Authority::new(s)),
        replica: |s: &Setup| Box::new(Replica::new(s)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qp_schema::data::data_digest_u64;

    fn bar(t: i64, c: f64) -> Bar {
        Bar {
            t,
            o: c,
            h: c + 0.5,
            l: c - 0.5,
            c,
            v: 10.0,
        }
    }

    #[test]
    fn chain_equals_data_digest_of_prefix() {
        let bars: Vec<Bar> = (0..50)
            .map(|i| bar(1_000 + i * 60_000, 100.0 + f64::from(i as i32) * 0.37))
            .collect();
        let mut ch = Chain::default();
        for i in 0..bars.len() {
            assert_eq!(ch.at(&bars, i).0, data_digest_u64(&bars[..=i]));
        }
        ch.invalidate(10);
        let mut changed = bars.clone();
        changed[10].c += 0.01;
        assert_eq!(ch.at(&changed, 49).0, data_digest_u64(&changed));
        assert_ne!(data_digest_u64(&changed), data_digest_u64(&bars));
        assert_eq!(first_diff(&bars, &changed), Some(changed[10].t));
        assert_eq!(first_diff(&bars, &bars), None);
    }

    #[test]
    fn key_codec_round_trips_every_detector() {
        let mut spec = crate::spec::DetectorSpec {
            pair: true,
            ..Default::default()
        };
        spec.invalidation = Some(crate::spec::InvalidationSpec::default());
        let cfg = DetectorCfg::new(spec);
        let ids = vec!["px0".to_owned(), "px1".to_owned()];
        let codec = Codec::new(ids.clone(), &cfg);
        // Keys as the detector itself builds them, from a series that crosses.
        let mut s = crate::series::SeriesState::new(14, 0);
        let mut bars = Vec::new();
        let mut p = 100.0;
        for i in 0..400_i64 {
            let phase = (i / 15) % 2;
            p += if phase == 0 { 0.9 } else { -0.9 } + if i % 3 == 0 { -0.4 } else { 0.2 };
            bars.push(bar(BASE_T + i * 60_000, p));
        }
        s.set_bars(bars);
        let series = [&s, &s];
        let claims = cfg.claims_batch(&ids, &series);
        let kinds: BTreeSet<&str> = claims.keys().map(Codec::kind).collect();
        assert!(kinds.contains("u") && kinds.contains("d") && kinds.contains("i"));
        for k in claims.keys() {
            assert_eq!(codec.decode(&codec.encode(k)).as_ref(), Some(k), "{}", k.text());
        }
    }

    #[test]
    fn approach_a_has_no_errors_on_basic_scenarios() {
        let a = register();
        for name in ["S1", "S6", "S9", "S15"] {
            let spec = crate::scenarios::find(name).expect("preset");
            for seed in 0..3 {
                let m = crate::sim::run_one(&spec, &a, seed).metrics;
                assert_eq!(m.errors(), 0, "{name} seed {seed}");
            }
        }
    }
}
