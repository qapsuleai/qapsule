//! Approach BS2 (R3/M7) — the chosen design (B + §18.5) with the **real authority and the
//! real replica** and the R3 self-repair path (`docs/PROTOCOL.md` §19.9). Results:
//! `rounds/R3/experiments/M7/RESULTS.md`.
//!
//! **Authority.** The real `qp_session::Session`. Its host (this adapter) is the data
//! layer §18.4 assigns to the host: the harness engine (gap-tolerant, per-field last
//! writer wins) receives the authority's own feed and hands the session a gapless local
//! stream (`bar` / `patch`; a change of several bars, e.g. a split snapshot, re-resolves
//! from the host provider: `Session::resync_bindings`). The history is resolved as one
//! bar and then appended bar by bar, so that the session makes the claims a live
//! authority would have made (its conditions are evaluated at the last bar). The replica's
//! connection and its subscriptions (`changes`, `claims`, the two `rsi_cross` conditions
//! per binding) exist before the run. Settled-bar hygiene is on
//! (`Session::set_settle_check`): every closed bar of the live feed is fetched from the
//! settled source `VERIFY_DELAY` after it closed (retried until settled) and handed to
//! `Session::settled_bars`; a `dispute` asks for `[anchor, t]` at once. Every line for the
//! replica's connection goes in frames numbered per connection (`{"f":n,"lines":[…]}`),
//! as in `bs`.
//!
//! **Replica.** As in `bs` (the real `qp_session::Replica`, same host data layer), with
//! `ReplicaOptions.dispute` on: its outbox (disputes, `log` catch-up) is sent to the
//! authority in numbered frames.
//!
//! Out of model (as in `bs`): `rsi_invalidation` (S18) and `rsi_pair_overbought` (S16)
//! are not `qp/0` conditions; those claims count as misses (M5).

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use qp_schema::data::{BarPatch, DataBinding, DataChange, DataEvent, FeedKey};
use qp_schema::protocol::{
    ClaimReason, ClaimRef, ClaimState, DisputeVerdict, Event, Maturity as QMaturity,
    OutboundLine, QueryResult, ServerMsg, SessionInit,
};
use qp_schema::{Bar, Condition, Document};
use qp_session::replica::{Replica, ReplicaOptions, Shown};
use qp_session::Session;
use qp_source::{Resolution, SourceProvider};
use serde::Serialize;
use serde_json::{json, Value};

use crate::adapter::{
    Approach, AuthorityAdapter, ChannelEvent, Ctx, FetchReply, ReplicaAdapter, Setup,
    SourceEvent,
};
use crate::detector::{ClaimKey, Direction, EvidenceRange, CROSS};
use crate::display::{DisplayItem, DisplayState, Label};
use crate::engine::{ApplyStatus, Engine};
use crate::spec::VTime;

const MS: VTime = 1_000;
const HOUSEKEEP: VTime = 500 * MS;
const REPAIR_FIRST: VTime = 2_000 * MS;
const RETRY: VTime = 1_000 * MS;
const MAX_RETRY: u32 = 20;
const FILL_WAIT: VTime = 3_000 * MS;

fn rsi_id(b: &str) -> String {
    format!("rsi_{b}")
}

/// Parses `ClaimKey::text`.
fn parse_key(s: &str) -> Option<ClaimKey> {
    let mut it = s.rsplitn(3, ':');
    let dir = it.next()?;
    let t: i64 = it.next()?.parse().ok()?;
    let rest = it.next()?;
    let (head, binding) = rest.rsplit_once('@')?;
    let (detector, params) = head.split_once('[')?;
    Some(ClaimKey {
        detector: detector.to_owned(),
        params: params.strip_suffix(']')?.to_owned(),
        binding: binding.to_owned(),
        bar_t: t,
        direction: match dir {
            "up" => Direction::Up,
            "down" => Direction::Down,
            _ => return None,
        },
    })
}

/// Parses a subscription id back to (detector, params, binding, direction).
fn parse_sub(s: &str, t: i64) -> Option<ClaimKey> {
    let (rest, dir) = s.rsplit_once(':')?;
    parse_key(&format!("{rest}:{t}:{dir}"))
}

/// The shared document: per binding a bound series and its RSI.
fn document(s: &Setup) -> Document {
    let mut series = Vec::new();
    for b in &s.bindings {
        series.push(json!({"id": b.id, "interval": b.binding.interval,
            "source": {"kind":"bind","symbol": b.binding.symbol,"interval": b.binding.interval,
                       "window": b.binding.window}}));
        series.push(json!({"id": rsi_id(&b.id), "interval": b.binding.interval,
            "source": {"kind":"study","study":"rsi","inputs":[b.id],
                       "params":{"period": f64::from(s.detector.spec.period)}}}));
    }
    serde_json::from_value(json!({"schema_version":"0","id":"harness","series":series}))
        .expect("document")
}

// ---------------------------------------------------------------------------------
// Authority: the real qp_session::Session with a host data layer (R3/M7)
// ---------------------------------------------------------------------------------

