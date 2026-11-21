//! The replica (RP2/T8, `docs/PROTOCOL.md` §19): a device-side follower of an authority.
//!
//! A [`Replica`] holds a follower [`Session`] — the **same engine code** as the authority:
//! the same document application ([`crate::live::apply_in_place`] + `Session::committed`),
//! the same data plane (`DataPlane`, `qp_source::FeedStore` with the §18.4 rules), the same
//! values cache and live path (`Session::data_changed`), the same condition semantics
//! ([`crate::claims::eval_at`]) and the same grammar compile (`Session::scene`). The
//! follower never writes engine entries, never stamps anchors and evaluates no watchers;
//! the document only changes by mirroring the authority's `change` events.
//!
//! - **Document.** On `welcome` it asks for `changes` and `claims` subscriptions and the
//!   document; then applies `change` events in `seq` order; a gap is caught up with a
//!   `log` query ([`Replica::take_outbox`] holds the messages for the host to send).
//! - **Data.** Its bound series are resolved by **its own** source provider, with its own
//!   principal's entitlement, from the document's anchor (§18.3); the host delivers its
//!   own feed's events ([`Replica::data_event`]) and re-resolves after a gap or a settled
//!   fetch ([`Replica::resync`]).
//! - **Claims.** It keeps the authority's latest word per claim (`claim` events; evidence
//!   on `change` as a fallback) and derives a deterministic display state
//!   ([`Replica::display`]) by the display rule (§19.4), re-checked after every input:
//!   a claim is shown only when its own data covers `t` with the same anchor and the
//!   claim's predicate holds **on its own data**; `final` needs its own bar closed, the
//!   authority's final and a clean side; values shown are its own.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use qp_schema::data::{BindingState, DataEvent};
use qp_schema::protocol::{
    ClaimEvidence, ClaimRef, ClaimState, DocumentRef, Event, EvidenceValue, LogEntry, Maturity,
    Principal, ProtocolError, QueryResult, Role, Seq, ServerMsg, SessionInit,
};
use qp_schema::{Author, Bar, Condition, Document, Series};
use serde::Serialize;

use crate::claims::{anchor_of, closed, eval_at};
use crate::plane::{reading_doc, DataError, DataPlane, Provider};
use crate::session::Session;
use crate::values::{bar_series, Delta};

/// At most this many claims are remembered (oldest dropped first).
const MAX_CLAIMS: usize = 4096;

const SUB_CHANGES: &str = "replica:changes";
const SUB_CLAIMS: &str = "replica:claims";
const Q_DOC: &str = "replica:doc";
const Q_LOG: &str = "replica:log";

/// FNV-1a 64 over `bytes` (standard offset basis and prime).
#[must_use]
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// Document digest (§19.6): FNV-1a 64 of the document's JSON bytes (`serde_json`, the
/// canonical field order of the schema types), 16 lowercase hex digits.
#[must_use]
pub fn document_digest(doc: &Document) -> String {
    let bytes = serde_json::to_vec(doc).unwrap_or_default();
    format!("{:016x}", fnv1a64(&bytes))
}

/// View digest (§19.6): FNV-1a 64 of the JSON bytes of view `view`'s scene description
/// at `width` × `height`, compiled by `session` over its own data.
///
/// # Errors
/// Unknown view or a compile error.
pub fn view_digest(
    session: &mut Session,
    view: &str,
    width: u32,
    height: u32,
) -> Result<String, ProtocolError> {
    let scene = session.scene(view, &qp_grammar::CompileOptions::new(width, height))?;
    let bytes = serde_json::to_vec(&scene).unwrap_or_default();
    Ok(format!("{:016x}", fnv1a64(&bytes)))
}

/// What the replica shows for one claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Shown {
    /// The authority found something; its own data does not (yet) cover or confirm it.
    /// Nothing is asserted.
    Pending,
    /// Asserted, with a label (`provisional` / `final`) and its own values.
    Shown,
    /// Its clean, refetched own data contradicts the claim: not shown as a claim, only
    /// "sources differ".
    Disputed,
}

