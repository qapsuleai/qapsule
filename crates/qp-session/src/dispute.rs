//! Disputes and the authority's settled-bar hygiene (R3/M7, `docs/PROTOCOL.md` §19.9).
//!
//! The consistency rule (§19) keeps a replica from showing a wrong claim, but a claim the
//! authority derived from its **own** bad feed (a bad tick, an out-of-order event, a
//! split) and asserted as final stayed asserted: the replica showed it `disputed` and
//! the claim was lost (RP2 final audit O1). Two mechanisms, both host-driven and
//! deterministic (DECISIONS K36 on A20):
//!
//! - **Settled-bar hygiene.** With [`Session::set_settle_check`] the authority asks its
//!   host to verify every closed bar of a bound series against the provider's settled
//!   source ([`Session::take_settle_requests`]); the host answers with the settled bars
//!   ([`Session::settled_bars`]). Bars that differ correct the authority's data; the
//!   registered claims resting on them are re-checked (withdrawn with reason
//!   `data_corrected`, or asserted again with the settled values), and condition claims
//!   the settled data now holds at those closed bars are asserted (a correction can make
//!   a claim; the edge logic alone only sees the last bar).
//! - **Dispute.** A replica sends `dispute` (claim, `t`, its values and data digest) when
//!   its clean, refetched data contradicts a final claim. The authority never decides on
//!   the replica's values: it asks its host for settled bars from the anchor to `t` and
//!   re-checks the claim on them. Upheld → withdrawn (`dispute_upheld`); a capsule whose
//!   invalidation claim falls is **retracted** (story stage `retracted`, K37; never in
//!   the ledger). Rejected → the claim stands; its evidence goes back to the disputer as
//!   `asserted` with reason `dispute_rejected`. One dispute per (claim, `t`, digest); at
//!   most [`MAX_OPEN`] open disputes per connection.
//!
//! Hosts without a settled source answer nothing: disputes stay `pending`. The test
//! provider (`init.data.settled`, §18.6) answers synchronously.

use std::collections::BTreeMap;

use qp_schema::capsule::{Stage, Transition};
use qp_schema::data::FeedKey;
use qp_schema::protocol::{
    ChangeOp, ClaimEvidence, ClaimReason, ClaimRef, ClaimState, DisputeVerdict, ErrorCode,
    EvidenceValue, InitSettled, OutboundLine, ProtocolError, QueryResult, ServerMsg, Topic,
};
use qp_schema::{Bar, TimestampMs};

use crate::claims::{assemble, closed, eval_at, ClaimKey, Owner, Pending};
use crate::error::E;
use crate::plane::Provider;
use crate::session::{out, Session};
use crate::values::{bar_series, condition_series, Delta};

/// At most this many open disputes per connection (further ones: `limited`).
pub const MAX_OPEN: usize = 16;
/// At most this many disputes accepted per connection over its life (further ones:
/// `limited`); bounds the idempotence record.
pub const MAX_PER_CONN: usize = 256;
/// At most this many different digests per (connection, claim, `t`) (further ones:
/// `limited`).
pub const MAX_DIGESTS: usize = 4;

/// A digest is exactly 16 lowercase hex digits (`data_digest`).
fn valid_digest(d: &str) -> bool {
    d.len() == 16
        && d.bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}

/// A request for settled bars the authority makes to its host (§19.9): bars of bound
/// series `series` (feed `feed`) with `from_t <= t <= to_t`, from the provider's settled
/// source. The host answers with [`Session::settled_bars`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettleRequest {
    /// The bound bar series.
    pub series: String,
    /// Its feed.
    pub feed: FeedKey,
    /// First bar time (inclusive).
    pub from_t: TimestampMs,
    /// Last bar time (inclusive).
    pub to_t: TimestampMs,
}

/// A `dispute` message's content.
#[derive(Debug, Clone)]
pub(crate) struct DisputeIn {
    pub claim: ClaimRef,
    pub t: TimestampMs,
    pub values: Vec<EvidenceValue>,
    pub anchor_t: Option<TimestampMs>,
    pub digest: Option<String>,
}

