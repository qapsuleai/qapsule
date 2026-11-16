//! (R3/K1, `docs/PROTOCOL.md` §20) The **public world**: what a reader without the
//! licensed flow entitlement receives is computed from public data only, by construction.
//!
//! With a licensed flow (`init.capsules.flow`) the authority runs a second capsule engine,
//! the **public-only engine** (same `EngineConfig::r3()`, the same final bars, only the
//! facts of public license class). Its looks are written into a second document and log,
//! the public world, entry for entry with the authority's (same `seq`, `at`, author):
//!
//! - an engine look's entry (and a settled-data retraction) holds the public-only
//!   engine's own changes (facts, findings, capsules, ledger rows of its stories);
//! - an entitled principal's direct command holds the same ops expanded in the public
//!   world (translated, below, when they do not apply there); a public-view reader's
//!   command (and proposal) is validated and expanded in the public world only — every
//!   error, conflict, id and change it sees comes from there — and the authority's world
//!   takes the same ops only when they apply there and touch no engine item the two
//!   worlds do not share ([`Session::diverged_touch`]), else the public changes
//!   translated back ([`Session::reverse_changes`]: people's own items kept);
//! - every other entry (bars, series, anchors, proposals and their acceptance, other
//!   engine entries) is translated ([`Session::public_entry`]): changes that put, change
//!   or name an engine item the public world has no item of (by id) are dropped, an
//!   engine finding or capsule of an id the public world has keeps the public world's
//!   content and takes only what changed (claim, confidence, invalidation; view, state;
//!   a retraction), an undo undoes the public world's version of the entry;
//! - request ops, summary and cause that name an engine item only the authority's world
//!   has are removed (as for a projected entry).
//!
//! The sequence number is shared, so every entry's **existence** is made to rest on
//! public data and people's commands: a look per bar close, a settled entry per corrected
//! answer, anchor stamps from bindings, and §8.1 invalidations only of people's own
//! capsules (an engine capsule ends by the outcome rule only), which both worlds hold.
//!
//! A principal not entitled to every licensed flow the engine reads is a **public-view**
//! reader ([`Session::public_view`]): every read path (`as_reader`: queries, `subscribe`,
//! `hello` summary, selectors, inputs; `change` events; topic `capsules`) runs with the
//! public world's document, log, reading document, ledger and caches swapped in
//! ([`Session::in_world`]); the usual projection (bar entitlement, other people's
//! audiences) then applies on top. The server comments (mühür) are the only messages
//! influenced by licensed data that such a reader gets. The authority's document and log
//! are unchanged (replay stays exact); the public world is never sent whole, never
//! logged and rebuilt from nothing (it is a function of the authority's inputs).
//!
//! Partial entitlement (some flows, not all) gets the public world too: an engine whose
//! ranking context spans every instrument cannot give a story free of the flows the reader
//! may not read (R3 runs one synthetic flow source; per-subset engines are a proposal).

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap};

use qp_schema::data::FeedKey;
use qp_schema::protocol::{ChangeOp, LogEntry};
use qp_schema::{Author, Document};

use crate::capidx::{HeadState, IdIndex, ReaderProj, ReaderView, SentenceMemo};
use crate::capsule::Proj;
use crate::plane::reading_doc;
use crate::session::Session;

/// The fields of one capsule world that [`Session`] holds for the world it is in.
pub(crate) struct World {
    pub doc: Document,
    pub log: Vec<LogEntry>,
    pub rd: Option<Document>,
    pub live_index: Option<crate::live::LiveIndex>,
    pub view_reads: Option<BTreeMap<String, BTreeSet<String>>>,
    pub fact_ix: RefCell<IdIndex>,
    pub cap_ix: RefCell<IdIndex>,
    pub sentences: RefCell<SentenceMemo>,
    pub projs: RefCell<HashMap<String, ReaderProj>>,
    pub views: Vec<ReaderView>,
    pub head_state: HeadState,
    pub headlines: BTreeMap<String, String>,
}