/// One item of the display state.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DisplayItem {
    /// What the claim is about.
    pub claim: ClaimRef,
    /// The claim's bar series.
    pub series: String,
    /// The claim's bar time.
    pub t: i64,
    /// Pending, shown or disputed.
    pub state: Shown,
    /// For `shown`: provisional or final.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<Maturity>,
    /// Its own values at `t` (empty when its data does not cover `t`).
    pub values: Vec<EvidenceValue>,
    /// Why it is pending (or disputed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<&'static str>,
}

/// Display policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplicaOptions {
    /// Show provisional claims on the live bar (the authority's provisional claim and the
    /// predicate on its own live bar). Off by default: a live-bar claim cannot be shown
    /// without error (§19.5, `T7-COMPARE.md` common finding 3); a product decision.
    pub live: bool,
    /// On disagreement on clean data, ask the host for a settled refetch
    /// ([`Replica::take_refetch`]) before marking the claim disputed. Off: disputed at once.
    pub refetch: bool,
    /// (R3, §19.9) When a claim becomes `disputed`, send the authority a `dispute`
    /// ([`Replica::take_outbox`]) once per (claim, `t`, authority's evidence): a changed
    /// own data digest alone does not dispute again (no dispute storm). Off by default
    /// (the host turns it on).
    pub dispute: bool,
}

impl Default for ReplicaOptions {
    fn default() -> Self {
        Self {
            live: false,
            refetch: true,
            dispute: false,
        }
    }
}

#[derive(Debug, Clone)]
struct Known {
    state: ClaimState,
    ev: ClaimEvidence,
    n: u64,
}

/// A replica of one document (see the module docs).
pub struct Replica {
    principal: String,
    /// The provider until the document arrives (then it lives in the session's plane).
    provider: Option<Provider>,
    s: Option<Session>,
    /// (R4/H-FP) The calendar of the session's headline pieces (kept across reloads).
    headline_cal: Option<qp_schema::Calendar>,
    seq: Seq,
    /// `change` events received before the document or after a gap.
    buffered: BTreeMap<Seq, LogEntry>,
    log_asked: bool,
    doc_asked: bool,
    claims: BTreeMap<(ClaimRef, i64), Known>,
    counter: u64,
    watches: BTreeMap<String, Condition>,
    opts: ReplicaOptions,
    unclean: BTreeMap<String, i64>,
    settled: BTreeMap<String, i64>,
    refetch_asked: BTreeSet<(String, i64)>,
    refetch_new: Vec<(String, i64)>,
    refetched: BTreeSet<(String, i64)>,
    outbox: Vec<serde_json::Value>,
    display: Vec<DisplayItem>,
    /// Bar series each displayed claim was judged on (a data change re-judges only the
    /// claims resting on a changed series).
    roots: BTreeMap<(ClaimRef, i64), Vec<String>>,
    display_version: u64,
    /// Capsule claims learnt from a mirrored entry's evidence (fallback).
    pending_learn: Vec<(ClaimRef, ClaimEvidence)>,
    /// (R3) Disputes sent: (claim, `t`, the authority's evidence as JSON).
    disputes_sent: BTreeSet<(ClaimRef, i64, String)>,
    dispute_counter: u64,
}

impl std::fmt::Debug for Replica {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Replica")
            .field("principal", &self.principal)
            .field("seq", &self.seq)
            .field("claims", &self.claims.len())
            .finish_non_exhaustive()
    }
}

/// A follower session over `doc` whose bound series are resolved by `provider` with
/// `principal`'s entitlement, from the document's anchors (no stamping, no engine entry).
fn follower(doc: Document, principal: &str, provider: Provider) -> Result<Session, String> {
    let init = SessionInit {
        document: DocumentRef::Inline(Box::new(doc.clone())),
        principals: vec![Principal {
            id: principal.to_owned(),
            author: Author::Human {
                name: principal.to_owned(),
            },
            role: Role::Viewer,
            on_behalf: None,
        }],
        clock: 0,
        epoch: 0,
        proposal_ttl_ms: None,
        data: None,
        capsules: None,
        replay: None,
        feed_rule: None,
    };
    // Built without a provider: every binding is pending, so nothing is stamped.
    let mut s = Session::build_with(
        init,
        Path::new("."),
        Provider::None,
        serde_json::Value::Null,
    )?;
    s.doc = doc.clone();
    s.initial = doc;
    s.log.clear();
    s.plane = DataPlane::new(provider, Some(principal.to_owned()), s.limits.max_bind_bars);
    let _ = s.plane.sync(&s.doc);
    s.rd = reading_doc(&s.doc, &s.plane);
    s.cache.clear();
    s.view_reads = None;
    s.pending_views.clear();
    s.pending_stamps.clear();
    let _ = s.claims.take();
    Ok(s)
}

