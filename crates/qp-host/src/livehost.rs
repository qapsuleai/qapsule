//! `qp demo --live` (R6/T7, M23): the live side of the demo host.
//!
//! Threads:
//! - the **connection** thread ([`spawn_connection`]) owns the one [`LiveConnection`]
//!   (`qp-live`: lock, backoff, multi-connect stop). Its sink ([`HubSink`]) writes every raw
//!   message and connection event to the raw log under `fixtures/local/live/raw/` (K60:
//!   the record is a by-product of the connector) and hands a copy to the hub. A local
//!   `drop` closes the connection (DISCONNECT), waits, and opens it again — never two at
//!   once.
//! - the **hub** thread (`demo.rs`) runs [`LiveRuntime`]: every record goes through the
//!   feed ([`FeedEngine`]: decoder + live bars), its clock to the authority (forward only),
//!   its events to the authority ([`ReplayHost::live_event`]); the viewers are told once
//!   per batch.
//! - a **repair** thread per reconnect fetches the gap from REST ([`BarSource`]).
//!
//! Stale and repair (M23, `docs/PROTOCOL.md` §18.4): when the connection is lost every
//! live feed is marked `stale` in the authority by the §18.4 gap rule
//! ([`ReplayHost::live_mark_stale`]): the bindings give no values, so no capsule is judged
//! on them, until they are resolved again. On reconnect the messages wait in a buffer; the
//! gap is fetched from REST, the bars that are complete by the exchange clock are written
//! into the host's store as `patch` / `bar` events (recorded: devices take the same), the
//! authority resolves the feeds again ([`ReplayHost::live_refresh`]), the feed's counters
//! are seeded at the repaired last bar and `seq`, and the buffered messages go through.
//!
//! Trades (RP4, K66): a feed made with [`MatriksFeed::with_trades`] builds its bars from
//! the trade stream. Trade-endpoint records come in as [`LiveIn::Trade`] (the feed's
//! `FeedSource::Trade`); when the trade connection opens, the runtime fills every watched
//! symbol's gap from its [`TradeHistory`] ([`LiveRuntime::set_trade_history`]) after moving
//! the feed's clock to the exchange time.
//!
//! Clock (RP4/T4): the runtime reads "now" from the wall clock, or — for the accelerated
//! simulation (`qp sim`, `qp demo --sim`, `crate::sim`) — from a simulated clock moved only
//! by [`LiveIn::Clock`] ([`LiveRuntime::use_sim_clock`]). Everything the runtime schedules
//! (closing bars by the exchange clock, the market status, the lag estimate) follows that
//! clock, so the same inputs give the same outputs whatever the real pace.
//!
//! Nothing here prints a value, a secret or an endpoint address.

use std::collections::BTreeMap;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

use qp_calendar::{ExchangeCalendar, Interval};
use qp_live::{LiveConfig as ConnConfig, LiveConnection, LiveEvent, LiveMessage, LiveSink};
use qp_matriks::{FeedSource, RawRecord, RecordKind, Trade, TradeGap, TradeHistory};
use qp_schema::data::{DataChange, DataEvent, FeedKey};
use qp_schema::protocol::OutboundLine;
use qp_schema::Bar;
use qp_session::ReplayHost;
use serde_json::{json, Value};

use crate::live::RawSink;

pub use qp_matriks::{ConnChange, FeedOut};

mod conn;
mod feed;
mod rest;
mod runtime;
mod status;

pub use conn::*;
pub use feed::*;
pub use rest::*;

/// A feed's seam: symbol, last bar, the first live event's `seq`, the day's cumulative
/// counter at the seam (`None`: the first snapshot's counter is the base).
pub type Seed = (String, Option<Bar>, u64, Option<f64>);

