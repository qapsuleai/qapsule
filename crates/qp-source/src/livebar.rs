//! Live bars from price-card updates (R6, M22; `rounds/R6/GOAL.md` scope 3–4).
//!
//! Matriks' live stream carries no bars: the price card (`SymbolMessage`) arrives as a full
//! snapshot after (re)subscription and then as field-level updates (an absent field means
//! "unchanged", never 0). Qapsule builds the live bar itself, aligned to the exchange
//! calendar (K14), and emits field-level [`DataEvent`]s (`docs/PROTOCOL.md` §18.4).
//!
//! This module is source-independent: callers translate their messages into [`CardTick`]
//! (only the fields that came). Quantex's bar rules (`docs/matriks/MESSAGES.md` §6: UTC
//! epoch alignment, carry bars, late ticks into the open bar) are deliberately **not**
//! inherited.
//!
//! Rules (each is tested in `tests/livebar.rs`):
//!
//! 1. **Alignment (K14).** A tick belongs to `cal.bar_at(interval, t)`. On a trading day a
//!    tick up to [`LiveConfig::pre_open_ms`] before the first session open (opening
//!    auction) joins the day's first bar; a tick up to [`LiveConfig::post_close_ms`] after
//!    the last session close (closing session, trades at the closing price) joins the
//!    day's last bar (same as the REST rule of R3: the 18:00 bar merges into the previous
//!    one). A tick in a break between sessions joins the earlier session's last bar. Other
//!    ticks (night, weekend, holiday) build nothing and are counted.
//! 2. **Events.** A new bar is one `bar` event with the first trade as `o = h = l = c`;
//!    later changes of the same bar are `patch` events holding only the changed fields
//!    (never empty). `h` never falls, `l` never rises, `v` never decreases.
//! 3. **Volume.** Bar volume = increase of the day's cumulative counter
//!    ([`VolumeUnit::Value`]: traded value in TL, like REST `volume`; or
//!    [`VolumeUnit::Quantity`]: lots). The counter restarts each trading day. A counter
//!    that goes backwards adds nothing (no negative volume) and is counted.
//! 4. **Methods (M22).** [`BarMethod::Ticks`]: bars from `last` and the cumulative counter.
//!    [`BarMethod::CardDaily`]: `1d` only, the bar is the card's day fields (`day_open`,
//!    `day_high`, `day_low`, `last`, cumulative counter). [`BarMethod::Mixed`]: `CardDaily`
//!    on `1d`, `Ticks` elsewhere, and a rise of the card's day high (fall of the day low)
//!    that is a new extreme for the current bar is written into that bar's `h` (`l`).
//! 5. **Provisional / final.** The last bar is live. It becomes final when the next bar
//!    opens or when [`LiveBars::close_until`] passes its end. A tick for a final bar
//!    (late tick) is not written anywhere and is counted (repair is the REST path).
//! 6. **Seam.** [`LiveBars::resume`] continues the last historical bar with `patch` events
//!    and starts the cumulative counter at the history's value, so history + live equals a
//!    series built live from the start.
//! 7. **Determinism.** The same ticks give the same events, byte for byte.
//! 8. **Trades (RP4, K66).** [`BarMethod::Trades`]: bars from the trade stream
//!    ([`LiveBars::push_trade`], one [`TradeTick`] per trade) instead of the price card.
//!    Alignment, opening auction (first bar), closing session (last bar), late and
//!    off-session handling are rule 1 and 5 unchanged. A trade sets `c`, raises `h`, lowers
//!    `l`; the first trade of a bar is `o = h = l = c`. Volume: [`VolumeUnit::Value`] adds
//!    `price · quantity` (TL), [`VolumeUnit::Quantity`] adds `quantity` (lots); the bar's
//!    lots are also kept apart ([`LiveBars::current_quantity`]). Special (reported)
//!    trades (`isTradeEx`, `mx/tradeex`) join the `1d` bar only
//!    ([`SpecialTrades::DayOnly`], the default; measured in RP4 M27: REST's daily bar
//!    holds them, its intraday bars do not), and there **only with their volume**: a
//!    special trade never sets `o`/`h`/`l`/`c` (REST's daily price fields exclude it); [`SpecialTrades::Exclude`] and
//!    [`SpecialTrades::Include`] keep them out of / in every interval. Under `Trades`
//!    a price-card tick builds nothing (the card still gives the day fields and the
//!    session phase to its other readers); under the card methods a trade builds nothing.
//!    Trades are pushed in exchange order (time, then trade number); summation order is
//!    push order, so the same trades in the same order give bit-equal bars.
//!
//! [`compare_bars`] measures live bars against REST bars (M22 table: counts only).

