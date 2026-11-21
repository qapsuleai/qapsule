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

    /// Applies one price-card update; returns the data events it causes (possibly none).
    pub fn push(&mut self, tick: &CardTick) -> Vec<DataEvent> {
        let mut out = Vec::new();
        if self.cfg.method == BarMethod::Trades {
            self.stats.wrong_source += 1;
            if tick.phase.is_some() {
                self.stats.last_phase = tick.phase;
            }
            return out;
        }
        self.started = true;
        self.stats.ticks += 1;
        if tick.phase.is_some() {
            self.stats.last_phase = tick.phase;
        }
        let Some(span) = self.bar_of(tick.t_ms) else {
            self.stats.off_session += 1;
            return out;
        };
        let cur_span = self.cur.as_ref().map(|(_, s)| *s);
        let late = self.final_at(span) <= self.closed_through
            || cur_span.is_some_and(|c| span.open < c.open || (span == c && self.cur_final));
        if late {
            self.stats.late += 1;
            return out;
        }

        // Sanitized fields.
        let bad = &mut self.stats.invalid_fields;
        let last = price(tick.last, bad);
        let day_open = price(tick.day_open, bad);
        let day_high = price(tick.day_high, bad);
        let day_low = price(tick.day_low, bad);
        let cum = counter(
            match self.cfg.volume {
                VolumeUnit::Value => tick.cum_volume,
                VolumeUnit::Quantity => tick.cum_quantity,
            },
            bad,
        );

        // Day state: the counter and the day fields restart each trading day.
        let day = self.cal.local_day(tick.t_ms);
        if self.day != Some(day) {
            self.day = Some(day);
            self.cum_last = self.seam_cum.take();
            self.day_open = None;
            self.day_high = None;
            self.day_low = None;
        }
        self.seam_cum = None;

        let mut inc = 0.0;
        if let Some(x) = cum {
            match self.cum_last {
                None if tick.initial => self.stats.volume_rebases += 1,
                None => inc = x,
                Some(p) if x >= p => inc = x - p,
                Some(_) => self.stats.volume_regressions += 1,
            }
            self.cum_last = Some(x);
        }
        if last.is_some() {
            self.known_last = last;
        }
        if day_open.is_some() {
            self.day_open = day_open;
        }
        // A rise of the day high / fall of the day low seen in an update (not a snapshot:
        // after a reconnect the extreme may belong to an earlier bar).
        let high_rise = day_high.filter(|&h| !tick.initial && self.day_high.is_some_and(|p| h > p));
        let low_fall = day_low.filter(|&l| !tick.initial && self.day_low.is_some_and(|p| l < p));
        if day_high.is_some() {
            self.day_high = day_high;
        }
        if day_low.is_some() {
            self.day_low = day_low;
        }

        let card = self.interval == Interval::Day
            && matches!(self.cfg.method, BarMethod::CardDaily | BarMethod::Mixed);
        let reflect = !card && self.cfg.method == BarMethod::Mixed;
        let same = cur_span == Some(span);
        let traded = (!tick.initial && last.is_some()) || inc > 0.0;

        let next = if same {
            let Some((old, _)) = self.cur.as_ref() else {
                return out;
            };
            let mut b = old.clone();
            if card {
                self.card_update(&mut b, last);
            } else {
                if let Some(p) = last {
                    b.c = p;
                    b.h = b.h.max(p);
                    b.l = b.l.min(p);
                }
                b.v += inc;
            }
            Some(b)
        } else if traded || (card && self.cum_last.is_some_and(|c| c > 0.0)) {
            let Some(p) = last.or(self.known_last) else {
                self.stats.no_price += 1;
                return out;
            };
            let mut b = Bar {
                t: span.open,
                o: p,
                h: p,
                l: p,
                c: p,
                v: inc,
            };
            if card {
                b.o = self.day_open.unwrap_or(p);
                b.h = b.o.max(p);
                b.l = b.o.min(p);
                b.v = 0.0;
                self.card_update(&mut b, last);
            }
            Some(b)
        } else {
            None
        };
        let Some(mut b) = next else {
            return out;
        };
        if reflect {
            if let Some(h) = high_rise.filter(|&h| h > b.h) {
                b.h = h;
                self.stats.extremes_reflected += 1;
            }
            if let Some(l) = low_fall.filter(|&l| l < b.l) {
                b.l = l;
                self.stats.extremes_reflected += 1;
            }
        }

        if same {
            let Some((old, _)) = self.cur.as_mut() else {
                return out;
            };
            let fields = bar_patch_diff(old, &b);
            *old = b;
            if !fields.is_empty() {
                let t = span.open;
                self.emit(DataChange::Patch { t, fields }, &mut out);
            }
        } else {
            if let Some((prev, _)) = &self.cur {
                self.last_final_t = Some(prev.t);
            }
            self.cur = Some((b.clone(), span));
            self.cur_final = false;
            self.stats.bars_opened += 1;
            self.emit(DataChange::Bar { bar: b }, &mut out);
        }
        out
    }

    /// Applies one trade (rule 8); returns the data events it causes (possibly none).
    pub fn push_trade(&mut self, tr: &TradeTick) -> Vec<DataEvent> {
        let mut out = Vec::new();
        if self.cfg.method != BarMethod::Trades {
            self.stats.wrong_source += 1;
            return out;
        }
        self.started = true;
        self.stats.trades += 1;
        if tr.special && !self.cfg.special_trades.joins(self.interval) {
            self.stats.special_excluded += 1;
            return out;
        }
        let Some(span) = self.bar_of(tr.t_ms) else {
            self.stats.off_session += 1;
            return out;
        };
        let cur_span = self.cur.as_ref().map(|(_, s)| *s);
        let late = self.final_at(span) <= self.closed_through
            || cur_span.is_some_and(|c| span.open < c.open || (span == c && self.cur_final));
        if late {
            self.stats.late += 1;
            return out;
        }
        let bad = &mut self.stats.invalid_fields;
        let Some(p) = price(Some(tr.price), bad) else {
            return out;
        };
        let qty = match tr.quantity {
            None => {
                self.stats.no_quantity += 1;
                0.0
            }
            Some(q) => counter(Some(q), bad).unwrap_or(0.0),
        };
        let inc = match self.cfg.volume {
            VolumeUnit::Value => p * qty,
            VolumeUnit::Quantity => qty,
        };
        if tr.special && self.cfg.special_trades == SpecialTrades::DayOnly {
            // (RP4 M27) A special (reported) trade adds to the bar's volume only: REST's
            // daily bar holds its quantity but not its price (one daily low 3.6 % off when
            // its price was taken, 0 otherwise; `M27/RESULTS.md`).
            if cur_span == Some(span) {
                let Some((old, _)) = self.cur.as_mut() else {
                    return out;
                };
                let mut b = old.clone();
                b.v += inc;
                let fields = bar_patch_diff(old, &b);
                *old = b;
                self.cur_qty += qty;
                if !fields.is_empty() {
                    let t = span.open;
                    self.emit(DataChange::Patch { t, fields }, &mut out);
                }
            } else {
                let (v0, q0) = match self.pending_special {
                    Some((t, v, q)) if t == span.open => (v, q),
                    _ => (0.0, 0.0),
                };
                self.pending_special = Some((span.open, v0 + inc, q0 + qty));
            }
            return out;
        }
        self.known_last = Some(p);
        if cur_span == Some(span) {
            let Some((old, _)) = self.cur.as_mut() else {
                return out;
            };
            let mut b = old.clone();
            b.c = p;
            b.h = b.h.max(p);
            b.l = b.l.min(p);
            b.v += inc;
            let fields = bar_patch_diff(old, &b);
            *old = b;
            self.cur_qty += qty;
            if !fields.is_empty() {
                let t = span.open;
                self.emit(DataChange::Patch { t, fields }, &mut out);
            }
        } else {
            if let Some((prev, _)) = &self.cur {
                self.last_final_t = Some(prev.t);
            }
            let (pv, pq) = match self.pending_special.take() {
                Some((t, v, q)) if t == span.open => (v, q),
                _ => (0.0, 0.0),
            };
            let b = Bar {
                t: span.open,
                o: p,
                h: p,
                l: p,
                c: p,
                v: pv + inc,
            };
            self.cur = Some((b.clone(), span));
            self.cur_final = false;
            self.cur_qty = pq + qty;
            self.stats.bars_opened += 1;
            self.emit(DataChange::Bar { bar: b }, &mut out);
        }
        out
    }

    /// `CardDaily` state of day bar `b` from the merged card (monotone `h`, `l`, `v`).
    fn card_update(&self, b: &mut Bar, last: Option<f64>) {
        if let Some(o) = self.day_open {
            b.o = o;
        }
        if let Some(p) = last {
            b.c = p;
        }
        let (mut h, mut l) = (b.h.max(b.o).max(b.c), b.l.min(b.o).min(b.c));
        if let Some(x) = self.day_high {
            h = h.max(x);
        }
        if let Some(x) = self.day_low {
            l = l.min(x);
        }
        b.h = h;
        b.l = l;
        if let Some(c) = self.cum_last {
            b.v = b.v.max(c);
        }
    }
}

