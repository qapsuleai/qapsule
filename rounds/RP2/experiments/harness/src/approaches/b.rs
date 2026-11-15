//! Approach B — evidence values (`rounds/RP2/RESEARCH.md` §3.B), built as its strongest
//! advocate would. Design and results: `rounds/RP2/experiments/T7b.md`.
//!
//! **Authority.** Runs the shared engine on its own feed. For every claim it sends the
//! claim's *evidence values*: the RSI points the claim rests on (`[series, t, value]` at
//! the bar before the claim bar and at the claim bar, per binding), the bar state
//! (`provisional` / `final`), its calculation start (`anchor`: first bar time per
//! binding) and `ok` (its own data up to the claim bar has no unrepaired sequence gap).
//! Every op carries a monotonic `n`; the replica keeps the newest op per claim (per-key
//! last-writer-wins: reordering and duplicates on the channel are harmless). Market data
//! never travels on the channel (K23): only these few meaning-carrying values.
//!
//! **Replica.** Shows a claim only when the authority asserts it *and* its own data
//! agrees: the evidence bars are present (and closed for a final claim), the claim's
//! predicate holds on its own data (variant `Predicate`; `Strict`: the evidence values
//! are bit-equal), and the data is trustworthy (its own data or the authority's has no
//! unrepaired gap up to the claim bar). Shown values are always the replica's own (the
//! evidence on screen never contradicts the claim). While waiting: `Pending`. If its
//! data disagrees: refetch its own settled bars (`ctx.fetch_settled`), compare again,
//! then `Disputed` (+ `dispute` to the authority, which refetches and corrects itself).
//! A local final claim the authority does not assert triggers a `check`.
//!
//! **Data hygiene** (both sides, `docs/PROTOCOL.md` §18.4: gap filling is the host's
//! job): sequence gaps that may have cost a bar its closing value mark that bar
//! *suspect*; suspect bars are refetched from the settled source. The authority also
//! verifies every closed bar against the settled source. The replica fills its own data
//! from the settled source when its feed lags far behind a claim (`fill`) and extends its
//! history back to the authority's calculation start (`anchor`).

use std::collections::{BTreeMap, BTreeSet};

use qp_schema::data::{DataChange, FeedKey};
use qp_schema::Bar;
use serde::{Deserialize, Serialize};

use crate::adapter::{
    Approach, AuthorityAdapter, Ctx, FetchReply, ReplicaAdapter, Setup, SourceEvent,
};
use crate::detector::{ClaimKey, Direction, Maturity};
use crate::display::{DisplayItem, DisplayState, Label};
use crate::engine::Engine;
use crate::spec::VTime;

const MS: VTime = 1_000;
/// First repair attempt after a suspect bar is found (the settled source needs the bar
/// to be closed for a while; a missing bar in the answer is simply retried).
const REPAIR_FIRST: VTime = 2_000 * MS;
/// Retry interval for settled-bar requests whose bar was not settled yet.
const RETRY: VTime = 1_000 * MS;
const MAX_RETRY: u32 = 20;
/// Authority: verify a closed bar against the settled source this long after it closed.
const VERIFY_DELAY: VTime = 2_500 * MS;
/// Replica: wait this long for its own feed before filling from the settled source.
const FILL_WAIT: VTime = 3_000 * MS;
/// Replica: poll interval while a claim keeps waiting for data.
const FILL_POLL: VTime = 5_000 * MS;
const MAX_POLL: u32 = 40;
/// Replica: a local final claim the authority has not asserted after this long → `check`.
const OMIT_WAIT: VTime = 3_000 * MS;
const MAX_MISSING: u64 = 100_000;

// ---------------------------------------------------------------------------------
// Configuration (variants)
// ---------------------------------------------------------------------------------

/// How the replica compares the authority's evidence with its own computation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compare {
    /// B-predicate: the claim's predicate holds on the replica's own data.
    Predicate,
    /// B-strict: the evidence values are bit-equal to the replica's own values.
    Strict,
}

/// What the replica shows before its own data has closed the claim's bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Prov {
    /// Nothing asserted before the bar is closed on the replica's data.
    Off,
    /// `Provisional` only for a claim the authority states as final (on clean data) whose
    /// bar the replica holds with settled values but has not closed on its chart yet
    /// (its feed lags). Live-bar claims are not shown.
    Settled,
    /// `Settled`, plus live-bar claims: `Provisional` while the authority asserts the
    /// claim and the replica's own live bar satisfies it.
    Live,
}

/// Variant configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cfg {
    /// Comparison.
    pub compare: Compare,
    /// Provisional policy.
    pub prov: Prov,
    /// Gap tracking, suspect-bar repair, authority verification, trust rule.
    pub hygiene: bool,
    /// Replica refetch on disagreement, `dispute` / `check` messages.
    pub disputes: bool,
    /// Replica fill-ahead from the settled source and anchor extension.
    pub fill: bool,
}

/// The chosen "B" (see T7b.md for why).
pub const BEST: Cfg = Cfg {
    compare: Compare::Predicate,
    prov: Prov::Settled,
    hygiene: true,
    disputes: true,
    fill: true,
};

// ---------------------------------------------------------------------------------
// Wire format
// ---------------------------------------------------------------------------------

/// An evidence point: `[series, bar t, value]` of the claim's study output (the RSI of
/// the detector's `period`).
type Pt = (String, i64, f64);

/// Authority → replica op (a message is a JSON array of ops).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
enum AOp {
    /// The authority asserts a claim.
    Claim {
        /// Op number (monotonic per authority).
        n: u64,
        /// Claim identity by meaning (`ClaimKey::text`).
        k: String,
        /// Bar state on the authority's data.
        m: Maturity,
        /// The authority's data up to the claim bar has no unrepaired gap.
        ok: bool,
        /// Calculation start (first bar time) per binding.
        anchor: BTreeMap<String, i64>,
        /// Evidence values.
        pts: Vec<Pt>,
    },
    /// The authority no longer asserts the claim.
    Retract {
        /// Op number.
        n: u64,
        /// Claim identity.
        k: String,
    },
}

