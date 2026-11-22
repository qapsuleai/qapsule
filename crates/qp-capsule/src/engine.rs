//! The capsule engine: final facts → story transitions → ledger (`docs/CAPSULE.md` §9.3,
//! §9.4). Deterministic; time is the bar (`Bar::t`), never a clock.
//!
//! # One bar ([`Engine::on_bar`])
//!
//! For the open stories of `(instrument, series)`, in this exact order:
//!
//! 1. the outcome rule judges every open story with `bars[..=now]` → `realized` /
//!    `invalidated` (closes, ledger row);
//! 2. stories whose horizon is reached (`now >= born_bar + horizon_bars`) → `expired`
//!    (closes, ledger miss, reason `horizon`);
//! 3. this bar's facts, sorted by `(detector, code, id)`, go through the theme rule:
//!    `New` → `born` (`new_story`); `Join` → `strengthened` (`same_family`) /
//!    `confirmed` (`new_family`) / `weakened` (`opposite_fact`, or `state_ended` for an
//!    `exit` of a state that an `enter` fact of the same detector opened in the story);
//!    `Ignore` → nothing;
//! 4. the ranker recomputes `(importance, tier)` for every story that transitioned on
//!    this bar (including the ones that closed), with a [`RankCtx`] that only counts
//!    what is known strictly before this bar's time. The bar's event snapshots of that
//!    story carry the bar-level rank.
//!
//! Interpretations (documented in `rounds/R3/experiments/T45.md`):
//!
//! - The theme rule sees the open stories of the same instrument **and series** (bar
//!   indices are per series), oldest first, including stories born earlier on this bar.
//! - `New` with a capsule id that already exists (same instrument, theme and bar) joins
//!   that story: one instrument + one theme = one story (K28).
//! - "Opposite" means `up` vs `down`; a `neutral` fact or story never opposes.
//! - An `exit` fact that does not end a state of the story neither supports nor opposes
//!   it: no transition.
//! - Strength: the maximum strength of the supporting facts (weakening keeps it).
//! - Facts of another instrument, series or bar, and facts without `meaning`, are
//!   skipped.
//!
//! # Ledger and retraction
//!
//! Every close except retraction appends one [`LedgerRecord`] stamped with the close
//! bar's time. [`Engine::ledger_stat`]`(key, before_t)` counts the rows closed strictly
//! before `before_t`. A retraction (K37) is never a ledger row. Retracting an
//! already-closed story does not delete its row: the row is marked `retracted_at = t`
//! and stops counting **only for questions asked after `t`**
//! (`before_t > retracted_at`). So the ledger is point-in-time: `ledger_stat(key, x)`
//! for `x <= t` is unchanged and the `ledger_at_birth` of stories born before the
//! retraction stays what it was (no look-ahead in either direction).

use std::collections::{BTreeMap, BTreeSet};

use qp_schema::capsule::{
    Direction, Edge, GlyphPhase, GlyphSpec, LedgerStat, MeaningKind, Resolution, Stage, Story,
    Transition, Trust, Validity,
};
use qp_schema::{Bar, CapsuleState, Fact, LedgerEntry, Outcome};
use serde::{Deserialize, Serialize};

use crate::{OpenStory, OutcomeRule, RankCtx, Ranker, ThemeDecision, ThemeRule, Verdict};

mod glyph;
mod index;
mod run;

pub use glyph::{glyph_spec, glyph_spec_at, FRESHNESS_HALF_BARS};
use index::{stat, LedgerIndex};
use run::BarIn;

/// The three pluggable rules the engine runs with.
pub struct EngineConfig {
    /// Theme identity rule (M4).
    pub theme: Box<dyn ThemeRule>,
    /// Outcome rule (M5).
    pub outcome: Box<dyn OutcomeRule>,
    /// Ranking rule (M3).
    pub ranker: Box<dyn Ranker>,
}

impl EngineConfig {
    /// The R3 choices: theme [`crate::theme::KindDir`] (M4a), outcome
    /// [`crate::outcome::Barrier`] (M5a), ranker [`crate::rank::Confirm`] (M3c), all with
    /// their measured defaults (`rounds/R3/experiments/M3–M5/RESULTS.md`).
    #[must_use]
    pub fn r3() -> Self {
        Self {
            theme: Box::new(crate::theme::KindDir::default()),
            outcome: Box::new(crate::outcome::Barrier::default()),
            ranker: Box::new(crate::rank::Confirm::default()),
        }
    }

