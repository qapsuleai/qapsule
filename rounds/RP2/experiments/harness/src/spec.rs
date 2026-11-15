//! Scenario parameters (`ScenarioSpec`). Every scenario preset (S1–S19, `scenarios.rs`)
//! is one value of this type; an auditor adds a scenario by writing it as JSON
//! (`harness spec S7 > my.json`, edit, `harness run --spec my.json`).
//!
//! Times in a spec are milliseconds (`f64`); inside the simulation they become integer
//! microseconds ([`VTime`]). Outage and disconnect windows are relative to the start of
//! the live phase.

use serde::{Deserialize, Serialize};

/// Virtual time in microseconds since the start of the run. There is no wall clock.
pub type VTime = i64;

/// Milliseconds (spec) to virtual microseconds.
#[must_use]
pub fn ms(x: f64) -> VTime {
    #[allow(clippy::cast_possible_truncation)]
    let v = (x * 1000.0).round() as i64;
    v
}

/// Start of the live phase: the first live bar opens here. The initial snapshots are
/// delivered from time 0.
pub const LIVE0: VTime = 1_000_000;

/// Open time (epoch ms) of live bar 0. History bars lie before it.
pub const BASE_T: i64 = 1_700_000_040_000;

/// A delay distribution: `base + U(0, jitter)`, plus with probability `spike_p` an extra
/// `spike_ms * U(0.5, 1.5)`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Delay {
    /// Constant part (ms).
    pub base_ms: f64,
    /// Uniform jitter on top (ms).
    pub jitter_ms: f64,
    /// Probability of a delay spike.
    pub spike_p: f64,
    /// Size of a spike (ms).
    pub spike_ms: f64,
}

impl Delay {
    /// A constant delay.
    #[must_use]
    pub fn constant(ms: f64) -> Self {
        Self {
            base_ms: ms,
            ..Self::default()
        }
    }

    /// Base plus uniform jitter.
    #[must_use]
    pub fn jitter(base_ms: f64, jitter_ms: f64) -> Self {
        Self {
            base_ms,
            jitter_ms,
            ..Self::default()
        }
    }

    /// Draws one delay in virtual microseconds.
    pub fn sample(&self, rng: &mut crate::rng::Rng) -> VTime {
        let mut d = self.base_ms;
        if self.jitter_ms > 0.0 {
            d += rng.range_f64(0.0, self.jitter_ms);
        }
        if rng.chance(self.spike_p) {
            d += self.spike_ms * rng.range_f64(0.5, 1.5);
        }
        ms(d.max(0.0))
    }
}

/// A time window relative to the start of the live phase (ms).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Window {
    /// Start (ms after live start).
    pub from_ms: f64,
    /// End (ms after live start).
    pub to_ms: f64,
}

impl Window {
    /// True if virtual time `t` lies inside.
    #[must_use]
    pub fn contains(&self, t: VTime) -> bool {
        t >= LIVE0 + ms(self.from_ms) && t < LIVE0 + ms(self.to_ms)
    }

    /// Virtual end time.
    #[must_use]
    pub fn end(&self) -> VTime {
        LIVE0 + ms(self.to_ms)
    }
}

/// How a feed represents prices.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PriceRepr {
    /// IEEE double, as the source has it.
    #[default]
    F64,
    /// Rounded through IEEE single (Matriks `TradeMessage.price` is a 32-bit float).
    F32,
}

