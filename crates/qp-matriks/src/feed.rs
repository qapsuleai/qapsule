//! The live feed: raw Matriks records → §18.4 data events (R6 scope 3–6).
//!
//! [`LiveFeed`] is the **one** pipeline shared by the live connector and raw-log replay
//! (acceptance criterion 7: same decoder). It takes [`RawRecord`]s (a received message or
//! a connection event) and returns a [`FeedOut`] per record: data events
//! (`docs/PROTOCOL.md` §18.4), the exchange clock, connection changes, session phases
//! ("seans evresi") and the bars that became final. Pure: no network, no wall clock, no
//! randomness; the same records give the same outputs, byte for byte.
//!
//! # Rules
//!
//! 1. **Only price cards build bars.** `mx/symbol/<SYM>[@lvl2]` (`SymbolMessage`, also via
//!    a `mx/user/<sub>/mx/...` mirror) goes through a [`SymbolBook`] (field presence kept,
//!    initial vs update) and then into one [`LiveBars`] per configured interval. A symbol
//!    that was not [`seed`](LiveFeed::seed)ed gets its builders on its first message (no
//!    history). A message that carries none of the bar fields (`last`, `open`,
//!    `dailyHigh`, `dailyLow`, the volume counter, the quantity counter) and is not an
//!    initial — e.g. an update holding only `askSize` — is not pushed: it builds nothing.
//! 2. **Bar fields.** `last` ← `last`; day open ← `open`; day high / low ←
//!    `dailyHigh` / `dailyLow`; cumulative value ← `dailyVolume`
//!    ([`VolumeField::DailyVolume`], default) or `volume` ([`VolumeField::Volume`]);
//!    cumulative quantity ← `dailyQuantity` or `quantity` (same choice). Which Matriks
//!    field equals REST `volume` is an open question (M22); `qp live compare` measures it.
//! 3. **Time ([`TimeSource`]).** `UpdateDate` (default): a tick's time is the message's
//!    `updateDate` (`YYYY-MM-DDTHH:MM:SS[.fff][Z|±HH[:MM]]`, no offset = Istanbul UTC+3;
//!    the delayed endpoint sends `+03`). A message without `updateDate` (or with an
//!    unreadable one) takes the **last known exchange time plus the arrival difference**:
//!    `t = ud₀ + (arrival − arrival₀)`, where `(ud₀, arrival₀)` is the latest *update*
//!    message (not an initial: an initial's `updateDate` is the symbol's last trade, which
//!    may be old) that carried a readable `updateDate`. The anchor is dropped on a lost
//!    connection. With no anchor yet the message builds nothing (counted as
//!    `time_unknown`). `Arrival`: the record's arrival time (`t_us`) for everything.
//! 4. **Clock.** `clock_ms` is monotone (UTC ms). `UpdateDate`: advanced by every readable
//!    `updateDate` that is not more than [`MAX_LEAD_MS`] ahead of its arrival (a guard
//!    against a bad value finalizing every bar) and by `mx/timestamp` ticks (real-time
//!    endpoint; the delayed one rejects that topic). Derived times do not move it.
//!    `Arrival`: every record moves it to its arrival time. When the clock moves, every
//!    builder is closed through `clock − grace` ([`DEFAULT_FINALIZE_GRACE_MS`]: ticks of
//!    different symbols arrive slightly out of exchange-time order) and the bars that
//!    became final are reported. [`LiveFeed::advance_clock`] moves it from outside and
//!    closes through `now` exactly (no grace).
//! 5. **Final bars.** A bar is final when the next bar of its feed opens or when the
//!    clock passes its end ([`LiveBars::close_until`]); each is reported once in
//!    [`FeedOut::finalized`]. Ticks for a final bar are late (counted, not written).
//! 6. **Connection.** `ConnectionOpened` → [`ConnChange::Opened`] (session counter, 1 =
//!    first). `ConnectionLost` → [`ConnChange::Lost`]; every symbol then waits for a new
//!    initial (the first message after the reconnect). `Subscribed` counts as a
//!    re-subscription only after a `ConnectionLost` and only for a symbol that has had no
//!    message since: the raw log may write a symbol's first message *before* its
//!    `Subscribed` record, and the first subscription needs nothing (the book already
//!    treats the first message as the initial). The book merges initials
//!    ([`InitialPolicy::Merge`]): whether a reconnect resends a full picture is not
//!    known yet, and merging is right either way.
//! 7. **Sequence.** Each feed (symbol × interval) numbers its events 1, 2, 3… without
//!    gaps; a [`seed`](LiveFeed::seed)ed feed starts at the given number.
//! 8. **Phase.** `stockStatus` is reported per symbol when it first arrives or changes.