    /// (RP4) The `rp4` record: the R3 choices with the outcome rule
    /// [`crate::outcome::Rp4`] (high / low touch on intraday bars, the R3 barrier on
    /// daily and longer bars; `rounds/RP4/experiments/M25/RESULTS.md`). Its detector set
    /// is `qp_facts::rp4_detectors` (same name, [`qp_facts::detectors_named`]).
    #[must_use]
    pub fn rp4() -> Self {
        Self {
            outcome: Box::new(crate::outcome::Rp4::default()),
            ..Self::r3()
        }
    }

    /// (RP4, `docs/CAPSULE.md` §9.9) A **named capsule constant record** by name
    /// ([`ENGINES`]): `r3` ([`EngineConfig::r3`]) or `rp4` ([`EngineConfig::rp4`]);
    /// `None` for an unknown name. The detector set of the same name is
    /// [`qp_facts::detectors_named`]. The protocol default is [`DEFAULT_ENGINE`].
    #[must_use]
    pub fn named(name: &str) -> Option<Self> {
        match name {
            "r3" => Some(Self::r3()),
            "rp4" => Some(Self::rp4()),
            _ => None,
        }
    }
}

/// (RP4) Names of the capsule constant records ([`EngineConfig::named`] and
/// `qp_facts::detectors_named`), oldest first. Old records stay selectable by name.
pub const ENGINES: &[&str] = &["r3", "rp4"];

/// (RP4) The record the protocol uses when the host names none (`init.capsules.engine`
/// absent): `r3`, so every existing transcript and dump stays byte-identical.
pub const DEFAULT_ENGINE: &str = "r3";

/// (RP4) The record whose outcome rule writes `rule` into a story's resolution
/// (`Story.resolution.rule`): `rp4` for the rule `rp4`, otherwise `r3`. Lets a reader
/// (a replica, a viewer's check) run the detector set the authority ran.
#[must_use]
pub fn engine_of_rule(rule: &str) -> &'static str {
    if rule == "rp4" {
        "rp4"
    } else {
        DEFAULT_ENGINE
    }
}

/// One transition of one story, with the story as it is after the transition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoryEvent {
    /// Capsule id (`cap.<instrument>.<theme>.<born bar>`).
    pub capsule: String,
    /// Finding id (`fnd.<instrument>.<theme>.<born bar>`).
    pub finding: String,
    /// Snapshot after the transition (importance and tier: the bar-level rank).
    pub story: Story,
    /// The transition.
    pub transition: Transition,
}

/// What [`Engine::candidates`] returns: a story that would be born or joined if the
/// open bar closed now. Never in the authority's log or the ledger.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CandidateStory {
    /// Capsule id the story has (join) or would get (birth).
    pub capsule: String,
    /// The story as it would be, with `stage = candidate`; the provisional transitions
    /// at the end of its history also carry `stage = candidate`.
    pub story: Story,
    /// The last provisional transition with the stage it would have on close
    /// (`born`, `strengthened`, `confirmed`, `weakened`).
    pub transition: Transition,
}

/// One ledger row with its point-in-time bookkeeping.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LedgerRecord {
    /// The row (`recorded_at` = close bar time).
    pub entry: LedgerEntry,
    /// Set when the story was retracted after it had closed: the row stops counting for
    /// questions asked strictly after this time.
    pub retracted_at: Option<i64>,
}

impl LedgerRecord {
    /// Whether the row counts for a ledger question asked at `before_t`: closed strictly
    /// before it and not retracted strictly before it.
    #[must_use]
    pub fn counts(&self, before_t: i64) -> bool {
        self.entry.recorded_at < before_t && self.retracted_at.is_none_or(|r| r >= before_t)
    }
}

/// A story as stored in an engine snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StorySnapshot {
    /// Capsule id.
    pub id: String,
    /// The story.
    pub story: Story,
    /// Facts that joined, in order.
    pub facts: Vec<Fact>,
}

impl From<&OpenStory> for StorySnapshot {
    fn from(s: &OpenStory) -> Self {
        Self {
            id: s.id.clone(),
            story: s.story.clone(),
            facts: s.facts.clone(),
        }
    }
}

/// Counts fixed at a story's birth (for [`RankCtx`]); `stories_all` is also the birth
/// sequence number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct BirthCounts {
    /// Same kind and direction on the same instrument, born before.
    pub same_kind_on_instrument: u32,
    /// Same kind on any instrument, born before.
    pub same_kind_all: u32,
    /// Any story, born before.
    pub stories_all: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
struct Counters {
    /// `instrument/kind/direction` → births.
    by_inst_kind_dir: BTreeMap<String, u32>,
    /// `kind` → births.
    by_kind: BTreeMap<String, u32>,
    /// All births.
    total: u32,
}