/// The replica connection's id on the authority.
const CONN: &str = "r";
/// Condition subscriptions per connection (the reference authority allows 32
/// subscriptions per connection; `claims` takes one, `changes` one more on `r`).
const SUBS_PER_CONN: usize = 30;

/// The connection carrying the `k`-th condition subscription (`r`, `r1`, `r2`, …).
fn conn_of(k: usize) -> String {
    match k / SUBS_PER_CONN {
        0 => CONN.to_owned(),
        i => format!("{CONN}{i}"),
    }
}
/// Verify a closed bar against the settled source this long after it closed (the
/// settled source needs `settle_lag_ms`; B's authority uses the same delay).
const VERIFY_DELAY: VTime = 2_500 * MS;

/// One settled fetch the authority host still owes the session.
#[derive(Debug, Clone)]
struct Due {
    at: VTime,
    b: usize,
    from_t: i64,
    to_t: i64,
    tries: u32,
}

struct Auth2 {
    setup: Setup,
    s: Session,
    prov: ProvState,
    eng: Engine,
    synced: Vec<Vec<Bar>>,
    lseq: Vec<u64>,
    resolved: Vec<bool>,
    /// Bars before this time came from a resolution (snapshot): settled already.
    verify_from: Vec<i64>,
    frame: u64,
    next_in: u64,
    frames_in: BTreeMap<u64, Vec<Value>>,
    due: Vec<Due>,
    fetches: BTreeMap<u64, Due>,
    timer_at: Option<VTime>,
    work: u64,
    /// Time each claim (wire key) was last sent as final, for the correction latency.
    finals: BTreeMap<String, VTime>,
    /// The connection is down: lines wait here and go as one frame on reconnect (a
    /// stream transport's send buffer; the harness channel would otherwise space the
    /// held frames 1 µs apart).
    down: bool,
    held: Vec<Value>,
    /// The replica's connections, and which one holds each condition subscription.
    conns: Vec<String>,
    sub_conn: BTreeMap<String, String>,
}

impl Auth2 {
    fn new(setup: &Setup, hygiene: bool) -> Self {
        let prov: ProvState = Rc::new(RefCell::new(BTreeMap::new()));
        let init: SessionInit = serde_json::from_value(json!({
            "document": document(setup),
            "principals": [
                {"id":"host","author":{"kind":"engine","name":"host","version":"0"},"role":"host"},
                {"id":"device","author":{"kind":"human","name":"device"},"role":"viewer"}
            ],
            "clock": 0, "epoch": 0
        }))
        .expect("init");
        let mut s = Session::with_provider(
            init,
            std::path::Path::new("."),
            Box::new(HostProvider(Rc::clone(&prov))),
        )
        .expect("session");
        s.set_settle_check(hygiene);
        // The replica's connection and subscriptions exist before the run (a resumed
        // session): the same ids the replica watches.
        // More than 30 condition subscriptions (S17: 100) go on further connections of
        // the same replica (one stream, as in `bs`).
        let mut conds = Vec::new();
        for b in &setup.bindings {
            let d = &setup.detector.spec;
            for (dir, op, level) in [("up", "cross_above", d.upper), ("down", "cross_below", d.lower)]
            {
                let k = format!("{CROSS}[period={},level={level}]@{}:{dir}", d.period, b.id);
                conds.push(json!({"type":"subscribe","id":k,"topic":{"kind":"condition",
                    "condition":{"kind":"threshold","series":rsi_id(&b.id),"op":op,"value":level}}}));
            }
        }
        let mut conns = Vec::new();
        let mut sub_conn = BTreeMap::new();
        for (k, m) in conds.iter().enumerate() {
            let c = conn_of(k);
            if !conns.contains(&c) {
                let _ = s.handle_raw(
                    &c,
                    &json!({"type":"hello","protocol":"qp/0","principal":"device","surface":{"kind":"phone"}})
                        .to_string(),
                );
                if c == CONN {
                    let _ = s.handle_raw(
                        &c,
                        &json!({"type":"subscribe","id":"replica:changes","topic":{"kind":"changes"}})
                            .to_string(),
                    );
                }
                let _ = s.handle_raw(
                    &c,
                    &json!({"type":"subscribe","id":"replica:claims","topic":{"kind":"claims"}})
                        .to_string(),
                );
                conns.push(c.clone());
            }
            let r = s.handle_raw(&c, &m.to_string());
            assert!(
                r.iter().all(|l| !matches!(l.msg, ServerMsg::Error { .. })),
                "subscription refused: {r:?}"
            );
            sub_conn.insert(m["id"].as_str().unwrap_or_default().to_owned(), c);
        }
        let n = setup.bindings.len();
        Self {
            setup: setup.clone(),
            s,
            prov,
            eng: Engine::new(setup),
            synced: vec![Vec::new(); n],
            lseq: vec![0; n],
            resolved: vec![false; n],
            verify_from: vec![i64::MIN; n],
            frame: 0,
            next_in: 1,
            frames_in: BTreeMap::new(),
            due: Vec::new(),
            fetches: BTreeMap::new(),
            timer_at: None,
            work: 0,
            finals: BTreeMap::new(),
            down: false,
            held: Vec::new(),
            conns,
            sub_conn,
        }
    }