/// Replica → authority op.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
enum ROp {
    /// "My (clean) data holds this final claim; you do not assert it."
    Check { k: String },
    /// "My (refetched) data contradicts your claim."
    Dispute { k: String },
}

fn parse_key(s: &str) -> Option<ClaimKey> {
    let (detector, rest) = s.split_once('[')?;
    let (params, rest) = rest.split_once("]@")?;
    let mut it = rest.rsplitn(3, ':');
    let dir = it.next()?;
    let t = it.next()?;
    let binding = it.next()?;
    Some(ClaimKey {
        detector: detector.to_owned(),
        params: params.to_owned(),
        binding: binding.to_owned(),
        bar_t: t.parse().ok()?,
        direction: match dir {
            "up" => Direction::Up,
            "down" => Direction::Down,
            _ => return None,
        },
    })
}

fn change_t(c: &DataChange) -> i64 {
    match c {
        DataChange::Bar { bar } => bar.t,
        DataChange::Patch { t, .. } => *t,
    }
}

// ---------------------------------------------------------------------------------
// Data hygiene: sequence gaps → suspect bars; bars written from the settled source
// ---------------------------------------------------------------------------------

#[derive(Debug, Default)]
struct Hyg {
    started: bool,
    last: u64,
    missing: BTreeSet<u64>,
    /// Received seq → bar time.
    seq_t: BTreeMap<u64, i64>,
    /// Bar time → smallest received seq.
    first_seq: BTreeMap<i64, u64>,
    /// Bars written from the settled source (trusted; the feed's stale ticks for them
    /// are not applied).
    rest: BTreeSet<i64>,
    /// Suspect bars with a repair requested.
    requested: BTreeSet<i64>,
    /// Unrepaired suspect bars: a missing seq lies between the last received event of
    /// the bar and the first received event of another bar (the bar's closing value may
    /// be lost).
    suspects: BTreeSet<i64>,
    repair_armed: bool,
}

impl Hyg {
    /// Accepts a snapshot unless it is older than what was applied (a delayed duplicate).
    fn on_snapshot(&mut self, seq: u64, bars: &[Bar]) -> bool {
        if self.started && seq < self.last {
            return false;
        }
        *self = Self {
            started: true,
            last: seq,
            ..Self::default()
        };
        if let Some(b) = bars.last() {
            self.seq_t.insert(seq, b.t);
        }
        true
    }

    /// Records a data event. Returns (apply it to the engine, gap state changed).
    fn on_data(&mut self, seq: u64, t: i64) -> (bool, bool) {
        let mut changed = false;
        if seq <= self.last {
            if !self.missing.remove(&seq) {
                return (true, false); // repeat: the engine ignores it
            }
            changed = true;
        } else {
            if seq > self.last + 1 {
                let from = (self.last + 1).max(seq.saturating_sub(MAX_MISSING));
                self.missing.extend(from..seq);
                changed = true;
            }
            self.last = seq;
        }
        // A tick of a bar that was already written from the settled source, delivered
        // before the feed moved past that bar, is stale (the feed lags); corrections come
        // after later bars started and are applied.
        let stale = self.rest.contains(&t)
            && self
                .first_seq
                .range(t + 1..)
                .map(|(_, s)| *s)
                .min()
                .is_none_or(|s| seq < s);
        self.seq_t.insert(seq, t);
        let e = self.first_seq.entry(t).or_insert(seq);
        *e = (*e).min(seq);
        if changed {
            self.recompute();
        }
        (!stale, changed)
    }

    fn recompute(&mut self) {
        let mut s = BTreeSet::new();
        for &m in &self.missing {
            let lo = self.seq_t.range(..m).next_back().map(|(_, t)| *t);
            let hi = self.seq_t.range(m + 1..).next().map(|(_, t)| *t);
            if let (Some(lo), Some(hi)) = (lo, hi) {
                if lo != hi && !self.rest.contains(&lo) {
                    s.insert(lo);
                }
            }
        }
        self.suspects = s;
    }

    fn mark_rest(&mut self, t: i64) {
        self.rest.insert(t);
        self.requested.remove(&t);
        self.suspects.remove(&t);
    }

    fn clean_through(&self, t: i64) -> bool {
        self.suspects.range(..=t).next().is_none()
    }

    /// Suspects without a repair request yet (marks them requested).
    fn take_new(&mut self) -> bool {
        let new: Vec<i64> = self.suspects.difference(&self.requested).copied().collect();
        self.requested.extend(new.iter().copied());
        !new.is_empty()
    }

    fn request_range(&self) -> Option<(i64, i64)> {
        Some((*self.requested.first()?, *self.requested.last()?))
    }
}

fn binding_of(feeds: &[FeedKey], feed: &FeedKey) -> Option<usize> {
    feeds.iter().position(|f| f == feed)
}

/// Bulk threshold: at least this many changed bars are applied as one replacement of the
/// binding's bars (one recomputation) instead of bar by bar (`Engine::apply_settled`
/// recomputes from every inserted or changed old bar).
const BULK: usize = 8;