/// The whole engine state, serializable (for byte comparisons in tests and for
/// persistence by the authority).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EngineSnapshot {
    /// Open stories (birth order).
    pub open: Vec<StorySnapshot>,
    /// Closed stories (close order).
    pub closed: Vec<StorySnapshot>,
    /// Every event, in order.
    pub events: Vec<StoryEvent>,
    /// Every ledger row.
    pub ledger: Vec<LedgerRecord>,
    /// Instruments seen.
    pub instruments: Vec<String>,
    /// Birth counts per capsule id.
    pub birth_counts: BTreeMap<String, BirthCounts>,
    counters: Counters,
}

/// The capsule engine.
pub struct Engine {
    cfg: EngineConfig,
    open: Vec<OpenStory>,
    closed: Vec<OpenStory>,
    events: Vec<StoryEvent>,
    ledger: Vec<LedgerRecord>,
    instruments: BTreeSet<String>,
    counters: Counters,
    birth_counts: BTreeMap<String, BirthCounts>,
    /// Ledger rows indexed per key and overall in close-time order (R3/S-C): lookups in
    /// O(log n + retracted rows) instead of a scan of every row. Derived from `ledger`;
    /// never serialized.
    index: LedgerIndex,
}

/// Capsule id: `cap.<instrument>.<theme>.<born bar>`.
#[must_use]
pub fn capsule_id(instrument: &str, theme: &str, born_bar: u64) -> String {
    format!("cap.{instrument}.{theme}.{born_bar}")
}

/// Finding id for a capsule id (`cap.` → `fnd.`).
#[must_use]
pub fn finding_id(capsule: &str) -> String {
    match capsule.strip_prefix("cap.") {
        Some(rest) => format!("fnd.{rest}"),
        None => format!("fnd.{capsule}"),
    }
}

/// Ledger key of a fact: `<kind>/<direction>/<detector>@<version>` (R4, K42/A23: the
/// direction is part of the key, so up, down and neutral stories of one detector keep
/// separate records and separate bases); `None` without meaning.
#[must_use]
pub fn ledger_key(f: &Fact) -> Option<String> {
    f.meaning.as_ref().map(|m| {
        format!(
            "{}/{}/{}@{}",
            m.kind.name(),
            direction_name(m.direction),
            m.detector,
            m.detector_version
        )
    })
}

/// Parts of a ledger key: `(kind, direction, detector, version)`. Takes the R4 form
/// `kind/direction/detector@version` and the R3 form `kind/detector@version` (direction
/// `None`). `None` when the key has neither form.
#[must_use]
pub fn parse_ledger_key(key: &str) -> Option<(MeaningKind, Option<Direction>, &str, &str)> {
    let (kind, rest) = key.split_once('/')?;
    let kind = MeaningKind::ALL.into_iter().find(|m| m.name() == kind)?;
    let (dir, rest) = match rest.split_once('/') {
        Some((d, r)) => (
            Some(match d {
                "up" => Direction::Up,
                "down" => Direction::Down,
                "neutral" => Direction::Neutral,
                _ => return None,
            }),
            r,
        ),
        None => (None, rest),
    };
    let (det, ver) = rest.split_once('@')?;
    Some((kind, dir, det, ver))
}

/// Snake-case name of a direction (`up`, `down`, `neutral`).
#[must_use]
pub fn direction_name(d: Direction) -> &'static str {
    match d {
        Direction::Up => "up",
        Direction::Down => "down",
        Direction::Neutral => "neutral",
    }
}

fn opposite(a: Direction, b: Direction) -> bool {
    matches!(
        (a, b),
        (Direction::Up, Direction::Down) | (Direction::Down, Direction::Up)
    )
}

/// Detector-local code of a fact (`kind` without the `<detector>.` prefix).
fn fact_code(f: &Fact) -> &str {
    let det = f.meaning.as_ref().map_or("", |m| m.detector.as_str());
    f.kind
        .strip_prefix(det)
        .and_then(|r| r.strip_prefix('.'))
        .unwrap_or(&f.kind)
}

fn push_transition(
    s: &mut OpenStory,
    stage: Stage,
    bar: u64,
    t: i64,
    facts: Vec<String>,
    reason: &str,
) -> Transition {
    let tr = Transition {
        stage,
        bar,
        t,
        facts,
        reason: reason.to_owned(),
    };
    s.story.stage = stage;
    s.story.last_bar = bar;
    s.story.history.push(tr.clone());
    tr
}