    /// Frames the session's lines for the replica's connection and sends them.
    fn emit(&mut self, ctx: &mut Ctx, lines: Vec<OutboundLine>) {
        let mut out = Vec::new();
        for l in lines {
            if !self.conns.contains(&l.conn) {
                continue;
            }
            if let ServerMsg::Notify {
                event:
                    Event::Claim {
                        claim,
                        state,
                        evidence,
                        reason,
                        ..
                    },
                ..
            } = &l.msg
            {
                let key = format!("{}@{}", serde_json::to_string(claim).unwrap_or_default(), evidence.t);
                let now = ctx.now();
                match (state, reason) {
                    (ClaimState::Asserted, None) if evidence.maturity == QMaturity::Final => {
                        self.finals.entry(key).or_insert(now);
                    }
                    (ClaimState::Withdrawn, Some(r)) => {
                        let since = self.finals.remove(&key);
                        STATS.with(|s| s.borrow_mut().withdrawn(*r, since.map(|t| now - t)));
                    }
                    (ClaimState::Asserted, Some(r)) => {
                        STATS.with(|s| s.borrow_mut().asserted(*r));
                    }
                    _ => {}
                }
            }
            out.push(serde_json::to_value(&l).expect("line"));
        }
        if self.down {
            self.held.extend(out);
            return;
        }
        if out.is_empty() {
            return;
        }
        self.frame += 1;
        ctx.send(serde_json::to_vec(&json!({"f": self.frame, "lines": out})).expect("frame"));
    }

    /// Hands the host's bars of binding `b` to the session: the first time a one-bar
    /// resolution and the history as appended bars (the claims a live authority would
    /// have made), then appends and live revisions as a gapless local stream; a change
    /// of more than the last bar (a correction of several bars, a split snapshot)
    /// re-resolves from the provider.
    fn sync(&mut self, ctx: &mut Ctx, b: usize) {
        let new: Vec<Bar> = self.eng.bars(b).iter().map(sane).collect();
        if new.is_empty() {
            return;
        }
        let feed = self.setup.bindings[b].feed.clone();
        if !self.resolved[b] {
            self.resolved[b] = true;
            self.prov
                .borrow_mut()
                .insert(feed.clone(), (new[..1].to_vec(), 0));
            let lines = self.s.refresh_bindings();
            self.emit(ctx, lines);
            self.synced[b] = new[..1].to_vec();
        }
        let old = std::mem::take(&mut self.synced[b]);
        let same_t = new.len() >= old.len() && old.iter().zip(&new).all(|(o, n)| o.t == n.t);
        let revised: Vec<usize> = old
            .iter()
            .zip(&new)
            .enumerate()
            .filter(|(_, (o, n))| o != n)
            .map(|(i, _)| i)
            .collect();
        // A stream of events: appends, and at most one revised bar (the last held, or a
        // single correction).
        let stream = same_t && revised.len() <= 1;
        let mut ok = stream;
        if stream {
            let mut evs = Vec::new();
            for &i in &revised {
                let p: BarPatch = qp_source::bar_patch_diff(&old[i], &new[i]);
                if !p.is_empty() {
                    evs.push(DataChange::Patch {
                        t: new[i].t,
                        fields: p,
                    });
                }
            }
            for n in &new[old.len()..] {
                evs.push(DataChange::Bar { bar: n.clone() });
            }
            for change in evs {
                self.lseq[b] += 1;
                let ev = DataEvent {
                    feed: feed.clone(),
                    seq: self.lseq[b],
                    change,
                };
                // Keep the provider current (a later re-resolution starts from it).
                self.work += 1;
                match self.s.host_data_event(&ev) {
                    Ok(o) => self.emit(ctx, o.lines),
                    Err(_) => {
                        ok = false;
                        break;
                    }
                }
            }
        }
        self.prov
            .borrow_mut()
            .insert(feed, (new.clone(), self.lseq[b]));
        let id = self.setup.bindings[b].id.clone();
        let stale = self.s.binding_state(&id) != Some(qp_schema::data::BindingState::Ready);
        if !ok || stale {
            self.work += new.len() as u64;
            let lines = self.s.resync_bindings();
            self.emit(ctx, lines);
        }
        self.synced[b] = new;
    }

    fn after(&mut self, ctx: &mut Ctx) {
        for b in 0..self.setup.bindings.len() {
            if self.eng.bars(b) != self.synced[b].as_slice() {
                self.sync(ctx, b);
            }
        }
        let now = ctx.now();
        for r in self.s.take_settle_requests() {
            let Some(b) = self.setup.bindings.iter().position(|x| x.id == r.series) else {
                continue;
            };
            // Bars that came from a resolution are settled data already.
            if r.from_t == r.to_t && r.to_t < self.verify_from[b] {
                continue;
            }
            let single = r.from_t == r.to_t;
            self.due.push(Due {
                at: if single { now + VERIFY_DELAY } else { now },
                b,
                from_t: r.from_t,
                to_t: r.to_t,
                tries: 0,
            });
        }
        self.fire(ctx);
    }