use std::collections::{BTreeMap, BTreeSet};

use prost::Message as _;
use qp_calendar::{ExchangeCalendar, Interval};
use qp_schema::data::{DataChange, DataEvent, FeedKey};
use qp_schema::Bar;
use qp_source::{BarMethod, CardTick, LiveBars, LiveConfig, VolumeUnit};

use crate::clock::decode_time;
use crate::messages::SymbolMessage;
use crate::rawlog::{RawRecord, RecordKind};
use crate::symbol::{InitialPolicy, SymbolBook, UpdateKind};
use crate::topic::{parse_topic, Topic};

/// Default lag between the clock and the instant bars are closed through (rule 4).
pub const DEFAULT_FINALIZE_GRACE_MS: i64 = 2_000;
/// An `updateDate` more than this ahead of its arrival does not move the clock (rule 4).
pub const MAX_LEAD_MS: i64 = 60_000;

/// Where a tick's time comes from (rule 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimeSource {
    /// The message's `updateDate` (exchange time); without it, the last known exchange
    /// time plus the arrival difference.
    #[default]
    UpdateDate,
    /// The record's arrival time on the host.
    Arrival,
}

impl TimeSource {
    /// Short name (`update_date`, `arrival`).
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::UpdateDate => "update_date",
            Self::Arrival => "arrival",
        }
    }
}

/// Which cumulative card fields are the volume counters (rule 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VolumeField {
    /// `dailyVolume` (value) and `dailyQuantity` (lots).
    #[default]
    DailyVolume,
    /// `volume` (value) and `quantity` (lots).
    Volume,
}

impl VolumeField {
    /// Short name (`daily`, `volume`).
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::DailyVolume => "daily",
            Self::Volume => "volume",
        }
    }
}

/// Live feed settings.
#[derive(Debug, Clone, PartialEq)]
pub struct LiveFeedConfig {
    /// Bar intervals built for every symbol (`1m`, `5m`, `1h`, `1d`, …). Unparsable
    /// intervals are dropped (and counted in [`FeedStats::bad_intervals`]).
    pub intervals: Vec<String>,
    /// Bar method (M22). `CardDaily` applies to `1d`; other intervals use `Ticks`.
    pub method: BarMethod,
    /// Volume unit (value in TL or quantity in lots).
    pub volume: VolumeUnit,
    /// Which card counters are read.
    pub volume_field: VolumeField,
    /// Tick time source.
    pub time: TimeSource,
    /// Feed name in every [`FeedKey`].
    pub feed_name: Option<String>,
}

impl Default for LiveFeedConfig {
    fn default() -> Self {
        Self {
            intervals: vec!["1m".into(), "5m".into(), "1h".into(), "1d".into()],
            method: BarMethod::Ticks,
            volume: VolumeUnit::Value,
            volume_field: VolumeField::DailyVolume,
            time: TimeSource::UpdateDate,
            feed_name: None,
        }
    }
}

/// A connection change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnChange {
    /// A connection opened; `session` counts them from 1.
    Opened {
        /// 1 for the first connection, 2 after the first reconnect, …
        session: u32,
    },
    /// The connection was lost or closed: data is stale until the next one.
    Lost,
}

/// What one record caused.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FeedOut {
    /// §18.4 data events, per feed with a gapless local `seq`.
    pub events: Vec<DataEvent>,
    /// The new exchange clock (UTC ms) if it moved.
    pub clock_ms: Option<i64>,
    /// A connection change.
    pub connection: Option<ConnChange>,
    /// `(symbol, stockStatus)` for symbols whose phase arrived or changed.
    pub phases: Vec<(String, i32)>,
    /// Bars that became final: feed and bar open time.
    pub finalized: Vec<(FeedKey, i64)>,
}

impl FeedOut {
    /// Whether nothing happened.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
            && self.clock_ms.is_none()
            && self.connection.is_none()
            && self.phases.is_empty()
            && self.finalized.is_empty()
    }
}

