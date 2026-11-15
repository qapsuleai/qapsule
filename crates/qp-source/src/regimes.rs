//! Regime synthetic market (R3/T2): deterministic daily (and hourly) bars aligned to the
//! BIST calendar (`qp-calendar`, `XIST`; K14: a daily bar's `t` is the session open in
//! UTC), driven by a schedule of market regimes — rising, falling, sideways, volatile.
//!
//! Like [`crate::synthetic_bars`] it uses an integer LCG and IEEE basic operations only
//! (`+ - * /`, `sqrt`, `abs`, `round`, `min`, `max`; no `exp`/`ln`), so results are bit
//! identical on every target. No clock, no `rand`.
//!
//! The process is built so that the classic events of the fact detectors occur:
//! - **Trends** with drift that fades in the last third of a rising/falling regime
//!   (price still makes marginal new extremes while momentum weakens: divergences).
//! - **Sideways** regimes mean-revert to a range centre and their volatility contracts
//!   towards the end (Bollinger squeeze); the next regime starts with a breakout day
//!   (a jump in the new direction on high volume), so range breakouts follow squeezes.
//! - **Volatility clustering** (a GARCH(1,1)-like scale), occasional jumps (fat tails),
//!   a shared market factor across instruments.
//! - **Volume** (turnover, TRY) clusters (AR(1) state), rises with |return|, spikes on big
//!   moves and sometimes spikes without a price move.

use qp_calendar::{builtin, days_from_civil, ExchangeCalendar, Interval};
use qp_schema::Bar;

use crate::synthetic::{round2, Lcg};

/// A market regime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Regime {
    /// Up trend: positive drift, moderate volatility.
    Rising,
    /// Down trend: negative drift, somewhat higher volatility.
    Falling,
    /// Range-bound: no drift, mean reversion to a centre, contracting volatility.
    Sideways,
    /// High volatility, no drift, more jumps.
    Volatile,
}

impl Regime {
    /// All regimes, in a fixed order.
    pub const ALL: [Regime; 4] = [
        Regime::Rising,
        Regime::Falling,
        Regime::Sideways,
        Regime::Volatile,
    ];

    /// Lower-case name (`rising`, `falling`, `sideways`, `volatile`).
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Regime::Rising => "rising",
            Regime::Falling => "falling",
            Regime::Sideways => "sideways",
            Regime::Volatile => "volatile",
        }
    }

    /// The default parameters of this regime.
    #[must_use]
    pub fn spec(self) -> RegimeSpec {
        match self {
            Regime::Rising => RegimeSpec {
                drift: 0.003,
                vol: 0.015,
                reversion: 0.0,
                jump_prob: 0.012,
                vol_end: 1.0,
            },
            Regime::Falling => RegimeSpec {
                drift: -0.003,
                vol: 0.018,
                reversion: 0.0,
                jump_prob: 0.015,
                vol_end: 1.0,
            },
            Regime::Sideways => RegimeSpec {
                drift: 0.0,
                vol: 0.013,
                reversion: 0.06,
                jump_prob: 0.004,
                vol_end: 0.35,
            },
            Regime::Volatile => RegimeSpec {
                drift: 0.0,
                vol: 0.034,
                reversion: 0.02,
                jump_prob: 0.04,
                vol_end: 1.0,
            },
        }
    }
}

/// Parameters of one regime (per trading day).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RegimeSpec {
    /// Mean daily return at the start of the regime. Trends fade linearly to a third of
    /// it over the last third of the regime.
    pub drift: f64,
    /// Daily return volatility (standard deviation, before clustering).
    pub vol: f64,
    /// Mean-reversion speed towards the price at the start of the regime (0 = none).
    pub reversion: f64,
    /// Probability of a jump day (|shock| of 3–6 sigma, high volume).
    pub jump_prob: f64,
    /// Volatility multiplier reached at the end of the regime (linear from 1); below 1
    /// contracts the range (squeeze).
    pub vol_end: f64,
}

/// One regime over a range of trading days `[start, end)` (indices into the daily bars).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegimeSpan {
    /// The regime.
    pub regime: Regime,
    /// First trading day (inclusive).
    pub start: usize,
    /// Last trading day (exclusive).
    pub end: usize,
}