/// Applies settled bars and marks them trusted. Returns the earliest bar time touched.
fn apply_settled(
    engine: &mut Engine,
    hyg: &mut Hyg,
    feed: &FeedKey,
    b: usize,
    bars: &[Bar],
) -> Option<i64> {
    if bars.is_empty() {
        return None;
    }
    let changed = bars
        .iter()
        .filter(|x| {
            let s = engine.series(b);
            s.index_of(x.t).map(|i| &s.bars()[i]) != Some(*x)
        })
        .count();
    if changed >= BULK {
        // Own bars with the settled ones replacing or inserted by time; the engine's
        // snapshot path (one rebuild). The sequence position stays where it is.
        let mut merged: BTreeMap<i64, Bar> =
            engine.bars(b).iter().map(|x| (x.t, x.clone())).collect();
        for x in bars {
            merged.insert(x.t, x.clone());
        }
        let seq = engine.last_seq(b);
        engine.apply(&SourceEvent::Snapshot {
            feed: feed.clone(),
            seq,
            bars: merged.into_values().collect(),
        });
    } else if changed > 0 {
        engine.apply_settled(b, bars);
    }
    for x in bars {
        hyg.mark_rest(x.t);
    }
    engine.add_work(bars.len() as u64);
    bars.first().map(|x| x.t)
}

// ---------------------------------------------------------------------------------
// Authority
// ---------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
struct Sent {
    m: Maturity,
    ok: bool,
    pts: Vec<Pt>,
}

#[derive(Debug, Clone, Copy)]
enum ATimer {
    Repair(usize, u32),
    Verify(usize, i64, u32),
}

#[derive(Debug, Clone, Copy)]
enum AFetch {
    Repair(u32),
    Verify(i64, u32),
    Dispute,
}

struct Authority {
    cfg: Cfg,
    engine: Engine,
    feeds: Vec<FeedKey>,
    hyg: Vec<Hyg>,
    n: u64,
    sent: BTreeMap<ClaimKey, Sent>,
    timers: BTreeMap<u64, ATimer>,
    fetches: BTreeMap<u64, AFetch>,
    next_token: u64,
    /// Bars up to this time are verified against (or came from) a clean source.
    verified: Vec<i64>,
    dispute_inflight: Vec<bool>,
    dirty: Vec<bool>,
}

impl Authority {
    fn new(s: &Setup, cfg: Cfg) -> Self {
        let nb = s.bindings.len();
        Self {
            cfg,
            engine: Engine::new(s),
            feeds: s.bindings.iter().map(|b| b.feed.clone()).collect(),
            hyg: (0..nb).map(|_| Hyg::default()).collect(),
            n: 0,
            sent: BTreeMap::new(),
            timers: BTreeMap::new(),
            fetches: BTreeMap::new(),
            next_token: 1,
            verified: vec![i64::MIN; nb],
            dispute_inflight: vec![false; nb],
            dirty: vec![false; nb],
        }
    }

    fn timer(&mut self, ctx: &mut Ctx, at: VTime, t: ATimer) {
        let tok = self.next_token;
        self.next_token += 1;
        self.timers.insert(tok, t);
        ctx.set_timer(at, tok);
    }

    fn ok(&self, key: &ClaimKey) -> bool {
        key.bindings().iter().all(|id| {
            self.engine
                .index(id)
                .is_some_and(|b| self.hyg[b].clean_through(key.bar_t))
        })
    }

    fn pts(&self, key: &ClaimKey) -> Option<Vec<Pt>> {
        let mut v = Vec::new();
        for id in key.bindings() {
            let b = self.engine.index(id)?;
            let s = self.engine.series(b);
            let i = s.index_of(key.bar_t)?;
            if i == 0 {
                return None;
            }
            v.push(((*id).to_owned(), s.bars()[i - 1].t, s.rsi()[i - 1]?));
            v.push(((*id).to_owned(), key.bar_t, s.rsi()[i]?));
        }
        Some(v)
    }

    fn anchor(&self, key: &ClaimKey) -> BTreeMap<String, i64> {
        key.bindings()
            .iter()
            .filter_map(|id| {
                let b = self.engine.index(id)?;
                Some(((*id).to_owned(), self.engine.bars(b).first()?.t))
            })
            .collect()
    }

    fn desired(&self, key: &ClaimKey) -> Option<Sent> {
        let c = self.engine.claim(key)?;
        if self.cfg.prov != Prov::Live && c.maturity != Maturity::Final {
            return None;
        }
        Some(Sent {
            m: c.maturity,
            ok: !self.cfg.hygiene || self.ok(key),
            pts: self.pts(key)?,
        })
    }

    fn publish(&mut self, ctx: &mut Ctx) {
        let mut keys: BTreeSet<ClaimKey> = self
            .engine
            .take_deltas()
            .into_iter()
            .map(|d| d.key)
            .collect();
        if self.dirty.iter().any(|d| *d) {
            let ids: Vec<String> = self
                .engine
                .ids()
                .iter()
                .enumerate()
                .filter(|(b, _)| self.dirty[*b])
                .map(|(_, id)| id.clone())
                .collect();
            let touches = |k: &ClaimKey| k.bindings().iter().any(|x| ids.iter().any(|i| i == x));
            keys.extend(self.sent.keys().filter(|k| touches(k)).cloned());
            keys.extend(self.engine.claims().keys().filter(|k| touches(k)).cloned());
            self.dirty.iter_mut().for_each(|d| *d = false);
        }
        let mut ops = Vec::new();
        let strict = self.cfg.compare == Compare::Strict;
        for k in keys {
            let want = self.desired(&k);
            match (want, self.sent.get(&k)) {
                (Some(w), cur) => {
                    let send = cur.is_none_or(|c| {
                        c.m != w.m
                            || c.ok != w.ok
                            || ((w.m == Maturity::Final || strict) && c.pts != w.pts)
                    });
                    if send {
                        self.n += 1;
                        ops.push(AOp::Claim {
                            n: self.n,
                            k: k.text(),
                            m: w.m,
                            ok: w.ok,
                            anchor: self.anchor(&k),
                            pts: w.pts.clone(),
                        });
                        self.sent.insert(k, w);
                    }
                }
                (None, Some(_)) => {
                    self.n += 1;
                    ops.push(AOp::Retract {
                        n: self.n,
                        k: k.text(),
                    });
                    self.sent.remove(&k);
                }
                (None, None) => {}
            }
        }
        self.engine.add_work(ops.len() as u64);
        if !ops.is_empty() {
            ctx.send(serde_json::to_vec(&ops).expect("json"));
        }
    }