/// One receiver's market-data feed (the authority's or the replica's).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FeedSpec {
    /// Delivery delay.
    pub delay: Delay,
    /// Probability that an event is delivered out of order (delayed by an extra
    /// `U(0, reorder_ms)` without holding back later events).
    pub reorder_p: f64,
    /// Maximum extra delay of a reordered event (ms).
    pub reorder_ms: f64,
    /// Probability that a tick is lost (QoS 0). Corrections and snapshots are not dropped.
    pub drop_p: f64,
    /// Probability that the last tick of a bar (the one that decides its close) is lost.
    pub drop_last_tick_p: f64,
    /// Probability that a tick that sets a new bar high is lost.
    pub drop_high_tick_p: f64,
    /// Probability that an event is delivered twice; the copy comes `U(0, dup_ms)` later.
    pub dup_p: f64,
    /// Maximum lag of a duplicate (ms).
    pub dup_ms: f64,
    /// Windows in which nothing is delivered (events are lost).
    pub outages: Vec<Window>,
    /// At the end of an outage the source sends a full snapshot (Matriks `REFRESH`).
    pub refresh_on_reconnect: bool,
    /// Conflation window (ms; 0 = off): patches of the same bar inside one window are
    /// merged into one event; the conflated stream is renumbered (its own `seq`).
    pub conflate_ms: f64,
    /// Price representation.
    pub price_repr: PriceRepr,
    /// Probability that an event's prices are wrong (faulty feed).
    pub glitch_p: f64,
    /// Relative size of a glitch (price × (1 ± size)).
    pub glitch_size: f64,
    /// Initial snapshot: only the last `n` history bars (`{"last": n}` binding). `None`:
    /// the whole history (the receiver's anchor is the world's bar 0).
    pub window_last: Option<u32>,
}

impl Default for FeedSpec {
    fn default() -> Self {
        Self {
            delay: Delay::default(),
            reorder_p: 0.0,
            reorder_ms: 0.0,
            drop_p: 0.0,
            drop_last_tick_p: 0.0,
            drop_high_tick_p: 0.0,
            dup_p: 0.0,
            dup_ms: 0.0,
            outages: Vec::new(),
            refresh_on_reconnect: true,
            conflate_ms: 0.0,
            price_repr: PriceRepr::F64,
            glitch_p: 0.0,
            glitch_size: 0.0,
            window_last: None,
        }
    }
}

/// One direction of the protocol channel (opaque bytes).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ChannelSpec {
    /// Delivery delay.
    pub delay: Delay,
    /// Probability that a message is delivered out of order.
    pub reorder_p: f64,
    /// Maximum extra delay of a reordered message (ms).
    pub reorder_ms: f64,
    /// Probability that a message is delivered twice.
    pub dup_p: f64,
    /// Maximum lag of a duplicate (ms).
    pub dup_ms: f64,
    /// Probability that a message held during a disconnect is delivered twice at
    /// reconnection (resend around a reconnect).
    pub dup_on_reconnect_p: f64,
}

/// The provider's settled-bar source ("REST"): closed bars as the provider has
/// finalised them, derived from the world, answered after a delay.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RestSpec {
    /// Answer delay (ms).
    pub delay_ms: f64,
    /// A bar is settled this long after its close (ms).
    pub settle_lag_ms: f64,
}

impl Default for RestSpec {
    fn default() -> Self {
        Self {
            delay_ms: 800.0,
            settle_lag_ms: 2000.0,
        }
    }
}

/// One step of a steering plan: drive RSI(14) of every binding towards `level`, then
/// keep it there for `bars` bars, oscillating within the bar by `amp` RSI points.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SteerStep {
    /// Target RSI level.
    pub level: f64,
    /// Bars to hold (once near the level).
    pub bars: u32,
    /// Oscillation amplitude inside a bar (RSI points).
    pub amp: f64,
    /// Oscillations per bar.
    pub osc: f64,
    /// Spread of the bar close around the level (RSI points).
    pub close_spread: f64,
    /// Noise on the intra-bar RSI path (RSI points).
    pub noise: f64,
}

impl Default for SteerStep {
    fn default() -> Self {
        Self {
            level: 70.0,
            bars: 1000,
            amp: 2.5,
            osc: 3.0,
            close_spread: 2.0,
            noise: 0.15,
        }
    }
}

/// Close race: inside a steered bar RSI stays `margin` below the level and the last
/// tick (at `before_close_ms` before the close) decides: with `cross_p` it lifts RSI
/// `margin` above the level.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Race {
    /// Distance from the level (RSI points).
    pub margin: f64,
    /// Time of the deciding tick before the close (ms).
    pub before_close_ms: f64,
    /// Probability that the deciding tick crosses.
    pub cross_p: f64,
}

impl Default for Race {
    fn default() -> Self {
        Self {
            margin: 1.0,
            before_close_ms: 150.0,
            cross_p: 0.5,
        }
    }
}