/// The regime of trading day `day` in `schedule` (`None` past the end).
#[must_use]
pub fn regime_at(schedule: &[RegimeSpan], day: usize) -> Option<Regime> {
    schedule
        .iter()
        .find(|s| s.start <= day && day < s.end)
        .map(|s| s.regime)
}

/// One synthetic instrument: symbol, regime schedule (trading-day indices) and bars.
#[derive(Debug, Clone, PartialEq)]
pub struct SyntheticInstrument {
    /// `SYN01`, `SYN02`, …
    pub symbol: String,
    /// Regime schedule over trading-day indices (for daily bars: bar indices).
    pub schedule: Vec<RegimeSpan>,
    /// The bars.
    pub bars: Vec<Bar>,
}

/// First trading day of the synthetic market: 2023-01-02 (a Monday).
pub const MARKET_START: (i64, u32, u32) = (2023, 1, 2);

fn xist() -> ExchangeCalendar {
    // The built-in XIST calendar exists and is valid by construction.
    let cal = builtin("XIST").expect("built-in XIST calendar");
    ExchangeCalendar::new(&cal).expect("built-in XIST calendar is valid")
}

/// The open times (UTC ms) of the first `n` XIST trading days from [`MARKET_START`].
#[must_use]
pub fn trading_day_opens(n: usize) -> Vec<i64> {
    let cal = xist();
    let (y, m, d) = MARKET_START;
    let mut day = days_from_civil(y, m, d);
    let mut out = Vec::with_capacity(n);
    while out.len() < n {
        if let Some(span) = cal.day_span(day) {
            out.push(span.open);
        }
        day += 1;
    }
    out
}

/// The 60-minute bar opens (UTC ms) of the first `n_days` XIST trading days, grouped by
/// day (bars aligned to the session open; the last bar is cut at the session close).
#[must_use]
pub fn trading_hour_opens(n_days: usize) -> Vec<Vec<i64>> {
    let cal = xist();
    let (y, m, d) = MARKET_START;
    let mut day = days_from_civil(y, m, d);
    let mut out = Vec::with_capacity(n_days);
    while out.len() < n_days {
        if let Some(span) = cal.day_span(day) {
            let bars = cal.bars_between(Interval::Minutes(60), span.open, span.close);
            out.push(bars.iter().map(|b| b.open).collect());
        }
        day += 1;
    }
    out
}

/// SplitMix64 finaliser: decorrelates derived seeds.
fn mix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

impl Lcg {
    /// Uniform in [0, 1).
    fn u01(&mut self) -> f64 {
        f64::from(self.next_u32()) / 4_294_967_296.0
    }

    /// Approximately standard normal: sum of 6 uniforms on [-1, 1) (variance 2), scaled.
    fn normal(&mut self) -> f64 {
        let mut s = 0.0;
        for _ in 0..6 {
            s += self.unit();
        }
        s / std::f64::consts::SQRT_2
    }

    /// Integer in `[lo, hi]`.
    fn range(&mut self, lo: usize, hi: usize) -> usize {
        let span = u64::try_from(hi - lo + 1).unwrap_or(1);
        lo + usize::try_from(u64::from(self.next_u32()) % span).unwrap_or(0)
    }
}