    fn after_gap_change(&mut self, ctx: &mut Ctx, b: usize) {
        self.dirty[b] = true;
        if self.cfg.hygiene && self.hyg[b].take_new() && !self.hyg[b].repair_armed {
            self.hyg[b].repair_armed = true;
            let at = ctx.now() + REPAIR_FIRST;
            self.timer(ctx, at, ATimer::Repair(b, 0));
        }
    }
}

impl AuthorityAdapter for Authority {
    fn on_source(&mut self, ctx: &mut Ctx, ev: &SourceEvent) {
        let Some(b) = binding_of(&self.feeds, ev.feed()) else {
            return;
        };
        match ev {
            SourceEvent::Snapshot { seq, bars, .. } => {
                if !self.hyg[b].on_snapshot(*seq, bars) {
                    return;
                }
                self.engine.apply(ev);
                let n = bars.len();
                self.verified[b] = if n >= 2 { bars[n - 2].t } else { i64::MIN };
                self.after_gap_change(ctx, b);
            }
            SourceEvent::Data(d) => {
                let (pass, changed) = self.hyg[b].on_data(d.seq, change_t(&d.change));
                if pass {
                    let a = self.engine.apply(ev);
                    if a.new_bar && self.cfg.hygiene {
                        let bars = self.engine.bars(b);
                        if bars.len() >= 2 {
                            let closed = bars[bars.len() - 2].t;
                            let at = ctx.now() + VERIFY_DELAY;
                            self.timer(ctx, at, ATimer::Verify(b, closed, 0));
                        }
                    }
                }
                if changed {
                    self.after_gap_change(ctx, b);
                }
            }
        }
        self.publish(ctx);
    }

    fn on_replica_msg(&mut self, ctx: &mut Ctx, msg: &[u8]) {
        if !self.cfg.disputes {
            return;
        }
        let Ok(ops) = serde_json::from_slice::<Vec<ROp>>(msg) else {
            return;
        };
        for op in ops {
            let (ROp::Check { k } | ROp::Dispute { k }) = op;
            let Some(key) = parse_key(&k) else { continue };
            for id in key.bindings() {
                let Some(b) = self.engine.index(id) else {
                    continue;
                };
                if self.dispute_inflight[b] {
                    continue;
                }
                let from = self.engine.bars(b).first().map_or(key.bar_t, |x| x.t);
                let fid = ctx.fetch_settled(b, from, key.bar_t);
                self.fetches.insert(fid, AFetch::Dispute);
                self.dispute_inflight[b] = true;
            }
        }
    }

    fn on_timer(&mut self, ctx: &mut Ctx, token: u64) {
        let Some(t) = self.timers.remove(&token) else {
            return;
        };
        match t {
            ATimer::Repair(b, att) => {
                self.hyg[b].repair_armed = false;
                if let Some((from, to)) = self.hyg[b].request_range() {
                    let id = ctx.fetch_settled(b, from, to);
                    self.fetches.insert(id, AFetch::Repair(att));
                }
            }
            ATimer::Verify(b, to, att) => {
                if self.verified[b] >= to {
                    return;
                }
                let from = if self.verified[b] == i64::MIN {
                    to
                } else {
                    self.verified[b] + 1
                };
                let id = ctx.fetch_settled(b, from, to);
                self.fetches.insert(id, AFetch::Verify(to, att));
            }
        }
    }

    fn on_fetch(&mut self, ctx: &mut Ctx, reply: &FetchReply) {
        let Some(f) = self.fetches.remove(&reply.id) else {
            return;
        };
        let b = reply.binding;
        apply_settled(&mut self.engine, &mut self.hyg[b], &self.feeds[b], b, &reply.bars);
        self.dirty[b] = true;
        match f {
            AFetch::Repair(att) => {
                if !self.hyg[b].requested.is_empty()
                    && att < MAX_RETRY
                    && !self.hyg[b].repair_armed
                {
                    self.hyg[b].repair_armed = true;
                    let at = ctx.now() + RETRY;
                    self.timer(ctx, at, ATimer::Repair(b, att + 1));
                }
            }
            AFetch::Verify(to, att) => {
                if reply.bars.iter().any(|x| x.t == to) {
                    self.verified[b] = self.verified[b].max(to);
                } else if att < MAX_RETRY {
                    let at = ctx.now() + RETRY;
                    self.timer(ctx, at, ATimer::Verify(b, to, att + 1));
                }
            }
            AFetch::Dispute => self.dispute_inflight[b] = false,
        }
        self.publish(ctx);
    }

    fn work_units(&self) -> u64 {
        self.engine.work()
    }
}

// ---------------------------------------------------------------------------------
// Replica
// ---------------------------------------------------------------------------------

#[derive(Debug, Clone)]
enum AState {
    Claim {
        n: u64,
        m: Maturity,
        ok: bool,
        anchor: BTreeMap<String, i64>,
        pts: Vec<Pt>,
    },
    Retracted {
        n: u64,
    },
}

impl AState {
    fn n(&self) -> u64 {
        match self {
            Self::Claim { n, .. } | Self::Retracted { n } => *n,
        }
    }
}