    /// Sends the due fetches; arms the timer for the next one.
    fn fire(&mut self, ctx: &mut Ctx) {
        let now = ctx.now();
        let (ready, wait): (Vec<Due>, Vec<Due>) =
            std::mem::take(&mut self.due).into_iter().partition(|d| d.at <= now);
        self.due = wait;
        for d in ready {
            let id = ctx.fetch_settled(d.b, d.from_t, d.to_t);
            STATS.with(|s| s.borrow_mut().fetches += 1);
            self.fetches.insert(id, d);
        }
        if let Some(next) = self.due.iter().map(|d| d.at).min() {
            if self.timer_at.is_none_or(|t| t <= now || next < t) {
                self.timer_at = Some(next);
                ctx.set_timer(next, 2);
            }
        }
    }
}

impl AuthorityAdapter for Auth2 {
    fn on_source(&mut self, ctx: &mut Ctx, ev: &SourceEvent) {
        let a = self.eng.apply(ev);
        if a.binding < self.setup.bindings.len() {
            if let SourceEvent::Snapshot { bars, .. } = ev {
                self.verify_from[a.binding] = bars.last().map_or(i64::MIN, |x| x.t);
            }
        }
        self.after(ctx);
    }

    fn on_replica_msg(&mut self, ctx: &mut Ctx, msg: &[u8]) {
        if let Ok(v) = serde_json::from_slice::<Value>(msg) {
            if let Some(f) = v["f"].as_u64() {
                if f >= self.next_in {
                    self.frames_in
                        .insert(f, v["lines"].as_array().cloned().unwrap_or_default());
                }
            }
        }
        while let Some(lines) = self.frames_in.remove(&self.next_in) {
            self.next_in += 1;
            for m in lines {
                if m["type"] == "dispute" {
                    STATS.with(|s| s.borrow_mut().disputes_in += 1);
                }
                self.work += 1;
                // A dispute on a condition claim goes on the connection that holds the
                // subscription.
                let conn = m["claim"]["sub"]
                    .as_str()
                    .and_then(|k| self.sub_conn.get(k))
                    .cloned()
                    .unwrap_or_else(|| CONN.to_owned());
                let out = self.s.handle_raw(&conn, &m.to_string());
                for l in &out {
                    if let ServerMsg::Result {
                        data: QueryResult::Dispute { verdict, .. },
                        ..
                    } = &l.msg
                    {
                        STATS.with(|s| s.borrow_mut().verdict(*verdict));
                    }
                }
                self.emit(ctx, out);
            }
        }
        self.after(ctx);
    }

    fn on_timer(&mut self, ctx: &mut Ctx, _token: u64) {
        self.timer_at = None;
        self.fire(ctx);
        self.after(ctx);
    }

    fn on_channel(&mut self, ctx: &mut Ctx, ev: ChannelEvent) {
        match ev {
            ChannelEvent::Disconnected => self.down = true,
            ChannelEvent::Reconnected => {
                self.down = false;
                let held = std::mem::take(&mut self.held);
                if !held.is_empty() {
                    self.frame += 1;
                    ctx.send(
                        serde_json::to_vec(&json!({"f": self.frame, "lines": held}))
                            .expect("frame"),
                    );
                }
            }
        }
    }

    fn on_fetch(&mut self, ctx: &mut Ctx, reply: &FetchReply) {
        let Some(mut d) = self.fetches.remove(&reply.id) else {
            return;
        };
        let b = d.b;
        if !reply.bars.iter().any(|x| x.t == d.to_t) {
            // Not settled yet: ask again later.
            if d.tries < MAX_RETRY {
                d.tries += 1;
                d.at = ctx.now() + RETRY;
                self.due.push(d);
            }
            self.fire(ctx);
            return;
        }
        let id = self.setup.bindings[b].id.clone();
        let lines = self.s.settled_bars(&id, &reply.bars);
        self.emit(ctx, lines);
        // The session corrected its closed bars; the host's data layer follows.
        let n = self.synced[b].len();
        if n >= 2 {
            let last_closed = self.synced[b][n - 2].t;
            for x in &reply.bars {
                if x.t > last_closed {
                    continue;
                }
                if let Ok(i) = self.synced[b].binary_search_by_key(&x.t, |y| y.t) {
                    self.synced[b][i] = x.clone();
                }
            }
        }
        self.eng.apply_settled(b, &reply.bars);
        self.after(ctx);
    }

    fn work_units(&self) -> u64 {
        self.eng.work() + self.work
    }
}

// ---------------------------------------------------------------------------------
// Extra counters (not metrics: the harness's metrics are untouched); read by `m7stats`.
// ---------------------------------------------------------------------------------

