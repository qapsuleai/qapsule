//! M4 labeled episode set: series built by splicing templates into a quiet base, each
//! template occurrence an **episode** with a true id.
//!
//! Layout of one instrument: 150 quiet warm-up bars, then 10 slots; each slot = one
//! template (seeded choice and direction) followed by a 35-bar quiet gap (the gap lets
//! open stories close before the next episode). Quiet bars: mean-reverting noise around
//! the price at the gap start (σ = 1 %/bar), ordinary volume.
//!
//! Templates (`d` = ±1, seeded):
//! - `breakout_volume_continuation`: a close beyond the 20-bar extreme with 4× volume,
//!   then 4 continuation bars (1.5× volume).
//! - `squeeze_breakout`: 30 bars of contracting volatility (0.6σ → 0.15σ, low volume),
//!   then the breakout bar and 3 continuation bars.
//! - `pair_same`: episode A (breakout + 3 continuation), an 8-bar reversal and 6 quiet
//!   bars (unlabeled), then episode B in the **same** direction: two episodes.
//! - `pair_opposite`: the same with episode B in the **opposite** direction.
//! - `extreme_enter_exit`: 9 strong bars in `d` (RSI into the extreme zone), 7 bars back.
//! - `divergence_breakdown`: 12-bar rise, 3-bar pullback, 6-bar slower rise to a marginal
//!   new extreme, 4-bar breakdown against it with 3× volume.
//!
//! **Labeling rule.** Episode window = the template's own bars plus a tail of 3 bars.
//! A fact whose bar lies in a window gets that episode's id; any other fact is
//! *background* (`-1`). **Limits:** detectors with long look-back can fire after the
//! tail because of an episode (late state exits, e.g. `squeeze_end`, `bull_end`) — those
//! facts are background; a noise-driven fact that happens inside a window is labeled with
//! that episode; the reversal between the two breakouts of a pair is unlabeled on purpose
//! (it belongs to neither breakout). The labels say which bars an episode spans, not what
//! a detector "meant".

use qp_capsule::InstrumentInput;
use qp_schema::Bar;
use serde::{Deserialize, Serialize};

use crate::common::{round2, Dataset, Lcg};

/// Template names by code.
pub const TEMPLATES: [&str; 6] = [
    "breakout_volume_continuation",
    "squeeze_breakout",
    "pair_same",
    "pair_opposite",
    "extreme_enter_exit",
    "divergence_breakdown",
];

/// Instruments in the labeled set.
pub const EP_INSTRUMENTS: usize = 24;
/// Template slots per instrument.
pub const EP_SLOTS: usize = 10;
/// Quiet warm-up bars.
pub const EP_WARMUP: usize = 150;
/// Quiet gap after every template.
pub const EP_GAP: usize = 35;
/// Labeled tail after a template's last bar.
pub const EP_TAIL: usize = 3;

/// One labeled episode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Episode {
    /// Episode id (unique in the dataset).
    pub id: u32,
    /// Instrument index.
    pub inst: u32,
    /// First bar.
    pub start: u32,
    /// End bar (exclusive), tail included.
    pub end: u32,
    /// Template code.
    pub template: u8,
    /// Part within the template (0, or 1 for the second breakout of a pair).
    pub part: u8,
    /// Direction `d` of the part (0 up, 1 down).
    pub dir: u8,
}

struct Builder {
    bars: Vec<Bar>,
    rng: Lcg,
    price: f64,
    shares: f64,
    sigma: f64,
    anchor: f64,
}

impl Builder {
    fn push(&mut self, ret: f64, vmult: f64) {
        let o = round2((self.price * (1.0 + 0.002 * self.rng.unit())).max(0.01));
        let c = round2((self.price * (1.0 + ret)).max(0.01));
        let wick = self.sigma * 0.5;
        let h = round2(o.max(c) * (1.0 + wick * self.rng.u01()));
        let l = round2((o.min(c) * (1.0 - wick * self.rng.u01())).max(0.01));
        let v = round2(self.shares * c * vmult * (0.85 + 0.3 * self.rng.u01()));
        self.bars.push(Bar {
            t: 0,
            o,
            h,
            l,
            c,
            v,
        });
        self.price = c;
    }

    fn quiet(&mut self, n: usize, smult: f64, vmult: f64) {
        for _ in 0..n {
            let rev = 0.08 * (self.anchor / self.price - 1.0);
            let ret = self.sigma * smult * 0.7 * self.rng.normal() + rev;
            self.push(ret, vmult);
        }
    }

    fn breakout(&mut self, d: f64) {
        let n = self.bars.len();
        let from = n.saturating_sub(20);
        let hh = self.bars[from..]
            .iter()
            .map(|b| b.h)
            .fold(f64::MIN, f64::max);
        let ll = self.bars[from..]
            .iter()
            .map(|b| b.l)
            .fold(f64::MAX, f64::min);
        let extra = 0.006 + 0.004 * self.rng.u01();
        let target = if d > 0.0 {
            hh * (1.0 + extra)
        } else {
            ll * (1.0 - extra)
        };
        let mut ret = target / self.price - 1.0;
        if ret * d < 2.5 * self.sigma {
            ret = d * 2.5 * self.sigma;
        }
        self.push(ret, 4.0);
    }