/// Counters (no market values).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FeedStats {
    /// Records pushed.
    pub records: u64,
    /// Price-card messages decoded.
    pub symbol_messages: u64,
    /// Of those, initials.
    pub initials: u64,
    /// Payloads that failed to decode (state unchanged).
    pub decode_errors: u64,
    /// `mx/timestamp` messages.
    pub time_messages: u64,
    /// Other topics (messenger, …): ignored.
    pub other_messages: u64,
    /// Card messages without any bar field (not pushed to the builders).
    pub no_bar_fields: u64,
    /// Ticks timed by their own `updateDate` (or arrival in `Arrival` mode).
    pub time_direct: u64,
    /// Ticks timed by the anchor rule (no / unreadable `updateDate`).
    pub time_derived: u64,
    /// Ticks dropped: no time known yet.
    pub time_unknown: u64,
    /// Unreadable `updateDate` values.
    pub bad_update_date: u64,
    /// `updateDate` values too far ahead of arrival to move the clock.
    pub clock_lead_rejected: u64,
    /// Ticks pushed to builders (per symbol message, not per interval).
    pub ticks: u64,
    /// Connections opened.
    pub connections_opened: u64,
    /// Connections lost.
    pub connections_lost: u64,
    /// `Subscribed` records taken as re-subscriptions.
    pub resubscribes: u64,
    /// `Subscribed` records that needed nothing (first subscription or already fresh).
    pub subscribed_ignored: u64,
    /// `SubscribeRejected` records.
    pub subscribe_rejected: u64,
    /// Note records.
    pub notes: u64,
    /// Configured intervals that could not be parsed.
    pub bad_intervals: u64,
    /// Feeds (symbol × interval).
    pub feeds: u64,
    /// Bars reported final.
    pub finalized: u64,
    /// Sum over feeds: ticks outside every trading window.
    pub off_session: u64,
    /// Sum over feeds: ticks for final bars.
    pub late: u64,
    /// Sum over feeds: trade evidence without a known price.
    pub no_price: u64,
    /// Sum over feeds: invalid field values dropped.
    pub invalid_fields: u64,
    /// Sum over feeds: counter went backwards.
    pub volume_regressions: u64,
    /// Sum over feeds: snapshot counter taken as the day's base.
    pub volume_rebases: u64,
    /// Sum over feeds: day extremes reflected into intraday bars (`Mixed`).
    pub extremes_reflected: u64,
    /// Sum over feeds: bars opened.
    pub bars_opened: u64,
    /// Sum over feeds: data events.
    pub events: u64,
}

/// Parses a Matriks `updateDate` (`YYYY-MM-DDTHH:MM:SS[.fff][Z|±HH[:MM]]`) to UTC ms. A
/// value without offset is Istanbul time (UTC+3).
#[must_use]
pub fn parse_update_date(s: &str) -> Option<i64> {
    let b = s.as_bytes();
    if b.len() < 19 || b[4] != b'-' || b[7] != b'-' || (b[10] != b'T' && b[10] != b' ') {
        return None;
    }
    if b[13] != b':' || b[16] != b':' {
        return None;
    }
    let num = |a: usize, n: usize| -> Option<i64> {
        let part = s.get(a..a + n)?;
        if !part.bytes().all(|c| c.is_ascii_digit()) {
            return None;
        }
        part.parse().ok()
    };
    let (y, mo, d, h, mi, se) = (
        num(0, 4)?,
        num(5, 2)?,
        num(8, 2)?,
        num(11, 2)?,
        num(14, 2)?,
        num(17, 2)?,
    );
    if !(1..=12).contains(&mo) || !(1..=31).contains(&d) || h > 23 || mi > 59 || se > 60 {
        return None;
    }
    let mut i = 19;
    let mut frac_ms = 0i64;
    if b.get(i) == Some(&b'.') {
        let start = i + 1;
        i = start;
        while b.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
        if i == start {
            return None;
        }
        let mut v: String = s[start..i].chars().take(3).collect();
        while v.len() < 3 {
            v.push('0');
        }
        frac_ms = v.parse().ok()?;
    }
    let rest = &s[i..];
    let off_min = if rest.is_empty() {
        180
    } else if rest == "Z" {
        0
    } else {
        let sign = match rest.as_bytes()[0] {
            b'+' => 1,
            b'-' => -1,
            _ => return None,
        };
        let r = rest[1..].replace(':', "");
        if r.len() != 2 && r.len() != 4 {
            return None;
        }
        let oh: i64 = r.get(0..2)?.parse().ok()?;
        let om: i64 = r.get(2..4).unwrap_or("00").parse().ok()?;
        sign * (oh * 60 + om)
    };
    let day = qp_calendar::days_from_civil(y, u32::try_from(mo).ok()?, u32::try_from(d).ok()?);
    Some(((day * 86_400 + h * 3600 + mi * 60 + se) - off_min * 60) * 1000 + frac_ms)
}

