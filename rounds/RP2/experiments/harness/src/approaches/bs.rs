//! Approach BS — the chosen design (B + §18.5) with the **real replica**
//! (`qp_session::replica::Replica`, RP2/T8; `docs/PROTOCOL.md` §19). Results: `T8.md`.
//!
//! **Authority.** B's authority logic unchanged (`approaches/b.rs`, `BEST`: its engine,
//! data hygiene, settled verification), wrapped: every message B would send is re-encoded
//! as real `qp/0` lines — a `change` (`put_series` with `anchor_t`) when a binding's
//! anchor is first known or changes, and `notify` `claim` events (topic `claims`) with
//! `ClaimEvidence` (bar series, `t`, RSI value at `t` and the bar before, maturity,
//! clean, anchor, feeds) for `rsi_cross` claims, withdrawals for B's `retract`. Lines go in
//! frames numbered per connection (`{"f":n,"lines":[…]}`): the transport's ordering (a
//! qp/0 connection is an ordered stream; the harness channel reorders and duplicates, so
//! the frame number restores the order, as TCP would).
//!
//! **Replica.** The real `qp_session::Replica` (follower session: document mirror, data
//! plane with the §18.4 rules, live path, `rsi` from `qp-compute`, the display rule). Its
//! document is the shared document (bound `px*`, `rsi_px*`); it watches the two
//! `rsi_cross` conditions per binding. Its **host** (this adapter) is the data layer
//! §18.4 assigns to the host: the harness engine (gap-tolerant, per-field last writer
//! wins) receives the replica's own feed; the host hands the replica a gapless local
//! stream (`bar` / `patch` events with a local `seq`; a non-append change → re-resolution
//! from the host provider), marks suspect bars after a sequence gap as unclean and repairs
//! them from the settled source, answers the replica's refetch requests, extends history
//! back to the document's anchor, and fills a claim's bar from the settled source when its
//! feed lags (then marks it settled). Every display decision is the replica's own.
//!
//! Out of model: `rsi_invalidation` (S18) and `rsi_pair_overbought` (S16) are not
//! `qp/0` conditions (a single threshold on one series); the authority's claims of those
//! detectors are not forwarded and count as misses (M5). B's `dispute` / `check`
//! replica→authority messages have no `qp/0` counterpart yet (§19.8): not sent.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use qp_schema::data::{BarPatch, DataBinding, DataChange, DataEvent, FeedKey};
use qp_schema::protocol::{ClaimRef, Maturity as QMaturity};
use qp_schema::{Bar, Condition, Document};
use qp_session::replica::{Replica, Shown};
use qp_source::{Resolution, SourceProvider};
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