/// The public world's own changes for the entry about to be committed.
#[derive(Debug, Clone)]
pub(crate) struct PubPending {
    pub changes: Vec<ChangeOp>,
    /// Its own summary (`None`: the authority's).
    pub summary: Option<String>,
}

/// Whether `c` only writes bars (the same in both worlds).
fn bar_op(c: &ChangeOp) -> bool {
    matches!(
        c,
        ChangeOp::AppendBars { .. } | ChangeOp::SetLastBar { .. } | ChangeOp::TruncateBars { .. }
    )
}

fn engine_author(a: &Author) -> bool {
    matches!(a, Author::Engine { .. })
}

impl Session {
    /// A public world for a session whose engine reads a licensed flow (at
    /// `start_capsules`, before the first entry).
    pub(crate) fn start_public(&mut self) {
        let flow = self
            .runner
            .as_ref()
            .is_some_and(crate::capsule::Runner::has_flow);
        self.other = flow.then(|| {
            Box::new(World {
                doc: self.doc.clone(),
                log: self.log.clone(),
                rd: self
                    .rd
                    .as_ref()
                    .and_then(|_| reading_doc(&self.doc, &self.plane)),
                live_index: None,
                view_reads: None,
                fact_ix: RefCell::default(),
                cap_ix: RefCell::default(),
                sentences: RefCell::default(),
                projs: RefCell::default(),
                views: Vec::new(),
                head_state: HeadState::default(),
                headlines: BTreeMap::new(),
            })
        });
    }

    /// Swaps the world the fields hold with the other one.
    fn swap_world(&mut self) {
        let Some(mut w) = self.other.take() else {
            return;
        };
        std::mem::swap(&mut self.doc, &mut w.doc);
        std::mem::swap(&mut self.log, &mut w.log);
        std::mem::swap(&mut self.rd, &mut w.rd);
        std::mem::swap(&mut self.live_index, &mut w.live_index);
        std::mem::swap(&mut self.view_reads, &mut w.view_reads);
        std::mem::swap(&mut self.fact_ix, &mut w.fact_ix);
        std::mem::swap(&mut self.cap_ix, &mut w.cap_ix);
        std::mem::swap(&mut self.sentences, &mut w.sentences);
        std::mem::swap(&mut self.projs, &mut w.projs);
        std::mem::swap(&mut self.views, &mut w.views);
        std::mem::swap(&mut self.head_state, &mut w.head_state);
        std::mem::swap(&mut self.headlines, &mut w.headlines);
        self.other = Some(w);
        self.in_public = !self.in_public;
    }

    /// Runs `f` with the public world (`public`) or the authority's world in the fields.
    pub(crate) fn in_world<R>(&mut self, public: bool, f: impl FnOnce(&mut Self) -> R) -> R {
        if self.in_public == public || self.other.is_none() {
            return f(self);
        }
        self.swap_world();
        let r = f(self);
        self.swap_world();
        r
    }

    /// `true` when `principal` reads the public world: a licensed flow is read and it is
    /// not entitled to every one of them.
    pub(crate) fn public_view(&self, principal: &str) -> bool {
        if self.other.is_none() {
            return false;
        }
        let Some(r) = self.runner.as_ref() else {
            return false;
        };
        r.flow_feeds()
            .iter()
            .any(|k: &FeedKey| !self.plane.entitled(principal, k))
    }

    /// Ids of the engine items (facts with a meaning, engine findings, capsules with a
    /// story) of the authority's document that the public world has no item of (by id:
    /// what the public world takes depends on the public world only).
    fn full_only(&self, pubdoc: &Document) -> BTreeSet<String> {
        only_in(&self.doc, pubdoc)
    }

