//! D, authority side: shared engine on the authority's feed; claims go out with evidence
//! envelopes; `fact_final` only after the claim's bars are confirmed against the
//! provider's settled bars; data marks (E) carry digests of settled bars.

use std::collections::{BTreeMap, BTreeSet};

use crate::adapter::{AuthorityAdapter, Ctx, FetchReply, Setup, SourceEvent};
use crate::detector::{Claim, ClaimKey, Maturity};
use crate::engine::Engine;
use crate::spec::VTime;

use super::wire::{
    chunk_of, chunk_span, digest, hex, Batch, Envelope, Mat, Op, Report, Why, ALG, CHUNK, ENGINE,
};
use super::{Cfg, DigestMode};

/// Wait after a local close before asking the provider for the settled bar: the
/// provider's settle lag (2 s in the harness; a property of the provider) + 1 ms.
const SETTLE_WAIT: VTime = 2_001_000;
/// Retry when the provider has not settled the bar yet.
const RETRY: VTime = 300_000;

const TOKEN_SETTLE: u64 = 1 << 56;

#[derive(Debug, Clone)]
struct Sent {
    id: u64,
    mat: Mat,
    v: BTreeMap<String, f64>,
    ev: Vec<Envelope>,
}

fn same_env(a: &[Envelope], b: &[Envelope]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(x, y)| {
            x.series == y.series
                && x.anchor_t == y.anchor_t
                && x.from_t == y.from_t
                && x.to_t == y.to_t
                && x.bars == y.bars
                && x.digest == y.digest
                && x.prefix == y.prefix
                && x.epoch == y.epoch
        })
}

fn same_values(a: &BTreeMap<String, f64>, b: &BTreeMap<String, f64>) -> bool {
    a.len() == b.len()
        && a
            .iter()
            .zip(b)
            .all(|((ka, va), (kb, vb))| ka == kb && va.to_bits() == vb.to_bits())
}

/// The authority.
#[derive(Debug)]
pub struct Authority {
    cfg: Cfg,
    engine: Engine,
    feeds: Vec<qp_schema::data::FeedKey>,
    interval_ms: i64,
    ids: BTreeMap<ClaimKey, u64>,
    next_id: u64,
    sent: BTreeMap<ClaimKey, Sent>,
    /// Bars up to this time are confirmed against the provider (per binding).
    verified: Vec<i64>,
    /// Latest locally closed bar waiting for confirmation.
    want: Vec<i64>,
    epoch: Vec<u32>,
    snapshots: Vec<u32>,
    busy: Vec<bool>,
    again: Vec<bool>,
    inflight: BTreeMap<u64, (usize, u64, i64)>,
    last_write: Vec<BTreeMap<i64, u64>>,
    marked_bar: Vec<BTreeMap<i64, u64>>,
    marked_chunk: Vec<BTreeMap<i64, u64>>,
    seq: u64,
    ops: Vec<Op>,
    dirty: BTreeSet<ClaimKey>,
    disputes: u64,
}

impl Authority {
    /// New authority side.
    #[must_use]
    pub fn new(setup: &Setup, cfg: Cfg) -> Self {
        let n = setup.bindings.len();
        Self {
            cfg,
            engine: Engine::new(setup),
            feeds: setup.bindings.iter().map(|b| b.feed.clone()).collect(),
            interval_ms: setup.bindings.first().map_or(60_000, |b| b.interval_ms),
            ids: BTreeMap::new(),
            next_id: 1,
            sent: BTreeMap::new(),
            verified: vec![i64::MIN; n],
            want: vec![i64::MIN; n],
            epoch: vec![0; n],
            snapshots: vec![0; n],
            busy: vec![false; n],
            again: vec![false; n],
            inflight: BTreeMap::new(),
            last_write: vec![BTreeMap::new(); n],
            marked_bar: vec![BTreeMap::new(); n],
            marked_chunk: vec![BTreeMap::new(); n],
            seq: 0,
            ops: Vec::new(),
            dirty: BTreeSet::new(),
            disputes: 0,
        }
    }

    fn final_ok(&self, key: &ClaimKey) -> bool {
        if !self.cfg.settle {
            return true;
        }
        key.bindings().iter().all(|id| {
            self.engine
                .index(id)
                .is_some_and(|b| key.bar_t <= self.verified[b])
        })
    }