/// The condition subscription id of a claim key (without its bar time).
fn sub_of(k: &ClaimKey) -> String {
    let dir = match k.direction {
        Direction::Up => "up",
        Direction::Down => "down",
    };
    format!("{}[{}]@{}:{dir}", k.detector, k.params, k.binding)
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

fn series_json(s: &Setup, b: usize, anchor: i64) -> Value {
    let bs = &s.bindings[b];
    json!({"id": bs.id, "interval": bs.binding.interval,
        "source": {"kind":"bind","symbol": bs.binding.symbol,"interval": bs.binding.interval,
                   "window": bs.binding.window, "anchor_t": anchor}})
}

// ---------------------------------------------------------------------------------
// Authority: B, re-encoded as qp/0
// ---------------------------------------------------------------------------------

struct Auth {
    inner: Box<dyn AuthorityAdapter>,
    setup: Setup,
    frame: u64,
    change_seq: u64,
    anchors: BTreeMap<String, i64>,
    last_ev: BTreeMap<String, Value>,
}

impl Auth {
    fn translate(&mut self, ctx: &mut Ctx, from: usize) {
        let msgs: Vec<Vec<u8>> = ctx.msgs.drain(from..).collect();
        for m in msgs {
            let Ok(ops) = serde_json::from_slice::<Vec<Value>>(&m) else {
                continue;
            };
            let mut lines = Vec::new();
            for op in ops {
                let Some(k) = op["k"].as_str().and_then(parse_key) else {
                    continue;
                };
                match op["op"].as_str() {
                    Some("claim") => {
                        for id in k.bindings() {
                            let Some(a) = op["anchor"][id].as_i64() else {
                                continue;
                            };
                            if self.anchors.get(id) != Some(&a) {
                                self.anchors.insert(id.to_owned(), a);
                                let Some(b) = self.setup.bindings.iter().position(|x| x.id == id)
                                else {
                                    continue;
                                };
                                self.change_seq += 1;
                                lines.push(json!({"conn":"r","msg":{"type":"notify","sub":"replica:changes",
                                    "event":{"kind":"change","seq":self.change_seq,"at":0,"author":"qp",
                                    "changes":[{"op":"put_series","series":series_json(&self.setup,b,a)}]}}}));
                            }
                        }
                        if k.detector != CROSS {
                            continue;
                        }
                        let pts = op["pts"].as_array().cloned().unwrap_or_default();
                        let at = |t: i64| {
                            pts.iter()
                                .find(|p| p[1].as_i64() == Some(t))
                                .and_then(|p| p[2].as_f64())
                        };
                        let prev_t = pts
                            .iter()
                            .filter_map(|p| p[1].as_i64())
                            .find(|t| *t != k.bar_t);
                        let feed = self
                            .setup
                            .bindings
                            .iter()
                            .find(|x| x.id == k.binding)
                            .map(|x| x.feed.clone());
                        let ev = json!({"series": k.binding, "t": k.bar_t,
                            "values":[{"series": rsi_id(&k.binding), "t": k.bar_t,
                                       "value": at(k.bar_t), "prev": prev_t.and_then(at)}],
                            "maturity": op["m"], "clean": op["ok"],
                            "anchor_t": op["anchor"][&k.binding], "feeds": feed.into_iter().collect::<Vec<_>>()});
                        self.last_ev.insert(k.text(), ev.clone());
                        lines.push(json!({"conn":"r","msg":{"type":"notify","sub":"replica:claims",
                            "event":{"kind":"claim","claim":{"kind":"condition","sub":sub_of(&k)},
                                     "state":"asserted","evidence":ev,"seq":self.change_seq}}}));
                    }
                    Some("retract") => {
                        let Some(ev) = self.last_ev.get(&k.text()).cloned() else {
                            continue;
                        };
                        lines.push(json!({"conn":"r","msg":{"type":"notify","sub":"replica:claims",
                            "event":{"kind":"claim","claim":{"kind":"condition","sub":sub_of(&k)},
                                     "state":"withdrawn","evidence":ev,"seq":self.change_seq}}}));
                    }
                    _ => {}
                }
            }
            if lines.is_empty() {
                continue;
            }
            self.frame += 1;
            let frame = json!({"f": self.frame, "lines": lines});
            ctx.msgs
                .push(serde_json::to_vec(&frame).expect("frame"));
        }
    }
}

impl AuthorityAdapter for Auth {
    fn on_source(&mut self, ctx: &mut Ctx, ev: &SourceEvent) {
        let n = ctx.msgs.len();
        self.inner.on_source(ctx, ev);
        self.translate(ctx, n);
    }
    fn on_replica_msg(&mut self, _ctx: &mut Ctx, _msg: &[u8]) {}
    fn on_timer(&mut self, ctx: &mut Ctx, token: u64) {
        let n = ctx.msgs.len();
        self.inner.on_timer(ctx, token);
        self.translate(ctx, n);
    }
    fn on_fetch(&mut self, ctx: &mut Ctx, reply: &FetchReply) {
        let n = ctx.msgs.len();
        self.inner.on_fetch(ctx, reply);
        self.translate(ctx, n);
    }
    fn on_channel(&mut self, ctx: &mut Ctx, ev: ChannelEvent) {
        let n = ctx.msgs.len();
        self.inner.on_channel(ctx, ev);
        self.translate(ctx, n);
    }
    fn work_units(&self) -> u64 {
        self.inner.work_units()
    }
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
}

fn sane(b: &Bar) -> Bar {
    let mut x = b.clone();
    x.h = x.h.max(x.o).max(x.c);
    x.l = x.l.min(x.o).min(x.c);
    x
}

impl Rep {
    fn new(s: &Setup) -> Self {
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

    fn after(&mut self, ctx: &mut Ctx) {
        for b in 0..self.setup.bindings.len() {
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
        self.rebuild_display();
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
        let _ = self.r.take_outbox();
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

/// Registry entry.
#[must_use]
pub fn register() -> Approach {
    Approach {
        name: "bs",
        summary: "B + §18.5 with the real qp_session::Replica: B's authority re-encoded as qp/0 claim events with ClaimEvidence; the replica's display rule decides",
        authority: |s| {
            Box::new(Auth {
                inner: (super::b::register().authority)(s),
                setup: s.clone(),
                frame: 0,
                change_seq: 0,
                anchors: BTreeMap::new(),
                last_ev: BTreeMap::new(),
            })
        },
        replica: |s| Box::new(Rep::new(s)),
    }
}
