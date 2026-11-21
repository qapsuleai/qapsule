//! RP4 M26 alternative `olcek26`: the product's seven detectors (same ids) with windows
//! and thresholds **scaled by the bar period** (`NOTES.md`).
//!
//! The harness factory takes no arguments (`fn() -> Vec<Box<dyn Detector>>`), so each
//! detector learns the period from its own bars: [`Scaled`] buffers the first
//! [`DECIDE_BARS`] bars, takes the smallest gap between consecutive bar times, picks the
//! period class ([`Class`]) and builds the inner detector with that class's constants,
//! replaying the buffered bars into it. Every inner detector's warm-up is longer than
//! [`DECIDE_BARS`], so the replay emits nothing and the wrapper's output equals the inner
//! detector's output bar for bar (tested).
//!
//! Scale: `z ∈ [0, 1]` = `ln(bars per 8 h session) / ln(96)`, i.e. 0 at daily, 1 at
//! 5 min (written as literals per class: no `ln` at run time). Every scaled constant is
//! `daily + z·(five_min − daily)` (windows: multiplier `1 + z·(M − 1)`; the 5 min ends
//! are in [`at5m`]). At daily (`z = 0`) the detectors are exactly `qp_facts::default_detectors()` (same constructors,
//! same version), so the daily behaviour does not change.
//!
//! Pure and deterministic: no clock, network, randomness or file access; `f64` basic
//! operations only.

use qp_facts::{
    BandSqueeze, Detection, Detector, FlowPoint, InstitutionalFlow, MaStack, RangeBreakout,
    RsiDivergence, RsiExtreme, VolumeSurge,
};
use qp_schema::capsule::{EvidenceFamily, License};
use qp_schema::Bar;

/// Bars buffered before the period is decided (≥ 2 same-day bars for any intraday period).
pub const DECIDE_BARS: usize = 3;

/// Window multipliers and thresholds at 5 min (`z = 1`); 1.0 / the product value means
/// "not scaled". Chosen on the calibration split only (`NOTES.md`, trials).
pub mod at5m {
    /// `breakout.range` window multiplier.
    pub const BREAKOUT_WINDOW: f64 = 3.0;
    /// `squeeze.bandwidth` bandwidth look-back multiplier.
    pub const SQUEEZE_LOOKBACK: f64 = 3.0;
    /// `extreme.rsi` RSI length multiplier.
    pub const RSI_LENGTH: f64 = 1.0;
    /// `extreme.rsi` band: `upper = 50 + band`, `lower = 50 − band` (product 20).
    pub const RSI_BAND: f64 = 20.0;
    /// `flow.volume_surge` average window multiplier.
    pub const VOL_WINDOW: f64 = 1.0;
    /// `flow.volume_surge` multiplier (product 2.5).
    pub const VOL_MULT: f64 = 4.0;
    /// `continuation.ma_stack` fast/slow window multiplier.
    pub const MA_WINDOW: f64 = 1.0;
    /// `reversal.rsi_divergence` swing gap multiplier.
    pub const DIV_GAP: f64 = 1.0;
}

/// Version of a scaled (intraday) detector; daily keeps the product version.
pub const SCALED_VERSION: u32 = 2;

/// Period class decided from the smallest bar gap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// Gap < 15 min (5 min bars).
    Min5,
    /// 15 min ≤ gap < 1 h.
    Min15,
    /// 1 h ≤ gap < 8 h.
    Hour1,
    /// Gap ≥ 8 h (daily and longer): product constants.
    Daily,
}

impl Class {
    /// Class from the smallest gap between consecutive bar times (ms).
    #[must_use]
    pub fn from_gap_ms(gap: i64) -> Self {
        const MIN: i64 = 60_000;
        if gap < 15 * MIN {
            Self::Min5
        } else if gap < 60 * MIN {
            Self::Min15
        } else if gap < 8 * 60 * MIN {
            Self::Hour1
        } else {
            Self::Daily
        }
    }

