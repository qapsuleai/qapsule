//! RP4/M25 alternative **`teyit2`** — "iki kapanışla teyit" (confirmation by two closes).
//!
//! Same barrier as the reference `r3` (`qp_capsule::outcome::Barrier`): reference price
//! `P0` = birth close, yardstick ATR(14) at birth from `bars[..=born]`, distance
//! `thr = K_SCALE · 0.3·√h · ATR14` (`K_SCALE = 1.25`), per-kind horizon `h` unchanged.
//! What changes is how
//! many **consecutive** closes beyond the barrier a verdict needs:
//!
//! - **stop** (`invalidated`, "tutmadı"): `STOP_CLOSES` consecutive closes with
//!   `d = ±(close − P0) ≤ −thr` (`r3`: one);
//! - **target** (`realized`): `TARGET_CLOSES` consecutive closes with `d ≥ thr`.
//!
//! A run is broken by any close back inside the barrier (or on the other side). A run
//! may only start after the birth bar (the birth close is `P0` itself, `d = 0`).
//!
//! **Horizon end.** Only bars in `(born, born + h]` count. If the last bar of the
//! horizon is the first close of a run (the confirmation would need bar `born + h + 1`),
//! there is no verdict and the engine closes the story as `expired`: the claim was
//! neither confirmed nor refuted inside its own life. No grace bar is added (the horizon,
//! hence the ledger and `ECR` windows, stay the same as `r3`'s).
//!
//! **Neutral stories** (squeeze, neutral flow) are judged exactly like `r3`: the first
//! close with `|close − P0| ≥ 0.65·√h·ATR100` → realized, none by the horizon →
//! invalidated (`no_expansion`). Their "stop" is the horizon itself, so the noise problem
//! this rule addresses does not apply to them.
//!
//! The verdict at `now` is a pure function of the story (kind, direction, birth bar) and
//! `bars[..=now]`: the rule rescans `(born, now]` each call. The only state is a per-rule
//! cache of the birth ATR whose key includes the birth bar's values (as in `r3`), so a
//! reused instance on other data cannot return a stale value. No clock, network, RNG or
//! files.

use std::cell::RefCell;
use std::collections::BTreeMap;

use qp_capsule::outcome::{base_params, wilder_atr, ATR_FAST, ATR_SLOW};
use qp_capsule::{OpenStory, OutcomeRule, Verdict};
use qp_schema::capsule::Direction;
use qp_schema::Bar;

/// Consecutive closes beyond the stop needed for `invalidated`.
pub const STOP_CLOSES: u32 = 2;
/// Consecutive closes beyond the target needed for `realized`.
pub const TARGET_CLOSES: u32 = 2;
/// Barrier multiplier relative to `r3` (`1.0` = the same `0.3·√h·ATR14`; chosen `1.25`, i.e. `0.375·√h·ATR14`; NOTES.md).
pub const K_SCALE: f64 = 1.25;

/// Outcome of scanning a directional story's signed moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scan {
    /// Target confirmed (index into the scanned slice of the confirming close).
    Target(usize),
    /// Stop confirmed (index of the confirming close).
    Stop(usize),
    /// Neither.
    Open,
}

/// The pure core: `d[i]` are the signed moves `±(close − P0)` of bars `born+1, born+2, …`
/// (already cut at the horizon and at `now`). The first index where `target_n`
/// consecutive `d ≥ thr` or `stop_n` consecutive `d ≤ −thr` end decides.
#[must_use]
pub fn scan(d: &[f64], thr: f64, stop_n: u32, target_n: u32) -> Scan {
    let (mut up, mut dn) = (0u32, 0u32);
    for (i, &x) in d.iter().enumerate() {
        if x >= thr {
            up += 1;
            dn = 0;
        } else if x <= -thr {
            dn += 1;
            up = 0;
        } else {
            up = 0;
            dn = 0;
        }
        if up >= target_n {
            return Scan::Target(i);
        }
        if dn >= stop_n {
            return Scan::Stop(i);
        }
    }
    Scan::Open
}

type CacheKey = (String, String, u64, i64, [u64; 3], usize);

/// The rule. `stop_n`, `target_n`, `k_scale` are fixed per instance.
#[derive(Debug)]
pub struct Confirm {
    label: &'static str,
    stop_n: u32,
    target_n: u32,
    k_scale: f64,
    atr_cache: RefCell<BTreeMap<CacheKey, Option<f64>>>,
}

impl Confirm {
    /// A rule with explicit constants (trials; the candidate is [`outcome`]).
    #[must_use]
    pub fn new(label: &'static str, stop_n: u32, target_n: u32, k_scale: f64) -> Self {
        Self {
            label,
            stop_n: stop_n.max(1),
            target_n: target_n.max(1),
            k_scale,
            atr_cache: RefCell::new(BTreeMap::new()),
        }
    }

    fn atr_at_birth(&self, s: &OpenStory, bars: &[Bar], born: usize, period: usize) -> Option<f64> {
        let b = bars.get(born)?;
        let key = (
            s.story.instrument.clone(),
            s.story.series.clone(),
            s.story.born_bar,
            b.t,
            [b.h.to_bits(), b.l.to_bits(), b.c.to_bits()],
            period,
        );
        if let Some(v) = self.atr_cache.borrow().get(&key) {
            return *v;
        }
        let v = wilder_atr(&bars[..=born], period);
        self.atr_cache.borrow_mut().insert(key, v);
        v
    }
}

