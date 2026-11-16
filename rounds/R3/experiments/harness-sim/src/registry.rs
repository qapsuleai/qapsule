//! Registries: the strategies the harness can measure, by name.
//!
//! An alternative adds **one line** to the list of its experiment (and one path
//! dependency in `Cargo.toml`); see README "Alternatif ekleme". The alternatives'
//! crates live under `rounds/R3/experiments/M<k>/<ad>/`.
//!
//! `reference` = the shared reference strategy (not a candidate); `control` = the
//! deliberately trivial self-test strategy (not a candidate).

use qp_capsule::{OutcomeRule, Ranker, ThemeRule, TrustPath};

use crate::control;
use crate::reference;

/// Builds a ranker.
pub type RankerFactory = fn() -> Box<dyn Ranker>;
/// Builds a theme rule.
pub type ThemeFactory = fn() -> Box<dyn ThemeRule>;
/// Builds an outcome rule and its perturbations (horizon ±20 %, thresholds ±10 %).
pub type OutcomeFactory = fn() -> (Box<dyn OutcomeRule>, Vec<(String, Box<dyn OutcomeRule>)>);
/// Builds a fresh trust path (one instance per instrument stream).
pub type TrustFactory = fn() -> Box<dyn TrustPath>;

/// M3 rankers.
#[must_use]
pub fn rankers() -> Vec<(&'static str, RankerFactory)> {
    vec![
        ("reference", || Box::new(reference::RefRanker)),
        ("control", || Box::new(control::RoundRobinRanker)),
        // M3 alternatifleri: bir satır, ör.
        // ("a1", m3_a1::ranker),
        ("rarity", m3_rarity::ranker),
        ("confirm", m3_confirm::ranker),
        ("ledger", m3_ledger::ranker),
    ]
}

/// M4 theme rules.
#[must_use]
pub fn themes() -> Vec<(&'static str, ThemeFactory)> {
    vec![
        ("reference", || Box::new(reference::RefTheme)),
        ("control", || Box::new(control::SingletonTheme)),
        // M4 alternatifleri: bir satır, ör.
        // ("a1", m4_a1::theme),
        ("kinddir", m4_kinddir::theme),
        ("arc", m4_arc::theme),
    ]
}

/// M5 outcome rules.
#[must_use]
pub fn outcomes() -> Vec<(&'static str, OutcomeFactory)> {
    vec![
        ("reference", || {
            (
                Box::new(reference::RefOutcome::default()),
                reference::RefOutcome::perturbations(),
            )
        }),
        ("control", || {
            (
                Box::new(control::AlwaysRealized),
                control::always_perturbations(),
            )
        }),
        // M5 alternatifleri: bir satır, ör.
        // ("a1", || (m5_a1::outcome(), m5_a1::perturbations())),
        ("barrier", || {
            (m5_barrier::outcome(), m5_barrier::perturbations())
        }),
        ("semantic", || (m5_semantic::outcome(), m5_semantic::perturbations())),
    ]
}

/// M6 trust paths.
#[must_use]
pub fn trust_paths() -> Vec<(&'static str, TrustFactory)> {
    vec![
        ("control", || Box::new(control::EchoTrust)),
        // M6 alternatifleri: bir satır, ör.
        // ("a1", m6_a1::trust_path),
        ("coarse", m6_coarse::trust_path),
        ("muhur", m6_muhur::trust_path),
    ]
}

/// Look up a factory by name.
#[must_use]
pub fn find<F: Copy>(list: &[(&'static str, F)], name: &str) -> Option<F> {
    list.iter().find(|(n, _)| *n == name).map(|(_, f)| *f)
}

/// Shared reference rules (the non-measured strategies of each experiment).
#[must_use]
pub fn reference_theme() -> Box<dyn ThemeRule> {
    if let Ok(n) = std::env::var("QP_R3_THEME") {
        return find(&themes(), &n).expect("QP_R3_THEME: unknown theme")();
    }
    Box::new(reference::RefTheme)
}
/// Reference outcome.
#[must_use]
pub fn reference_outcome() -> Box<dyn OutcomeRule> {
    if let Ok(n) = std::env::var("QP_R3_OUTCOME") {
        return find(&outcomes(), &n).expect("QP_R3_OUTCOME: unknown outcome")().0;
    }
    Box::new(reference::RefOutcome::default())
}
/// Reference ranker.
#[must_use]
pub fn reference_ranker() -> Box<dyn Ranker> {
    if let Ok(n) = std::env::var("QP_R3_RANKER") {
        return find(&rankers(), &n).expect("QP_R3_RANKER: unknown ranker")();
    }
    Box::new(reference::RefRanker)
}
