//! Claim evidence and the authority's claim lifecycle (RP2/T8, `docs/PROTOCOL.md` §19).
//!
//! A *claim* is a meaning the authority derives from data: a condition subscription
//! turning true, an intent's trigger turning true, a live capsule's invalidation holding.
//! The engine evaluates conditions at the last bar; this module adds, without any extra
//! computation (the values are the cache columns the evaluation just used):
//!
//! - [`evidence`]: the claim's evidence — bar series, bar time `t`, each operand's value
//!   at `t` and at the bar before, maturity (`final` once the bar is closed on the
//!   authority's data), cleanliness (no unrepaired sequence gap up to `t`) and anchor;
//! - [`eval_at`]: the same predicate at a given bar (by time) on a side's own data — the
//!   authority uses it to finalise or withdraw a claim when its bar closes or a
//!   correction lands, the replica (`crate::replica`) to re-check a claim on its own data;
//! - [`Claims`]: the registry of asserted claims and the queue of `claim` events for the
//!   `claims` topic (provisional at the edge, final at the bar's close, withdrawn, clean
//!   changes after a gap).
//!
//! Determinism: no clock, no hash maps; the queue is filled in evaluation order, which is
//! the same on the incremental and the from-scratch path.

use std::collections::BTreeMap;

use qp_schema::data::BindingState;
use qp_schema::protocol::{ClaimEvidence, ClaimRef, ClaimState, EvidenceValue, Maturity};
use qp_schema::{CompareOp, Condition, Document, SeriesSource};

use crate::plane::DataPlane;
use crate::values::{bar_series, condition_series, find_series, values, Cache, Delta};

/// At most this many claims are remembered for finalisation (oldest dropped first).
pub(crate) const MAX_CLAIMS: usize = 4096;

/// Who receives a claim's events: the connection that holds the condition subscription,
/// or every `claims` subscriber (document claims: intents, capsules).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Owner {
    /// A connection id.
    Conn(String),
    /// The document.
    Doc,
}

/// A registered claim.
#[derive(Debug, Clone)]
struct Reg {
    cond: Condition,
    ev: ClaimEvidence,
    /// Registration order (eviction).
    n: u64,
}

/// A sequence gap on a bound series that no resolution has repaired yet.
#[derive(Debug, Clone, Copy)]
struct Gap {
    /// The `seq` the store waited for.
    expected: u64,
    /// Bars from this time on may lack events (the last bar held when the gap appeared).
    from_t: i64,
}

/// One `claim` event waiting for the `claims` topic.
#[derive(Debug, Clone)]
pub(crate) struct Pending {
    pub owner: Owner,
    pub claim: ClaimRef,
    pub state: ClaimState,
    pub evidence: ClaimEvidence,
}

/// The authority's claims (registry, gap marks, queue).
#[derive(Debug, Default)]
pub(crate) struct Claims {
    reg: BTreeMap<(Owner, ClaimRef, i64), Reg>,
    gaps: BTreeMap<String, Gap>,
    pending: Vec<Pending>,
    counter: u64,
}

/// The value of `id` at index `i` and `i - 1`, with the bar time of `i`.
fn at_index(rd: &Document, cache: &mut Cache, id: &str, i: Option<usize>) -> Option<EvidenceValue> {
    let c = values(rd, cache, id).ok()?;
    let n = c.len();
    let i = match i {
        Some(i) if i < n => i,
        Some(_) => return None,
        None => n.checked_sub(1)?,
    };
    let main = c.main_values();
    Some(EvidenceValue {
        series: id.to_owned(),
        t: c.t[i],
        value: main.get(i).copied().flatten(),
        prev: i
            .checked_sub(1)
            .and_then(|j| main.get(j).copied().flatten()),
    })
}

/// The value of `id` at the bar with time `t` (and the bar before), if `rd` holds it.
fn at_time(rd: &Document, cache: &mut Cache, id: &str, t: i64) -> Option<EvidenceValue> {
    let c = values(rd, cache, id).ok()?;
    let i = c.t.binary_search(&t).ok()?;
    at_index(rd, cache, id, Some(i))
}

