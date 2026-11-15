//! Approach C — "the replica works only with its own data and reports divergence
//! afterwards" (RESEARCH §3.C), built as its strongest advocate would.
//!
//! **Display rule.** The replica shows only claims its own engine finds on its own data
//! (its feed plus its own settled-bar fetches). Nothing the authority sends decides
//! *whether* a claim is shown; the authority's claim stream is used only afterwards, to
//! mark disagreement (`SourceDiffers` on a claim the replica holds and the authority does
//! not, `Disputed` — not asserted — on a claim only the authority holds) and to report it
//! (`report{kind:"divergence"}`, replica → authority).
//!
//! **Labels from the replica's own data.**
//! - Claim on the replica's live bar → `Provisional` (only while the replica's feed is
//!   fresh: not silent, not behind the settled source).
//! - Claim on closed bars → `Final` only when every bar up to the claim bar is
//!   *confirmed*: either the replica's feed is proven exact (an audit against the
//!   provider's settled bars matched) and no event is missing (`seq` gap) up to that bar,
//!   or the bars were replaced by settled bars (`ctx.fetch_settled`). Until then a claim
//!   already shown stays `Provisional`; a new one waits.
//! - The replica polls its settled source once per bar (learning when a bar becomes
//!   fetchable), which doubles as audit (f32 or glitching feeds are detected and then
//!   only settled bars are trusted), as confirmation of bars with gaps, as look-ahead when
//!   the feed lags behind the settled source, and as the source of closed bars during a
//!   feed outage.
//!
//! **Protocol.** Authority → replica: a hello with the authority's compute anchor per
//! binding (binding metadata: the first bar the authority computes from), its final claim
//! set as add/del of claim keys, and its watermark (last closed bar per binding). No
//! market data crosses the channel (K23). Replica → authority: divergence reports; the
//! authority answers by refetching settled bars for the disputed range ("the authority
//! takes note").

mod replica;

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use qp_schema::Bar;

use crate::adapter::{Approach, AuthorityAdapter, Ctx, FetchReply, Setup, SourceEvent};
use crate::detector::{ClaimKey, Direction, Maturity};
use crate::engine::Engine;

/// Authority → replica message (JSON).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub(crate) struct AuthMsg {
    /// Message number (1, 2, …): the replica applies messages in this order.
    pub n: u64,
    /// Compute anchor (first bar time) per binding id — sent once (hello).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub a: BTreeMap<String, i64>,
    /// Watermark per binding id: last bar closed on the authority's data; its final
    /// claim set is complete up to here.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub w: BTreeMap<String, i64>,
    /// Final claims added (key text).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub add: Vec<String>,
    /// Final claims removed (key text).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub del: Vec<String>,
}

impl AuthMsg {
    fn is_empty(&self) -> bool {
        self.a.is_empty() && self.w.is_empty() && self.add.is_empty() && self.del.is_empty()
    }
}

/// One divergence item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Divergence {
    /// Claim key text.
    pub claim: String,
    /// The replica's data holds the claim (final).
    pub local: bool,
    /// The authority's stream holds it.
    pub authority: bool,
    /// The replica's confirmed bar frontier of the claim's first binding.
    pub sealed_t: i64,
}

/// Replica → authority message: `report{kind:"divergence"}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ReportMsg {
    #[serde(rename = "type")]
    pub ty: String,
    pub kind: String,
    pub items: Vec<Divergence>,
}

/// Parses a claim key from its text form
/// `detector[params]@binding:bar_t:up|down` ([`ClaimKey::text`]).
pub(crate) fn parse_key(s: &str) -> Option<ClaimKey> {
    let (detector, rest) = s.split_once('[')?;
    let (params, rest) = rest.split_once("]@")?;
    let (rest, dir) = rest.rsplit_once(':')?;
    let (binding, bar_t) = rest.rsplit_once(':')?;
    let direction = match dir {
        "up" => Direction::Up,
        "down" => Direction::Down,
        _ => return None,
    };
    Some(ClaimKey {
        detector: detector.to_owned(),
        params: params.to_owned(),
        binding: binding.to_owned(),
        bar_t: bar_t.parse().ok()?,
        direction,
    })
}

/// Applies settled bars to binding `b` of an engine: unchanged bars are skipped; a few
/// changed bars go through `Engine::apply_settled`; many changed bars before the last
/// bar are merged and applied as one snapshot (one recomputation instead of one per
/// bar). Returns the number of bars that differed.
pub(crate) fn apply_settled_bulk(engine: &mut Engine, setup: &Setup, b: usize, bars: &[Bar]) -> usize {
    let cur = engine.bars(b);
    let last_t = cur.last().map(|x| x.t);
    let diffs: Vec<Bar> = bars
        .iter()
        .filter(|x| {
            cur.binary_search_by_key(&x.t, |y| y.t)
                .map_or(true, |i| cur[i] != **x)
        })
        .cloned()
        .collect();
    if diffs.is_empty() {
        return 0;
    }
    let nontail = diffs
        .iter()
        .filter(|x| last_t.is_some_and(|l| x.t < l))
        .count();
    if nontail > 2 {
        let mut merged: BTreeMap<i64, Bar> = cur.iter().map(|x| (x.t, x.clone())).collect();
        for x in &diffs {
            merged.insert(x.t, x.clone());
        }
        let seq = engine.last_seq(b);
        engine.apply(&SourceEvent::Snapshot {
            feed: setup.bindings[b].feed.clone(),
            seq,
            bars: merged.into_values().collect(),
        });
    } else {
        engine.apply_settled(b, &diffs);
    }
    diffs.len()
}

