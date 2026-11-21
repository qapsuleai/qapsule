//! Outcome rule chosen in R3/M5: **volatility barrier** ([`Barrier`], oynaklık bariyeri).
//!
//! Provenance: productized from the measured experiment crate
//! `rounds/R3/experiments/M5/barrier/` (commit `ab4f689`, branch `r3/m5a-barrier`; choice
//! and reasons in `rounds/R3/experiments/M5/RESULTS.md`). Behaviour is identical to the
//! measured crate (`rounds/R3/experiments/S/crosscheck`, `S/S-A.md`). Do not change the
//! constants here without a new measurement.
//!
//! Note for sentences (M5 RESULTS): for an `extreme` story, *realized* means the stretch
//! continued (price moved further in the story's direction), *invalidated* means it
//! snapped back; the realizer says so explicitly ([`crate::realize`]).
//!
//! A per-kind, ATR-scaled triple barrier judged on closes:
//!
//! - **Reference price** `P0` = close of the birth bar.
//! - **Yardstick** `A` = Wilder ATR at the birth bar, computed with `qp_compute::Atr`
//!   from `bars[..=born]` only (no look-ahead). Directional stories use ATR(14);
//!   neutral stories use ATR(100) ("normal" volatility, see below).
//! - **Directional story** (`up` / `down`): signed move `d = ±(close − P0)`. The first
//!   bar in `(born, now]` with `d ≥ k·A` → `realized` (`barrier.target`); with
//!   `d ≤ −k·A` → `invalidated` (`barrier.stop`). Symmetric (`k` up = `k` down), so a
//!   driftless market scores 50 % of the decided cases for every kind: the ledger number
//!   reads directly against one base rate.
//! - **Neutral story** (squeeze, neutral flow): the claim is "a move is coming, direction
//!   unknown". The first close with `|close − P0| ≥ k_n·A100` → `realized`
//!   (`barrier.expansion`); reaching the horizon without it → `invalidated`
//!   (`barrier.no_expansion`): for this claim, staying inside the corridor *is* the
//!   outcome. `k_n` is set so that a driftless random walk at normal volatility expands
//!   in about half of the cases.
//! - **Vertical barrier** = the per-kind horizon; a directional story that touches
//!   neither barrier expires (engine, reason `horizon`).
//!
//! Barrier sizes follow diffusive scaling from the kind's horizon `h`: directional
//! `k = 0.3·√h`, neutral `k_n = 0.65·√h` (ATR units). The constants were set on
//! synthetic data, unconditionally (every bar as a birth, no detector, no story): about
//! 10 % of directional cases expire, and about half of the neutral cases expand. See
//! `NOTES.md`.
//!
//! The verdict at `now` is a pure function of `bars[..=now]` and the story (kind,
//! direction, birth bar): the rule rescans `(born, now]` for the first crossing, so it
//! does not depend on being called on every bar. A per-rule cache keeps the birth ATR;
//! its key includes the birth bar's own values so a reused instance on other data
//! cannot return a stale yardstick.

use std::cell::RefCell;
use std::collections::BTreeMap;

use crate::{OpenStory, OutcomeRule, Verdict};
use qp_schema::capsule::{Direction, MeaningKind, TimeKind};
use qp_schema::Bar;

/// ATR period for directional stories.
pub const ATR_FAST: usize = 14;
/// ATR period for neutral stories ("normal" volatility).
pub const ATR_SLOW: usize = 100;

/// Base horizon (bars) and directional barrier (ATR multiples) of one meaning kind.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KindParams {
    /// Horizon in bars.
    pub horizon: u32,
    /// Directional barrier: target = stop = `k · ATR(14)`.
    pub k: f64,
    /// Neutral barrier: `k_n · ATR(100)`.
    pub k_neutral: f64,
}

/// Directional barrier constant: `k = C_DIR · √h` (ATR(14) units).
pub const C_DIR: f64 = 0.3;
/// Neutral barrier constant: `k_n = C_NEUTRAL · √h` (ATR(100) units).
pub const C_NEUTRAL: f64 = 0.65;

/// The base table (see `NOTES.md` for the reasoning per kind). Barriers follow
/// diffusive scaling from the kind's base horizon; a horizon perturbation does not move
/// them.
#[must_use]
pub fn base_params(kind: MeaningKind) -> KindParams {
    let horizon: u32 = match kind {
        MeaningKind::Continuation | MeaningKind::Squeeze => 20,
        MeaningKind::Reversal => 15,
        MeaningKind::Breakout | MeaningKind::Extreme | MeaningKind::Flow => 10,
    };
    let r = f64::from(horizon).sqrt();
    KindParams {
        horizon,
        k: C_DIR * r,
        k_neutral: C_NEUTRAL * r,
    }
}

