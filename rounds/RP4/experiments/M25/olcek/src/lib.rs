//! RP4 M25d alternative `olcek`: **period-scaled barrier** (periyoda göre ölçeklenen eşik).
//!
//! Same triple barrier as the product rule `r3` (`qp_capsule::outcome::Barrier`): `P0` =
//! birth close, ATR(14) yardstick at the birth bar, first close that crosses decides,
//! same per-kind horizons, same neutral rule. One change: the **directional** barrier is
//! multiplied by a factor that depends on the bar period,
//!
//! ```text
//! k = C_DIR · √h · m(P),   m(P) = max(1, SESSION / P)^BETA
//! ```
//!
//! where `P` is the bar period and `SESSION` the XIST continuous session (480 min). For
//! daily (and longer) bars `m = 1`: the rule *is* `r3` there. Intraday the factor grows
//! as the bar shortens (5 min: 96 bars/session; 15 min: 32; 1 h: 8).
//!
//! Why: on short intraday bars ATR(14) is a noisy, microstructure-dominated yardstick
//! (tick size, bid/ask bounce, the open's gap inside the window) and short-horizon
//! returns mean-revert, so `0.3·√h·ATR14` is only a few ticks: one noisy close stops the
//! story and the price later reaches the target anyway (Arkin's case, R6 §10). The
//! diffusive `√h` scaling was set on synthetic random walks, where this does not happen.
//!
//! The period is read from the bars themselves: the **smallest positive time step**
//! among the last [`PERIOD_LOOKBACK`] bars of `bars[..=born]` (overnight / weekend gaps
//! are larger, so the minimum is the bar period). Deterministic, no look-ahead, no story
//! metadata needed. Horizon is not scaled: `OutcomeRule::horizon` gets only the kind,
//! so it cannot know the period (see `NOTES.md`, weaknesses).
//!
//! The verdict at `now` is a pure function of `(story, bars[..=now])`: the rule rescans
//! `(born, now]` like `r3`. Like `r3`, a per-instance cache keeps the birth levels; its key
//! includes the birth bar's own values so a reused instance on other data cannot return a
//! stale yardstick.

use std::cell::RefCell;
use std::collections::BTreeMap;

use qp_capsule::outcome::{base_params, wilder_atr, ATR_FAST, ATR_SLOW};
use qp_capsule::{OpenStory, OutcomeRule, Verdict};
use qp_schema::capsule::{Direction, MeaningKind, TimeKind};
use qp_schema::Bar;

/// Exponent of the period factor (the only tuned constant; chosen on calibration data,
/// see `NOTES.md`).
pub const BETA: f64 = 0.1;
/// XIST continuous session length in milliseconds (10:00–18:00).
pub const SESSION_MS: f64 = 480.0 * 60_000.0;
/// Bars looked back (ending at the birth bar) to read the bar period.
pub const PERIOD_LOOKBACK: usize = 64;

/// Bar period in ms read from `bars` (smallest positive step among the last
/// [`PERIOD_LOOKBACK`] bars). `None` with fewer than two bars.
#[must_use]
pub fn bar_period_ms(bars: &[Bar]) -> Option<i64> {
    let from = bars.len().saturating_sub(PERIOD_LOOKBACK);
    bars[from..]
        .windows(2)
        .map(|w| w[1].t - w[0].t)
        .filter(|d| *d > 0)
        .min()
}

/// Period factor `m(P) = max(1, SESSION / P)^beta`; `1` for daily and longer bars.
#[must_use]
pub fn period_factor(period_ms: i64, beta: f64) -> f64 {
    #[allow(clippy::cast_precision_loss)]
    let per_session = SESSION_MS / period_ms.max(1) as f64;
    per_session.max(1.0).powf(beta)
}

type CacheKey = (String, String, u64, i64, [u64; 3], MeaningKind, Direction);
type Lv = Option<(f64, f64, Option<f64>)>;

/// The rule.
#[derive(Debug)]
pub struct Olcek {
    label: &'static str,
    beta: f64,
    cache: RefCell<BTreeMap<CacheKey, Lv>>,
}

impl Olcek {
    /// With a name and an exponent (`BETA` is the candidate).
    #[must_use]
    pub fn new(label: &'static str, beta: f64) -> Self {
        Self {
            label,
            beta,
            cache: RefCell::new(BTreeMap::new()),
        }
    }