/// One open dispute.
#[derive(Debug, Clone)]
struct Open {
    conn: String,
    key: ClaimKey,
    series: String,
    seen: SeenKey,
}

/// Idempotence key: (connection, claim, `t`, digest).
pub(crate) type SeenKey = (String, ClaimRef, TimestampMs, String);

/// Dispute and settled-request state of a session.
#[derive(Debug, Default)]
pub(crate) struct Settle {
    /// Verify every closed bar of a bound series (settled-bar hygiene).
    verify_closed: bool,
    requests: Vec<SettleRequest>,
    /// The test provider's settled source (`init.data.settled`): per feed, settled bars
    /// that replace the provider's.
    test: Option<BTreeMap<FeedKey, BTreeMap<TimestampMs, Bar>>>,
    open: Vec<Open>,
    seen: BTreeMap<SeenKey, DisputeVerdict>,
}

impl Settle {
    /// Whether this dispute (connection, claim, `t`, digest) was made already.
    pub(crate) fn seen_contains(&self, k: &SeenKey) -> bool {
        self.seen.contains_key(k)
    }

    /// Records (or decides) a dispute's verdict.
    pub(crate) fn seen_insert(&mut self, k: SeenKey, v: DisputeVerdict) {
        self.seen.insert(k, v);
    }

    /// A dispute's verdict so far.
    pub(crate) fn seen_get(&self, k: &SeenKey) -> Option<DisputeVerdict> {
        self.seen.get(k).copied()
    }

    /// Queues a settled request for the host.
    pub(crate) fn push_request(&mut self, r: SettleRequest) {
        self.requests.push(r);
    }

    /// Sets the synchronous test source from `init.data.settled`.
    pub(crate) fn set_test_source(&mut self, s: &InitSettled) {
        let mut m: BTreeMap<FeedKey, BTreeMap<TimestampMs, Bar>> = BTreeMap::new();
        for f in &s.feeds {
            let e = m.entry(f.feed.clone()).or_default();
            for b in &f.bars {
                e.insert(b.t, b.clone());
            }
        }
        self.test = Some(m);
        self.verify_closed = s.verify_closed;
    }
}

fn verdict_line(
    conn: &str,
    id: &str,
    seq: u64,
    now: i64,
    verdict: DisputeVerdict,
    evidence: Option<ClaimEvidence>,
) -> OutboundLine {
    out(
        conn,
        ServerMsg::Result {
            id: id.to_owned(),
            seq,
            now,
            data: QueryResult::Dispute { verdict, evidence },
            resolved: BTreeMap::new(),
        },
    )
}

fn reason_code(r: ClaimReason) -> &'static str {
    match r {
        ClaimReason::DataCorrected => "data_corrected",
        ClaimReason::DisputeUpheld => "dispute_upheld",
        ClaimReason::DisputeRejected => "dispute_rejected",
    }
}

impl Session {
    /// (R3, §19.9) Settled-bar hygiene on or off (default off): with it, every bar of a
    /// bound series that closes is queued for verification against the provider's
    /// settled source ([`Session::take_settle_requests`]).
    pub fn set_settle_check(&mut self, on: bool) {
        self.settle.verify_closed = on;
    }

    /// (R3, §19.9) The settled-bar requests made since the last call (closed bars to
    /// verify, disputes to decide), in order. The host fetches them from its provider's
    /// settled source (when the bars are settled) and answers with
    /// [`Session::settled_bars`]; a request it cannot answer yet it may repeat later.
    pub fn take_settle_requests(&mut self) -> Vec<SettleRequest> {
        std::mem::take(&mut self.settle.requests)
    }