use std::collections::BTreeMap;
use std::fmt;

use qp_calendar::{ExchangeCalendar, Interval, Span};
use qp_schema::data::{DataChange, DataEvent, FeedKey};
use qp_schema::{Bar, TimestampMs};

use crate::bar_patch_diff;

mod compare;
mod push;

pub use compare::{compare_bars, compare_bars_with, BarDiff, DiffTolerance, FieldDiff};

const MS_PER_MIN: i64 = 60_000;

/// One decoded price-card update: only the fields that came (`None` = unchanged).
///
/// Field choice for Matriks `SymbolMessage` (made by the translator, not here):
/// `last` ← `last`; `day_open` ← `open`; `day_high`/`day_low` ← `dailyHigh`/`dailyLow`
/// (day, not session extremes); `cum_volume` ← `dailyVolume` (or `volume`);
/// `cum_quantity` ← `dailyQuantity` (or `quantity`); `phase` ← `stockStatus`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CardTick {
    /// Event time, UTC milliseconds, chosen by the caller (exchange time or arrival).
    pub t_ms: TimestampMs,
    /// First message after (re)subscription: a full snapshot.
    pub initial: bool,
    /// Last trade price.
    pub last: Option<f64>,
    /// Day open price.
    pub day_open: Option<f64>,
    /// Day high price.
    pub day_high: Option<f64>,
    /// Day low price.
    pub day_low: Option<f64>,
    /// Day's cumulative traded value (TL).
    pub cum_volume: Option<f64>,
    /// Day's cumulative traded quantity (lots).
    pub cum_quantity: Option<f64>,
    /// Session phase code (`stockStatus`). Carried for statistics; no bar rule uses it.
    pub phase: Option<i32>,
}

impl CardTick {
    /// An update at `t_ms` with no field set.
    #[must_use]
    pub fn at(t_ms: TimestampMs) -> Self {
        Self {
            t_ms,
            ..Self::default()
        }
    }
}

/// One trade (işlem) for [`LiveBars::push_trade`] (rule 8). The translator (Matriks
/// `TradeMessage`, REST `trade.gz`) fills it; buyer / seller are never part of it.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TradeTick {
    /// Trade time, UTC milliseconds (exchange time).
    pub t_ms: TimestampMs,
    /// Trade price.
    pub price: f64,
    /// Traded quantity (lots); `None` when the source did not send it (level 0).
    pub quantity: Option<f64>,
    /// A special (reported) trade (`isTradeEx`, `mx/tradeex`).
    pub special: bool,
}

/// Whether special (reported) trades join the bars (rule 8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SpecialTrades {
    /// Not in any bar (counted).
    Exclude,
    /// In every bar like any trade.
    Include,
    /// In the `1d` bar like any trade, in no intraday bar (counted there). The default:
    /// RP4 M27 measured it against REST on the calibration days (2026-10-05..10-16, 30
    /// symbols): REST's daily volume holds the special trades, its 1m / 5m / 1h bars do
    /// not (`rounds/RP4/experiments/M27/RESULTS.md`).
    #[default]
    DayOnly,
}

impl SpecialTrades {
    /// Whether a special trade joins a bar of `interval`.
    #[must_use]
    pub fn joins(self, interval: Interval) -> bool {
        match self {
            Self::Exclude => false,
            Self::Include => true,
            Self::DayOnly => interval == Interval::Day,
        }
    }