/// Per-thread counters of the bs2 path, reset and read by the `m7stats` binary.
#[derive(Debug, Default, Clone, Serialize)]
pub struct Stats {
    /// Disputes the replica sent.
    pub disputes_sent: u64,
    /// Disputes the authority received.
    pub disputes_in: u64,
    /// Verdicts in the `result` replies.
    pub verdicts: BTreeMap<String, u64>,
    /// Withdrawals with a reason (data_corrected / dispute_upheld).
    pub withdrawn: BTreeMap<String, u64>,
    /// Assertions with a reason (data_corrected / dispute_rejected).
    pub asserted: BTreeMap<String, u64>,
    /// Correction latency (µs): a final claim sent → its withdrawal with a reason.
    pub correction_us: Vec<i64>,
    /// Settled fetches the authority host made.
    pub fetches: u64,
}

impl Stats {
    fn verdict(&mut self, v: DisputeVerdict) {
        let k = serde_json::to_value(v).ok().and_then(|x| x.as_str().map(str::to_owned));
        *self.verdicts.entry(k.unwrap_or_default()).or_default() += 1;
    }
    fn withdrawn(&mut self, r: ClaimReason, lat: Option<VTime>) {
        let k = serde_json::to_value(r).ok().and_then(|x| x.as_str().map(str::to_owned));
        *self.withdrawn.entry(k.unwrap_or_default()).or_default() += 1;
        if let Some(l) = lat {
            self.correction_us.push(l);
        }
    }
    fn asserted(&mut self, r: ClaimReason) {
        let k = serde_json::to_value(r).ok().and_then(|x| x.as_str().map(str::to_owned));
        *self.asserted.entry(k.unwrap_or_default()).or_default() += 1;
    }
}

thread_local! {
    /// The counters of the current thread's runs.
    pub static STATS: RefCell<Stats> = RefCell::new(Stats::default());
}

/// Takes (and resets) this thread's counters.
#[must_use]
pub fn take_stats() -> Stats {
    STATS.with(|s| std::mem::take(&mut *s.borrow_mut()))
}


// ---------------------------------------------------------------------------------
// Replica: the real qp_session::Replica with a host data layer
// ---------------------------------------------------------------------------------

/// What the host provider resolves: per feed, its bars and local `seq`.
type ProvState = Rc<RefCell<BTreeMap<FeedKey, (Vec<Bar>, u64)>>>;

struct HostProvider(ProvState);

impl SourceProvider for HostProvider {
    fn entitled(&self, _principal: &str, _feed: &FeedKey) -> bool {
        true
    }

    fn resolve(&self, _principal: &str, binding: &DataBinding) -> Resolution {
        let st = self.0.borrow();
        let Some((bars, seq)) = st.get(&binding.feed_key()) else {
            return Resolution::Pending;
        };
        match qp_source::resolve_window(bars, &binding.window, binding.anchor_t, u32::MAX) {
            Ok(w) if !w.is_empty() => Resolution::Ready {
                bars: w.slice(bars).to_vec(),
                seq: *seq,
            },
            Ok(_) => Resolution::NoData {
                reason: "empty window".to_owned(),
            },
            Err(e) => Resolution::NoData { reason: e },
        }
    }
}

#[derive(Debug, Clone)]
enum Fetch {
    Repair(usize, i64, u32),
    Refetch(usize, String, i64, u32),
    Fill(usize, i64),
    Anchor,
}

struct Rep {
    setup: Setup,
    r: Replica,
    prov: ProvState,
    eng: Engine,
    synced: Vec<Vec<Bar>>,
    lseq: Vec<u64>,
    display: DisplayState,
    empty: Vec<Bar>,
    next_frame: u64,
    frames: BTreeMap<u64, Vec<Value>>,
    suspects: Vec<BTreeSet<i64>>,
    repair_due: BTreeMap<(usize, i64), (VTime, u32)>,
    fetches: BTreeMap<u64, Fetch>,
    seen_pending: BTreeMap<(String, i64), VTime>,
    filled: BTreeSet<(usize, i64)>,
    anchor_asked: Vec<Option<i64>>,
    timer_at: Option<VTime>,
    work: u64,
    /// Bars written from the settled source (per binding, by time), and the latest bar
    /// time the feed itself has delivered: a settled bar is not overwritten by the
    /// lagging feed's ticks until the feed has moved past it (then a tick is a
    /// correction).
    settled: Vec<BTreeMap<i64, Bar>>,
    feed_last: Vec<i64>,
    /// Bindings whose host data changed since the last `after` (`None`: all).
    dirty: Option<BTreeSet<usize>>,
    shown_version: u64,
    /// Frames sent to the authority.
    out_frame: u64,
}

fn sane(b: &Bar) -> Bar {
    let mut x = b.clone();
    x.h = x.h.max(x.o).max(x.c);
    x.l = x.l.min(x.o).min(x.c);
    x
}