impl Replica {
    /// A replica for `principal` whose data comes from the host's `provider`.
    #[must_use]
    pub fn new(principal: &str, provider: Box<dyn qp_source::SourceProvider>) -> Self {
        Self::with(principal, Provider::Host(provider))
    }

    /// A replica whose provider is a [`qp_source::MemoryProvider`] that also takes the
    /// data events the host delivers (so a re-resolution sees them): tests, simple hosts,
    /// the WASM binding.
    #[must_use]
    pub fn with_memory(principal: &str, provider: qp_source::MemoryProvider) -> Self {
        Self::with(principal, Provider::Memory(provider))
    }

    fn with(principal: &str, provider: Provider) -> Self {
        Self {
            principal: principal.to_owned(),
            provider: Some(provider),
            s: None,
            headline_cal: None,
            seq: 0,
            buffered: BTreeMap::new(),
            log_asked: false,
            doc_asked: false,
            claims: BTreeMap::new(),
            counter: 0,
            watches: BTreeMap::new(),
            opts: ReplicaOptions::default(),
            unclean: BTreeMap::new(),
            settled: BTreeMap::new(),
            refetch_asked: BTreeSet::new(),
            refetch_new: Vec::new(),
            refetched: BTreeSet::new(),
            outbox: Vec::new(),
            display: Vec::new(),
            roots: BTreeMap::new(),
            display_version: 0,
            pending_learn: Vec::new(),
            disputes_sent: BTreeSet::new(),
            dispute_counter: 0,
        }
    }

    /// Starts from a document the host already holds (a snapshot, e.g. after a hand-over
    /// or from local storage) at log position `seq`; later `change` events continue it.
    pub fn start(&mut self, doc: Document, seq: Seq) {
        self.load(doc, seq);
        self.reconcile();
    }

    /// Sets the display policy.
    pub fn set_options(&mut self, opts: ReplicaOptions) {
        self.opts = opts;
        self.reconcile();
    }

    /// The display policy.
    #[must_use]
    pub fn options(&self) -> ReplicaOptions {
        self.opts
    }

    /// Watches condition `condition` as subscription `sub` (queues the `subscribe`): its
    /// claims arrive on the `claims` topic as `{"kind":"condition","sub":…}`.
    pub fn watch(&mut self, sub: &str, condition: Condition) {
        self.outbox.push(serde_json::json!({
            "type": "subscribe", "id": sub,
            "topic": {"kind": "condition", "condition": condition}
        }));
        self.watches.insert(sub.to_owned(), condition);
    }

    /// Client messages for the host to send to the authority on this replica's
    /// connection, in order (subscriptions, the document, `log` catch-up).
    pub fn take_outbox(&mut self) -> Vec<serde_json::Value> {
        std::mem::take(&mut self.outbox)
    }

    /// The mirrored log position.
    #[must_use]
    pub fn seq(&self) -> Seq {
        self.seq
    }

    /// The mirrored document (`None` before the first `document` answer).
    #[must_use]
    pub fn document(&self) -> Option<&Document> {
        self.s.as_ref().map(Session::document)
    }

    /// The follower session (read access: queries over the replica's own data).
    pub fn session_mut(&mut self) -> Option<&mut Session> {
        self.s.as_mut()
    }

    /// (R4/H-FP) The exchange calendar of its session's headline pieces
    /// ([`Session::set_headline_calendar`]), kept across reloads.
    pub fn set_headline_calendar(&mut self, cal: Option<qp_schema::Calendar>) {
        if let Some(s) = self.s.as_mut() {
            s.set_headline_calendar(cal.clone());
        }
        self.headline_cal = cal;
    }