#[derive(Debug, Default, Clone)]
struct KState {
    /// The authority op `n` for which the replica's refetch-and-compare is complete.
    verified_n: Option<u64>,
    verify_pending: usize,
    verify_all: bool,
    verify_tries: u32,
    disputed_n: Option<u64>,
    fill_armed: bool,
    polls: u32,
    omit_armed: bool,
    check_sent: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ext {
    Idle,
    InFlight,
    Done,
    Failed,
}

#[derive(Debug, Clone)]
enum RTimer {
    Repair(usize, u32),
    Fill(ClaimKey),
    Omit(ClaimKey),
    VerifyRetry(ClaimKey),
}

#[derive(Debug, Clone)]
enum RFetch {
    Repair(u32),
    Extend(i64),
    Verify(ClaimKey),
    Fill,
}

struct Replica {
    cfg: Cfg,
    engine: Engine,
    feeds: Vec<FeedKey>,
    interval: Vec<i64>,
    hyg: Vec<Hyg>,
    display: DisplayState,
    auth: BTreeMap<ClaimKey, AState>,
    by_b: Vec<BTreeSet<ClaimKey>>,
    st: BTreeMap<ClaimKey, KState>,
    waiting: Vec<BTreeSet<ClaimKey>>,
    ext: Vec<Ext>,
    timers: BTreeMap<u64, RTimer>,
    fetches: BTreeMap<u64, RFetch>,
    next_token: u64,
}

impl Replica {
    fn new(s: &Setup, cfg: Cfg) -> Self {
        let nb = s.bindings.len();
        Self {
            cfg,
            engine: Engine::new(s),
            feeds: s.bindings.iter().map(|b| b.feed.clone()).collect(),
            interval: s.bindings.iter().map(|b| b.interval_ms).collect(),
            hyg: (0..nb).map(|_| Hyg::default()).collect(),
            display: DisplayState::default(),
            auth: BTreeMap::new(),
            by_b: vec![BTreeSet::new(); nb],
            st: BTreeMap::new(),
            waiting: vec![BTreeSet::new(); nb],
            ext: vec![Ext::Idle; nb],
            timers: BTreeMap::new(),
            fetches: BTreeMap::new(),
            next_token: 1,
        }
    }

    fn timer(&mut self, ctx: &mut Ctx, at: VTime, t: RTimer) {
        let tok = self.next_token;
        self.next_token += 1;
        self.timers.insert(tok, t);
        ctx.set_timer(at, tok);
    }

    fn bindings(&self, key: &ClaimKey) -> Option<Vec<usize>> {
        key.bindings()
            .iter()
            .map(|id| self.engine.index(id))
            .collect()
    }

    fn pts_equal(&self, pts: &[Pt]) -> bool {
        pts.chunks(2).all(|w| {
            let [(s, tp, vp), (_, t, v)] = w else {
                return false;
            };
            let Some(b) = self.engine.index(s) else {
                return false;
            };
            let ser = self.engine.series(b);
            let Some(i) = ser.index_of(*t) else {
                return false;
            };
            i > 0
                && ser.bars()[i - 1].t == *tp
                && ser.rsi()[i - 1].map(f64::to_bits) == Some(vp.to_bits())
                && ser.rsi()[i].map(f64::to_bits) == Some(v.to_bits())
        })
    }

    fn lclean(&self, bs: &[usize], t: i64) -> bool {
        !self.cfg.hygiene || bs.iter().all(|&b| self.hyg[b].clean_through(t))
    }

    fn unwait(&mut self, key: &ClaimKey, bs: &[usize]) {
        for &b in bs {
            self.waiting[b].remove(key);
        }
        if let Some(s) = self.st.get_mut(key) {
            s.fill_armed = false;
            s.polls = 0;
        }
    }

    fn request_verify(&mut self, ctx: &mut Ctx, key: &ClaimKey, bs: &[usize]) {
        if !self.cfg.disputes {
            return;
        }
        let st = self.st.entry(key.clone()).or_default();
        if st.verify_pending > 0 {
            return;
        }
        st.verify_pending = bs.len();
        st.verify_all = true;
        for &b in bs {
            let from = self.engine.bars(b).first().map_or(key.bar_t, |x| x.t);
            let id = ctx.fetch_settled(b, from, key.bar_t);
            self.fetches.insert(id, RFetch::Verify(key.clone()));
        }
    }

    fn maybe_extend(&mut self, ctx: &mut Ctx, anchor: &BTreeMap<String, i64>, bs: &[usize]) -> bool {
        if !self.cfg.fill {
            return false;
        }
        let mut wait = false;
        for &b in bs {
            let id = self.engine.ids()[b].clone();
            let Some(&a) = anchor.get(&id) else { continue };
            match self.ext[b] {
                Ext::InFlight => wait = true,
                Ext::Failed => {}
                Ext::Idle | Ext::Done => {
                    if let Some(first) = self.engine.bars(b).first().map(|x| x.t) {
                        if a < first {
                            let fid = ctx.fetch_settled(b, a, first - 1);
                            self.fetches.insert(fid, RFetch::Extend(a));
                            self.ext[b] = Ext::InFlight;
                            wait = true;
                        }
                    }
                }
            }
        }
        wait
    }

