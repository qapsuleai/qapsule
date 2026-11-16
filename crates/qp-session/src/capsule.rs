//! The capsule engine in the authority (R3, `docs/PROTOCOL.md` §20; `docs/CAPSULE.md` §9).
//!
//! # Runner
//!
//! With `init.capsules` the authority watches the named bar series (bound or inline). At
//! every settle point it looks at each watched series: every bar that became **final**
//! since the last look (a later bar exists, or the window is closed: RP2 maturity) goes,
//! in order, through the detectors (`qp_facts::default_detectors`, streaming `push`) and
//! the capsule engine (`qp_capsule::Engine` with `EngineConfig::r3()`). The licensed flow
//! of a series (`init.capsules.flow`) is read only here (server facts), and only here it
//! also feeds the chosen trust path ([`qp_capsule::trust::Muhur`]).
//!
//! What the engine says becomes **one engine entry** (author `qp`) per look and series,
//! written even when nothing changed: the log's rhythm is the bar close, never the
//! existence of a (licensed) story. Its changes, in order: the facts that caused a
//! transition (`put_fact`, `evidence_ref` stamped as for `add_fact`), then per story its
//! finding (`put_finding`, claim = the TR sentence) and capsule (`put_capsule`, story and
//! state), then the ledger rows of the stories that closed (`push_ledger`, R3 fields).
//! The document and the log hold the whole capsule history; replaying the log gives the
//! same document byte for byte (the conformance runner checks it on every transcript).
//!
//! State mapping (`docs/CONCEPTS.md`): `born … confirmed` → `live`; `realized` → `live`
//! (the close is in the ledger); `invalidated` → `invalidated`; `expired` → `expired`;
//! `retracted` → `invalidated` and never a ledger row. A person's decision
//! (`approve_finding` / `reject_finding`) is kept: the engine does not overwrite
//! `approved` / `rejected`.
//!
//! # Projection (K33, K34, K36; M6 integration rule)
//!
//! Nothing licensed and no data-plane value reaches a principal that may not see it.
//!
//! (R3/K1) A principal not entitled to every licensed flow the engine reads does not get
//! a projection of the authority's capsule items at all: it reads the **public world**
//! ([`crate::public`]), written by a second, public-only engine, so everything it gets
//! about capsules is a function of public data (and of people's commands); the mühür
//! messages are its only channel influenced by licensed data. The rules below then
//! hide licensed items in the authority's world (a reader entitled to the flows but not
//! to some bars, the host) and apply the bar entitlement and audiences in both worlds.
//!
//! The authority keeps one document and one log; what it **sends** is projected per
//! principal ([`Proj`]):
//!
//! - a story born of a licensed fact the principal is not entitled to is **hidden**:
//!   its fact(s), finding, capsule, views showing it and ledger rows never go, in any
//!   message (queries, `change` events, `log`, `welcome` summary, errors' suggestions);
//!   the principal gets the trust path's server comments instead (topic and query
//!   `capsules`). Ledger **statistics** stay license-independent (K33);
//! - a licensed fact that joined a public story is hidden from it: the story is shown
//!   without that transition, with the families of its visible facts and the rank it had
//!   before the hidden fact;
//! - a principal without the entitlement of a story's bar feed gets its facts without
//!   evidence values and without `evidence_ref` (data digest), and sentences without a
//!   digit (`redacted`).
//!
//! Pre-R3 items (facts without `meaning`, capsules without a story) are never projected.

use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use qp_capsule::realize::Template;
use qp_capsule::sentence::{condition_now_memo, StudyMemo};
use qp_capsule::{Engine, EngineConfig, FlowObservation, Lang, Realizer, SentenceInput, TrustPath};
use qp_facts::{Detector, FlowPoint};
use qp_schema::capsule::{Audience, EvidenceFamily, LicenseClass, Stage, Story, UserGroup};
use qp_schema::data::FeedKey;
use qp_schema::protocol::{
    CapsuleDetail, CapsuleInit, CapsuleItem, CapsuleSentence, ChangeOp, ClaimEvidence, ClaimRef,
    DisputeVerdict, ErrorCode, Event, EvidenceAccess, Language, LedgerRow, LogEntry, Maturity,
    OutboundLine, QueryResult, Seq, ServerComment, ServerMsg, Topic,
};
use qp_schema::{
    Author, Bar, Capsule, CapsuleState, Condition, Depth, Document, Fact, Finding, Provenance,
    SeriesSource,
};

use crate::error::{ptr, PResult, E};
use crate::session::{out, Session};
use crate::values::{bar_series, find_series};

/// Engine component name in provenance.
const ENGINE_NAME: &str = "qp-capsule";

/// The invalidation of an engine finding: decided by the outcome rule, never by a
/// document condition (a text condition is never evaluated).
const OUTCOME_TEXT: &str = "outcome rule (qp-capsule barrier, docs/CAPSULE.md §9.3)";

/// One watched bar series.
pub(crate) struct Watched {
    series: String,
    symbol: String,
    interval: String,
    /// Bars fed so far (final bars).
    done: usize,
    dets: Vec<Box<dyn Detector>>,
    /// Licensed flow: seed and feed name.
    flow: Option<(u64, String)>,
    muhur: Option<qp_capsule::trust::Muhur>,
    /// (R3/perf) Position of the series in the reading document, for structure
    /// generation [`Runner::pos_gen`].
    pos: Option<usize>,
}

/// An open dispute on an engine capsule (§19.9 + §20).
#[derive(Debug, Clone)]
struct CapDispute {
    conn: String,
    capsule: String,
    series: String,
    t: i64,
    seen: (String, ClaimRef, i64, String),
    /// (R3/K1) Disputed by a public-view reader: decided on the public-only engine.
    public: bool,
}

/// The authority's capsule engine and its state.
pub(crate) struct Runner {
    engine: Engine,
    /// (R3/K1) With a licensed flow: the public-only engine (same configuration, the
    /// same final bars, only the facts of public license class), whose looks the public
    /// world takes ([`crate::public`]).
    pub_engine: Option<Engine>,
    /// The licensed flow feeds the runner reads.
    flow_keys: Vec<FeedKey>,
    watched: Vec<Watched>,
    /// Server comments made so far, with the log position they were made at.
    comments: Vec<(Seq, ServerComment)>,
    disputes: Vec<CapDispute>,
    /// (R3/perf) Structure generation the watched series' positions are of.
    pos_gen: u64,
}

impl std::fmt::Debug for Runner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Runner")
            .field(
                "series",
                &self.watched.iter().map(|w| &w.series).collect::<Vec<_>>(),
            )
            .field("events", &self.engine.events().len())
            .finish_non_exhaustive()
    }
}

impl Runner {
    /// A runner for `init` over `doc` (series without a symbol are skipped).
    pub(crate) fn new(init: &CapsuleInit, doc: &Document, keys: Option<(&[u8], &[u8])>) -> Self {
        let mut watched = Vec::new();
        for id in &init.series {
            let Some(s) = find_series(doc, id) else {
                continue;
            };
            let Some(symbol) = s.symbol().map(str::to_owned) else {
                continue;
            };
            let interval = s
                .binding()
                .map(|b| b.interval)
                .or_else(|| s.interval.clone())
                .unwrap_or_default();
            let flow = init
                .flow
                .iter()
                .find(|f| &f.series == id)
                .map(|f| (f.seed, f.feed.clone().unwrap_or_else(|| "flow".to_owned())));
            watched.push(Watched {
                series: id.clone(),
                symbol,
                interval,
                done: 0,
                dets: qp_facts::default_detectors(),
                muhur: flow.as_ref().map(|_| match keys {
                    Some((r, a)) => qp_capsule::trust::Muhur::with_keys(
                        qp_capsule::trust::Params::default(),
                        r,
                        a,
                    ),
                    None => qp_capsule::trust::Muhur::demo(qp_capsule::trust::Params::default()),
                }),
                flow,
                pos: None,
            });
        }
        let flow = watched.iter().any(|w| w.flow.is_some());
        let flow_keys = watched
            .iter()
            .filter(|w| w.flow.is_some())
            .map(|w| FeedKey {
                symbol: w.symbol.clone(),
                interval: w.interval.clone(),
                feed: w.flow.as_ref().map(|(_, f)| f.clone()),
            })
            .collect();
        Self {
            flow_keys,
            engine: Engine::new(EngineConfig::r3()),
            pub_engine: flow.then(|| Engine::new(EngineConfig::r3())),
            watched,
            comments: Vec::new(),
            disputes: Vec::new(),
            pos_gen: 0,
        }
    }

    fn watched_of(&self, series: &str) -> Option<&Watched> {
        self.watched.iter().find(|w| w.series == series)
    }

    /// The licensed flow feed of `instrument`, if the runner reads one.
    fn flow_feed(&self, instrument: &str) -> Option<FeedKey> {
        self.watched
            .iter()
            .find(|w| w.symbol == instrument && w.flow.is_some())
            .map(|w| FeedKey {
                symbol: w.symbol.clone(),
                interval: w.interval.clone(),
                feed: w.flow.as_ref().map(|(_, f)| f.clone()),
            })
    }

    /// Whether the runner reads any licensed flow.
    pub(crate) fn has_flow(&self) -> bool {
        self.watched.iter().any(|w| w.flow.is_some())
    }

    /// The licensed flow feeds the runner reads.
    pub(crate) fn flow_feeds(&self) -> &[FeedKey] {
        &self.flow_keys
    }
}

/// Number of final bars of bar series `bs` with `n` bars: all of them when the window is
/// closed (`to_t`), else all but the live last bar.
fn final_count(doc: &Document, bs: &str, n: usize) -> usize {
    final_count_of(find_series(doc, bs), n)
}

/// [`final_count`] for the series `s` found under its id.
fn final_count_of(s: Option<&qp_schema::Series>, n: usize) -> usize {
    let closed_window = s.is_some_and(
        |s| matches!(&s.source, SeriesSource::Bind { window, .. } if window.to_t.is_some()),
    );
    if closed_window {
        n
    } else {
        n.saturating_sub(1)
    }
}

/// The synthetic licensed flow of `bars` (prefix-stable: record `i` depends on bars
/// `..=i` only).
fn flow_points(bars: &[Bar], seed: u64) -> Vec<FlowPoint> {
    qp_source::synthetic_flow(bars, seed)
        .into_iter()
        .map(|r| FlowPoint {
            t: r.t,
            values: r.values,
        })
        .collect()
}

/// Capsule state for a story stage (`docs/CONCEPTS.md`, "Kapsülün yaşam döngüsü").
fn state_of(stage: Stage) -> CapsuleState {
    match stage {
        Stage::Invalidated | Stage::Retracted => CapsuleState::Invalidated,
        Stage::Expired => CapsuleState::Expired,
        _ => CapsuleState::Live,
    }
}

fn is_licensed(f: &Fact) -> bool {
    f.meaning
        .as_ref()
        .is_some_and(|m| m.license.class == LicenseClass::Licensed)
}

/// Fact ids of a story in join order (transitions' facts, first occurrence).
pub(crate) fn story_fact_ids(story: &Story) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for t in &story.history {
        for f in &t.facts {
            if !out.contains(f) {
                out.push(f.clone());
            }
        }
    }
    out
}

/// The sentence pair of a story for one reader as of bar `now` of `bars` (K26), from the
/// story and its facts only (no engine needed): `qp_capsule::sentence` rules with the
/// chosen realizer. `redacted`: no digit at all (K33).
///
/// (R3/perf) Study values come from `memo` (one pass over unchanged bars), and the
/// realizer's output from `cache` (capsule id, memo) when it already wrote this exact
/// input. `only`: a caller that reads one language gets that one written (the other is
/// empty unless the cache had both; nothing is cached then).
#[allow(clippy::too_many_arguments)]
pub(crate) fn sentence_pair_memo(
    story: &Story,
    facts: &[&Fact],
    bars: &[Bar],
    now: u64,
    redacted: bool,
    memo: &mut StudyMemo,
    cache: Option<(&str, &std::cell::RefCell<crate::capidx::SentenceMemo>)>,
    only: Option<Lang>,
) -> Option<(String, String)> {
    let now_i = usize::try_from(now).ok()?;
    let bars = bars.get(..=now_i)?;
    let facts: Vec<&Fact> = facts.iter().copied().filter(|f| f.bar <= now).collect();
    let lead = *facts.first()?;
    let latest = *facts.last()?;
    let (still_holds, current) = condition_now_memo(lead, &facts, bars, memo);
    let still_holds = qp_capsule::sentence::closed_override(story, still_holds);
    lead.meaning.as_ref()?;
    let current = if redacted { None } else { current };
    let key = crate::capidx::SentenceKeyRef {
        story,
        lead,
        latest,
        now_bar: now,
        now_t: bars[now_i].t,
        still_holds,
        current: current.map(f64::to_bits),
        redacted,
    };
    if let Some((id, c)) = cache {
        if let Some(v) = c.borrow().get(id, &key) {
            return Some(v);
        }
    }
    // (K42/A28) A state's expected resolution is not its state condition.
    let lead_view = qp_capsule::sentence::sentence_lead(lead);
    let lead_ref = lead_view.as_ref().unwrap_or(lead);
    let inp = SentenceInput {
        story,
        lead: lead_ref,
        latest: if latest.id == lead.id {
            lead_ref
        } else {
            latest
        },
        now_bar: now,
        now_t: bars[now_i].t,
        still_holds,
        current,
        redacted,
    };
    if let Some(l) = only {
        let t = Template.sentence(&inp, l);
        return Some(match l {
            Lang::Tr => (t, String::new()),
            Lang::En => (String::new(), t),
        });
    }
    let v = (
        Template.sentence(&inp, Lang::Tr),
        Template.sentence(&inp, Lang::En),
    );
    if let Some((id, c)) = cache {
        c.borrow_mut().put(id, &key, v.clone());
    }
    Some(v)
}

