//! R3/M3 candidate **"confirm"** — confirmation-led combination (teyit merkezli birleşim).
//!
//! Premise (K30, `docs/CAPSULE.md` §4): a finding's strength grows with how many
//! **independent evidence families** confirm it; five indicators from one family count as
//! one confirmation. So the ranking is lexicographic in the net confirmation count and only
//! then uses the secondary evidence:
//!
//! 1. **Net confirmation** `c = support − oppose − ½·ended`
//!    - `support`: distinct families that confirmed the story (`story.families`),
//!    - `oppose`: distinct families among joined facts pointing the opposite way,
//!    - `ended`: the story's own state ended (an `exit` of a detector that entered it).
//! 2. **Secondary score** `s ∈ [0, 1)`: a weighted mean of
//!    - strength (`story.strength`, weight 0.2),
//!    - track record (shrunk ledger lift of the story's ledger key over all keys,
//!      weight 0.5),
//!    - rarity (how rare the kind is overall and on this instrument, weight 0.3).
//!
//! Importance = `c + s` (so one more independent family always outranks any secondary
//! evidence). Tier:
//!
//! | Net confirmation | Tier |
//! |------------------|------|
//! | `c ≥ 2` (two or more independent families) | 1 |
//! | `1 ≤ c < 2` | 1 if `s ≥ θ₁`, 2 if `s ≥ θ₂`, else 3 |
//! | `½ ≤ c < 1` (single family, own state ended) | 2 if `s ≥ θ₁`, else 3 |
//! | `c < ½` (contradicted) | 3 |
//!
//! The ranker is a pure function of the story and its [`RankCtx`]: no state, no clock, no
//! randomness, nothing after the bar it ranks at. It is called at birth and at every
//! later transition, so a story rises when a new family confirms it and falls when it is
//! contradicted or its state ends.

use qp_capsule::{OpenStory, RankCtx, Ranker};
use qp_schema::capsule::{Direction, Edge, EvidenceFamily};

/// Parameters of the rule (all fixed; see `NOTES.md` for why these values).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Config {
    /// Weight of strength in the secondary score.
    pub w_strength: f64,
    /// Weight of the track record in the secondary score.
    pub w_track: f64,
    /// Weight of rarity in the secondary score.
    pub w_rarity: f64,
    /// Pseudo-count that shrinks a key's ledger rate towards the all-keys rate.
    pub shrink: f64,
    /// Gain that maps the shrunk lift (`p_key − p_all`) to `[0, 1]` around ½.
    pub track_gain: f64,
    /// Kind share at which kind rarity reaches 1 (`−ln q / −ln q_min`).
    pub rare_share: f64,
    /// Secondary score a single-family story needs for tier 1 (θ₁).
    pub theta_1: f64,
    /// Secondary score a single-family story needs for tier 2 (θ₂).
    pub theta_2: f64,
    /// Penalty on the confirmation count when the story's own state ended.
    pub ended_penalty: f64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            w_strength: 0.2,
            w_track: 0.5,
            w_rarity: 0.3,
            shrink: 30.0,
            track_gain: 2.0,
            rare_share: 0.02,
            theta_1: 0.69,
            theta_2: 0.58,
            ended_penalty: 0.5,
        }
    }
}

/// The confirmation-led ranker.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ConfirmRanker {
    /// Parameters.
    pub cfg: Config,
}

/// Entry point for the harness (`harness-sim` registry).
#[must_use]
pub fn ranker() -> Box<dyn Ranker> {
    Box::new(ConfirmRanker::default())
}

fn clamp01(x: f64) -> f64 {
    if x.is_nan() {
        0.0
    } else {
        x.clamp(0.0, 1.0)
    }
}

fn opposite(a: Direction, b: Direction) -> bool {
    matches!(
        (a, b),
        (Direction::Up, Direction::Down) | (Direction::Down, Direction::Up)
    )
}

/// The parts of the rank, exposed for tests and reports.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Parts {
    /// Distinct families that confirmed the story.
    pub support: u32,
    /// Distinct families among facts pointing the opposite way.
    pub oppose: u32,
    /// Whether the story's own state ended.
    pub ended: bool,
    /// Net confirmation `support − oppose − penalty·ended`.
    pub confirmation: f64,
    /// Strength term in `[0, 1]`.
    pub strength: f64,
    /// Track-record term in `[0, 1]` (½ = no evidence or average).
    pub track: f64,
    /// Rarity term in `[0, 1]`.
    pub rarity: f64,
    /// Secondary score in `[0, 1)`.
    pub secondary: f64,
}

