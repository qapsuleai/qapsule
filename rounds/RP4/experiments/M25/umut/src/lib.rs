//! RP4/M25 alternative **`umut`** ("umut kesildi" — the claim has become implausible).
//!
//! Same yardstick and horizon as the product rule `r3` ([`qp_capsule::outcome::Barrier`]):
//! `P0` = birth close, `A` = Wilder ATR(14) at birth (from `bars[..=born]` only),
//! `k = 0.3·√h`, per-kind horizon `h`. Neutral stories are judged exactly as in `r3`.
//!
//! The **directional stop** changes. `r3` stops at a fixed `−k·A`, the mirror of the
//! target. `umut` asks instead: *can the claim still come true in the time it has left?*
//! With target `T = M·k·A`, at a bar `t` bars after birth with `τ = h − t ≥ 1` bars left
//! and signed move `d = ±(close − P0)`, the story is **invalidated** when
//!
//! ```text
//! d < 0   and   T − d ≥ Z · A · √τ
//! ```
//!
//! i.e. the price is against the claim and the distance still to go to the target is at
//! least `Z` "remaining-life volatilities" (`A·√τ`, diffusive scaling, the same scaling
//! `r3` uses for its barriers). Under a driftless random walk the chance of still
//! reaching the target from there is small (first passage), so "tutmadı" means "very
//! unlikely to come true in its own life", which is the complement of the early-close
//! measure (`ECR_own`). `d ≥ T` on any bar → **realized** (first event wins).
//!
//! The stop level `T − Z·A·√τ` is far below `P0` early in the story's life and **narrows
//! with time** (it rises towards `P0` as `τ` shrinks). Consequences:
//!
//! - one adverse bar right after birth no longer closes the story (the R6 case: a 5-minute
//!   down claim "failed" on the next bar and the price then crossed the target);
//! - a story that missed the stop by a tick meets a tighter stop on the next bar, so the
//!   verdict is less sensitive to single-bar noise;
//! - on the horizon bar (`τ = 0`) the stop is off: a story that is still undecided
//!   expires (engine, `horizon`) as in `r3`, so no expiry is relabelled as a failure.
//!
//! **Asymmetry and the ledger base.** With the stop alone (`M = 1`) a driftless market
//! would realize ≈ 61 % of the decided cases (stops are harder to hit early). The target
//! is therefore widened by `M` so that the rule's driftless base is again ≈ 50 % of the
//! decided cases per kind, as in `r3` (so the ledger and the K53 coin-flip threshold read
//! against the same base). `M` was set on a synthetic random walk only (`NOTES.md`).
//!
//! Two new constants ([`Z`], [`M`]); everything else is `r3`'s. The verdict at `now` is a
//! pure function of `bars[..=now]` and the story (kind, direction, birth bar): the rule
//! rescans `(born, now]` for the first event. A cache keeps the birth ATR, keyed with the
//! birth bar's own values (as in `r3`) so a reused instance on other data cannot go stale.

use std::cell::RefCell;
use std::collections::BTreeMap;

use qp_capsule::outcome::{base_params, wilder_atr, ATR_FAST, ATR_SLOW};
use qp_capsule::{OpenStory, OutcomeRule, Verdict};
use qp_schema::capsule::{Direction, MeaningKind, TimeKind};
use qp_schema::Bar;

/// Rule name (`--rule`).
pub const NAME: &str = "umut";

/// Implausibility bound in ATR(14) units per `√(bars left)`. Set a priori, before any
/// market-data run and never changed: on a random walk ATR(14) ≈ 1.35 close-to-close σ,
/// so `1.5·A ≈ 2σ` and a driftless walk recovers from the stop with probability ≲ 5 %
/// (`NOTES.md`).
pub const Z: f64 = 1.5;

/// Target scale over `r3`'s `k` (target = `M·k·A`). Set on a synthetic driftless random
/// walk only, so that the rule's null base is 50 % of the decided cases, as in `r3`
/// (`NOTES.md`); not tuned on market data.
pub const M: f64 = 1.45;