/// (R4, K42/A28) Validity of `story` as of bar `now` over `facts` (the reader's) and
/// `bars`: the same `still_holds` the sentence reads ([`sentence_pair_memo`]), so the
/// item's `validity`, the glyph and the sentence agree.
pub(crate) fn story_validity(
    story: &Story,
    facts: &[&Fact],
    bars: &[Bar],
    now: u64,
    memo: &mut StudyMemo,
) -> qp_schema::capsule::Validity {
    let still = usize::try_from(now)
        .ok()
        .and_then(|i| bars.get(..=i))
        .and_then(|bars| {
            let facts: Vec<&Fact> = facts.iter().copied().filter(|f| f.bar <= now).collect();
            let lead = *facts.first()?;
            Some(condition_now_memo(lead, &facts, bars, memo).0)
        })
        .flatten();
    let still = qp_capsule::sentence::closed_override(story, still);
    qp_capsule::sentence::validity_now(story, still)
}

/// What a principal may receive (see the module docs).
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Proj {
    /// The principal it is for: only its own capsule preferences (audience) go to it
    /// (empty: the host, which receives every audience).
    pub principal: String,
    /// Fact, finding and capsule ids that never go.
    pub hidden: BTreeSet<String>,
    /// Fact ids sent without evidence values and `evidence_ref`.
    pub redact: BTreeSet<String>,
    /// Capsule id → its story as the principal sees it (licensed joins removed), when
    /// that differs from the stored one.
    pub stories: BTreeMap<String, Story>,
    /// Capsules whose sentence must be written for this principal (redacted or with
    /// hidden facts).
    pub rewrite: BTreeSet<String>,
}

impl Proj {
    /// Whether the JSON of `v` holds a hidden id as a string (`"id"`).
    pub(crate) fn mentions_hidden(&self, v: &impl serde::Serialize) -> bool {
        if self.hidden.is_empty() {
            return false;
        }
        let s = serde_json::to_string(v).unwrap_or_default();
        // An id without a quote occurs as `"id"` exactly when it is the text between two
        // consecutive quotes of the JSON: look those up instead of searching every id.
        let mut quotes = s.match_indices('"').map(|(i, _)| i);
        if let Some(mut a) = quotes.next() {
            for b in quotes {
                if self.hidden.contains(&s[a + 1..b]) {
                    return true;
                }
                a = b;
            }
        }
        self.hidden
            .iter()
            .filter(|h| h.contains('"'))
            .any(|h| s.contains(&format!("\"{h}\"")))
    }
}

/// The story as seen without the transitions caused only by `hidden` facts: families of
/// the visible supporting facts, strength their maximum, and the rank as it was before
/// the first hidden transition.
fn project_story(story: &Story, hidden: &BTreeSet<String>, facts: &[Fact]) -> Story {
    let mut s = story.clone();
    let mut kept = Vec::new();
    let mut first_hidden: Option<usize> = None;
    for (i, t) in story.history.iter().enumerate() {
        if !t.facts.is_empty() && t.facts.iter().all(|f| hidden.contains(f)) {
            first_hidden.get_or_insert(i);
            continue;
        }
        kept.push(t.clone());
    }
    if first_hidden.is_none() {
        return s;
    }
    s.history = kept;
    if let Some(last) = s.history.last() {
        s.stage = last.stage;
        s.last_bar = last.bar;
    }
    let visible: Vec<&Fact> = facts.iter().filter(|f| !hidden.contains(&f.id)).collect();
    let mut fams: Vec<EvidenceFamily> = Vec::new();
    let mut strength: f64 = 0.0;
    for f in &visible {
        let Some(m) = f.meaning.as_ref() else {
            continue;
        };
        let opposes = matches!(
            (story.direction, m.direction),
            (
                qp_schema::capsule::Direction::Up,
                qp_schema::capsule::Direction::Down
            ) | (
                qp_schema::capsule::Direction::Down,
                qp_schema::capsule::Direction::Up
            )
        );
        if opposes || m.edge == qp_schema::capsule::Edge::Exit {
            continue;
        }
        if !fams.contains(&m.family) {
            fams.push(m.family);
        }
        if m.strength > strength {
            strength = m.strength;
        }
    }
    fams.sort();
    if !fams.is_empty() {
        s.families = fams;
        s.strength = strength;
    }
    s
}

impl Session {
    /// Starts the runner (`init.capsules`), before the first settle.
    pub(crate) fn start_capsules(&mut self, init: Option<&CapsuleInit>) {
        let keys = self
            .trust_keys
            .as_ref()
            .map(|(r, a)| (r.as_slice(), a.as_slice()));
        self.runner = init.map(|i| Runner::new(i, &self.doc, keys));
        self.start_public();
    }

    /// (R3, §20) Host-supplied secret keys of the server-fact trust path (mühür): the
    /// receipt key and the audit-log key (kept in the host's secret store, never in the
    /// document or the log). Re-keys every running trust path. Refused once any server
    /// comment has been issued (receipts already made with the old key would no longer
    /// match), so a host passes its keys at construction ([`Session::new_with_trust_keys`]). Without them
    /// the authority uses the public demo keys, which only exist for the synthetic
    /// test-host flow (`init.capsules.flow`, the only flow source in R3) and transcripts.
    ///
    /// # Errors
    ///
    /// A message when a server comment has already been issued.
    pub fn set_trust_keys(
        &mut self,
        receipt_key: Vec<u8>,
        audit_key: Vec<u8>,
    ) -> Result<(), String> {
        if let Some(r) = &mut self.runner {
            if !r.comments.is_empty() {
                return Err("trust keys must be set before the first server comment".to_owned());
            }
            for w in &mut r.watched {
                if w.muhur.is_some() {
                    w.muhur = Some(qp_capsule::trust::Muhur::with_keys(
                        qp_capsule::trust::Params::default(),
                        &receipt_key,
                        &audit_key,
                    ));
                }
            }
        }
        self.trust_keys = Some((receipt_key, audit_key));
        Ok(())
    }

    /// `true` when `principal` may read the licensed flow of `instrument` (or there is
    /// none).
    pub(crate) fn flow_ok(&self, principal: &str, instrument: &str) -> bool {
        let Some(r) = self.runner.as_ref() else {
            return true;
        };
        match r.flow_feed(instrument) {
            Some(k) => self.plane.entitled(principal, &k),
            None => true,
        }
    }

    /// `true` when `principal` may read the bars of series `id` (inline: always).
    pub(crate) fn bars_ok(&self, principal: &str, id: &str) -> bool {
        let doc = rd!(self);
        let Some(bs) = bar_series(doc, id).or_else(|| find_series(doc, id)) else {
            return true;
        };
        match bs.binding() {
            Some(b) => self.plane.entitled(principal, &b.feed_key()),
            None => true,
        }
    }

    /// What `principal` may receive of the engine's items; `None` when everything.
    ///
    /// (R3/perf) Kept per reader and capsule ([`crate::capidx::ReaderProj`]): brought up to
    /// date from the log entries since the last call (the capsules they put, and those
    /// naming the facts they put), rebuilt when a series is put, anything is deleted or an
    /// entitlement answer it was made with changed. Inside [`Session::as_reader`] (a
    /// projected document) and when its per-capsule rule does not hold, it is
    /// [`Session::proj_scratch`].
    pub(crate) fn proj_for(&self, principal: &str) -> Option<Rc<Proj>> {
        self.runner.as_ref()?;
        if self.proj.is_some() {
            return self.proj_scratch(principal).map(Rc::new);
        }
        let mut all = self.projs.borrow_mut();
        let rp = all
            .entry(principal.to_owned())
            .or_insert_with(|| crate::capidx::ReaderProj::new(principal));
        let fresh = rp.built
            && !rp.degenerate
            && rp.log_len <= self.log.len()
            && !crate::capidx::proj_breaks(&self.log[rp.log_len..])
            && self.entitlements_hold(principal, rp);
        if !fresh {
            *rp = crate::capidx::ReaderProj::new(principal);
            for c in &self.doc.capsules {
                if c.story.is_none() {
                    continue;
                }
                if rp.contains(&c.id) {
                    rp.degenerate = true;
                    break;
                }
                let x = self.contrib(principal, c, rp);
                rp.add(&c.id, x);
            }
            rp.built = true;
        } else if rp.log_len < self.log.len() {
            let mut caps: BTreeSet<String> = BTreeSet::new();
            for e in &self.log[rp.log_len..] {
                for ch in &e.changes {
                    match ch {
                        ChangeOp::PutCapsule { capsule, .. } => {
                            caps.insert(capsule.id.clone());
                        }
                        ChangeOp::PutFact { fact, .. } => {
                            caps.extend(rp.capsules_of_fact(&fact.id));
                        }
                        _ => {}
                    }
                }
            }
            for id in caps {
                rp.remove(&id);
                if let Some(c) = self.doc_capsule(&id).filter(|c| c.story.is_some()) {
                    let x = self.contrib(principal, c, rp);
                    rp.add(&id, x);
                }
            }
        }
        rp.log_len = self.log.len();
        if rp.degenerate {
            drop(all);
            return self.proj_scratch(principal).map(Rc::new);
        }
        rp.result()
    }

    /// Whether the entitlement answers `rp` was made with still hold.
    fn entitlements_hold(&self, principal: &str, rp: &crate::capidx::ReaderProj) -> bool {
        rp.flow
            .values()
            .chain(rp.bars.values())
            .all(|(k, a)| k.as_ref().is_none_or(|k| self.plane.entitled(principal, k)) == *a)
    }

    /// [`Session::flow_ok`], recorded in `rp`.
    fn flow_ok_in(
        &self,
        principal: &str,
        instrument: &str,
        rp: &mut crate::capidx::ReaderProj,
    ) -> bool {
        if let Some((_, a)) = rp.flow.get(instrument) {
            return *a;
        }
        let k = self.runner.as_ref().and_then(|r| r.flow_feed(instrument));
        let a = k.as_ref().is_none_or(|k| self.plane.entitled(principal, k));
        rp.flow.insert(instrument.to_owned(), (k, a));
        a
    }

    /// [`Session::bars_ok`], recorded in `rp`.
    fn bars_ok_in(&self, principal: &str, id: &str, rp: &mut crate::capidx::ReaderProj) -> bool {
        if let Some((_, a)) = rp.bars.get(id) {
            return *a;
        }
        let doc = rd!(self);
        let k = bar_series(doc, id)
            .or_else(|| find_series(doc, id))
            .and_then(qp_schema::Series::binding)
            .map(|b| b.feed_key());
        let a = k.as_ref().is_none_or(|k| self.plane.entitled(principal, k));
        rp.bars.insert(id.to_owned(), (k, a));
        a
    }

    /// What capsule `c` (with a story) adds to `principal`'s projection: the steps of
    /// [`Session::proj_scratch`] for one capsule.
    fn contrib(
        &self,
        principal: &str,
        c: &Capsule,
        rp: &mut crate::capidx::ReaderProj,
    ) -> crate::capidx::Contrib {
        let mut x = crate::capidx::Contrib::default();
        let Some(story) = c.story.as_ref() else {
            return x;
        };
        let ids = story_fact_ids(story);
        x.shown = ids
            .iter()
            .filter(|i| {
                self.doc_fact(i).is_some_and(|f| {
                    !is_licensed(f)
                        || self.flow_ok_in(
                            principal,
                            f.meaning.as_ref().map_or("", |m| m.instrument.as_str()),
                            rp,
                        )
                })
            })
            .cloned()
            .collect();
        x.facts.clone_from(&ids);
        let Some(lead) = ids.first().and_then(|i| self.doc_fact(i)) else {
            return x;
        };
        let flow_ok = self.flow_ok_in(principal, &story.instrument, rp);
        if is_licensed(lead) && !flow_ok {
            x.full = true;
            x.hidden.push(c.id.clone());
            x.hidden.push(c.finding.clone());
            x.hidden.extend(ids.iter().cloned());
            return x;
        }
        let bars_ok = self.bars_ok_in(principal, &story.series, rp);
        let mut hidden_here = BTreeSet::new();
        for i in &ids {
            if let Some(f) = self.doc_fact(i) {
                if is_licensed(f) && !flow_ok {
                    hidden_here.insert(i.clone());
                } else if !bars_ok && f.meaning.is_some() {
                    x.redact.push(i.clone());
                }
            }
        }
        if !bars_ok {
            x.rewrite = true;
        }
        if !hidden_here.is_empty() {
            let story_facts: Vec<Fact> = ids
                .iter()
                .filter_map(|i| self.doc_fact(i).cloned())
                .collect();
            x.story = Some(project_story(story, &hidden_here, &story_facts));
            x.rewrite = true;
            x.hidden.extend(hidden_here);
        }
        x
    }

