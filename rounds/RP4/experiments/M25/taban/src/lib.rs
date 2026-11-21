//! RP4/M25 alternative `taban`: the R3 volatility barrier with a **price-step floor**.
//!
//! Identical to `qp_capsule::outcome::Barrier` (`r3`: per-kind horizon, symmetric
//! barrier judged on closes, first crossing decides) except for the barrier size:
//!
//! - directional: target = stop = `max(k·ATR14, N·tick(P0))`, `k = 0.3·√h`;
//! - neutral: expansion threshold = `max(k_n·ATR100, N·tick(P0))`, `k_n = 0.65·√h`.
//!
//! `P0` is the birth close, `tick` the BIST equity price-step table (SPEC §1). Where the
//! ATR barrier is already wider than `N` steps (daily bars, volatile names) the rule is
//! exactly `r3`; on quiet intraday bars, where `r3`'s barrier is only a few price steps
//! and a single bar's noise decides the story (R6 REPORT §10), the floor holds it at `N`
//! steps. The chosen `N` and the trials are in `NOTES.md`.
//!
//! The verdict at `now` is a pure function of `(story, bars[..=now])`: no clock, network,
//! randomness or file. A per-instance cache keeps the birth threshold; its key includes
//! the story's kind and direction and the birth bar's own values so a reused instance
//! cannot return a stale threshold.

use std::cell::RefCell;
use std::collections::BTreeMap;

use qp_capsule::outcome::{base_params, wilder_atr, ATR_FAST, ATR_SLOW};
use qp_capsule::{OpenStory, OutcomeRule, Verdict};
use qp_schema::capsule::{Direction, MeaningKind, TimeKind};
use qp_schema::Bar;

/// The floor in price steps (chosen on the calibration range, `NOTES.md`).
pub const N_TICKS: f64 = 8.0;

/// BIST equity price step at price `p` (SPEC §1): ≤20: 0.01; ≤50: 0.02; ≤100: 0.05;
/// ≤250: 0.10; ≤500: 0.25; ≤1000: 0.50; ≤2500: 1.00; above: 2.50.
#[must_use]
pub fn tick(p: f64) -> f64 {
    const TABLE: [(f64, f64); 7] = [
        (20.0, 0.01),
        (50.0, 0.02),
        (100.0, 0.05),
        (250.0, 0.10),
        (500.0, 0.25),
        (1000.0, 0.50),
        (2500.0, 1.00),
    ];
    for (hi, step) in TABLE {
        if p <= hi {
            return step;
        }
    }
    2.50
}

/// Birth levels `(p0, threshold)` for a floor of `n` steps, from `bars[..=born]` only:
/// `max(k·ATR, n·tick(p0)) − STEP_TOL·tick(p0)`.
/// `None` while the ATR warms up (as `r3`).
#[must_use]
pub fn threshold(s: &OpenStory, bars: &[Bar], n: f64) -> Option<(f64, f64)> {
    let born = usize::try_from(s.story.born_bar).ok()?;
    let p0 = bars.get(born)?.c;
    let p = base_params(s.story.kind);
    let atr_part = if s.story.direction == Direction::Neutral {
        p.k_neutral * wilder_atr(&bars[..=born], ATR_SLOW)?
    } else {
        p.k * wilder_atr(&bars[..=born], ATR_FAST)?
    };
    let step = tick(p0);
    // Closes sit on the price-step grid; when the floor binds, "moved N steps" must not
    // depend on float rounding of `c − p0` (10.08 − 10.00 < 0.08 in f64). The tolerance
    // is far below one step, so it never changes a decision by a real price.
    Some((p0, atr_part.max(n * step) - STEP_TOL * step))
}

/// Tolerance on the barrier, in price steps (float rounding only).
pub const STEP_TOL: f64 = 1e-6;

/// Harness levels `(p0, target, stop)` for a floor of `n` steps.
#[must_use]
pub fn levels_n(s: &OpenStory, bars: &[Bar], n: f64) -> Option<(f64, f64, Option<f64>)> {
    let (p0, thr) = threshold(s, bars, n)?;
    let stop = (s.story.direction != Direction::Neutral).then_some(thr);
    Some((p0, thr, stop))
}

/// Harness levels of the chosen rule.
#[must_use]
pub fn levels(s: &OpenStory, bars: &[Bar]) -> Option<(f64, f64, Option<f64>)> {
    levels_n(s, bars, N_TICKS)
}

/// Fresh instance of the chosen rule.
#[must_use]
pub fn outcome() -> Box<dyn OutcomeRule> {
    Box::new(Taban::new("taban", N_TICKS))
}

/// Instrument, series, kind and direction (the threshold depends on both), birth bar
/// index, its time and values.
type CacheKey = (String, String, String, u64, i64, [u64; 3]);