/// A random regime schedule covering `n_days` trading days: spans of 25–110 days, the
/// regime switching by a fixed transition table (never the same regime twice in a row;
/// sideways often precedes a trend).
#[must_use]
pub fn regime_schedule(n_days: usize, seed: u64) -> Vec<RegimeSpan> {
    let mut rng = Lcg(mix(seed ^ 0x5EED_0001));
    let mut out = Vec::new();
    let mut regime = Regime::ALL[rng.range(0, 3)];
    let mut start = 0;
    while start < n_days {
        let len = match regime {
            Regime::Volatile => rng.range(15, 50),
            _ => rng.range(25, 110),
        };
        let end = (start + len).min(n_days);
        out.push(RegimeSpan { regime, start, end });
        start = end;
        let u = rng.u01();
        regime = match regime {
            Regime::Sideways => {
                if u < 0.4 {
                    Regime::Rising
                } else if u < 0.75 {
                    Regime::Falling
                } else {
                    Regime::Volatile
                }
            }
            Regime::Rising | Regime::Falling => {
                if u < 0.55 {
                    Regime::Sideways
                } else if u < 0.8 {
                    Regime::Volatile
                } else if regime == Regime::Rising {
                    Regime::Falling
                } else {
                    Regime::Rising
                }
            }
            Regime::Volatile => {
                if u < 0.5 {
                    Regime::Sideways
                } else if u < 0.75 {
                    Regime::Rising
                } else {
                    Regime::Falling
                }
            }
        };
    }
    out
}

/// Per-instrument constants.
#[derive(Debug, Clone, Copy)]
struct Profile {
    /// Starting price.
    price: f64,
    /// Typical number of shares traded per day.
    shares: f64,
    /// Sensitivity to the market factor.
    beta: f64,
}

/// Shared market shocks, one per trading day (deterministic in `seed`).
fn market_factor(n_days: usize, seed: u64) -> Vec<f64> {
    let mut rng = Lcg(mix(seed ^ 0x003A_11E7));
    (0..n_days).map(|_| rng.normal()).collect()
}

/// Daily state of the price process.
struct Process {
    rng: Lcg,
    close: f64,
    /// GARCH-like variance multiplier (long-run mean 1).
    h: f64,
    /// Previous standardized shock.
    prev_z: f64,
    /// Volume AR(1) state.
    vstate: f64,
    /// Range centre of the current regime.
    centre: f64,
    /// Index of the current regime span.
    span: usize,
}

/// Result of one day's draw.
struct DayDraw {
    /// Return from previous close to this close.
    ret: f64,
    /// Overnight gap (part of `ret`).
    gap: f64,
    /// Effective daily sigma.
    sigma: f64,
    /// Volume multiplier for the day (≥ 0.2).
    vmult: f64,
}

const GARCH_A: f64 = 0.08;
const GARCH_B: f64 = 0.88;

impl Process {
    fn new(seed: u64, price: f64) -> Self {
        Self {
            rng: Lcg(seed),
            close: price,
            h: 1.0,
            prev_z: 0.0,
            vstate: 0.0,
            centre: price,
            span: usize::MAX,
        }
    }