    /// The reference computation of [`Session::proj_for`], over the document as it is.
    pub(crate) fn proj_scratch(&self, principal: &str) -> Option<Proj> {
        self.runner.as_ref()?;
        let doc = &self.doc;
        let mut p = Proj {
            principal: principal.to_owned(),
            ..Proj::default()
        };
        let facts: BTreeMap<&str, &Fact> = doc.facts.iter().map(|f| (f.id.as_str(), f)).collect();
        for c in &doc.capsules {
            let Some(story) = c.story.as_ref() else {
                continue;
            };
            let ids = story_fact_ids(story);
            let Some(lead) = ids.first().and_then(|i| facts.get(i.as_str())) else {
                continue;
            };
            let flow_ok = self.flow_ok(principal, &story.instrument);
            if is_licensed(lead) && !flow_ok {
                p.hidden.insert(c.id.clone());
                p.hidden.insert(c.finding.clone());
                for i in &ids {
                    p.hidden.insert(i.clone());
                }
                continue;
            }
            let bars_ok = self.bars_ok(principal, &story.series);
            let mut hidden_here = BTreeSet::new();
            for i in &ids {
                if let Some(f) = facts.get(i.as_str()) {
                    if is_licensed(f) && !flow_ok {
                        hidden_here.insert(i.clone());
                    } else if !bars_ok && f.meaning.is_some() {
                        p.redact.insert(i.clone());
                    }
                }
            }
            if !bars_ok {
                p.rewrite.insert(c.id.clone());
            }
            if !hidden_here.is_empty() {
                let story_facts: Vec<Fact> = ids
                    .iter()
                    .filter_map(|i| facts.get(i.as_str()).map(|f| (*f).clone()))
                    .collect();
                p.stories.insert(
                    c.id.clone(),
                    project_story(story, &hidden_here, &story_facts),
                );
                p.rewrite.insert(c.id.clone());
                p.hidden.extend(hidden_here);
            }
        }
        // A fact hidden in one story but shown in another stays shown there.
        let shown: BTreeSet<String> = doc
            .capsules
            .iter()
            .filter(|c| !p.hidden.contains(&c.id))
            .filter_map(|c| c.story.as_ref())
            .flat_map(story_fact_ids)
            .filter(|i| {
                facts.get(i.as_str()).is_some_and(|f| {
                    !is_licensed(f)
                        || self.flow_ok(
                            principal,
                            f.meaning.as_ref().map_or("", |m| m.instrument.as_str()),
                        )
                })
            })
            .collect();
        p.hidden.retain(|h| !shown.contains(h));
        if p.hidden.is_empty() && p.redact.is_empty() {
            return None;
        }
        Some(p)
    }

    /// The document's facts with their values (inside [`Session::as_reader`] the
    /// document is projected; sentences are still computed from the real facts and
    /// redacted afterwards).
    pub(crate) fn all_facts(&self) -> &[Fact] {
        if self.proj.is_some() {
            &self.real_facts
        } else {
            &self.doc.facts
        }
    }

    /// What `principal` may receive: [`Session::proj_for`], or — when another person's
    /// capsule preferences exist (or ever existed) — a projection that only removes
    /// those (positions and watch lists are personal: a reader gets its own audience
    /// only, in every document, change and log entry, owners included). `None`: the
    /// stored items go as they are.
    pub(crate) fn reader_proj(&self, principal: &str) -> Option<Rc<Proj>> {
        if let Some(p) = self.proj_for(principal) {
            if self.is_host(principal) {
                let mut p = (*p).clone();
                p.principal = String::new();
                return Some(Rc::new(p));
            }
            return Some(p);
        }
        if self.is_host(principal) {
            return None;
        }
        self.audience_seen.iter().any(|k| k != principal).then(|| {
            Rc::new(Proj {
                principal: principal.to_owned(),
                ..Proj::default()
            })
        })
    }

    /// Fact `id` among [`Session::all_facts`] (the first with that id).
    pub(crate) fn fact_by_id(&self, id: &str) -> Option<&Fact> {
        let facts = self.all_facts();
        let mut ix = self.fact_ix.borrow_mut();
        ix.refresh(&self.log, facts);
        ix.get(facts, id).map(|i| &facts[i])
    }

    /// Fact `id` of `self.doc` (inside [`Session::as_reader`]: the projected one).
    pub(crate) fn doc_fact(&self, id: &str) -> Option<&Fact> {
        if self.proj.is_some() {
            return self.doc.facts.iter().find(|f| f.id == id);
        }
        self.fact_by_id(id)
    }

    /// Capsule `id` of `self.doc`.
    pub(crate) fn doc_capsule(&self, id: &str) -> Option<&Capsule> {
        let caps = &self.doc.capsules;
        if self.proj.is_some() {
            return caps.iter().find(|c| c.id == id);
        }
        let mut ix = self.cap_ix.borrow_mut();
        ix.refresh(&self.log, caps);
        ix.get(caps, id).map(|i| &caps[i])
    }

    /// The first capsule of finding `f` in `self.doc`.
    pub(crate) fn doc_capsule_of(&self, f: &str) -> Option<&Capsule> {
        let caps = &self.doc.capsules;
        if self.proj.is_some() {
            return caps.iter().find(|c| c.finding == f);
        }
        let mut ix = self.cap_ix.borrow_mut();
        ix.refresh(&self.log, caps);
        ix.by_finding(caps, f).map(|i| &caps[i])
    }

    /// The host (the authority's operator, which holds the store anyway) receives every
    /// person's capsule preferences; nobody else, owners included, gets another's.
    fn is_host(&self, principal: &str) -> bool {
        self.principal(principal)
            .is_some_and(|p| p.role == qp_schema::protocol::Role::Host)
    }

    /// The language of connection `conn`'s capsule sentences.
    pub(crate) fn lang_of(&self, ci: usize) -> Language {
        self.conns[ci].surface.lang.unwrap_or_default()
    }