/// The predicate of `op` on evidence values: exactly the engine's condition semantics
/// (`values::eval_on`): the current pair must be known, crossings need the previous pair.
fn holds_on(op: CompareOp, left: &EvidenceValue, right: Option<&EvidenceValue>, k: f64) -> bool {
    let (rv, rp) = match right {
        Some(r) => (r.value, r.prev),
        None => (Some(k), Some(k)),
    };
    let (Some(a), Some(b)) = (left.value, rv) else {
        return false;
    };
    let prev = match (left.prev, rp) {
        (Some(x), Some(y)) => Some((x, y)),
        _ => None,
    };
    crate::values::cmp(op, prev, (a, b))
}

/// The bar series a claim on `c` rests on (that of its first operand).
fn claim_series(rd: &Document, c: &Condition) -> Option<String> {
    let first = condition_series(c).first().map(|(_, s)| (*s).to_owned())?;
    bar_series(rd, &first).map(|s| s.id.clone())
}

/// True if bar `t` of bar series `bs` is closed on `rd`: a later bar exists, or the
/// series is bound with a closed window (`to_t`: no live bar).
pub(crate) fn closed(rd: &Document, bs: &str, t: i64) -> bool {
    let Some(s) = find_series(rd, bs) else {
        return false;
    };
    if let SeriesSource::Bind { window, .. } = &s.source {
        if window.to_t.is_some() {
            return true;
        }
    }
    s.bars()
        .and_then(<[qp_schema::Bar]>::last)
        .is_some_and(|b| b.t > t)
}

/// The anchor of bound bar series `bs` in `rd` (`None`: inline or not anchored yet).
pub(crate) fn anchor_of(rd: &Document, bs: &str) -> Option<i64> {
    match &find_series(rd, bs)?.source {
        SeriesSource::Bind { anchor_t, .. } => *anchor_t,
        _ => None,
    }
}

/// Builds the evidence of `c` from operand values `vals`: maturity is `final` only if
/// every operand's bar is closed; `clean` only if no operand's bar series has an
/// unrepaired gap at or before its bar (`unclean`: bar series → first suspect time).
pub(crate) fn assemble(
    rd: &Document,
    c: &Condition,
    vals: Vec<EvidenceValue>,
    unclean: &dyn Fn(&str, i64) -> bool,
) -> Option<ClaimEvidence> {
    let series = claim_series(rd, c)?;
    let t = vals.first()?.t;
    let mut fin = true;
    let mut clean = true;
    for v in &vals {
        let bs = bar_series(rd, &v.series).map_or_else(|| v.series.clone(), |s| s.id.clone());
        fin &= closed(rd, &bs, v.t);
        clean &= !unclean(&bs, v.t);
    }
    let mut ids: Vec<&str> = vec![series.as_str()];
    ids.extend(vals.iter().map(|v| v.series.as_str()));
    let feeds = feeds_of(rd, &ids)?;
    Some(ClaimEvidence {
        feeds,
        anchor_t: anchor_of(rd, &series),
        series,
        t,
        values: vals,
        maturity: if fin {
            Maturity::Final
        } else {
            Maturity::Provisional
        },
        clean,
    })
}

/// Evaluates condition `c` at the bars named by `at` (one entry per operand, by time;
/// `None`: at the last bar) on `rd`. Returns whether it holds and the operand values,
/// or `None` if an operand's bar is not held (not covered) or `c` is a text condition.
pub(crate) fn eval_at(
    rd: &Document,
    cache: &mut Cache,
    c: &Condition,
    at: Option<&[EvidenceValue]>,
) -> Option<(bool, Vec<EvidenceValue>)> {
    let ops = condition_series(c);
    if ops.is_empty() {
        return None;
    }
    let mut vals = Vec::with_capacity(ops.len());
    for (k, (_, id)) in ops.iter().enumerate() {
        let v = match at {
            Some(a) => at_time(rd, cache, id, a.get(k)?.t)?,
            None => at_index(rd, cache, id, None)?,
        };
        vals.push(v);
    }
    let ok = match c {
        Condition::Threshold { op, value, .. } => holds_on(*op, &vals[0], None, *value),
        Condition::Compare { op, .. } => holds_on(*op, &vals[0], vals.get(1), 0.0),
        Condition::Text { .. } => return None,
    };
    Some((ok, vals))
}