impl Rep {
    fn new(s: &Setup, dispute: bool) -> Self {
        let prov: ProvState = Rc::new(RefCell::new(BTreeMap::new()));
        let mut r = Replica::new("device", Box::new(HostProvider(Rc::clone(&prov))));
        r.start(document(s), 0);
        for b in &s.bindings {
            let d = &s.detector.spec;
            for (dir, op, level) in [("up", "cross_above", d.upper), ("down", "cross_below", d.lower)]
            {
                let k = format!(
                    "{CROSS}[period={},level={level}]@{}:{dir}",
                    d.period, b.id
                );
                let c: Condition = serde_json::from_value(
                    json!({"kind":"threshold","series":rsi_id(&b.id),"op":op,"value":level}),
                )
                .expect("condition");
                r.watch(&k, c);
            }
        }
        let _ = r.take_outbox();
        r.set_options(ReplicaOptions {
            dispute,
            ..ReplicaOptions::default()
        });
        let n = s.bindings.len();
        Self {
            setup: s.clone(),
            r,
            prov,
            eng: Engine::new(s),
            synced: vec![Vec::new(); n],
            lseq: vec![0; n],
            display: DisplayState::default(),
            empty: Vec::new(),
            next_frame: 1,
            frames: BTreeMap::new(),
            suspects: vec![BTreeSet::new(); n],
            repair_due: BTreeMap::new(),
            fetches: BTreeMap::new(),
            seen_pending: BTreeMap::new(),
            filled: BTreeSet::new(),
            anchor_asked: vec![None; n],
            timer_at: None,
            work: 0,
            settled: vec![BTreeMap::new(); n],
            feed_last: vec![i64::MIN; n],
            dirty: None,
            shown_version: u64::MAX,
            out_frame: 0,
        }
    }

    fn index(&self, id: &str) -> Option<usize> {
        self.setup.bindings.iter().position(|b| b.id == id)
    }

    fn doc_anchor(&self, b: usize) -> Option<i64> {
        let id = &self.setup.bindings[b].id;
        self.r.document()?.series.iter().find(|s| &s.id == id)?.binding()?.anchor_t
    }

    /// Hands the host's bars of binding `b` to the replica: a gapless local stream when
    /// the change is an append or revision, else a re-resolution.
    fn sync(&mut self, b: usize) {
        let new: Vec<Bar> = self.eng.bars(b).iter().map(sane).collect();
        let feed = self.setup.bindings[b].feed.clone();
        let old = std::mem::take(&mut self.synced[b]);
        let append = !old.is_empty()
            && new.len() >= old.len()
            && old.iter().zip(&new).all(|(o, n)| o.t == n.t);
        let mut ok = append;
        if append {
            let mut evs = Vec::new();
            for (o, n) in old.iter().zip(&new) {
                if o != n {
                    let p: BarPatch = qp_source::bar_patch_diff(o, n);
                    if !p.is_empty() {
                        evs.push(DataChange::Patch { t: n.t, fields: p });
                    }
                }
            }
            for n in &new[old.len()..] {
                evs.push(DataChange::Bar { bar: n.clone() });
            }
            for change in evs {
                self.lseq[b] += 1;
                let ev = DataEvent {
                    feed: feed.clone(),
                    seq: self.lseq[b],
                    change,
                };
                self.work += 1;
                if self.r.data_event(&ev).is_err() {
                    ok = false;
                    break;
                }
            }
        }
        self.prov
            .borrow_mut()
            .insert(feed, (new.clone(), self.lseq[b]));
        let id = self.setup.bindings[b].id.clone();
        let stale = self.r.binding_state(&id) != Some(qp_schema::data::BindingState::Ready);
        if !ok || (stale && !new.is_empty()) {
            self.work += new.len() as u64;
            self.r.resync();
        }
        self.synced[b] = new;
    }

    fn touch(&mut self, b: usize) {
        if let Some(d) = self.dirty.as_mut() {
            d.insert(b);
        }
    }

