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