/// Verdict of `umut` at bar `now` for a story of `kind` / `dir` born at `born`, from
/// `bars[..=now]` only. `atr` gives the birth yardstick for a period (cached by the rule).
/// `None` = undecided.
fn decide(
    kind: MeaningKind,
    dir: Direction,
    born: usize,
    bars: &[Bar],
    now: usize,
    atr: &dyn Fn(usize) -> Option<f64>,
) -> Option<(Verdict, &'static str)> {
    if now <= born || bars.len() <= now {
        return None;
    }
    let p = base_params(kind);
    let h = p.horizon as usize;
    let p0 = bars[born].c;
    let last = now.min(born + h);
    if dir == Direction::Neutral {
        // As r3: expansion to k_n·ATR100 realizes; none by the horizon invalidates.
        let thr = p.k_neutral * atr(ATR_SLOW)?;
        if bars[born + 1..=last]
            .iter()
            .any(|b| (b.c - p0).abs() >= thr)
        {
            return Some((Verdict::Realized, "barrier.expansion"));
        }
        return (now >= born + h).then_some((Verdict::Invalidated, "barrier.no_expansion"));
    }
    let a = atr(ATR_FAST)?;
    let target = M * p.k * a;
    let sign = if dir == Direction::Up { 1.0 } else { -1.0 };
    for (i, b) in bars[born + 1..=last].iter().enumerate() {
        let d = sign * (b.c - p0);
        if d >= target {
            return Some((Verdict::Realized, "umut.target"));
        }
        let left = h - (i + 1);
        #[allow(clippy::cast_precision_loss)]
        let tau = left as f64;
        if left >= 1 && d < 0.0 && target - d >= Z * a * tau.sqrt() {
            return Some((Verdict::Invalidated, "umut.implausible"));
        }
    }
    None
}

type CacheKey = (String, String, u64, i64, [u64; 3], usize);

/// The `umut` outcome rule.
#[derive(Debug, Default)]
pub struct Umut {
    atr_cache: RefCell<BTreeMap<CacheKey, Option<f64>>>,
}

impl Umut {
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

impl OutcomeRule for Umut {
    fn name(&self) -> &'static str {
        NAME
    }

    fn horizon(&self, kind: MeaningKind, _time: TimeKind) -> u32 {
        base_params(kind).horizon
    }

    fn judge(&self, s: &OpenStory, bars: &[Bar], now: u64) -> Option<(Verdict, &'static str)> {
        let born = usize::try_from(s.story.born_bar).ok()?;
        let now = usize::try_from(now).ok()?;
        if born >= bars.len() {
            return None;
        }
        let atr = |period| self.atr_at_birth(s, bars, born, period);
        decide(s.story.kind, s.story.direction, born, bars, now, &atr)
    }
}

/// A fresh rule instance (harness registry).
#[must_use]
pub fn outcome() -> Box<dyn OutcomeRule> {
    Box::new(Umut::default())
}

/// Levels fixed at birth, from `bars[..=born]`: `(P0, target, stop)`.
///
/// Directional: target `M·k·ATR14`; the stop moves with time, so its birth value
/// is given — the first bar it can act on (`τ = h − 1`): `Z·A·√(h−1) − k·A`.
/// Neutral: target `k_n·ATR100`, no stop (as `r3`).
#[must_use]
pub fn levels(s: &OpenStory, bars: &[Bar]) -> Option<(f64, f64, Option<f64>)> {
    let born = usize::try_from(s.story.born_bar).ok()?;
    let p0 = bars.get(born)?.c;
    let p = base_params(s.story.kind);
    if s.story.direction == Direction::Neutral {
        let a = wilder_atr(&bars[..=born], ATR_SLOW)?;
        return Some((p0, p.k_neutral * a, None));
    }
    let a = wilder_atr(&bars[..=born], ATR_FAST)?;
    let target = M * p.k * a;
    let stop = stop_distance(p.horizon, 1, target, a);
    Some((p0, target, (stop > 0.0).then_some(stop)))
}

