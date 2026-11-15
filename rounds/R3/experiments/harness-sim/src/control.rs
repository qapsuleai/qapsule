//! **Control** strategies: deliberately trivial, used only to self-test the harness.
//! Not candidates.
//!
//! - M3 [`RoundRobinRanker`]: tier = `stories_all mod 3 + 1` (the story's birth sequence
//!   number): carries no information, so tier 1 − tier 3 realized share should be ≈ 0.
//! - M4 [`SingletonTheme`]: every fact its own story (= the baseline): reduction 0, no
//!   wrong merge, every within-episode pair split.
//! - M5 [`AlwaysRealized`]: every story realized on the first bar after birth: perfectly
//!   "stable", meaningless (decoy difference 0).
//! - M6 [`EchoTrust`]: one message per `flow.institutional` fact, shown at once, stating
//!   only "unusual institutional buying/selling", server-verified.

use std::collections::BTreeMap;

use qp_capsule::{
    FlowObservation, OpenStory, OutcomeRule, QualMessage, RankCtx, Ranker, ThemeDecision,
    ThemeRule, TrustPath, Verdict,
};
use qp_schema::capsule::{Direction, MeaningKind, TimeKind, Trust};
use qp_schema::{Bar, Fact};

/// M3 control.
#[derive(Debug, Clone, Copy, Default)]
pub struct RoundRobinRanker;

impl Ranker for RoundRobinRanker {
    fn name(&self) -> &'static str {
        "control.round_robin"
    }
    fn rank(&self, _s: &OpenStory, ctx: &RankCtx) -> (f64, u8) {
        let tier = u8::try_from(ctx.stories_all % 3).unwrap_or(0) + 1;
        (f64::from(3 - tier), tier)
    }
}

/// M4 control.
#[derive(Debug, Clone, Copy, Default)]
pub struct SingletonTheme;

impl ThemeRule for SingletonTheme {
    fn name(&self) -> &'static str {
        "control.singleton"
    }
    fn decide(&self, fact: &Fact, _open: &[&OpenStory]) -> ThemeDecision {
        if fact.meaning.is_none() {
            return ThemeDecision::Ignore;
        }
        ThemeDecision::New {
            theme: format!("solo.{}", fact.kind),
        }
    }
}

/// M5 control.
#[derive(Debug, Clone, Copy, Default)]
pub struct AlwaysRealized;

impl OutcomeRule for AlwaysRealized {
    fn name(&self) -> &'static str {
        "control.always_realized"
    }
    fn horizon(&self, _kind: MeaningKind, _time: TimeKind) -> u32 {
        10
    }
    fn judge(&self, s: &OpenStory, _bars: &[Bar], now: u64) -> Option<(Verdict, &'static str)> {
        (now > s.story.born_bar).then_some((Verdict::Realized, "control.always"))
    }
}

/// Perturbations of the M5 control (horizon ±20 %; it has no threshold).
#[must_use]
pub fn always_perturbations() -> Vec<(String, Box<dyn OutcomeRule>)> {
    vec![
        (
            "h-20%".to_owned(),
            Box::new(AlwaysRealized) as Box<dyn OutcomeRule>,
        ),
        ("h+20%".to_owned(), Box::new(AlwaysRealized)),
    ]
}

/// M6 control.
#[derive(Debug, Clone, Copy, Default)]
pub struct EchoTrust;

impl TrustPath for EchoTrust {
    fn name(&self) -> &'static str {
        "control.echo"
    }
    fn observe(&mut self, obs: &FlowObservation, facts: &[Fact]) -> Vec<QualMessage> {
        facts
            .iter()
            .filter_map(|f| {
                let m = f.meaning.as_ref()?;
                if !m.server {
                    return None;
                }
                let (code, tr, en) = match m.direction {
                    Direction::Up => (
                        "unusual_buy",
                        "Sunucu doğruladı: olağandışı kurumsal alış.",
                        "Server-verified: unusual institutional buying.",
                    ),
                    Direction::Down => (
                        "unusual_sell",
                        "Sunucu doğruladı: olağandışı kurumsal satış.",
                        "Server-verified: unusual institutional selling.",
                    ),
                    Direction::Neutral => (
                        "unusual_flow",
                        "Sunucu doğruladı: olağandışı kurumsal akış.",
                        "Server-verified: unusual institutional flow.",
                    ),
                };
                Some(QualMessage {
                    t: obs.t,
                    bar: obs.bar,
                    instrument: obs.instrument.clone(),
                    codes: vec![code.to_owned()],
                    tr: tr.to_owned(),
                    en: en.to_owned(),
                    trust: Trust::Server,
                    shown_bar: obs.bar,
                    attachments: BTreeMap::new(),
                })
            })
            .collect()
    }
}