/// The feed: raw records in, data events out ([`MatriksFeed`] over the shared
/// `qp_matriks::LiveFeed`; a fake in tests).
pub trait FeedEngine: Send {
    /// Starts every feed after its last historical bar, numbering its events from the
    /// given `seq` (§18.4: the initial bars hold every event before it). Called at the
    /// start and again after a repair (then on feeds that already had live ticks).
    ///
    /// # Errors
    /// A bar the builder refuses.
    fn seed_all(&mut self, seeds: &[Seed]) -> Result<(), String>;
    /// One raw record.
    fn push(&mut self, rec: &RawRecord) -> FeedOut;
    /// (RP4) One raw record from `source` (the trade endpoint's records: `Trade`). A feed
    /// without trades ignores trade-endpoint records.
    fn push_from(&mut self, source: FeedSource, rec: &RawRecord) -> FeedOut {
        match source {
            FeedSource::Market => self.push(rec),
            FeedSource::Trade => FeedOut::default(),
        }
    }
    /// (RP4) Starts watching `symbols`' trades: everything from `from_ms` is a gap until
    /// the trade connection delivers and the gap is filled from history. No effect
    /// without trades.
    fn watch_trades(&mut self, _symbols: &[String], _from_ms: i64) {}
    /// (RP4) Moves the clock to `now_ms` and fills every open trade gap from `history`;
    /// the errors are `(symbol, text)`. No effect without trades.
    fn repair_trades(
        &mut self,
        _now_ms: i64,
        _history: &dyn TradeHistory,
    ) -> (FeedOut, Vec<(String, String)>) {
        (FeedOut::default(), Vec::new())
    }
    /// (RP4, L-prep) Whether bars come from the trade stream.
    fn trades_enabled(&self) -> bool {
        false
    }
    /// (RP4, L-prep) Starts watching `symbols`' trades from each symbol's seam (the end of
    /// its seeded last bar), or from `fallback_ms` without one.
    fn watch_trades_after_seams(&mut self, _symbols: &[String], _fallback_ms: i64) {}
    /// (RP4, L-prep) Moves the clock to `now_ms` and lists the open trade gaps (empty while
    /// the trade connection is down).
    fn trade_gaps(&mut self, _now_ms: i64) -> (FeedOut, Vec<TradeGap>) {
        (FeedOut::default(), Vec::new())
    }
    /// (RP4, L-prep) Fills `gap` with `trades` (history; any order, duplicates allowed).
    fn apply_trade_history(&mut self, _gap: &TradeGap, _trades: Vec<Trade>) -> FeedOut {
        FeedOut::default()
    }
    /// The exchange clock reached `now_ms` (bars close).
    fn advance_clock(&mut self, now_ms: i64) -> FeedOut;
    /// Counters for the stats (no values).
    fn stats_json(&self) -> Value {
        Value::Null
    }
}

/// Settled bars for history and repair (REST; a fake in tests).
pub trait BarSource: Send + Sync {
    /// Bars of each symbol on `interval` covering `from_ms..=to_ms` (calendar-aligned,
    /// closing session merged), in order.
    fn fetch(
        &self,
        symbols: &[String],
        interval: &str,
        from_ms: i64,
        to_ms: i64,
    ) -> Vec<(String, Result<Vec<Bar>, String>)>;
}

/// Input of the hub's live side.
#[derive(Debug)]
pub enum LiveIn {
    /// A raw record (message or connection event).
    Record(RawRecord),
    /// A repair finished (generation, per-symbol settled bars, fetch time).
    Repaired {
        /// The stale period it repairs.
        generation: u64,
        /// Settled bars per symbol.
        bars: Vec<(String, Result<Vec<Bar>, String>)>,
        /// Fetch duration.
        fetch_ms: u64,
    },
    /// The connection stopped for good (halt: multi-connect, too many failures).
    Halted(String),
    /// A connection event worth showing (backoff, auth failure, notice; masked text).
    Note(String),
    /// (RP4) A raw record from the trade endpoint (`FeedSource::Trade`).
    Trade(RawRecord),
    /// (RP4, L-prep) Trade history for the gaps of a trade (re)connection, fetched off the
    /// hub thread.
    TradeHistory {
        /// The stale period it repairs (results of an older one are dropped).
        generation: u64,
        /// Per gap, the history or the (masked) error.
        results: Vec<(TradeGap, Result<Vec<Trade>, String>)>,
        /// Fetch duration.
        fetch_ms: u64,
    },
    /// (RP4/T4) The simulated clock moved to this time (UTC µs; the simulation's exchange
    /// time). Ignored unless the runtime uses the simulated clock
    /// ([`LiveRuntime::use_sim_clock`]); then the periodic work runs at that time.
    Clock(i64),
    /// (RP4/T4) Answered once every input before it was handled (back-pressure for a
    /// producer that runs ahead, e.g. the simulation at `--speed max`).
    Sync(Sender<()>),
    /// (RP4/T6) The paced simulation's wall clock (UTC µs) when it sent the inputs that
    /// follow (the trades' "arrival"): tick-to-frame on the simulated clock is measured
    /// from it, as live measures from the connection thread's arrival time. Changes no
    /// output.
    Arrival(i64),
}