/// Tolerances for [`compare_bars_with`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DiffTolerance {
    /// Absolute price tolerance (e.g. half the price step).
    pub price_abs: f64,
    /// Relative volume tolerance.
    pub volume_rel: f64,
}

impl Default for DiffTolerance {
    fn default() -> Self {
        Self {
            price_abs: 1e-6,
            volume_rel: 1e-6,
        }
    }
}

/// Agreement of one bar field over the matched bars.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FieldDiff {
    /// Bit-equal values.
    pub exact: u64,
    /// Values within tolerance (includes `exact`).
    pub within_tol: u64,
    /// Largest absolute difference.
    pub max_abs: f64,
    /// Largest relative difference (`|live − rest| / |rest|`; infinite if `rest` is 0
    /// and `live` is not).
    pub max_rel: f64,
}

impl FieldDiff {
    fn add(&mut self, live: f64, rest: f64, tol_ok: bool) {
        if live.to_bits() == rest.to_bits() {
            self.exact += 1;
        }
        if tol_ok {
            self.within_tol += 1;
        }
        let abs = (live - rest).abs();
        let rel = if rest == 0.0 {
            if abs == 0.0 {
                0.0
            } else {
                f64::INFINITY
            }
        } else {
            abs / rest.abs()
        };
        self.max_abs = self.max_abs.max(abs);
        self.max_rel = self.max_rel.max(rel);
    }
}

