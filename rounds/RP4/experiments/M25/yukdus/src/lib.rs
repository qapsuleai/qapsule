//! RP4 M25 alternative `yukdus`: **decide on the bar's high / low instead of its close**.
//!
//! Same frame as the product rule `r3` (`qp_capsule::outcome::Barrier`): reference
//! price `P0` = birth close, yardstick = Wilder ATR at the birth bar (ATR(14) directional,
//! ATR(100) neutral) from `bars[..=born]` only, per-kind horizon and diffusive barrier
//! size (`k = C_DIR·√h`, `k_n = C_NEUTRAL·√h`). Two differences:
//!
//! 1. **Touch, not close.** A directional story (sign `s` = +1 up, −1 down) at bar `j`
//!    has a favourable excursion `f = s>0 ? h − P0 : P0 − l` and an adverse excursion
//!    `a = s>0 ? P0 − l : h − P0`. The first bar in `(born, now]` with `f ≥ T` realizes
//!    (`yukdus.target`), with `a ≥ T` invalidates (`yukdus.stop`). A neutral story realizes
//!    at the first bar with `max(h − P0, P0 − l) ≥ T_n` (`yukdus.expansion`); at the
//!    horizon without it, it is invalidated (`yukdus.no_expansion`).
//! 2. **Wider barrier.** Touching is easier than closing beyond, so the barrier is scaled
//!    by [`S_DIR`] / [`S_NEU`] (tuned on the calibration range only; `NOTES.md`).
//!
//! **Tie rule** (one bar touches both target and stop). OHLC does not say which extreme
//! came first. The rule uses the only within-bar information about the bar's final
//! state, its close: `s·(c − P0) > 0` → realized (`yukdus.tie_target`), otherwise
//! invalidated (`yukdus.tie_stop`; a close exactly at `P0` did not move the claim's
//! way, so it does not count as holding). Rationale: always taking the stop (the usual
//! "pessimistic" backtest choice) biases `invalidated` upward, which is exactly the early
//! close M25 tries to cut; always taking the target is the mirror bias. The close rule is
//! symmetric between up and down claims and deterministic.
//!
//! The verdict at `now` is a pure function of `(story, bars[..=now])`: the rule rescans
//! `(born, min(now, born + h)]`. The birth ATR is cached per instance with a key that
//! includes the birth bar's own values (as `Barrier` does), so a reused instance on other
//! data cannot return a stale yardstick.

use std::cell::RefCell;
use std::collections::BTreeMap;

use qp_capsule::outcome::{base_params, wilder_atr, KindParams, ATR_FAST, ATR_SLOW};
use qp_capsule::{OpenStory, OutcomeRule, Verdict};
use qp_schema::capsule::{Direction, MeaningKind, TimeKind};
use qp_schema::Bar;

/// Scale on the directional barrier of `r3` (`k = S_DIR · 0.3·√h`, ATR(14)).
pub const S_DIR: f64 = 2.0;
/// Scale on the neutral barrier of `r3` (`k_n = S_NEU · 0.65·√h`, ATR(100)).
pub const S_NEU: f64 = 1.3;

type CacheKey = (String, String, u64, i64, [u64; 3], usize);

/// The high / low touch rule.
#[derive(Debug)]
pub struct Yukdus {
    label: &'static str,
    s_dir: f64,
    s_neu: f64,
    atr_cache: RefCell<BTreeMap<CacheKey, Option<f64>>>,
}

impl Yukdus {
    /// With barrier scales (directional, neutral) on `r3`'s barriers.
    #[must_use]
    pub fn new(label: &'static str, s_dir: f64, s_neu: f64) -> Self {
        Self {
            label,
            s_dir,
            s_neu,
            atr_cache: RefCell::new(BTreeMap::new()),
        }
    }