    /// A new provisional claim may be told only when every closed bar it rests on is
    /// confirmed (all but its own bar).
    fn prov_ok(&self, key: &ClaimKey) -> bool {
        if !self.cfg.settle || !self.cfg.prov_settled_prev {
            return true;
        }
        key.bindings().iter().all(|id| {
            self.engine
                .index(id)
                .is_some_and(|b| key.bar_t - self.interval_ms <= self.verified[b])
        })
    }

    fn envelopes(&mut self, c: &Claim, fin: bool) -> Vec<Envelope> {
        let mut out = Vec::with_capacity(c.evidence.len());
        for r in &c.evidence {
            let Some(b) = self.engine.index(&r.binding) else {
                continue;
            };
            let anchor_t = self.engine.bars(b).first().map_or(r.from_t, |x| x.t);
            let (n, d) = {
                let w = self.engine.range(b, r.from_t, r.to_t);
                (w.len() as u64, digest(w))
            };
            let mut work = n;
            let digest_s = (fin && self.cfg.digest != DigestMode::None).then(|| hex(d));
            let prefix = if fin && self.cfg.digest == DigestMode::Prefix {
                let p = self.engine.range(b, anchor_t, r.to_t);
                work += p.len() as u64;
                Some(hex(digest(p)))
            } else {
                None
            };
            self.engine.add_work(work);
            out.push(Envelope {
                series: r.binding.clone(),
                anchor_t,
                from_t: r.from_t,
                to_t: r.to_t,
                bars: n,
                alg: ALG.to_owned(),
                digest: digest_s,
                prefix,
                rev: self.engine.last_seq(b),
                epoch: self.epoch[b],
                engine: ENGINE.to_owned(),
            });
        }
        out
    }

    fn id_for(&mut self, key: &ClaimKey) -> u64 {
        if let Some(id) = self.ids.get(key) {
            return *id;
        }
        let id = self.next_id;
        self.next_id += 1;
        self.ids.insert(key.clone(), id);
        id
    }

    fn refresh(&mut self, key: &ClaimKey) {
        let Some(c) = self.engine.claim(key).cloned() else {
            if let Some(s) = self.sent.remove(key) {
                self.ops.push(Op::FactRetracted {
                    id: s.id,
                    why: Why::NoLongerHolds,
                });
            }
            return;
        };
        let fin = c.maturity == Maturity::Final && self.final_ok(key);
        let id = self.id_for(key);
        if fin {
            let ev = self.envelopes(&c, true);
            if let Some(s) = self.sent.get(key) {
                if s.mat == Mat::Final && same_values(&s.v, &c.values) && same_env(&s.ev, &ev) {
                    return;
                }
            }
            self.ops.push(Op::FactFinal {
                id,
                key: key.clone(),
                v: c.values.clone(),
                ev: ev.clone(),
            });
            self.sent.insert(
                key.clone(),
                Sent {
                    id,
                    mat: Mat::Final,
                    v: c.values,
                    ev,
                },
            );
        } else {
            // Provisional: told once; live value changes are not streamed (the replica
            // shows its own values; it only needs to know the authority holds the claim).
            if self.sent.get(key).is_some_and(|s| s.mat == Mat::Provisional) {
                return;
            }
            if !self.prov_ok(key) {
                // Its closed bars are not confirmed yet: say nothing until they are (a
                // provisional claim leaves only the live bar unconfirmed).
                if let Some(s) = self.sent.remove(key) {
                    self.ops.push(Op::FactRetracted {
                        id: s.id,
                        why: Why::NoLongerHolds,
                    });
                }
                return;
            }
            let ev = self.envelopes(&c, false);
            self.ops.push(Op::Fact {
                id,
                key: key.clone(),
                m: Mat::Provisional,
                v: c.values.clone(),
                ev: ev.clone(),
            });
            self.sent.insert(
                key.clone(),
                Sent {
                    id,
                    mat: Mat::Provisional,
                    v: c.values,
                    ev,
                },
            );
        }
    }

    /// Marks the sent claims of binding `b` whose window reaches `t` or later as dirty.
    fn dirty_from(&mut self, b: usize, t: i64) {
        let id = self.engine.ids()[b].clone();
        for (k, s) in &self.sent {
            if s.ev.iter().any(|e| e.series == id && e.to_t >= t) {
                self.dirty.insert(k.clone());
            }
        }
    }

