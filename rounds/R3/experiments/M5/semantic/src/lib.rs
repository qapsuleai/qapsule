//! R3 M5 candidate **"semantic"** — the kind's own meaning of "came true".
//!
//! Each meaning kind (K30) has its own definition of realized and invalidated, written
//! with the story's lead fact evidence (level, band, swing points) and the bars up to
//! and including `now` only. The claim is always the story's **direction** (so a
//! direction-flipped decoy is judged as the opposite claim); evidence gives the anchors.
//!
//! Scale: `atr` = mean true range over the 14 bars ending at the birth bar (inclusive;
//! fewer when the series is shorter, at least 5, otherwise the story is never judged and
//! expires; bar 0's true range is `h − l`). `d` = +1 for
//! `up`, −1 for `down`. `c` = close at `now`, `c0` = close at birth. All thresholds are
//! ATR multiples and are multiplied by `t_scale`; all horizons by `h_scale`.
//!
//! | Kind | Realized | Invalidated | Horizon |
//! |------|----------|-------------|---------|
//! | breakout | `d·(c − c0) ≥ 1.0·atr` (extends) | `d·(c − level) < −0.25·atr` (back inside the broken range) | 10 |
//! | reversal | close beyond the neckline (`down`: lowest low, `up`: highest high of bars `p1..=p2`) | `d·(c − p2_price) < −0.25·atr` (new extreme beyond the divergence swing) | 20 |
//! | squeeze | close outside the birth band by `≥ 0.25·atr` **and** mean TR of the last 5 bars `≥ 1.0·atr` | — (fizzles → expires) | 30 |
//! | extreme | `d·(c − sma20(now)) ≤ 0.25·atr` (back to the mean) | `d·(c − c0) ≥ 1.5·atr` (stretches further) | 15 |
//! | continuation | `d·(c − c0) ≥ 1.5·atr` (keeps going) | `d·(c − sma50(now)) < −0.25·atr` (trend structure broken) | 20 |
//! | flow | `d·(c − c0) ≥ 1.0·atr` (price follows the flow) | `d·(c − anchor) < −0.25·atr`, anchor = birth bar's low (`up`) / high (`down`) | 10 |
//! | neutral flow / unknown evidence | `|c − c0| ≥ 1.0·atr` (neutral) or the directional 1·atr barrier | opposite 1·atr (directional only) | kind's |
//!
//! Invalidation is checked before realization on the same bar. The rule is stateless
//! (no cache): it reads `s.facts[0]`, `s.story` and `bars[..=now]` only, and never a bar
//! after `now` even if a longer slice is passed.

use qp_capsule::{OpenStory, OutcomeRule, Verdict};
use qp_schema::capsule::{Direction, MeaningKind, TimeKind};
use qp_schema::Bar;

/// Bars in the ATR window (ending at the birth bar).
pub const ATR_BARS: usize = 14;
/// Fewest bars a true-range mean is taken over.
pub const MIN_TR_BARS: usize = 5;

/// Base horizon in bars per kind (before `h_scale`).
#[must_use]
pub fn base_horizon(kind: MeaningKind) -> u32 {
    match kind {
        MeaningKind::Breakout | MeaningKind::Flow => 10,
        MeaningKind::Reversal | MeaningKind::Continuation => 20,
        MeaningKind::Squeeze => 30,
        MeaningKind::Extreme => 15,
    }
}

/// Base thresholds (ATR multiples, before `t_scale`).
pub mod thr {
    /// Breakout: extension from the birth close.
    pub const BRK_EXT: f64 = 1.0;
    /// Breakout: close back inside the range, beyond the level.
    pub const BRK_FAIL: f64 = 0.25;
    /// Reversal: close beyond the divergence's second swing.
    pub const REV_INV: f64 = 0.25;
    /// Reversal: extension when the neckline was already crossed at birth.
    pub const REV_EXT: f64 = 1.0;
    /// Squeeze: close outside the birth band.
    pub const SQZ_OUT: f64 = 0.25;
    /// Squeeze: mean TR of the last 5 bars, as a multiple of the birth ATR.
    pub const SQZ_VOL: f64 = 1.0;
    /// Extreme: distance to the mean still counted as "back to the mean".
    pub const EXT_NEAR: f64 = 0.25;
    /// Extreme: further stretch from the birth close.
    pub const EXT_INV: f64 = 1.5;
    /// Continuation: extension from the birth close.
    pub const CONT_EXT: f64 = 1.5;
    /// Continuation: close beyond SMA50 on the wrong side.
    pub const CONT_INV: f64 = 0.25;
    /// Flow: price follows from the birth close.
    pub const FLOW_EXT: f64 = 1.0;
    /// Flow: close beyond the birth bar's opposite extreme.
    pub const FLOW_INV: f64 = 0.25;
    /// Fallback barrier.
    pub const GEN: f64 = 1.0;
}