/// The evidence of condition `c` evaluated at the last bar (an edge just fired): the
/// cached values the evaluation used, no recomputation.
pub(crate) fn evidence(
    rd: &Document,
    cache: &mut Cache,
    claims: &Claims,
    plane: &DataPlane,
    c: &Condition,
) -> Option<ClaimEvidence> {
    let (_, vals) = eval_at(rd, cache, c, None)?;
    assemble(rd, c, vals, &|bs, t| claims.unclean(plane, bs, t))
}

impl Claims {
    /// True if bar series `bs` has an unrepaired gap at or before `t` on the authority
    /// (`plane`: a gap repaired by a later in-order event clears lazily).
    pub(crate) fn unclean(&self, plane: &DataPlane, bs: &str, t: i64) -> bool {
        let Some(g) = self.gaps.get(bs) else {
            return false;
        };
        if repaired(plane, bs, g) {
            return false;
        }
        t >= g.from_t
    }

    /// Records an asserted claim and queues its `claim` event.
    pub(crate) fn assert(
        &mut self,
        owner: Owner,
        claim: ClaimRef,
        cond: Condition,
        ev: ClaimEvidence,
    ) {
        self.counter += 1;
        let key = (owner.clone(), claim.clone(), ev.t);
        self.reg.insert(
            key,
            Reg {
                cond,
                ev: ev.clone(),
                n: self.counter,
            },
        );
        if self.reg.len() > MAX_CLAIMS {
            if let Some(k) = self
                .reg
                .iter()
                .min_by_key(|(_, r)| r.n)
                .map(|(k, _)| k.clone())
            {
                self.reg.remove(&k);
            }
        }
        self.pending.push(Pending {
            owner,
            claim,
            state: ClaimState::Asserted,
            evidence: ev,
        });
    }

    /// Takes the queued events.
    pub(crate) fn take(&mut self) -> Vec<Pending> {
        std::mem::take(&mut self.pending)
    }

    /// After the bars of bar series changed (`changed`, `rd` and the cache already
    /// updated): gap marks follow the plane; registered claims on a changed series whose
    /// bar closed, was corrected, or whose data was rebuilt are re-checked at their bar
    /// on the authority's data — still holding with different evidence (now final, new
    /// values, other cleanliness): asserted again; not holding: withdrawn. Live revisions
    /// of the last bar re-check nothing (the edge logic handles the live bar). While a
    /// series holds no bars (stale), its claims that a gap makes unclean are re-asserted
    /// with `clean: false` and their last values.
    pub(crate) fn after_bars(
        &mut self,
        rd: &Document,
        cache: &mut Cache,
        plane: &DataPlane,
        changed: &[(String, Delta)],
    ) {
        if self.reg.is_empty() && self.gaps.is_empty() && plane.series.is_empty() {
            return;
        }
        for (id, d) in changed {
            self.follow_gap(plane, id);
            if self.reg.is_empty() {
                continue;
            }
            let bars = find_series(rd, id).and_then(qp_schema::Series::bars);
            let n = bars.map_or(0, <[qp_schema::Bar]>::len);
            // Claims at or after this time are re-checked (`None`: none).
            let from_t: Option<i64> = match (bars, *d) {
                (None, _) => Some(i64::MIN),
                (Some(b), Delta::Appended { from }) => {
                    from.checked_sub(1).and_then(|i| b.get(i)).map(|x| x.t)
                }
                (Some(_), Delta::Revised { index }) if index + 1 >= n => None,
                (Some(b), Delta::Revised { index }) => b.get(index).map(|x| x.t),
                (Some(b), Delta::Rebuild { from }) => Some(match from {
                    0 => i64::MIN,
                    i => b.get(i).map_or(i64::MIN, |x| x.t),
                }),
            };
            let Some(from_t) = from_t else { continue };
            let last_t = bars.and_then(<[qp_schema::Bar]>::last).map(|b| b.t);
            let keys: Vec<(Owner, ClaimRef, i64)> = self
                .reg
                .iter()
                .filter(|((_, _, t), r)| r.ev.series == *id && *t >= from_t)
                .map(|(k, _)| k.clone())
                .collect();
            for key in keys {
                let Some(reg) = self.reg.get(&key).cloned() else {
                    continue;
                };
                // A live claim on a live revision or a rebuild that keeps the live bar:
                // only a closed bar (or a lost one) changes it.
                let rebuild = matches!(d, Delta::Rebuild { .. });
                if bars.is_some() && Some(reg.ev.t) == last_t && !rebuild {
                    continue;
                }
                let next = if bars.is_none() {
                    // No bars (stale, not ready): only cleanliness can be stated.
                    if reg.ev.clean && self.unclean(plane, id, reg.ev.t) {
                        let mut ev = reg.ev.clone();
                        ev.clean = false;
                        Some((ClaimState::Asserted, ev))
                    } else {
                        None
                    }
                } else {
                    match eval_at(rd, cache, &reg.cond, Some(&reg.ev.values)) {
                        Some((true, vals)) => {
                            let ev =
                                assemble(rd, &reg.cond, vals, &|bs, t| self.unclean(plane, bs, t));
                            ev.filter(|e| *e != reg.ev)
                                .map(|e| (ClaimState::Asserted, e))
                        }
                        Some((false, vals)) => {
                            let ev =
                                assemble(rd, &reg.cond, vals, &|bs, t| self.unclean(plane, bs, t))
                                    .unwrap_or_else(|| reg.ev.clone());
                            Some((ClaimState::Withdrawn, ev))
                        }
                        // The claim's bar is gone (truncation): withdrawn with its last
                        // evidence.
                        None => Some((ClaimState::Withdrawn, reg.ev.clone())),
                    }
                };
                let Some((state, ev)) = next else { continue };
                match state {
                    ClaimState::Withdrawn => {
                        self.reg.remove(&key);
                    }
                    ClaimState::Asserted => {
                        if let Some(r) = self.reg.get_mut(&key) {
                            r.ev = ev.clone();
                        }
                    }
                }
                self.pending.push(Pending {
                    owner: key.0.clone(),
                    claim: key.1.clone(),
                    state,
                    evidence: ev,
                });
            }
        }
    }