    /// Marks the engine's claims on binding `b` with bar in `(lo, hi]` as dirty.
    fn dirty_bars(&mut self, b: usize, lo: i64, hi: i64) {
        let id = &self.engine.ids()[b];
        for k in self.engine.claims().keys() {
            if k.bar_t > lo && k.bar_t <= hi && k.bindings().contains(&id.as_str()) {
                self.dirty.insert(k.clone());
            }
        }
    }

    /// Data marks (E) for settled bars of `b` in `[lo, hi]`: complete chunks as chunk
    /// digests; bars of incomplete chunks (and, with `per_bar`, every bar) one by one.
    fn mark_range(&mut self, b: usize, lo: i64, hi: i64, per_bar: bool) {
        if !self.cfg.marks || hi < lo {
            return;
        }
        let hi = hi.min(self.verified[b]);
        let iv = self.interval_ms;
        let mut bars_out = Vec::new();
        let mut chunks_out = Vec::new();
        let (c0, c1) = (chunk_of(lo, iv), chunk_of(hi, iv));
        let mut work = 0;
        for c in c0..=c1 {
            let (cs, ce) = chunk_span(c, iv);
            let full: Vec<(i64, u64)> = {
                let r = self.engine.range(b, cs, ce);
                let complete = r.len() as i64 == CHUNK && ce <= self.verified[b];
                work += r.len() as u64;
                if complete {
                    let d = digest(r);
                    if self.marked_chunk[b].get(&c) != Some(&d) {
                        chunks_out.push((c, d));
                    }
                }
                if !complete || per_bar {
                    r.iter()
                        .filter(|x| x.t >= lo && x.t <= hi)
                        .map(|x| (x.t, digest(std::slice::from_ref(x))))
                        .collect()
                } else {
                    Vec::new()
                }
            };
            for (t, d) in full {
                if self.marked_bar[b].get(&t) != Some(&d) {
                    bars_out.push((t, d));
                }
            }
        }
        self.engine.add_work(work);
        for &(c, d) in &chunks_out {
            self.marked_chunk[b].insert(c, d);
        }
        for &(t, d) in &bars_out {
            self.marked_bar[b].insert(t, d);
        }
        if bars_out.is_empty() && chunks_out.is_empty() {
            return;
        }
        self.ops.push(Op::DataMark {
            series: self.engine.ids()[b].clone(),
            epoch: self.epoch[b],
            bars: bars_out.into_iter().map(|(t, d)| (t, hex(d))).collect(),
            chunks: chunks_out.into_iter().map(|(c, d)| (c, hex(d))).collect(),
        });
    }

    fn request(&mut self, ctx: &mut Ctx, b: usize) {
        if !self.cfg.settle {
            return;
        }
        if self.busy[b] {
            self.again[b] = true;
            return;
        }
        if self.want[b] <= self.verified[b] {
            return;
        }
        let from = self.verified[b].saturating_add(1);
        let to = self.want[b];
        let id = ctx.fetch_settled(b, from, to);
        self.inflight.insert(id, (b, self.engine.last_seq(b), to));
        self.busy[b] = true;
    }

    fn sync(&mut self, ctx: &mut Ctx) {
        for d in self.engine.take_deltas() {
            self.dirty.insert(d.key);
        }
        let dirty = std::mem::take(&mut self.dirty);
        self.engine.add_work(dirty.len() as u64);
        for k in &dirty {
            self.refresh(k);
        }
        if !self.ops.is_empty() {
            self.seq += 1;
            let batch = Batch {
                s: self.seq,
                ops: std::mem::take(&mut self.ops),
            };
            ctx.send(serde_json::to_vec(&batch).expect("json"));
        }
    }
}