/// The semantic outcome rule.
#[derive(Debug, Clone)]
pub struct Semantic {
    /// Name used in reports.
    pub label: &'static str,
    /// Horizon multiplier.
    pub h_scale: f64,
    /// Threshold multiplier.
    pub t_scale: f64,
}

impl Default for Semantic {
    fn default() -> Self {
        Self {
            label: "semantic",
            h_scale: 1.0,
            t_scale: 1.0,
        }
    }
}

/// Entry point for the harness.
#[must_use]
pub fn outcome() -> Box<dyn OutcomeRule> {
    Box::new(Semantic::default())
}

/// Horizon ±20 % and threshold ±10 % variants.
#[must_use]
pub fn perturbations() -> Vec<(String, Box<dyn OutcomeRule>)> {
    let v = |name: &str, label: &'static str, h: f64, t: f64| -> (String, Box<dyn OutcomeRule>) {
        (
            name.to_owned(),
            Box::new(Semantic {
                label,
                h_scale: h,
                t_scale: t,
            }),
        )
    };
    vec![
        v("h-20%", "semantic.h-20", 0.8, 1.0),
        v("h+20%", "semantic.h+20", 1.2, 1.0),
        v("k-10%", "semantic.k-10", 1.0, 0.9),
        v("k+10%", "semantic.k+10", 1.0, 1.1),
    ]
}

fn sign(d: Direction) -> f64 {
    match d {
        Direction::Up => 1.0,
        Direction::Down => -1.0,
        Direction::Neutral => 0.0,
    }
}

fn true_range(bars: &[Bar], i: usize) -> f64 {
    let b = &bars[i];
    if i == 0 {
        b.h - b.l
    } else {
        let pc = bars[i - 1].c;
        (b.h - b.l).max((b.h - pc).abs()).max((b.l - pc).abs())
    }
}

/// Mean true range of the `n` bars ending at `end` (inclusive); fewer bars when the
/// series is shorter. `None` with fewer than `min(n, MIN_TR_BARS)` bars or a result that
/// is not positive.
#[must_use]
pub fn mean_tr(bars: &[Bar], end: usize, n: usize) -> Option<f64> {
    if end >= bars.len() {
        return None;
    }
    let need = n.min(MIN_TR_BARS);
    let n = n.min(end + 1);
    if n == 0 || n < need {
        return None;
    }
    let mut sum = 0.0;
    for i in end + 1 - n..=end {
        sum += true_range(bars, i);
    }
    #[allow(clippy::cast_precision_loss)]
    let m = sum / n as f64;
    (m.is_finite() && m > 0.0).then_some(m)
}

/// Simple moving average of closes over `n` bars ending at `end` (inclusive).
#[must_use]
pub fn sma_close(bars: &[Bar], end: usize, n: usize) -> Option<f64> {
    if n == 0 || end + 1 < n || end >= bars.len() {
        return None;
    }
    let mut sum = 0.0;
    for b in &bars[end + 1 - n..=end] {
        sum += b.c;
    }
    #[allow(clippy::cast_precision_loss)]
    Some(sum / n as f64)
}