    /// Short name (`exclude`, `include`, `day_only`).
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Exclude => "exclude",
            Self::Include => "include",
            Self::DayOnly => "day_only",
        }
    }
}

/// Which cumulative counter is the bar volume.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VolumeUnit {
    /// Traded value (TL): [`CardTick::cum_volume`]. Same unit as REST `volume` (R3).
    #[default]
    Value,
    /// Traded quantity (lots): [`CardTick::cum_quantity`].
    Quantity,
}

/// How bars are built (M22 alternatives).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BarMethod {
    /// (a) From `last` and the cumulative counter only.
    #[default]
    Ticks,
    /// (b) `1d` only: the card's day fields are the bar.
    CardDaily,
    /// (c) `CardDaily` on `1d`; `Ticks` elsewhere plus new day extremes reflected into
    /// the current bar.
    Mixed,
    /// (RP4) From the trade stream only ([`LiveBars::push_trade`]), every interval.
    Trades,
}

impl BarMethod {
    /// Short name (`ticks`, `card_daily`, `mixed`).
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Ticks => "ticks",
            Self::CardDaily => "card_daily",
            Self::Mixed => "mixed",
            Self::Trades => "trades",
        }
    }
}

/// Live bar settings.
#[derive(Debug, Clone, PartialEq)]
pub struct LiveConfig {
    /// Bar construction method.
    pub method: BarMethod,
    /// Volume counter.
    pub volume: VolumeUnit,
    /// How long before the first session open a tick still joins the first bar
    /// (opening auction; XIST 09:40–10:00). Default 30 minutes.
    pub pre_open_ms: i64,
    /// How long after the last session close a tick still joins the last bar (closing
    /// session and trades at the closing price; XIST 18:00–18:10). Default 30 minutes.
    pub post_close_ms: i64,
    /// Sequence number of the first emitted event (the data layer's next number).
    pub first_seq: u64,
    /// `Trades` only: whether special (reported) trades join the bars.
    pub special_trades: SpecialTrades,
}

impl Default for LiveConfig {
    fn default() -> Self {
        Self {
            method: BarMethod::Ticks,
            volume: VolumeUnit::Value,
            pre_open_ms: 30 * MS_PER_MIN,
            post_close_ms: 30 * MS_PER_MIN,
            first_seq: 1,
            special_trades: SpecialTrades::default(),
        }
    }
}

/// Counters for the report (no values).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LiveStats {
    /// Ticks pushed.
    pub ticks: u64,
    /// Ticks outside every trading day window (night, weekend, holiday): no bar.
    pub off_session: u64,
    /// Ticks for a bar already final: not written.
    pub late: u64,
    /// Ticks with trade evidence but no known price: no bar.
    pub no_price: u64,
    /// Non-finite or non-positive price / negative or non-finite counter fields dropped.
    pub invalid_fields: u64,
    /// Cumulative counter went backwards (restart or correction): no volume added.
    pub volume_regressions: u64,
    /// Snapshots whose counter became the day's base (subscription mid-day, no seam).
    pub volume_rebases: u64,
    /// Day extremes written into an intraday bar (`Mixed`).
    pub extremes_reflected: u64,
    /// Bars opened (`bar` events).
    pub bars_opened: u64,
    /// Events emitted.
    pub events: u64,
    /// Last session phase code seen.
    pub last_phase: Option<i32>,
    /// Trades pushed (`Trades` method).
    pub trades: u64,
    /// Special trades left out of the bars ([`SpecialTrades::joins`] false).
    pub special_excluded: u64,
    /// Trades without a quantity (no volume added).
    pub no_quantity: u64,
    /// Price-card ticks ignored because bars come from trades, or trades ignored because
    /// bars come from the card.
    pub wrong_source: u64,
}

/// A live bar builder error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveError(String);

impl fmt::Display for LiveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for LiveError {}