    /// (In the authority's world) a public-view reader's command changes `changes`
    /// (inverse `inverse`, of the public world) as the authority's document takes them:
    /// [`translate`] the other way (engine items only the public world has dropped;
    /// people's own items — findings, capsules without a story, views — kept, so that
    /// both worlds hold them and their invalidations coincide).
    pub(crate) fn reverse_changes(
        &self,
        changes: &[ChangeOp],
        inverse: &[ChangeOp],
    ) -> Vec<ChangeOp> {
        let Some(w) = self.other.as_ref() else {
            return Vec::new();
        };
        let p = Proj {
            hidden: only_in(&w.doc, &self.doc),
            ..Proj::default()
        };
        changes
            .iter()
            .filter_map(|c| translate(c, inverse, &self.doc, &p))
            .collect()
    }

    /// Whether `changes` (of the authority's world) name an engine item the two worlds
    /// do not share: one only the authority's world has, or one whose id the public world
    /// gives to another item (a capsule born of another fact, and its finding).
    pub(crate) fn diverged_touch(&self, changes: &[ChangeOp]) -> bool {
        let Some(w) = self.other.as_ref() else {
            return false;
        };
        let mut hidden = self.full_only(&w.doc);
        let lead = |c: &qp_schema::Capsule| {
            c.story
                .as_ref()
                .and_then(|s| s.history.first())
                .and_then(|t| t.facts.first())
                .cloned()
        };
        let pc: HashMap<&str, &qp_schema::Capsule> =
            w.doc.capsules.iter().map(|c| (c.id.as_str(), c)).collect();
        for c in self.doc.capsules.iter().filter(|c| c.story.is_some()) {
            if pc.get(c.id.as_str()).is_some_and(|p| lead(p) != lead(c)) {
                hidden.insert(c.id.clone());
                hidden.insert(c.finding.clone());
            }
        }
        let p = Proj {
            hidden,
            ..Proj::default()
        };
        changes.iter().any(|c| p.mentions_hidden(c))
    }

    /// The public world's version of `e` (an entry of the authority's world, about to
    /// be logged; the authority's document already holds its changes): the writer's own
    /// ([`PubPending`]) or a translation (see the module docs).
    pub(crate) fn public_entry(&mut self, e: &LogEntry) -> LogEntry {
        let mut x = LogEntry {
            reason: e.reason.clone(),
            seq: e.seq,
            at: e.at,
            author: e.author.clone(),
            approved_by: e.approved_by.clone(),
            cause: e.cause.clone(),
            request: Vec::new(),
            changes: Vec::new(),
            inverse: Vec::new(),
            undoes: e.undoes,
            summary: e.summary.clone(),
            evidence: e.evidence.clone(),
        };
        let pending = self.pending_pub.take();
        let own = pending.is_some();
        let Some(w) = self.other.as_ref() else {
            return e.clone();
        };
        match pending {
            Some(p) => {
                x.changes = p.changes;
                if let Some(s) = p.summary {
                    x.summary = s;
                }
                // An engine look names nothing a reader may not see.
                if e.request.is_empty() {
                    return x;
                }
            }
            None if e.changes.iter().chain(&e.inverse).all(bar_op) => {
                x.request.clone_from(&e.request);
                x.changes.clone_from(&e.changes);
                return x;
            }
            None => {}
        }
        let pubdoc = &w.doc;
        let hidden = self.full_only(pubdoc);
        let p = Proj {
            hidden,
            ..Proj::default()
        };
        if !own {
            if let Some(t) = e
                .undoes
                .and_then(|u| usize::try_from(u).ok())
                .and_then(|u| w.log.get(u.wrapping_sub(1)))
            {
                x.changes.clone_from(&t.inverse);
            } else {
                for c in &e.changes {
                    if let Some(c) = translate(c, &e.inverse, pubdoc, &p) {
                        x.changes.push(c);
                    }
                }
            }
            // An engine entry about items the public world does not have carries no
            // evidence and no summary there. (By construction none is written: the
            // engine entries outside the looks — anchor stamps, invalidations of
            // non-engine capsules, which both worlds share — rest on public data.)
            if e.author == crate::session::ENGINE_PRINCIPAL
                && x.changes.is_empty()
                && !e.changes.is_empty()
            {
                x.evidence = None;
                "…".clone_into(&mut x.summary);
            }
        }
        x.request = e
            .request
            .iter()
            .filter(|op| !p.mentions_hidden(op))
            .cloned()
            .collect();
        if p.hidden
            .iter()
            .any(|h| h.len() <= x.summary.len() && x.summary.contains(h.as_str()))
        {
            "…".clone_into(&mut x.summary);
        }
        if x.cause
            .as_ref()
            .is_some_and(|c| p.hidden.iter().any(|h| c.contains(h.as_str())))
        {
            x.cause = None;
        }
        x
    }