impl ConfirmRanker {
    /// Track record: the key's realized rate shrunk towards the all-keys rate, as a lift
    /// mapped to `[0, 1]` (½ = same as everything else, or no closed capsules yet).
    #[must_use]
    pub fn track(&self, ctx: &RankCtx) -> f64 {
        let all = (f64::from(ctx.ledger_all.realized) + 1.0) / (f64::from(ctx.ledger_all.n) + 2.0);
        let key = (f64::from(ctx.ledger.realized) + self.cfg.shrink * all)
            / (f64::from(ctx.ledger.n) + self.cfg.shrink);
        clamp01(0.5 + self.cfg.track_gain * (key - all))
    }

    /// Rarity: mean of kind rarity (share of the kind among all stories so far) and
    /// local rarity (this instrument's count of the kind and direction against the
    /// average instrument's).
    #[must_use]
    pub fn rarity(&self, s: &OpenStory, ctx: &RankCtx) -> f64 {
        // Kind share, smoothed over the six kinds.
        let q = (f64::from(ctx.same_kind_all) + 1.0) / (f64::from(ctx.stories_all) + 7.0);
        let kind = clamp01(q.ln() / self.cfg.rare_share.ln());
        // Local: same kind + direction on this instrument vs the average instrument
        // (`same_kind_all` counts both directions of a directional kind).
        let dirs = if s.story.direction == Direction::Neutral {
            1.0
        } else {
            2.0
        };
        let expected = f64::from(ctx.same_kind_all) / (f64::from(ctx.instruments.max(1)) * dirs);
        let rho = (f64::from(ctx.same_kind_on_instrument) + 1.0) / (expected + 1.0);
        let local = clamp01(1.5 - 0.5 * rho);
        0.5 * kind + 0.5 * local
    }

    /// Every part of the rank.
    #[must_use]
    pub fn parts(&self, s: &OpenStory, ctx: &RankCtx) -> Parts {
        let st = &s.story;
        let support = u32::try_from(st.families.len()).unwrap_or(u32::MAX);
        let mut opp: Vec<EvidenceFamily> = Vec::new();
        let mut entered: Vec<&str> = Vec::new();
        let mut ended = false;
        for f in &s.facts {
            let Some(m) = f.meaning.as_ref() else {
                continue;
            };
            if opposite(st.direction, m.direction) && !opp.contains(&m.family) {
                opp.push(m.family);
            }
            match m.edge {
                Edge::Enter => entered.push(m.detector.as_str()),
                Edge::Exit if entered.contains(&m.detector.as_str()) => ended = true,
                _ => {}
            }
        }
        let oppose = u32::try_from(opp.len()).unwrap_or(u32::MAX);
        let confirmation = f64::from(support)
            - f64::from(oppose)
            - if ended { self.cfg.ended_penalty } else { 0.0 };
        let strength = clamp01(st.strength);
        let track = self.track(ctx);
        let rarity = self.rarity(s, ctx);
        let c = &self.cfg;
        let wsum = c.w_strength + c.w_track + c.w_rarity;
        let secondary = if wsum > 0.0 {
            (c.w_strength * strength + c.w_track * track + c.w_rarity * rarity) / wsum
        } else {
            0.0
        };
        Parts {
            support,
            oppose,
            ended,
            confirmation,
            strength,
            track,
            rarity,
            // Strictly below 1 so one more family always outranks secondary evidence.
            secondary: secondary.clamp(0.0, 0.999),
        }
    }

    /// Tier from the parts (see the module table).
    #[must_use]
    pub fn tier(&self, p: &Parts) -> u8 {
        let c = p.confirmation;
        let s = p.secondary;
        if c >= 2.0 {
            1
        } else if c >= 1.0 {
            if s >= self.cfg.theta_1 {
                1
            } else if s >= self.cfg.theta_2 {
                2
            } else {
                3
            }
        } else if c >= 0.5 && s >= self.cfg.theta_1 {
            2
        } else {
            3
        }
    }
}

impl Ranker for ConfirmRanker {
    fn name(&self) -> &'static str {
        "confirm"
    }

    fn rank(&self, s: &OpenStory, ctx: &RankCtx) -> (f64, u8) {
        let p = self.parts(s, ctx);
        (p.confirmation + p.secondary, self.tier(&p))
    }
}

#[cfg(test)]
mod tests;