    /// (R3, §19.9) The host's answer to a settled request: settled bars of bound series
    /// `series` (any range; bars the authority does not hold are ignored, only closed
    /// bars count). Bars that differ from the authority's correct its data (the claims
    /// resting on them are re-checked: withdrawn with reason `data_corrected`, or
    /// `dispute_upheld` when disputed; a capsule whose claim falls is retracted);
    /// condition claims that the corrected data holds at a closed bar are asserted; open
    /// disputes whose bar the answer covers are decided. Returns the notifications.
    pub fn settled_bars(&mut self, series: &str, bars: &[Bar]) -> Vec<OutboundLine> {
        let Some(bs) = self.plane.series.get_mut(series) else {
            return Vec::new();
        };
        if bs.bound.current_state() != qp_schema::data::BindingState::Ready {
            return Vec::new();
        }
        let Some(store) = bs.bound.store.as_mut() else {
            return Vec::new();
        };
        let held = store.bars();
        let n = held.len();
        if n == 0 {
            return Vec::new();
        }
        // Only closed bars count (the live bar is not settled).
        let last_closed = n.checked_sub(2).map(|i| held[i].t);
        let mut new = held.to_vec();
        let mut first: Option<usize> = None;
        let (mut lo, mut hi) = (i64::MAX, i64::MIN);
        for b in bars {
            if last_closed.is_none_or(|lc| b.t > lc) {
                continue;
            }
            let Ok(i) = new.binary_search_by_key(&b.t, |x| x.t) else {
                continue;
            };
            lo = lo.min(b.t);
            hi = hi.max(b.t);
            if new[i] != *b {
                new[i] = b.clone();
                first = Some(first.map_or(i, |f: usize| f.min(i)));
            }
        }
        if lo > hi {
            return Vec::new();
        }
        let prev = self.claims.ctx_reason.replace(ClaimReason::DataCorrected);
        let lo_i = new.binary_search_by_key(&lo, |x| x.t).unwrap_or(0);
        if let Some(i) = first {
            let seq = store.seq();
            if store.reset(new, seq).is_ok() {
                self.data_changed(vec![(series.to_owned(), Delta::Rebuild { from: i })]);
            }
        }
        // Condition claims the settled data holds anywhere in the answer's range.
        self.scan_conditions(series, lo_i);
        self.decide_disputes(series, lo, hi);
        let retracted = self.retractions();
        self.claims.ctx_reason = prev;
        let mut lines = Vec::new();
        for s in retracted {
            lines.extend(self.entry_events(s));
        }
        // (R3, §20) Engine stories resting on corrected bars are retracted; capsule
        // disputes the answer covers are decided.
        lines.extend(self.capsule_settled(series, lo, hi, first.is_some()));
        lines.extend(self.settle_data());
        lines
    }

    /// (R3, §19.9) Re-resolves every bound series from the host's provider (the host
    /// after it put settled or re-delivered data into its provider, e.g. a split
    /// snapshot): values are recomputed and the registered claims re-checked. Returns the
    /// notifications.
    pub fn resync_bindings(&mut self) -> Vec<OutboundLine> {
        let ids: Vec<String> = self.plane.series.keys().cloned().collect();
        self.plane.series.clear();
        self.pending_stamps = self.plane.sync(&self.doc);
        self.data_changed(
            ids.into_iter()
                .map(|id| (id, Delta::Rebuild { from: 0 }))
                .collect(),
        );
        match self.stamp_anchors() {
            Some(seq) => self.settle(seq),
            None => self.settle_data(),
        }
    }

    /// After bar changes (hygiene on): a bar of a bound series that closed, or a closed
    /// bar the feed corrected (with the closed bars after it), is queued for settled
    /// verification; a feed correction also asserts at once the condition claims it makes
    /// at closed bars (the provider's own correction; the settled answer confirms).
    pub(crate) fn note_closed(&mut self, changed: &[(String, Delta)]) {
        if !self.settle.verify_closed {
            return;
        }
        let mut scans = Vec::new();
        for (id, d) in changed {
            let Some(bs) = self.plane.series.get(id) else {
                continue;
            };
            let Some(bars) = self.plane.bars(id) else {
                continue;
            };
            let feed = bs.binding.feed_key();
            let lo = match *d {
                // Every bar closed by this append: the one before the first new bar and
                // the new ones except the last.
                Delta::Appended { from } => from.saturating_sub(1),
                // A correction of a closed bar: from it to the last closed bar.
                Delta::Revised { index } if index + 1 < bars.len() => index,
                _ => continue,
            };
            let hi = bars.len().saturating_sub(1);
            if lo >= hi {
                continue;
            }
            self.settle.requests.push(SettleRequest {
                series: id.clone(),
                feed,
                from_t: bars[lo].t,
                to_t: bars[hi - 1].t,
            });
            if matches!(d, Delta::Revised { .. }) {
                scans.push((id.clone(), lo));
            }
        }
        for (id, lo) in scans {
            self.scan_conditions(&id, lo);
        }
    }

