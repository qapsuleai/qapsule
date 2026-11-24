//! The capsule engine in the authority (R3, `docs/PROTOCOL.md` §20; `docs/CAPSULE.md` §9).
//!
//! # Runner
//!
//! With `init.capsules` the authority watches the named bar series (bound or inline). At
//! every settle point it looks at each watched series: every bar that became **final**
//! since the last look (a later bar exists, or the window is closed: RP2 maturity) goes,
//! in order, through the detectors (`qp_facts::default_detectors`, streaming `push`) and
//! the capsule engine (`qp_capsule::Engine` with `EngineConfig::r3()`); (RP4) or, when
//! the host names another record (`init.capsules.engine`, `docs/CAPSULE.md` §9.9), that
//! record's detector set and engine (`qp_facts::detectors_named`,
//! `EngineConfig::named`). The licensed flow
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

mod headsource;
mod project;
mod queries;
mod reader;
mod run;
mod settled;

#[cfg(test)]
mod tests;

pub(crate) mod claims;
pub(crate) mod comments;
pub(crate) mod dispute;
pub(crate) mod public;
pub(crate) mod story;
pub(crate) mod watch;

use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use qp_capsule::sentence::StudyMemo;
use qp_capsule::{Engine, EngineConfig, FlowObservation, Lang, TrustPath};
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

use qp_plane::error::{ptr, PResult, E};
use qp_plane::proj::Proj;
use qp_plane::sentences::{sentence_pair_memo, story_fact_ids, story_validity};
use qp_plane::values::{bar_series, find_series};

use crate::session::{out, Session};

/// Engine component name in provenance.
const ENGINE_NAME: &str = "qp-capsule";

/// The invalidation of an engine finding: decided by the outcome rule, never by a
/// document condition (a text condition is never evaluated).
const OUTCOME_TEXT: &str = "outcome rule (qp-capsule barrier, docs/CAPSULE.md §9.3)";

/// (RP4) [`OUTCOME_TEXT`] of the `rp4` record.
const OUTCOME_TEXT_RP4: &str =
    "outcome rule (qp-capsule rp4: high/low touch intraday, barrier daily; docs/CAPSULE.md §9.9)";

/// (RP4) A fresh detector set of a known record (`r3` otherwise).
fn detectors_of(engine: &str) -> Vec<Box<dyn Detector>> {
    qp_facts::detectors_named(engine).unwrap_or_else(qp_facts::default_detectors)
}

/// (RP4) A fresh engine of a known record (`r3` otherwise).
fn engine_config_of(engine: &str) -> EngineConfig {
    EngineConfig::named(engine).unwrap_or_else(EngineConfig::r3)
}

/// (RP4) The named constant record the authority ran for `doc`'s engine stories, read
/// from their resolution rule (`qp_capsule::engine_of_rule`): `rp4` when any story was
/// resolved by the `rp4` rule, else `r3`. For a reader that runs the detectors itself
/// (a replica's candidates, a viewer's check).
#[must_use]
pub fn engine_of_doc(doc: &Document) -> &'static str {
    doc.capsules
        .iter()
        .filter_map(|c| c.story.as_ref()?.resolution.as_ref())
        .map(|r| qp_capsule::engine_of_rule(&r.rule))
        .find(|e| *e != qp_capsule::DEFAULT_ENGINE)
        .unwrap_or(qp_capsule::DEFAULT_ENGINE)
}

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
    /// (RP4) The named constant record (`init.capsules.engine`, default `r3`).
    engine_name: &'static str,
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
        // (RP4) Validated at session start; an unknown name falls back to the default.
        let engine_name = qp_capsule::ENGINES
            .iter()
            .copied()
            .find(|n| init.engine.as_deref() == Some(*n))
            .unwrap_or(qp_capsule::DEFAULT_ENGINE);
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
                dets: detectors_of(engine_name),
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
            engine: Engine::new(engine_config_of(engine_name)),
            pub_engine: flow.then(|| Engine::new(engine_config_of(engine_name))),
            watched,
            comments: Vec::new(),
            disputes: Vec::new(),
            pos_gen: 0,
            engine_name,
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
pub(crate) fn final_count(doc: &Document, bs: &str, n: usize) -> usize {
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

    /// (RP4/T6) The bar series "now" is read on for a story on `series` (as
    /// [`Session::now_bar`] chooses it: the reading document's when it has bars, else
    /// `doc`'s).
    fn item_series<'d>(&'d self, doc: &'d Document, series: &str) -> Option<&'d qp_schema::Series> {
        let has = |s: &&qp_schema::Series| s.bars().is_some_and(|b| !b.is_empty());
        bar_series(rd!(self), series)
            .filter(has)
            .or_else(|| bar_series(doc, series).filter(has))
    }

    /// Whether a story reaches `principal`'s audience: watch list (empty = every
    /// instrument), positions and intents; then the sensitivity filter with the
    /// position/intent override (`qp_capsule::admits`).
    ///
    /// (K42/A22) `stage` is the transition judged: the story's current stage for the
    /// `capsules` query, the notified transition's stage for the topic. Only a negative
    /// transition of a position/intent story passes whatever the level.
    ///
    /// (R5/M18, K53) With the host's coin-flip rule, a marked story passes only at the
    /// `all` level ([`Session::feed_place`], the same rule the story list states).
    fn admits(&self, doc: &Document, principal: &str, story: &Story, stage: Stage) -> bool {
        self.feed_place(doc, principal, story, stage).0
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
    candidates_with(qp_capsule::DEFAULT_ENGINE, symbol, series, bars)
}

/// (RP4) [`candidates`] with the named constant record `engine` (`r3`, `rp4`; an unknown
/// name runs `r3`).
#[must_use]
pub fn candidates_with(
    engine: &str,
    symbol: &str,
    series: &str,
    bars: &[Bar],
) -> Vec<qp_capsule::CandidateStory> {
    let Some(last) = bars.len().checked_sub(1) else {
        return Vec::new();
    };
    let mut dets = detectors_of(engine);
    let mut engine = Engine::new(engine_config_of(engine));
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