type CacheKey = (String, String, u64, i64, [u64; 3], usize);

/// The barrier outcome rule. `h_scale` and `k_scale` exist for the perturbations
/// (horizon ±20 %, barriers ±10 %); the candidate itself is `1.0, 1.0`.
#[derive(Debug)]
pub struct Barrier {
    label: &'static str,
    h_scale: f64,
    k_scale: f64,
    atr_cache: RefCell<BTreeMap<CacheKey, Option<f64>>>,
}

impl Barrier {
    /// With scales (1.0, 1.0 = the candidate).
    #[must_use]
    pub fn new(label: &'static str, h_scale: f64, k_scale: f64) -> Self {
        Self {
            label,
            h_scale,
            k_scale,
            atr_cache: RefCell::new(BTreeMap::new()),
        }
    }

    /// Effective parameters for a kind (scales applied).
    #[must_use]
    pub fn params(&self, kind: MeaningKind) -> KindParams {
        let b = base_params(kind);
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            clippy::cast_precision_loss
        )]
        let horizon = ((f64::from(b.horizon) * self.h_scale).round() as u32).max(1);
        KindParams {
            horizon,
            k: b.k * self.k_scale,
            k_neutral: b.k_neutral * self.k_scale,
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

/// Wilder ATR(`period`) at the last bar of `bars`, via `qp_compute::Atr`. `None` while
/// warming up (fewer than `period` bars) or when not finite / not positive.
#[must_use]
pub fn wilder_atr(bars: &[Bar], period: usize) -> Option<f64> {
    let (last, head) = bars.split_last()?;
    let mut atr = qp_compute::Atr::new(period).ok()?;
    for b in head {
        atr.commit_hlc(b.h, b.l, b.c);
    }
    let v = atr.peek_hl(last.h, last.l)?;
    (v.is_finite() && v > 0.0).then_some(v)
}

impl OutcomeRule for Barrier {
    fn name(&self) -> &'static str {
        self.label
    }

    fn horizon(&self, kind: MeaningKind, _time: TimeKind) -> u32 {
        // Per kind; the time kind does not change the claim's natural life here (state
        // kinds already got longer horizons in the table). Expectations, if a detector
        // ever emits one, live as long as their kind.
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
                let a = self.atr_at_birth(s, bars, born, ATR_SLOW)?;
                let thr = p.k_neutral * a;
                for b in &bars[born + 1..=last] {
                    if (b.c - p0).abs() >= thr {
                        return Some((Verdict::Realized, "barrier.expansion"));
                    }
                }
                (now_i >= born + p.horizon as usize)
                    .then_some((Verdict::Invalidated, "barrier.no_expansion"))
            }
            dir => {
                let a = self.atr_at_birth(s, bars, born, ATR_FAST)?;
                let thr = p.k * a;
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

impl Default for Barrier {
    /// The chosen rule (scales 1.0, 1.0), named `barrier`.
    fn default() -> Self {
        Self::new("barrier", 1.0, 1.0)
    }
}

impl Barrier {
    /// Levels fixed at birth, `(P0, target, stop)` in price units from `bars[..=born]`:
    /// directional `k·ATR14` both ways; neutral `k_n·ATR100` and no stop. `None` while
    /// the ATR warms up.
    #[must_use]
    pub fn levels(&self, s: &OpenStory, bars: &[Bar]) -> Option<(f64, f64, Option<f64>)> {
        let born = usize::try_from(s.story.born_bar).ok()?;
        let p0 = bars.get(born)?.c;
        let p = self.params(s.story.kind);
        if s.story.direction == Direction::Neutral {
            let a = wilder_atr(&bars[..=born], ATR_SLOW)?;
            Some((p0, p.k_neutral * a, None))
        } else {
            let a = wilder_atr(&bars[..=born], ATR_FAST)?;
            Some((p0, p.k * a, Some(p.k * a)))
        }
    }
}

// ---------------------------------------------------------------------------------------
// RP4/M25: high / low touch (`yukdus`), and the `rp4` rule by bar period.
// ---------------------------------------------------------------------------------------

/// (RP4) Scale on the directional barrier of [`Barrier`] for [`HighLow`]
/// (`k = HL_S_DIR · 0.3·√h`, ATR(14)). Tuned on the calibration range only
/// (`rounds/RP4/experiments/M25/yukdus/NOTES.md`).
pub const HL_S_DIR: f64 = 2.0;
/// (RP4) Scale on the neutral barrier of [`Barrier`] for [`HighLow`]
/// (`k_n = HL_S_NEU · 0.65·√h`, ATR(100)).
pub const HL_S_NEU: f64 = 1.3;

/// (RP4/M25, `docs/CAPSULE.md` §9.9) **High / low touch** rule: the same frame as
/// [`Barrier`] (`P0` = birth close, Wilder ATR at the birth bar from `bars[..=born]`,
/// per-kind horizon, diffusive barrier sizes) with two differences:
///
/// 1. **Touch, not close.** A directional story (sign `s`) at bar `j` has a favourable
///    excursion `f = s>0 ? high − P0 : P0 − low` and an adverse excursion
///    `a = s>0 ? P0 − low : high − P0`. The first bar in `(born, min(now, born+h)]` with
///    `f ≥ T` realizes (`highlow.target`), with `a ≥ T` invalidates (`highlow.stop`). A
///    neutral story realizes at the first bar with `max(high − P0, P0 − low) ≥ T_n`
///    (`highlow.expansion`); at the horizon without it, it is invalidated
///    (`highlow.no_expansion`).
/// 2. **Wider barrier.** `T = HL_S_DIR · 0.3·√h · ATR14` (2.0), `T_n = HL_S_NEU ·
///    0.65·√h · ATR100` (1.3).
///
/// **Tie** (one bar touches target and stop): the close decides; `s·(close − P0) > 0`
/// realizes (`highlow.tie_target`), otherwise invalidated (`highlow.tie_stop`; a close
/// exactly at `P0` does not hold the claim).
///
/// Provenance: `rounds/RP4/experiments/M25/yukdus` (measured; choice in
/// `rounds/RP4/experiments/M25/RESULTS.md`). The same verdict on the same bar as that
/// crate for every story; only the reason codes' prefix differs (`highlow.` for
/// `yukdus.`). Cross-check: `rounds/RP4/experiments/harness/tests/rp4_product.rs`.
#[derive(Debug)]
pub struct HighLow {
    label: &'static str,
    s_dir: f64,
    s_neu: f64,
    atr_cache: RefCell<BTreeMap<CacheKey, Option<f64>>>,
}

impl HighLow {
    /// With barrier scales (directional, neutral) on [`Barrier`]'s barriers.
    #[must_use]
    pub fn new(label: &'static str, s_dir: f64, s_neu: f64) -> Self {
        Self {
            label,
            s_dir,
            s_neu,
            atr_cache: RefCell::new(BTreeMap::new()),
        }
    }

    /// Effective parameters for a kind ([`base_params`] horizon, scaled barriers).
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

    /// Levels fixed at birth, `(P0, target, stop)` in price units from `bars[..=born]`:
    /// directional `HL_S_DIR·k·ATR14` both ways; neutral `HL_S_NEU·k_n·ATR100`, no stop.
    #[must_use]
    pub fn levels(&self, s: &OpenStory, bars: &[Bar]) -> Option<(f64, f64, Option<f64>)> {
        let born = usize::try_from(s.story.born_bar).ok()?;
        let p0 = bars.get(born)?.c;
        let p = self.params(s.story.kind);
        if s.story.direction == Direction::Neutral {
            let a = wilder_atr(&bars[..=born], ATR_SLOW)?;
            Some((p0, p.k_neutral * a, None))
        } else {
            let a = wilder_atr(&bars[..=born], ATR_FAST)?;
            Some((p0, p.k * a, Some(p.k * a)))
        }
    }
}

impl Default for HighLow {
    /// The chosen rule (scales [`HL_S_DIR`], [`HL_S_NEU`]), named `highlow`.
    fn default() -> Self {
        Self::new("highlow", HL_S_DIR, HL_S_NEU)
    }
}

impl OutcomeRule for HighLow {
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
                        return Some((Verdict::Realized, "highlow.expansion"));
                    }
                }
                (now_i >= born + p.horizon as usize)
                    .then_some((Verdict::Invalidated, "highlow.no_expansion"))
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
                        (true, false) => return Some((Verdict::Realized, "highlow.target")),
                        (false, true) => return Some((Verdict::Invalidated, "highlow.stop")),
                        (true, true) => {
                            let dc = if up { b.c - p0 } else { p0 - b.c };
                            return Some(if dc > 0.0 {
                                (Verdict::Realized, "highlow.tie_target")
                            } else {
                                (Verdict::Invalidated, "highlow.tie_stop")
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

/// (RP4) Bars at least this far apart are not intraday: [`Rp4`] judges them with
/// [`Barrier`]. Half a day, so a daily series stays daily across any time-zone shift.
pub const INTRADAY_MAX_STEP_MS: i64 = 43_200_000;

/// Whether the bar series `bars[..=born]` is intraday: its smallest positive time step is
/// below [`INTRADAY_MAX_STEP_MS`]. Deterministic and without look-ahead (only bars up to
/// the birth); a series with a single bar is not intraday.
#[must_use]
pub fn intraday_at(bars: &[Bar], born: usize) -> bool {
    let Some(head) = bars.get(..=born) else {
        return false;
    };
    head.windows(2)
        .map(|w| w[1].t - w[0].t)
        .filter(|&d| d > 0)
        .min()
        .is_some_and(|d| d < INTRADAY_MAX_STEP_MS)
}

/// (RP4/M25, `docs/CAPSULE.md` §9.9) The `rp4` outcome rule: **by the bar period** of the
/// story's series. Intraday bars (smallest time step in `bars[..=born]` below
/// [`INTRADAY_MAX_STEP_MS`], [`intraday_at`]) → [`HighLow`]; daily and longer bars →
/// [`Barrier`] (the `r3` rule, unchanged). Both have the same per-kind horizons
/// ([`base_params`]), so the horizon does not depend on the period.
///
/// The period is read from the bars, not from the series id or the session: the engine
/// only knows the series' bars, and a story's period is fixed at its birth (cached per
/// story with the birth bar's values in the key, like the ATR).
#[derive(Debug)]
pub struct Rp4 {
    highlow: HighLow,
    barrier: Barrier,
    period_cache: RefCell<BTreeMap<CacheKey, bool>>,
}

impl Default for Rp4 {
    fn default() -> Self {
        Self {
            highlow: HighLow::default(),
            barrier: Barrier::default(),
            period_cache: RefCell::new(BTreeMap::new()),
        }
    }
}

impl Rp4 {
    /// Whether the story's series is intraday at its birth (cached).
    fn intraday(&self, s: &OpenStory, bars: &[Bar]) -> Option<bool> {
        let born = usize::try_from(s.story.born_bar).ok()?;
        let b = bars.get(born)?;
        let key = (
            s.story.instrument.clone(),
            s.story.series.clone(),
            s.story.born_bar,
            b.t,
            [b.h.to_bits(), b.l.to_bits(), b.c.to_bits()],
            0,
        );
        if let Some(v) = self.period_cache.borrow().get(&key) {
            return Some(*v);
        }
        let v = intraday_at(bars, born);
        self.period_cache.borrow_mut().insert(key, v);
        Some(v)
    }

    /// The levels of the sub-rule the story's period picks ([`HighLow::levels`] or
    /// [`Barrier::levels`]), from `bars[..=born]`.
    #[must_use]
    pub fn levels(&self, s: &OpenStory, bars: &[Bar]) -> Option<(f64, f64, Option<f64>)> {
        if self.intraday(s, bars)? {
            self.highlow.levels(s, bars)
        } else {
            self.barrier.levels(s, bars)
        }
    }
}

impl OutcomeRule for Rp4 {
    fn name(&self) -> &'static str {
        "rp4"
    }

    fn horizon(&self, kind: MeaningKind, time: TimeKind) -> u32 {
        // Equal for both sub-rules (`base_params`).
        self.barrier.horizon(kind, time)
    }

    fn judge(&self, s: &OpenStory, bars: &[Bar], now: u64) -> Option<(Verdict, &'static str)> {
        if self.intraday(s, bars)? {
            self.highlow.judge(s, bars, now)
        } else {
            self.barrier.judge(s, bars, now)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{OpenStory, OutcomeRule, Verdict};
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
    fn rule() -> HighLow {
        HighLow::new("t", 1.0, 1.0)
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
            Some((Verdict::Realized, "highlow.target"))
        );
        // The close-based reference does not decide on that bar.
        let r3 = Barrier::default();
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
            Some((Verdict::Invalidated, "highlow.stop"))
        );
        assert_eq!(
            rule().judge(&dn, &b, n as u64),
            Some((Verdict::Realized, "highlow.target"))
        );
    }

    #[test]
    fn tie_goes_by_close_side() {
        for (c, want) in [
            (100.3, (Verdict::Realized, "highlow.tie_target")),
            (99.7, (Verdict::Invalidated, "highlow.tie_stop")),
            (100.0, (Verdict::Invalidated, "highlow.tie_stop")),
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
            Some((Verdict::Invalidated, "highlow.no_expansion"))
        );
        let mut w = base();
        w[125].l = 97.0; // wick 3.0 below P0
        assert_eq!(
            r.judge(&s, &w, 130),
            Some((Verdict::Realized, "highlow.expansion"))
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
        let a2 = HighLow::new("t", 1.0, 1.0).judge(&s, &b[..=141], 141);
        assert_eq!(a1, a2);
        let mut b2 = b.clone();
        for x in &mut b2 {
            x.h += 1.0;
        }
        // Reused instance on other data: wider ranges → other ATR, not a stale cache.
        let lv1 = r.levels(&s, &b[..=130]).unwrap();
        let lv2 = r.levels(&s, &b2[..=130]).unwrap();
        assert!(lv2.1 > lv1.1);
    }

    #[test]
    fn levels_match_judge_threshold() {
        let r = HighLow::default();
        let b = base();
        let s = story(MeaningKind::Breakout, Direction::Up, 140);
        let (p0, t, st) = r.levels(&s, &b[..=140]).unwrap();
        assert!((p0 - 100.0).abs() < 1e-12);
        let want = HL_S_DIR * 0.3 * 10f64.sqrt();
        assert!((t - want).abs() < 1e-9 && st == Some(t));
        // A wick just below / just past the target.
        let mut w = b.clone();
        w[141].h = 100.0 + t - 1e-9;
        assert_eq!(r.judge(&s, &w[..=141], 141), None);
        w[141].h = 100.0 + t + 1e-9;
        assert_eq!(r.judge(&s, &w[..=141], 141).unwrap().0, Verdict::Realized);
    }

    #[test]
    fn rp4_picks_by_period() {
        // 5 minute bars (`base`): the high touch decides, as `HighLow`.
        let mut b = base();
        b.push(bar(150, 100.0, 103.0, 99.9, 100.1));
        let n = b.len() - 1;
        let s = story(MeaningKind::Breakout, Direction::Up, n as u64 - 1);
        let r = Rp4::default();
        assert_eq!(r.name(), "rp4");
        assert_eq!(
            r.judge(&s, &b, n as u64),
            HighLow::default().judge(&s, &b, n as u64)
        );
        assert_eq!(
            r.judge(&s, &b, n as u64),
            Some((Verdict::Realized, "highlow.target"))
        );
        assert_eq!(r.levels(&s, &b), HighLow::default().levels(&s, &b));
        // The same bars one day apart: `Barrier` (close-based) decides, as `r3`.
        let mut d = b.clone();
        for (i, x) in d.iter_mut().enumerate() {
            x.t = i64::try_from(i).unwrap() * 86_400_000;
        }
        assert_eq!(
            r.judge(&s, &d, n as u64),
            Barrier::default().judge(&s, &d, n as u64)
        );
        assert_eq!(r.judge(&s, &d, n as u64), None);
        assert_eq!(r.levels(&s, &d), Barrier::default().levels(&s, &d));
        // Horizons are the same for both sub-rules.
        for k in MeaningKind::ALL {
            assert_eq!(
                r.horizon(k, TimeKind::Event),
                Barrier::default().horizon(k, TimeKind::Event)
            );
            assert_eq!(
                HighLow::default().horizon(k, TimeKind::Event),
                Barrier::default().horizon(k, TimeKind::Event)
            );
        }
    }

    #[test]
    fn intraday_from_bars_up_to_birth() {
        let mk = |ts: &[i64]| -> Vec<Bar> {
            ts.iter()
                .map(|&t| Bar {
                    t,
                    o: 1.0,
                    h: 1.0,
                    l: 1.0,
                    c: 1.0,
                    v: 1.0,
                })
                .collect()
        };
        let day = 86_400_000;
        // Daily with weekends; a single bar; out of range.
        assert!(!intraday_at(&mk(&[0, day, 2 * day, 5 * day]), 3));
        assert!(!intraday_at(&mk(&[0]), 0));
        assert!(!intraday_at(&mk(&[0, 300_000]), 5));
        // Hourly with an overnight gap; only bars up to the birth count.
        let b = mk(&[0, 3_600_000, day, day + 300_000]);
        assert!(intraday_at(&b, 1));
        assert!(!intraday_at(&mk(&[0, day, day + 300_000]), 1));
        assert!(intraday_at(&mk(&[0, day, day + 300_000]), 2));
    }
}
