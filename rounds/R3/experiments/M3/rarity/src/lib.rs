//! R3 M3 candidate **"nadirlik" (rarity / surprise)**.
//!
//! Premise: a story is as important as it is *rare*. The score is a surprise in nats,
//! `S = −ln p`, where `p` is the no-look-ahead probability, per bar of this instrument,
//! of seeing a story like this one. It is the sum of three independent parts:
//!
//! 1. **Rate surprise** — how often stories of this kind and direction are born on this
//!    instrument, shrunk towards the market rate of the same kind (empirical Bayes), and
//!    narrowed to the story's detector by the detector's share of its kind in the ledger:
//!    `p_rate = p_inst(kind, dir) × share(detector | kind)`.
//! 2. **Magnitude surprise** — among events of that kind, a stronger one is rarer.
//!    Detector strengths measure distance past the trigger threshold; an exponential tail
//!    gives `P(strength ≥ s | event) ≈ e^(−κ·s)`, so it adds `κ·s` nats. Strengths
//!    saturate often (a third or more of some detectors' events sit at 1), so κ is kept
//!    at 1: strength 1 ≈ "about a third of events of this kind".
//! 3. **Joint surprise** — every further independent evidence family (K30) supporting the
//!    story is a co-occurrence; under independence it multiplies `p` by the chance that
//!    another story-producing fact lands on this instrument within the co-occurrence window.
//!
//! The surprise is read as an equivalent recurrence `N = e^S` in bars ("an event like this
//! comes once every N bars on this instrument"). Tiers are thresholds on `N`:
//! tier 1 when `N ≥ 150` bars (rarer than about twice a year on daily bars), tier 2
//! when `N ≥ 50` (rarer than about once a quarter), else tier 3. `NOTES.md` gives the
//! attention budget behind them.
//!
//! Only [`RankCtx`] and the story itself are read (no look-ahead, no state, no clock):
//! the same story with the same context always gets the same `(importance, tier)`.

use qp_capsule::{OpenStory, RankCtx, Ranker};
use qp_schema::capsule::Direction;

/// Parameters of the rarity ranker (all in bars or nats).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Params {
    /// Pseudo-bars of the market prior in the instrument rate (shrinkage strength).
    pub shrink_bars: f64,
    /// Pseudo instrument-bars of the fixed prior in the market rate (cold start).
    pub prior_bars: f64,
    /// Fixed prior rate per instrument-bar of one kind and direction (cold start).
    pub prior_rate: f64,
    /// Tail constant κ of the magnitude surprise (nats at strength 1).
    pub kappa: f64,
    /// Co-occurrence window in bars for the joint surprise.
    pub joint_window: f64,
    /// Recurrence (bars) at or above which a story is tier 1.
    pub tier1_bars: f64,
    /// Recurrence (bars) at or above which a story is tier 2.
    pub tier2_bars: f64,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            shrink_bars: 120.0,
            prior_bars: 200.0,
            prior_rate: 0.02,
            kappa: 1.0,
            joint_window: 5.0,
            tier1_bars: 150.0,
            tier2_bars: 50.0,
        }
    }
}

/// The rarity ranker.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Rarity {
    /// Parameters.
    pub p: Params,
}

/// Parts of the surprise (nats), for tests and reports.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Surprise {
    /// `−ln p_rate`.
    pub rate: f64,
    /// `κ·strength`.
    pub magnitude: f64,
    /// Extra families.
    pub joint: f64,
}

impl Surprise {
    /// Total surprise in nats.
    #[must_use]
    pub fn total(&self) -> f64 {
        self.rate + self.magnitude + self.joint
    }
}

const P_MIN: f64 = 1e-6;

impl Rarity {
    /// Surprise parts of a story in its context.
    #[must_use]
    pub fn surprise(&self, s: &OpenStory, ctx: &RankCtx) -> Surprise {
        let p = &self.p;
        let bars = ctx.bars_seen.max(1) as f64;
        let inst = f64::from(ctx.instruments.max(1));
        let exposure = inst * bars; // instrument-bars seen by the market (approx.)

        // Direction share: a directional story is one of two directions of its kind.
        let dir_share = if s.story.direction == Direction::Neutral {
            1.0
        } else {
            0.5
        };

        // Market rate of this kind (and direction) per instrument-bar, with a cold prior.
        let mkt = (f64::from(ctx.same_kind_all) * dir_share + p.prior_bars * p.prior_rate)
            / (exposure + p.prior_bars);

        // Instrument rate, shrunk towards the market rate.
        let inst_rate =
            (f64::from(ctx.same_kind_on_instrument) + p.shrink_bars * mkt) / (bars + p.shrink_bars);

        // Detector share within its kind, from closed stories (ledger) — strictly past.
        // det births ≈ (ledger.n / ledger_all.n) × stories_all; kind births = same_kind_all.
        let share = if ctx.ledger_all.n == 0 || ctx.same_kind_all == 0 {
            1.0
        } else {
            let det_births = (f64::from(ctx.ledger.n) + 0.5) / (f64::from(ctx.ledger_all.n) + 1.0)
                * f64::from(ctx.stories_all);
            (det_births / f64::from(ctx.same_kind_all)).clamp(0.05, 1.0)
        };

        let p_rate = (inst_rate * share).clamp(P_MIN, 1.0);
        let rate = -p_rate.ln();

        let magnitude = p.kappa * s.story.strength.clamp(0.0, 1.0);

        // Any-story rate per instrument-bar (cold prior: six kinds × prior rate).
        let any_rate = (f64::from(ctx.stories_all) + p.prior_bars * p.prior_rate * 6.0)
            / (exposure + p.prior_bars);
        let q = (any_rate * p.joint_window).clamp(0.01, 0.9);
        let extra = s.story.families.len().saturating_sub(1) as f64;
        let joint = extra * -q.ln();

        Surprise {
            rate,
            magnitude,
            joint,
        }
    }

    /// Tier from a surprise in nats.
    #[must_use]
    pub fn tier_of(&self, nats: f64) -> u8 {
        if nats >= self.p.tier1_bars.ln() {
            1
        } else if nats >= self.p.tier2_bars.ln() {
            2
        } else {
            3
        }
    }
}

impl Ranker for Rarity {
    fn name(&self) -> &'static str {
        "rarity"
    }

    fn rank(&self, s: &OpenStory, ctx: &RankCtx) -> (f64, u8) {
        let nats = self.surprise(s, ctx).total();
        let nats = if nats.is_finite() { nats } else { 0.0 };
        (nats, self.tier_of(nats))
    }
}

/// Entry point for the harness (`harness-sim`, M3).
#[must_use]
pub fn ranker() -> Box<dyn qp_capsule::Ranker> {
    Box::new(Rarity::default())
}

#[cfg(test)]
mod tests;