    /// Decides what to show for one claim.
    #[allow(clippy::too_many_lines)]
    fn reconcile(&mut self, ctx: &mut Ctx, key: &ClaimKey) {
        self.engine.add_work(1);
        let Some(bs) = self.bindings(key) else {
            return;
        };
        let Some(AState::Claim {
            n,
            m,
            ok,
            anchor,
            pts,
        }) = self.auth.get(key).cloned()
        else {
            self.display.remove(key);
            self.unwait(key, &bs);
            return;
        };
        let ext_wait = self.maybe_extend(ctx, &anchor, &bs);
        let present = pts.iter().all(|(s, t, _)| {
            self.engine
                .index(s)
                .is_some_and(|b| self.engine.has_bar(b, *t))
        });
        let local = self.engine.claim(key).cloned();
        let lclean = self.lclean(&bs, key.bar_t);
        let trusted = !self.cfg.hygiene || lclean || ok;
        let closed_all = bs.iter().all(|&b| self.engine.is_closed(b, key.bar_t));
        let st = self.st.get(key).cloned().unwrap_or_default();
        let cur = self.display.items.get(key).map(|i| i.label);
        let mut wait = false;
        let label: Option<Label> = if ext_wait || !present {
            wait = true;
            Some(Label::Pending)
        } else if let Some(c) = &local {
            let cmp_ok = match self.cfg.compare {
                Compare::Predicate => true,
                Compare::Strict => self.pts_equal(&pts),
            };
            match (c.maturity, m) {
                (Maturity::Final, Maturity::Final) => {
                    if !trusted {
                        Some(Label::Pending)
                    } else if cmp_ok {
                        Some(Label::Final)
                    } else if st.verified_n == Some(n) || !self.cfg.disputes {
                        Some(Label::SourceDiffers)
                    } else {
                        self.request_verify(ctx, key, &bs);
                        if cur.is_some_and(Label::asserts) {
                            cur
                        } else {
                            Some(Label::Pending)
                        }
                    }
                }
                (Maturity::Final, Maturity::Provisional) => {
                    if self.cfg.prov == Prov::Live && cmp_ok {
                        Some(Label::Provisional)
                    } else {
                        Some(Label::Pending)
                    }
                }
                (Maturity::Provisional, Maturity::Final) => {
                    wait = true;
                    let settled_live = bs.iter().all(|&b| {
                        self.engine.is_closed(b, key.bar_t) || self.hyg[b].rest.contains(&key.bar_t)
                    });
                    // Settled: the replica holds the bar's settled values (its feed
                    // lags); Live: the replica's live bar satisfies the claim.
                    let settled_ok = self.cfg.prov != Prov::Off && ok && trusted && settled_live;
                    if cmp_ok && (settled_ok || self.cfg.prov == Prov::Live) {
                        Some(Label::Provisional)
                    } else {
                        Some(Label::Pending)
                    }
                }
                (Maturity::Provisional, Maturity::Provisional) => {
                    (self.cfg.prov == Prov::Live && cmp_ok).then_some(Label::Provisional)
                }
            }
        } else if m == Maturity::Provisional {
            None
        } else if !closed_all {
            wait = true;
            Some(Label::Pending)
        } else if !lclean {
            Some(Label::Pending)
        } else if !self.cfg.disputes {
            Some(Label::Disputed)
        } else if st.verified_n == Some(n) {
            if st.disputed_n != Some(n) {
                self.st.entry(key.clone()).or_default().disputed_n = Some(n);
                let op = vec![ROp::Dispute { k: key.text() }];
                ctx.send(serde_json::to_vec(&op).expect("json"));
            }
            Some(Label::Disputed)
        } else {
            self.request_verify(ctx, key, &bs);
            Some(Label::Pending)
        };
        match label {
            None => {
                self.display.remove(key);
            }
            Some(l) => {
                let item = match (&local, l.asserts()) {
                    (Some(c), true) => DisplayItem {
                        key: key.clone(),
                        label: l,
                        shown_values: c.values.clone(),
                        evidence: c.evidence.clone(),
                    },
                    _ => DisplayItem {
                        key: key.clone(),
                        label: l,
                        shown_values: pts
                            .iter()
                            .enumerate()
                            .map(|(i, p)| (format!("src{i}"), p.2))
                            .collect(),
                        evidence: Vec::new(),
                    },
                };
                if self.display.items.get(key) != Some(&item) {
                    self.display.put(item);
                }
            }
        }
        if wait {
            for &b in &bs {
                self.waiting[b].insert(key.clone());
            }
            let s = self.st.entry(key.clone()).or_default();
            if self.cfg.fill && !s.fill_armed {
                s.fill_armed = true;
                let at = ctx.now() + FILL_WAIT;
                self.timer(ctx, at, RTimer::Fill(key.clone()));
            }
        } else {
            self.unwait(key, &bs);
        }
    }

    /// After the data of binding `b` changed: reconcile what may be affected.
    fn after_change(&mut self, ctx: &mut Ctx, b: usize, all: bool) {
        let deltas = self.engine.take_deltas();
        let mut keys: BTreeSet<ClaimKey> = BTreeSet::new();
        for d in &deltas {
            let claimed = matches!(self.auth.get(&d.key), Some(AState::Claim { .. }));
            if claimed {
                keys.insert(d.key.clone());
            } else if self.cfg.disputes
                && d.after.as_ref().is_some_and(|c| c.maturity == Maturity::Final)
            {
                let s = self.st.entry(d.key.clone()).or_default();
                if !s.omit_armed && !s.check_sent {
                    s.omit_armed = true;
                    let at = ctx.now() + OMIT_WAIT;
                    self.timer(ctx, at, RTimer::Omit(d.key.clone()));
                }
            }
        }
        keys.extend(self.waiting[b].iter().cloned());
        if all {
            keys.extend(self.by_b[b].iter().cloned());
        }
        for k in &keys {
            self.reconcile(ctx, k);
        }
        if self.cfg.hygiene && self.hyg[b].take_new() && !self.hyg[b].repair_armed {
            self.hyg[b].repair_armed = true;
            let at = ctx.now() + REPAIR_FIRST;
            self.timer(ctx, at, RTimer::Repair(b, 0));
        }
    }
}

impl ReplicaAdapter for Replica {
    fn on_source(&mut self, ctx: &mut Ctx, ev: &SourceEvent) {
        let Some(b) = binding_of(&self.feeds, ev.feed()) else {
            return;
        };
        let all = match ev {
            SourceEvent::Snapshot { seq, bars, .. } => {
                if !self.hyg[b].on_snapshot(*seq, bars) {
                    return;
                }
                self.engine.apply(ev);
                if self.ext[b] == Ext::Done {
                    self.ext[b] = Ext::Idle;
                }
                true
            }
            SourceEvent::Data(d) => {
                let (pass, changed) = self.hyg[b].on_data(d.seq, change_t(&d.change));
                if pass {
                    self.engine.apply(ev);
                }
                changed
            }
        };
        self.after_change(ctx, b, all);
    }

