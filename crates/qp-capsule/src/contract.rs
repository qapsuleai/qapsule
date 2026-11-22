//! The capsule contract (R3/T1): the open story and the traits of the measured design
//! questions (M2–M6), kept apart from `lib.rs` so the crate root only declares.

use std::collections::BTreeMap;

use qp_schema::capsule::{LedgerStat, MeaningKind, Story, TimeKind, Trust};
use qp_schema::{Bar, Fact};

/// A story that is still open, with the facts that joined it so far (in join order).
#[derive(Debug, Clone, PartialEq)]
pub struct OpenStory {
    /// Capsule id (`cap.<instrument>.<theme>.<born bar>`).
    pub id: String,
    /// The story.
    pub story: Story,
    /// Facts that joined, in order (the first one gave birth).
    pub facts: Vec<Fact>,
}

// ---------------------------------------------------------------------------
// M4: theme identity
// ---------------------------------------------------------------------------

/// What to do with a new final fact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThemeDecision {
    /// Join this open story (capsule id).
    Join(String),
    /// Start a new story with this theme key.
    New {
        /// Theme key (stable, deterministic; becomes part of the capsule id).
        theme: String,
    },
    /// The fact starts nothing and joins nothing (e.g. a state exit with no story).
    Ignore,
}

/// Theme identity rule (M4).
pub trait ThemeRule {
    /// Name used in reports.
    fn name(&self) -> &'static str;
    /// Decide for `fact` (which has `meaning`), given the open stories of the same
    /// instrument (oldest first).
    fn decide(&self, fact: &Fact, open: &[&OpenStory]) -> ThemeDecision;
}

// ---------------------------------------------------------------------------
// M5: outcome definition
// ---------------------------------------------------------------------------

/// Outcome of a story decided by the rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// It came true: `realized`, ledger hit.
    Realized,
    /// Its invalidation happened: `invalidated`, ledger miss.
    Invalidated,
}

/// Outcome rule (M5). Must read nothing after `now` (the engine only passes
/// `bars[..=now]`; the look-ahead test shuffles later data and checks the ledger).
pub trait OutcomeRule {
    /// Name used in reports.
    fn name(&self) -> &'static str;
    /// Natural life in bars for a kind and time kind; reaching it without a verdict
    /// closes the story as `expired`.
    fn horizon(&self, kind: MeaningKind, time: TimeKind) -> u32;
    /// Judge an open story at bar `now` (`bars` = the story's bar series up to and
    /// including `now`). `None` = undecided. The reason is a short machine code.
    fn judge(&self, s: &OpenStory, bars: &[Bar], now: u64) -> Option<(Verdict, &'static str)>;
}

// ---------------------------------------------------------------------------
// M3: ranking
// ---------------------------------------------------------------------------

/// What a ranker may read about a story's context, all as of the bar it ranks at
/// (no look-ahead).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RankCtx {
    /// Ledger of the story's ledger key, closed strictly before this bar.
    pub ledger: LedgerStat,
    /// Ledger over all keys, closed strictly before this bar.
    pub ledger_all: LedgerStat,
    /// Stories of the same kind and direction born on this instrument before this one.
    pub same_kind_on_instrument: u32,
    /// Bars of this instrument seen so far (including this bar).
    pub bars_seen: u64,
    /// Stories of the same kind born on any instrument before this one.
    pub same_kind_all: u32,
    /// Stories of any kind born on any instrument before this one.
    pub stories_all: u32,
    /// Instruments seen so far.
    pub instruments: u32,
}

/// Ranking rule (M3): importance score and tier (1 most important … 3).
pub trait Ranker {
    /// Name used in reports.
    fn name(&self) -> &'static str;
    /// Rank a story at its birth and at every later transition.
    fn rank(&self, s: &OpenStory, ctx: &RankCtx) -> (f64, u8);
}

// ---------------------------------------------------------------------------
// M2: sentence
// ---------------------------------------------------------------------------

/// Sentence language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Lang {
    /// Turkish.
    Tr,
    /// English.
    En,
}

/// Everything a realizer may say; every number in here comes from the engine (K32).
#[derive(Debug, Clone, PartialEq)]
pub struct SentenceInput<'a> {
    /// The story (stage, kind, direction, time kind, families, ledger at birth…).
    pub story: &'a Story,
    /// The fact that gave birth.
    pub lead: &'a Fact,
    /// The latest fact that joined (may be `lead`).
    pub latest: &'a Fact,
    /// Bar the sentence is written for ("now").
    pub now_bar: u64,
    /// Its time.
    pub now_t: i64,
    /// Whether the lead fact's state/condition still holds now (`None` if not
    /// applicable, e.g. a point event without a condition).
    pub still_holds: Option<bool>,
    /// Current value of the lead fact's subject (e.g. today's RSI) if applicable and
    /// visible to the reader; `None` otherwise.
    pub current: Option<f64>,
    /// `true` when the reader may not see numbers of this fact (licensed, unentitled,
    /// K33): the sentence must contain no digits at all.
    pub redacted: bool,
}

/// Sentence producer (M2). Must be a pure function of its input.
pub trait Realizer {
    /// Name used in reports.
    fn name(&self) -> &'static str;
    /// One sentence in `lang`.
    fn sentence(&self, inp: &SentenceInput<'_>, lang: Lang) -> String;
}

// ---------------------------------------------------------------------------
// M6: server-fact trust path
// ---------------------------------------------------------------------------

/// One licensed observation the server sees (never sent to an unentitled user).
#[derive(Debug, Clone, PartialEq)]
pub struct FlowObservation {
    /// Bar time.
    pub t: i64,
    /// Bar index.
    pub bar: u64,
    /// Instrument.
    pub instrument: String,
    /// Raw licensed values (e.g. `net`, `avg5`).
    pub values: BTreeMap<String, f64>,
}

/// What an unentitled user receives about a server fact.
#[derive(Debug, Clone, PartialEq)]
pub struct QualMessage {
    /// Bar time the comment is about.
    pub t: i64,
    /// Bar index.
    pub bar: u64,
    /// Instrument.
    pub instrument: String,
    /// Machine codes of the qualitative relations stated (`above_avg`, `rising`…).
    pub codes: Vec<String>,
    /// Turkish sentence (no digits).
    pub tr: String,
    /// English sentence (no digits).
    pub en: String,
    /// Trust mark shown with it (K38).
    pub trust: Trust,
    /// When it is shown, as a bar index (`>= bar`; display delay in bars).
    pub shown_bar: u64,
    /// Anything else the user receives (e.g. a verification token); never a raw value.
    pub attachments: BTreeMap<String, String>,
}

/// Server-fact trust path (M6). Deterministic; sees raw values, emits only messages.
pub trait TrustPath {
    /// Name used in reports.
    fn name(&self) -> &'static str;
    /// Observe one bar's licensed data (and the server facts detected on it); return the
    /// messages for an unentitled user.
    fn observe(&mut self, obs: &FlowObservation, facts: &[Fact]) -> Vec<QualMessage>;
}
