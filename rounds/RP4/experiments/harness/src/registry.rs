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
        RuleEntry {
            name: "taban",
            make: m25_taban::outcome,
            levels: |s, b| m25_taban::levels(s, b).map(Levels::from),
            // r3's 7 + N_TICKS (STEP_TOL is float rounding, not tuned).
            constants: 8,
        // RP4/M25b "iki kapanışla teyit" (M25/teyit2): r3'ün 7 sabiti (C_DIR·K_SCALE tek
        // çarpan sayılır) + STOP_CLOSES + TARGET_CLOSES.
        RuleEntry {
            name: "teyit2",
            make: m25_teyit2::outcome,
            levels: |s, b| m25_teyit2::levels(s, b).map(Levels::from),
            constants: 9,
            name: "olcek",
            make: m25_olcek::outcome,
            levels: |s, b| m25_olcek::levels(s, b).map(Levels::from),
            constants: 8,
        },
        // M25 alternatifleri: bir satır, ör.
        // RuleEntry { name: "a1", make: m25_a1::outcome, levels: |s, b| m25_a1::levels(s, b).map(Levels::from), constants: 4 },
        RuleEntry {
            name: "yukdus",
            make: m25_yukdus::outcome,
            levels: |s, b| m25_yukdus::levels(s, b).map(Levels::from),
            constants: 7,
        // r3's 7 + Z + M.
        RuleEntry {
            name: "umut",
            make: m25_umut::outcome,
            levels: |s, b| m25_umut::levels(s, b).map(Levels::from),
            constants: 9,
        },
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
        DetectorEntry {
            name: "olcek26",
            make: m26_olcek26::detectors,
            name: "mevsim",
            make: m26_mevsim::detectors,
        },
        // `mevsim` denemeleri (M26/mevsim/NOTES.md): yeniden üretim için.
        DetectorEntry {
            name: "mevsim_t1v",
            make: m26_mevsim::trial_volume,
        },
        DetectorEntry {
            name: "mevsim_t1vb",
            make: m26_mevsim::trial_volume_breakout,
        },
        DetectorEntry {
            name: "mevsim_t2g",
            make: m26_mevsim::trial_gate,
        },
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