    /// `z = ln(bars per 8 h) / ln(96)`: 5 min 1, 15 min ln 32 / ln 96, 1 h ln 8 / ln 96.
    #[must_use]
    pub fn z(self) -> f64 {
        match self {
            Self::Min5 => 1.0,
            Self::Min15 => 0.759_31,
            Self::Hour1 => 0.455_59,
            Self::Daily => 0.0,
        }
    }
}

/// `a + z·(b − a)`.
#[must_use]
pub fn lerp(a: f64, b: f64, z: f64) -> f64 {
    a + z * (b - a)
}

/// A product window times multiplier `m` (rounded to nearest, never below the product).
#[must_use]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
pub fn win(w: usize, m: f64) -> usize {
    let s = (w as f64 * m).round() as usize;
    s.max(w)
}

/// Same for `u64` gaps.
#[must_use]
pub fn gap(g: u64, m: f64) -> u64 {
    win(usize::try_from(g).unwrap_or(usize::MAX), m) as u64
}

/// The scaled constants of one class (multipliers are `1 + z·(M − 1)`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Params {
    /// `breakout.range` window multiplier.
    pub breakout_window: f64,
    /// `squeeze.bandwidth` look-back multiplier.
    pub squeeze_lookback: f64,
    /// `extreme.rsi` length multiplier.
    pub rsi_length: f64,
    /// `extreme.rsi` band.
    pub rsi_band: f64,
    /// `flow.volume_surge` window multiplier.
    pub vol_window: f64,
    /// `flow.volume_surge` multiplier.
    pub vol_mult: f64,
    /// `continuation.ma_stack` window multiplier.
    pub ma_window: f64,
    /// `reversal.rsi_divergence` gap multiplier.
    pub div_gap: f64,
}

impl Params {
    /// Constants at scale `z`.
    #[must_use]
    pub fn at(z: f64) -> Self {
        Self {
            breakout_window: lerp(1.0, at5m::BREAKOUT_WINDOW, z),
            squeeze_lookback: lerp(1.0, at5m::SQUEEZE_LOOKBACK, z),
            rsi_length: lerp(1.0, at5m::RSI_LENGTH, z),
            rsi_band: lerp(20.0, at5m::RSI_BAND, z),
            vol_window: lerp(1.0, at5m::VOL_WINDOW, z),
            vol_mult: lerp(VolumeSurge::MULT, at5m::VOL_MULT, z),
            ma_window: lerp(1.0, at5m::MA_WINDOW, z),
            div_gap: lerp(1.0, at5m::DIV_GAP, z),
        }
    }
}

/// Which product detector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Which {
    /// `continuation.ma_stack`
    MaStack,
    /// `reversal.rsi_divergence`
    RsiDivergence,
    /// `breakout.range`
    RangeBreakout,
    /// `squeeze.bandwidth`
    BandSqueeze,
    /// `extreme.rsi`
    RsiExtreme,
    /// `flow.volume_surge`
    VolumeSurge,
    /// `flow.institutional` (not scaled: no flow feed intraday in RP4)
    InstitutionalFlow,
}

impl Which {
    /// Product order (`qp_facts::default_detectors`).
    pub const ALL: [Self; 7] = [
        Self::MaStack,
        Self::RsiDivergence,
        Self::RangeBreakout,
        Self::BandSqueeze,
        Self::RsiExtreme,
        Self::VolumeSurge,
        Self::InstitutionalFlow,
    ];

    /// Whether the class changes this detector's constants (5 min endpoint ≠ product).
    #[must_use]
    #[allow(clippy::float_cmp)]
    pub fn scaled(self, class: Class) -> bool {
        if class == Class::Daily {
            return false;
        }
        match self {
            Self::MaStack => at5m::MA_WINDOW != 1.0,
            Self::RsiDivergence => at5m::DIV_GAP != 1.0,
            Self::RangeBreakout => at5m::BREAKOUT_WINDOW != 1.0,
            Self::BandSqueeze => at5m::SQUEEZE_LOOKBACK != 1.0,
            Self::RsiExtreme => at5m::RSI_LENGTH != 1.0 || at5m::RSI_BAND != 20.0,
            Self::VolumeSurge => at5m::VOL_WINDOW != 1.0 || at5m::VOL_MULT != VolumeSurge::MULT,
            Self::InstitutionalFlow => false,
        }
    }