/// Barrier with a price-step floor.
#[derive(Debug)]
pub struct Taban {
    label: &'static str,
    n: f64,
    cache: RefCell<BTreeMap<CacheKey, Option<(f64, f64)>>>,
}

impl Taban {
    /// A rule with a floor of `n` price steps.
    #[must_use]
    pub fn new(label: &'static str, n: f64) -> Self {
        Self {
            label,
            n,
            cache: RefCell::new(BTreeMap::new()),
        }
    }

    fn birth(&self, s: &OpenStory, bars: &[Bar], born: usize) -> Option<(f64, f64)> {
        let b = bars.get(born)?;
        let key = (
            s.story.instrument.clone(),
            s.story.series.clone(),
            format!("{:?}/{:?}", s.story.kind, s.story.direction),
            s.story.born_bar,
            b.t,
            [b.h.to_bits(), b.l.to_bits(), b.c.to_bits()],
        );
        if let Some(v) = self.cache.borrow().get(&key) {
            return *v;
        }
        let v = threshold(s, bars, self.n);
        self.cache.borrow_mut().insert(key, v);
        v
    }
}

impl OutcomeRule for Taban {
    fn name(&self) -> &'static str {
        self.label
    }

    fn horizon(&self, kind: MeaningKind, _time: TimeKind) -> u32 {
        base_params(kind).horizon
    }

    fn judge(&self, s: &OpenStory, bars: &[Bar], now: u64) -> Option<(Verdict, &'static str)> {
        let born = usize::try_from(s.story.born_bar).ok()?;
        let now_i = usize::try_from(now).ok()?;
        if now_i <= born || bars.len() <= now_i {
            return None;
        }
        let h = base_params(s.story.kind).horizon as usize;
        let (p0, thr) = self.birth(s, bars, born)?;
        let last = now_i.min(born + h);
        match s.story.direction {
            Direction::Neutral => {
                if bars[born + 1..=last]
                    .iter()
                    .any(|b| (b.c - p0).abs() >= thr)
                {
                    return Some((Verdict::Realized, "barrier.expansion"));
                }
                (now_i >= born + h).then_some((Verdict::Invalidated, "barrier.no_expansion"))
            }
            dir => {
                let sign = if dir == Direction::Up { 1.0 } else { -1.0 };
                for b in &bars[born + 1..=last] {
                    let d = sign * (b.c - p0);
                    if d >= thr {
                        return Some((Verdict::Realized, "barrier.target"));
                    }
                    if d <= -thr {
                        return Some((Verdict::Invalidated, "barrier.stop"));
                    }
                }
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qp_capsule::outcome::Barrier;
    use qp_schema::capsule::{LedgerStat, Stage, Story, Trust};

    fn bar(i: usize, c: f64, range: f64) -> Bar {
        Bar {
            t: 1_700_000_000_000 + i64::try_from(i).unwrap() * 300_000,
            o: c,
            h: c + range / 2.0,
            l: c - range / 2.0,
            c,
            v: 1000.0,
        }
    }

    /// `n` flat bars at `p` with high−low = `range` (ATR = `range` exactly).
    fn flat(n: usize, p: f64, range: f64) -> Vec<Bar> {
        (0..n).map(|i| bar(i, p, range)).collect()
    }

    fn story(kind: MeaningKind, dir: Direction, born: u64) -> OpenStory {
        OpenStory {
            id: format!("cap.TEST.x.{born}"),
            story: Story {
                instrument: "TEST".to_owned(),
                theme: "x".to_owned(),
                kind,
                direction: dir,
                time: TimeKind::Event,
                stage: Stage::Born,
                families: vec![],
                strength: 0.5,
                importance: 0.0,
                tier: 3,
                series: "5min".to_owned(),
                born_bar: born,
                last_bar: born,
                horizon_bars: base_params(kind).horizon,
                ledger_key: String::new(),
                ledger_at_birth: LedgerStat::default(),
                trust: Trust::Replica,
                history: vec![],
                condition: None,
                resolution: None,
            },
            facts: vec![],
        }
    }

    fn first_verdict(
        r: &dyn OutcomeRule,
        s: &OpenStory,
        bars: &[Bar],
    ) -> Option<(usize, Verdict, &'static str)> {
        let born = usize::try_from(s.story.born_bar).unwrap();
        let h = r.horizon(s.story.kind, s.story.time) as usize;
        (born + 1..=(born + h).min(bars.len() - 1)).find_map(|now| {
            r.judge(s, &bars[..=now], now as u64)
                .map(|(v, c)| (now, v, c))
        })
    }

    #[test]
    fn tick_table_limits() {
        assert!((tick(20.0) - 0.01).abs() < 1e-12);
        assert!((tick(20.01) - 0.02).abs() < 1e-12);
        assert!((tick(100.0) - 0.05).abs() < 1e-12);
        assert!((tick(2500.0) - 1.0).abs() < 1e-12);
        assert!((tick(2500.5) - 2.5).abs() < 1e-12);
    }

    /// With `N = 0` the floor never binds: every verdict equals `r3`'s.
    #[test]
    fn zero_floor_is_r3() {
        let mut b = flat(120, 10.0, 0.02);
        let path = [
            0.0, 0.01, -0.02, 0.03, -0.01, 0.05, -0.04, 0.02, 0.0, 0.06, -0.07,
        ];
        for (j, d) in path.iter().enumerate() {
            b.push(bar(120 + j, 10.0 + d, 0.02));
        }
        let r3 = Barrier::default();
        let t0 = Taban::new("t0", 0.0);
        for kind in [MeaningKind::Breakout, MeaningKind::Continuation] {
            for dir in [Direction::Up, Direction::Down, Direction::Neutral] {
                let s = story(kind, dir, 119);
                assert_eq!(first_verdict(&t0, &s, &b), first_verdict(&r3, &s, &b));
            }
        }
    }

    /// Quiet 5 dk bars: `r3`'s barrier is ~2 steps, a 3-step bar against the claim
    /// stops it; the floor (8 steps) keeps the story open until a real move.
    #[test]
    fn floor_holds_through_one_bar_noise() {
        // P0 = 10 (step 0.01), ATR = 0.02 → r3 breakout barrier ≈ 0.019.
        let mut b = flat(120, 10.0, 0.02);
        b.push(bar(120, 9.97, 0.02)); // −3 steps: r3 stop
        b.push(bar(121, 10.02, 0.02));
        b.push(bar(122, 10.09, 0.02)); // +9 steps: taban target
        let s = story(MeaningKind::Breakout, Direction::Up, 119);
        let r3 = Barrier::default();
        assert_eq!(
            first_verdict(&r3, &s, &b),
            Some((120, Verdict::Invalidated, "barrier.stop"))
        );
        let t = Taban::new("taban", N_TICKS);
        assert_eq!(
            first_verdict(&t, &s, &b),
            Some((122, Verdict::Realized, "barrier.target"))
        );
        let (p0, tgt, stop) = levels_n(&s, &b[..=119], N_TICKS).unwrap();
        assert!((p0 - 10.0).abs() < 1e-12);
        assert!((tgt - N_TICKS * 0.01).abs() < 1e-7);
        assert_eq!(stop, Some(tgt));
    }

    /// A move of exactly `N` steps decides, despite f64 rounding (10.12 − 10.00 < 0.12).
    #[test]
    fn exact_n_steps_decides() {
        let (p0, c) = (
            std::hint::black_box(10.0_f64),
            std::hint::black_box(10.12_f64),
        );
        assert!(c - p0 < 12.0 * 0.01, "the knife edge this test guards");
        let mut b = flat(120, 10.0, 0.02);
        b.push(bar(120, 10.12, 0.02));
        let s = story(MeaningKind::Breakout, Direction::Up, 119);
        let t = Taban::new("t12", 12.0);
        assert_eq!(
            first_verdict(&t, &s, &b),
            Some((120, Verdict::Realized, "barrier.target"))
        );
        let mut b = flat(120, 10.0, 0.02);
        b.push(bar(120, 9.88, 0.02));
        assert_eq!(
            first_verdict(&t, &s, &b),
            Some((120, Verdict::Invalidated, "barrier.stop"))
        );
    }

    /// Where ATR is many steps wide, the floor is inert (daily-like bars).
    #[test]
    fn wide_atr_unchanged() {
        let s = story(MeaningKind::Breakout, Direction::Down, 119);
        let b = flat(120, 100.0, 2.0); // step 0.05, ATR 2 = 40 steps
        let (_, thr) = threshold(&s, &b, N_TICKS).unwrap();
        assert!((thr - base_params(MeaningKind::Breakout).k * 2.0).abs() < 1e-6);
        let n = story(MeaningKind::Squeeze, Direction::Neutral, 119);
        let (_, thr_n) = threshold(&n, &b, N_TICKS).unwrap();
        assert!((thr_n - base_params(MeaningKind::Squeeze).k_neutral * 2.0).abs() < 1e-6);
    }

    /// Same input, same verdict, also on a reused instance (cache keyed by the birth bar).
    #[test]
    fn deterministic_and_cache_safe() {
        let s = story(MeaningKind::Breakout, Direction::Up, 119);
        let mut a = flat(120, 10.0, 0.02);
        a.push(bar(120, 10.09, 0.02));
        let mut c = flat(120, 50.0, 0.04);
        c.push(bar(120, 50.09, 0.04)); // step 0.02: floor 0.16, +0.09 undecided
        let t = Taban::new("taban", N_TICKS);
        let first = first_verdict(&t, &s, &a);
        assert_eq!(first, Some((120, Verdict::Realized, "barrier.target")));
        assert_eq!(first_verdict(&t, &s, &c), None);
        assert_eq!(first_verdict(&t, &s, &a), first);
    }
}