    fn levels_cached(&self, s: &OpenStory, bars: &[Bar]) -> Lv {
        let born = usize::try_from(s.story.born_bar).ok()?;
        let b = bars.get(born)?;
        let key = (
            s.story.instrument.clone(),
            s.story.series.clone(),
            s.story.born_bar,
            b.t,
            [b.h.to_bits(), b.l.to_bits(), b.c.to_bits()],
            s.story.kind,
            s.story.direction,
        );
        if let Some(v) = self.cache.borrow().get(&key) {
            return *v;
        }
        let v = self.levels(s, bars);
        self.cache.borrow_mut().insert(key, v);
        v
    }

    /// Birth levels `(p0, target, stop)`; directional: target = stop.
    #[must_use]
    pub fn levels(&self, s: &OpenStory, bars: &[Bar]) -> Option<(f64, f64, Option<f64>)> {
        let born = usize::try_from(s.story.born_bar).ok()?;
        let upto = bars.get(..=born)?;
        let p0 = upto.last()?.c;
        let p = base_params(s.story.kind);
        if s.story.direction == Direction::Neutral {
            let a = wilder_atr(upto, ATR_SLOW)?;
            Some((p0, p.k_neutral * a, None))
        } else {
            let a = wilder_atr(upto, ATR_FAST)?;
            let m = period_factor(bar_period_ms(upto)?, self.beta);
            let thr = p.k * m * a;
            Some((p0, thr, Some(thr)))
        }
    }
}

impl Default for Olcek {
    fn default() -> Self {
        Self::new("olcek", BETA)
    }
}

impl OutcomeRule for Olcek {
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
        let last = now_i.min(born + h);
        let (p0, target, _) = self.levels_cached(s, bars)?;
        match s.story.direction {
            Direction::Neutral => {
                for b in &bars[born + 1..=last] {
                    if (b.c - p0).abs() >= target {
                        return Some((Verdict::Realized, "barrier.expansion"));
                    }
                }
                (now_i >= born + h).then_some((Verdict::Invalidated, "barrier.no_expansion"))
            }
            dir => {
                let sign = if dir == Direction::Up { 1.0 } else { -1.0 };
                for b in &bars[born + 1..=last] {
                    let d = sign * (b.c - p0);
                    if d >= target {
                        return Some((Verdict::Realized, "barrier.target"));
                    }
                    if d <= -target {
                        return Some((Verdict::Invalidated, "barrier.stop"));
                    }
                }
                None
            }
        }
    }
}

/// Harness entry: a fresh rule.
#[must_use]
pub fn outcome() -> Box<dyn OutcomeRule> {
    Box::new(Olcek::default())
}

/// Harness entry: birth levels from `bars[..=born]`.
#[must_use]
pub fn levels(s: &OpenStory, bars: &[Bar]) -> Option<(f64, f64, Option<f64>)> {
    Olcek::default().levels(s, bars)
}

#[cfg(test)]
mod tests {
    use super::*;
    use qp_capsule::outcome::Barrier;
    use qp_schema::capsule::{EvidenceFamily, LedgerStat, Stage, Story, Trust};

    const MIN5: i64 = 5 * 60_000;
    const DAY: i64 = 86_400_000;

    fn story(kind: MeaningKind, direction: Direction, born: u64) -> OpenStory {
        OpenStory {
            id: format!("cap.SYN.t.{born}"),
            story: Story {
                instrument: "SYN".into(),
                theme: "t".into(),
                kind,
                direction,
                time: TimeKind::Event,
                stage: Stage::Born,
                families: vec![EvidenceFamily::Price],
                strength: 0.5,
                importance: 1.0,
                tier: 1,
                series: "px".into(),
                born_bar: born,
                last_bar: born,
                horizon_bars: base_params(kind).horizon,
                ledger_key: "k".into(),
                ledger_at_birth: LedgerStat { n: 0, realized: 0 },
                trust: Trust::Replica,
                history: Vec::new(),
                condition: None,
                resolution: None,
            },
            facts: Vec::new(),
        }
    }

