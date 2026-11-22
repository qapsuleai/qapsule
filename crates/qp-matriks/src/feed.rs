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
//!    the delayed endpoint sends `+03`). An **initial**'s `updateDate` is the symbol's last
//!    trade (possibly hours old) while the snapshot describes the present, so an initial
//!    is timed `max(updateDate, clock)`: otherwise it would fall into a final bar, be
//!    dropped as late, and its counter would not become the day's volume base (the next
//!    trade would then carry the whole day's volume). A message without `updateDate` (or with an
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
//! 9. **Trades (RP4, K66).** A feed made with [`LiveFeed::with_trades`] builds every bar
//!    from the trade stream (`mx/trade/<SYM>`, `mx/tradeex/<SYM>`; [`crate::trade`]) with
//!    [`BarMethod::Trades`]; the price card keeps the day fields, the session phase and the
//!    clock but builds no bar. Trade-endpoint connection records go in through
//!    [`LiveFeed::push_from`] with [`FeedSource::Trade`] (same pipeline, same `LiveFeed`).
//!    There is **no initial state** on the trade stream: every watched symbol starts with
//!    a **gap** (from [`LiveFeed::watch_trades`]'s `history_from_ms`, or the local midnight
//!    of its first trade) and a lost trade connection opens a gap at the time trades were
//!    known complete. While a symbol has a gap its trades are buffered, its bars are not
//!    finalized and it is **stale** ([`LiveFeed::trade_stale_symbols`]). Once the trade
//!    connection is open, [`LiveFeed::pending_trade_repairs`] lists the gaps and
//!    [`LiveFeed::apply_trade_history`] (or [`LiveFeed::repair_trades`] with a
//!    [`TradeHistory`]) fills them: history ∪ buffer, de-duplicated by trade number
//!    ([`crate::trade::Trade::dedup_key`]), applied in exchange order, so the bars equal
//!    those built from the full trade list. Buyer and seller never enter the feed.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use prost::Message as _;
use qp_calendar::{ExchangeCalendar, Interval};
use qp_schema::data::{DataChange, DataEvent, FeedKey};
use qp_schema::Bar;
use qp_source::{BarMethod, CardTick, LiveBars, LiveConfig, SpecialTrades, VolumeUnit};

use crate::clock::decode_time;
use crate::messages::SymbolMessage;
use crate::rawlog::{RawRecord, RecordKind};
use crate::symbol::{InitialPolicy, SymbolBook, UpdateKind};
use crate::topic::{parse_topic, Topic};
use crate::trade::{
    decode_trade, Trade, TradeDecodeConfig, TradeError, TradeHistory, TradeKey, TradeTimeUnit,
};

mod ingest;
mod output;
mod trades;

pub use output::{ConnChange, FeedOut, FeedStats, TradeFeedStats};
pub use trades::{FeedSource, TradeFeedConfig, TradeGap};

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

/// Per-symbol trade state.
#[derive(Debug, Clone, Default)]
struct TradeSym {
    /// Open gap: trades before this are known; after it, until repaired, they are not.
    gap_from: Option<i64>,
    /// Live trades received during the gap.
    buffer: Vec<Trade>,
    /// Trades are complete up to here (UTC ms).
    covered_ms: Option<i64>,
    /// De-duplication: keys seen, with their times, oldest first.
    seen: BTreeSet<SeenKey>,
    recent: VecDeque<(i64, SeenKey)>,
}

/// (RP4/T6) A de-duplication key as kept for the window: a plain decimal trade number as
/// its value (no allocation; the usual case), anything else as the [`TradeKey`] itself.
/// One-to-one with [`TradeKey`] (a plain decimal has no leading zero, so it is the only
/// text of its value).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum SeenKey {
    Number(u64),
    Other(Box<TradeKey>),
}

impl SeenKey {
    fn number(k: &TradeKey) -> Option<u64> {
        let TradeKey::No(n) = k else { return None };
        let plain = !n.is_empty()
            && n.len() <= 19
            && n.bytes().all(|b| b.is_ascii_digit())
            && (n == "0" || !n.starts_with('0'));
        plain.then(|| n.parse::<u64>().ok()).flatten()
    }

    fn of(k: TradeKey) -> Self {
        match Self::number(&k) {
            Some(v) => Self::Number(v),
            None => Self::Other(Box::new(k)),
        }
    }
}