    /// Its own bars of bound or inline bar series `series` (what its chart shows).
    #[must_use]
    pub fn bars(&self, series: &str) -> Option<&[Bar]> {
        let s = self.s.as_ref()?;
        crate::values::find_series(rd!(s), series).and_then(qp_schema::Series::bars)
    }

    /// (R3, K28, §20) Capsule candidates on the open (last) bar of bar series `series`,
    /// computed from its **own** data ([`crate::capsule::candidates`]: public detectors
    /// and `Engine::candidates`). Shown only on this replica; never sent to the
    /// authority, never in its log or ledger; a candidate whose formation the next tick
    /// undoes simply disappears.
    #[must_use]
    pub fn candidates(&self, series: &str) -> Vec<qp_capsule::CandidateStory> {
        let Some(s) = self.s.as_ref() else {
            return Vec::new();
        };
        let Some(bs) = crate::values::find_series(rd!(s), series) else {
            return Vec::new();
        };
        let (Some(bars), Some(symbol)) = (bs.bars(), bs.symbol()) else {
            return Vec::new();
        };
        // (RP4) The record the authority runs, read from its stories (default `r3`).
        let engine = crate::capsule::engine_of_doc(rd!(s));
        crate::capsule::candidates_with(engine, symbol, series, bars)
    }

    /// Its binding state of bound series `series`.
    #[must_use]
    pub fn binding_state(&self, series: &str) -> Option<BindingState> {
        self.s.as_ref()?.binding_state(series)
    }

    /// [`document_digest`] of the mirrored document (`None` before it arrives).
    #[must_use]
    pub fn document_digest(&self) -> Option<String> {
        self.document().map(document_digest)
    }

    /// [`view_digest`] of view `view` over its own data.
    ///
    /// # Errors
    /// No document yet, unknown view or a compile error (as text).
    pub fn view_digest(&mut self, view: &str, width: u32, height: u32) -> Result<String, String> {
        let s = self.s.as_mut().ok_or("no document yet")?;
        view_digest(s, view, width, height).map_err(|e| e.message)
    }

    /// The display state, in key order (claim, then `t`).
    #[must_use]
    pub fn display(&self) -> &[DisplayItem] {
        &self.display
    }

    /// FNV-1a 64 of the display state's JSON (native ↔ WASM comparison).
    #[must_use]
    pub fn display_digest(&self) -> String {
        let bytes = serde_json::to_vec(&self.display).unwrap_or_default();
        format!("{:016x}", fnv1a64(&bytes))
    }

    /// One message from the authority on this replica's connection.
    pub fn on_message(&mut self, msg: &ServerMsg) {
        match msg {
            ServerMsg::Welcome { .. } => {
                if self.s.is_none() && !self.doc_asked {
                    self.doc_asked = true;
                    for (id, kind) in [(SUB_CHANGES, "changes"), (SUB_CLAIMS, "claims")] {
                        self.outbox.push(serde_json::json!({
                            "type": "subscribe", "id": id, "topic": {"kind": kind}
                        }));
                    }
                    self.outbox.push(serde_json::json!({
                        "type": "query", "id": Q_DOC, "query": {"kind": "document"}
                    }));
                }
            }
            ServerMsg::Result { id, seq, data, .. } => match data {
                QueryResult::Document { document } if id == Q_DOC => {
                    self.load((**document).clone(), *seq);
                }
                QueryResult::Log { entries } if id.starts_with(Q_LOG) => {
                    self.log_asked = false;
                    for e in entries {
                        self.buffered.entry(e.seq).or_insert_with(|| e.clone());
                    }
                    self.drain();
                }
                _ => {}
            },
            ServerMsg::Notify { event, .. } => self.on_event(event),
            _ => {}
        }
        self.reconcile();
    }