/// Live versus REST bars (M22 table): counts and extreme differences only.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BarDiff {
    /// Bars with the same `t` on both sides.
    pub matched: u64,
    /// Bars only in the live series.
    pub live_only: u64,
    /// Bars only in the REST series.
    pub rest_only: u64,
    /// Matched bars equal in all five fields (bit-equal).
    pub all_exact: u64,
    /// Matched bars equal in all five fields within tolerance.
    pub all_within_tol: u64,
    /// Open.
    pub o: FieldDiff,
    /// High.
    pub h: FieldDiff,
    /// Low.
    pub l: FieldDiff,
    /// Close.
    pub c: FieldDiff,
    /// Volume.
    pub v: FieldDiff,
}

impl BarDiff {
    /// `n / matched` (0 when nothing matched).
    #[must_use]
    pub fn ratio(&self, n: u64) -> f64 {
        if self.matched == 0 {
            0.0
        } else {
            n as f64 / self.matched as f64
        }
    }
}

/// [`compare_bars_with`] at the default tolerance.
#[must_use]
pub fn compare_bars(live: &[Bar], rest: &[Bar]) -> BarDiff {
    compare_bars_with(live, rest, &DiffTolerance::default())
}

/// Compares live bars with REST bars by `t` (any order; a repeated `t` keeps the last).
#[must_use]
pub fn compare_bars_with(live: &[Bar], rest: &[Bar], tol: &DiffTolerance) -> BarDiff {
    let live: BTreeMap<TimestampMs, &Bar> = live.iter().map(|b| (b.t, b)).collect();
    let rest: BTreeMap<TimestampMs, &Bar> = rest.iter().map(|b| (b.t, b)).collect();
    let mut d = BarDiff::default();
    for (t, a) in &live {
        let Some(b) = rest.get(t) else {
            d.live_only += 1;
            continue;
        };
        d.matched += 1;
        let p = |x: f64, y: f64| (x - y).abs() <= tol.price_abs;
        let ok = [
            p(a.o, b.o),
            p(a.h, b.h),
            p(a.l, b.l),
            p(a.c, b.c),
            (a.v - b.v).abs() <= tol.volume_rel * b.v.abs(),
        ];
        d.o.add(a.o, b.o, ok[0]);
        d.h.add(a.h, b.h, ok[1]);
        d.l.add(a.l, b.l, ok[2]);
        d.c.add(a.c, b.c, ok[3]);
        d.v.add(a.v, b.v, ok[4]);
        let bits = |x: f64, y: f64| x.to_bits() == y.to_bits();
        if bits(a.o, b.o) && bits(a.h, b.h) && bits(a.l, b.l) && bits(a.c, b.c) && bits(a.v, b.v) {
            d.all_exact += 1;
        }
        if ok.iter().all(|&x| x) {
            d.all_within_tol += 1;
        }
    }
    d.rest_only = rest.keys().filter(|t| !live.contains_key(t)).count() as u64;
    d
}