/// The live feed (module docs).
#[derive(Debug, Clone)]
pub struct LiveFeed {
    cfg: LiveFeedConfig,
    intervals: Vec<String>,
    cal: ExchangeCalendar,
    book: SymbolBook,
    feeds: BTreeMap<FeedKey, LiveBars>,
    clock_ms: Option<i64>,
    anchor: Option<(i64, i64)>,
    phases: BTreeMap<String, i32>,
    sessions: u32,
    seen_lost: bool,
    fresh_since_lost: BTreeSet<String>,
    grace_ms: i64,
    stats: FeedStats,
}

impl LiveFeed {
    /// A feed on calendar `cal` (rules in the module docs).
    #[must_use]
    pub fn new(cfg: LiveFeedConfig, cal: ExchangeCalendar) -> Self {
        let mut stats = FeedStats::default();
        let mut intervals = Vec::new();
        for iv in &cfg.intervals {
            if Interval::parse(iv).is_ok() {
                if !intervals.contains(iv) {
                    intervals.push(iv.clone());
                }
            } else {
                stats.bad_intervals += 1;
            }
        }
        Self {
            cfg,
            intervals,
            cal,
            book: SymbolBook::with_policy(InitialPolicy::Merge),
            feeds: BTreeMap::new(),
            clock_ms: None,
            anchor: None,
            phases: BTreeMap::new(),
            sessions: 0,
            seen_lost: false,
            fresh_since_lost: BTreeSet::new(),
            grace_ms: DEFAULT_FINALIZE_GRACE_MS,
            stats,
        }
    }

    /// The settings.
    #[must_use]
    pub fn config(&self) -> &LiveFeedConfig {
        &self.cfg
    }

    /// The intervals actually built (configured, parsable, de-duplicated).
    #[must_use]
    pub fn intervals(&self) -> &[String] {
        &self.intervals
    }

    /// Sets the finalize grace of rule 4 (ms, ≥ 0).
    pub fn set_finalize_grace(&mut self, ms: i64) {
        self.grace_ms = ms.max(0);
    }

    /// The exchange clock (UTC ms), if known.
    #[must_use]
    pub fn clock_ms(&self) -> Option<i64> {
        self.clock_ms
    }

    /// The price-card book.
    #[must_use]
    pub fn book(&self) -> &SymbolBook {
        &self.book
    }

    /// The bar builders, by feed.
    pub fn feeds(&self) -> impl Iterator<Item = (&FeedKey, &LiveBars)> {
        self.feeds.iter()
    }

    /// The feed key of `symbol` × `interval`.
    #[must_use]
    pub fn key(&self, symbol: &str, interval: &str) -> FeedKey {
        FeedKey {
            symbol: symbol.to_owned(),
            interval: interval.to_owned(),
            feed: self.cfg.feed_name.clone(),
        }
    }

    fn builder(&self, key: &FeedKey, first_seq: u64) -> Result<LiveBars, String> {
        let interval = Interval::parse(&key.interval).map_err(|e| e.to_string())?;
        let method = match self.cfg.method {
            BarMethod::CardDaily if interval != Interval::Day => BarMethod::Ticks,
            m => m,
        };
        LiveBars::new(
            key.clone(),
            self.cal.clone(),
            LiveConfig {
                method,
                volume: self.cfg.volume,
                first_seq,
                ..LiveConfig::default()
            },
        )
        .map_err(|e| e.to_string())
    }

    /// The seam (T4): starts `symbol` × `interval` from the last historical (REST) bar,
    /// before any live message for it. Live continues that bar with `patch` events if
    /// the first tick falls in it ([`LiveBars::resume`]); `next_seq` is the `seq` of the
    /// first live event (the number after the one the data layer's `ready` reported, so
    /// the live events follow the REST bars without a gap). The first snapshot's counter
    /// becomes the day's volume base; see [`seed_with_counter`](Self::seed_with_counter).
    ///
    /// # Errors
    /// Unparsable interval; the feed already received a live tick; a bar not aligned to
    /// the calendar.
    pub fn seed(
        &mut self,
        symbol: &str,
        interval: &str,
        last_hist_bar: Option<Bar>,
        next_seq: u64,
    ) -> Result<(), String> {
        self.seed_with_counter(symbol, interval, last_hist_bar, next_seq, None)
    }