    /// One outbound line of the authority (`{"conn":…,"msg":…}`) or a bare message, as
    /// JSON text.
    ///
    /// # Errors
    /// Not JSON or not a server message.
    pub fn on_line(&mut self, line: &str) -> Result<(), String> {
        let v: serde_json::Value = serde_json::from_str(line).map_err(|e| e.to_string())?;
        let m = v.get("msg").cloned().unwrap_or(v);
        let msg: ServerMsg = serde_json::from_value(m).map_err(|e| e.to_string())?;
        self.on_message(&msg);
        Ok(())
    }

    fn on_event(&mut self, event: &Event) {
        match event {
            Event::Change {
                seq,
                at,
                author,
                approved_by,
                cause,
                changes,
                evidence,
            } => {
                let entry = LogEntry {
                    seq: *seq,
                    at: *at,
                    author: author.clone(),
                    approved_by: approved_by.clone(),
                    cause: cause.clone(),
                    reason: None,
                    request: Vec::new(),
                    changes: changes.clone(),
                    inverse: Vec::new(),
                    undoes: None,
                    summary: String::new(),
                    evidence: evidence.clone(),
                };
                self.buffered.entry(*seq).or_insert(entry);
                self.drain();
            }
            Event::Claim {
                claim,
                state,
                evidence,
                ..
            } => self.learn(claim.clone(), *state, evidence.clone(), true),
            _ => {}
        }
    }

    /// Records the authority's word on a claim. `authoritative`: from the `claims` topic
    /// (replaces); else a fallback (only for a claim not known yet).
    fn learn(
        &mut self,
        claim: ClaimRef,
        state: ClaimState,
        ev: ClaimEvidence,
        authoritative: bool,
    ) {
        let key = (claim, ev.t);
        if !authoritative && self.claims.contains_key(&key) {
            return;
        }
        self.counter += 1;
        self.claims.insert(
            key,
            Known {
                state,
                ev,
                n: self.counter,
            },
        );
        if self.claims.len() > MAX_CLAIMS {
            if let Some(k) = self
                .claims
                .iter()
                .min_by_key(|(_, k)| k.n)
                .map(|(k, _)| k.clone())
            {
                self.claims.remove(&k);
            }
        }
    }

    fn load(&mut self, doc: Document, seq: Seq) {
        let provider = match self.provider.take() {
            Some(p) => p,
            None => self.s.take().map_or(Provider::None, |mut s| {
                std::mem::replace(&mut s.plane.provider, Provider::None)
            }),
        };
        match follower(doc, &self.principal, provider) {
            Ok(mut s) => {
                s.set_headline_calendar(self.headline_cal.clone());
                self.s = Some(s);
                self.seq = seq;
                self.doc_asked = false;
                self.buffered.retain(|k, _| *k > seq);
                self.drain();
            }
            Err(_) => self.provider = Some(Provider::None),
        }
    }

    /// Applies buffered entries in order; a hole asks for the `log` after `seq`.
    fn drain(&mut self) {
        if self.s.is_none() {
            return;
        }
        self.buffered.retain(|k, _| *k > self.seq);
        while let Some(e) = self.buffered.remove(&(self.seq + 1)) {
            if !self.mirror(e) {
                // The mirror does not fit (should not happen): reload the document.
                self.buffered.clear();
                if !self.doc_asked {
                    self.doc_asked = true;
                    self.outbox.push(serde_json::json!({
                        "type": "query", "id": Q_DOC, "query": {"kind": "document"}
                    }));
                }
                return;
            }
        }
        if !self.buffered.is_empty() && !self.log_asked {
            self.log_asked = true;
            self.outbox.push(serde_json::json!({
                "type": "query", "id": format!("{Q_LOG}:{}", self.seq),
                "query": {"kind": "log", "after_seq": self.seq}
            }));
        }
    }