/// Link state of the live side.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Link {
    /// Before the first connection.
    Connecting,
    /// Data flows.
    Live,
    /// The connection is lost; feeds are stale.
    Stale,
    /// Reconnected; the gap is being repaired, messages wait.
    Repairing,
    /// Halted for good.
    Halted,
}

impl Link {
    fn name(&self) -> &'static str {
        match self {
            Self::Connecting => "connecting",
            Self::Live => "live",
            Self::Stale => "stale",
            Self::Repairing => "repairing",
            Self::Halted => "halted",
        }
    }
}

/// One repair (for the stats; counts only).
#[derive(Debug, Clone)]
struct RepairStat {
    stale_ms: u64,
    fetch_ms: u64,
    total_ms: u64,
    feeds: usize,
    patched: usize,
    added: usize,
    failed: usize,
    buffered: usize,
}

/// One trade history fill (for the stats; counts only).
#[derive(Debug, Clone)]
struct TradeFillStat {
    fetch_ms: u64,
    total_ms: u64,
    gaps: usize,
    failed: usize,
    trades: usize,
    after_loss: bool,
}

/// A capsule event for the log (no values: kind, state, exchange time, symbol).
#[derive(Debug, Clone)]
struct CapsuleEvent {
    kind: &'static str,
    state: String,
    t: i64,
    symbol: String,
    stale: bool,
}

/// The live side of the hub (see the module docs).
pub struct LiveRuntime {
    feed: Box<dyn FeedEngine>,
    source: Arc<dyn BarSource>,
    to_hub: Arc<dyn Fn(LiveIn) + Send + Sync>,
    cal: ExchangeCalendar,
    interval: String,
    iv: Interval,
    feeds: Vec<FeedKey>,
    link: Link,
    generation: u64,
    stale_since: Option<Instant>,
    stale_from_t: Option<i64>,
    repair_started: Option<Instant>,
    buffer: Vec<(FeedSource, RawRecord)>,
    /// Exchange clock: the latest exchange time seen, the arrival lag estimate.
    clock: i64,
    lag_ms: i64,
    lag_cur: Option<i64>,
    lag_prev: Option<i64>,
    /// When (runtime clock, µs) the lag window, the bar closing and the market status
    /// last ran.
    lag_rot_us: i64,
    last_advance_us: i64,
    last_print: Instant,
    /// (RP4/T4) The simulated clock (UTC µs), when used ([`LiveRuntime::use_sim_clock`]).
    sim_now_us: Option<i64>,
    /// (RP4/T6) The paced simulation's wall-clock arrival of the current inputs
    /// ([`LiveIn::Arrival`]).
    sim_arrival_us: Option<i64>,
    /// (RP4) Where trade gaps are filled from when the trade connection opens.
    trade_history: Option<Arc<dyn TradeHistory + Send + Sync>>,
    /// (RP4) Trade gap repairs: (filled, failed requests).
    pub trade_repairs: (u64, u64),
    /// (RP4, L-prep) Bars come from the trade stream: the trade connection decides
    /// whether the feeds are live or stale; the market connection gives the phases and
    /// the clock only (its loss stales nothing, its reconnect repairs nothing).
    trade_mode: bool,
    /// (RP4, L-prep) Connection states (for the stats and the market status).
    market_up: bool,
    trade_up: bool,
    /// (RP4, L-prep) Fill trade gaps on the hub thread (the simulation: deterministic) or
    /// on a fetch thread (live: REST must not block the hub).
    trade_fill_sync: bool,
    /// (RP4, L-prep) A trade history fetch is running (since).
    trade_fetch: Option<Instant>,
    /// When (runtime clock, µs) the last trade history fetch started.
    trade_fetch_at_us: i64,
    /// The clock move of the fetch's start, given to the authority with its results.
    trade_fetch_clock: Option<i64>,
    /// The authority's feeds were marked stale (a refresh is due when they are live again).
    marked_stale: bool,
    trade_fills: Vec<TradeFillStat>,
    pub delayed: bool,
    // Stats.
    started: Instant,
    pub messages: u64,
    pub events: u64,
    pub finalized: u64,
    pub dropped_while_stale: u64,
    pub rejected: u64,
    pub busy: Duration,
    arrivals: Vec<i64>,
    latency_us: Vec<i64>,
    repairs: Vec<RepairStat>,
    phases: BTreeMap<i32, u64>,
    capsules: BTreeMap<String, (String, u32)>,
    capsule_log: Vec<CapsuleEvent>,
    pub drops_asked: u64,
    pub halted: Option<String>,
    conn_events: Vec<String>,
    /// (T3, §24) The market status last given to the authority, the phases by symbol and
    /// when it was last given.
    market: Option<qp_schema::market::MarketStatus>,
    sym_phase: BTreeMap<String, i32>,
    market_at_us: i64,
    market_dirty: bool,
}