type Out = Option<(Verdict, &'static str)>;

impl Semantic {
    fn k(&self, base: f64) -> f64 {
        base * self.t_scale
    }

    fn generic(&self, d: f64, c: f64, c0: f64, atr: f64) -> Out {
        let b = self.k(thr::GEN) * atr;
        let mv = c - c0;
        if d == 0.0 {
            return (mv.abs() >= b).then_some((Verdict::Realized, "sem.generic.move"));
        }
        if d * mv <= -b {
            Some((Verdict::Invalidated, "sem.generic.stop"))
        } else if d * mv >= b {
            Some((Verdict::Realized, "sem.generic.target"))
        } else {
            None
        }
    }
}

fn ev(s: &OpenStory, key: &str) -> Option<f64> {
    s.facts
        .first()?
        .evidence
        .get(key)
        .copied()
        .filter(|v| v.is_finite())
}

impl OutcomeRule for Semantic {
    fn name(&self) -> &'static str {
        self.label
    }

    fn horizon(&self, kind: MeaningKind, _time: TimeKind) -> u32 {
        let h = f64::from(base_horizon(kind)) * self.h_scale;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let h = h.round().max(1.0) as u32;
        h
    }

    #[allow(clippy::too_many_lines)]
    fn judge(&self, s: &OpenStory, bars: &[Bar], now: u64) -> Out {
        let born = usize::try_from(s.story.born_bar).ok()?;
        let now = usize::try_from(now).ok()?;
        if now <= born || now >= bars.len() {
            return None;
        }
        // Never look past `now`, whatever slice we were given.
        let bars = &bars[..=now];
        let atr = mean_tr(bars, born, ATR_BARS)?;
        let c = bars[now].c;
        let c0 = bars[born].c;
        let d = sign(s.story.direction);

        if d == 0.0 && s.story.kind != MeaningKind::Squeeze {
            return self.generic(d, c, c0, atr);
        }

        match s.story.kind {
            MeaningKind::Breakout => {
                let Some(level) = ev(s, "level") else {
                    return self.generic(d, c, c0, atr);
                };
                if d * (c - level) < -self.k(thr::BRK_FAIL) * atr {
                    Some((Verdict::Invalidated, "sem.breakout.failed"))
                } else if d * (c - c0) >= self.k(thr::BRK_EXT) * atr {
                    Some((Verdict::Realized, "sem.breakout.extended"))
                } else {
                    None
                }
            }
            MeaningKind::Reversal => {
                let (Some(p1), Some(p2), Some(p2_price)) =
                    (ev(s, "p1_bar"), ev(s, "p2_bar"), ev(s, "p2_price"))
                else {
                    return self.generic(d, c, c0, atr);
                };
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let (p1, p2) = (p1.max(0.0) as usize, p2.max(0.0) as usize);
                if p1 > p2 || p2 >= born {
                    return self.generic(d, c, c0, atr);
                }
                let win = &bars[p1..=p2];
                // Neckline: the swing extreme of the prior move between the two peaks.
                let neck = if d > 0.0 {
                    win.iter().map(|b| b.h).fold(f64::NEG_INFINITY, f64::max)
                } else {
                    win.iter().map(|b| b.l).fold(f64::INFINITY, f64::min)
                };
                if d * (c - p2_price) < -self.k(thr::REV_INV) * atr {
                    return Some((Verdict::Invalidated, "sem.reversal.new_extreme"));
                }
                if d * (c0 - neck) > 0.0 {
                    // Neckline already crossed at detection: ask for a further move.
                    return (d * (c - c0) >= self.k(thr::REV_EXT) * atr)
                        .then_some((Verdict::Realized, "sem.reversal.extended"));
                }
                (d * (c - neck) > 0.0).then_some((Verdict::Realized, "sem.reversal.neckline"))
            }
            MeaningKind::Squeeze => {
                let (Some(up), Some(lo)) = (ev(s, "upper"), ev(s, "lower")) else {
                    return self.generic(0.0, c, c0, atr);
                };
                let m = self.k(thr::SQZ_OUT) * atr;
                let outside = c > up + m || c < lo - m;
                let vol = mean_tr(bars, now, 5).unwrap_or(0.0);
                (outside && vol >= self.k(thr::SQZ_VOL) * atr)
                    .then_some((Verdict::Realized, "sem.squeeze.expanded"))
            }
            MeaningKind::Extreme => {
                if d * (c - c0) >= self.k(thr::EXT_INV) * atr {
                    return Some((Verdict::Invalidated, "sem.extreme.stretched"));
                }
                let mean = sma_close(bars, now, 20)?;
                (d * (c - mean) <= self.k(thr::EXT_NEAR) * atr)
                    .then_some((Verdict::Realized, "sem.extreme.reverted"))
            }
            MeaningKind::Continuation => {
                if let Some(ma50) = sma_close(bars, now, 50) {
                    if d * (c - ma50) < -self.k(thr::CONT_INV) * atr {
                        return Some((Verdict::Invalidated, "sem.continuation.broke_ma50"));
                    }
                }
                (d * (c - c0) >= self.k(thr::CONT_EXT) * atr)
                    .then_some((Verdict::Realized, "sem.continuation.extended"))
            }
            MeaningKind::Flow => {
                let anchor = if d > 0.0 { bars[born].l } else { bars[born].h };
                if d * (c - anchor) < -self.k(thr::FLOW_INV) * atr {
                    Some((Verdict::Invalidated, "sem.flow.reversed"))
                } else if d * (c - c0) >= self.k(thr::FLOW_EXT) * atr {
                    Some((Verdict::Realized, "sem.flow.followed"))
                } else {
                    None
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
