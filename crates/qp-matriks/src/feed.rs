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
    /// A trade-endpoint connection change (rule 9).
    pub trade_connection: Option<ConnChange>,
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
            && self.trade_connection.is_none()
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
    /// Pushed ticks timed by their own `updateDate` (or arrival in `Arrival` mode).
    pub time_direct: u64,
    /// Pushed ticks timed by the anchor rule (no / unreadable `updateDate`).
    pub time_derived: u64,
    /// Ticks dropped: no time known yet.
    pub time_unknown: u64,
    /// Ticks whose time was before the clock when they arrived (out of order).
    pub behind_clock: u64,
    /// Largest such lag (ms); the finalize grace should cover most of it.
    pub max_behind_ms: i64,
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
    /// Trade stream counters (rule 9).
    pub trades: TradeFeedStats,
}

/// Trade stream counters (no market values).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TradeFeedStats {
    /// Trade messages received.
    pub messages: u64,
    /// Payloads that are not a `TradeMessage`.
    pub decode_errors: u64,
    /// Trades without a valid price.
    pub no_price: u64,
    /// Trades without a usable timestamp.
    pub no_time: u64,
    /// Trades whose topic symbol and payload symbol differ (the topic wins).
    pub symbol_mismatch: u64,
    /// Decoded trades, by the time unit actually used (`Auto` shows which one won).
    pub unit_ns_from_midnight: u64,
    /// See `unit_ns_from_midnight`.
    pub unit_epoch_ms: u64,
    /// See `unit_ns_from_midnight`.
    pub unit_epoch_us: u64,
    /// See `unit_ns_from_midnight`.
    pub unit_epoch_ns: u64,
    /// Live trades whose time is more than 60 s from their arrival (a wrong unit shows
    /// here first).
    pub far_from_arrival: u64,
    /// Special trades seen (`isTradeEx` or `mx/tradeex`).
    pub special: u64,
    /// Trades dropped as duplicates (same trade number).
    pub duplicates: u64,
    /// Live trades held while their symbol had a gap.
    pub buffered: u64,
    /// History trades before the gap start (already covered): dropped.
    pub history_before_gap: u64,
    /// History trades received inside gaps (before de-duplication).
    pub history_applied: u64,
    /// Gaps filled.
    pub repairs: u64,
    /// History requests that failed (the gap stays).
    pub repair_errors: u64,
    /// Trade-endpoint connections opened.
    pub connections_opened: u64,
    /// Trade-endpoint connections lost.
    pub connections_lost: u64,
}

/// Which connection a record came from (rule 9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FeedSource {
    /// The market endpoint (price cards, clock, notices).
    #[default]
    Market,
    /// The trade endpoint (`/trade`).
    Trade,
}

/// Trade input settings (rule 9).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TradeFeedConfig {
    /// Decoding (time unit).
    pub decode: TradeDecodeConfig,
    /// Whether special (reported) trades join the bars (default: no).
    pub special: SpecialTrades,
    /// How long trade numbers are remembered behind the newest trade, for de-duplication
    /// (ms; default 30 minutes).
    pub dedup_window_ms: i64,
}

impl Default for TradeFeedConfig {
    fn default() -> Self {
        Self {
            decode: TradeDecodeConfig::default(),
            special: SpecialTrades::Exclude,
            dedup_window_ms: 30 * 60_000,
        }
    }
}

/// A trade gap to fill from history: `[from_ms, to_ms]`, UTC ms, inclusive.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TradeGap {
    /// Symbol.
    pub symbol: String,
    /// From (the time trades were known complete).
    pub from_ms: i64,
    /// To (the clock or the newest buffered trade, whichever is later).
    pub to_ms: i64,
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
    seen: BTreeSet<TradeKey>,
    recent: VecDeque<(i64, TradeKey)>,
}

