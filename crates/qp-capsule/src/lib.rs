//! Qapsule capsule engine (R3): facts → findings → capsule stories → ledger.
//!
//! Deterministic like the rest of the core: no clock (time is the bar), no network, no
//! randomness, no LLM. The same bars and the same detector output always give the same
//! stories, transitions, ledger and sentences.
//!
//! The **contract** written in R3/T1 (`contract.rs`, re-exported here): the measured design questions are
//! traits so that alternatives (R3 M2–M6) plug in without seeing each other:
//!
//! | Trait | Question | Experiment |
//! |-------|----------|------------|
//! | [`Realizer`] | How is the sentence produced? | M2 |
//! | [`Ranker`] | How important is a story (importance, tier)? | M3 |
//! | [`ThemeRule`] | Which facts belong to the same story? | M4 |
//! | [`OutcomeRule`] | What does "realized" mean per kind; horizon in bars? | M5 |
//! | [`TrustPath`] | How does a licensed server fact reach an unentitled user? | M6 |
//!
//! The lifecycle rules (`docs/CAPSULE.md` §9.3) are fixed, not an experiment.
//!
//! Modules (R3/T4+T5):
//!
//! - [`engine`]: the lifecycle state machine, candidates, ledger, retraction, glyph spec.
//! - [`pipeline`]: headless simulation (detectors → facts → engine) in global time order.
//! - [`audience`]: the sensitivity filter (§9.8).
//! - [`feedrule`]: the feed's coin-flip rule (§9.10, K53; R5/M19).
//! - [`scaffold`]: trivial strategies for tests only.
//!
//! The measured choices (R3/S-A, productized from `rounds/R3/experiments/`):
//!
//! | Module | Strategy | Experiment |
//! |--------|----------|------------|
//! | [`theme`] | [`theme::KindDir`] (kind + direction, flow as evidence) | M4a |
//! | [`outcome`] | [`outcome::Barrier`] (per-kind ATR triple barrier) | M5a |
//! | [`rank`] | [`rank::Confirm`] (confirmation-led combination) | M3c |
//! | [`realize`] | [`realize::Template`] (template grammar, TR + EN) | M2a |
//! | [`trust`] | [`trust::Muhur`] (binary-lattice seal with receipts) | M6b |
//!
//! [`EngineConfig::r3`] is the engine with these choices; [`sentence::sentences_at`]
//! writes a story's sentences from engine data (K26).
//!
//! (RP4) Capsule constants are **named records** ([`EngineConfig::named`], [`ENGINES`];
//! `docs/CAPSULE.md` §9.9): `r3` (above, the protocol default [`DEFAULT_ENGINE`]) and
//! `rp4` (outcome [`outcome::Rp4`]: [`outcome::HighLow`] on intraday bars, the R3
//! barrier on daily bars; detector set `qp_facts::rp4_detectors`).

pub mod audience;
pub mod comment;
pub mod engine;
pub mod feedrule;
pub mod market;
pub mod outcome;
pub mod pipeline;
pub mod rank;
pub mod realize;
pub mod scaffold;
pub mod sentence;
pub mod sentinel;
pub mod theme;
pub mod trust;

mod contract;
pub use contract::{
    FlowObservation, Lang, OpenStory, OutcomeRule, QualMessage, RankCtx, Ranker, Realizer,
    SentenceInput, ThemeDecision, ThemeRule, TrustPath, Verdict,
};

pub use audience::{admits, admits_at};
pub use engine::{
    engine_of_rule, glyph_spec, glyph_spec_at, parse_ledger_key, CandidateStory, Engine,
    EngineConfig, EngineSnapshot, LedgerRecord, StoryEvent, StorySnapshot, DEFAULT_ENGINE, ENGINES,
    FRESHNESS_HALF_BARS,
};
pub use pipeline::{simulate, InstrumentInput, SimResult};
