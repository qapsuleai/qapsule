//! Test scaffold only — not an M3/M4/M5 candidate.
//!
//! Deliberately minimal strategies so the engine can be exercised before the measured
//! alternatives exist. Do not tune them and do not start from them: M3/M4/M5 are designed
//! independently.

use qp_schema::capsule::{Edge, MeaningKind, TimeKind};
use qp_schema::{Bar, Fact};

use crate::{OpenStory, OutcomeRule, RankCtx, Ranker, ThemeDecision, ThemeRule, Verdict};

/// Theme = meaning kind. A fact joins the oldest open story of its kind; an `exit` joins
/// the oldest story holding an `enter` of the same detector, else is ignored.
#[derive(Debug, Clone, Copy, Default)]
pub struct KindTheme;

impl ThemeRule for KindTheme {
    fn name(&self) -> &'static str {
        "scaffold.kind_theme"
    }

    fn decide(&self, fact: &Fact, open: &[&OpenStory]) -> ThemeDecision {
        let Some(m) = fact.meaning.as_ref() else {
            return ThemeDecision::Ignore;
        };
        if m.edge == Edge::Exit {
            let own = open.iter().find(|s| {
                s.facts.iter().any(|g| {
                    g.meaning
                        .as_ref()
                        .is_some_and(|gm| gm.edge == Edge::Enter && gm.detector == m.detector)
                })
            });
            return own.map_or(ThemeDecision::Ignore, |s| ThemeDecision::Join(s.id.clone()));
        }
        match open.iter().find(|s| s.story.kind == m.kind) {
            Some(s) => ThemeDecision::Join(s.id.clone()),
            None => ThemeDecision::New {
                theme: m.kind.name().to_owned(),
            },
        }
    }
}

/// Never judges; every story expires after a fixed horizon.
#[derive(Debug, Clone, Copy)]
pub struct FixedHorizon(pub u32);

impl OutcomeRule for FixedHorizon {
    fn name(&self) -> &'static str {
        "scaffold.fixed_horizon"
    }

    fn horizon(&self, _kind: MeaningKind, _time: TimeKind) -> u32 {
        self.0
    }

    fn judge(&self, _s: &OpenStory, _bars: &[Bar], _now: u64) -> Option<(Verdict, &'static str)> {
        None
    }
}

/// Constant importance 1 and tier 2.
#[derive(Debug, Clone, Copy, Default)]
pub struct ConstRank;

impl Ranker for ConstRank {
    fn name(&self) -> &'static str {
        "scaffold.const_rank"
    }

    fn rank(&self, _s: &OpenStory, _ctx: &RankCtx) -> (f64, u8) {
        (1.0, 2)
    }
}

/// The three scaffold rules as an engine config (horizon in bars).
#[must_use]
pub fn config(horizon: u32) -> crate::engine::EngineConfig {
    crate::engine::EngineConfig {
        theme: Box::new(KindTheme),
        outcome: Box::new(FixedHorizon(horizon)),
        ranker: Box::new(ConstRank),
    }
}