    /// [`seed`](Self::seed) with the cumulative counter at the end of history (the day's
    /// traded value or quantity so far, in the configured unit).
    ///
    /// # Errors
    /// As [`seed`](Self::seed); a negative or non-finite counter.
    pub fn seed_with_counter(
        &mut self,
        symbol: &str,
        interval: &str,
        last_hist_bar: Option<Bar>,
        next_seq: u64,
        cum_at_seam: Option<f64>,
    ) -> Result<(), String> {
        let key = self.key(symbol, interval);
        if let Some(lb) = self.feeds.get(&key) {
            if lb.stats().ticks > 0 {
                return Err(format!(
                    "seed {symbol} {interval}: live ticks already applied"
                ));
            }
        }
        let mut lb = self.builder(&key, next_seq.max(1))?;
        lb.resume(last_hist_bar, cum_at_seam)
            .map_err(|e| format!("seed {symbol} {interval}: {e}"))?;
        if !self.feeds.contains_key(&key) {
            self.stats.feeds += 1;
        }
        self.feeds.insert(key, lb);
        Ok(())
    }

    /// Applies one record (message or connection event).
    pub fn push(&mut self, rec: &RawRecord) -> FeedOut {
        let mut out = FeedOut::default();
        self.stats.records += 1;
        let arrival = rec.t_us.div_euclid(1000);
        match rec.kind {
            RecordKind::Message => self.on_message(rec, arrival, &mut out),
            RecordKind::ConnectionOpened => {
                self.sessions += 1;
                self.stats.connections_opened += 1;
                out.connection = Some(ConnChange::Opened {
                    session: self.sessions,
                });
            }
            RecordKind::ConnectionLost => {
                self.stats.connections_lost += 1;
                self.book.resubscribed_all();
                self.seen_lost = true;
                self.fresh_since_lost.clear();
                self.anchor = None;
                out.connection = Some(ConnChange::Lost);
            }
            RecordKind::Subscribed => {
                let sym = match parse_topic(&rec.topic).unwrap_mirror() {
                    Topic::Symbol { symbol, .. } => Some(symbol.clone()),
                    _ => None,
                };
                match sym {
                    Some(s) if self.seen_lost && !self.fresh_since_lost.contains(&s) => {
                        self.book.resubscribed(&s);
                        self.stats.resubscribes += 1;
                    }
                    _ => self.stats.subscribed_ignored += 1,
                }
            }
            RecordKind::SubscribeRejected => self.stats.subscribe_rejected += 1,
            RecordKind::Note => self.stats.notes += 1,
        }
        if self.cfg.time == TimeSource::Arrival {
            self.move_clock(arrival, self.grace_ms, &mut out);
        }
        out
    }

    /// Moves the clock from outside to `now_ms` (UTC ms) and closes every bar through
    /// `now_ms` exactly. A time not after the clock changes nothing.
    pub fn advance_clock(&mut self, now_ms: i64) -> FeedOut {
        let mut out = FeedOut::default();
        self.move_clock(now_ms, 0, &mut out);
        out
    }

    /// Counters (feed counters plus the sum of the builders' counters).
    #[must_use]
    pub fn stats(&self) -> FeedStats {
        let mut s = self.stats.clone();
        for lb in self.feeds.values() {
            let l = lb.stats();
            s.off_session += l.off_session;
            s.late += l.late;
            s.no_price += l.no_price;
            s.invalid_fields += l.invalid_fields;
            s.volume_regressions += l.volume_regressions;
            s.volume_rebases += l.volume_rebases;
            s.extremes_reflected += l.extremes_reflected;
            s.bars_opened += l.bars_opened;
            s.events += l.events;
        }
        s
    }

    fn move_clock(&mut self, t: i64, grace: i64, out: &mut FeedOut) {
        if self.clock_ms.is_some_and(|c| c >= t) {
            return;
        }
        self.clock_ms = Some(t);
        out.clock_ms = Some(t);
        let until = t - grace;
        for (k, lb) in &mut self.feeds {
            if let Some(b) = lb.close_until(until) {
                out.finalized.push((k.clone(), b.t));
                self.stats.finalized += 1;
            }
        }
    }