    /// Gap marks follow the plane: a bound series that went stale on a `seq` gap is
    /// marked from its last bar held; a mark is dropped once the store's position covers
    /// the missing event (a re-resolution that included it, or the event itself arriving
    /// in order after a re-resolution).
    fn follow_gap(&mut self, plane: &DataPlane, id: &str) {
        let Some(bs) = plane.series.get(id) else {
            return;
        };
        let Some(store) = bs.bound.store.as_ref() else {
            return;
        };
        if let Some(qp_source::StaleReason::Gap { expected, .. }) = store.stale() {
            if !self.gaps.contains_key(id) {
                let from_t = store.bars().last().map_or(i64::MIN, |b| b.t);
                self.gaps.insert(id.to_owned(), Gap { expected, from_t });
            }
            return;
        }
        if self.gaps.get(id).is_some_and(|g| repaired(plane, id, g)) {
            self.gaps.remove(id);
        }
    }
}

/// True if the store of bound series `id` is ready and its position covers the gap's
/// missing event.
fn repaired(plane: &DataPlane, id: &str, g: &Gap) -> bool {
    plane.series.get(id).is_some_and(|bs| {
        bs.bound.current_state() == BindingState::Ready
            && bs
                .bound
                .store
                .as_ref()
                .is_some_and(|s| s.seq() >= g.expected)
    })
}

impl crate::session::Session {
    /// Evidence of condition `c` (an edge just fired at the last bar) and the claim's
    /// registration for the `claims` topic.
    pub(crate) fn claim_asserted(
        &mut self,
        owner: Owner,
        claim: ClaimRef,
        c: &Condition,
    ) -> Option<ClaimEvidence> {
        let rd = self.rd.as_ref().unwrap_or(&self.doc);
        let ev = evidence(rd, &mut self.cache, &self.claims, &self.plane, c)?;
        self.claims.assert(owner, claim, c.clone(), ev.clone());
        Some(ev)
    }

    /// Claim follow-up after bar changes (see [`Claims::after_bars`]).
    pub(crate) fn claims_after_bars(&mut self, changed: &[(String, Delta)]) {
        let rd = self.rd.as_ref().unwrap_or(&self.doc);
        self.claims
            .after_bars(rd, &mut self.cache, &self.plane, changed);
    }