/// The live bar builder of one feed (symbol × interval).
#[derive(Debug, Clone)]
pub struct LiveBars {
    cal: ExchangeCalendar,
    interval: Interval,
    feed: FeedKey,
    cfg: LiveConfig,
    next_seq: u64,
    started: bool,
    /// Last bar (live unless `cur_final`) and its calendar span.
    cur: Option<(Bar, Span)>,
    cur_final: bool,
    last_final_t: Option<TimestampMs>,
    /// Clock: every bar whose absorbing end is at or before this is final.
    closed_through: TimestampMs,
    /// Merged card state.
    known_last: Option<f64>,
    day: Option<i64>,
    cum_last: Option<f64>,
    day_open: Option<f64>,
    day_high: Option<f64>,
    day_low: Option<f64>,
    seam_cum: Option<f64>,
    /// Lots of the last bar (rule 8).
    cur_qty: f64,
    /// (RP4 M27) Special-trade volume / lots waiting for the bar of this open time to open
    /// with a regular trade (special trades never set a price).
    pending_special: Option<(i64, f64, f64)>,
    stats: LiveStats,
}

fn price(x: Option<f64>, bad: &mut u64) -> Option<f64> {
    let x = x?;
    if x.is_finite() && x > 0.0 {
        Some(x)
    } else {
        *bad += 1;
        None
    }
}

fn counter(x: Option<f64>, bad: &mut u64) -> Option<f64> {
    let x = x?;
    if x.is_finite() && x >= 0.0 {
        Some(x)
    } else {
        *bad += 1;
        None
    }
}

impl LiveBars {
    /// A builder for `feed` (its `interval` is parsed) on calendar `cal`.
    ///
    /// # Errors
    /// Unparsable interval; `CardDaily` on an interval other than `1d`.
    pub fn new(feed: FeedKey, cal: ExchangeCalendar, cfg: LiveConfig) -> Result<Self, LiveError> {
        let interval = Interval::parse(&feed.interval).map_err(|e| LiveError(e.to_string()))?;
        if cfg.method == BarMethod::CardDaily && interval != Interval::Day {
            return Err(LiveError(format!(
                "card_daily builds only 1d bars, not `{}`",
                feed.interval
            )));
        }
        Ok(Self {
            cal,
            interval,
            feed,
            next_seq: cfg.first_seq,
            cfg,
            started: false,
            cur: None,
            cur_final: false,
            last_final_t: None,
            closed_through: TimestampMs::MIN,
            known_last: None,
            day: None,
            cum_last: None,
            day_open: None,
            day_high: None,
            day_low: None,
            seam_cum: None,
            cur_qty: 0.0,
            pending_special: None,
            stats: LiveStats::default(),
        })
    }

    /// Starts from a seam with history (before the first [`push`](Self::push)).
    ///
    /// `last_hist_bar`: the last historical bar (REST). If the first live tick falls in its
    /// span, live continues it with `patch` events; otherwise it is final and live opens
    /// the next bar. `cum_volume_at_seam`: the cumulative counter (in the configured unit)
    /// at the end of history, for the trading day of the first live tick (e.g. the sum of
    /// that day's historical intraday bar volumes); live volume is added from it. `None`:
    /// the first snapshot's counter becomes the base.
    ///
    /// # Errors
    /// Called after a push; a bar not aligned to the calendar (`bar_at(interval, t).open
    /// != t`) or breaking the bar rules; a negative or non-finite counter.
    pub fn resume(
        &mut self,
        last_hist_bar: Option<Bar>,
        cum_volume_at_seam: Option<f64>,
    ) -> Result<(), LiveError> {
        if self.started {
            return Err(LiveError("resume after the first tick".to_owned()));
        }
        if let Some(b) = last_hist_bar {
            qp_schema::validate_bars(std::slice::from_ref(&b))
                .map_err(|e| LiveError(format!("seam bar: {e}")))?;
            let span = self
                .cal
                .bar_at(self.interval, b.t)
                .filter(|s| s.open == b.t)
                .ok_or_else(|| {
                    LiveError(format!(
                        "seam bar t={} is not a {} bar open of {}",
                        b.t,
                        self.feed.interval,
                        self.cal.id()
                    ))
                })?;
            self.known_last = Some(b.c);
            self.cur = Some((b, span));
            self.cur_final = false;
        }
        if let Some(x) = cum_volume_at_seam {
            if !(x.is_finite() && x >= 0.0) {
                return Err(LiveError(
                    "seam counter must be finite and non-negative".to_owned(),
                ));
            }
        }
        self.seam_cum = cum_volume_at_seam;
        Ok(())
    }