    fn after(&mut self, ctx: &mut Ctx) {
        let todo: Vec<usize> = match self.dirty.replace(BTreeSet::new()) {
            None => (0..self.setup.bindings.len()).collect(),
            Some(d) => d.into_iter().collect(),
        };
        for b in todo {
            if self.eng.bars(b) != self.synced[b].as_slice() {
                self.sync(b);
            }
            // History back to the document's anchor (§18.3: resolve from the anchor).
            if let Some(a) = self.doc_anchor(b) {
                let first = self.eng.bars(b).first().map(|x| x.t);
                if first.is_some_and(|f| f > a) && self.anchor_asked[b] != Some(a) {
                    self.anchor_asked[b] = Some(a);
                    let f = first.unwrap_or(a);
                    let id = ctx.fetch_settled(b, a, f - 1);
                    self.fetches.insert(id, Fetch::Anchor);
                }
            }
            let id = self.setup.bindings[b].id.clone();
            let from = self.suspects[b].first().copied();
            self.r.set_unclean(&id, from);
        }
        for (series, t) in self.r.take_refetch() {
            if let Some(b) = self.index(&series) {
                let from = self.doc_anchor(b).unwrap_or(t);
                let id = ctx.fetch_settled(b, from, t);
                self.fetches.insert(id, Fetch::Refetch(b, series, t, 0));
            }
        }
        // Fill: a claim waiting for its bar on a lagging feed is filled from the settled
        // source after FILL_WAIT.
        let now = ctx.now();
        let mut waiting = BTreeSet::new();
        let items: Vec<_> = self.r.display().to_vec();
        for it in &items {
            if it.state != Shown::Pending {
                continue;
            }
            let key = (it.series.clone(), it.t);
            waiting.insert(key.clone());
            let since = *self.seen_pending.entry(key).or_insert(now);
            let fillable = matches!(it.reason, Some("not_covered" | "not_closed" | "no_data" | "anchor" | "stale" | "not_held"));
            if fillable && now >= since + FILL_WAIT {
                if let Some(b) = self.index(&it.series) {
                    if self.filled.insert((b, it.t)) {
                        let from = match self.eng.bars(b).last() {
                            Some(x) => x.t.min(it.t),
                            None => self.doc_anchor(b).unwrap_or(it.t),
                        };
                        let to = it.t + self.setup.bindings[b].interval_ms;
                        let id = ctx.fetch_settled(b, from, to);
                        self.fetches.insert(id, Fetch::Fill(b, it.t));
                    }
                }
            }
        }
        self.seen_pending.retain(|k, _| waiting.contains(k));
        if self.r.display_version() != self.shown_version {
            self.shown_version = self.r.display_version();
            self.rebuild_display();
        }
        // The replica's messages to the authority (disputes, `log` catch-up).
        let out = self.r.take_outbox();
        if !out.is_empty() {
            STATS.with(|s| {
                s.borrow_mut().disputes_sent +=
                    out.iter().filter(|m| m["type"] == "dispute").count() as u64;
            });
            self.out_frame += 1;
            ctx.send(
                serde_json::to_vec(&json!({"f": self.out_frame, "lines": out})).expect("frame"),
            );
        }
        let busy = !waiting.is_empty() || !self.repair_due.is_empty();
        if busy && self.timer_at.is_none_or(|t| t <= now) {
            let at = now + HOUSEKEEP;
            self.timer_at = Some(at);
            ctx.set_timer(at, 1);
        }
    }

    fn rebuild_display(&mut self) {
        let mut d = DisplayState::default();
        for it in self.r.display() {
            let ClaimRef::Condition { sub } = &it.claim else {
                continue;
            };
            let Some(key) = parse_sub(sub, it.t) else {
                continue;
            };
            let label = match (it.state, it.label) {
                (Shown::Shown, Some(QMaturity::Final)) => Label::Final,
                (Shown::Shown, _) => Label::Provisional,
                (Shown::Pending, _) => Label::Pending,
                (Shown::Disputed, _) => Label::Disputed,
            };
            let mut shown = BTreeMap::new();
            if let Some(v) = it.values.first() {
                if let (Some(p), Some(x)) = (v.prev, v.value) {
                    shown.insert("prev".to_owned(), p);
                    shown.insert("value".to_owned(), x);
                }
            }
            let level: f64 = key
                .params
                .split(',')
                .find_map(|p| p.strip_prefix("level="))
                .and_then(|x| x.parse().ok())
                .unwrap_or(0.0);
            let mut evidence = Vec::new();
            if label.asserts() {
                shown.insert("level".to_owned(), level);
                if let Some(bars) = self.r.bars(&key.binding) {
                    if let Some(i) = bars.iter().position(|b| b.t == key.bar_t) {
                        let from = i.saturating_sub(self.setup.detector.spec.period as usize);
                        evidence.push(EvidenceRange {
                            binding: key.binding.clone(),
                            from_t: bars[from].t,
                            to_t: key.bar_t,
                        });
                    }
                }
            }
            d.put(DisplayItem {
                key,
                label,
                shown_values: shown,
                evidence,
            });
        }
        self.display = d;
    }
}

impl ReplicaAdapter for Rep {
    fn on_source(&mut self, ctx: &mut Ctx, ev: &SourceEvent) {
        let before = self.eng.bars(self.setup.binding_of(ev.feed()).unwrap_or(0)).last().map(|b| b.t);
        let a = self.eng.apply(ev);
        if a.binding < self.setup.bindings.len() {
            let b = a.binding;
            self.touch(b);
            match ev {
                SourceEvent::Data(d) => {
                    let t = match &d.change {
                        DataChange::Bar { bar } => bar.t,
                        DataChange::Patch { t, .. } => *t,
                    };
                    if let Some(sb) = self.settled[b].get(&t).cloned() {
                        if self.feed_last[b] <= t {
                            // The feed lags behind the settled bar: keep the settled values.
                            self.eng.apply_settled(b, &[sb]);
                        }
                    }
                    self.feed_last[b] = self.feed_last[b].max(t);
                    let fl = self.feed_last[b];
                    self.settled[b].retain(|k, _| *k >= fl);
                }
                SourceEvent::Snapshot { bars, .. } => {
                    self.feed_last[b] = bars.last().map_or(i64::MIN, |x| x.t);
                }
            }
            match a.status {
                ApplyStatus::Gap { .. } => {
                    // The closing ticks of the bar held before the gap may be lost when
                    // the event belongs to a later bar (an intra-bar gap is rewritten).
                    let t_ev = match ev {
                        SourceEvent::Data(d) => match &d.change {
                            DataChange::Bar { bar } => Some(bar.t),
                            DataChange::Patch { t, .. } => Some(*t),
                        },
                        SourceEvent::Snapshot { .. } => None,
                    };
                    if let (Some(prev), Some(te)) = (before, t_ev) {
                        if te > prev && self.suspects[b].insert(prev) {
                            self.repair_due
                                .insert((b, prev), (ctx.now() + REPAIR_FIRST, 0));
                        }
                    }
                }
                ApplyStatus::Applied if self.eng.missing(b) == 0 => {
                    // Late events filled every hole: nothing is suspect any more.
                    for t in std::mem::take(&mut self.suspects[b]) {
                        self.repair_due.remove(&(b, t));
                    }
                }
                _ => {}
            }
            if matches!(ev, SourceEvent::Snapshot { .. }) {
                self.suspects[b].clear();
                self.repair_due.retain(|(x, _), _| *x != b);
            }
        }
        self.after(ctx);
    }