    /// Applies `changes` one by one to the document the fields hold, leaving out those
    /// that do not fit; returns the applied changes and their inverse (reversed).
    pub(crate) fn apply_each(&mut self, changes: Vec<ChangeOp>) -> (Vec<ChangeOp>, Vec<ChangeOp>) {
        let mut kept = Vec::with_capacity(changes.len());
        let mut inverse = Vec::with_capacity(changes.len());
        for c in changes {
            let restore = crate::live::Restore {
                initial: &self.initial,
                log: &self.log,
            };
            if let Ok(inv) =
                crate::live::apply_in_place(&mut self.doc, std::slice::from_ref(&c), restore)
            {
                inverse.extend(inv);
                kept.push(c);
            }
        }
        inverse.reverse();
        (kept, inverse)
    }

    /// Applies `pe` to the public world and logs it there. A change that does not fit
    /// the public document is left out (the entry keeps its place in the sequence).
    pub(crate) fn commit_public(&mut self, mut pe: LogEntry) {
        self.in_world(true, |s| {
            let (kept, inverse) = s.apply_each(std::mem::take(&mut pe.changes));
            pe.changes = kept;
            pe.inverse = inverse;
            let series_set = pe
                .changes
                .iter()
                .any(|c| matches!(c, ChangeOp::PutSeries { .. } | ChangeOp::Delete { .. }));
            if series_set {
                s.rd = reading_doc(&s.doc, &s.plane);
                s.view_reads = None;
            } else if let Some(rd) = s.rd.as_mut() {
                if crate::apply::apply_all(rd, &pe.changes).is_err() {
                    s.rd = reading_doc(&s.doc, &s.plane);
                }
            }
            if !pe.changes.iter().all(crate::live::keeps_view_reads) {
                s.view_reads = None;
            }
            s.log.push(pe);
        });
    }
}

/// Ids of the engine items (facts with a meaning, engine findings, capsules with a story
/// and their findings) of `a` that `b` has no item of (by id).
fn only_in(a: &Document, b: &Document) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let bf: BTreeSet<&str> = b.facts.iter().map(|f| f.id.as_str()).collect();
    for f in &a.facts {
        if f.meaning.is_some() && !bf.contains(f.id.as_str()) {
            out.insert(f.id.clone());
        }
    }
    let bc: BTreeSet<&str> = b
        .capsules
        .iter()
        .filter(|c| c.story.is_some())
        .map(|c| c.id.as_str())
        .collect();
    for c in a.capsules.iter().filter(|c| c.story.is_some()) {
        if !bc.contains(c.id.as_str()) {
            out.insert(c.id.clone());
            out.insert(c.finding.clone());
        }
    }
    let bfi: BTreeSet<&str> = b.findings.iter().map(|f| f.id.as_str()).collect();
    for f in &a.findings {
        if engine_author(&f.provenance.author) && !bfi.contains(f.id.as_str()) {
            out.insert(f.id.clone());
        }
    }
    out
}