    /// Index of the last final bar of the story's series in `doc` (the sentences' "now").
    fn now_bar<'d>(doc: &'d Document, story: &Story) -> Option<(u64, &'d [Bar])> {
        let bs = bar_series(doc, &story.series)?;
        let bars = bs.bars()?;
        let n = bars.len();
        if n == 0 {
            return None;
        }
        let f = final_count(doc, &bs.id, n).max(1);
        let now = u64::try_from(f - 1).unwrap_or(0).max(story.last_bar);
        Some((now.min(u64::try_from(n - 1).unwrap_or(0)), bars))
    }

    /// The sentences of capsule `c` (with story) for a reader with projection `p`
    /// (`None`: entitled to everything), over the facts of `doc`.
    pub(crate) fn reader_sentence(
        &self,
        doc: &Document,
        c: &Capsule,
        p: Option<&Proj>,
        lang: Language,
    ) -> Option<(CapsuleSentence, bool)> {
        self.reader_sentence_with(doc, c, p, lang, &mut StudyMemo::default())
    }

    /// [`Session::reader_sentence`] with the study values of one pass over unchanged
    /// bars (`memo`) and the session's sentence memo.
    pub(crate) fn reader_sentence_with(
        &self,
        doc: &Document,
        c: &Capsule,
        p: Option<&Proj>,
        lang: Language,
        memo: &mut StudyMemo,
    ) -> Option<(CapsuleSentence, bool)> {
        let story = p.and_then(|p| p.stories.get(&c.id)).or(c.story.as_ref())?;
        let hidden = p.map(|p| &p.hidden);
        let facts: Vec<&Fact> = story_fact_ids(story)
            .iter()
            .filter(|i| hidden.is_none_or(|h| !h.contains(*i)))
            .filter_map(|i| self.fact_by_id(i))
            .collect();
        let redacted = p.is_some_and(|p| facts.iter().any(|f| p.redact.contains(&f.id)));
        let (now, bars) = Self::now_bar(rd!(self), story).or_else(|| Self::now_bar(doc, story))?;
        let (tr, en) = sentence_pair_memo(
            story,
            &facts,
            bars,
            now,
            redacted,
            memo,
            Some((&c.id, &self.sentences)),
            None,
        )?;
        let text = match lang {
            Language::Tr => tr.clone(),
            Language::En => en.clone(),
        };
        Some((CapsuleSentence { text, tr, en }, redacted))
    }

    /// The text of [`Session::reader_sentence_with`] only (one language written).
    pub(crate) fn reader_text_with(
        &self,
        doc: &Document,
        c: &Capsule,
        p: Option<&Proj>,
        lang: Language,
        memo: &mut StudyMemo,
    ) -> Option<String> {
        let story = p.and_then(|p| p.stories.get(&c.id)).or(c.story.as_ref())?;
        let hidden = p.map(|p| &p.hidden);
        let facts: Vec<&Fact> = story_fact_ids(story)
            .iter()
            .filter(|i| hidden.is_none_or(|h| !h.contains(*i)))
            .filter_map(|i| self.fact_by_id(i))
            .collect();
        let redacted = p.is_some_and(|p| facts.iter().any(|f| p.redact.contains(&f.id)));
        let (now, bars) = Self::now_bar(rd!(self), story).or_else(|| Self::now_bar(doc, story))?;
        let (tr, en) = sentence_pair_memo(
            story,
            &facts,
            bars,
            now,
            redacted,
            memo,
            Some((&c.id, &self.sentences)),
            Some(match lang {
                Language::Tr => Lang::Tr,
                Language::En => Lang::En,
            }),
        )?;
        Some(match lang {
            Language::Tr => tr,
            Language::En => en,
        })
    }

    /// (R4, K42/A28) Validity of capsule `c` for a reader with projection `p`, as of the
    /// same bar and over the same facts as [`Session::reader_sentence_with`].
    pub(crate) fn reader_validity(
        &self,
        doc: &Document,
        c: &Capsule,
        p: Option<&Proj>,
        memo: &mut StudyMemo,
    ) -> Option<qp_schema::capsule::Validity> {
        let story = p.and_then(|p| p.stories.get(&c.id)).or(c.story.as_ref())?;
        let hidden = p.map(|p| &p.hidden);
        let facts: Vec<&Fact> = story_fact_ids(story)
            .iter()
            .filter(|i| hidden.is_none_or(|h| !h.contains(*i)))
            .filter_map(|i| self.fact_by_id(i))
            .collect();
        let (now, bars) = Self::now_bar(rd!(self), story).or_else(|| Self::now_bar(doc, story))?;
        Some(story_validity(story, &facts, bars, now, memo))
    }

    /// (R4/H-FP) The headline pieces of capsule `c` for a reader with projection `p` in
    /// `lang` ([`crate::headparts`]): the same story, facts, "now", validity and
    /// redaction as [`Session::reader_text_with`] writes the reader's sentence with.
    pub(crate) fn reader_parts(
        &self,
        doc: &Document,
        c: &Capsule,
        p: Option<&Proj>,
        lang: Language,
        memo: &mut StudyMemo,
    ) -> Option<qp_grammar::HeadlineParts> {
        let story = p.and_then(|p| p.stories.get(&c.id)).or(c.story.as_ref())?;
        let hidden = p.map(|p| &p.hidden);
        let facts: Vec<&Fact> = story_fact_ids(story)
            .iter()
            .filter(|i| hidden.is_none_or(|h| !h.contains(*i)))
            .filter_map(|i| self.fact_by_id(i))
            .collect();
        let redacted = p.is_some_and(|p| facts.iter().any(|f| p.redact.contains(&f.id)));
        let (now, bars, bs) =
            Self::now_series(rd!(self), story).or_else(|| Self::now_series(doc, story))?;
        let validity = story_validity(story, &facts, bars, now, memo);
        crate::headparts::story_parts(
            story,
            (bs, bars, now),
            validity,
            redacted,
            lang,
            self.headline_cal.as_ref(),
        )
    }

    /// [`Session::now_bar`] with the bar series it read.
    fn now_series<'d>(
        doc: &'d Document,
        story: &Story,
    ) -> Option<(u64, &'d [Bar], &'d qp_schema::Series)> {
        let (now, bars) = Self::now_bar(doc, story)?;
        Some((now, bars, bar_series(doc, &story.series)?))
    }

    /// (R4/H-FP) The headline pieces of the capsule of `item` (the authority's `capsules`
    /// answer for this reader) in `lang`, over this session's own story and bars: what the
    /// shell's replica gives a face design's headline template. Validity and redaction are
    /// the item's (the authority's, K42/A28, K33); "now" is the last final bar of the
    /// story's series, as for the sentence. `None` without the capsule's story or bars
    /// (an unentitled reader's replica has no bars; its face is text anyway).
    #[must_use]
    pub fn item_headline_parts(
        &self,
        item: &qp_schema::protocol::CapsuleItem,
        lang: Language,
    ) -> Option<qp_grammar::HeadlineParts> {
        let caps = &rd!(self).capsules;
        let c = caps
            .iter()
            .find(|c| c.id == item.id)
            .or_else(|| caps.iter().find(|c| c.finding == item.finding))
            .or_else(|| self.doc.capsules.iter().find(|c| c.id == item.id))?;
        let story = c.story.as_ref()?;
        let (now, bars, bs) =
            Self::now_series(rd!(self), story).or_else(|| Self::now_series(&self.doc, story))?;
        crate::headparts::story_parts(
            story,
            (bs, bars, now),
            item.validity,
            item.redacted,
            lang,
            self.headline_cal.as_ref(),
        )
    }

    /// (R4/H-FP) The exchange calendar of headline pieces for bar series whose instrument
    /// names none (`None`: the built-in `XIST`); see [`crate::headparts`]. A replay host
    /// sets [`crate::headparts::replay_calendar`] of its source.
    pub fn set_headline_calendar(&mut self, cal: Option<qp_schema::Calendar>) {
        self.headline_cal = cal;
    }

    // -----------------------------------------------------------------------
    // Runner: final bars → engine entries
    // -----------------------------------------------------------------------

    /// Feeds every bar that became final on a watched series to the detectors and the
    /// engine; one engine entry per series that advanced; returns its notifications
    /// (`change`, `finding_state`, topic `capsules`).
    pub(crate) fn run_capsules(&mut self) -> Vec<OutboundLine> {
        let Some(mut runner) = self.runner.take() else {
            return Vec::new();
        };
        let mut lines = Vec::new();
        // (R3/perf) The watched series' positions in the reading document, per structure
        // generation (only a structure change moves or renames series).
        if runner.pos_gen != self.struct_gen {
            let rd = rd!(self);
            for w in &mut runner.watched {
                w.pos = rd.series.iter().position(|s| s.id == w.series);
            }
            runner.pos_gen = self.struct_gen;
        }
        let mut wrote = false;
        for wi in 0..runner.watched.len() {
            let rd = rd!(self);
            let w = &runner.watched[wi];
            let Some(bs) = w
                .pos
                .and_then(|i| rd.series.get(i))
                .filter(|s| s.id == w.series)
                .or_else(|| find_series(rd, &w.series))
            else {
                continue;
            };
            let Some(all) = bs.bars() else {
                continue;
            };
            // `final_count(rd, &bs.id, …)`: `bs` is the series found under that id.
            let fin = final_count_of(Some(bs), all.len());
            if fin <= w.done {
                continue;
            }
            let bars: &[Bar] = &all[..fin];
            let meta = bs;
            let w = &mut runner.watched[wi];
            let flow = w.flow.as_ref().map(|(seed, _)| flow_points(bars, *seed));
            let from = w.done;
            let mut events = Vec::new();
            let mut events_pub = Vec::new();
            let mut made: BTreeMap<String, Fact> = BTreeMap::new();
            let mut said: Vec<ServerComment> = Vec::new();
            for i in from..fin {
                let fp = flow.as_ref().and_then(|f| f.get(i));
                let mut facts = Vec::new();
                for det in &mut w.dets {
                    for d in det.push(&bars[i], fp) {
                        let mut f = qp_facts::to_fact(
                            det.as_ref(),
                            &d,
                            &w.symbol,
                            &w.series,
                            i as u64,
                            bars[i].t,
                            qp_capsule::pipeline::ENGINE_VERSION,
                        );
                        f.evidence_ref = Some(crate::plane::evidence_ref(meta, bars, i));
                        facts.push(f);
                    }
                }
                let evs = runner
                    .engine
                    .on_bar(&w.symbol, &w.series, i as u64, &bars[..=i], &facts);
                // (R3/K1) The public-only engine: the same bar, public facts only.
                if let Some(pe) = runner.pub_engine.as_mut() {
                    let evp = if facts.iter().any(is_licensed) {
                        let public: Vec<Fact> =
                            facts.iter().filter(|f| !is_licensed(f)).cloned().collect();
                        pe.on_bar(&w.symbol, &w.series, i as u64, &bars[..=i], &public)
                    } else {
                        pe.on_bar(&w.symbol, &w.series, i as u64, &bars[..=i], &facts)
                    };
                    events_pub.extend(evp);
                }
                if let (Some(m), Some(p)) = (w.muhur.as_mut(), fp) {
                    let server: Vec<Fact> =
                        facts.iter().filter(|f| is_licensed(f)).cloned().collect();
                    let obs = FlowObservation {
                        t: bars[i].t,
                        bar: i as u64,
                        instrument: w.symbol.clone(),
                        values: p.values.clone(),
                    };
                    for q in m.observe(&obs, &server) {
                        said.push(ServerComment {
                            instrument: q.instrument,
                            series: w.series.clone(),
                            bar: q.bar,
                            t: q.t,
                            codes: q.codes,
                            tr: q.tr,
                            en: q.en,
                            trust: q.trust,
                            attachments: q.attachments,
                        });
                    }
                }
                for f in facts {
                    made.insert(f.id.clone(), f);
                }
                events.extend(evs);
            }
            w.done = fin;
            let series = w.series.clone();
            let changes = self.capsule_changes(&runner.engine, &events, &made, fin - 1);
            if let Some(pe) = runner.pub_engine.as_ref() {
                let changes =
                    self.in_world(true, |s| s.capsule_changes(pe, &events_pub, &made, fin - 1));
                self.pending_pub = Some(crate::public::PubPending::of(changes));
            }
            let summary = if fin - from == 1 {
                format!("capsules {series} bar {from}")
            } else {
                format!("capsules {series} bars {from}..={}", fin - 1)
            };
            let Some(seq) = self.engine_entry_with(changes, summary, None) else {
                self.pending_pub = None;
                continue;
            };
            for c in &said {
                runner.comments.push((seq, c.clone()));
            }
            self.runner = Some(runner);
            wrote = true;
            lines.extend(self.entry_events(seq));
            lines.extend(self.capsule_topic_lines(seq, &events, &events_pub, &said));
            runner = self.runner.take().expect("runner put back");
        }
        self.runner = Some(runner);
        // (R3/perf) The headlines once, after the last engine entry (nothing in between
        // reads them; the document they are written from is the last entry's).
        if wrote {
            self.refresh_headlines();
        }
        lines
    }

    /// The changes of one look: facts that caused transitions, then per story its
    /// finding and capsule (last snapshot), then ledger rows of the closes.
    fn capsule_changes(
        &self,
        engine: &Engine,
        events: &[qp_capsule::StoryEvent],
        made: &BTreeMap<String, Fact>,
        now: usize,
    ) -> Vec<ChangeOp> {
        let mut changes = Vec::new();
        let mut put_facts: BTreeSet<String> = BTreeSet::new();
        let mut order: Vec<&str> = Vec::new();
        let mut last: BTreeMap<&str, &qp_capsule::StoryEvent> = BTreeMap::new();
        for e in events {
            for f in &e.transition.facts {
                if put_facts.contains(f) || self.doc_fact(f).is_some() {
                    continue;
                }
                if let Some(fact) = made.get(f) {
                    put_facts.insert(f.clone());
                    changes.push(ChangeOp::PutFact {
                        fact: fact.clone(),
                        index: None,
                    });
                }
            }
            if !last.contains_key(e.capsule.as_str()) {
                order.push(&e.capsule);
            }
            last.insert(&e.capsule, e);
        }
        let rd = rd!(self);
        let mut memo = StudyMemo::default();
        for id in order {
            let e = last[id];
            let story = &e.story;
            let ids = story_fact_ids(story);
            let facts: Vec<&Fact> = ids
                .iter()
                .filter_map(|i| made.get(i).or_else(|| self.doc_fact(i)))
                .collect();
            let bars = bar_series(rd, &story.series)
                .and_then(qp_schema::Series::bars)
                .unwrap_or_default();
            let now = (now as u64).max(story.last_bar);
            let (tr, _) = sentence_pair_memo(
                story,
                &facts,
                bars,
                now,
                false,
                &mut memo,
                Some((&e.capsule, &self.sentences)),
                Some(Lang::Tr),
            )
            .unwrap_or_else(|| (e.finding.clone(), e.finding.clone()));
            let at = e.transition.t;
            let prov = Provenance {
                author: Author::Engine {
                    name: ENGINE_NAME.to_owned(),
                    version: qp_capsule::pipeline::ENGINE_VERSION.to_owned(),
                },
                at,
                based_on: vec![story.series.clone()],
                approved_by: None,
            };
            let existing = self.doc_capsule(&e.capsule);
            changes.push(ChangeOp::PutFinding {
                finding: Finding {
                    id: e.finding.clone(),
                    claim: tr.clone(),
                    facts: ids,
                    confidence: story.strength.clamp(0.0, 1.0),
                    invalidation: Condition::Text {
                        text: OUTCOME_TEXT.to_owned(),
                    },
                    provenance: prov.clone(),
                },
                index: None,
            });
            let state = match existing.map(|c| c.state) {
                Some(s @ (CapsuleState::Approved | CapsuleState::Rejected)) => s,
                _ => state_of(story.stage),
            };
            changes.push(ChangeOp::PutCapsule {
                capsule: Capsule {
                    id: e.capsule.clone(),
                    finding: e.finding.clone(),
                    state,
                    revision: u32::try_from(story.history.len().saturating_sub(1))
                        .unwrap_or(u32::MAX),
                    headline: tr,
                    glyphs: Vec::new(),
                    view: existing.and_then(|c| c.view.clone()),
                    depth: Depth::Capsule,
                    expires_at: None,
                    story: Some(story.clone()),
                    provenance: prov,
                },
                index: None,
            });
        }
        for e in events {
            if matches!(
                e.transition.stage,
                Stage::Realized | Stage::Invalidated | Stage::Expired
            ) {
                if let Some(r) = engine
                    .ledger_records()
                    .iter()
                    .rev()
                    .find(|r| r.entry.capsule.as_deref() == Some(e.capsule.as_str()))
                {
                    changes.push(ChangeOp::PushLedger {
                        entry: r.entry.clone(),
                    });
                }
            }
        }
        changes
    }

    // -----------------------------------------------------------------------
    // Items for a reader
    // -----------------------------------------------------------------------

    /// One capsule as listed for a reader (`p`: its projection; `doc`: the document it
    /// sees).
    pub(crate) fn capsule_item(
        &self,
        doc: &Document,
        c: &Capsule,
        p: Option<&Proj>,
        lang: Language,
        principal: &str,
    ) -> Option<CapsuleItem> {
        self.capsule_item_with(doc, c, p, lang, principal, &mut StudyMemo::default())
    }

    /// [`Session::capsule_item`] with the study values of one pass (`memo`).
    pub(crate) fn capsule_item_with(
        &self,
        doc: &Document,
        c: &Capsule,
        p: Option<&Proj>,
        lang: Language,
        principal: &str,
        memo: &mut StudyMemo,
    ) -> Option<CapsuleItem> {
        let story = p.and_then(|p| p.stories.get(&c.id)).or(c.story.as_ref())?;
        let (sentence, redacted) = self.reader_sentence_with(doc, c, p, lang, memo)?;
        let validity = self.reader_validity(doc, c, p, memo)?;
        let last = story.history.last()?;
        Some(CapsuleItem {
            id: c.id.clone(),
            finding: c.finding.clone(),
            instrument: story.instrument.clone(),
            series: story.series.clone(),
            kind: story.kind,
            direction: story.direction,
            time: story.time,
            stage: story.stage,
            state: c.state,
            tier: story.tier,
            importance: story.importance,
            confirmations: u8::try_from(story.families.len().clamp(1, 5)).unwrap_or(5),
            trust: story.trust,
            born_bar: story.born_bar,
            last_bar: story.last_bar,
            t: last.t,
            ledger_at_birth: story.ledger_at_birth,
            sentence,
            validity,
            relevant: self.relevant(doc, principal, &story.instrument),
            redacted,
        })
    }

    /// The audience of `principal` (the `habituated` default when it set none).
    pub(crate) fn audience_of(doc: &Document, principal: &str) -> Audience {
        doc.audiences.get(principal).cloned().unwrap_or(Audience {
            group: UserGroup::Habituated,
            sensitivity: None,
            watchlist: Vec::new(),
            positions: BTreeMap::new(),
        })
    }

    /// Instruments of the active intents' triggers (symbols of the series they read).
    fn intents_on(doc: &Document) -> Vec<String> {
        let mut out = Vec::new();
        for i in doc.intents.iter().filter(|i| i.active) {
            let Some(t) = &i.trigger else { continue };
            for (_, s) in crate::values::condition_series(t) {
                if let Some(sym) = bar_series(doc, s).and_then(qp_schema::Series::symbol) {
                    if !out.iter().any(|x| x == sym) {
                        out.push(sym.to_owned());
                    }
                }
            }
        }
        out
    }

    /// About `principal`'s position or one of the document's active intents.
    fn relevant(&self, doc: &Document, principal: &str, instrument: &str) -> bool {
        let a = Self::audience_of(doc, principal);
        a.positions.get(instrument).is_some_and(|q| *q != 0.0)
            || Self::intents_on(doc).iter().any(|s| s == instrument)
    }

    /// Whether a story reaches `principal`'s audience: watch list (empty = every
    /// instrument), positions and intents; then the sensitivity filter with the
    /// position/intent override (`qp_capsule::admits`).
    ///
    /// (K42/A22) `stage` is the transition judged: the story's current stage for the
    /// `capsules` query, the notified transition's stage for the topic. Only a negative
    /// transition of a position/intent story passes whatever the level.
    fn admits(doc: &Document, principal: &str, story: &Story, stage: Stage) -> bool {
        let a = Self::audience_of(doc, principal);
        let intents = Self::intents_on(doc);
        let watched = a.watchlist.is_empty()
            || a.watchlist.iter().any(|s| s == &story.instrument)
            || a.positions.contains_key(&story.instrument)
            || intents.iter().any(|s| s == &story.instrument);
        watched && qp_capsule::admits_at(&a, story, stage, &intents)
    }

    /// Server comments `principal` gets: those on instruments whose licensed flow it may
    /// not read.
    fn comments_for(&self, principal: &str) -> Vec<ServerComment> {
        let Some(r) = self.runner.as_ref() else {
            return Vec::new();
        };
        r.comments
            .iter()
            .filter(|(_, c)| !self.flow_ok(principal, &c.instrument))
            .map(|(_, c)| c.clone())
            .collect()
    }

    /// Whether `story` passes the filters of the `capsules` query `q` (all but
    /// `latest` and `limit`).
    fn capsule_passes(
        &self,
        doc: &Document,
        principal: &str,
        story: &Story,
        q: &qp_schema::protocol::Query,
    ) -> bool {
        let qp_schema::protocol::Query::Capsules {
            instrument,
            stages,
            open,
            meaning,
            since_bar,
            since_t,
            born_since_bar,
            born_since_t,
            audience,
            ..
        } = q
        else {
            return false;
        };
        !(instrument.as_ref().is_some_and(|i| i != &story.instrument)
            || (!stages.is_empty() && !stages.contains(&story.stage))
            || open.is_some_and(|o| o == story.stage.is_closed())
            || meaning.is_some_and(|k| k != story.kind)
            || since_bar.is_some_and(|b| story.last_bar < b)
            || since_t.is_some_and(|t| story.history.last().is_none_or(|l| l.t < t))
            || born_since_bar.is_some_and(|b| story.born_bar < b)
            || born_since_t.is_some_and(|t| story.history.first().is_none_or(|h| h.t < t))
            || (*audience && !Self::admits(doc, principal, story, story.stage)))
    }

    /// Answer to the `capsules` query (run on the asker's projected document).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn capsules_query(
        &self,
        ci: usize,
        q: &qp_schema::protocol::Query,
    ) -> PResult<QueryResult> {
        let qp_schema::protocol::Query::Capsules {
            instrument,
            since_bar,
            since_t,
            latest,
            limit,
            ..
        } = q
        else {
            unreachable!("capsules query");
        };
        let principal = self.conns[ci].principal.clone();
        let lang = self.lang_of(ci);
        let p = self.proj.as_deref();
        let doc = &self.doc;
        let mut items: Vec<CapsuleItem> = Vec::new();
        let mut memo = StudyMemo::default();
        // (R4) `latest`: the most recently born match only (birth time, bar, id).
        let mut newest: Option<(i64, u64, &str)> = None;
        if *latest {
            for c in &doc.capsules {
                let Some(story) = p.and_then(|p| p.stories.get(&c.id)).or(c.story.as_ref()) else {
                    continue;
                };
                if !self.capsule_passes(doc, &principal, story, q) {
                    continue;
                }
                let key = (
                    story.history.first().map_or(i64::MIN, |h| h.t),
                    story.born_bar,
                    c.id.as_str(),
                );
                if newest.is_none_or(|n| key > n) {
                    newest = Some(key);
                }
            }
        }
        for c in &doc.capsules {
            if *latest && newest.is_none_or(|n| n.2 != c.id) {
                continue;
            }
            let Some(story) = p.and_then(|p| p.stories.get(&c.id)).or(c.story.as_ref()) else {
                continue;
            };
            if !self.capsule_passes(doc, &principal, story, q) {
                continue;
            }
            if let Some(it) = self.capsule_item_with(doc, c, p, lang, &principal, &mut memo) {
                items.push(it);
            }
        }
        items.sort_by(|a, b| {
            b.importance
                .total_cmp(&a.importance)
                .then(b.t.cmp(&a.t))
                .then(a.id.cmp(&b.id))
        });
        if let Some(l) = limit {
            items.truncate(*l as usize);
        }
        let comments = self
            .comments_for(&principal)
            .into_iter()
            .filter(|c| instrument.as_ref().is_none_or(|i| i == &c.instrument))
            .filter(|c| since_bar.is_none_or(|b| c.bar >= b))
            .filter(|c| since_t.is_none_or(|t| c.t >= t))
            .collect();
        Ok(QueryResult::Capsules {
            capsules: items,
            comments,
        })
    }

    /// Answer to the `capsule` query (run on the asker's projected document).
    pub(crate) fn capsule_query(&self, ci: usize, id: &str, path: &str) -> PResult<QueryResult> {
        let principal = self.conns[ci].principal.clone();
        let lang = self.lang_of(ci);
        let p = self.proj.as_deref();
        let doc = &self.doc;
        let Some(c) = doc
            .capsules
            .iter()
            .filter(|c| c.story.is_some())
            .find(|c| c.id == id || c.finding == id)
        else {
            return Err(E::new(
                ErrorCode::NotFound,
                format!("no capsule with a story `{id}`"),
            )
            .at(ptr(path, "capsule"))
            .dym(
                id,
                doc.capsules
                    .iter()
                    .filter(|c| c.story.is_some())
                    .map(|c| c.id.as_str()),
            )
            .build());
        };
        let item = self
            .capsule_item(doc, c, p, lang, &principal)
            .ok_or_else(|| {
                E::new(ErrorCode::NoData, "the capsule's bars are not available").build()
            })?;
        let story = p
            .and_then(|p| p.stories.get(&c.id))
            .or(c.story.as_ref())
            .cloned()
            .unwrap_or_else(|| unreachable!("story checked"));
        let facts: Vec<Fact> = story_fact_ids(&story)
            .iter()
            .filter_map(|i| doc.facts.iter().find(|f| &f.id == i).cloned())
            .collect();
        let rd = rd!(self);
        let (now, bars) = Self::now_bar(rd, &story).unwrap_or((story.last_bar, &[]));
        let now_t = bars
            .get(usize::try_from(now).unwrap_or(0) + 1)
            .or_else(|| bars.get(usize::try_from(now).unwrap_or(0)))
            .map_or(i64::MAX, |b| b.t);
        let ledger = self
            .ledger_now()
            .into_iter()
            .filter(|(k, ..)| k == &story.ledger_key)
            .fold(
                qp_schema::capsule::LedgerStat::default(),
                |mut s, (_, _, t, real)| {
                    if t < now_t {
                        s.n += 1;
                        s.realized += u32::from(real);
                    }
                    s
                },
            );
        // (K42/A28) The glyph carries the item's validity: it says what the sentence says.
        let glyph = qp_capsule::glyph_spec_at(
            &story,
            now,
            qp_grammar::depth::GLYPH_MIN_LEDGER_N,
            item.validity,
        );
        let evidence = facts.iter().map(|f| self.evidence_access(f, p)).collect();
        Ok(QueryResult::CapsuleDetail {
            detail: Box::new(CapsuleDetail {
                item,
                story,
                facts,
                ledger,
                glyph,
                evidence,
                withheld: None,
            }),
        })
    }

    /// (R4/S1, §20.2, ergonomics C11) What a capsule selector `sel` may also have matched
    /// that `principal` cannot see: when the reader lacks the licensed flow of the
    /// instrument (the selector's, else the resolved capsule's) and the selector does not
    /// rule flow out, the server comments the reader already gets (`capsules`) that are
    /// newer than the resolved capsule's latest transition, with their entitlement. No new
    /// existence oracle: only what the comments told this reader already.
    pub(crate) fn withheld_for(
        &self,
        principal: &str,
        sel: &qp_schema::protocol::CapsuleSel,
        item: &CapsuleItem,
    ) -> Option<qp_schema::protocol::Withheld> {
        if sel
            .meaning
            .is_some_and(|m| m != qp_schema::capsule::MeaningKind::Flow)
        {
            return None;
        }
        let instrument = sel.instrument.as_deref().unwrap_or(&item.instrument);
        let comments: Vec<ServerComment> = self
            .comments_for(principal)
            .into_iter()
            .filter(|c| c.instrument == instrument && c.t > item.t)
            .collect();
        if comments.is_empty() {
            return None;
        }
        let mut requires: Vec<String> = comments
            .iter()
            .filter_map(|c| c.attachments.get("entitlement").cloned())
            .collect();
        requires.sort();
        requires.dedup();
        Some(qp_schema::protocol::Withheld { requires, comments })
    }

    /// One fact's evidence for a reader (K34): the values, or the entitlement they
    /// require and what they rest on.
    fn evidence_access(&self, f: &Fact, p: Option<&Proj>) -> EvidenceAccess {
        let m = f.meaning.as_ref();
        let licensed = m.is_some_and(|m| m.license.class == LicenseClass::Licensed);
        let open = p.is_none_or(|p| !p.redact.contains(&f.id));
        let doc = rd!(self);
        let bs = bar_series(doc, &f.series);
        let feed = bs.and_then(qp_schema::Series::binding);
        let symbol = m.map_or_else(|| f.series.clone(), |m| m.instrument.clone());
        let mut rests_on = match &feed {
            Some(b) => format!(
                "bars {} {} (feed {})",
                b.symbol,
                b.interval,
                b.feed.as_deref().unwrap_or("default")
            ),
            None => format!(
                "bars of series {}",
                bs.map_or(f.series.as_str(), |s| s.id.as_str())
            ),
        };
        let requires = if open {
            None
        } else if licensed {
            m.and_then(|m| m.license.entitlement.clone())
        } else {
            feed.as_ref().map(|b| match &b.feed {
                Some(fd) => format!("{fd}/{}", b.symbol),
                None => b.symbol.clone(),
            })
        };
        if licensed {
            let det = m.map_or("", |m| m.detector.as_str());
            rests_on = format!("licensed flow of {symbol} ({det}); {rests_on}");
        }
        EvidenceAccess {
            fact: f.id.clone(),
            family: m.map_or(EvidenceFamily::Price, |m| m.family),
            license: m.map_or(LicenseClass::Public, |m| m.license.class),
            open,
            values: if open {
                f.evidence.clone()
            } else {
                BTreeMap::new()
            },
            evidence_ref: if open { f.evidence_ref.clone() } else { None },
            requires,
            rests_on,
        }
    }

    /// The ledger rows with a key, retracted capsules left out: the engine's when the
    /// authority runs it (license-independent, K33: the same for every reader, also
    /// inside [`Session::as_reader`]), else the document's.
    fn ledger_now(&self) -> Vec<(String, qp_schema::capsule::MeaningKind, i64, bool)> {
        match self.runner.as_ref() {
            Some(r) => {
                // (R3/K1) In the public world, the public-only engine's.
                let engine = match (self.in_public, r.pub_engine.as_ref()) {
                    (true, Some(e)) => e,
                    _ => &r.engine,
                };
                let doc = Document {
                    ledger: qp_schema::Ledger {
                        entries: engine.ledger(),
                    },
                    ..Document {
                        schema_version: String::new(),
                        id: String::new(),
                        title: None,
                        display_tz: None,
                        series: Vec::new(),
                        views: Vec::new(),
                        facts: Vec::new(),
                        findings: Vec::new(),
                        capsules: Vec::new(),
                        intents: Vec::new(),
                        ledger: qp_schema::Ledger::default(),
                        narratives: Vec::new(),
                        proposals: Vec::new(),
                        audiences: BTreeMap::new(),
                    }
                };
                ledger_rows(&doc)
            }
            None => ledger_rows(&self.doc),
        }
    }

    /// Answer to the `ledger` query: rows of the ledger with a key, the retracted
    /// capsules left out.
    pub(crate) fn ledger_query(
        &self,
        key: Option<&str>,
        meaning: Option<qp_schema::capsule::MeaningKind>,
        detector: Option<&str>,
        direction: Option<qp_schema::capsule::Direction>,
        by_kind: bool,
    ) -> QueryResult {
        let mut rows: BTreeMap<String, LedgerRow> = BTreeMap::new();
        let mut total = qp_schema::capsule::LedgerStat::default();
        for (k, kind, _, real) in self.ledger_now() {
            let parsed = qp_capsule::parse_ledger_key(&k);
            let det = parsed.map(|(_, _, d, _)| d.to_owned());
            let dir = parsed.and_then(|(_, d, _, _)| d);
            if key.is_some_and(|x| x != k)
                || meaning.is_some_and(|m| m != kind)
                || detector.is_some_and(|d| det.as_deref() != Some(d))
                || direction.is_some_and(|d| dir != Some(d))
            {
                continue;
            }
            let rk = if by_kind {
                kind.name().to_owned()
            } else {
                k.clone()
            };
            let row = rows.entry(rk.clone()).or_insert(LedgerRow {
                key: rk,
                kind,
                direction: if by_kind { direction } else { dir },
                detector: if by_kind { None } else { det },
                n: 0,
                realized: 0,
            });
            row.n += 1;
            row.realized += u32::from(real);
            total.n += 1;
            total.realized += u32::from(real);
        }
        QueryResult::Ledger {
            rows: rows.into_values().collect(),
            total,
        }
    }

    // -----------------------------------------------------------------------
    // Notifications
    // -----------------------------------------------------------------------

    /// Topic `capsules` notifications of one engine entry: per subscriber, the story
    /// transitions it may see and that pass its filter; server comments to subscribers
    /// without the flow entitlement. (R3/K1) A public-view reader's transitions are the
    /// public-only engine's (`events_pub`), read in the public world.
    pub(crate) fn capsule_topic_lines(
        &mut self,
        seq: Seq,
        events: &[qp_capsule::StoryEvent],
        events_pub: &[qp_capsule::StoryEvent],
        said: &[ServerComment],
    ) -> Vec<OutboundLine> {
        let mut lines = self.capsule_topic_lines_in(seq, events, said, false);
        // (R3/K1b) Under the public entry's own number (every look makes one).
        let pseq = self.pub_of(seq);
        if let (true, Some(pseq)) = (
            self.other.is_some() && self.conns.iter().any(|c| self.public_view(&c.principal)),
            pseq,
        ) {
            let p = self.in_world(true, |s| {
                s.capsule_topic_lines_in(pseq, events_pub, said, true)
            });
            lines.extend(p);
            lines.sort_by_key(|(ci, _)| *ci);
        }
        lines.into_iter().map(|(_, l)| l).collect()
    }

    /// [`Session::capsule_topic_lines`] for the connections of the world the fields hold
    /// (`public`), each line with its connection index.
    fn capsule_topic_lines_in(
        &self,
        seq: Seq,
        events: &[qp_capsule::StoryEvent],
        said: &[ServerComment],
        public: bool,
    ) -> Vec<(usize, OutboundLine)> {
        let mut lines = Vec::new();
        for (ci, c) in self.conns.iter().enumerate() {
            if self.other.is_some() && self.public_view(&c.principal) != public {
                continue;
            }
            let subs: Vec<(&str, Option<&String>, bool)> = c
                .subs
                .iter()
                .filter_map(|s| match &s.topic {
                    Topic::Capsules { instrument, all } => {
                        Some((s.id.as_str(), instrument.as_ref(), *all))
                    }
                    _ => None,
                })
                .collect();
            if subs.is_empty() {
                continue;
            }
            let p = self.proj_for(&c.principal);
            let doc = &self.doc;
            let lang = self.lang_of(ci);
            let mut memo = StudyMemo::default();
            for e in events {
                if p.as_ref().is_some_and(|p| {
                    p.hidden.contains(&e.capsule)
                        || (!e.transition.facts.is_empty()
                            && e.transition.facts.iter().all(|f| p.hidden.contains(f)))
                }) {
                    continue;
                }
                let Some(cap) = self.doc_capsule(&e.capsule) else {
                    continue;
                };
                let Some(item) =
                    self.capsule_item_with(doc, cap, p.as_deref(), lang, &c.principal, &mut memo)
                else {
                    continue;
                };
                for (sub, inst, all) in &subs {
                    if inst.is_some_and(|i| i != &item.instrument) {
                        continue;
                    }
                    if !all {
                        let story = p
                            .as_ref()
                            .and_then(|p| p.stories.get(&cap.id))
                            .or(cap.story.as_ref());
                        if !story
                            .is_some_and(|s| Self::admits(doc, &c.principal, s, e.transition.stage))
                        {
                            continue;
                        }
                    }
                    lines.push((
                        ci,
                        out(
                            &c.id,
                            ServerMsg::Notify {
                                sub: (*sub).to_owned(),
                                event: Event::Capsule {
                                    capsule: Box::new(item.clone()),
                                    transition: e.transition.clone(),
                                    seq,
                                },
                            },
                        ),
                    ));
                }
            }
            for sc in said {
                if self.flow_ok(&c.principal, &sc.instrument) {
                    continue;
                }
                for (sub, inst, all) in &subs {
                    if inst.is_some_and(|i| i != &sc.instrument) {
                        continue;
                    }
                    if !all {
                        let a = Self::audience_of(doc, &c.principal);
                        let watched = a.watchlist.is_empty()
                            || a.watchlist.contains(&sc.instrument)
                            || a.positions.contains_key(&sc.instrument);
                        if !watched {
                            continue;
                        }
                    }
                    lines.push((
                        ci,
                        out(
                            &c.id,
                            ServerMsg::Notify {
                                sub: (*sub).to_owned(),
                                event: Event::ServerComment {
                                    comment: sc.clone(),
                                    seq,
                                },
                            },
                        ),
                    ));
                }
            }
        }
        lines
    }

    // -----------------------------------------------------------------------
    // Projection of documents and changes
    // -----------------------------------------------------------------------

    /// `doc` as `p`'s principal may receive it.
    ///
    /// `pre`: the reader's sentences by capsule ([`Session::reader_sentences`]), when
    /// already written.
    pub(crate) fn project_doc(
        &self,
        doc: &Document,
        p: &Proj,
        lang: Language,
        pre: Option<&std::collections::HashMap<String, String>>,
    ) -> Document {
        let mut d = doc.clone();
        if !p.principal.is_empty() {
            d.audiences.retain(|k, _| k == &p.principal);
        }
        d.facts.retain(|f| !p.hidden.contains(&f.id));
        for f in &mut d.facts {
            if p.redact.contains(&f.id) {
                f.evidence.clear();
                f.evidence_ref = None;
            }
        }
        d.findings.retain(|f| !p.hidden.contains(&f.id));
        for f in &mut d.findings {
            f.facts.retain(|x| !p.hidden.contains(x));
        }
        d.capsules.retain(|c| !p.hidden.contains(&c.id));
        let views_kept: Vec<qp_schema::View> = d
            .views
            .iter()
            .filter(|v| !p.mentions_hidden(*v))
            .cloned()
            .collect();
        d.views = views_kept;
        let view_ids: BTreeSet<String> = d.views.iter().map(|v| v.id.clone()).collect();
        for c in &mut d.capsules {
            if let Some(s) = p.stories.get(&c.id) {
                c.story = Some(s.clone());
            }
            if c.view.as_ref().is_some_and(|v| !view_ids.contains(v)) {
                c.view = None;
            }
        }
        // Sentences written for the reader where the stored one does not fit it.
        let mut memo = StudyMemo::default();
        let rewrites: Vec<(usize, String)> = d
            .capsules
            .iter()
            .enumerate()
            .filter(|(_, c)| p.rewrite.contains(&c.id))
            .filter_map(|(i, c)| match pre {
                Some(m) => m.get(&c.id).map(|t| (i, t.clone())),
                None => self
                    .reader_sentence_with(&d, c, Some(p), lang, &mut memo)
                    .map(|(s, _)| (i, s.text)),
            })
            .collect();
        // (R4/T6) The first finding of each id, looked up once (a linear search per
        // rewritten capsule was most of a public reader's first `hello`).
        let mut finding_at: std::collections::HashMap<&str, usize> =
            std::collections::HashMap::with_capacity(d.findings.len());
        for (k, f) in d.findings.iter().enumerate() {
            finding_at.entry(f.id.as_str()).or_insert(k);
        }
        let targets: Vec<(usize, Option<usize>)> = rewrites
            .iter()
            .map(|(i, _)| (*i, finding_at.get(d.capsules[*i].finding.as_str()).copied()))
            .collect();
        for ((i, k), (_, text)) in targets.into_iter().zip(rewrites) {
            d.capsules[i].headline.clone_from(&text);
            if let Some(k) = k {
                d.findings[k].claim = text;
            }
        }
        d.ledger.entries.retain(|e| {
            !p.hidden.contains(&e.finding)
                && e.capsule.as_ref().is_none_or(|c| !p.hidden.contains(c))
        });
        d.narratives.retain(|n| !p.mentions_hidden(n));
        d.proposals.retain(|x| !p.mentions_hidden(x));
        d
    }

    /// `changes` as `p`'s principal may receive them.
    pub(crate) fn project_changes(
        &self,
        changes: &[ChangeOp],
        p: &Proj,
        lang: Language,
    ) -> Vec<ChangeOp> {
        let mut outv = Vec::new();
        for c in changes {
            match c {
                ChangeOp::PutFact { fact, index } => {
                    if p.hidden.contains(&fact.id) {
                        continue;
                    }
                    let mut f = fact.clone();
                    if p.redact.contains(&f.id) {
                        f.evidence.clear();
                        f.evidence_ref = None;
                    }
                    outv.push(ChangeOp::PutFact {
                        fact: f,
                        index: *index,
                    });
                }
                ChangeOp::PutFinding { finding, index } => {
                    if p.hidden.contains(&finding.id) {
                        continue;
                    }
                    let mut f = finding.clone();
                    f.facts.retain(|x| !p.hidden.contains(x));
                    if let Some(cap) = self.doc_capsule_of(&f.id) {
                        if p.rewrite.contains(&cap.id) {
                            if let Some((s, _)) =
                                self.reader_sentence(&self.doc, cap, Some(p), lang)
                            {
                                f.claim = s.text;
                            }
                        }
                    }
                    outv.push(ChangeOp::PutFinding {
                        finding: f,
                        index: *index,
                    });
                }
                ChangeOp::PutCapsule { capsule, index } => {
                    if p.hidden.contains(&capsule.id) {
                        continue;
                    }
                    let mut cap = capsule.clone();
                    if let Some(st) = cap.story.as_ref() {
                        let ids = story_fact_ids(st);
                        let hidden: BTreeSet<String> = ids
                            .iter()
                            .filter(|i| p.hidden.contains(*i))
                            .cloned()
                            .collect();
                        if !hidden.is_empty() {
                            let facts: Vec<Fact> = ids
                                .iter()
                                .filter_map(|i| self.doc_fact(i).cloned())
                                .collect();
                            cap.story = Some(project_story(st, &hidden, &facts));
                        }
                    }
                    if p.rewrite.contains(&cap.id) {
                        if let Some((s, _)) = self.reader_sentence(&self.doc, &cap, Some(p), lang) {
                            cap.headline = s.text;
                        }
                    }
                    outv.push(ChangeOp::PutCapsule {
                        capsule: cap,
                        index: *index,
                    });
                }
                ChangeOp::SetCapsuleState { capsule, .. } if p.hidden.contains(capsule) => {}
                ChangeOp::PutAudience { principal, .. }
                    if !p.principal.is_empty() && principal != &p.principal => {}
                ChangeOp::Delete { id } if p.hidden.contains(id) => {}
                ChangeOp::PushLedger { entry }
                    if p.hidden.contains(&entry.finding)
                        || entry.capsule.as_ref().is_some_and(|c| p.hidden.contains(c)) => {}
                ChangeOp::PutView { .. }
                | ChangeOp::InsertLayer { .. }
                | ChangeOp::PutLayer { .. }
                | ChangeOp::SetEmphasis { .. }
                | ChangeOp::PutNarrative { .. }
                | ChangeOp::PutProposal { .. }
                    if p.mentions_hidden(c) => {}
                other => outv.push(other.clone()),
            }
        }
        outv
    }

    /// A log entry as `p`'s principal may receive it: changes and inverse projected,
    /// request ops and a summary that name a hidden item removed.
    pub(crate) fn project_entry(&self, e: &LogEntry, p: &Proj, lang: Language) -> LogEntry {
        let mut x = e.clone();
        x.changes = self.project_changes(&e.changes, p, lang);
        x.inverse = self.project_changes(&e.inverse, p, lang);
        x.request.retain(|op| !p.mentions_hidden(op));
        // Another person's capsule preferences (positions, watch list, level) never go.
        let own = |w: &Option<String>| {
            p.principal.is_empty() || w.as_deref().unwrap_or(e.author.as_str()) == p.principal
        };
        let before = x.request.len();
        x.request.retain(|op| match op {
            qp_schema::protocol::Op::SetAudience { principal, .. }
            | qp_schema::protocol::Op::SetSensitivity { principal, .. } => own(principal),
            _ => true,
        });
        let foreign = !p.principal.is_empty()
            && e.changes.iter().any(
                |c| matches!(c, ChangeOp::PutAudience { principal, .. } if principal != &p.principal),
            );
        if foreign || x.request.len() != before {
            x.summary = "set_audience".to_owned();
        }
        if p.hidden
            .iter()
            .any(|h| h.len() <= x.summary.len() && x.summary.contains(h.as_str()))
        {
            x.summary = "…".to_owned();
        }
        if let Some(c) = x.cause.as_ref() {
            if p.hidden.iter().any(|h| c.contains(h.as_str())) {
                x.cause = None;
            }
        }
        x
    }

    /// Runs `f` with the document (and the reading document) as `principal` may receive
    /// them (R3, §20): every read path, selector and error suggestion then only sees
    /// what the principal may see. Nothing written inside `f` may touch the document.
    ///
    /// (R3/perf) What it gives a reader (headlines, projected documents) is kept per
    /// reader and language ([`crate::capidx::ReaderView`]) and given again while the log
    /// position, the projection and the reading document's bars are the same.
    pub(crate) fn as_reader<R>(
        &mut self,
        principal: &str,
        lang: Language,
        f: impl FnOnce(&mut Self) -> R,
    ) -> R {
        // (R3/K1) A public-view reader reads the public world. Its headlines are brought
        // up to date here, when a reader needs them (not after every engine entry).
        let public = self.public_view(principal);
        if public != self.in_public && self.other.is_some() {
            return self.in_world(public, |s| s.as_reader(principal, lang, f));
        }
        if self.in_public && self.proj.is_none() && !self.headlines_current() {
            self.refresh_headlines();
        }
        let p = self.reader_proj(principal);
        let lines = self.runner.is_some() || lang == Language::En;
        if p.is_none() && !lines {
            return f(self);
        }
        let nested = self.proj.is_some();
        let cached = if nested {
            None
        } else {
            self.views
                .iter()
                .position(|v| v.principal == principal && v.lang == lang)
                .map(|i| self.views.remove(i))
                .filter(|v| self.view_valid(v, p.as_ref(), lines))
        };
        let mut view = if let Some(v) = cached {
            v
        } else {
            let (heads, pre) = if lines {
                self.reader_sentences(p.as_deref(), lang, !nested)
            } else {
                (BTreeMap::new(), None)
            };
            let (doc, rdp) = match p.as_deref() {
                Some(p) => (
                    Some(self.project_doc(&self.doc, p, lang, pre.as_ref())),
                    self.rd
                        .as_ref()
                        .map(|r| self.project_doc(r, p, lang, pre.as_ref())),
                ),
                None => (None, None),
            };
            crate::capidx::ReaderView {
                principal: principal.to_owned(),
                lang,
                log_len: self.log.len(),
                lines,
                proj: p.clone(),
                rd: if nested {
                    None
                } else {
                    crate::capidx::rd_snap(self.rd.as_ref())
                },
                heads: Some(heads),
                doc,
                rdp,
            }
        };
        let heads = view.heads.take().unwrap_or_default();
        let saved_heads = std::mem::replace(&mut self.headlines, heads);
        let r = if let Some(p) = p {
            let doc = view.doc.take().expect("a projection has its document");
            let rd = view.rdp.take();
            let mut real_doc = std::mem::replace(&mut self.doc, doc);
            let real_rd = std::mem::replace(&mut self.rd, rd);
            let vr = self.view_reads.take();
            let li = self.live_index.take();
            let saved = self.proj.replace(p);
            // The real facts, moved (not copied) for the sentences.
            let real_facts =
                std::mem::replace(&mut self.real_facts, std::mem::take(&mut real_doc.facts));
            let r = f(self);
            real_doc.facts = std::mem::replace(&mut self.real_facts, real_facts);
            self.proj = saved;
            view.doc = Some(std::mem::replace(&mut self.doc, real_doc));
            view.rdp = std::mem::replace(&mut self.rd, real_rd);
            self.view_reads = vr;
            self.live_index = li;
            r
        } else {
            f(self)
        };
        view.heads = Some(std::mem::replace(&mut self.headlines, saved_heads));
        if !nested {
            self.views.push(view);
            if self.views.len() > crate::capidx::VIEWS {
                self.views.remove(0);
            }
        }
        r
    }

    /// Whether `v` is what [`Session::as_reader`] would make now for projection `p`.
    fn view_valid(&self, v: &crate::capidx::ReaderView, p: Option<&Rc<Proj>>, lines: bool) -> bool {
        v.log_len == self.log.len()
            && v.lines == lines
            && v.heads.is_some()
            && match (v.proj.as_ref(), p) {
                (None, None) => true,
                (Some(a), Some(b)) => Rc::ptr_eq(a, b) || **a == **b,
                _ => false,
            }
            && (p.is_none() || (v.doc.is_some() && v.rdp.is_some() == self.rd.is_some()))
            && crate::capidx::rd_same(&v.rd, self.rd.as_ref())
    }

    /// Finding id → the realizer's sentence of every capsule with a story, for a reader
    /// with projection `p` in `lang` (hidden capsules left out).
    /// [`Session::story_headlines`], and the same sentences by capsule. With `reuse`, a
    /// capsule the projection does not touch (its story, facts and redaction are the
    /// entitled reader's) takes the authority's TR headline when that is current
    /// ([`Session::headlines_current`]).
    pub(crate) fn reader_sentences(
        &self,
        p: Option<&Proj>,
        lang: Language,
        reuse: bool,
    ) -> (
        BTreeMap<String, String>,
        Option<std::collections::HashMap<String, String>>,
    ) {
        let base =
            (reuse && lang == Language::Tr && self.headlines_current()).then_some(&self.headlines);
        let mut memo = StudyMemo::default();
        let mut heads = BTreeMap::new();
        let mut by = std::collections::HashMap::new();
        let mut dup = false;
        for c in &self.doc.capsules {
            let Some(story) = c.story.as_ref() else {
                continue;
            };
            if p.is_some_and(|p| p.hidden.contains(&c.id)) {
                continue;
            }
            let touched = p.is_some_and(|p| {
                p.stories.contains_key(&c.id)
                    || story_fact_ids(story)
                        .iter()
                        .any(|i| p.hidden.contains(i) || p.redact.contains(i))
            });
            let text = match base {
                Some(b) if !touched => b.get(&c.finding).cloned(),
                _ => self.reader_text_with(&self.doc, c, p, lang, &mut memo),
            };
            if let Some(t) = text {
                dup |= by.insert(c.id.clone(), t.clone()).is_some();
                heads.insert(c.finding.clone(), t);
            }
        }
        // Two capsules with one id: sentences by capsule would not say whose.
        (heads, (!dup).then_some(by))
    }

    /// Whether [`Session::headlines`] are what [`Session::story_headlines`] gives now
    /// (TR, entitled): written by the last refresh, with no log entry and no change of
    /// the bars they read since, and one capsule per finding.
    fn headlines_current(&self) -> bool {
        let st = &self.head_state;
        self.runner.is_some()
            && st.built
            && st.log_len == self.log.len()
            && st.snaps.iter().all(|(s, v)| self.bars_snaps(s) == *v)
    }

    pub(crate) fn story_headlines(
        &self,
        p: Option<&Proj>,
        lang: Language,
    ) -> BTreeMap<String, String> {
        let mut memo = StudyMemo::default();
        self.doc
            .capsules
            .iter()
            .filter(|c| c.story.is_some() && p.is_none_or(|p| !p.hidden.contains(&c.id)))
            .filter_map(|c| {
                self.reader_text_with(&self.doc, c, p, lang, &mut memo)
                    .map(|t| (c.finding.clone(), t))
            })
            .collect()
    }

    /// Refreshes the authority's own headlines (TR, entitled) after the engine wrote.
    ///
    /// (R3/perf) Only the capsules whose sentence input may have changed since the last
    /// refresh are rewritten ([`crate::capidx::HeadState`]): those the log entries since
    /// then put or whose facts they put, and those of a series whose bars (as the
    /// sentences read them, bit for bit) changed. A series put, anything deleted, a
    /// capsule put at an index or two capsules with a story sharing a finding: all of
    /// them, as [`Session::story_headlines`] does.
    pub(crate) fn refresh_headlines(&mut self) {
        if self.runner.is_none() {
            return;
        }
        let mut st = std::mem::take(&mut self.head_state);
        let full = !st.built
            || st.log_len > self.log.len()
            || self.log[st.log_len..].iter().any(|e| {
                e.changes.iter().any(|c| {
                    matches!(
                        c,
                        ChangeOp::PutSeries { .. }
                            | ChangeOp::Delete { .. }
                            | ChangeOp::PutCapsule { index: Some(_), .. }
                    )
                })
            })
            || !self.refresh_some(&mut st);
        if full {
            self.headlines = self.story_headlines(None, Language::Tr);
            st = crate::capidx::HeadState::default();
            let mut ok = true;
            for c in &self.doc.capsules {
                let Some(s) = c.story.as_ref() else { continue };
                ok &= st.register(&c.id, &s.series, &c.finding, story_fact_ids(s));
            }
            if ok {
                let series: Vec<String> = st.by_series.keys().cloned().collect();
                for s in series {
                    let v = self.bars_snaps(&s);
                    st.snaps.insert(s, v);
                }
                st.built = true;
            }
        }
        st.log_len = self.log.len();
        self.head_state = st;
    }

    /// The bars the sentences of series `s` read: in the reading document and in the
    /// document.
    fn bars_snaps(&self, s: &str) -> (crate::capidx::BarsSnap, crate::capidx::BarsSnap) {
        (
            crate::capidx::snap(bar_series(rd!(self), s).and_then(qp_schema::Series::bars)),
            crate::capidx::snap(bar_series(&self.doc, s).and_then(qp_schema::Series::bars)),
        )
    }

    /// The incremental step of [`Session::refresh_headlines`]; `false`: not possible
    /// (rewrite all).
    fn refresh_some(&mut self, st: &mut crate::capidx::HeadState) -> bool {
        let mut dirty: BTreeSet<String> = BTreeSet::new();
        for e in &self.log[st.log_len..] {
            for c in &e.changes {
                match c {
                    ChangeOp::PutCapsule { capsule, .. } => {
                        dirty.insert(capsule.id.clone());
                    }
                    ChangeOp::PutFact { fact, .. } => {
                        if let Some(s) = st.by_fact.get(&fact.id) {
                            dirty.extend(s.iter().cloned());
                        }
                    }
                    _ => {}
                }
            }
        }
        let series: Vec<String> = st.snaps.keys().cloned().collect();
        for s in series {
            let now = self.bars_snaps(&s);
            if st.snaps.get(&s) != Some(&now) {
                if let Some(c) = st.by_series.get(&s) {
                    dirty.extend(c.iter().cloned());
                }
                st.snaps.insert(s, now);
            }
        }
        let mut heads = std::mem::take(&mut self.headlines);
        let mut memo = StudyMemo::default();
        let mut ok = true;
        for id in &dirty {
            if let Some(f) = st.unregister(id) {
                heads.remove(&f);
            }
            let Some(c) = self.doc_capsule(id) else {
                continue;
            };
            let Some(s) = c.story.as_ref() else { continue };
            if !st.register(&c.id, &s.series, &c.finding, story_fact_ids(s)) {
                ok = false;
                break;
            }
            if !st.snaps.contains_key(&s.series) {
                let v = self.bars_snaps(&s.series);
                st.snaps.insert(s.series.clone(), v);
            }
            if let Some(t) = self.reader_text_with(&self.doc, c, None, Language::Tr, &mut memo) {
                heads.insert(c.finding.clone(), t);
            }
        }
        self.headlines = heads;
        ok
    }

    // -----------------------------------------------------------------------
    // Disputes and settled data (§19.9 + §20)
    // -----------------------------------------------------------------------

    /// (R3/K1) Whether capsule `cap` is the running engine's: a story on a watched
    /// series (by the capsule index).
    pub(crate) fn runs_capsule(&self, cap: &str) -> bool {
        let Some(r) = self.runner.as_ref() else {
            return false;
        };
        self.doc_capsule(cap)
            .and_then(|c| c.story.as_ref())
            .is_some_and(|s| r.watched_of(&s.series).is_some())
    }

    /// `true` when `capsule` is a story of a watched series.
    pub(crate) fn engine_capsule(&self, capsule: &str) -> Option<String> {
        let r = self.runner.as_ref()?;
        let c = self.doc.capsules.iter().find(|c| c.id == capsule)?;
        let s = c.story.as_ref()?;
        r.watched_of(&s.series).map(|w| w.series.clone())
    }

    /// A `dispute` on an engine capsule: the claim is "the story's facts hold on the
    /// settled bars". Settled bars from the anchor to `t` are asked; the answer re-runs
    /// the detectors on the corrected data (see [`Session::capsule_settled`]).
    pub(crate) fn capsule_dispute(
        &mut self,
        ci: usize,
        id: &str,
        capsule: &str,
        t: i64,
        digest: Option<String>,
    ) -> Vec<OutboundLine> {
        let conn = self.conns[ci].id.clone();
        let principal = self.conns[ci].principal.clone();
        // (R3/K1) The capsule as the disputer's world has it.
        let public = self.public_view(&principal);
        let (finding, visible, series, story) = self.in_world(public, |s| {
            let c = s.doc.capsules.iter().find(|c| c.id == capsule);
            let finding = c.map(|c| c.finding.clone()).unwrap_or_default();
            let story = c.and_then(|c| c.story.clone());
            let visible = s
                .proj_for(&principal)
                .is_none_or(|p| !p.hidden.contains(capsule));
            (finding, visible, s.engine_capsule(capsule), story)
        });
        let claim = ClaimRef::Capsule {
            capsule: capsule.to_owned(),
            finding,
        };
        let reply = |s: &Self, v: DisputeVerdict, ev: Option<ClaimEvidence>| {
            out(
                &conn,
                ServerMsg::Result {
                    id: id.to_owned(),
                    seq: s.seq_to(&principal, s.seq()),
                    now: s.clock(),
                    data: QueryResult::Dispute {
                        verdict: v,
                        evidence: ev,
                    },
                    resolved: BTreeMap::new(),
                },
            )
        };
        let (Some(series), Some(story), true) = (series, story, visible) else {
            return vec![reply(self, DisputeVerdict::Unknown, None)];
        };
        if !self.bars_ok(&principal, &series) {
            return vec![reply(self, DisputeVerdict::Unknown, None)];
        }
        let seen = (conn.clone(), claim.clone(), t, digest.unwrap_or_default());
        if self.settle.seen_contains(&seen) {
            return vec![reply(self, DisputeVerdict::Duplicate, None)];
        }
        if story.stage == Stage::Retracted || t < story.history.first().map_or(i64::MAX, |h| h.t) {
            return vec![reply(self, DisputeVerdict::Unknown, None)];
        }
        self.settle
            .seen_insert(seen.clone(), DisputeVerdict::Pending);
        if let Some(r) = self.runner.as_mut() {
            r.disputes.push(CapDispute {
                conn: conn.clone(),
                capsule: capsule.to_owned(),
                series: series.clone(),
                t,
                seen: seen.clone(),
                public,
            });
        }
        let feed = self.plane.series.get(&series).map(|b| b.binding.feed_key());
        let from_t = self
            .plane
            .bars(&series)
            .and_then(<[Bar]>::first)
            .map_or(t, |b| b.t);
        if let Some(feed) = feed {
            self.settle.push_request(crate::dispute::SettleRequest {
                series: series.clone(),
                feed,
                from_t,
                to_t: t,
            });
        }
        let lines = self.serve_test_source();
        let verdict = self
            .settle
            .seen_get(&seen)
            .unwrap_or(DisputeVerdict::Pending);
        let ev = (verdict == DisputeVerdict::Rejected).then(|| self.capsule_evidence(&series, t));
        let mut v = vec![reply(self, verdict, ev)];
        v.extend(lines);
        v
    }

    fn capsule_evidence(&self, series: &str, t: i64) -> ClaimEvidence {
        let doc = rd!(self);
        ClaimEvidence {
            series: series.to_owned(),
            t,
            values: Vec::new(),
            maturity: Maturity::Final,
            clean: true,
            anchor_t: crate::claims::anchor_of(doc, series),
            feeds: find_series(doc, series)
                .and_then(qp_schema::Series::binding)
                .map(|b| vec![b.feed_key()])
                .unwrap_or_default(),
        }
    }

    /// After a settled answer on `series` (`lo..=hi`; `corrected`: bars changed): the
    /// detectors re-run on the authority's (corrected) final bars; every story with a fact
    /// the corrected data no longer gives (or gives with other evidence values) is
    /// **retracted** (K37: `data_corrected`, or `dispute_upheld` when disputed; never in
    /// the ledger). The detectors continue from the corrected data. Facts the corrected
    /// data would newly give are not asserted (history is not rewritten). Open capsule
    /// disputes the answer covers are decided. Returns the notifications.
    pub(crate) fn capsule_settled(
        &mut self,
        series: &str,
        lo: i64,
        hi: i64,
        corrected: bool,
    ) -> Vec<OutboundLine> {
        let Some(mut runner) = self.runner.take() else {
            return Vec::new();
        };
        let Some(wi) = runner.watched.iter().position(|w| w.series == series) else {
            self.runner = Some(runner);
            return Vec::new();
        };
        let mut lines = Vec::new();
        let mut retract: Vec<(String, u64, i64, &'static str)> = Vec::new();
        let mut retract_pub: Vec<(String, u64, i64, &'static str)> = Vec::new();
        if corrected {
            let rd = rd!(self);
            let bars: Vec<Bar> = find_series(rd, series)
                .and_then(qp_schema::Series::bars)
                .map(<[Bar]>::to_vec)
                .unwrap_or_default();
            let w = &mut runner.watched[wi];
            let done = w.done.min(bars.len());
            let flow = w
                .flow
                .as_ref()
                .map(|(seed, _)| flow_points(&bars[..done], *seed));
            let mut dets = qp_facts::default_detectors();
            let mut again: BTreeMap<String, BTreeMap<String, u64>> = BTreeMap::new();
            for (i, b) in bars[..done].iter().enumerate() {
                let fp = flow.as_ref().and_then(|f| f.get(i));
                for det in &mut dets {
                    for d in det.push(b, fp) {
                        let f = qp_facts::to_fact(
                            det.as_ref(),
                            &d,
                            &w.symbol,
                            &w.series,
                            i as u64,
                            b.t,
                            qp_capsule::pipeline::ENGINE_VERSION,
                        );
                        let bits = f
                            .evidence
                            .iter()
                            .map(|(k, v)| (k.clone(), v.to_bits()))
                            .collect();
                        again.insert(f.id, bits);
                    }
                }
            }
            w.dets = dets;
            let broken = |engine: &Engine, public: bool| {
                broken_stories(
                    engine,
                    series,
                    &again,
                    &bars,
                    done,
                    runner.disputes.iter().filter(|d| !public || d.public),
                )
            };
            retract = broken(&runner.engine, false);
            if let Some(pe) = runner.pub_engine.as_ref() {
                retract_pub = broken(pe, true);
            }
        }
        let mut events = Vec::new();
        for (id, bar, t, why) in retract {
            if let Some(ev) = runner.engine.retract(&id, bar, t, why) {
                events.push(ev);
            }
        }
        // (R3/K1) The public-only engine's stories, retracted on the same corrected data.
        let mut events_pub = Vec::new();
        if let Some(pe) = runner.pub_engine.as_mut() {
            for (id, bar, t, why) in retract_pub {
                if let Some(ev) = pe.retract(&id, bar, t, why) {
                    events_pub.push(ev);
                }
            }
        }
        // Decide the open disputes the answer covers.
        let open = std::mem::take(&mut runner.disputes);
        for d in open {
            if d.series != series || d.t < lo || d.t > hi {
                runner.disputes.push(d);
                continue;
            }
            // (R3/K1) A public-view reader's dispute is decided on the public-only engine.
            let (evs, engine) = match (d.public, runner.pub_engine.as_ref()) {
                (true, Some(pe)) => (&events_pub, pe),
                _ => (&events, &runner.engine),
            };
            let retracted = evs.iter().any(|e| e.capsule == d.capsule)
                || engine
                    .closed_stories()
                    .iter()
                    .any(|s| s.id == d.capsule && s.story.stage == Stage::Retracted);
            let verdict = if retracted {
                DisputeVerdict::Upheld
            } else {
                DisputeVerdict::Rejected
            };
            self.settle.seen_insert(d.seen.clone(), verdict);
            let _ = &d.conn;
        }
        // (R3/K1) With a public world, whether an entry is written may not depend on the
        // licensed data (which stories of the authority's engine broke): one entry per
        // corrected answer, each world with its own retractions.
        let public_world = runner.pub_engine.is_some();
        if events.is_empty() && !(public_world && corrected) {
            self.runner = Some(runner);
            return lines;
        }
        let summary = |evs: &[qp_capsule::StoryEvent]| {
            if evs.is_empty() {
                format!("settled {series}")
            } else {
                let ids: Vec<&str> = evs.iter().map(|e| e.capsule.as_str()).collect();
                format!("retract {}", ids.join(", "))
            }
        };
        let changes = self.capsule_changes(&runner.engine, &events, &BTreeMap::new(), 0);
        if let Some(pe) = runner.pub_engine.as_ref() {
            let changes = self.in_world(true, |s| {
                s.capsule_changes(pe, &events_pub, &BTreeMap::new(), 0)
            });
            self.pending_pub = Some(crate::public::PubNext::Entry(crate::public::PubPending {
                changes,
                summary: Some(summary(&events_pub)),
                evidence: None,
                undoes: None,
                request: None,
            }));
        }
        if let Some(seq) = self.engine_entry_with(changes, summary(&events), None) {
            self.runner = Some(runner);
            self.refresh_headlines();
            lines.extend(self.entry_events(seq));
            lines.extend(self.capsule_topic_lines(seq, &events, &events_pub, &[]));
            return lines;
        }
        self.pending_pub = None;
        self.runner = Some(runner);
        lines
    }
}

/// The stories of `engine` on `series` (not retracted) with a fact the corrected data no
/// longer gives with the same evidence (`again`: fact id → evidence bits over the first
/// `done` corrected `bars`): (capsule, bar, t, reason) of their retraction, by id.
fn broken_stories<'a>(
    engine: &Engine,
    series: &str,
    again: &BTreeMap<String, BTreeMap<String, u64>>,
    bars: &[Bar],
    done: usize,
    disputes: impl Iterator<Item = &'a CapDispute> + Clone,
) -> Vec<(String, u64, i64, &'static str)> {
    let mut out = Vec::new();
    let mut stories: Vec<&qp_capsule::OpenStory> = engine
        .open_stories()
        .iter()
        .chain(engine.closed_stories())
        .filter(|s| s.story.series == series && s.story.stage != Stage::Retracted)
        .collect();
    stories.sort_by(|a, b| a.id.cmp(&b.id));
    for s in stories {
        let broken = s.facts.iter().any(|f| {
            let bits: BTreeMap<String, u64> = f
                .evidence
                .iter()
                .map(|(k, v)| (k.clone(), v.to_bits()))
                .collect();
            again.get(&f.id) != Some(&bits)
        });
        if !broken {
            continue;
        }
        let disputed = disputes
            .clone()
            .any(|d| d.capsule == s.id && d.series == series);
        let last = s
            .story
            .history
            .last()
            .map_or((0, i64::MIN), |l| (l.bar, l.t));
        let at_bar = u64::try_from(done.saturating_sub(1))
            .unwrap_or(0)
            .max(last.0);
        let at_t = bars
            .get(usize::try_from(at_bar).unwrap_or(0))
            .map_or(last.1, |b| b.t)
            .max(last.1);
        out.push((
            s.id.clone(),
            at_bar,
            at_t,
            if disputed {
                "dispute_upheld"
            } else {
                "data_corrected"
            },
        ));
    }
    out
}