impl OutcomeRule for Confirm {
    fn name(&self) -> &'static str {
        self.label
    }

    fn horizon(
        &self,
        kind: qp_schema::capsule::MeaningKind,
        _time: qp_schema::capsule::TimeKind,
    ) -> u32 {
        base_params(kind).horizon
    }

    fn judge(&self, s: &OpenStory, bars: &[Bar], now: u64) -> Option<(Verdict, &'static str)> {
        let born = usize::try_from(s.story.born_bar).ok()?;
        let now_i = usize::try_from(now).ok()?;
        if now_i <= born || bars.len() <= now_i {
            return None;
        }
        let p = base_params(s.story.kind);
        let h = p.horizon as usize;
        let p0 = bars[born].c;
        let last = now_i.min(born + h);
        match s.story.direction {
            Direction::Neutral => {
                let a = self.atr_at_birth(s, bars, born, ATR_SLOW)?;
                let thr = p.k_neutral * a;
                if bars[born + 1..=last]
                    .iter()
                    .any(|b| (b.c - p0).abs() >= thr)
                {
                    return Some((Verdict::Realized, "teyit2.expansion"));
                }
                (now_i >= born + h).then_some((Verdict::Invalidated, "teyit2.no_expansion"))
            }
            dir => {
                let a = self.atr_at_birth(s, bars, born, ATR_FAST)?;
                let thr = self.k_scale * p.k * a;
                let sign = if dir == Direction::Up { 1.0 } else { -1.0 };
                let d: Vec<f64> = bars[born + 1..=last]
                    .iter()
                    .map(|b| sign * (b.c - p0))
                    .collect();
                match scan(&d, thr, self.stop_n, self.target_n) {
                    Scan::Target(_) => Some((Verdict::Realized, "teyit2.target")),
                    Scan::Stop(_) => Some((Verdict::Invalidated, "teyit2.stop")),
                    Scan::Open => None,
                }
            }
        }
    }
}

/// The candidate (fresh instance each call).
#[must_use]
pub fn outcome() -> Box<dyn OutcomeRule> {
    Box::new(Confirm::new("teyit2", STOP_CLOSES, TARGET_CLOSES, K_SCALE))
}

/// Levels fixed at birth (`bars` = `bars[..=born]`): `(P0, target, stop)`; directional
/// `K_SCALE·k·ATR14` both ways, neutral `k_n·ATR100` and no stop.
#[must_use]
pub fn levels(s: &OpenStory, bars: &[Bar]) -> Option<(f64, f64, Option<f64>)> {
    levels_with(s, bars, K_SCALE)
}

/// [`levels`] with an explicit barrier multiplier (trials).
#[must_use]
pub fn levels_with(s: &OpenStory, bars: &[Bar], k_scale: f64) -> Option<(f64, f64, Option<f64>)> {
    let born = usize::try_from(s.story.born_bar).ok()?;
    let p0 = bars.get(born)?.c;
    let p = base_params(s.story.kind);
    if s.story.direction == Direction::Neutral {
        let a = wilder_atr(&bars[..=born], ATR_SLOW)?;
        Some((p0, p.k_neutral * a, None))
    } else {
        let a = wilder_atr(&bars[..=born], ATR_FAST)?;
        let x = k_scale * p.k * a;
        Some((p0, x, Some(x)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_close_beyond_the_stop_is_not_enough() {
        assert_eq!(scan(&[0.2, -1.2, -0.5, 0.3], 1.0, 2, 2), Scan::Open);
        assert_eq!(scan(&[0.2, -1.2, -1.0], 1.0, 2, 2), Scan::Stop(2));
    }

    #[test]
    fn runs_reset_on_a_close_inside_or_across() {
        // -1.1, inside, -1.1, +1.1, -1.1: no two consecutive anywhere.
        assert_eq!(scan(&[-1.1, 0.0, -1.1, 1.1, -1.1], 1.0, 2, 2), Scan::Open);
        assert_eq!(scan(&[1.1, 0.9, 1.1, 1.0], 1.0, 2, 2), Scan::Target(3));
    }

    #[test]
    fn one_close_target_when_target_n_is_one() {
        assert_eq!(scan(&[-1.5, 1.0], 1.0, 2, 1), Scan::Target(1));
        assert_eq!(scan(&[-1.5, -1.0], 1.0, 2, 1), Scan::Stop(1));
    }

    #[test]
    fn r3_equivalent_with_one_close_each_way() {
        assert_eq!(scan(&[0.1, -1.0], 1.0, 1, 1), Scan::Stop(1));
        assert_eq!(scan(&[0.1, 1.0], 1.0, 1, 1), Scan::Target(1));
    }

    #[test]
    fn pending_first_close_at_the_end_is_open() {
        // The caller cuts the slice at the horizon: a lone last close decides nothing.
        assert_eq!(scan(&[0.0, 0.5, -1.3], 1.0, 2, 2), Scan::Open);
    }

    #[test]
    fn candidate_constants() {
        let r = outcome();
        assert_eq!(r.name(), "teyit2");
        assert_eq!((STOP_CLOSES, TARGET_CLOSES), (2, 2));
        assert!((K_SCALE - 1.25).abs() < 1e-12);
    }
}