    /// Mirrors one log entry into the follower (the same application and bookkeeping as
    /// the authority's own entries).
    fn mirror(&mut self, mut e: LogEntry) -> bool {
        let Some(s) = self.s.as_mut() else {
            return false;
        };
        let restore = crate::live::Restore {
            initial: &s.initial,
            log: &s.log,
        };
        let Ok(inverse) = crate::live::apply_in_place(&mut s.doc, &e.changes, restore) else {
            return false;
        };
        e.inverse = inverse;
        if let Some(ev) = e.evidence.clone() {
            for c in &e.changes {
                if let qp_schema::protocol::ChangeOp::SetCapsuleState { capsule, .. } = c {
                    let finding = s
                        .doc
                        .capsules
                        .iter()
                        .find(|x| &x.id == capsule)
                        .map(|x| x.finding.clone())
                        .unwrap_or_default();
                    let claim = ClaimRef::Capsule {
                        capsule: capsule.clone(),
                        finding,
                    };
                    self.pending_learn.push((claim, ev.clone()));
                }
            }
        }
        self.seq = e.seq;
        s.committed(e);
        s.pending_views.clear();
        let _ = s.claims.take();
        let learn = std::mem::take(&mut self.pending_learn);
        for (c, ev) in learn {
            self.learn(c, ClaimState::Asserted, ev, false);
        }
        true
    }

    /// A data event from this replica's own feed (§18.4 rules; a gap makes the binding
    /// stale until the host re-resolves: [`Replica::resync`]). The display is re-checked
    /// in the same step.
    ///
    /// # Errors
    /// [`DataError`] as on the authority (unknown feed, rejected event).
    pub fn data_event(&mut self, ev: &DataEvent) -> Result<(), DataError> {
        let Some(s) = self.s.as_mut() else {
            // No document yet: the provider (memory) keeps the event for the resolution.
            if let Some(Provider::Memory(m)) = self.provider.as_mut() {
                let _ = m.apply(ev);
            }
            return Ok(());
        };
        let outcome = s.plane.apply_event(ev)?;
        if outcome.changed.is_empty() {
            return Ok(());
        }
        let ids: Vec<String> = outcome.changed.iter().map(|(id, _)| id.clone()).collect();
        s.data_changed(outcome.changed);
        s.pending_views.clear();
        self.reconcile_on(Some(&ids));
        Ok(())
    }

    /// Resolves every bound series again from the provider (the host after a gap, after a
    /// settled fetch it put into its provider, or after the provider caught up).
    pub fn resync(&mut self) {
        if let Some(s) = self.s.as_mut() {
            let ids: Vec<String> = s.plane.series.keys().cloned().collect();
            s.plane.series.clear();
            let _ = s.plane.sync(&s.doc);
            s.pending_stamps.clear();
            s.data_changed(
                ids.into_iter()
                    .map(|id| (id, Delta::Rebuild { from: 0 }))
                    .collect(),
            );
            s.pending_views.clear();
        }
        self.reconcile();
    }

    /// The host declares that its data of bar series `series` from `from_t` on may lack
    /// events (a gap it has not repaired); `None` clears it.
    pub fn set_unclean(&mut self, series: &str, from_t: Option<i64>) {
        if self.unclean.get(series).copied() == from_t {
            return;
        }
        match from_t {
            Some(t) => {
                self.unclean.insert(series.to_owned(), t);
            }
            None => {
                self.unclean.remove(series);
            }
        }
        self.reconcile_on(Some(&[series.to_owned()]));
    }

    /// The host declares that bars of `series` up to `t` hold settled values (from a
    /// settled fetch), although its chart has not closed the last of them.
    pub fn set_settled(&mut self, series: &str, t: i64) {
        if self.settled.get(series) == Some(&t) {
            return;
        }
        self.settled.insert(series.to_owned(), t);
        self.reconcile_on(Some(&[series.to_owned()]));
    }

    /// Settled refetches the replica asks the host for: (bar series, claim bar time).
    /// The host fetches settled bars up to `t`, applies them ([`Replica::resync`] or data
    /// events) and answers with [`Replica::refetched`].
    pub fn take_refetch(&mut self) -> Vec<(String, i64)> {
        std::mem::take(&mut self.refetch_new)
    }

    /// The host has refetched settled data of `series` up to `t`.
    pub fn refetched(&mut self, series: &str, t: i64) {
        self.refetched.insert((series.to_owned(), t));
        self.reconcile();
    }