    fn on_authority_msg(&mut self, ctx: &mut Ctx, msg: &[u8]) {
        if let Ok(v) = serde_json::from_slice::<Value>(msg) {
            if let Some(f) = v["f"].as_u64() {
                if f >= self.next_frame {
                    self.frames
                        .insert(f, v["lines"].as_array().cloned().unwrap_or_default());
                }
            }
        }
        while let Some(lines) = self.frames.remove(&self.next_frame) {
            self.next_frame += 1;
            for l in lines {
                self.work += 1;
                let _ = self.r.on_line(&l.to_string());
            }
        }
        // A mirrored change may set an anchor: every binding is looked at.
        self.dirty = None;
        self.after(ctx);
    }

    fn on_timer(&mut self, ctx: &mut Ctx, _token: u64) {
        self.timer_at = None;
        let now = ctx.now();
        let due: Vec<(usize, i64, u32)> = self
            .repair_due
            .iter()
            .filter(|(_, (at, _))| *at <= now)
            .map(|((b, t), (_, n))| (*b, *t, *n))
            .collect();
        for (b, t, n) in due {
            self.repair_due.remove(&(b, t));
            let id = ctx.fetch_settled(b, t, t);
            self.fetches.insert(id, Fetch::Repair(b, t, n));
        }
        self.after(ctx);
    }

    fn on_fetch(&mut self, ctx: &mut Ctx, reply: &FetchReply) {
        let Some(f) = self.fetches.remove(&reply.id) else {
            return;
        };
        let b = reply.binding;
        self.touch(b);
        let has = |t: i64| reply.bars.iter().any(|x| x.t == t);
        self.eng.apply_settled(b, &reply.bars);
        let fl = self.feed_last[b];
        for x in &reply.bars {
            if x.t >= fl {
                self.settled[b].insert(x.t, x.clone());
            }
        }
        match f {
            Fetch::Repair(b, t, n) => {
                if has(t) {
                    self.suspects[b].remove(&t);
                } else if n < MAX_RETRY {
                    self.repair_due.insert((b, t), (ctx.now() + RETRY, n + 1));
                }
            }
            Fetch::Refetch(b, series, t, n) => {
                if has(t) {
                    self.after(ctx);
                    self.r.refetched(&series, t);
                } else if n < MAX_RETRY {
                    let from = self.doc_anchor(b).unwrap_or(t);
                    let id = ctx.fetch_settled(b, from, t);
                    self.fetches.insert(id, Fetch::Refetch(b, series, t, n + 1));
                }
            }
            Fetch::Fill(b, t) => {
                if has(t) {
                    self.after(ctx);
                    let id = self.setup.bindings[b].id.clone();
                    self.r.set_settled(&id, t);
                } else {
                    self.filled.remove(&(b, t));
                }
            }
            Fetch::Anchor => {}
        }
        self.after(ctx);
    }

    fn display(&self) -> &DisplayState {
        &self.display
    }

    fn visible_data(&self, binding: usize) -> &[Bar] {
        self.r
            .bars(&self.setup.bindings[binding].id)
            .unwrap_or(&self.empty)
    }

    fn work_units(&self) -> u64 {
        self.eng.work() + self.work
    }
}

/// The authority side; `H`: settled-bar hygiene on.
#[must_use]
pub fn authority<const H: bool>(s: &Setup) -> Box<dyn AuthorityAdapter> {
    Box::new(Auth2::new(s, H))
}

/// The replica side; `D`: disputes on.
#[must_use]
pub fn replica<const D: bool>(s: &Setup) -> Box<dyn ReplicaAdapter> {
    Box::new(Rep::new(s, D))
}

/// Registry entry: hygiene and disputes on. Ablations: `bs2h` (hygiene only), `bs2d`
/// (disputes only), `bs2n` (neither: the real authority alone).
#[must_use]
pub fn register() -> Approach {
    Approach {
        name: "bs2",
        summary: "R3/M7: real qp_session authority (settled-bar hygiene) + real replica (dispute): §19.9",
        authority: authority::<true>,
        replica: replica::<true>,
    }
}