    /// Draws day `day` under `schedule` with market shock `mz`.
    fn draw(&mut self, schedule: &[RegimeSpan], day: usize, mz: f64, beta: f64) -> DayDraw {
        let si = schedule
            .iter()
            .position(|s| s.start <= day && day < s.end)
            .unwrap_or(schedule.len().saturating_sub(1));
        let span = schedule[si];
        let spec = span.regime.spec();
        let first_of_span = si != self.span;
        if first_of_span {
            self.span = si;
            self.centre = self.close;
        }
        // Position inside the span, 0..1.
        let len = (span.end - span.start).max(1);
        let pos = f64::from(u32::try_from(day - span.start).unwrap_or(0))
            / f64::from(u32::try_from(len).unwrap_or(1));
        // Trend drift fades over the last third (divergence material).
        let fade = if pos > 2.0 / 3.0 {
            1.0 - (pos - 2.0 / 3.0) * 2.0
        } else {
            1.0
        };
        let drift = spec.drift * fade;
        let vol_mult = 1.0 + (spec.vol_end - 1.0) * pos;
        // Volatility clustering.
        self.h = (1.0 - GARCH_A - GARCH_B) + GARCH_A * self.prev_z * self.prev_z + GARCH_B * self.h;
        self.h = self.h.clamp(0.3, 6.0);
        let sigma = spec.vol * vol_mult * self.h.sqrt();
        // Shock: idiosyncratic + market factor (unit variance overall).
        let w = beta.clamp(0.0, 0.9);
        let mut z = self.rng.normal() * (1.0 - w * w).sqrt() + mz * w;
        let mut vmult = 1.0;
        let mut jump = false;
        if self.rng.u01() < spec.jump_prob {
            let size = 3.0 + 3.0 * self.rng.u01();
            z = if self.rng.unit() < 0.0 { -size } else { size };
            jump = true;
        }
        // Breakout day at the start of a trend after a sideways regime.
        let after_sideways = si > 0 && schedule[si - 1].regime == Regime::Sideways;
        if first_of_span && after_sideways {
            match span.regime {
                Regime::Rising => z = 2.5 + 1.5 * self.rng.u01(),
                Regime::Falling => z = -(2.5 + 1.5 * self.rng.u01()),
                Regime::Volatile => z += 3.0 * z.signum(),
                Regime::Sideways => {}
            }
            jump = true;
        }
        let reversion = -spec.reversion * (self.close / self.centre - 1.0);
        let ret = (drift + reversion + sigma * z).clamp(-0.095, 0.095);
        self.prev_z = z.clamp(-6.0, 6.0) / (1.0 + 0.5 * f64::from(u8::from(jump)));
        // Volume: AR(1) state, |shock| response, spikes with and without price moves.
        self.vstate = 0.75 * self.vstate + 0.25 * self.rng.unit() * 1.5;
        vmult *= (1.0 + 0.6 * self.vstate).max(0.3);
        vmult *= 0.6 + 0.4 * z.abs();
        if jump {
            vmult *= 2.0 + 2.5 * self.rng.u01();
        }
        if self.rng.u01() < 0.015 {
            // A volume spike with no particular price move.
            vmult *= 2.5 + 2.5 * self.rng.u01();
        }
        let gap = sigma * 0.25 * self.rng.unit();
        DayDraw {
            ret,
            gap,
            sigma,
            vmult: vmult.max(0.2),
        }
    }
}

/// Bars for one instrument: daily bars at `opens`, regimes from `schedule` (indexed by
/// bar), starting from `price`, using `market` shocks (one per bar, missing = 0).
fn daily_bars(
    opens: &[i64],
    schedule: &[RegimeSpan],
    seed: u64,
    profile: Profile,
    market: &[f64],
) -> Vec<Bar> {
    let mut p = Process::new(seed, profile.price);
    let mut bars = Vec::with_capacity(opens.len());
    for (i, &t) in opens.iter().enumerate() {
        let mz = market.get(i).copied().unwrap_or(0.0);
        let d = p.draw(schedule, i, mz, profile.beta);
        let prev = p.close;
        let o = round2((prev * (1.0 + d.gap)).max(0.01));
        let c = round2((prev * (1.0 + d.ret)).max(0.01));
        let wick_h = d.sigma * 0.6 * p.rng.u01();
        let wick_l = d.sigma * 0.6 * p.rng.u01();
        let h = round2(o.max(c) * (1.0 + wick_h));
        let l = round2((o.min(c) * (1.0 - wick_l)).max(0.01));
        let v = round2(profile.shares * d.vmult * c);
        bars.push(Bar { t, o, h, l, c, v });
        p.close = c;
    }
    bars
}

/// Daily bars for `schedule` (one bar per trading day from [`MARKET_START`]; the bar
/// count is the schedule's end), with starting price `price`. No market factor.
#[must_use]
pub fn regime_bars(schedule: &[RegimeSpan], seed: u64, price: f64) -> Vec<Bar> {
    let n = schedule.last().map_or(0, |s| s.end);
    if n == 0 {
        return Vec::new();
    }
    let opens = trading_day_opens(n);
    let profile = Profile {
        price,
        shares: 1_000_000.0,
        beta: 0.0,
    };
    daily_bars(&opens, schedule, mix(seed), profile, &[])
}

fn profile(rng: &mut Lcg) -> Profile {
    // Prices 5–300 TRY, daily share volume 0.3–30 M, beta 0.2–0.7.
    let price = round2(5.0 + 295.0 * rng.u01() * rng.u01());
    let shares = 300_000.0 + 29_700_000.0 * rng.u01() * rng.u01();
    let beta = 0.2 + 0.5 * rng.u01();
    Profile {
        price,
        shares,
        beta,
    }
}