impl TradeSym {
    fn seen(&self, key: &TradeKey) -> bool {
        match SeenKey::number(key) {
            Some(v) => self.seen.contains(&SeenKey::Number(v)),
            None => self.seen.contains(&SeenKey::Other(Box::new(key.clone()))),
        }
    }

    fn remember(&mut self, t_ms: i64, key: TradeKey, window_ms: i64) {
        let key = SeenKey::of(key);
        if self.seen.insert(key.clone()) {
            self.recent.push_back((t_ms, key));
        }
        let newest = self.recent.back().map_or(t_ms, |(t, _)| *t).max(t_ms);
        let keep_from = newest - window_ms;
        let keep_from = self
            .gap_from
            .map_or(keep_from, |g| keep_from.min(g - window_ms));
        while self.recent.front().is_some_and(|(t, _)| *t < keep_from) {
            if let Some((_, k)) = self.recent.pop_front() {
                self.seen.remove(&k);
            }
        }
    }
}

/// Trade stream state of a feed.
#[derive(Debug, Clone)]
struct TradeState {
    cfg: TradeFeedConfig,
    /// The trade connection is open (after `ConnectionOpened`, until `ConnectionLost`).
    live: bool,
    sessions: u32,
    syms: BTreeMap<String, TradeSym>,
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
    trades: Option<TradeState>,
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
            trades: None,
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
            _ if self.trades.is_some() => BarMethod::Trades,
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
                special_trades: self
                    .trades
                    .as_ref()
                    .map_or(SpecialTrades::default(), |t| t.cfg.special),
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
            if lb.stats().ticks > 0 || lb.stats().trades > 0 {
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

    /// Applies one record (message or connection event) from the market endpoint.
    pub fn push(&mut self, rec: &RawRecord) -> FeedOut {
        self.push_from(FeedSource::Market, rec)
    }

    /// Applies one record from `source`. Messages are routed by topic whatever the
    /// source; connection records of [`FeedSource::Trade`] change the trade connection
    /// (rule 9) and leave the market state alone.
    pub fn push_from(&mut self, source: FeedSource, rec: &RawRecord) -> FeedOut {
        let mut out = FeedOut::default();
        self.stats.records += 1;
        let arrival = rec.t_us.div_euclid(1000);
        if source == FeedSource::Trade && rec.kind != RecordKind::Message {
            self.on_trade_conn(rec.kind, &mut out);
            if self.cfg.time == TimeSource::Arrival {
                self.move_clock(arrival, self.grace_ms, &mut out);
            }
            return out;
        }
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
}

#[cfg(test)]
mod seen_key_tests {
    use super::{SeenKey, TradeKey, TradeSym};

    /// (RP4/T6) The compact window key is one-to-one with the trade key: plain decimals
    /// become numbers, anything else stays itself; the window answers as before.
    #[test]
    fn seen_keys_are_one_to_one() {
        let keys = [
            TradeKey::No("0".to_owned()),
            TradeKey::No("00".to_owned()),
            TradeKey::No("12".to_owned()),
            TradeKey::No("012".to_owned()),
            TradeKey::No("18446744073709551615".to_owned()),
            TradeKey::No("18446744073709551616".to_owned()),
            TradeKey::No("1a".to_owned()),
            TradeKey::Fields(5, 7, Some(1), false),
            TradeKey::Fields(5, 7, None, false),
        ];
        for a in &keys {
            for b in &keys {
                assert_eq!(
                    a == b,
                    SeenKey::of(a.clone()) == SeenKey::of(b.clone()),
                    "{a:?} {b:?}"
                );
            }
        }
        assert_eq!(SeenKey::of(keys[2].clone()), SeenKey::Number(12));
        let mut s = TradeSym::default();
        for (i, k) in keys.iter().enumerate() {
            assert!(!s.seen(k));
            s.remember(i64::try_from(i).expect("small"), k.clone(), 60_000);
            assert!(s.seen(k));
        }
        // Past the window the oldest are forgotten.
        s.remember(200_000, TradeKey::No("77".to_owned()), 60_000);
        assert!(!s.seen(&keys[0]));
        assert!(s.seen(&TradeKey::No("77".to_owned())));
    }
}