/// Most latency samples kept (a ring).
const MAX_SAMPLES: usize = 200_000;

fn now_us() -> i64 {
    crate::live::now_us()
}

fn pct(v: &[i64], p: f64) -> i64 {
    if v.is_empty() {
        return 0;
    }
    let mut s = v.to_vec();
    s.sort_unstable();
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    let i = ((s.len() - 1) as f64 * p).round() as usize;
    s[i]
}

impl LiveRuntime {
    /// A runtime over `host`'s feeds; `delay_ms` is the default arrival lag (delayed
    /// mode: 15 minutes). Seeds every feed at its last initial bar.
    ///
    /// # Errors
    /// Unknown interval; a feed the engine refuses.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        host: &ReplayHost,
        mut feed: Box<dyn FeedEngine>,
        source: Arc<dyn BarSource>,
        to_hub: Arc<dyn Fn(LiveIn) + Send + Sync>,
        cal: ExchangeCalendar,
        interval: &str,
        delay_ms: i64,
    ) -> Result<Self, String> {
        let iv = Interval::parse(interval).map_err(|e| e.to_string())?;
        let feeds: Vec<FeedKey> = host
            .init()
            .data
            .iter()
            .flat_map(|d| d.feeds.iter().map(|f| f.feed.clone()))
            .collect();
        // The seam's day counter is not given: which card counter equals REST `volume`
        // is still open (M22), so the first snapshot's counter is the day's base
        // (`LiveFeed::seed`); the bar the first tick falls in misses the volume traded
        // in it before that snapshot.
        let mut seeds = Vec::new();
        for k in &feeds {
            let (bars, seq) = host.live_feed(k).ok_or("live: feed without store")?;
            seeds.push((k.symbol.clone(), bars.last().cloned(), seq + 1, None));
        }
        feed.seed_all(&seeds)?;
        let trade_mode = feed.trades_enabled();
        Ok(Self {
            feed,
            source,
            to_hub,
            cal,
            interval: interval.to_owned(),
            iv,
            feeds,
            link: Link::Connecting,
            generation: 0,
            stale_since: None,
            stale_from_t: None,
            repair_started: None,
            buffer: Vec::new(),
            clock: host.session().clock(),
            lag_ms: delay_ms,
            lag_cur: None,
            lag_prev: None,
            lag_rot_us: now_us(),
            last_advance_us: now_us(),
            last_print: Instant::now(),
            sim_now_us: None,
            sim_arrival_us: None,
            trade_history: None,
            trade_repairs: (0, 0),
            trade_mode,
            market_up: false,
            trade_up: false,
            trade_fill_sync: false,
            trade_fetch: None,
            trade_fetch_at_us: 0,
            trade_fetch_clock: None,
            marked_stale: false,
            trade_fills: Vec::new(),
            delayed: delay_ms > 0,
            started: Instant::now(),
            messages: 0,
            events: 0,
            finalized: 0,
            dropped_while_stale: 0,
            rejected: 0,
            busy: Duration::ZERO,
            arrivals: Vec::new(),
            latency_us: Vec::new(),
            repairs: Vec::new(),
            phases: BTreeMap::new(),
            capsules: BTreeMap::new(),
            capsule_log: Vec::new(),
            drops_asked: 0,
            halted: None,
            conn_events: Vec::new(),
            market: None,
            sym_phase: BTreeMap::new(),
            market_at_us: now_us(),
            market_dirty: true,
        })
    }

    /// (RP4/T4) From now on "now" is the simulated clock, starting at `t0_us` (UTC µs) and
    /// moved only by [`LiveIn::Clock`]: bars close, the market status and the lag follow
    /// it (module docs).
    pub fn use_sim_clock(&mut self, t0_us: i64) {
        self.trade_fill_sync = true;
        self.sim_now_us = Some(t0_us);
        self.lag_rot_us = t0_us;
        self.last_advance_us = t0_us;
        self.market_at_us = t0_us;
        self.market_dirty = true;
    }

    /// (RP4/T4) Whether the runtime runs on the simulated clock.
    #[must_use]
    pub fn is_simulated(&self) -> bool {
        self.sim_now_us.is_some()
    }

    /// (RP4) Where trade gaps are filled from (REST in live use, the local cache in the
    /// simulation).
    pub fn set_trade_history(&mut self, h: Arc<dyn TradeHistory + Send + Sync>) {
        self.trade_history = Some(h);
    }

    /// (RP4) Starts watching every feed's symbol's trades from `from_ms` (a gap until the
    /// trade connection opens and history fills it).
    pub fn watch_trades(&mut self, from_ms: i64) {
        let mut symbols: Vec<String> = self.feeds.iter().map(|k| k.symbol.clone()).collect();
        symbols.dedup();
        self.feed.watch_trades(&symbols, from_ms);
    }

    /// (RP4, L-prep) Starts watching every feed's symbol's trades from its seam (the end
    /// of its last initial bar: what history already holds); a symbol without one from
    /// `fallback_ms`.
    pub fn watch_trades_after_seams(&mut self, fallback_ms: i64) {
        let mut symbols: Vec<String> = self.feeds.iter().map(|k| k.symbol.clone()).collect();
        symbols.dedup();
        self.feed.watch_trades_after_seams(&symbols, fallback_ms);
    }

    /// (RP4, L-prep) Whether trade gaps are filled on the hub thread (default: only on the
    /// simulated clock).
    #[cfg(test)]
    pub fn set_trade_fill_sync(&mut self, sync: bool) {
        self.trade_fill_sync = sync;
    }

    /// (RP4, L-prep) Whether bars come from the trade stream.
    #[cfg(test)]
    #[must_use]
    pub fn trade_mode(&self) -> bool {
        self.trade_mode
    }

    /// (RP4, L-prep) Whether the market connection is up (`None` before it opened once).
    #[cfg(test)]
    #[must_use]
    pub fn stats_market_connected(&self) -> Option<bool> {
        self.conn_events
            .iter()
            .any(|e| e.contains("market opened"))
            .then_some(self.market_up)
    }

    /// (RP4, L-prep) Trade history fills done (counts).
    #[cfg(test)]
    #[must_use]
    pub fn trade_fill_count(&self) -> usize {
        self.trade_fills.len()
    }

    /// (RP4/T4) The market status last given to the authority.
    #[must_use]
    pub fn market(&self) -> Option<&qp_schema::market::MarketStatus> {
        self.market.as_ref()
    }

    /// The runtime's "now" (UTC µs): the simulated clock or the wall clock.
    fn now(&self) -> i64 {
        self.sim_now_us.unwrap_or_else(now_us)
    }

    /// The link state's name (`connecting`, `live`, `stale`, `repairing`, `halted`).
    #[must_use]
    pub fn link(&self) -> &'static str {
        self.link.name()
    }

    /// True while the feeds are stale (lost, or repairing).
    #[must_use]
    pub fn is_stale(&self) -> bool {
        matches!(self.link, Link::Stale | Link::Repairing | Link::Halted)
    }

    /// The exchange time now (wall clock minus the arrival lag).
    #[must_use]
    pub fn exchange_now(&self) -> i64 {
        self.now() / 1000 - self.lag_ms
    }

    fn note_lag(&mut self, arrival_us: i64, exch_ms: i64) {
        let lag = arrival_us / 1000 - exch_ms;
        self.lag_cur = Some(self.lag_cur.map_or(lag, |c| c.min(lag)));
        if self.now() - self.lag_rot_us >= 60_000_000 {
            self.lag_prev = self.lag_cur.take();
            self.lag_rot_us = self.now();
        }
        let best = match (self.lag_cur, self.lag_prev) {
            (Some(a), Some(b)) => a.min(b),
            (Some(a), None) | (None, Some(a)) => a,
            (None, None) => return,
        };
        self.lag_ms = best.max(0);
    }
}

/// (Tests) One live connection per endpoint key per process: the tests that open mock
/// connections take this first so that they do not hold the same key at once.
#[cfg(test)]
pub(crate) static MOCK_ENDPOINTS: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
mod tests;