/// `n_instruments` instruments (`SYN01`, `SYN02`, …) of `n_days` daily bars each, aligned
/// to the XIST calendar from [`MARKET_START`], each with its own regime schedule and a
/// shared market factor. Deterministic in `seed`.
#[must_use]
pub fn synthetic_market(
    n_instruments: usize,
    n_days: usize,
    seed: u64,
) -> Vec<SyntheticInstrument> {
    let opens = trading_day_opens(n_days);
    let market = market_factor(n_days, seed);
    let mut prng = Lcg(mix(seed ^ 0xF00D));
    (0..n_instruments)
        .map(|i| {
            let iseed =
                mix(seed.wrapping_add(u64::try_from(i).unwrap_or(0).wrapping_mul(0x1000_0001)));
            let schedule = regime_schedule(n_days, iseed);
            let prof = profile(&mut prng);
            let bars = daily_bars(&opens, &schedule, iseed, prof, &market);
            SyntheticInstrument {
                symbol: format!("SYN{:02}", i + 1),
                schedule,
                bars,
            }
        })
        .collect()
}

/// Intraday volume profile (U shape) for up to 9 hourly bars; normalised by the caller.
const HOUR_PROFILE: [f64; 9] = [1.7, 1.1, 0.9, 0.8, 0.75, 0.8, 0.95, 1.3, 1.0];

/// Like [`synthetic_market`] but with 60-minute bars inside the XIST sessions; the
/// schedule is indexed by **trading day** (not by bar). Each day's return follows the
/// same regime process, spread over the day's hourly bars.
#[must_use]
pub fn synthetic_market_hourly(
    n_instruments: usize,
    n_days: usize,
    seed: u64,
) -> Vec<SyntheticInstrument> {
    let days = trading_hour_opens(n_days);
    let market = market_factor(n_days, seed);
    let mut prng = Lcg(mix(seed ^ 0xF00D));
    (0..n_instruments)
        .map(|i| {
            let iseed =
                mix(seed.wrapping_add(u64::try_from(i).unwrap_or(0).wrapping_mul(0x1000_0001)));
            let schedule = regime_schedule(n_days, iseed);
            let prof = profile(&mut prng);
            let mut p = Process::new(iseed, prof.price);
            let mut bars = Vec::with_capacity(n_days * 8);
            for (di, hours) in days.iter().enumerate() {
                let d = p.draw(&schedule, di, market[di], prof.beta);
                let k = hours.len().max(1);
                let kf = f64::from(u32::try_from(k).unwrap_or(1));
                let step_sigma = d.sigma / kf.sqrt();
                let target = p.close * (1.0 + d.ret);
                let mut prev = p.close;
                let mut profile_sum = 0.0;
                for j in 0..k {
                    profile_sum += HOUR_PROFILE[j.min(8)];
                }
                for (j, &t) in hours.iter().enumerate() {
                    let o = if j == 0 {
                        round2((prev * (1.0 + d.gap)).max(0.01))
                    } else {
                        prev
                    };
                    // Bridge towards the day's target close with hourly noise.
                    let left = f64::from(u32::try_from(k - j).unwrap_or(1));
                    let pull = (target - o) / left;
                    let c = if j + 1 == k {
                        round2(target.max(0.01))
                    } else {
                        round2((o + pull + o * step_sigma * p.rng.normal() * 0.8).max(0.01))
                    };
                    let wick_h = step_sigma * 0.5 * p.rng.u01();
                    let wick_l = step_sigma * 0.5 * p.rng.u01();
                    let h = round2(o.max(c) * (1.0 + wick_h));
                    let l = round2((o.min(c) * (1.0 - wick_l)).max(0.01));
                    let share = HOUR_PROFILE[j.min(8)] / profile_sum;
                    let v = round2(prof.shares * d.vmult * share * c * (0.8 + 0.4 * p.rng.u01()));
                    bars.push(Bar { t, o, h, l, c, v });
                    prev = c;
                }
                p.close = prev;
            }
            SyntheticInstrument {
                symbol: format!("SYN{:02}", i + 1),
                schedule,
                bars,
            }
        })
        .collect()
}
