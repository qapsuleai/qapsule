//! Registries: the outcome rules (M25) and detector sets (M26) the harness can measure,
//! by name. An alternative adds **one line** here and one path dependency in
//! `Cargo.toml` (README "Alternatif ekleme").
//!
//! `r3` is the reference: the product's outcome rule (`qp_capsule::outcome::Barrier`
//! with its defaults) and the product's detector set (`qp_facts::default_detectors`).

use qp_capsule::outcome::{base_params, wilder_atr, Barrier, ATR_FAST, ATR_SLOW};
use qp_capsule::{OpenStory, OutcomeRule};
use qp_facts::Detector;
use qp_schema::capsule::Direction;
use qp_schema::Bar;

/// A rule's barrier levels for one story, fixed at its birth (SPEC §3 `ECR_own`, `SENS`).
///
/// - Directional: the claim's signed move `d = ±(close − p0)`; `d ≥ target` is the
///   rule's own target, `d ≤ −stop` its stop (if it has one).
/// - Neutral: `|close − p0| ≥ target` is the rule's own target; `stop` unused.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Levels {
    /// Reference price.
    pub p0: f64,
    /// Target distance from `p0` (price units, > 0).
    pub target: f64,
    /// Stop distance from `p0` (price units, > 0), if the rule has a fixed stop.
    pub stop: Option<f64>,
}

impl From<(f64, f64, Option<f64>)> for Levels {
    fn from((p0, target, stop): (f64, f64, Option<f64>)) -> Self {
        Self { p0, target, stop }
    }
}

/// Builds a fresh outcome rule (one instance per symbol run).
pub type RuleFactory = fn() -> Box<dyn OutcomeRule>;
/// The rule's levels for a story, from `bars[..=born]` only (the harness passes exactly
/// that slice). `None` = undefined (e.g. warm-up).
pub type LevelsFn = fn(&OpenStory, &[Bar]) -> Option<Levels>;
/// Builds a fresh detector set (one per symbol).
pub type DetectorFactory = fn() -> Vec<Box<dyn Detector>>;

/// One outcome rule entry.
#[derive(Clone, Copy)]
pub struct RuleEntry {
    /// Name (`--rule`).
    pub name: &'static str,
    /// Factory.
    pub make: RuleFactory,
    /// Levels.
    pub levels: LevelsFn,
    /// Number of tuned constants (SPEC §4 (d) simplicity).
    pub constants: u32,
}

/// One detector set entry.
#[derive(Clone, Copy)]
pub struct DetectorEntry {
    /// Name (`--detectors`).
    pub name: &'static str,
    /// Factory.
    pub make: DetectorFactory,
}

/// `r3` levels: `p0` = birth close; directional `k·ATR14` both ways, neutral
/// `k_n·ATR100` (`qp_capsule::outcome`, base parameters).
#[must_use]
pub fn r3_levels(s: &OpenStory, bars: &[Bar]) -> Option<Levels> {
    let born = usize::try_from(s.story.born_bar).ok()?;
    let p0 = bars.get(born)?.c;
    let p = base_params(s.story.kind);
    if s.story.direction == Direction::Neutral {
        let a = wilder_atr(&bars[..=born], ATR_SLOW)?;
        Some(Levels {
            p0,
            target: p.k_neutral * a,
            stop: None,
        })
    } else {
        let a = wilder_atr(&bars[..=born], ATR_FAST)?;
        Some(Levels {
            p0,
            target: p.k * a,
            stop: Some(p.k * a),
        })
    }
}

/// M25 outcome rules.
#[must_use]
pub fn rules() -> Vec<RuleEntry> {
    vec![
        RuleEntry {
            name: "r3",
            make: || Box::new(Barrier::default()),
            levels: r3_levels,
            // C_DIR, C_NEUTRAL, ATR_FAST, ATR_SLOW, horizons 20 / 15 / 10.
            constants: 7,
        },
        // M25 alternatifleri: bir satır, ör.
        // RuleEntry { name: "a1", make: m25_a1::outcome, levels: |s, b| m25_a1::levels(s, b).map(Levels::from), constants: 4 },
    ]
}

/// M26 detector sets.
#[must_use]
pub fn detector_sets() -> Vec<DetectorEntry> {
    vec![
        DetectorEntry {
            name: "r3",
            make: qp_facts::default_detectors,
        },
        // M26 alternatifleri: bir satır, ör.
        // DetectorEntry { name: "a1", make: m26_a1::detectors },
    ]
}

/// Rule by name.
#[must_use]
pub fn rule(name: &str) -> Option<RuleEntry> {
    rules().into_iter().find(|r| r.name == name)
}

/// Detector set by name.
#[must_use]
pub fn detector_set(name: &str) -> Option<DetectorEntry> {
    detector_sets().into_iter().find(|r| r.name == name)
}

/// The reference rule (`r3`).
///
/// # Panics
/// Never (`r3` is always registered).
#[must_use]
pub fn reference_rule() -> RuleEntry {
    rule("r3").expect("r3")
}

/// The reference detector set (`r3`).
///
/// # Panics
/// Never.
#[must_use]
pub fn reference_detectors() -> DetectorEntry {
    detector_set("r3").expect("r3")
}