/// Shape of the live price path.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Shape {
    /// Regime-switching random walk (RSI regularly crosses 70/30).
    #[default]
    Free,
    /// RSI steered along a plan (cycled), optionally with a close race.
    Steer {
        /// Plan steps, cycled.
        plan: Vec<SteerStep>,
        /// Close race inside steered bars.
        #[serde(default)]
        race: Option<Race>,
    },
}

/// Kind of a historical bar correction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CorrectionKind {
    /// Inside the evidence window of the latest closed cross: the claim falls (S9a).
    DropClaim,
    /// Inside the evidence window of the latest closed cross: the claim stands (S9b).
    KeepClaim,
    /// A recent closed bar is corrected so that a new cross appears there (S9d, an
    /// addition: a correction that creates a claim).
    CreateClaim,
    /// A bar `bars_back` bars before the live bar (S9c).
    Old {
        /// Bars before the live bar.
        bars_back: u32,
    },
}

/// A correction of a past bar, issued in the middle of live bar `at_bar`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CorrectionSpec {
    /// Live bar index.
    pub at_bar: u32,
    /// What is corrected.
    #[serde(flatten)]
    pub kind: CorrectionKind,
}

/// A split / scale adjustment of the whole history (all bars ×`factor`), in the middle
/// of live bar `at_bar`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RescaleSpec {
    /// Live bar index.
    pub at_bar: u32,
    /// Price factor.
    pub factor: f64,
}

/// The reference detector (shared by all approaches).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DetectorSpec {
    /// RSI period.
    pub period: u32,
    /// Upper level (cross above).
    pub upper: f64,
    /// Lower level (cross below).
    pub lower: f64,
    /// Reference invalidation condition (S18): RSI crosses below `level` within
    /// `lookback` bars after a cross above `upper`.
    pub invalidation: Option<InvalidationSpec>,
    /// Two-binding claim (S16): RSI of both bindings above `upper` at the same bar time
    /// (and not both at the bar before).
    pub pair: bool,
}

impl Default for DetectorSpec {
    fn default() -> Self {
        Self {
            period: 14,
            upper: 70.0,
            lower: 30.0,
            invalidation: None,
            pair: false,
        }
    }
}

/// Reference invalidation condition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct InvalidationSpec {
    /// Level whose downward cross invalidates.
    pub level: f64,
    /// Bars after the cross above `upper` in which it counts.
    pub lookback: u32,
}

impl Default for InvalidationSpec {
    fn default() -> Self {
        Self {
            level: 60.0,
            lookback: 20,
        }
    }
}

/// Oracle thresholds (ms).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct OracleSpec {
    /// Δ_miss: a final-true claim not displayed within this time after it became true is
    /// missed (M5).
    pub miss_ms: f64,
    /// Δ_stale: a stale display episode longer than this is counted (M7).
    pub stale_ms: f64,
    /// A wrong-blocking episode longer than this is counted (M8).
    pub block_ms: f64,
    /// Window after a correction in which display changes are attributed to it (M11).
    pub corr_window_ms: f64,
    /// Grace for provisional wrongness (M3p): a provisional show is wrong only if the
    /// claim has not held on the true live bar within this time before the show.
    /// 0 = strict definition of RESEARCH §1.3.
    pub live_grace_ms: f64,
}

impl Default for OracleSpec {
    fn default() -> Self {
        Self {
            miss_ms: 30_000.0,
            stale_ms: 5_000.0,
            block_ms: 5_000.0,
            corr_window_ms: 30_000.0,
            live_grace_ms: 0.0,
        }
    }
}