    /// The detector for a class. Daily = product default constructor.
    ///
    /// # Panics
    /// Never: every scaled window is ≥ the product window (valid).
    #[must_use]
    pub fn build(self, class: Class) -> Box<dyn Detector> {
        if !self.scaled(class) {
            return self.product();
        }
        let p = Params::at(class.z());
        match self {
            Self::MaStack => Box::new(
                MaStack::new(
                    win(MaStack::FAST, p.ma_window),
                    win(MaStack::SLOW, p.ma_window),
                    MaStack::ATR,
                    MaStack::RISE,
                )
                .expect("valid")
                .detector(),
            ),
            Self::RsiDivergence => Box::new(
                RsiDivergence::new(
                    RsiDivergence::RSI,
                    gap(RsiDivergence::MIN_GAP, p.div_gap),
                    gap(RsiDivergence::MAX_GAP, p.div_gap),
                )
                .expect("valid")
                .detector(),
            ),
            Self::RangeBreakout => Box::new(
                RangeBreakout::new(
                    win(RangeBreakout::WINDOW, p.breakout_window),
                    RangeBreakout::ATR,
                )
                .expect("valid")
                .detector(),
            ),
            Self::BandSqueeze => Box::new(
                BandSqueeze::new(
                    BandSqueeze::PERIOD,
                    BandSqueeze::K,
                    win(BandSqueeze::LOOKBACK, p.squeeze_lookback),
                    BandSqueeze::EXIT_MULT,
                )
                .expect("valid")
                .detector(),
            ),
            Self::RsiExtreme => Box::new(
                RsiExtreme::new(
                    win(RsiExtreme::RSI, p.rsi_length),
                    50.0 + p.rsi_band,
                    50.0 - p.rsi_band,
                )
                .expect("valid")
                .detector(),
            ),
            Self::VolumeSurge => Box::new(
                VolumeSurge::new(win(VolumeSurge::WINDOW, p.vol_window), p.vol_mult)
                    .expect("valid")
                    .detector(),
            ),
            Self::InstitutionalFlow => self.product(),
        }
    }

    /// Product detector with default constants.
    #[must_use]
    pub fn product(self) -> Box<dyn Detector> {
        match self {
            Self::MaStack => Box::new(MaStack::default().detector()),
            Self::RsiDivergence => Box::new(RsiDivergence::default().detector()),
            Self::RangeBreakout => Box::new(RangeBreakout::default().detector()),
            Self::BandSqueeze => Box::new(BandSqueeze::default().detector()),
            Self::RsiExtreme => Box::new(RsiExtreme::default().detector()),
            Self::VolumeSurge => Box::new(VolumeSurge::default().detector()),
            Self::InstitutionalFlow => Box::new(InstitutionalFlow::default().detector()),
        }
    }
}

/// A product detector whose constants are chosen from the bar period (see crate docs).
pub struct Scaled {
    which: Which,
    proto: Box<dyn Detector>,
    inner: Option<(Class, Box<dyn Detector>)>,
    seen: Vec<(Bar, Option<FlowPoint>)>,
    pending: Option<(Bar, Option<FlowPoint>)>,
}

impl Scaled {
    /// A fresh detector (no bars).
    #[must_use]
    pub fn new(which: Which) -> Self {
        Self {
            which,
            proto: which.product(),
            inner: None,
            seen: Vec::with_capacity(DECIDE_BARS),
            pending: None,
        }
    }

    /// The decided class (after [`DECIDE_BARS`] bars).
    #[must_use]
    pub fn class(&self) -> Option<Class> {
        self.inner.as_ref().map(|(c, _)| *c)
    }

