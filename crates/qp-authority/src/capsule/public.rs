//! (R3/K1, `docs/PROTOCOL.md` §20.5) The **public world**: what a reader without the
//! licensed flow entitlement receives is computed from public data and people's commands
//! only, by construction.
//!
//! With a licensed flow (`init.capsules.flow`) the authority runs a second capsule engine,
//! the **public-only engine** (same `EngineConfig::r3()`, the same final bars, only the
//! facts of public license class), and keeps a second document and log, the public world,
//! with **its own sequence numbers** (R3/K1b). What it logs:
//!
//! - every engine look (one per bar close and series) and every settled-data answer that
//!   corrected bars: the public-only engine's own changes;
//! - anchor stamps and bar writes (series data, the same in both worlds);
//! - every person's command (direct, proposal, acceptance, undo), **expanded in the public
//!   world**, with public sequence numbers and ids, whatever the authority's world made of
//!   it: an entry when it applies there, none when it does not. A public-view writer gets
//!   the public world's answer (errors, conflicts, `seq`, ids); an entitled writer the
//!   authority's. An entitled writer's `undo` and `accept_proposal` name the authority's
//!   entries: they are mapped to the public entries those entries made (none: nothing);
//! - §8.1 invalidations of the public world's own (non-engine) capsules, evaluated there.
//!
//! Nothing else: an authority entry that has no public counterpart leaves no trace there
//! (no placeholder, no sequence number), so which authority entries exist (an entitled
//! person approving a licensed-only story, a capsule only the authority holds being
//! invalidated) is not observable in the public world.
//!
//! A public-view writer's command reaches the authority's world only as the public changes
//! translated back, **all or nothing**, when every one of them names an item the two worlds
//! hold alike (a new id, or a person's own item both hold) — never an engine item, never an
//! item only one world has ([`Session::reverse_changes`]); proposals and acceptances of a
//! public-view person stay in the public world.
//!
//! A principal not entitled to every licensed flow the engine reads is a **public-view**
//! reader ([`Session::public_view`]): every read path (`as_reader`: queries, `subscribe`,
//! `hello` summary, selectors, inputs; `change` events; topic `capsules`) runs with the
//! public world's document, log, reading document, ledger and caches swapped in
//! ([`Session::in_world`]); every `seq` it gets is a public one ([`Session::seq_to`]); the
//! usual projection (bar entitlement, other people's audiences) then applies on top. The
//! server comments (mühür) are the only messages influenced by licensed data that such a
//! reader gets. The authority's document and log are unchanged (replay stays exact); the
//! public world is never sent whole, never logged and rebuilt from nothing (it is a
//! function of the authority's inputs).
//!
//! Partial entitlement (some flows, not all) gets the public world too: an engine whose
//! ranking context spans every instrument cannot give a story free of the flows the reader
//! may not read (R3 runs one synthetic flow source; per-subset engines are a proposal).

mod commit;

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap};

use qp_schema::data::FeedKey;
use qp_schema::protocol::{ChangeOp, ClaimEvidence, LogEntry, Seq};
use qp_schema::{Author, Document};

use crate::session::Session;
use qp_plane::capidx::{HeadState, IdIndex, ReaderProj, ReaderView, SentenceMemo};
use qp_plane::plane::reading_doc;
use qp_plane::proj::Proj;

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
    pub series_writer: BTreeMap<String, String>,
    pub invalidation_writer: BTreeMap<String, String>,
    pub intent_state: BTreeMap<String, bool>,
    pub intent_epoch: u64,
}

/// What the public world logs for the authority entry about to be committed.
#[derive(Debug, Clone)]
pub(crate) enum PubNext {
    /// Its own entry.
    Entry(PubPending),
    /// Nothing.
    Skip,
}

/// The public world's own entry for the authority entry about to be committed (the
/// other fields — `at`, author, cause, reason, approver, request — are the authority's).
#[derive(Debug, Clone)]
pub(crate) struct PubPending {
    pub changes: Vec<ChangeOp>,
    /// Its own summary (`None`: the authority's).
    pub summary: Option<String>,
    /// Its claim evidence.
    pub evidence: Option<ClaimEvidence>,
    /// Its own `undoes` (a public sequence number).
    pub undoes: Option<Seq>,
    /// Its own request ops (`None`: the authority's).
    pub request: Option<Vec<qp_schema::protocol::Op>>,
}