fn event_of(s: &OpenStory, tr: Transition) -> StoryEvent {
    StoryEvent {
        capsule: s.id.clone(),
        finding: finding_id(&s.id),
        story: s.story.clone(),
        transition: tr,
    }
}

impl Engine {
    /// A new engine with no stories and an empty ledger.
    #[must_use]
    pub fn new(cfg: EngineConfig) -> Self {
        Self {
            cfg,
            open: Vec::new(),
            closed: Vec::new(),
            events: Vec::new(),
            ledger: Vec::new(),
            instruments: BTreeSet::new(),
            counters: Counters::default(),
            birth_counts: BTreeMap::new(),
            index: LedgerIndex::default(),
        }
    }

    /// Names of the rules (theme, outcome, ranker), for reports.
    #[must_use]
    pub fn rule_names(&self) -> (&'static str, &'static str, &'static str) {
        (
            self.cfg.theme.name(),
            self.cfg.outcome.name(),
            self.cfg.ranker.name(),
        )
    }

    /// Process one **final** bar of `(instrument, series)`: `bars` is that series up to
    /// and including `bar_index` (anything after it is ignored), `facts` the final facts
    /// of this bar. Returns this bar's events in order (also appended to
    /// [`Engine::events`]). Callers feed bars in global time order (`t`, then symbol).
    ///
    /// # Panics
    ///
    /// If `bars.len() <= bar_index`.
    pub fn on_bar(
        &mut self,
        instrument: &str,
        series: &str,
        bar_index: u64,
        bars: &[Bar],
        facts: &[Fact],
    ) -> Vec<StoryEvent> {
        let now = usize::try_from(bar_index).expect("bar index fits usize");
        assert!(bars.len() > now, "bars must include bar_index");
        let bars = &bars[..=now];
        self.instruments.insert(instrument.to_owned());
        let inp = BarIn {
            instrument,
            series,
            now: bar_index,
            t: bars[now].t,
            bars,
            facts,
            instruments: u32::try_from(self.instruments.len()).unwrap_or(u32::MAX),
        };

        let (mut mine, others): (Vec<OpenStory>, Vec<OpenStory>) = std::mem::take(&mut self.open)
            .into_iter()
            .partition(|s| s.story.instrument == instrument && s.story.series == series);
        let mut counters = std::mem::take(&mut self.counters);
        let out = self.run_bar(&inp, &mut mine, &mut counters);
        self.counters = counters;
        self.birth_counts.extend(out.new_counts);

        for s in &out.closed {
            self.record_close(s, inp.t);
        }
        self.closed.extend(out.closed);
        let mut open = others;
        open.extend(mine);
        open.sort_by_key(|s| self.birth_counts.get(&s.id).map_or(0, |b| b.stories_all));
        self.open = open;
        self.events.extend(out.events.iter().cloned());
        out.events
    }

    /// What would be born or joined if the open bar `bar_index` closed now with
    /// `provisional_facts`. Pure: runs the bar on a scratch copy of this instrument's
    /// open stories; the engine, its events and its ledger are untouched (K28, A16).
    /// Stories the outcome rule or the horizon would close on this bar are left out
    /// (they cannot be joined).
    ///
    /// # Panics
    ///
    /// If `bars.len() <= bar_index`.
    #[must_use]
    pub fn candidates(
        &self,
        instrument: &str,
        series: &str,
        bar_index: u64,
        bars: &[Bar],
        provisional_facts: &[Fact],
    ) -> Vec<CandidateStory> {
        let now = usize::try_from(bar_index).expect("bar index fits usize");
        assert!(bars.len() > now, "bars must include bar_index");
        let bars = &bars[..=now];
        let seen = self.instruments.len() + usize::from(!self.instruments.contains(instrument));
        let inp = BarIn {
            instrument,
            series,
            now: bar_index,
            t: bars[now].t,
            bars,
            facts: provisional_facts,
            instruments: u32::try_from(seen).unwrap_or(u32::MAX),
        };
        let mut scratch: Vec<OpenStory> = self
            .open
            .iter()
            .filter(|s| s.story.instrument == instrument && s.story.series == series)
            .cloned()
            .collect();
        let mut counters = self.counters.clone();
        let out = self.run_bar(&inp, &mut scratch, &mut counters);

        let mut res = Vec::new();
        for id in &out.step3_order {
            let Some(s) = scratch.iter().find(|s| &s.id == id) else {
                continue;
            };
            let n = out.step3.get(id).copied().unwrap_or(0);
            let mut story = s.story.clone();
            let Some(last) = story.history.last().cloned() else {
                continue;
            };
            let len = story.history.len();
            for tr in &mut story.history[len.saturating_sub(n)..] {
                tr.stage = Stage::Candidate;
            }
            story.stage = Stage::Candidate;
            res.push(CandidateStory {
                capsule: id.clone(),
                story,
                transition: last,
            });
        }
        res
    }