    /// The condition a claim is about.
    fn condition_of(&self, claim: &ClaimRef) -> Option<Condition> {
        let doc = self.document()?;
        match claim {
            ClaimRef::Condition { sub } => self.watches.get(sub).cloned(),
            ClaimRef::Intent { intent } => doc
                .intents
                .iter()
                .find(|i| &i.id == intent)
                .and_then(|i| i.trigger.clone()),
            ClaimRef::Capsule { finding, .. } => doc
                .findings
                .iter()
                .find(|f| &f.id == finding)
                .map(|f| f.invalidation.clone()),
        }
    }

    /// The display rule (§19.4) over every claim the authority asserts.
    fn reconcile(&mut self) {
        self.reconcile_on(None);
    }

    /// [`Replica::reconcile`] after a change of bar series `changed` only: claims not
    /// resting on any of them keep their item (their data did not change).
    fn reconcile_on(&mut self, changed: Option<&[String]>) {
        let keys: Vec<(ClaimRef, i64)> = self
            .claims
            .iter()
            .filter(|(_, k)| k.state == ClaimState::Asserted)
            .map(|(k, _)| k.clone())
            .collect();
        let old: BTreeMap<(ClaimRef, i64), DisplayItem> = std::mem::take(&mut self.display)
            .into_iter()
            .map(|i| ((i.claim.clone(), i.t), i))
            .collect();
        let mut out = Vec::with_capacity(keys.len());
        let mut changed_any = old.len() != keys.len();
        for key in keys {
            if let (Some(ch), Some(prev)) = (changed, old.get(&key)) {
                let roots = self.roots.get(&key);
                if roots.is_some_and(|r| !r.iter().any(|x| ch.contains(x))) {
                    out.push(prev.clone());
                    continue;
                }
            }
            let ev = self.claims[&key].ev.clone();
            let cond = self.condition_of(&key.0);
            let item = self.judge(&key.0, &ev, cond.as_ref());
            if old.get(&key) != Some(&item) {
                changed_any = true;
            }
            out.push(item);
        }
        let claims = &self.claims;
        self.roots.retain(|k, _| claims.contains_key(k));
        if changed_any {
            self.display_version += 1;
        }
        self.display = out;
    }

    /// Queues a `dispute` (§19.9) for a claim its own data contradicts, once per (claim,
    /// `t`, authority's evidence); it carries the digest of its own bars of the claim's bar
    /// series up to `t`. A rejection re-asserts the same evidence: no new dispute.
    fn send_dispute(&mut self, claim: &ClaimRef, ev: &ClaimEvidence, item: &DisplayItem) {
        let Some(s) = self.s.as_ref() else {
            return;
        };
        let rd = rd!(s);
        let Some(bars) = crate::values::find_series(rd, &ev.series).and_then(Series::bars) else {
            return;
        };
        let Ok(i) = bars.binary_search_by_key(&ev.t, |b| b.t) else {
            return;
        };
        let digest = qp_schema::data::data_digest(&bars[..=i]);
        let anchor_t = anchor_of(&s.doc, &ev.series);
        if !self.disputes_sent.insert((
            claim.clone(),
            ev.t,
            serde_json::to_string(ev).unwrap_or_default(),
        )) {
            return;
        }
        self.dispute_counter += 1;
        self.outbox.push(serde_json::json!({
            "type": "dispute",
            "id": format!("replica:dispute:{}", self.dispute_counter),
            "claim": claim,
            "t": ev.t,
            "values": item.values,
            "anchor_t": anchor_t,
            "digest": digest,
        }));
    }

    /// Disputes sent so far.
    #[must_use]
    pub fn disputes_sent(&self) -> u64 {
        self.dispute_counter
    }

    /// Increases whenever the display state changes.
    #[must_use]
    pub fn display_version(&self) -> u64 {
        self.display_version
    }