    /// Deterministic wiggly series: `n` bars, `step` ms apart, with an overnight-like gap
    /// every `gap_every` bars.
    fn series(n: usize, step: i64, gap_every: usize) -> Vec<Bar> {
        let mut t = 1_700_000_000_000_i64;
        let mut x = 100.0_f64;
        let mut s = 12_345_u64;
        (0..n)
            .map(|i| {
                if i > 0 {
                    t += if gap_every > 0 && i % gap_every == 0 {
                        16 * 3_600_000
                    } else {
                        step
                    };
                }
                s = s.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
                #[allow(clippy::cast_precision_loss)]
                let u = ((s >> 33) as f64 / f64::from(1_u32 << 31)) - 0.5;
                let o = x;
                x += u;
                Bar {
                    t,
                    o,
                    h: o.max(x) + 0.2,
                    l: o.min(x) - 0.2,
                    c: x,
                    v: 1.0,
                }
            })
            .collect()
    }

    #[test]
    fn period_is_read_from_bars() {
        assert_eq!(bar_period_ms(&series(200, MIN5, 96)), Some(MIN5));
        assert_eq!(bar_period_ms(&series(200, DAY, 0)), Some(DAY));
        assert_eq!(bar_period_ms(&series(1, MIN5, 0)), None);
    }

    #[test]
    fn factor_is_one_on_daily_and_grows_intraday() {
        assert!((period_factor(DAY, BETA) - 1.0).abs() < 1e-15);
        assert!((period_factor(7 * DAY, BETA) - 1.0).abs() < 1e-15);
        let f5 = period_factor(MIN5, BETA);
        let f15 = period_factor(3 * MIN5, BETA);
        let f60 = period_factor(12 * MIN5, BETA);
        assert!(f5 > f15 && f15 > f60 && f60 > 1.0);
        assert!((f5 - 96.0_f64.powf(BETA)).abs() < 1e-12);
    }

    /// On daily bars the rule gives exactly `r3`'s verdicts.
    #[test]
    fn daily_equals_r3() {
        let bars = series(400, DAY, 0);
        let r3 = Barrier::default();
        let me = Olcek::default();
        for kind in [
            MeaningKind::Breakout,
            MeaningKind::Continuation,
            MeaningKind::Squeeze,
        ] {
            for dir in [Direction::Up, Direction::Down, Direction::Neutral] {
                for born in (120..360).step_by(7) {
                    let s = story(kind, dir, born);
                    for now in born + 1..born + 21 {
                        let n = usize::try_from(now).unwrap();
                        assert_eq!(
                            me.judge(&s, &bars[..=n], now),
                            r3.judge(&s, &bars[..=n], now)
                        );
                    }
                }
            }
        }
    }

    /// Intraday the directional barrier is `m` times `r3`'s; neutral is unchanged.
    #[test]
    fn intraday_levels_scale() {
        let bars = series(400, MIN5, 96);
        let m = period_factor(MIN5, BETA);
        let born = 300_u64;
        let up = story(MeaningKind::Breakout, Direction::Up, born);
        let upto = &bars[..=300];
        let (p0, tg, st) = levels(&up, upto).unwrap();
        let r3 = base_params(MeaningKind::Breakout).k * wilder_atr(upto, ATR_FAST).unwrap();
        assert!((p0 - bars[300].c).abs() < 1e-12);
        assert!((tg - m * r3).abs() < 1e-9 && st == Some(tg));
        let ne = story(MeaningKind::Squeeze, Direction::Neutral, born);
        let (_, tn, sn) = levels(&ne, upto).unwrap();
        let r3n = base_params(MeaningKind::Squeeze).k_neutral * wilder_atr(upto, ATR_SLOW).unwrap();
        assert!((tn - r3n).abs() < 1e-9 && sn.is_none());
    }

    /// The verdict at `now` reads nothing after `now`; a fresh or reused instance agrees;
    /// two stories with the same birth bar but different kinds keep their own levels.
    #[test]
    fn pure_and_no_look_ahead() {
        let bars = series(400, MIN5, 96);
        let mut later = bars.clone();
        for b in &mut later[330..] {
            b.c += 50.0;
            b.h += 50.0;
        }
        let reused = Olcek::default();
        for kind in [MeaningKind::Breakout, MeaningKind::Continuation] {
            for dir in [Direction::Up, Direction::Down] {
                let s = story(kind, dir, 300);
                for now in 301..330_u64 {
                    let n = usize::try_from(now).unwrap();
                    let a = reused.judge(&s, &bars[..=n], now);
                    assert_eq!(a, Olcek::default().judge(&s, &later[..=n], now));
                    assert_eq!(a, Olcek::default().judge(&s, &bars[..=n], now));
                }
            }
        }
    }
}