impl PubPending {
    /// An entry with these changes and the authority's summary.
    pub(crate) fn of(changes: Vec<ChangeOp>) -> PubNext {
        PubNext::Entry(Self {
            changes,
            summary: None,
            evidence: None,
            undoes: None,
            request: None,
        })
    }
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
                log: Vec::new(),
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
                series_writer: self.series_writer.clone(),
                invalidation_writer: self.invalidation_writer.clone(),
                intent_state: self.intent_state.clone(),
                intent_epoch: self.intent_epoch,
            })
        });
        self.full_to_pub = vec![None; self.log.len()];
        self.pub_at = vec![0; self.log.len() + 1];
        self.pub_caps = self
            .doc
            .capsules
            .iter()
            .filter(|c| c.story.is_none())
            .map(|c| c.id.clone())
            .collect();
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
        std::mem::swap(&mut self.series_writer, &mut w.series_writer);
        std::mem::swap(&mut self.invalidation_writer, &mut w.invalidation_writer);
        std::mem::swap(&mut self.intent_state, &mut w.intent_state);
        std::mem::swap(&mut self.intent_epoch, &mut w.intent_epoch);
        self.slots.swap_world();
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

    /// The public log's length.
    pub(crate) fn pub_len(&self) -> Seq {
        if self.in_public {
            return self.log.len() as Seq;
        }
        self.other.as_ref().map_or(0, |w| w.log.len() as Seq)
    }

    /// (R3/K1b) Sequence number `s` (of the authority's log, as the authority's world
    /// writes it into a message) as `principal` may receive it: for a public-view reader
    /// the public log's length at the time the authority's log had length `s`. Hidden
    /// public (RF1 S3.3): the replay host answers with it (`qp-replayhost`).
    #[doc(hidden)]
    #[must_use]
    pub fn seq_to(&self, principal: &str, s: Seq) -> Seq {
        if self.in_public || !self.public_view(principal) {
            return s;
        }
        usize::try_from(s)
            .ok()
            .and_then(|i| self.pub_at.get(i).copied())
            .unwrap_or_else(|| self.pub_len())
    }

    /// The public entry authority entry `s` made, if any.
    pub(crate) fn pub_of(&self, s: Seq) -> Option<Seq> {
        usize::try_from(s)
            .ok()
            .and_then(|i| i.checked_sub(1))
            .and_then(|i| self.full_to_pub.get(i).copied().flatten())
    }

    /// `ops` of an entitled writer (naming the authority's entries) as the public world
    /// takes them: `undo` of an authority entry → `undo` of the public entry it made;
    /// `accept_proposal` likewise (`prop-<seq>`). `None`: an op names an authority entry
    /// with no public counterpart (the public world takes nothing).
    pub(crate) fn ops_to_pub(
        &self,
        ops: &[qp_schema::protocol::Op],
    ) -> Option<Vec<qp_schema::protocol::Op>> {
        use qp_schema::protocol::Op;
        // (R5B audit B1, fail-closed) An entitled writer's comment never enters the public
        // world: its text may carry a licensed number in a form no scan reads. (A
        // public-view writer's comments are written in the public world itself.)
        if ops.iter().any(|o| matches!(o, Op::AddComment { .. })) {
            return None;
        }
        ops.iter()
            .map(|o| match o {
                // (R3/K1c) An authority entry → the public entry it made, or nothing.
                Op::Undo { seq: Some(s) } => self.pub_of(*s).map(|p| Op::Undo { seq: Some(p) }),
                // "Undo my last" is resolved in the public world, among the person's public
                // entries: mapping the authority's resolution would make the public outcome
                // depend on which of the person's entries the authority's world holds (e.g.
                // an approval of a licensed-only story, which has no public entry).
                Op::Undo { seq: None } => Some(Op::Undo { seq: None }),
                Op::RejectProposal { proposal, note } => {
                    self.proposal_to_pub(proposal).map(|p| Op::RejectProposal {
                        proposal: p,
                        note: note.clone(),
                    })
                }
                Op::AcceptProposal { proposal, note } => {
                    self.proposal_to_pub(proposal).map(|p| Op::AcceptProposal {
                        proposal: p,
                        note: note.clone(),
                    })
                }
                other => self
                    .registry
                    .op(other)
                    .map_or_else(|| Some(other.clone()), |h| h.to_public(other)),
            })
            .collect()
    }

    /// A proposal id of the authority's world → the same proposal in the public world
    /// (both worlds stored it from one command).
    pub(crate) fn proposal_to_pub(&self, pid: &str) -> Option<String> {
        self.prop_map.get(pid).cloned()
    }
}

/// The canonical JSON of the item `id` of `doc` (the first one, in collection order).
fn item_json(doc: &Document, id: &str) -> Option<String> {
    fn find<T: serde::Serialize>(v: &[T], key: impl Fn(&T) -> &str, id: &str) -> Option<String> {
        v.iter()
            .find(|x| key(x) == id)
            .and_then(|x| serde_json::to_string(x).ok())
    }
    find(&doc.series, |x| x.id.as_str(), id)
        .or_else(|| find(&doc.views, |x| x.id.as_str(), id))
        .or_else(|| find(&doc.facts, |x| x.id.as_str(), id))
        .or_else(|| find(&doc.findings, |x| x.id.as_str(), id))
        .or_else(|| find(&doc.capsules, |x| x.id.as_str(), id))
        .or_else(|| find(&doc.intents, |x| x.id.as_str(), id))
        .or_else(|| find(&doc.narratives, |x| x.id.as_str(), id))
        .or_else(|| find(&doc.proposals, |x| x.id.as_str(), id))
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

#[allow(clippy::too_many_lines)]
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