/// One change of an entry of the authority's world as the public world takes it
/// (`inverse`: the entry's inverse, for what the person changed; `p.hidden`: the engine
/// items only the authority's world has).
fn translate(c: &ChangeOp, inverse: &[ChangeOp], pubdoc: &Document, p: &Proj) -> Option<ChangeOp> {
    match c {
        ChangeOp::PutFact { fact, index } => {
            if p.hidden.contains(&fact.id) {
                return None;
            }
            if fact.meaning.is_some() {
                let pf = pubdoc.facts.iter().find(|f| f.id == fact.id)?;
                return Some(ChangeOp::PutFact {
                    fact: pf.clone(),
                    index: *index,
                });
            }
            Some(c.clone())
        }
        ChangeOp::PutFinding { finding, index } => {
            if p.hidden.contains(&finding.id) {
                return None;
            }
            let Some(pf) = pubdoc
                .findings
                .iter()
                .find(|f| f.id == finding.id && engine_author(&f.provenance.author))
            else {
                let mut f = finding.clone();
                f.facts.retain(|x| !p.hidden.contains(x));
                return Some(ChangeOp::PutFinding {
                    finding: f,
                    index: *index,
                });
            };
            let old = inverse.iter().find_map(|i| match i {
                ChangeOp::PutFinding { finding: o, .. } if o.id == finding.id => Some(o),
                _ => None,
            });
            let mut nf = pf.clone();
            if let Some(o) = old {
                if finding.claim != o.claim {
                    nf.claim.clone_from(&finding.claim);
                }
                if finding.confidence.to_bits() != o.confidence.to_bits() {
                    nf.confidence = finding.confidence;
                }
                if finding.invalidation != o.invalidation {
                    nf.invalidation = finding.invalidation.clone();
                }
            }
            Some(ChangeOp::PutFinding {
                finding: nf,
                index: *index,
            })
        }
        ChangeOp::PutCapsule { capsule, index } => {
            if p.hidden.contains(&capsule.id) {
                return None;
            }
            let Some(pc) = pubdoc
                .capsules
                .iter()
                .find(|x| x.id == capsule.id && x.story.is_some())
            else {
                return Some(c.clone());
            };
            let old = inverse.iter().find_map(|i| match i {
                ChangeOp::PutCapsule { capsule: o, .. } if o.id == capsule.id => Some(o),
                _ => None,
            });
            let mut nc = pc.clone();
            match old {
                Some(o) => {
                    if o.view != capsule.view {
                        nc.view.clone_from(&capsule.view);
                    }
                    if o.state != capsule.state {
                        nc.state = capsule.state;
                    }
                    // A retraction written outside the engine (§19.9, K37).
                    let retracted = |x: &qp_schema::Capsule| {
                        x.story
                            .as_ref()
                            .is_some_and(|s| s.stage == qp_schema::capsule::Stage::Retracted)
                    };
                    if retracted(capsule) && !retracted(o) && !retracted(&nc) {
                        if let (Some(st), Some(t)) = (
                            nc.story.as_mut(),
                            capsule.story.as_ref().and_then(|s| s.history.last()),
                        ) {
                            st.stage = qp_schema::capsule::Stage::Retracted;
                            st.last_bar = st.last_bar.max(t.bar);
                            st.history.push(t.clone());
                        }
                    }
                }
                None => {
                    nc.view.clone_from(&capsule.view);
                    nc.state = capsule.state;
                }
            }
            Some(ChangeOp::PutCapsule {
                capsule: nc,
                index: *index,
            })
        }
        ChangeOp::SetCapsuleState { capsule, .. } => (!p.hidden.contains(capsule)
            && pubdoc.capsules.iter().any(|x| &x.id == capsule))
        .then(|| c.clone()),
        ChangeOp::PushLedger { entry } => (!p.hidden.contains(&entry.finding)
            && entry.capsule.as_ref().is_none_or(|x| !p.hidden.contains(x)))
        .then(|| c.clone()),
        ChangeOp::Delete { id } => (!p.hidden.contains(id)).then(|| c.clone()),
        ChangeOp::PutView { .. }
        | ChangeOp::InsertLayer { .. }
        | ChangeOp::PutLayer { .. }
        | ChangeOp::SetEmphasis { .. }
        | ChangeOp::PutNarrative { .. }
        | ChangeOp::PutProposal { .. } => (!p.mentions_hidden(c)).then(|| c.clone()),
        _ => Some(c.clone()),
    }
}