    fn judge(
        &mut self,
        claim: &ClaimRef,
        ev: &ClaimEvidence,
        cond: Option<&Condition>,
    ) -> DisplayItem {
        let mut item = DisplayItem {
            claim: claim.clone(),
            series: ev.series.clone(),
            t: ev.t,
            state: Shown::Pending,
            label: None,
            values: Vec::new(),
            reason: None,
        };
        let pending = |mut i: DisplayItem, why: &'static str| {
            i.reason = Some(why);
            i
        };
        let Some(cond) = cond else {
            return pending(item, "unknown_condition");
        };
        let Some(s) = self.s.as_mut() else {
            return pending(item, "no_document");
        };
        // The bar series of every operand must be resolved from the document's anchor,
        // and the authority's values computed from the same anchor.
        let mut bss: Vec<String> = Vec::new();
        for v in &ev.values {
            let rd = rd!(s);
            let bs = bar_series(rd, &v.series)
                .map(|x| x.id.clone())
                .or_else(|| crate::plane::bar_root(&s.doc, &v.series).map(|x| x.id.clone()))
                .unwrap_or_else(|| v.series.clone());
            bss.push(bs);
        }
        let mut roots = bss.clone();
        roots.push(ev.series.clone());
        roots.sort();
        roots.dedup();
        self.roots.insert((claim.clone(), ev.t), roots);
        for bs in &bss {
            if let Some((state, _)) = s.plane.state_for(bs, None) {
                match state {
                    BindingState::Ready => {}
                    BindingState::NoEntitlement => return pending(item, "no_entitlement"),
                    BindingState::Stale => return pending(item, "stale"),
                    _ => return pending(item, "no_data"),
                }
                let doc_anchor = anchor_of(&s.doc, bs);
                let own = s.plane.series.get(bs).and_then(|b| b.bound.anchor_t);
                if doc_anchor.is_none() || own != doc_anchor {
                    return pending(item, "anchor");
                }
            }
        }
        if anchor_of(&s.doc, &ev.series) != ev.anchor_t {
            return pending(item, "anchor");
        }
        let rd = s.rd.as_ref().unwrap_or(&s.doc);
        let Some((holds, own)) = eval_at(rd, &mut s.cache, cond, Some(&ev.values)) else {
            return pending(item, "not_covered");
        };
        item.values = own;
        let own_closed = item
            .values
            .iter()
            .zip(&bss)
            .all(|(v, bs)| closed(rd, bs, v.t));
        let own_settled = item
            .values
            .iter()
            .zip(&bss)
            .all(|(v, bs)| self.settled.get(bs).is_some_and(|t| v.t <= *t));
        let own_clean = item
            .values
            .iter()
            .zip(&bss)
            .all(|(v, bs)| self.unclean.get(bs).is_none_or(|t| v.t < *t));
        let auth_final = ev.maturity == Maturity::Final;
        let trust = ev.clean || own_clean;
        if holds {
            if auth_final && own_closed && trust {
                item.state = Shown::Shown;
                item.label = Some(Maturity::Final);
                return item;
            }
            if auth_final && own_settled && trust {
                item.state = Shown::Shown;
                item.label = Some(Maturity::Provisional);
                return item;
            }
            if !auth_final && !own_closed && self.opts.live {
                item.state = Shown::Shown;
                item.label = Some(Maturity::Provisional);
                return item;
            }
            let why = if !trust {
                "unclean"
            } else if auth_final {
                "not_closed"
            } else {
                "awaiting_final"
            };
            return pending(item, why);
        }
        // Its own data does not hold the claim.
        if auth_final && own_settled && !own_closed && own_clean {
            // The bar came from a settled fetch while its own feed lags (e.g. an outage):
            // the bars before it may miss corrections the feed never delivered. Ask for
            // the settled range once (§19.5 gap repair); never disputed before closing.
            let key = (ev.series.clone(), ev.t);
            if self.opts.refetch && self.refetch_asked.insert(key.clone()) {
                self.refetch_new.push(key);
            }
            return pending(item, "not_held");
        }
        if auth_final && own_closed && own_clean {
            let key = (ev.series.clone(), ev.t);
            if !self.opts.refetch || self.refetched.contains(&key) {
                item.state = Shown::Disputed;
                item.reason = Some("sources_differ");
                if self.opts.dispute {
                    self.send_dispute(claim, ev, &item);
                }
                return item;
            }
            if self.refetch_asked.insert(key.clone()) {
                self.refetch_new.push(key);
            }
            return pending(item, "refetch");
        }
        pending(item, "not_held")
    }
}