impl AuthorityAdapter for Authority {
    fn on_source(&mut self, ctx: &mut Ctx, ev: &SourceEvent) {
        let Some(b) = self.feeds.iter().position(|f| f == ev.feed()) else {
            return;
        };
        match ev {
            SourceEvent::Snapshot { bars, .. } => {
                let first = self.snapshots[b] == 0;
                self.snapshots[b] += 1;
                let closed_old = self.engine.bars(b).len().saturating_sub(1);
                let closed_new = bars.len().saturating_sub(1);
                let changed = {
                    let old = self.engine.bars(b);
                    let n = closed_old.min(closed_new);
                    old[..n] != bars[..n]
                };
                self.engine.apply(ev);
                if !first && changed {
                    self.epoch[b] += 1;
                }
                if bars.len() >= 2 {
                    let t = bars[bars.len() - 2].t;
                    self.verified[b] = self.verified[b].max(t);
                    self.want[b] = self.want[b].max(t);
                }
                if first {
                    let anchor = bars.first().map_or(0, |x| x.t);
                    self.ops.push(Op::Hello {
                        engine: ENGINE.to_owned(),
                        anchors: vec![(self.engine.ids()[b].clone(), anchor)],
                    });
                }
                if let (Some(f), Some(l)) = (bars.first(), bars.last()) {
                    self.mark_range(b, f.t, l.t, false);
                }
                self.dirty_from(b, i64::MIN);
            }
            SourceEvent::Data(d) => {
                let t = match &d.change {
                    qp_schema::data::DataChange::Bar { bar } => bar.t,
                    qp_schema::data::DataChange::Patch { t, .. } => *t,
                };
                let head = self.engine.bars(b).last().map_or(i64::MIN, |x| x.t);
                self.last_write[b].insert(t, d.seq);
                let applied = self.engine.apply(ev);
                if applied.changed_from_t.is_some() && t < head && t <= self.verified[b] {
                    // A confirmed bar changed (provider correction): new data epoch, new
                    // marks, new final envelopes for the claims whose data reaches it.
                    self.epoch[b] += 1;
                    self.mark_range(b, t, t, true);
                    self.dirty_from(b, t);
                }
                if applied.new_bar {
                    let bars = self.engine.bars(b);
                    if bars.len() >= 2 {
                        self.want[b] = self.want[b].max(bars[bars.len() - 2].t);
                    }
                    if self.cfg.settle {
                        ctx.set_timer(ctx.now() + SETTLE_WAIT, TOKEN_SETTLE | b as u64);
                    } else {
                        // Without provider confirmation, a locally closed bar counts as
                        // confirmed.
                        let old = self.verified[b];
                        self.verified[b] = self.want[b];
                        self.dirty_bars(b, old, self.verified[b]);
                        self.mark_range(b, old.saturating_add(1), self.verified[b], true);
                    }
                }
            }
        }
        self.sync(ctx);
    }

    fn on_timer(&mut self, ctx: &mut Ctx, token: u64) {
        if token & TOKEN_SETTLE != 0 {
            #[allow(clippy::cast_possible_truncation)]
            let b = (token & 0xffff_ffff) as usize;
            self.request(ctx, b);
        }
        self.sync(ctx);
    }

    fn on_fetch(&mut self, ctx: &mut Ctx, reply: &FetchReply) {
        let Some((b, req_seq, to)) = self.inflight.remove(&reply.id) else {
            return;
        };
        self.busy[b] = false;
        // Bars the feed wrote after the request are newer than the answer: keep them.
        let bars: Vec<qp_schema::Bar> = reply
            .bars
            .iter()
            .filter(|x| self.last_write[b].get(&x.t).is_none_or(|s| *s <= req_seq))
            .cloned()
            .collect();
        self.engine.apply_settled(b, &bars);
        let old = self.verified[b];
        if let Some(last) = reply.bars.last() {
            self.verified[b] = self.verified[b].max(last.t);
        }
        if self.verified[b] < to {
            ctx.set_timer(ctx.now() + RETRY, TOKEN_SETTLE | b as u64);
        }
        if self.verified[b] > old {
            self.dirty_bars(b, old, self.verified[b].saturating_add(self.interval_ms));
            self.mark_range(b, old.saturating_add(1), self.verified[b], true);
        }
        if self.again[b] {
            self.again[b] = false;
            self.request(ctx, b);
        }
        self.sync(ctx);
    }

    fn on_replica_msg(&mut self, _ctx: &mut Ctx, msg: &[u8]) {
        if let Ok(Report::DataDispute { .. }) = serde_json::from_slice::<Report>(msg) {
            self.disputes += 1;
        }
    }

    fn work_units(&self) -> u64 {
        self.engine.work()
    }
}