    /// Retract a story (K37): its data proved wrong. Works on an open story and on one
    /// that already closed as realized / invalidated / expired; never writes a ledger row.
    /// For a closed story its existing row is marked `retracted_at = t` and stops
    /// counting only for ledger questions asked after `t` (births before `t` keep their
    /// `ledger_at_birth`). Returns `None` if the capsule is unknown, already retracted, or
    /// `t` is earlier than its last transition. The rank is not recomputed.
    pub fn retract(&mut self, capsule: &str, bar: u64, t: i64, reason: &str) -> Option<StoryEvent> {
        if let Some(pos) = self.open.iter().position(|s| s.id == capsule) {
            if self.open[pos].story.history.last().is_some_and(|l| t < l.t) {
                return None;
            }
            let mut s = self.open.remove(pos);
            let tr = push_transition(&mut s, Stage::Retracted, bar, t, Vec::new(), reason);
            let ev = event_of(&s, tr);
            self.closed.push(s);
            self.events.push(ev.clone());
            return Some(ev);
        }
        let s = self.closed.iter_mut().find(|s| s.id == capsule)?;
        if s.story.stage == Stage::Retracted || s.story.history.last().is_some_and(|l| t < l.t) {
            return None;
        }
        let tr = push_transition(s, Stage::Retracted, bar, t, Vec::new(), reason);
        let ev = event_of(s, tr);
        for r in &mut self.ledger {
            if r.entry.capsule.as_deref() == Some(capsule) && r.retracted_at.is_none() {
                r.retracted_at = Some(t);
                self.index.retract(r, t);
            }
        }
        self.events.push(ev.clone());
        Some(ev)
    }

    /// Ledger statistic of `key` over the closes strictly before `before_t` (rows of
    /// stories retracted before `before_t` excluded).
    #[must_use]
    ///
    /// O(log n) in the rows of `key` (plus its retracted rows): the rows are indexed per
    /// key in close-time order (R3/S-C); the result is the row scan's
    /// ([`Engine::ledger_stat_scan`]).
    pub fn ledger_stat(&self, key: &str, before_t: i64) -> LedgerStat {
        self.index
            .by_key
            .get(key)
            .map_or_else(LedgerStat::default, |k| k.stat(before_t))
    }

    /// Ledger statistic over all keys, closes strictly before `before_t` (indexed).
    #[must_use]
    pub fn ledger_stat_all(&self, before_t: i64) -> LedgerStat {
        self.index.all.stat(before_t)
    }

    /// [`Engine::ledger_stat`] by scanning every row (the definition; tests compare the
    /// index against it).
    #[must_use]
    pub fn ledger_stat_scan(&self, key: Option<&str>, before_t: i64) -> LedgerStat {
        stat(
            self.ledger
                .iter()
                .filter(|r| key.is_none_or(|k| r.entry.key.as_deref() == Some(k))),
            before_t,
        )
    }

    /// Open stories, in birth order.
    #[must_use]
    pub fn open_stories(&self) -> &[OpenStory] {
        &self.open
    }

    /// Closed stories (including retracted), in close order.
    #[must_use]
    pub fn closed_stories(&self) -> &[OpenStory] {
        &self.closed
    }

    /// Every event so far, in order.
    #[must_use]
    pub fn events(&self) -> &[StoryEvent] {
        &self.events
    }

    /// Ledger rows as of now: retracted rows left out.
    #[must_use]
    pub fn ledger(&self) -> Vec<LedgerEntry> {
        self.ledger
            .iter()
            .filter(|r| r.retracted_at.is_none())
            .map(|r| r.entry.clone())
            .collect()
    }

    /// Every ledger row with its point-in-time bookkeeping, in record order.
    #[must_use]
    pub fn ledger_records(&self) -> &[LedgerRecord] {
        &self.ledger
    }

    /// Full serializable state.
    #[must_use]
    pub fn snapshot(&self) -> EngineSnapshot {
        EngineSnapshot {
            open: self.open.iter().map(StorySnapshot::from).collect(),
            closed: self.closed.iter().map(StorySnapshot::from).collect(),
            events: self.events.clone(),
            ledger: self.ledger.clone(),
            instruments: self.instruments.iter().cloned().collect(),
            birth_counts: self.birth_counts.clone(),
            counters: self.counters.clone(),
        }
    }
}
