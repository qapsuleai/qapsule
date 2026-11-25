//! The replica (RP2/T8, `docs/PROTOCOL.md` §19): a device-side follower of an authority.
//!
//! A [`Replica`] holds a follower [`Session`] — the **same engine code** as the authority:
//! the same document application (`Session::mirror_apply` + `Session::mirror_commit`),
//! the same data plane (`DataPlane`, `qp_source::FeedStore` with the §18.4 rules), the same
//! values cache and live path (`Session::follow_data`), the same condition semantics
//! ([`qp_authority::eval_at`]) and the same grammar compile (`Session::scene`). The
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

mod follow;
mod follower;
mod judge;

use std::collections::{BTreeMap, BTreeSet};

use qp_schema::data::{BindingState, DataEvent};
use qp_schema::protocol::{
    ClaimEvidence, ClaimRef, ClaimState, Event, EvidenceValue, LogEntry, Maturity, ProtocolError,
    QueryResult, Seq, ServerMsg,
};
use qp_schema::{Bar, Condition, Document, Series};
use serde::Serialize;

use qp_authority::{anchor_of, closed, eval_at, Session, Subsystems};
use qp_plane::plane::{DataError, Provider};
use qp_plane::values::bar_series;
use qp_subsystem::SessionView;

use follower::Follower;

/// At most this many claims are remembered (oldest dropped first).
const MAX_CLAIMS: usize = 4096;

const SUB_CHANGES: &str = "replica:changes";
const SUB_CLAIMS: &str = "replica:claims";
const Q_DOC: &str = "replica:doc";
const Q_LOG: &str = "replica:log";

pub use qp_ops::digest::{document_digest, fnv1a64};

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
    let scene = session.scene(view, &qp_designs::options(width, height))?;
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
    /// (RF1 S3.1) The subsystems its follower authority is built with.
    subs: Subsystems,
    principal: String,
    /// The provider until the document arrives (then it lives in the session's plane).
    provider: Option<Provider>,
    s: Option<Follower>,
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

impl Replica {
    /// A replica for `principal` whose data comes from the host's `provider`; its
    /// follower authority is built with subsystems `subs`.
    #[must_use]
    pub fn new(
        principal: &str,
        provider: Box<dyn qp_source::SourceProvider>,
        subs: Subsystems,
    ) -> Self {
        Self::with(principal, Provider::Host(provider), subs)
    }

    /// A replica whose provider is a [`qp_source::MemoryProvider`] that also takes the
    /// data events the host delivers (so a re-resolution sees them): tests, simple hosts,
    /// the WASM binding.
    #[must_use]
    pub fn with_memory(
        principal: &str,
        provider: qp_source::MemoryProvider,
        subs: Subsystems,
    ) -> Self {
        Self::with(principal, Provider::Memory(provider), subs)
    }

    fn with(principal: &str, provider: Provider, subs: Subsystems) -> Self {
        Self {
            subs,
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

    /// [`Replica::start`] from a borrowed document (cloned here: one clone instance for
    /// every caller, RF1 S3.3; the viewer's open side starts from its slice).
    #[doc(hidden)]
    pub fn start_from(&mut self, doc: &Document, seq: Seq) {
        self.start(doc.clone(), seq);
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
        self.s.as_deref().map(Session::document)
    }

    /// The follower session (read access to the replica's own data through its methods:
    /// document, scene, provider). Built by the composition with the follower's subsystems
    /// (`qp_session::FOLLOWER`, without the reading subsystem; RF1 S3.2): a `query` or an
    /// `input` sent to it with `handle_raw` is answered `unsupported`.
    pub fn session_mut(&mut self) -> Option<&mut Session> {
        self.s.as_deref_mut()
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
        qp_plane::values::find_series(s.reading(), series).and_then(qp_schema::Series::bars)
    }

    /// (R3, K28, §20) Capsule candidates on the open (last) bar of bar series `series`,
    /// computed from its **own** data ([`qp_authority::capsule::candidates`]: public detectors
    /// and `Engine::candidates`). Shown only on this replica; never sent to the
    /// authority, never in its log or ledger; a candidate whose formation the next tick
    /// undoes simply disappears.
    #[must_use]
    pub fn candidates(&self, series: &str) -> Vec<qp_capsule::CandidateStory> {
        let Some(s) = self.s.as_ref() else {
            return Vec::new();
        };
        let Some(bs) = qp_plane::values::find_series(s.reading(), series) else {
            return Vec::new();
        };
        let (Some(bars), Some(symbol)) = (bs.bars(), bs.symbol()) else {
            return Vec::new();
        };
        // (RP4) The record the authority runs, read from its stories (default `r3`).
        let engine = qp_authority::capsule::engine_of_doc(s.reading());
        qp_authority::capsule::candidates_with(engine, symbol, series, bars)
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
}