    fn trend(&mut self, n: usize, d: f64, lo: f64, hi: f64, vmult: f64) {
        for _ in 0..n {
            let m = lo + (hi - lo) * self.rng.u01();
            self.push(d * m * self.sigma, vmult);
        }
    }
}

/// Build the labeled set of `seed`.
#[must_use]
pub fn build(seed: u64) -> (Dataset, Vec<Episode>) {
    let mut inputs = Vec::new();
    let mut eps = Vec::new();
    let mut day_of_bar = Vec::new();
    let mut max_len = 0;
    for i in 0..EP_INSTRUMENTS {
        let mut b = Builder {
            bars: Vec::new(),
            rng: Lcg::new(seed.wrapping_mul(0x9E37).wrapping_add(i as u64 * 131 + 7)),
            price: 0.0,
            shares: 0.0,
            sigma: 0.01,
            anchor: 0.0,
        };
        b.price = round2(20.0 + 180.0 * b.rng.u01());
        b.anchor = b.price;
        b.shares = 1_000_000.0 * (0.5 + 4.0 * b.rng.u01());
        b.quiet(EP_WARMUP, 1.0, 1.0);
        let inst = u32::try_from(i).unwrap_or(0);
        let add = |eps: &mut Vec<Episode>, start: usize, end: usize, t: usize, part: u8, d: f64| {
            eps.push(Episode {
                id: u32::try_from(eps.len()).unwrap_or(0),
                inst,
                start: u32::try_from(start).unwrap_or(0),
                end: u32::try_from(end + EP_TAIL).unwrap_or(0),
                template: u8::try_from(t).unwrap_or(0),
                part,
                dir: u8::from(d < 0.0),
            });
        };
        for _ in 0..EP_SLOTS {
            let t = b.rng.below(TEMPLATES.len());
            let d = if b.rng.u01() < 0.5 { 1.0 } else { -1.0 };
            let s = b.bars.len();
            match t {
                0 => {
                    b.breakout(d);
                    b.trend(4, d, 0.6, 1.2, 1.5);
                    add(&mut eps, s, b.bars.len(), t, 0, d);
                }
                1 => {
                    b.anchor = b.price;
                    for k in 0..30u32 {
                        let sm = 0.6 - 0.45 * f64::from(k) / 29.0;
                        b.quiet(1, sm, 0.7);
                    }
                    b.breakout(d);
                    b.trend(3, d, 0.6, 1.2, 1.5);
                    add(&mut eps, s, b.bars.len(), t, 0, d);
                }
                2 | 3 => {
                    b.breakout(d);
                    b.trend(3, d, 0.6, 1.2, 1.5);
                    add(&mut eps, s, b.bars.len(), t, 0, d);
                    b.trend(8, -d, 0.6, 1.2, 1.1);
                    b.anchor = b.price;
                    b.quiet(6, 1.0, 1.0);
                    let d2 = if t == 2 { d } else { -d };
                    let s2 = b.bars.len();
                    b.breakout(d2);
                    b.trend(3, d2, 0.6, 1.2, 1.5);
                    add(&mut eps, s2, b.bars.len(), t, 1, d2);
                }
                4 => {
                    b.trend(9, d, 1.8, 2.4, 1.2);
                    b.trend(7, -d, 0.6, 1.2, 1.0);
                    add(&mut eps, s, b.bars.len(), t, 0, d);
                }
                _ => {
                    b.trend(12, d, 1.1, 1.7, 1.1);
                    b.trend(3, -d, 1.2, 1.8, 1.0);
                    b.trend(6, d, 0.8, 1.2, 0.9);
                    b.trend(4, -d, 2.2, 2.8, 3.0);
                    add(&mut eps, s, b.bars.len(), t, 0, -d);
                }
            }
            b.anchor = b.price;
            b.quiet(EP_GAP, 1.0, 1.0);
        }
        max_len = max_len.max(b.bars.len());
        inputs.push((format!("EP{:02}", i + 1), b.bars));
    }
    let opens = qp_source::trading_day_opens(max_len);
    let inputs: Vec<InstrumentInput> = inputs
        .into_iter()
        .map(|(sym, mut bars)| {
            for (k, x) in bars.iter_mut().enumerate() {
                x.t = opens[k];
            }
            day_of_bar.push(
                (0..bars.len())
                    .map(|k| u32::try_from(k).unwrap_or(0))
                    .collect(),
            );
            InstrumentInput {
                series: format!("{sym}.1day"),
                symbol: sym,
                bars,
                flow: None,
            }
        })
        .collect();
    (
        Dataset {
            name: format!("episodes-s{seed}"),
            inputs,
            day_of_bar,
            regime: None,
            n_days: u32::try_from(max_len).unwrap_or(0),
            first_day: 0,
        },
        eps,
    )
}

/// Episode id of a fact on instrument `inst` at `bar` (`-1` = background).
#[must_use]
pub fn label(eps: &[Episode], inst: u32, bar: u64) -> i64 {
    eps.iter()
        .find(|e| e.inst == inst && u64::from(e.start) <= bar && bar < u64::from(e.end))
        .map_or(-1, |e| i64::from(e.id))
}