/// Stop distance below `P0` (price units, > 0) at `t` bars after birth for horizon `h`;
/// `0` when the stop is off (`t ≥ h`) or would sit at or above `P0` (then `d < 0` binds).
#[must_use]
pub fn stop_distance(h: u32, t: u32, target: f64, atr: f64) -> f64 {
    if t >= h {
        return 0.0;
    }
    let tau = f64::from(h - t);
    (Z * atr * tau.sqrt() - target).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(i: i64, c: f64) -> Bar {
        Bar {
            t: i * 300_000,
            o: c,
            h: c + 0.5,
            l: c - 0.5,
            c,
            v: 1.0,
        }
    }

    /// 30 flat-ish warm-up bars (ATR(14) = 1.0: range 1 around a constant close), then `tail`.
    fn series(tail: &[f64]) -> Vec<Bar> {
        let mut v: Vec<Bar> = (0..30).map(|i| bar(i, 100.0)).collect();
        for (j, &c) in tail.iter().enumerate() {
            v.push(bar(30 + i64::try_from(j).unwrap(), c));
        }
        v
    }

    const BORN: usize = 29;

    fn run(kind: MeaningKind, dir: Direction, bars: &[Bar], now: usize) -> Option<Verdict> {
        let atr = |p| wilder_atr(&bars[..=BORN], p);
        decide(kind, dir, BORN, &bars[..=now], now, &atr).map(|(v, _)| v)
    }

    #[test]
    fn atr_is_one() {
        let b = series(&[]);
        let a = wilder_atr(&b, ATR_FAST).unwrap();
        assert!((a - 1.0).abs() < 1e-12, "{a}");
    }

    #[test]
    fn one_adverse_bar_does_not_stop_early() {
        // Breakout: h = 10, k = 0.3·√10 ≈ 0.949. r3 would stop at d ≤ −0.949 on bar 1.
        // umut at t = 1 needs k − d ≥ 1.5·√9 = 4.5 → d ≤ −3.55.
        let b = series(&[98.5, 101.5]);
        assert_eq!(
            run(MeaningKind::Breakout, Direction::Up, &b, BORN + 1),
            None
        );
        assert_eq!(
            run(MeaningKind::Breakout, Direction::Up, &b, BORN + 2),
            Some(Verdict::Realized)
        );
        // A crash of 4 ATR on bar 1 is implausible to recover.
        let b = series(&[96.0]);
        assert_eq!(
            run(MeaningKind::Breakout, Direction::Up, &b, BORN + 1),
            Some(Verdict::Invalidated)
        );
    }

    #[test]
    fn stop_narrows_with_time_and_is_off_on_horizon_bar() {
        // Down claim, h = 10: price drifts slightly up (against the claim) by 0.6 ATR.
        // At τ = 1: k − d = 0.949 + 0.6 = 1.549 ≥ 1.5 → invalidated on bar 9, not before.
        let tail = [100.6; 10];
        let b = series(&tail);
        for t in 1..=8 {
            assert_eq!(
                run(MeaningKind::Breakout, Direction::Down, &b, BORN + t),
                None,
                "t={t}"
            );
        }
        assert_eq!(
            run(MeaningKind::Breakout, Direction::Down, &b, BORN + 9),
            Some(Verdict::Invalidated)
        );
        // Same, but the adverse move appears only on the horizon bar: no verdict (expiry).
        let mut tail = [100.0; 10];
        tail[9] = 103.0;
        let b = series(&tail);
        assert_eq!(
            run(MeaningKind::Breakout, Direction::Down, &b, BORN + 10),
            None
        );
    }

    #[test]
    fn never_invalidates_on_the_claim_side() {
        // Up claim, price slightly above P0 for the whole life: never invalidated.
        let b = series(&[100.3; 10]);
        for t in 1..=10 {
            assert_eq!(
                run(MeaningKind::Breakout, Direction::Up, &b, BORN + t),
                None
            );
        }
    }

    #[test]
    fn pure_in_now_and_prefix() {
        // The verdict at `now` depends only on bars[..=now]: appending later bars or
        // judging on every bar vs. once gives the same first verdict.
        let tail = [99.0, 98.0, 97.0, 96.0, 95.0, 94.0, 99.0, 104.0];
        let b = series(&tail);
        let mut first = None;
        for t in 1..=tail.len() {
            if let Some(v) = run(MeaningKind::Breakout, Direction::Up, &b, BORN + t) {
                first = Some((t, v));
                break;
            }
        }
        let (t, v) = first.unwrap();
        for extra in t..=tail.len() {
            assert_eq!(
                run(MeaningKind::Breakout, Direction::Up, &b, BORN + extra),
                Some(v)
            );
        }
        assert_eq!(v, Verdict::Invalidated);
    }

    #[test]
    fn neutral_matches_r3() {
        // r3 for neutral: same thresholds; compare on a series with ATR100 warm-up.
        let mut b: Vec<Bar> = (0..120).map(|i| bar(i, 100.0)).collect();
        for (j, c) in [100.5, 101.0, 102.0, 104.0].iter().enumerate() {
            b.push(bar(120 + i64::try_from(j).unwrap(), *c));
        }
        let born = 119;
        let atr = |p| wilder_atr(&b[..=born], p);
        let a100 = atr(ATR_SLOW).unwrap();
        let thr = base_params(MeaningKind::Squeeze).k_neutral * a100;
        for now in born + 1..b.len() {
            let got = decide(
                MeaningKind::Squeeze,
                Direction::Neutral,
                born,
                &b,
                now,
                &atr,
            );
            let crossed = b[born + 1..=now].iter().any(|x| (x.c - 100.0).abs() >= thr);
            assert_eq!(got.is_some(), crossed, "now={now}");
        }
    }

    #[test]
    fn stop_distance_profile() {
        let k = 0.3 * 10f64.sqrt();
        let s1 = stop_distance(10, 1, k, 1.0);
        let s9 = stop_distance(10, 9, k, 1.0);
        assert!(s1 > s9);
        assert!((s1 - (4.5 - k)).abs() < 1e-12);
        assert!((s9 - (1.5 - k)).abs() < 1e-12);
        assert!(stop_distance(10, 10, k, 1.0).abs() < f64::EPSILON);
    }
}