    fn on_authority_msg(&mut self, ctx: &mut Ctx, msg: &[u8]) {
        let Ok(ops) = serde_json::from_slice::<Vec<AOp>>(msg) else {
            return;
        };
        for op in ops {
            let (n, k) = match &op {
                AOp::Claim { n, k, .. } | AOp::Retract { n, k } => (*n, k.as_str()),
            };
            let Some(key) = parse_key(k) else { continue };
            if self.auth.get(&key).is_some_and(|s| s.n() >= n) {
                continue; // older or repeated op
            }
            let Some(bs) = self.bindings(&key) else {
                continue;
            };
            let state = match op {
                AOp::Claim {
                    n,
                    m,
                    ok,
                    anchor,
                    pts,
                    ..
                } => {
                    for &b in &bs {
                        self.by_b[b].insert(key.clone());
                    }
                    AState::Claim {
                        n,
                        m,
                        ok,
                        anchor,
                        pts,
                    }
                }
                AOp::Retract { n, .. } => {
                    for &b in &bs {
                        self.by_b[b].remove(&key);
                    }
                    AState::Retracted { n }
                }
            };
            self.auth.insert(key.clone(), state);
            self.reconcile(ctx, &key);
        }
    }

    fn on_timer(&mut self, ctx: &mut Ctx, token: u64) {
        let Some(t) = self.timers.remove(&token) else {
            return;
        };
        match t {
            RTimer::Repair(b, att) => {
                self.hyg[b].repair_armed = false;
                if let Some((from, to)) = self.hyg[b].request_range() {
                    let id = ctx.fetch_settled(b, from, to);
                    self.fetches.insert(id, RFetch::Repair(att));
                }
            }
            RTimer::Fill(key) => {
                let Some(bs) = self.bindings(&key) else {
                    return;
                };
                let waiting = bs.iter().any(|&b| self.waiting[b].contains(&key));
                let claimed = matches!(self.auth.get(&key), Some(AState::Claim { .. }));
                if !waiting || !claimed {
                    if let Some(s) = self.st.get_mut(&key) {
                        s.fill_armed = false;
                    }
                    return;
                }
                for &b in &bs {
                    if self.engine.is_closed(b, key.bar_t) {
                        continue;
                    }
                    // No data yet (feed far behind): fill from the claim's calculation start.
                    let id_s = self.engine.ids()[b].clone();
                    let start = match self.auth.get(&key) {
                        Some(AState::Claim { anchor, .. }) => anchor.get(&id_s).copied(),
                        _ => None,
                    };
                    let from = self
                        .engine
                        .bars(b)
                        .last()
                        .map_or(start.unwrap_or(key.bar_t), |x| x.t.min(key.bar_t));
                    let id = ctx.fetch_settled(b, from, key.bar_t + self.interval[b]);
                    self.fetches.insert(id, RFetch::Fill);
                }
                let s = self.st.entry(key.clone()).or_default();
                s.polls += 1;
                if s.polls < MAX_POLL {
                    let at = ctx.now() + FILL_POLL;
                    self.timer(ctx, at, RTimer::Fill(key));
                } else {
                    s.fill_armed = false;
                }
            }
            RTimer::Omit(key) => {
                let s = self.st.entry(key.clone()).or_default();
                s.omit_armed = false;
                if s.check_sent {
                    return;
                }
                let claimed = matches!(self.auth.get(&key), Some(AState::Claim { .. }));
                let local_final = self
                    .engine
                    .claim(&key)
                    .is_some_and(|c| c.maturity == Maturity::Final);
                let Some(bs) = self.bindings(&key) else {
                    return;
                };
                if !claimed && local_final && self.lclean(&bs, key.bar_t) {
                    self.st.entry(key.clone()).or_default().check_sent = true;
                    let op = vec![ROp::Check { k: key.text() }];
                    ctx.send(serde_json::to_vec(&op).expect("json"));
                }
            }
            RTimer::VerifyRetry(key) => self.reconcile(ctx, &key),
        }
    }