/// One scenario.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ScenarioSpec {
    /// Short name (`S7`, or anything for a custom spec).
    pub name: String,
    /// What it tests.
    pub description: String,
    /// Number of bindings (instruments), all on the same interval.
    pub bindings: u32,
    /// Bar interval (ms). The feed key says `1m` for 60 000; other values are `<n>s`.
    pub interval_ms: i64,
    /// History bars before the live phase (in the authority's initial snapshot).
    pub history_bars: u32,
    /// Live bars generated tick by tick (one more bar is opened to close the last).
    pub live_bars: u32,
    /// Mean ticks per live bar (each bar draws `U(0.7, 1.3)` × this).
    pub ticks_per_bar: u32,
    /// Bar volatility (fraction of price).
    pub volatility: f64,
    /// Price tick of live prices (0 = no rounding). History bars are always in cents.
    pub price_tick: f64,
    /// Shape of the live path.
    pub shape: Shape,
    /// Authority's feed.
    pub authority_feed: FeedSpec,
    /// Replica's feed.
    pub replica_feed: FeedSpec,
    /// Replica feed overrides per binding index (S16: one binding lags).
    pub replica_feed_overrides: Vec<(u32, FeedSpec)>,
    /// Authority feed overrides per binding index.
    pub authority_feed_overrides: Vec<(u32, FeedSpec)>,
    /// Authority → replica channel.
    pub a2r: ChannelSpec,
    /// Replica → authority channel.
    pub r2a: ChannelSpec,
    /// Channel disconnects (both directions).
    pub disconnects: Vec<Window>,
    /// Settled-bar source.
    pub rest: RestSpec,
    /// Historical corrections.
    pub corrections: Vec<CorrectionSpec>,
    /// Splits.
    pub rescales: Vec<RescaleSpec>,
    /// Spontaneous source snapshots (`REFRESH`) in the middle of these live bars.
    pub refreshes: Vec<u32>,
    /// Reference detector.
    pub detector: DetectorSpec,
    /// Extra streaming studies per binding in the shared engine (load, S17).
    pub extra_studies: u32,
    /// Oracle thresholds.
    pub oracle: OracleSpec,
    /// Simulated time after the last world event (ms) so late messages arrive.
    pub drain_ms: f64,
    /// Seeds to run by default.
    pub default_seeds: u32,
}

impl Default for ScenarioSpec {
    fn default() -> Self {
        Self {
            name: "custom".to_owned(),
            description: String::new(),
            bindings: 1,
            interval_ms: 60_000,
            history_bars: 400,
            live_bars: 60,
            ticks_per_bar: 30,
            volatility: 0.004,
            price_tick: 0.01,
            shape: Shape::Free,
            authority_feed: FeedSpec::default(),
            replica_feed: FeedSpec::default(),
            replica_feed_overrides: Vec::new(),
            authority_feed_overrides: Vec::new(),
            a2r: ChannelSpec::default(),
            r2a: ChannelSpec::default(),
            disconnects: Vec::new(),
            rest: RestSpec::default(),
            corrections: Vec::new(),
            rescales: Vec::new(),
            refreshes: Vec::new(),
            detector: DetectorSpec::default(),
            extra_studies: 0,
            oracle: OracleSpec::default(),
            drain_ms: 90_000.0,
            default_seeds: 100,
        }
    }
}

impl ScenarioSpec {
    /// The replica's feed spec for binding `b`.
    #[must_use]
    pub fn replica_feed_for(&self, b: u32) -> &FeedSpec {
        self.replica_feed_overrides
            .iter()
            .find(|(i, _)| *i == b)
            .map_or(&self.replica_feed, |(_, f)| f)
    }

    /// The authority's feed spec for binding `b`.
    #[must_use]
    pub fn authority_feed_for(&self, b: u32) -> &FeedSpec {
        self.authority_feed_overrides
            .iter()
            .find(|(i, _)| *i == b)
            .map_or(&self.authority_feed, |(_, f)| f)
    }

    /// Interval string of the feed key.
    #[must_use]
    pub fn interval_name(&self) -> String {
        if self.interval_ms % 60_000 == 0 {
            format!("{}m", self.interval_ms / 60_000)
        } else {
            format!("{}s", self.interval_ms / 1000)
        }
    }

    /// Basic sanity checks.
    ///
    /// # Errors
    /// A message naming the bad field.
    pub fn validate(&self) -> Result<(), String> {
        if self.bindings == 0 || self.bindings > 500 {
            return Err("bindings must be 1..=500".into());
        }
        if self.interval_ms < 1000 {
            return Err("interval_ms must be at least 1000".into());
        }
        if self.history_bars < self.detector.period + 2 {
            return Err("history_bars must exceed the RSI period".into());
        }
        if self.ticks_per_bar == 0 {
            return Err("ticks_per_bar must be at least 1".into());
        }
        if self.detector.pair && self.bindings < 2 {
            return Err("detector.pair needs at least 2 bindings".into());
        }
        Ok(())
    }
}