/// Ledger rows of `doc` with a key (R3), the retracted capsules left out:
/// `(key, kind, recorded_at, realized)`.
pub(crate) fn ledger_rows(
    doc: &Document,
) -> Vec<(String, qp_schema::capsule::MeaningKind, i64, bool)> {
    let retracted: BTreeSet<&str> = doc
        .capsules
        .iter()
        .filter(|c| {
            c.story
                .as_ref()
                .is_some_and(|s| s.stage == Stage::Retracted)
        })
        .map(|c| c.id.as_str())
        .collect();
    doc.ledger
        .entries
        .iter()
        .filter(|e| e.capsule.as_deref().is_none_or(|c| !retracted.contains(c)))
        .filter_map(|e| {
            let k = e.key.clone()?;
            let kind = qp_schema::capsule::MeaningKind::ALL
                .into_iter()
                .find(|m| k.starts_with(&format!("{}/", m.name())))?;
            Some((
                k,
                kind,
                e.recorded_at,
                e.outcome == qp_schema::Outcome::Confirmed,
            ))
        })
        .collect()
}

/// Candidates on the open (last) bar of `bars` (K28, A16): a fresh detector set and
/// engine run over the final bars (all but the last), then `Engine::candidates` with the
/// last bar's provisional facts. For a replica or any local reader of an open bar; the
/// authority never sends them, never logs them and never writes them to the ledger.
/// Public detectors only (a device holds no licensed flow).
#[must_use]
pub fn candidates(symbol: &str, series: &str, bars: &[Bar]) -> Vec<qp_capsule::CandidateStory> {
    let Some(last) = bars.len().checked_sub(1) else {
        return Vec::new();
    };
    let mut engine = Engine::new(EngineConfig::r3());
    let mut dets = qp_facts::default_detectors();
    let mut provisional = Vec::new();
    for (i, b) in bars.iter().enumerate() {
        let mut facts = Vec::new();
        for det in &mut dets {
            for d in det.push(b, None) {
                facts.push(qp_facts::to_fact(
                    det.as_ref(),
                    &d,
                    symbol,
                    series,
                    i as u64,
                    b.t,
                    qp_capsule::pipeline::ENGINE_VERSION,
                ));
            }
        }
        if i == last {
            provisional = facts;
        } else {
            engine.on_bar(symbol, series, i as u64, &bars[..=i], &facts);
        }
    }
    engine.candidates(symbol, series, last as u64, bars, &provisional)
}