struct Authority {
    setup: Setup,
    engine: Engine,
    n: u64,
    finals: BTreeSet<ClaimKey>,
    w: Vec<Option<i64>>,
    anchored: Vec<bool>,
    fetched_to: Vec<Option<i64>>,
    pending: AuthMsg,
}

impl Authority {
    fn new(setup: &Setup) -> Self {
        let n = setup.bindings.len();
        Self {
            setup: setup.clone(),
            engine: Engine::new(setup),
            n: 0,
            finals: BTreeSet::new(),
            w: vec![None; n],
            anchored: vec![false; n],
            fetched_to: vec![None; n],
            pending: AuthMsg::default(),
        }
    }

    fn flush(&mut self, ctx: &mut Ctx) {
        for d in self.engine.take_deltas() {
            let should = d
                .after
                .as_ref()
                .is_some_and(|c| c.maturity == Maturity::Final);
            let has = self.finals.contains(&d.key);
            if should && !has {
                self.pending.add.push(d.key.text());
                self.finals.insert(d.key);
            } else if !should && has {
                self.pending.del.push(d.key.text());
                self.finals.remove(&d.key);
            }
        }
        for b in 0..self.setup.bindings.len() {
            let bars = self.engine.bars(b);
            let w = (bars.len() >= 2).then(|| bars[bars.len() - 2].t);
            if w != self.w[b] {
                self.w[b] = w;
                if let Some(w) = w {
                    self.pending.w.insert(self.setup.bindings[b].id.clone(), w);
                }
            }
        }
        if !self.pending.is_empty() {
            self.n += 1;
            let mut msg = std::mem::take(&mut self.pending);
            msg.n = self.n;
            ctx.send(serde_json::to_vec(&msg).expect("json"));
        }
    }
}

impl AuthorityAdapter for Authority {
    fn on_source(&mut self, ctx: &mut Ctx, ev: &SourceEvent) {
        if let (SourceEvent::Snapshot { bars, .. }, Some(b)) = (ev, self.setup.binding_of(ev.feed())) {
            if !self.anchored[b] {
                if let Some(first) = bars.first() {
                    self.anchored[b] = true;
                    self.pending
                        .a
                        .insert(self.setup.bindings[b].id.clone(), first.t);
                }
            }
        }
        self.engine.apply(ev);
        self.flush(ctx);
    }

    fn on_replica_msg(&mut self, ctx: &mut Ctx, msg: &[u8]) {
        let Ok(rep) = serde_json::from_slice::<ReportMsg>(msg) else {
            return;
        };
        for item in rep.items {
            let Some(key) = parse_key(&item.claim) else {
                continue;
            };
            for id in key.bindings() {
                let Some(b) = self.engine.index(id) else {
                    continue;
                };
                let iv = self.setup.bindings[b].interval_ms;
                let from = match self.fetched_to[b] {
                    Some(t) => t + iv,
                    None => self.engine.bars(b).first().map_or(key.bar_t, |x| x.t),
                };
                if from <= key.bar_t {
                    ctx.fetch_settled(b, from, key.bar_t);
                    self.fetched_to[b] = Some(key.bar_t);
                }
            }
        }
    }

    fn on_fetch(&mut self, ctx: &mut Ctx, reply: &FetchReply) {
        if reply.binding < self.setup.bindings.len() {
            apply_settled_bulk(&mut self.engine, &self.setup, reply.binding, &reply.bars);
            self.flush(ctx);
        }
    }

    fn work_units(&self) -> u64 {
        self.engine.work()
    }
}

/// Registry entry.
#[must_use]
pub fn register() -> Approach {
    Approach {
        name: "c",
        summary: "replica shows only its own findings (own feed + own settled fetches); divergence from the authority marked and reported afterwards",
        authority: |s: &Setup| Box::new(Authority::new(s)),
        replica: |s: &Setup| Box::new(replica::Replica::new(s)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_text_round_trips() {
        for (binding, dir) in [("px0", Direction::Up), ("px0+px1", Direction::Down)] {
            let k = ClaimKey {
                detector: "rsi_cross".into(),
                params: "period=14,level=70".into(),
                binding: binding.into(),
                bar_t: 1_700_000_040_000,
                direction: dir,
            };
            assert_eq!(parse_key(&k.text()), Some(k));
        }
        assert_eq!(parse_key("nonsense"), None);
    }

    /// Same seed → byte-identical display log; and C never shows early or contradictory
    /// claims (M1 = M2 = 0 hold by construction: it shows only its own engine's claims).
    #[test]
    fn deterministic_and_never_early_or_contradictory() {
        let c = register();
        for name in ["S4", "S6", "S11", "S12s", "S14", "S15k", "S16"] {
            let spec = crate::scenarios::find(name).expect("preset");
            for seed in [1, 2] {
                let a = crate::sim::run_one(&spec, &c, seed);
                let b = crate::sim::run_one(&spec, &c, seed);
                assert_eq!(a.log, b.log, "{name} seed {seed}");
                assert_eq!(a.metrics.m1_early, 0, "{name} seed {seed}");
                assert_eq!(a.metrics.m2_contradictory, 0, "{name} seed {seed}");
                assert_eq!(a.metrics.m3_wrong_final, 0, "{name} seed {seed}");
            }
        }
    }
}