    fn on_message(&mut self, rec: &RawRecord, arrival: i64, out: &mut FeedOut) {
        let topic = parse_topic(&rec.topic);
        match topic.unwrap_mirror() {
            Topic::Symbol { symbol, .. } => {
                let symbol = symbol.clone();
                self.on_symbol(&symbol, &rec.payload, arrival, out);
            }
            Topic::Timestamp => {
                self.stats.time_messages += 1;
                match decode_time(&rec.payload) {
                    Ok(tick) if self.cfg.time == TimeSource::UpdateDate && tick.micros > 0 => {
                        let ms = tick.millis();
                        if ms <= arrival + MAX_LEAD_MS {
                            self.move_clock(ms, self.grace_ms, out);
                        } else {
                            self.stats.clock_lead_rejected += 1;
                        }
                    }
                    Ok(_) => {}
                    Err(_) => self.stats.decode_errors += 1,
                }
            }
            _ => self.stats.other_messages += 1,
        }
    }

    fn on_symbol(&mut self, symbol: &str, payload: &[u8], arrival: i64, out: &mut FeedOut) {
        let Ok(upd) = self.book.apply(symbol, payload) else {
            self.stats.decode_errors += 1;
            return;
        };
        let Ok(m) = SymbolMessage::decode(payload) else {
            self.stats.decode_errors += 1;
            return;
        };
        self.stats.symbol_messages += 1;
        self.fresh_since_lost.insert(symbol.to_owned());
        let initial = upd.kind == UpdateKind::Initial;
        if initial {
            self.stats.initials += 1;
        }
        if let Some(code) = m.stock_status {
            if self.phases.get(symbol) != Some(&code) {
                self.phases.insert(symbol.to_owned(), code);
                out.phases.push((symbol.to_owned(), code));
            }
        }

        // Time (rule 3) and the clock candidate (rule 4).
        let mut clock_t = None;
        let t = match self.cfg.time {
            TimeSource::Arrival => {
                self.stats.time_direct += 1;
                Some(arrival)
            }
            TimeSource::UpdateDate => {
                let ud = m.update_date.as_deref().map(parse_update_date);
                if matches!(ud, Some(None)) {
                    self.stats.bad_update_date += 1;
                }
                if let Some(Some(ms)) = ud {
                    self.stats.time_direct += 1;
                    if ms <= arrival + MAX_LEAD_MS {
                        clock_t = Some(ms);
                    } else {
                        self.stats.clock_lead_rejected += 1;
                    }
                    if !initial {
                        self.anchor = Some((ms, arrival));
                    }
                    Some(ms)
                } else if let Some((ud0, a0)) = self.anchor {
                    self.stats.time_derived += 1;
                    Some(ud0 + (arrival - a0))
                } else {
                    None
                }
            }
        };

        let (cum_v, cum_q) = match self.cfg.volume_field {
            VolumeField::DailyVolume => (m.daily_volume, m.daily_quantity),
            VolumeField::Volume => (m.volume, m.quantity),
        };
        let has_bar_fields = m.last.is_some()
            || m.open.is_some()
            || m.daily_high.is_some()
            || m.daily_low.is_some()
            || cum_v.is_some()
            || cum_q.is_some();
        if !(has_bar_fields || initial) {
            self.stats.no_bar_fields += 1;
        } else if let Some(t) = t {
            self.stats.ticks += 1;
            let tick = CardTick {
                t_ms: t,
                initial,
                last: m.last,
                day_open: m.open,
                day_high: m.daily_high,
                day_low: m.daily_low,
                cum_volume: cum_v,
                cum_quantity: cum_q,
                phase: m.stock_status,
            };
            self.push_tick(symbol, &tick, out);
        } else {
            self.stats.time_unknown += 1;
        }

        if let Some(c) = clock_t {
            self.move_clock(c, self.grace_ms, out);
        }
    }

    fn push_tick(&mut self, symbol: &str, tick: &CardTick, out: &mut FeedOut) {
        for i in 0..self.intervals.len() {
            let key = self.key(symbol, &self.intervals[i]);
            if !self.feeds.contains_key(&key) {
                let Ok(mut lb) = self.builder(&key, 1) else {
                    continue;
                };
                if let Some(c) = self.clock_ms {
                    let _ = lb.close_until(c - self.grace_ms);
                }
                self.feeds.insert(key.clone(), lb);
                self.stats.feeds += 1;
            }
            let Some(lb) = self.feeds.get_mut(&key) else {
                continue;
            };
            let prev = lb.current().map(|(b, fin)| (b.t, fin));
            let events = lb.push(tick);
            if let Some((pt, false)) = prev {
                if events
                    .iter()
                    .any(|e| matches!(e.change, DataChange::Bar { .. }))
                {
                    out.finalized.push((key.clone(), pt));
                    self.stats.finalized += 1;
                }
            }
            out.events.extend(events);
        }
    }
}