    /// A `dispute` from connection `ci` (§19.9).
    pub(crate) fn dispute(
        &mut self,
        ci: usize,
        id: &str,
        d: DisputeIn,
    ) -> Result<Vec<OutboundLine>, ProtocolError> {
        if let ClaimRef::Capsule { capsule, .. } = &d.claim {
            // (R3/K1) Routed in the disputer's world: for a public-view reader a capsule
            // only the authority's world has is an absent capsule.
            let public = self.public_view(&self.conns[ci].principal);
            let engine = self.in_world(public, |s| s.engine_capsule(capsule).is_some());
            if engine {
                if let Some(dg) = &d.digest {
                    if !valid_digest(dg) {
                        return Err(E::new(
                            ErrorCode::Invalid,
                            "`digest` must be 16 lowercase hex digits (data_digest)",
                        )
                        .at("/digest")
                        .build());
                    }
                }
                let capsule = capsule.clone();
                return Ok(self.capsule_dispute(ci, id, &capsule, d.t, d.digest));
            }
        }
        let conn = self.conns[ci].id.clone();
        let principal = self.conns[ci].principal.clone();
        let owner = match d.claim {
            ClaimRef::Condition { .. } => Owner::Conn(conn.clone()),
            _ => Owner::Doc,
        };
        let key: ClaimKey = (owner, d.claim.clone(), d.t);
        let (seq, now) = (self.seq_to(&principal, self.seq()), self.clock());
        if let Some(dg) = &d.digest {
            if !valid_digest(dg) {
                return Err(E::new(
                    ErrorCode::Invalid,
                    "`digest` must be 16 lowercase hex digits (data_digest)",
                )
                .at("/digest")
                .build());
            }
        }
        // (R3/K1c) Claims on document items (capsules, intents) are registered in the
        // authority's world only: a public-view reader gets `unknown` for them, whatever
        // the registry holds (as `claim_events` never sends them to it).
        if !matches!(d.claim, ClaimRef::Condition { .. }) && self.public_view(&principal) {
            return Ok(vec![verdict_line(
                &conn,
                id,
                seq,
                now,
                DisputeVerdict::Unknown,
                None,
            )]);
        }
        // The verdict says something about the data: only for a principal entitled to
        // every feed the claim rests on (§19.7). An absent claim and a claim the principal
        // may not hear get the same answer (`unknown`): no existence oracle.
        let ev = self
            .claims
            .registered(&key)
            .filter(|ev| ev.feeds.iter().all(|k| self.plane.entitled(&principal, k)))
            .cloned();
        let Some(ev) = ev else {
            return Ok(vec![verdict_line(
                &conn,
                id,
                seq,
                now,
                DisputeVerdict::Unknown,
                None,
            )]);
        };
        let _ = (&d.values, d.anchor_t);
        let seen: SeenKey = (
            conn.clone(),
            d.claim.clone(),
            d.t,
            d.digest.unwrap_or_default(),
        );
        if self.settle.seen.contains_key(&seen) {
            return Ok(vec![verdict_line(
                &conn,
                id,
                seq,
                now,
                DisputeVerdict::Duplicate,
                None,
            )]);
        }
        let per_conn = self.settle.seen.keys().filter(|k| k.0 == conn).count();
        let per_claim = self
            .settle
            .seen
            .keys()
            .filter(|k| k.0 == conn && k.1 == d.claim && k.2 == d.t)
            .count();
        if self.settle.open.iter().filter(|o| o.conn == conn).count() >= MAX_OPEN
            || per_conn >= MAX_PER_CONN
            || per_claim >= MAX_DIGESTS
        {
            return Ok(vec![verdict_line(
                &conn,
                id,
                seq,
                now,
                DisputeVerdict::Limited,
                None,
            )]);
        }
        self.settle
            .seen
            .insert(seen.clone(), DisputeVerdict::Pending);
        self.claims.disputed.insert(key.clone());
        let feed = self
            .plane
            .series
            .get(&ev.series)
            .map(|b| b.binding.feed_key());
        let from_t = self
            .plane
            .bars(&ev.series)
            .and_then(<[Bar]>::first)
            .map_or(d.t, |b| b.t);
        if let Some(feed) = feed {
            self.settle.requests.push(SettleRequest {
                series: ev.series.clone(),
                feed,
                from_t,
                to_t: d.t,
            });
        }
        self.settle.open.push(Open {
            conn: conn.clone(),
            key: key.clone(),
            series: ev.series.clone(),
            seen: seen.clone(),
        });
        let lines = self.serve_test_source();
        let verdict = self
            .settle
            .seen
            .get(&seen)
            .copied()
            .unwrap_or(DisputeVerdict::Pending);
        let evidence = match verdict {
            DisputeVerdict::Rejected => self.claims.registered(&key).cloned(),
            _ => None,
        };
        let mut v = vec![verdict_line(
            &conn,
            id,
            self.seq_to(&principal, self.seq()),
            self.clock(),
            verdict,
            evidence,
        )];
        v.extend(lines);
        Ok(v)
    }