    /// Effective parameters for a kind (horizon of `r3`, scaled barriers).
    #[must_use]
    pub fn params(&self, kind: MeaningKind) -> KindParams {
        let b = base_params(kind);
        KindParams {
            horizon: b.horizon,
            k: b.k * self.s_dir,
            k_neutral: b.k_neutral * self.s_neu,
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

impl Default for Yukdus {
    /// The candidate (scales [`S_DIR`], [`S_NEU`]).
    fn default() -> Self {
        Self::new("yukdus", S_DIR, S_NEU)
    }
}

impl OutcomeRule for Yukdus {
    fn name(&self) -> &'static str {
        self.label
    }

    fn horizon(&self, kind: MeaningKind, _time: TimeKind) -> u32 {
        self.params(kind).horizon
    }

    fn judge(&self, s: &OpenStory, bars: &[Bar], now: u64) -> Option<(Verdict, &'static str)> {
        let born = usize::try_from(s.story.born_bar).ok()?;
        let now_i = usize::try_from(now).ok()?;
        if now_i <= born || bars.len() <= now_i {
            return None;
        }
        let p = self.params(s.story.kind);
        let p0 = bars[born].c;
        let last = now_i.min(born + p.horizon as usize);
        match s.story.direction {
            Direction::Neutral => {
                let thr = p.k_neutral * self.atr_at_birth(s, bars, born, ATR_SLOW)?;
                for b in &bars[born + 1..=last] {
                    if (b.h - p0).max(p0 - b.l) >= thr {
                        return Some((Verdict::Realized, "yukdus.expansion"));
                    }
                }
                (now_i >= born + p.horizon as usize)
                    .then_some((Verdict::Invalidated, "yukdus.no_expansion"))
            }
            dir => {
                let thr = p.k * self.atr_at_birth(s, bars, born, ATR_FAST)?;
                let up = dir == Direction::Up;
                for b in &bars[born + 1..=last] {
                    let (fav, adv) = if up {
                        (b.h - p0, p0 - b.l)
                    } else {
                        (p0 - b.l, b.h - p0)
                    };
                    match (fav >= thr, adv >= thr) {
                        (true, false) => return Some((Verdict::Realized, "yukdus.target")),
                        (false, true) => return Some((Verdict::Invalidated, "yukdus.stop")),
                        (true, true) => {
                            let dc = if up { b.c - p0 } else { p0 - b.c };
                            return Some(if dc > 0.0 {
                                (Verdict::Realized, "yukdus.tie_target")
                            } else {
                                (Verdict::Invalidated, "yukdus.tie_stop")
                            });
                        }
                        (false, false) => {}
                    }
                }
                None
            }
        }
    }
}

/// A fresh candidate instance (harness registry).
#[must_use]
pub fn outcome() -> Box<dyn OutcomeRule> {
    Box::new(Yukdus::default())
}

/// A rule with scales in percent (`D` directional, `N` neutral); for calibration trials
/// (`NOTES.md`), registered under a trial name only while trying.
#[must_use]
pub fn outcome_pct<const D: u32, const N: u32>() -> Box<dyn OutcomeRule> {
    Box::new(Yukdus::new(
        "yukdus",
        f64::from(D) / 100.0,
        f64::from(N) / 100.0,
    ))
}

/// [`levels`] for [`outcome_pct`].
#[must_use]
pub fn levels_pct<const D: u32, const N: u32>(
    s: &OpenStory,
    bars: &[Bar],
) -> Option<(f64, f64, Option<f64>)> {
    levels_with(
        &Yukdus::new("yukdus", f64::from(D) / 100.0, f64::from(N) / 100.0),
        s,
        bars,
    )
}

/// Levels fixed at birth: `(P0, target, stop)` in price units, from `bars[..=born]`.
/// Directional: target = stop = `S_DIR·k·ATR14`; neutral: target `S_NEU·k_n·ATR100`.
#[must_use]
pub fn levels(s: &OpenStory, bars: &[Bar]) -> Option<(f64, f64, Option<f64>)> {
    levels_with(&Yukdus::default(), s, bars)
}

/// [`levels`] for a rule instance with other scales.
#[must_use]
pub fn levels_with(r: &Yukdus, s: &OpenStory, bars: &[Bar]) -> Option<(f64, f64, Option<f64>)> {
    let born = usize::try_from(s.story.born_bar).ok()?;
    let p0 = bars.get(born)?.c;
    let p = r.params(s.story.kind);
    if s.story.direction == Direction::Neutral {
        let a = wilder_atr(&bars[..=born], ATR_SLOW)?;
        Some((p0, p.k_neutral * a, None))
    } else {
        let a = wilder_atr(&bars[..=born], ATR_FAST)?;
        Some((p0, p.k * a, Some(p.k * a)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qp_schema::capsule::{EvidenceFamily, LedgerStat, Stage, Story, Trust};

    fn story(kind: MeaningKind, dir: Direction, born: u64) -> OpenStory {
        OpenStory {
            id: "cap.T.x.0".into(),
            story: Story {
                instrument: "T".into(),
                theme: "x".into(),
                kind,
                direction: dir,
                time: TimeKind::Event,
                stage: Stage::Born,
                families: vec![EvidenceFamily::Price],
                strength: 0.5,
                importance: 1.0,
                tier: 1,
                series: "px".into(),
                born_bar: born,
                last_bar: born,
                horizon_bars: 10,
                ledger_key: "k".into(),
                ledger_at_birth: LedgerStat::default(),
                trust: Trust::Replica,
                history: Vec::new(),
                condition: None,
                resolution: None,
            },
            facts: Vec::new(),
        }
    }

    fn bar(i: usize, o: f64, h: f64, l: f64, c: f64) -> Bar {
        Bar {
            t: i64::try_from(i).unwrap() * 300_000,
            o,
            h,
            l,
            c,
            v: 1.0,
        }
    }

    /// 150 flat bars with range 1 (ATR = 1 for both periods), close 100.
    fn base() -> Vec<Bar> {
        (0..150)
            .map(|i| bar(i, 100.0, 100.5, 99.5, 100.0))
            .collect()
    }

    /// Breakout: h = 10, directional threshold 1.0·0.3·√10 ≈ 0.949 with scale 1.
    fn rule() -> Yukdus {
        Yukdus::new("t", 1.0, 1.0)
    }

    #[test]
    fn high_touch_realizes_where_close_does_not() {
        let mut b = base();
        let born = 140;
        b.push(bar(150, 100.0, 101.0, 99.9, 100.1)); // high +1.0 ≥ 0.949, close +0.1
        let s = story(MeaningKind::Breakout, Direction::Up, born);
        let r = rule();
        // bar 141 is the base flat bar: range ±0.5 < 0.949 → undecided.
        assert_eq!(r.judge(&s, &b[..=141], 141), None);
        let n = b.len() - 1;
        let s2 = story(MeaningKind::Breakout, Direction::Up, n as u64 - 1);
        assert_eq!(
            r.judge(&s2, &b, n as u64),
            Some((Verdict::Realized, "yukdus.target"))
        );
        // The close-based reference does not decide on that bar.
        let r3 = qp_capsule::outcome::Barrier::default();
        assert_eq!(r3.judge(&s2, &b, n as u64), None);
    }

    #[test]
    fn low_touch_invalidates_up_and_realizes_down() {
        let mut b = base();
        b.push(bar(150, 100.0, 100.2, 99.0, 99.9));
        let n = b.len() - 1;
        let up = story(MeaningKind::Breakout, Direction::Up, n as u64 - 1);
        let dn = story(MeaningKind::Breakout, Direction::Down, n as u64 - 1);
        assert_eq!(
            rule().judge(&up, &b, n as u64),
            Some((Verdict::Invalidated, "yukdus.stop"))
        );
        assert_eq!(
            rule().judge(&dn, &b, n as u64),
            Some((Verdict::Realized, "yukdus.target"))
        );
    }

    #[test]
    fn tie_goes_by_close_side() {
        for (c, want) in [
            (100.3, (Verdict::Realized, "yukdus.tie_target")),
            (99.7, (Verdict::Invalidated, "yukdus.tie_stop")),
            (100.0, (Verdict::Invalidated, "yukdus.tie_stop")),
        ] {
            let mut b = base();
            b.push(bar(150, 100.0, 101.5, 98.5, c));
            let n = b.len() - 1;
            let up = story(MeaningKind::Breakout, Direction::Up, n as u64 - 1);
            assert_eq!(rule().judge(&up, &b, n as u64), Some(want), "c={c}");
            // Down claim: mirror (close below P0 realizes).
            let dn = story(MeaningKind::Breakout, Direction::Down, n as u64 - 1);
            let mirror = rule().judge(&dn, &b, n as u64).unwrap().0;
            let expect = if c < 100.0 {
                Verdict::Realized
            } else {
                Verdict::Invalidated
            };
            assert_eq!(mirror, expect, "c={c}");
        }
    }

    #[test]
    fn neutral_expands_on_wick_and_lapses_at_horizon() {
        // Squeeze: h = 20, neutral threshold 0.65·√20 ≈ 2.907 (ATR100 = 1).
        let b = base();
        let s = story(MeaningKind::Squeeze, Direction::Neutral, 120);
        let r = rule();
        assert_eq!(r.judge(&s, &b, 139), None);
        assert_eq!(
            r.judge(&s, &b, 140),
            Some((Verdict::Invalidated, "yukdus.no_expansion"))
        );
        let mut w = base();
        w[125].l = 97.0; // wick 3.0 below P0
        assert_eq!(
            r.judge(&s, &w, 130),
            Some((Verdict::Realized, "yukdus.expansion"))
        );
    }

    #[test]
    fn pure_and_horizon_bounded() {
        let mut b = base();
        b[145].h = 102.0; // beyond born 130 + h 10
        let s = story(MeaningKind::Breakout, Direction::Up, 130);
        let r = rule();
        // Judged at a bar past the horizon, bars after born + h are not read.
        assert_eq!(r.judge(&s, &b, 146), None);
        // Same answer from a fresh instance and from a reused one (cache keyed by values).
        let a1 = r.judge(&s, &b[..=141], 141);
        let a2 = Yukdus::new("t", 1.0, 1.0).judge(&s, &b[..=141], 141);
        assert_eq!(a1, a2);
        let mut b2 = b.clone();
        for x in &mut b2 {
            x.h += 1.0;
        }
        // Reused instance on other data: wider ranges → other ATR, not a stale cache.
        let lv1 = levels_with(&r, &s, &b[..=130]).unwrap();
        let lv2 = levels_with(&r, &s, &b2[..=130]).unwrap();
        assert!(lv2.1 > lv1.1);
    }

    #[test]
    fn levels_match_judge_threshold() {
        let r = Yukdus::default();
        let b = base();
        let s = story(MeaningKind::Breakout, Direction::Up, 140);
        let (p0, t, st) = levels(&s, &b[..=140]).unwrap();
        assert!((p0 - 100.0).abs() < 1e-12);
        let want = S_DIR * 0.3 * 10f64.sqrt();
        assert!((t - want).abs() < 1e-9 && st == Some(t));
        // A wick just below / just past the target.
        let mut w = b.clone();
        w[141].h = 100.0 + t - 1e-9;
        assert_eq!(r.judge(&s, &w[..=141], 141), None);
        w[141].h = 100.0 + t + 1e-9;
        assert_eq!(r.judge(&s, &w[..=141], 141).unwrap().0, Verdict::Realized);
    }
}