impl TradeSym {
    fn remember(&mut self, t_ms: i64, key: TradeKey, window_ms: i64) {
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

    /// A feed whose bars come from the trade stream (rule 9). `cfg.method` is ignored
    /// (every builder uses [`BarMethod::Trades`]); `cfg.volume` applies.
    #[must_use]
    pub fn with_trades(
        cfg: LiveFeedConfig,
        cal: ExchangeCalendar,
        trades: TradeFeedConfig,
    ) -> Self {
        let mut f = Self::new(cfg, cal);
        f.trades = Some(TradeState {
            cfg: trades,
            live: false,
            sessions: 0,
            syms: BTreeMap::new(),
        });
        f
    }

    /// Whether bars come from trades.
    #[must_use]
    pub fn trades_enabled(&self) -> bool {
        self.trades.is_some()
    }

    /// Starts watching `symbol`'s trades: everything from `history_from_ms` (UTC ms; e.g.
    /// the close of the last complete REST bar, or the day's start) until the trade
    /// connection delivers is a gap to fill from history. A symbol already watched is
    /// unchanged. No effect without trades.
    pub fn watch_trades(&mut self, symbol: &str, history_from_ms: i64) {
        if let Some(ts) = self.trades.as_mut() {
            ts.syms
                .entry(symbol.to_owned())
                .or_insert_with(|| TradeSym {
                    gap_from: Some(history_from_ms),
                    covered_ms: Some(history_from_ms),
                    ..TradeSym::default()
                });
        }
    }

    /// Whether the trade connection is open.
    #[must_use]
    pub fn trade_connected(&self) -> bool {
        self.trades.as_ref().is_some_and(|t| t.live)
    }

    /// Symbols whose trade data is stale: all watched symbols while the trade connection is
    /// down, else those with an open gap. Empty without trades.
    #[must_use]
    pub fn trade_stale_symbols(&self) -> Vec<String> {
        let Some(ts) = &self.trades else {
            return Vec::new();
        };
        ts.syms
            .iter()
            .filter(|(_, s)| !ts.live || s.gap_from.is_some())
            .map(|(k, _)| k.clone())
            .collect()
    }

    /// The gaps to fill now (only while the trade connection is open; rule 9). A gap ends
    /// at the later of the clock and the newest buffered trade: give the feed the exchange
    /// time first ([`advance_clock`](Self::advance_clock)) or a quiet symbol's gap ends
    /// where it starts. Trades between that end and the first live trade are covered only
    /// if the history already has them (de-duplication makes a second request harmless).
    #[must_use]
    pub fn pending_trade_repairs(&self) -> Vec<TradeGap> {
        let Some(ts) = self.trades.as_ref().filter(|t| t.live) else {
            return Vec::new();
        };
        ts.syms
            .iter()
            .filter_map(|(sym, s)| {
                let from = s.gap_from?;
                let newest = s.buffer.iter().map(Trade::t_ms).max();
                let to = [Some(from), self.clock_ms, newest]
                    .into_iter()
                    .flatten()
                    .max()
                    .unwrap_or(from);
                Some(TradeGap {
                    symbol: sym.clone(),
                    from_ms: from,
                    to_ms: to,
                })
            })
            .collect()
    }

    /// Fills `gap` with `history` (any order, duplicates allowed): history trades of the
    /// symbol at or after the gap start and the trades buffered during the gap are
    /// de-duplicated, sorted into exchange order and applied. The gap closes if the trade
    /// connection is open (else a new one starts where trades are now complete). Trades of
    /// another symbol are ignored.
    pub fn apply_trade_history(&mut self, gap: &TradeGap, history: Vec<Trade>) -> FeedOut {
        let mut out = FeedOut::default();
        let Some(ts) = self.trades.as_mut() else {
            return out;
        };
        let live = ts.live;
        let window = ts.cfg.dedup_window_ms;
        let Some(st) = ts.syms.get_mut(&gap.symbol) else {
            return out;
        };
        let from = st.gap_from.unwrap_or(gap.from_ms);
        let mut all: Vec<Trade> = Vec::with_capacity(history.len() + st.buffer.len());
        for t in history {
            if t.symbol != gap.symbol {
                continue;
            }
            if t.t_ms() < from {
                self.stats.trades.history_before_gap += 1;
                continue;
            }
            all.push(t);
        }
        let n_hist = all.len();
        let buffered = std::mem::take(&mut st.buffer);
        all.extend(buffered);
        all.sort_by(Trade::exchange_cmp);
        let mut to_apply = Vec::with_capacity(all.len());
        for t in all {
            let k = t.dedup_key();
            if st.seen.contains(&k) {
                self.stats.trades.duplicates += 1;
                continue;
            }
            st.remember(t.t_ms(), k, window);
            to_apply.push(t);
        }
        let newest = to_apply.iter().map(Trade::t_ms).max();
        let covered = [st.covered_ms, Some(gap.to_ms), newest]
            .into_iter()
            .flatten()
            .max();
        st.covered_ms = covered;
        st.gap_from = if live { None } else { covered };
        self.stats.trades.repairs += 1;
        self.stats.trades.history_applied += n_hist as u64;
        let symbol = gap.symbol.clone();
        for t in &to_apply {
            self.push_trade_tick(&symbol, t, &mut out);
        }
        if live {
            if let Some(c) = self.clock_ms {
                let until = c - self.grace_ms;
                for (k, lb) in &mut self.feeds {
                    if k.symbol == symbol {
                        if let Some(b) = lb.close_until(until) {
                            out.finalized.push((k.clone(), b.t));
                            self.stats.finalized += 1;
                        }
                    }
                }
            }
        }
        out
    }

    /// Fills every pending gap from `history` ([`pending_trade_repairs`](Self::pending_trade_repairs)
    /// then [`apply_trade_history`](Self::apply_trade_history)). A failed request leaves its
    /// gap open (counted); its error text is returned with the symbol.
    pub fn repair_trades(
        &mut self,
        history: &dyn TradeHistory,
    ) -> (FeedOut, Vec<(String, String)>) {
        let mut out = FeedOut::default();
        let mut errors = Vec::new();
        for gap in self.pending_trade_repairs() {
            match history.trades(&gap.symbol, gap.from_ms, gap.to_ms) {
                Ok(trades) => {
                    let o = self.apply_trade_history(&gap, trades);
                    out.events.extend(o.events);
                    out.finalized.extend(o.finalized);
                }
                Err(e) => {
                    self.stats.trades.repair_errors += 1;
                    errors.push((gap.symbol.clone(), e));
                }
            }
        }
        (out, errors)
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
                    .map_or(SpecialTrades::Exclude, |t| t.cfg.special),
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

    fn move_clock(&mut self, t: i64, grace: i64, out: &mut FeedOut) {
        if self.clock_ms.is_some_and(|c| c >= t) {
            return;
        }
        self.clock_ms = Some(t);
        out.clock_ms = Some(t);
        let until = t - grace;
        let gaps: BTreeSet<&str> = self.trades.as_ref().map_or_else(BTreeSet::new, |ts| {
            ts.syms
                .iter()
                .filter(|(_, s)| s.gap_from.is_some())
                .map(|(k, _)| k.as_str())
                .collect()
        });
        for (k, lb) in &mut self.feeds {
            if gaps.contains(k.symbol.as_str()) {
                continue;
            }
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
            Topic::Trade {
                symbol, special, ..
            } => {
                let (symbol, special) = (symbol.clone(), *special);
                self.on_trade(&symbol, special, &rec.payload, arrival, out);
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
            TimeSource::Arrival => Some((arrival, false)),
            TimeSource::UpdateDate => {
                let ud = m.update_date.as_deref().map(parse_update_date);
                if matches!(ud, Some(None)) {
                    self.stats.bad_update_date += 1;
                }
                if let Some(Some(ms)) = ud {
                    if ms <= arrival + MAX_LEAD_MS {
                        clock_t = Some(ms);
                    } else {
                        self.stats.clock_lead_rejected += 1;
                    }
                    if !initial {
                        self.anchor = Some((ms, arrival));
                    }
                    // An initial's `updateDate` is the symbol's last trade, possibly long
                    // ago: the snapshot describes now, so it is not placed before the clock.
                    let t = if initial {
                        self.clock_ms.map_or(ms, |c| c.max(ms))
                    } else {
                        ms
                    };
                    Some((t, false))
                } else if let Some((ud0, a0)) = self.anchor {
                    Some((ud0 + (arrival - a0), true))
                } else {
                    None
                }
            }
        };

        let (cum_v, cum_q) = match self.cfg.volume_field {
            VolumeField::DailyVolume => (m.daily_volume, m.daily_quantity),
            VolumeField::Volume => (m.volume, m.quantity),
        };
        if self.trades.is_some() {
            // Rule 9: bars come from trades; the card gives the phase and the clock only.
            if let Some(c) = clock_t {
                self.move_clock(c, self.grace_ms, out);
            }
            return;
        }
        let has_bar_fields = m.last.is_some()
            || m.open.is_some()
            || m.daily_high.is_some()
            || m.daily_low.is_some()
            || cum_v.is_some()
            || cum_q.is_some();
        if !(has_bar_fields || initial) {
            self.stats.no_bar_fields += 1;
        } else if let Some((t, derived)) = t {
            self.stats.ticks += 1;
            if derived {
                self.stats.time_derived += 1;
            } else {
                self.stats.time_direct += 1;
            }
            if let Some(c) = self.clock_ms.filter(|&c| c > t && !initial) {
                self.stats.behind_clock += 1;
                self.stats.max_behind_ms = self.stats.max_behind_ms.max(c - t);
            }
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

    fn on_trade_conn(&mut self, kind: RecordKind, out: &mut FeedOut) {
        let Some(ts) = self.trades.as_mut() else {
            self.stats.other_messages += 1;
            return;
        };
        match kind {
            RecordKind::ConnectionOpened => {
                ts.sessions += 1;
                ts.live = true;
                self.stats.trades.connections_opened += 1;
                out.trade_connection = Some(ConnChange::Opened {
                    session: ts.sessions,
                });
            }
            RecordKind::ConnectionLost => {
                ts.live = false;
                for s in ts.syms.values_mut() {
                    if s.gap_from.is_none() {
                        s.gap_from = s.covered_ms.or(self.clock_ms);
                    }
                }
                self.stats.trades.connections_lost += 1;
                out.trade_connection = Some(ConnChange::Lost);
            }
            RecordKind::SubscribeRejected => self.stats.subscribe_rejected += 1,
            RecordKind::Note => self.stats.notes += 1,
            RecordKind::Subscribed | RecordKind::Message => {}
        }
    }

    fn on_trade(
        &mut self,
        symbol: &str,
        special_topic: bool,
        payload: &[u8],
        arrival: i64,
        out: &mut FeedOut,
    ) {
        let Some(ts) = self.trades.as_mut() else {
            self.stats.other_messages += 1;
            return;
        };
        let st = &mut self.stats.trades;
        st.messages += 1;
        let (mut trade, unit) = match decode_trade(payload, &ts.cfg.decode, arrival, special_topic)
        {
            Ok(x) => x,
            Err(TradeError::Decode(_)) => {
                st.decode_errors += 1;
                return;
            }
            Err(TradeError::NoPrice) => {
                st.no_price += 1;
                return;
            }
            Err(TradeError::NoTime) => {
                st.no_time += 1;
                return;
            }
        };
        match unit {
            TradeTimeUnit::NanosFromMidnight => st.unit_ns_from_midnight += 1,
            TradeTimeUnit::EpochMillis => st.unit_epoch_ms += 1,
            TradeTimeUnit::EpochMicros => st.unit_epoch_us += 1,
            TradeTimeUnit::EpochNanos => st.unit_epoch_ns += 1,
            TradeTimeUnit::Auto => {}
        }
        if trade.symbol != symbol {
            st.symbol_mismatch += 1;
            symbol.clone_into(&mut trade.symbol);
        }
        if trade.is_trade_ex {
            st.special += 1;
        }
        let t = trade.t_ms();
        if (t - arrival).abs() > 60_000 {
            st.far_from_arrival += 1;
        }
        let window = ts.cfg.dedup_window_ms;
        let live = ts.live;
        let sym = ts.syms.entry(symbol.to_owned()).or_insert_with(|| {
            // Not watched: the day so far is a gap (no initial state on this stream).
            let off = i64::from(ts.cfg.decode.utc_offset_min) * 60_000;
            let midnight = (t + off).div_euclid(86_400_000) * 86_400_000 - off;
            TradeSym {
                gap_from: Some(midnight),
                covered_ms: Some(midnight),
                ..TradeSym::default()
            }
        });
        if !live && sym.gap_from.is_none() {
            sym.gap_from = sym.covered_ms;
        }
        if sym.gap_from.is_some() {
            st.buffered += 1;
            sym.buffer.push(trade);
        } else {
            let k = trade.dedup_key();
            if sym.seen.contains(&k) {
                st.duplicates += 1;
            } else {
                sym.remember(t, k, window);
                sym.covered_ms = Some(sym.covered_ms.map_or(t, |c| c.max(t)));
                self.push_trade_tick(symbol, &trade, out);
            }
        }
        if self.cfg.time == TimeSource::UpdateDate {
            if t <= arrival + MAX_LEAD_MS {
                self.move_clock(t, self.grace_ms, out);
            } else {
                self.stats.clock_lead_rejected += 1;
            }
        }
    }

    fn push_trade_tick(&mut self, symbol: &str, trade: &Trade, out: &mut FeedOut) {
        let tick = trade.tick();
        self.stats.ticks += 1;
        for i in 0..self.intervals.len() {
            let key = self.key(symbol, &self.intervals[i]);
            if !self.feeds.contains_key(&key) {
                let Ok(lb) = self.builder(&key, 1) else {
                    continue;
                };
                self.feeds.insert(key.clone(), lb);
                self.stats.feeds += 1;
            }
            let Some(lb) = self.feeds.get_mut(&key) else {
                continue;
            };
            let prev = lb.current().map(|(b, fin)| (b.t, fin));
            let events = lb.push_trade(&tick);
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