    /// The queued `claim` events for `claims` subscribers: a condition claim to its own
    /// connection, a document claim to every subscriber; per event, by connection and
    /// subscription order. A connection that is gone receives nothing.
    pub(crate) fn claim_events(&mut self) -> Vec<qp_schema::protocol::OutboundLine> {
        let pend = self.claims.take();
        if pend.is_empty() {
            return Vec::new();
        }
        let seq = self.seq();
        let mut lines = Vec::new();
        for p in pend {
            for c in &self.conns {
                if let Owner::Conn(id) = &p.owner {
                    if *id != c.id {
                        continue;
                    }
                }
                for s in &c.subs {
                    if !matches!(s.topic, qp_schema::protocol::Topic::Claims {}) {
                        continue;
                    }
                    lines.push(crate::session::out(
                        &c.id,
                        qp_schema::protocol::ServerMsg::Notify {
                            sub: s.id.clone(),
                            event: qp_schema::protocol::Event::Claim {
                                claim: p.claim.clone(),
                                state: p.state,
                                evidence: for_principal(
                                    &self.doc,
                                    &self.plane,
                                    &c.principal,
                                    &p.evidence,
                                ),
                                seq,
                            },
                        },
                    ));
                }
            }
        }
        lines
    }
}

/// The bound feeds series `ids` rest on in `doc` (through study inputs), sorted and
/// unique; `None` if a series cannot be resolved (then no evidence is made: a claim whose
/// data dependencies are unknown cannot be gated, §19.7).
pub(crate) fn feeds_of(doc: &Document, ids: &[&str]) -> Option<Vec<qp_schema::data::FeedKey>> {
    let mut out: Vec<qp_schema::data::FeedKey> = Vec::new();
    let mut stack: Vec<&str> = ids.to_vec();
    let mut seen = 0usize;
    while let Some(id) = stack.pop() {
        seen += 1;
        if seen > 1024 {
            return None;
        }
        let s = find_series(doc, id)?;
        match &s.source {
            SeriesSource::Bind { .. } => {
                let k = s.binding()?.feed_key();
                if !out.contains(&k) {
                    out.push(k);
                }
            }
            SeriesSource::Study { inputs, .. } => stack.extend(inputs.iter().map(String::as_str)),
            _ => {}
        }
    }
    out.sort();
    Some(out)
}

/// True if `principal` holds the data entitlement of every feed recorded in the evidence
/// (fail closed: no provider, or a feed the provider does not grant, means no).
fn entitled_all(plane: &DataPlane, principal: &str, ev: &ClaimEvidence) -> bool {
    ev.feeds.iter().all(|k| plane.entitled(principal, k))
}

/// The evidence as `principal` may receive it (§18.3: data-plane values do not go to an
/// unentitled principal, meaning does): without the entitlement of every bound feed it
/// rests on, every operand's `value` and `prev` are removed; series ids, `t`, maturity,
/// cleanliness and the anchor stay.
pub(crate) fn for_principal(
    doc: &Document,
    plane: &DataPlane,
    principal: &str,
    ev: &ClaimEvidence,
) -> ClaimEvidence {
    let mut ev = ev.clone();
    let _ = doc;
    if !entitled_all(plane, principal, &ev) {
        for v in &mut ev.values {
            v.value = None;
            v.prev = None;
        }
    }
    ev
}

/// An event as `principal` may receive it: its claim evidence filtered by
/// [`for_principal`] (events without evidence unchanged).
pub(crate) fn event_for(
    doc: &Document,
    plane: &DataPlane,
    principal: &str,
    e: &qp_schema::protocol::Event,
) -> qp_schema::protocol::Event {
    use qp_schema::protocol::Event;
    let mut e = e.clone();
    match &mut e {
        Event::Change { evidence, .. }
        | Event::FindingState { evidence, .. }
        | Event::ConditionMet { evidence, .. }
        | Event::IntentTriggered { evidence, .. } => {
            if let Some(ev) = evidence.as_mut() {
                *ev = for_principal(doc, plane, principal, ev);
            }
        }
        Event::Claim { evidence, .. } => *evidence = for_principal(doc, plane, principal, evidence),
        _ => {}
    }
    e
}
