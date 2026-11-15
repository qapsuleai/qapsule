//! R3 M3 candidate **"ledger"** (karne): a story is as important as its ledger key
//! (`kind/detector@version`) has proven to come true in the past, with its sampling
//! uncertainty.
//!
//! Rule (all numbers come from [`RankCtx`], i.e. ledger rows closed strictly before the
//! ranked bar; no look-ahead, no state, no clock):
//!
//! 1. **Reference rate** `b`: hit rate of the *other* keys (`ledger_all − ledger`), smoothed
//!    with a Beta(1, 1); `0.5` when nothing has closed yet.
//! 2. **Posterior** of the key's hit rate: Beta prior centred on `b` with weight
//!    [`Params::prior_n`] pseudo-closes (`α₀ = k·b`, `β₀ = k·(1 − b)`), updated with the
//!    key's `realized` of `n`. A new key therefore starts *at the reference rate*, never
//!    above it: it has to earn tier 1 with closes (cold start = caution).
//! 3. **Credible band** `[lo, hi] = mean ∓ z·sd` (normal approximation of the Beta
//!    posterior; `z` = [`Params::z`]).
//! 4. **Tier**:
//!    - 1 if `lo ≥ b + δ`: the key comes true clearly (by at least `δ`) more often than the
//!      rest, and the sample is large enough to say so;
//!    - 3 if `hi < b`: the key comes true demonstrably less often than the rest;
//!    - 2 otherwise (no evidence either way, including cold start).
//! 5. **Importance** = `lo` (the cautious hit rate, in `[0, 1]`): orders stories inside a
//!    tier; the realizer can say "came true at least this often".
//!
//! Notes and measurements: `NOTES.md` next to this crate.

use qp_capsule::{OpenStory, RankCtx, Ranker};
use qp_schema::capsule::LedgerStat;

/// Parameters of the rule.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Params {
    /// Prior weight in pseudo-closes (cold-start caution).
    pub prior_n: f64,
    /// Width of the credible band in posterior standard deviations (one-sided).
    pub z: f64,
    /// Minimum lift of the cautious rate over the reference rate for tier 1.
    pub delta: f64,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            prior_n: 50.0,
            z: 2.326,
            delta: 0.0,
        }
    }
}

/// The ranker.
#[derive(Debug, Clone, Copy, Default)]
pub struct LedgerRanker {
    /// Parameters.
    pub p: Params,
}

/// What the rule derived from the ledger (exposed for tests and the notes).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Assessment {
    /// Reference rate of the other keys.
    pub base: f64,
    /// Posterior mean of the key.
    pub mean: f64,
    /// Lower end of the credible band.
    pub lo: f64,
    /// Upper end of the credible band.
    pub hi: f64,
    /// Tier 1…3.
    pub tier: u8,
}

impl LedgerRanker {
    /// Assess a key from its ledger and the ledger over all keys.
    #[must_use]
    pub fn assess(&self, key: LedgerStat, all: LedgerStat) -> Assessment {
        // Defensive: the key's rows are part of `all`; tolerate inconsistent input.
        let kn = key.n.min(all.n);
        let kr = key.realized.min(kn);
        let rest_n = f64::from(all.n - kn);
        let rest_r = f64::from(all.realized.saturating_sub(kr).min(all.n - kn));
        let base = (rest_r + 1.0) / (rest_n + 2.0);

        let k = self.p.prior_n.max(1e-9);
        let a = k * base + f64::from(kr);
        let b = k * (1.0 - base) + f64::from(kn - kr);
        let s = a + b;
        let mean = a / s;
        let sd = (a * b / (s * s * (s + 1.0))).sqrt();
        let lo = (mean - self.p.z * sd).clamp(0.0, 1.0);
        let hi = (mean + self.p.z * sd).clamp(0.0, 1.0);

        let tier = if lo >= base + self.p.delta {
            1
        } else if hi < base {
            3
        } else {
            2
        };
        Assessment {
            base,
            mean,
            lo,
            hi,
            tier,
        }
    }
}

impl Ranker for LedgerRanker {
    fn name(&self) -> &'static str {
        "m3.ledger"
    }

    fn rank(&self, _s: &OpenStory, ctx: &RankCtx) -> (f64, u8) {
        let a = self.assess(ctx.ledger, ctx.ledger_all);
        (a.lo, a.tier)
    }
}

/// Entry point for the harness.
#[must_use]
pub fn ranker() -> Box<dyn Ranker> {
    Box::new(LedgerRanker::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn st(n: u32, realized: u32) -> LedgerStat {
        LedgerStat { n, realized }
    }

    #[test]
    fn cold_start_is_tier_two_at_reference() {
        let r = LedgerRanker::default();
        let a = r.assess(st(0, 0), st(0, 0));
        assert_eq!(a.tier, 2);
        assert!((a.mean - 0.5).abs() < 1e-12);
        // New key in a mature ledger: starts at the others' rate, not above it.
        let a = r.assess(st(0, 0), st(1000, 470));
        assert_eq!(a.tier, 2);
        assert!((a.mean - a.base).abs() < 1e-12);
    }

    #[test]
    fn small_perfect_sample_is_not_tier_one() {
        let r = LedgerRanker::default();
        // 3 of 3 realized: too few to claim.
        assert_ne!(r.assess(st(3, 3), st(1003, 473)).tier, 1);
        // 60 of 60 realized: proven.
        assert_eq!(r.assess(st(60, 57), st(1060, 527)).tier, 1);
    }

    #[test]
    fn proven_weak_key_is_tier_three() {
        let r = LedgerRanker::default();
        // 430 of 1000 vs the rest 490 of 1000.
        assert_eq!(r.assess(st(1000, 430), st(2000, 920)).tier, 3);
        // Same rate on 20 closes: not proven.
        assert_eq!(r.assess(st(20, 9), st(1020, 499)).tier, 2);
    }

    #[test]
    fn tier_and_importance_in_range_on_a_grid() {
        let r = LedgerRanker::default();
        for an in [0_u32, 1, 2, 10, 100, 5000] {
            for ar in 0..=an.min(50) {
                let ar = ar * an / an.clamp(1, 50);
                for kn in [0_u32, 1, 5, 50, 4000, 6000] {
                    for kr in [0_u32, 1, 3, 40, 3000, 6000] {
                        let a = r.assess(st(kn, kr), st(an, ar));
                        assert!((1..=3).contains(&a.tier));
                        assert!(a.lo.is_finite() && (0.0..=1.0).contains(&a.lo));
                        assert!(a.lo <= a.mean && a.mean <= a.hi);
                    }
                }
            }
        }
    }

    #[test]
    fn deterministic_and_monotone() {
        let r = LedgerRanker::default();
        let x = r.assess(st(200, 120), st(2000, 950));
        let y = r.assess(st(200, 120), st(2000, 950));
        assert_eq!(x, y);
        assert_eq!(x.lo.to_bits(), y.lo.to_bits());
        // More realized on the same sample never lowers importance.
        let mut prev = -1.0;
        for kr in 0..=200 {
            let a = r.assess(st(200, kr), st(2000, 950 + kr));
            assert!(a.lo >= prev - 1e-12);
            prev = a.lo;
        }
    }
}