    /// The feed.
    #[must_use]
    pub fn feed(&self) -> &FeedKey {
        &self.feed
    }

    /// The interval.
    #[must_use]
    pub fn interval(&self) -> Interval {
        self.interval
    }

    /// The settings.
    #[must_use]
    pub fn config(&self) -> &LiveConfig {
        &self.cfg
    }

    /// The sequence number the next event will carry.
    #[must_use]
    pub fn next_seq(&self) -> u64 {
        self.next_seq
    }

    /// Counters.
    #[must_use]
    pub fn stats(&self) -> &LiveStats {
        &self.stats
    }

    /// The last bar and whether it is final.
    #[must_use]
    pub fn current(&self) -> Option<(&Bar, bool)> {
        self.cur.as_ref().map(|(b, _)| (b, self.cur_final))
    }

    /// Lots traded in the last bar (`Trades` method; 0 otherwise or for a seam bar).
    #[must_use]
    pub fn current_quantity(&self) -> Option<f64> {
        self.cur.as_ref().map(|_| self.cur_qty)
    }

    /// Open time of the latest final bar.
    #[must_use]
    pub fn last_final_t(&self) -> Option<TimestampMs> {
        self.last_final_t
    }

    /// The bar span a tick at `t_ms` belongs to (rule 1), or `None` outside every trading
    /// day window.
    #[must_use]
    pub fn bar_of(&self, t_ms: TimestampMs) -> Option<Span> {
        let day = self.cal.local_day(t_ms);
        let sessions = self.cal.sessions_on(day);
        let first = *sessions.first()?;
        let last = *sessions.last()?;
        if t_ms < first.open {
            return (t_ms >= first.open - self.cfg.pre_open_ms)
                .then(|| self.cal.bar_at(self.interval, first.open))
                .flatten();
        }
        let s = sessions.iter().rev().find(|s| s.open <= t_ms)?;
        if t_ms < s.close {
            return self.cal.bar_at(self.interval, t_ms);
        }
        if s.close == last.close && t_ms >= last.close + self.cfg.post_close_ms {
            return None;
        }
        self.cal.bar_at(self.interval, s.close - 1)
    }

    /// The instant from which no tick can join bar `span` any more (its close, plus the
    /// closing window if it ends the day, or the next session open if it ends a session
    /// before a break).
    #[must_use]
    pub fn final_at(&self, span: Span) -> TimestampMs {
        let day = self.cal.local_day(span.close - 1);
        let sessions = self.cal.sessions_on(day);
        match sessions.iter().position(|s| s.close == span.close) {
            Some(i) if i + 1 == sessions.len() => span.close + self.cfg.post_close_ms,
            Some(i) => sessions[i + 1].open,
            None => span.close,
        }
    }

    /// Clock input: exchange time reached `t_ms`. If the live bar's absorbing end
    /// ([`final_at`](Self::final_at)) is at or before `t_ms`, it becomes final and is
    /// returned (once). Opens no bar and emits no event; later ticks for it are late.
    pub fn close_until(&mut self, t_ms: TimestampMs) -> Option<Bar> {
        self.closed_through = self.closed_through.max(t_ms);
        let (bar, span) = self.cur.as_ref()?;
        if self.cur_final || self.final_at(*span) > self.closed_through {
            return None;
        }
        self.cur_final = true;
        self.last_final_t = Some(bar.t);
        Some(bar.clone())
    }

    fn emit(&mut self, change: DataChange, out: &mut Vec<DataEvent>) {
        out.push(DataEvent {
            feed: self.feed.clone(),
            seq: self.next_seq,
            change,
        });
        self.next_seq += 1;
        self.stats.events += 1;
    }
}