    fn on_fetch(&mut self, ctx: &mut Ctx, reply: &FetchReply) {
        let Some(f) = self.fetches.remove(&reply.id) else {
            return;
        };
        let b = reply.binding;
        apply_settled(&mut self.engine, &mut self.hyg[b], &self.feeds[b], b, &reply.bars);
        match f {
            RFetch::Repair(att) => {
                if !self.hyg[b].requested.is_empty()
                    && att < MAX_RETRY
                    && !self.hyg[b].repair_armed
                {
                    self.hyg[b].repair_armed = true;
                    let at = ctx.now() + RETRY;
                    self.timer(ctx, at, RTimer::Repair(b, att + 1));
                }
            }
            RFetch::Extend(a) => {
                let first = self.engine.bars(b).first().map(|x| x.t);
                self.ext[b] = if first.is_some_and(|f| f <= a) {
                    Ext::Done
                } else {
                    Ext::Failed
                };
            }
            RFetch::Verify(key) => {
                let has = reply.bars.iter().any(|x| x.t == key.bar_t);
                let s = self.st.entry(key.clone()).or_default();
                s.verify_pending = s.verify_pending.saturating_sub(1);
                s.verify_all &= has;
                if s.verify_pending == 0 {
                    if s.verify_all {
                        s.verified_n = match self.auth.get(&key) {
                            Some(AState::Claim { n, .. }) => Some(*n),
                            _ => None,
                        };
                    } else if s.verify_tries < MAX_RETRY {
                        s.verify_tries += 1;
                        let at = ctx.now() + RETRY;
                        self.timer(ctx, at, RTimer::VerifyRetry(key));
                    }
                }
            }
            RFetch::Fill => {}
        }
        self.after_change(ctx, b, true);
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

// ---------------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------------

macro_rules! approach {
    ($name:literal, $summary:literal, $cfg:expr) => {
        Approach {
            name: $name,
            summary: $summary,
            authority: |s: &Setup| -> Box<dyn AuthorityAdapter> {
                Box::new(Authority::new(s, $cfg))
            },
            replica: |s: &Setup| -> Box<dyn ReplicaAdapter> { Box::new(Replica::new(s, $cfg)) },
        }
    };
}

/// Registry entry: the chosen B ([`BEST`]).
#[must_use]
pub fn register() -> Approach {
    approach!(
        "b",
        "evidence values: authority sends the claim's RSI points + anchor; replica shows only when its own data agrees (predicate), refetches/disputes otherwise",
        BEST
    )
}

/// All measured variants (the first is [`register`]'s). Used by the ignored
/// `variants` test (`cargo test --release -- --ignored variants --nocapture`).
#[must_use]
pub fn variants() -> Vec<Approach> {
    vec![
        register(),
        approach!(
            "b-off",
            "B, nothing provisional",
            Cfg {
                prov: Prov::Off,
                ..BEST
            }
        ),
        approach!(
            "b-live",
            "B, live-bar provisional claims shown",
            Cfg {
                prov: Prov::Live,
                ..BEST
            }
        ),
        approach!(
            "b-strict",
            "B-strict: bit-equal evidence values",
            Cfg {
                compare: Compare::Strict,
                ..BEST
            }
        ),
        approach!(
            "b-core",
            "B core (RESEARCH §3.B literally): wait, predicate, show; no hygiene/refetch/fill",
            Cfg {
                compare: Compare::Predicate,
                prov: Prov::Off,
                hygiene: false,
                disputes: false,
                fill: false,
            }
        ),
        approach!(
            "b-core-live",
            "B core with live provisional claims",
            Cfg {
                compare: Compare::Predicate,
                prov: Prov::Live,
                hygiene: false,
                disputes: false,
                fill: false,
            }
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenarios;
    use crate::sim::run_one;

    #[test]
    fn key_text_roundtrip() {
        let k = ClaimKey {
            detector: "rsi_pair_overbought".into(),
            params: "period=14,level=70".into(),
            binding: "px0+px1".into(),
            bar_t: 1_700_000_040_000,
            direction: Direction::Up,
        };
        assert_eq!(parse_key(&k.text()), Some(k));
        assert_eq!(parse_key("nonsense"), None);
    }

    #[test]
    fn evidence_values_survive_json_bit_exact() {
        let v = 70.000_000_123_456_78_f64;
        let op = vec![AOp::Claim {
            n: 1,
            k: "rsi_cross[period=14,level=70]@px0:1700000040000:up".into(),
            m: Maturity::Final,
            ok: true,
            anchor: [("px0".to_owned(), 1_699_976_040_000)].into_iter().collect(),
            pts: vec![("px0".into(), 1_699_999_980_000, 69.8), ("px0".into(), 1_700_000_040_000, v)],
        }];
        let bytes = serde_json::to_vec(&op).unwrap();
        let back: Vec<AOp> = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(back, op);
        let AOp::Claim { pts, .. } = &back[0] else {
            panic!()
        };
        assert_eq!(pts[1].2.to_bits(), v.to_bits());
    }

    #[test]
    fn ideal_and_lagging_feeds_have_no_hard_errors() {
        let a = register();
        for name in ["S1", "S2", "S6"] {
            let spec = scenarios::find(name).unwrap();
            for seed in 0..2 {
                let m = run_one(&spec, &a, seed).metrics;
                assert_eq!(m.errors(), 0, "{name} seed {seed}: {m:?}");
            }
        }
    }

    #[test]
    fn deterministic_log() {
        let a = register();
        let spec = scenarios::find("S8").unwrap();
        let x = run_one(&spec, &a, 3);
        let y = run_one(&spec, &a, 3);
        assert_eq!(x.log, y.log);
        assert_eq!(x.metrics.log_hash, y.metrics.log_hash);
    }

    /// Prints a compact table of every variant on every preset (default seeds).
    #[test]
    #[ignore = "long: run with --release -- --ignored variants --nocapture"]
    fn variants_table() {
        let threads = std::thread::available_parallelism().map_or(4, std::num::NonZero::get);
        for a in variants() {
            println!("== {} ({})", a.name, a.summary);
            println!(
                "scen   M1  M2  M3f  M3p  live  M5/elig      M6p p50/p99   M6f p50/p99   M8>  M8s      M13s   B/s   r2aB/s"
            );
            for spec in scenarios::presets() {
                let (_, s) = crate::run_scenario(&spec, &a, 0, u64::from(spec.default_seeds), threads);
                let p = |x: &crate::metrics::Pct| {
                    if x.n == 0 {
                        "-".to_owned()
                    } else {
                        format!("{:.0}/{:.0}", x.p50, x.p99)
                    }
                };
                println!(
                    "{:<5} {:>3} {:>3} {:>4} {:>4} {:>5} {:>5}/{:<5} {:>13} {:>13} {:>4} {:>6.0} {:>8.1} {:>6.1} {:>6.2}",
                    s.scenario,
                    s.m1_early,
                    s.m2_contradictory,
                    s.m3_wrong_final,
                    s.m3_wrong_provisional,
                    s.live_errors,
                    s.m5_missed,
                    s.m5_eligible,
                    p(&s.m6_provisional),
                    p(&s.m6_final),
                    s.m8_blocked,
                    s.m8_blocked_ms / 1000.0,
                    s.m13_dispute_ms / 1000.0,
                    s.m9_a2r_bytes_per_sec,
                    s.m9_r2a_bytes_per_sec,
                );
            }
        }
    }
}