    /// Answers the queued settled requests from the test provider's settled source, if
    /// the session has one (`init.data.settled`); requests made while answering are
    /// answered too (bounded).
    pub(crate) fn serve_test_source(&mut self) -> Vec<OutboundLine> {
        if self.settle.test.is_none() {
            return Vec::new();
        }
        let mut lines = Vec::new();
        for _ in 0..8 {
            let reqs = std::mem::take(&mut self.settle.requests);
            if reqs.is_empty() {
                break;
            }
            for r in reqs {
                let bars = self.test_settled(&r);
                lines.extend(self.settled_bars(&r.series, &bars));
            }
        }
        lines
    }

    /// The test settled source's bars for `r`: the provider's closed bars in range, the
    /// configured settled bars replacing theirs.
    fn test_settled(&self, r: &SettleRequest) -> Vec<Bar> {
        let Provider::Memory(m) = &self.plane.provider else {
            return Vec::new();
        };
        let Some(store) = m.feed(&r.feed) else {
            return Vec::new();
        };
        let over = self.settle.test.as_ref().and_then(|t| t.get(&r.feed));
        let bars = store.bars();
        let closed = &bars[..bars.len().saturating_sub(1)];
        closed
            .iter()
            .filter(|b| b.t >= r.from_t && b.t <= r.to_t)
            .map(|b| {
                over.and_then(|o| o.get(&b.t))
                    .cloned()
                    .unwrap_or_else(|| b.clone())
            })
            .collect()
    }

    /// Condition claims the (corrected) data of bar series `series` holds at a closed bar
    /// from index `from` on and that are not registered: asserted (final).
    fn scan_conditions(&mut self, series: &str, from: usize) {
        let mut todo: Vec<(String, String, String, qp_schema::Condition)> = Vec::new();
        for c in &self.conns {
            for s in &c.subs {
                if let Topic::Condition { condition } = &s.topic {
                    todo.push((
                        c.id.clone(),
                        c.principal.clone(),
                        s.id.clone(),
                        condition.clone(),
                    ));
                }
            }
        }
        for (conn, principal, sub, cond) in todo {
            if !crate::live::hears(&self.plane, &self.doc, &principal, &cond)
                || self.cond_blocked(&cond)
            {
                continue;
            }
            let rd = self.rd.as_ref().unwrap_or(&self.doc);
            let ops: Vec<String> = condition_series(&cond)
                .iter()
                .map(|(_, s)| (*s).to_owned())
                .collect();
            // Only conditions whose every operand rests on `series` (same bar times).
            if ops.is_empty()
                || !ops
                    .iter()
                    .all(|o| bar_series(rd, o).is_some_and(|b| b.id == series))
            {
                continue;
            }
            let Some(times) = bar_series(rd, series)
                .and_then(qp_schema::Series::bars)
                .map(|b| b.iter().map(|x| x.t).collect::<Vec<_>>())
            else {
                continue;
            };
            let n = times.len();
            if n < 2 {
                continue;
            }
            for &t in &times[from.min(n - 1)..n - 1] {
                let key: ClaimKey = (
                    Owner::Conn(conn.clone()),
                    ClaimRef::Condition { sub: sub.clone() },
                    t,
                );
                if self.claims.registered(&key).is_some() {
                    continue;
                }
                let at: Vec<EvidenceValue> = ops
                    .iter()
                    .map(|o| EvidenceValue {
                        series: o.clone(),
                        t,
                        value: None,
                        prev: None,
                    })
                    .collect();
                let rd = self.rd.as_ref().unwrap_or(&self.doc);
                let Some((true, vals)) = eval_at(rd, &mut self.cache, &cond, Some(&at)) else {
                    continue;
                };
                let rd = self.rd.as_ref().unwrap_or(&self.doc);
                let claims = &self.claims;
                let plane = &self.plane;
                let Some(ev) = assemble(rd, &cond, vals, &|bs, t| claims.unclean(plane, bs, t))
                else {
                    continue;
                };
                self.claims.assert(key.0, key.1, cond.clone(), ev);
            }
        }
    }