    fn stage(&mut self, bar: &Bar, flow: Option<&FlowPoint>) -> Vec<Detection> {
        self.pending = Some((bar.clone(), flow.cloned()));
        if self.seen.len() + 1 < DECIDE_BARS {
            return Vec::new();
        }
        let mut gap = i64::MAX;
        let mut prev: Option<i64> = None;
        for (b, _) in &self.seen {
            if let Some(p) = prev {
                gap = gap.min(b.t - p);
            }
            prev = Some(b.t);
        }
        if let Some(p) = prev {
            gap = gap.min(bar.t - p);
        }
        let class = Class::from_gap_ms(gap);
        let mut inner = self.which.build(class);
        for (b, f) in std::mem::take(&mut self.seen) {
            let out = inner.push(&b, f.as_ref());
            debug_assert!(out.is_empty(), "warm-up longer than DECIDE_BARS");
        }
        self.pending = None;
        let out = inner.push(bar, flow);
        self.inner = Some((class, inner));
        out
    }
}

impl Detector for Scaled {
    fn id(&self) -> &'static str {
        self.proto.id()
    }
    fn version(&self) -> u32 {
        match &self.inner {
            Some((c, _)) if self.which.scaled(*c) => SCALED_VERSION,
            _ => self.proto.version(),
        }
    }
    fn family(&self) -> EvidenceFamily {
        self.proto.family()
    }
    fn license(&self) -> License {
        self.proto.license()
    }
    fn warmup(&self) -> usize {
        match &self.inner {
            Some((_, d)) => d.warmup(),
            None => self.proto.warmup().max(DECIDE_BARS),
        }
    }
    fn push(&mut self, bar: &Bar, flow: Option<&FlowPoint>) -> Vec<Detection> {
        if let Some((_, d)) = self.inner.as_mut() {
            return d.push(bar, flow);
        }
        if let Some(p) = self.pending.take() {
            self.seen.push(p);
        }
        self.stage(bar, flow)
    }
    fn update_last(&mut self, bar: &Bar, flow: Option<&FlowPoint>) -> Vec<Detection> {
        if let Some((_, d)) = self.inner.as_mut() {
            return d.update_last(bar, flow);
        }
        self.stage(bar, flow)
    }
}