    /// Decides the open disputes on `series` whose bar is closed and within `[lo, hi]`
    /// (the settled answer covers it): registered still → rejected (its evidence to the
    /// disputer), withdrawn → upheld.
    fn decide_disputes(&mut self, series: &str, lo: i64, hi: i64) {
        let open = std::mem::take(&mut self.settle.open);
        for o in open {
            let t = o.key.2;
            let rd = self.rd.as_ref().unwrap_or(&self.doc);
            if o.series != series || t < lo || t > hi || !closed(rd, series, t) {
                self.settle.open.push(o);
                continue;
            }
            let verdict = match self.claims.registered(&o.key).cloned() {
                Some(ev) => {
                    self.claims.push(Pending {
                        owner: Owner::Conn(o.conn.clone()),
                        claim: o.key.1.clone(),
                        state: ClaimState::Asserted,
                        evidence: ev,
                        reason: Some(ClaimReason::DisputeRejected),
                    });
                    DisputeVerdict::Rejected
                }
                None => DisputeVerdict::Upheld,
            };
            self.claims.disputed.remove(&o.key);
            self.settle.seen.insert(o.seen, verdict);
        }
    }

    /// Engine entries retracting the capsules whose claim the queued events withdraw for
    /// a reason (settled data, a dispute): K37, story stage `retracted`, never in the
    /// ledger. A capsule without a story (pre-R3) gets no entry (only the withdrawal).
    fn retractions(&mut self) -> Vec<u64> {
        let todo: Vec<(String, i64, ClaimReason)> = self
            .claims
            .queued()
            .iter()
            .filter(|p| p.state == ClaimState::Withdrawn)
            .filter_map(|p| match (&p.claim, p.reason) {
                (ClaimRef::Capsule { capsule, .. }, Some(r)) => {
                    Some((capsule.clone(), p.evidence.t, r))
                }
                _ => None,
            })
            .collect();
        let mut out = Vec::new();
        for (cap, t, reason) in todo {
            let Some(mut c) = self.doc.capsules.iter().find(|c| c.id == cap).cloned() else {
                continue;
            };
            let Some(story) = c.story.as_mut() else {
                continue;
            };
            if story.stage == Stage::Retracted {
                continue;
            }
            let rd = self.rd.as_ref().unwrap_or(&self.doc);
            let bar = bar_series(rd, &story.series)
                .and_then(qp_schema::Series::bars)
                .and_then(|b| b.binary_search_by_key(&t, |x| x.t).ok())
                .map_or(story.last_bar, |i| i as u64);
            story.stage = Stage::Retracted;
            story.last_bar = story.last_bar.max(bar);
            story.history.push(Transition {
                stage: Stage::Retracted,
                bar,
                t,
                facts: Vec::new(),
                reason: reason_code(reason).to_owned(),
            });
            if let Some(s) = self.engine_entry_with(
                vec![ChangeOp::PutCapsule {
                    capsule: c,
                    index: None,
                }],
                format!("retract {cap}: {}", reason_code(reason)),
                None,
            ) {
                out.push(s);
            }
        }
        out
    }
}