/// The `olcek26` set: product ids and order, period-scaled constants (fresh each call).
#[must_use]
pub fn detectors() -> Vec<Box<dyn Detector>> {
    Which::ALL
        .iter()
        .map(|w| Box::new(Scaled::new(*w)) as Box<dyn Detector>)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use qp_facts::run_batch;

    /// Deterministic bars: `n` bars `gap_ms` apart inside 96-bar days (overnight gap).
    fn series(n: usize, gap_ms: i64) -> Vec<Bar> {
        let mut out = Vec::with_capacity(n);
        let mut x: u64 = 0x2026_1121;
        let mut c = 100.0_f64;
        let day = 86_400_000_i64;
        let per_day = if gap_ms >= day { 1 } else { 96 };
        for i in 0..n {
            x = x
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            #[allow(clippy::cast_precision_loss)]
            let u = ((x >> 11) as f64) / ((1_u64 << 53) as f64) - 0.5;
            let o = c;
            // Waves of shrinking amplitude on a slow zig-zag trend, plus noise: swings
            // with diverging momentum (divergences), breakouts, squeezes, extremes.
            #[allow(clippy::cast_precision_loss)]
            let (tri, amp, trend) = {
                let tri = ((i % 12) as f64 - 6.0).abs() / 6.0 - 0.5;
                let amp = 4.0 - 3.5 * ((i % 120) as f64) / 120.0;
                let k = (i % 400) as f64;
                let trend = if k < 200.0 {
                    0.05 * k
                } else {
                    0.05 * (400.0 - k)
                };
                (tri, amp, trend)
            };
            c = 100.0 + trend + amp * tri + 0.6 * u;
            let i = i64::try_from(i).expect("small");
            let d = i / per_day;
            let k = i % per_day;
            let t = if per_day == 1 {
                d * day
            } else {
                d * day + k * gap_ms
            };
            #[allow(clippy::cast_precision_loss)]
            let v = 1_000.0
                * (1.5 + u)
                * if (per_day > 1 && k == 0) || i % 37 == 0 {
                    6.0
                } else {
                    1.0
                };
            out.push(Bar {
                t,
                o,
                // Wick depends on the bar's own noise: no ties with the neighbours.
                h: o.max(c) * (1.003 + 0.002 * u),
                l: o.min(c) * (0.997 - 0.002 * u),
                c,
                v,
            });
        }
        out
    }

    #[test]
    fn same_ids_and_order_as_product() {
        let a: Vec<_> = detectors().iter().map(|d| d.id()).collect();
        let b: Vec<_> = qp_facts::default_detectors()
            .iter()
            .map(|d| d.id())
            .collect();
        assert_eq!(a, b);
    }

    #[test]
    fn classes() {
        assert_eq!(Class::from_gap_ms(300_000), Class::Min5);
        assert_eq!(Class::from_gap_ms(900_000), Class::Min15);
        assert_eq!(Class::from_gap_ms(3_600_000), Class::Hour1);
        assert_eq!(Class::from_gap_ms(86_400_000), Class::Daily);
        let p = Params::at(0.0);
        assert_eq!(win(20, p.breakout_window), 20);
        assert!((p.rsi_band - 20.0).abs() < 1e-12);
        assert!((p.vol_mult - VolumeSurge::MULT).abs() < 1e-12);
    }

    /// The wrapper equals the inner detector built directly, every class; daily equals
    /// the product set (ids, versions, detections).
    #[test]
    fn wrapper_equals_inner_and_daily_equals_product() {
        for (gap, class) in [
            (300_000, Class::Min5),
            (900_000, Class::Min15),
            (3_600_000, Class::Hour1),
            (86_400_000, Class::Daily),
        ] {
            let bars = series(1_500, gap);
            for w in Which::ALL {
                let mut a = Scaled::new(w);
                let mut b = w.build(class);
                let ra = run_batch(&mut a, &bars, None);
                let rb = run_batch(b.as_mut(), &bars, None);
                assert_eq!(ra, rb, "{w:?} {class:?}");
                if w != Which::InstitutionalFlow {
                    assert!(!ra.is_empty(), "no detections: {w:?} {class:?}");
                }
                assert_eq!(a.class(), Some(class));
                if class == Class::Daily {
                    let mut p = w.product();
                    assert_eq!(ra, run_batch(p.as_mut(), &bars, None));
                    assert_eq!(a.version(), p.version());
                } else if w.scaled(class) {
                    assert_eq!(a.version(), SCALED_VERSION);
                } else {
                    assert_eq!(a.version(), w.product().version());
                }
            }
        }
    }

    /// Streaming with revisions (`update_last`) = batch, also across the decision bar.
    #[test]
    fn revisions_equal_batch() {
        let bars = series(1_500, 300_000);
        for w in Which::ALL {
            let mut a = Scaled::new(w);
            let batch = run_batch(&mut a, &bars, None);
            let mut s = Scaled::new(w);
            let mut out = Vec::new();
            for (i, b) in bars.iter().enumerate() {
                let mut wrong = b.clone();
                wrong.c *= 1.03;
                wrong.v *= 7.0;
                let _ = s.push(&wrong, None);
                for d in s.update_last(b, None) {
                    out.push((i as u64, d));
                }
            }
            assert_eq!(batch, out, "{w:?}");
            assert!(w == Which::InstitutionalFlow || !out.is_empty(), "{w:?}");
        }
    }

    /// Intraday the scaled detectors differ from the product (the scaling is live).
    #[test]
    fn scaled_differs_intraday() {
        let bars = series(1_500, 300_000);
        for w in Which::ALL.into_iter().filter(|w| w.scaled(Class::Min5)) {
            let mut a = Scaled::new(w);
            let mut p = w.product();
            assert_ne!(
                run_batch(&mut a, &bars, None),
                run_batch(p.as_mut(), &bars, None),
                "{w:?}"
            );
        }
    }

    #[test]
    fn deterministic() {
        let bars = series(800, 900_000);
        let run = || {
            let mut all = Vec::new();
            for mut d in detectors() {
                all.extend(run_batch(d.as_mut(), &bars, None));
            }
            all
        };
        assert_eq!(run(), run());
    }
}
