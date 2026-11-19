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
//! Nothing here prints a value, a secret or an endpoint address.

use std::collections::BTreeMap;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

use qp_calendar::{ExchangeCalendar, Interval};
use qp_live::{LiveConfig as ConnConfig, LiveConnection, LiveEvent, LiveMessage, LiveSink};
use qp_matriks::{RawRecord, RecordKind};
use qp_schema::data::{DataChange, DataEvent, FeedKey};
use qp_schema::protocol::OutboundLine;
use qp_schema::Bar;
use qp_session::ReplayHost;
use serde_json::{json, Value};

use crate::live::RawSink;

pub use qp_matriks::{ConnChange, FeedOut};

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

/// The REST period of an interval.
#[must_use]
pub fn rest_period(interval: &str) -> Option<&'static str> {
    Some(match interval {
        "1m" => "1min",
        "5m" => "5min",
        "10m" => "10min",
        "15m" => "15min",
        "30m" => "30min",
        "1h" | "60m" => "1hour",
        "2h" | "120m" => "120min",
        "4h" | "240m" => "240min",
        "1d" => "1day",
        _ => return None,
    })
}

fn local_date(cal: &ExchangeCalendar, t_ms: i64) -> String {
    let (y, m, d) = qp_calendar::civil_from_days(cal.local_day(t_ms));
    format!("{y:04}-{m:02}-{d:02}")
}

fn merge(into: &mut Bar, b: &Bar) {
    into.h = into.h.max(b.h);
    into.l = into.l.min(b.l);
    into.c = b.c;
    into.v += b.v;
}

/// REST bars (`time` = bar start; daily: local midnight) → calendar bars (K14): `t` is the
/// span open; a bar before the day's first session (opening auction) joins the day's first
/// bar, one after the last close or in a break (closing session) joins the bar before
/// (`docs/DATA_ACCESS.md`). Bars on no trading day are dropped.
#[must_use]
pub fn normalize_rest(rows: &[(i64, Bar)], interval: Interval, cal: &ExchangeCalendar) -> Vec<Bar> {
    let mut out: Vec<Bar> = Vec::new();
    let mut pre: Option<Bar> = None;
    for (time, b) in rows {
        let span = cal.bar_at(interval, *time).or_else(|| {
            (interval == Interval::Day)
                .then(|| cal.bar_at(interval, time + 12 * 3_600_000))
                .flatten()
        });
        match span {
            Some(s) => {
                let mut bar = Bar {
                    t: s.open,
                    ..b.clone()
                };
                if let Some(p) = pre.take() {
                    if cal.local_day(p.t) == cal.local_day(s.open) {
                        bar.o = p.o;
                        bar.h = bar.h.max(p.h);
                        bar.l = bar.l.min(p.l);
                        bar.v += p.v;
                    }
                }
                match out.last_mut() {
                    Some(last) if last.t == bar.t => merge(last, &bar),
                    Some(last) if last.t > bar.t => {}
                    _ => out.push(bar),
                }
            }
            None => {
                let day = cal.local_day(*time);
                let sessions = cal.sessions_on(day);
                match sessions.first() {
                    Some(first) if *time < first.open => match &mut pre {
                        Some(p) => merge(p, b),
                        None => pre = Some(b.clone()),
                    },
                    Some(_) => {
                        if let Some(last) = out.last_mut() {
                            if cal.local_day(last.t) == day {
                                merge(last, b);
                            }
                        }
                    }
                    None => {}
                }
            }
        }
    }
    out
}

/// The REST bar source (service token; sequential, paced by the client).
pub struct RestSource {
    pub cfg: ConnConfig,
    pub auth: Arc<qp_live::AuthClient>,
    pub cal: ExchangeCalendar,
}

impl BarSource for RestSource {
    fn fetch(
        &self,
        symbols: &[String],
        interval: &str,
        from_ms: i64,
        to_ms: i64,
    ) -> Vec<(String, Result<Vec<Bar>, String>)> {
        let Some(period) = rest_period(interval) else {
            return symbols
                .iter()
                .map(|s| (s.clone(), Err(format!("no REST period for `{interval}`"))))
                .collect();
        };
        let Ok(iv) = Interval::parse(interval) else {
            return Vec::new();
        };
        let rest = qp_live::rest::RestClient::new(&self.auth, self.cfg.redactor().clone());
        let (start, end) = (local_date(&self.cal, from_ms), local_date(&self.cal, to_ms));
        symbols
            .iter()
            .map(|s| {
                let r = rest
                    .bars(s, period, &start, &end, false)
                    .map(|rows| {
                        let rows: Vec<(i64, Bar)> = rows
                            .iter()
                            .map(|x| {
                                (
                                    x.time,
                                    Bar {
                                        t: x.time,
                                        o: x.open,
                                        h: x.high,
                                        l: x.low,
                                        c: x.close,
                                        v: x.volume,
                                    },
                                )
                            })
                            .collect();
                        normalize_rest(&rows, iv, &self.cal)
                    })
                    .map_err(|e| e.to_string());
                (s.clone(), r)
            })
            .collect()
    }
}

/// Bars complete by exchange time `cutoff` (their span closed).
fn complete_by(bars: Vec<Bar>, iv: Interval, cal: &ExchangeCalendar, cutoff: i64) -> Vec<Bar> {
    bars.into_iter()
        .filter(|b| cal.bar_at(iv, b.t).is_some_and(|s| s.close <= cutoff))
        .collect()
}

/// Symbols with their bars, and the symbols left out (with the reason).
pub type History = (Vec<(String, Vec<Bar>)>, Vec<String>);

/// The initial history: the last `n` bars of each symbol complete by `cutoff` (exchange
/// time: in delayed mode, now minus the delay — REST is not delayed). Symbols without a
/// bar are left out (named in the second list).
pub fn history(
    src: &dyn BarSource,
    symbols: &[String],
    interval: &str,
    n: usize,
    cutoff: i64,
    cal: &ExchangeCalendar,
) -> Result<History, String> {
    let iv = Interval::parse(interval).map_err(|e| format!("--interval: {e}"))?;
    // Calendar days to cover `n` bars (about 8 trading hours a day, weekends, holidays).
    let per_day = match iv {
        Interval::Minutes(m) => (480 / m.max(1)).max(1) as usize,
        _ => 1,
    };
    let days = i64::try_from(n.div_ceil(per_day)).unwrap_or(365) * 7 / 5 + 5;
    let from = cutoff - days * 86_400_000;
    let mut ok = Vec::new();
    let mut missing = Vec::new();
    for (s, r) in src.fetch(symbols, interval, from, cutoff) {
        match r {
            Ok(b) => {
                let mut b = complete_by(b, iv, cal, cutoff);
                let cut = b.len().saturating_sub(n);
                b.drain(..cut);
                if b.is_empty() {
                    missing.push(s);
                } else {
                    ok.push((s, b));
                }
            }
            Err(e) => missing.push(format!("{s} ({e})")),
        }
    }
    Ok((ok, missing))
}

/// The repair events of one feed: the store's bars against the settled bars (complete by
/// the cutoff, from the gap's start): a differing bar → `patch` (only the changed fields),
/// a later bar → `bar`. Numbered from `seq + 1`. Returns (events, patched, added).
#[must_use]
pub fn repair_events(
    feed: &FeedKey,
    store: &[Bar],
    seq: u64,
    settled: &[Bar],
) -> (Vec<DataEvent>, usize, usize) {
    let mut out = Vec::new();
    let (mut patched, mut added) = (0, 0);
    let mut next = seq + 1;
    let mut last_t = store.last().map_or(i64::MIN, |b| b.t);
    for b in settled {
        let change = if b.t > last_t {
            last_t = b.t;
            added += 1;
            DataChange::Bar { bar: b.clone() }
        } else {
            let Ok(i) = store.binary_search_by_key(&b.t, |x| x.t) else {
                continue;
            };
            let fields = qp_source::bar_patch_diff(&store[i], b);
            if fields.is_empty() {
                continue;
            }
            patched += 1;
            DataChange::Patch { t: b.t, fields }
        };
        out.push(DataEvent {
            feed: feed.clone(),
            seq: next,
            change,
        });
        next += 1;
    }
    (out, patched, added)
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
    buffer: Vec<RawRecord>,
    /// Exchange clock: the latest exchange time seen, the arrival lag estimate.
    clock: i64,
    lag_ms: i64,
    lag_cur: Option<i64>,
    lag_prev: Option<i64>,
    lag_rot: Instant,
    last_advance: Instant,
    last_print: Instant,
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
    market_at: Instant,
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
        let today = cal.local_day(now_us() / 1000 - delay_ms);
        let mut seeds = Vec::new();
        for k in &feeds {
            let (bars, seq) = host.live_feed(k).ok_or("live: feed without store")?;
            // The day's traded value so far, when history reaches into today (REST
            // `volume` is TL, like the card's `dailyVolume`).
            let cum = bars
                .last()
                .filter(|b| cal.local_day(b.t) == today && iv != Interval::Day)
                .map(|_| {
                    bars.iter()
                        .filter(|b| cal.local_day(b.t) == today)
                        .map(|b| b.v)
                        .sum::<f64>()
                });
            seeds.push((k.symbol.clone(), bars.last().cloned(), seq + 1, cum));
        }
        feed.seed_all(&seeds)?;
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
            lag_rot: Instant::now(),
            last_advance: Instant::now(),
            last_print: Instant::now(),
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
            market_at: Instant::now(),
            market_dirty: true,
        })
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
        now_us() / 1000 - self.lag_ms
    }

    fn note_lag(&mut self, arrival_us: i64, exch_ms: i64) {
        let lag = arrival_us / 1000 - exch_ms;
        self.lag_cur = Some(self.lag_cur.map_or(lag, |c| c.min(lag)));
        if self.lag_rot.elapsed() >= Duration::from_secs(60) {
            self.lag_prev = self.lag_cur.take();
            self.lag_rot = Instant::now();
        }
        let best = match (self.lag_cur, self.lag_prev) {
            (Some(a), Some(b)) => a.min(b),
            (Some(a), None) | (None, Some(a)) => a,
            (None, None) => return,
        };
        self.lag_ms = best.max(0);
    }

    /// One input from the connection thread or a repair thread.
    pub fn on_in(&mut self, host: &mut ReplayHost, input: LiveIn) -> Vec<OutboundLine> {
        let t0 = Instant::now();
        let lines = match input {
            LiveIn::Record(rec) => self.on_record(host, rec),
            LiveIn::Repaired {
                generation,
                bars,
                fetch_ms,
            } => self.on_repaired(host, generation, &bars, fetch_ms),
            LiveIn::Halted(r) => {
                self.conn_events.push(format!("halted: {r}"));
                self.halted = Some(r);
                let mut lines = Vec::new();
                if !self.is_stale() {
                    lines = self.go_stale(host);
                }
                self.link = Link::Halted;
                lines
            }
        };
        self.busy += t0.elapsed();
        lines
    }

    fn on_record(&mut self, host: &mut ReplayHost, rec: RawRecord) -> Vec<OutboundLine> {
        if rec.kind == RecordKind::Message {
            self.messages += 1;
            if self.link == Link::Repairing {
                self.buffer.push(rec);
                return Vec::new();
            }
        }
        let arrival = rec.t_us;
        let out = self.feed.push(&rec);
        self.apply(host, out, Some(arrival))
    }

    fn go_stale(&mut self, host: &mut ReplayHost) -> Vec<OutboundLine> {
        self.market_dirty = true;
        self.generation += 1;
        self.link = Link::Stale;
        self.stale_since = Some(Instant::now());
        self.stale_from_t = Some(self.clock);
        self.buffer.clear();
        self.conn_events
            .push(format!("lost at {}", crate::live::utc_stamp(now_us())));
        host.live_mark_stale()
    }

    fn apply(
        &mut self,
        host: &mut ReplayHost,
        out: FeedOut,
        arrival_us: Option<i64>,
    ) -> Vec<OutboundLine> {
        let mut lines = Vec::new();
        if let Some(c) = out.connection {
            match c {
                ConnChange::Lost => {
                    if matches!(self.link, Link::Live | Link::Connecting | Link::Repairing) {
                        lines.extend(self.go_stale(host));
                    }
                }
                ConnChange::Opened { session } => {
                    self.conn_events.push(format!(
                        "opened #{session} at {}",
                        crate::live::utc_stamp(now_us())
                    ));
                    match self.link {
                        Link::Connecting => {
                            self.link = Link::Live;
                            self.market_dirty = true;
                        }
                        Link::Stale => self.start_repair(),
                        _ => {}
                    }
                }
            }
        }
        if let (Some(c), Some(a)) = (out.clock_ms, arrival_us) {
            self.note_lag(a, c);
        }
        if let Some(c) = out.clock_ms {
            if c > self.clock && !self.is_stale() {
                self.clock = c;
                match host.session_mut().host_clock(c) {
                    Ok(l) => lines.extend(l),
                    Err(e) => self.conn_events.push(format!("clock: {}", e.message)),
                }
            }
        }
        for (sym, code) in &out.phases {
            *self.phases.entry(*code).or_default() += 1;
            self.sym_phase.insert(sym.clone(), *code);
            self.market_dirty = true;
        }
        self.finalized += out.finalized.len() as u64;
        if !out.events.is_empty() {
            if self.is_stale() {
                self.dropped_while_stale += out.events.len() as u64;
            } else {
                for ev in &out.events {
                    match host.live_event(ev) {
                        Ok(l) => {
                            self.events += 1;
                            lines.extend(l);
                        }
                        Err(_) => self.rejected += 1,
                    }
                }
                if let Some(a) = arrival_us {
                    if self.arrivals.len() < MAX_SAMPLES {
                        self.arrivals.push(a);
                    }
                }
            }
        }
        lines
    }

    fn start_repair(&mut self) {
        self.link = Link::Repairing;
        self.repair_started = Some(Instant::now());
        let generation = self.generation;
        let src = self.source.clone();
        let to_hub = self.to_hub.clone();
        let symbols: Vec<String> = self.feeds.iter().map(|k| k.symbol.clone()).collect();
        let interval = self.interval.clone();
        // From the stale point's bar (that bar may have been live) to now.
        let from = self.stale_from_t.unwrap_or(self.clock) - 86_400_000;
        let to = self.exchange_now();
        std::thread::spawn(move || {
            let t0 = Instant::now();
            let bars = src.fetch(&symbols, &interval, from, to);
            to_hub(LiveIn::Repaired {
                generation,
                bars,
                fetch_ms: u64::try_from(t0.elapsed().as_millis()).unwrap_or(u64::MAX),
            });
        });
    }

    fn on_repaired(
        &mut self,
        host: &mut ReplayHost,
        generation: u64,
        bars: &[(String, Result<Vec<Bar>, String>)],
        fetch_ms: u64,
    ) -> Vec<OutboundLine> {
        if generation != self.generation || self.link != Link::Repairing {
            return Vec::new();
        }
        let cutoff = self.exchange_now();
        let (mut patched, mut added, mut failed) = (0, 0, 0);
        let by_symbol: BTreeMap<&str, &Result<Vec<Bar>, String>> =
            bars.iter().map(|(s, r)| (s.as_str(), r)).collect();
        for k in self.feeds.clone() {
            let Some((store, seq)) = host.live_feed(&k) else {
                continue;
            };
            match by_symbol.get(k.symbol.as_str()) {
                Some(Ok(settled)) => {
                    let settled = complete_by(settled.clone(), self.iv, &self.cal, cutoff);
                    let (evs, p, a) = repair_events(&k, &store, seq, &settled);
                    patched += p;
                    added += a;
                    for ev in &evs {
                        if host.live_event(ev).is_err() {
                            self.rejected += 1;
                        }
                    }
                }
                _ => failed += 1,
            }
        }
        let mut lines = host.live_refresh();
        let seeds: Vec<Seed> = self
            .feeds
            .iter()
            .filter_map(|k| {
                let (store, seq) = host.live_feed(k)?;
                Some((k.symbol.clone(), store.last().cloned(), seq + 1, None))
            })
            .collect();
        if let Err(e) = self.feed.seed_all(&seeds) {
            self.conn_events.push(format!("seed: {e}"));
        }
        self.link = Link::Live;
        self.market_dirty = true;
        let buffered = std::mem::take(&mut self.buffer);
        let n_buf = buffered.len();
        for rec in buffered {
            let a = rec.t_us;
            let out = self.feed.push(&rec);
            lines.extend(self.apply(host, out, Some(a)));
        }
        let ms = |d: Option<Instant>| {
            d.map_or(0, |i| {
                u64::try_from(i.elapsed().as_millis()).unwrap_or(u64::MAX)
            })
        };
        self.repairs.push(RepairStat {
            stale_ms: ms(self.stale_since),
            fetch_ms,
            total_ms: ms(self.repair_started),
            feeds: self.feeds.len(),
            patched,
            added,
            failed,
            buffered: n_buf,
        });
        self.stale_since = None;
        lines
    }

    /// (T3, §24) The market status now: exchange time (the feed's clock), open and
    /// holiday from the XIST calendar, delayed, stale, each symbol's phase.
    #[must_use]
    pub fn market_status(&self) -> qp_schema::market::MarketStatus {
        let t = if self.is_stale() || self.clock > 0 && self.link == Link::Live {
            self.clock
        } else {
            self.exchange_now()
        };
        let now = if self.is_stale() {
            self.clock
        } else {
            self.exchange_now().max(self.clock)
        };
        let day = self.cal.local_day(now);
        let trading = self.cal.is_trading_day(day);
        let open = trading
            && self
                .cal
                .sessions_on(day)
                .iter()
                .any(|s| s.open <= now && now < s.close);
        let phases = self
            .sym_phase
            .iter()
            .filter_map(|(s, c)| {
                let p = qp_matriks::session_phase(*c)?;
                Some((
                    s.clone(),
                    qp_schema::market::PhaseCode {
                        code: *c,
                        key: p.key.to_owned(),
                    },
                ))
            })
            .collect();
        qp_schema::market::MarketStatus {
            exchange_time: t.max(0),
            open: Some(open),
            holiday: Some(!trading),
            source: qp_schema::market::MarketSource::Calendar,
            delayed: self.delayed,
            stale: self.is_stale(),
            phases,
        }
    }

    /// Gives the authority the market status when it changed (phases, link) or every
    /// 10 s (exchange time, open).
    fn publish_market(&mut self, host: &mut ReplayHost) -> Vec<OutboundLine> {
        if !self.market_dirty && self.market_at.elapsed() < Duration::from_secs(10) {
            return Vec::new();
        }
        let m = self.market_status();
        self.market_dirty = false;
        self.market_at = Instant::now();
        if self.market.as_ref() == Some(&m) {
            return Vec::new();
        }
        self.market = Some(m.clone());
        host.set_market_status(m)
    }

    /// Periodic work (call often): the market status when due; closes bars by the
    /// exchange clock once a second.
    pub fn tick(&mut self, host: &mut ReplayHost) -> Vec<OutboundLine> {
        let mut lines = self.publish_market(host);
        if self.last_advance.elapsed() < Duration::from_secs(1) || self.link != Link::Live {
            return lines;
        }
        self.last_advance = Instant::now();
        if self.last_print.elapsed() >= Duration::from_secs(30) {
            self.last_print = Instant::now();
            println!(
                "live: {} · {} messages · {} events · {} capsule events",
                self.link(),
                self.messages,
                self.events,
                self.capsule_log.len()
            );
        }
        let t0 = Instant::now();
        let now = self.exchange_now();
        let out = self.feed.advance_clock(now);
        lines.extend(self.apply(host, out, None));
        self.busy += t0.elapsed();
        lines
    }

    /// After a broadcast: the latency samples of the events just sent (arrival → frame).
    pub fn after_broadcast(&mut self) {
        if self.arrivals.is_empty() {
            return;
        }
        let now = now_us();
        for a in self.arrivals.drain(..) {
            if self.latency_us.len() >= MAX_SAMPLES {
                self.latency_us.remove(0);
            }
            self.latency_us.push(now - a);
        }
    }

    /// Notes capsules born or changed since the last call (no values; the symbol from
    /// the capsule's first fact's series).
    pub fn watch_capsules(&mut self, host: &ReplayHost) {
        let doc = host.session().document();
        if doc.capsules.len() == self.capsules.len()
            && doc.capsules.iter().all(|c| {
                self.capsules
                    .get(&c.id)
                    .is_some_and(|(s, r)| *r == c.revision && *s == format!("{:?}", c.state))
            })
        {
            return;
        }
        let stale = self.is_stale();
        for c in &doc.capsules {
            let state = format!("{:?}", c.state).to_lowercase();
            let symbol = || {
                let f = doc.findings.iter().find(|f| f.id == c.finding)?;
                let fact = doc.facts.iter().find(|x| f.facts.first() == Some(&x.id))?;
                let s = doc.series.iter().find(|s| s.id == fact.series)?;
                Some(s.instrument.as_ref()?.symbol.clone())
            };
            let prev = self.capsules.get(&c.id).cloned();
            let st = format!("{:?}", c.state);
            match prev {
                None => {
                    self.capsule_log.push(CapsuleEvent {
                        kind: "born",
                        state,
                        t: host.session().clock(),
                        symbol: symbol().unwrap_or_default(),
                        stale,
                    });
                }
                Some((s, _)) if s != st => {
                    self.capsule_log.push(CapsuleEvent {
                        kind: "state",
                        state,
                        t: host.session().clock(),
                        symbol: symbol().unwrap_or_default(),
                        stale,
                    });
                }
                _ => {}
            }
            self.capsules.insert(c.id.clone(), (st, c.revision));
        }
    }

    /// Takes the capsule set at start (capsules from history are not "born live").
    pub fn baseline_capsules(&mut self, host: &ReplayHost) {
        for c in &host.session().document().capsules {
            self.capsules
                .insert(c.id.clone(), (format!("{:?}", c.state), c.revision));
        }
    }

    /// Capsules born or changed while live, and those while stale (must be 0).
    #[must_use]
    pub fn capsule_counts(&self) -> (usize, usize) {
        let stale = self.capsule_log.iter().filter(|e| e.stale).count();
        (self.capsule_log.len() - stale, stale)
    }

    /// Aggregate statistics (counts, times; no market values).
    #[must_use]
    pub fn stats(&self, host: &ReplayHost) -> Value {
        let secs = self.started.elapsed().as_secs_f64().max(1e-3);
        #[allow(clippy::cast_precision_loss)]
        let rate = self.messages as f64 / secs;
        let (rss_kb, cpu) = process_usage();
        let repairs: Vec<Value> = self
            .repairs
            .iter()
            .map(|r| {
                json!({"stale_ms": r.stale_ms, "fetch_ms": r.fetch_ms, "repair_ms": r.total_ms,
                       "feeds": r.feeds, "patched": r.patched, "added": r.added,
                       "failed": r.failed, "buffered": r.buffered})
            })
            .collect();
        let caps: Vec<Value> = self
            .capsule_log
            .iter()
            .rev()
            .take(200)
            .map(|e| {
                json!({"kind": e.kind, "state": e.state, "t": e.t, "symbol": e.symbol,
                       "while_stale": e.stale})
            })
            .collect();
        let (live_caps, stale_caps) = self.capsule_counts();
        json!({
            "link": self.link.name(),
            "delayed": self.delayed,
            "feeds": self.feeds.len(),
            "interval": self.interval,
            "uptime_s": secs.round(),
            "messages": self.messages,
            "messages_per_s": (rate * 10.0).round() / 10.0,
            "events": self.events,
            "finalized_bars": self.finalized,
            "events_dropped_while_stale": self.dropped_while_stale,
            "events_rejected": self.rejected,
            "hub_busy_ms": u64::try_from(self.busy.as_millis()).unwrap_or(u64::MAX),
            "hub_busy_pct": ((self.busy.as_secs_f64() / secs) * 1000.0).round() / 10.0,
            "process_rss_kb": rss_kb,
            "process_cpu": cpu,
            "tick_to_frame_ms": {
                "n": self.latency_us.len(),
                "p50": pct(&self.latency_us, 0.5) as f64 / 1000.0,
                "p95": pct(&self.latency_us, 0.95) as f64 / 1000.0,
                "max": self.latency_us.iter().max().copied().unwrap_or(0) as f64 / 1000.0,
            },
            "exchange_lag_ms": self.lag_ms,
            "bindings_not_ready": host.live_not_ready(),
            "feed": self.feed.stats_json(),
            "market": self.market.as_ref().map(|m| json!({"open": m.open, "holiday": m.holiday,
                "stale": m.stale, "delayed": m.delayed, "exchange_time": m.exchange_time,
                "phase_symbols": m.phases.len()})),
            "repairs": repairs,
            "drops_asked": self.drops_asked,
            "connection_events": self.conn_events,
            "halted": self.halted,
            "session_phases_seen": self.phases.iter().map(|(k, v)| (k.to_string(), *v)).collect::<BTreeMap<_, _>>(),
            "capsules": {"born_or_changed_live": live_caps, "born_or_changed_while_stale": stale_caps,
                          "recent": caps},
        })
    }
}

/// The process's resident memory (KiB) and CPU time (`ps`; `None` if unavailable).
fn process_usage() -> (Option<u64>, Option<String>) {
    let out = std::process::Command::new("ps")
        .args(["-o", "rss=,time=", "-p", &std::process::id().to_string()])
        .output();
    let Ok(o) = out else { return (None, None) };
    let s = String::from_utf8_lossy(&o.stdout);
    let mut it = s.split_whitespace();
    (
        it.next().and_then(|x| x.parse().ok()),
        it.next().map(str::to_owned),
    )
}

/// The connection's sink: the raw log (K60) and a copy to the hub.
pub struct HubSink {
    raw: RawSink,
    to_hub: Arc<dyn Fn(LiveIn) + Send + Sync>,
}

impl HubSink {
    /// A sink writing `raw` and sending to the hub.
    pub fn new(raw: RawSink, to_hub: Arc<dyn Fn(LiveIn) + Send + Sync>) -> Self {
        Self { raw, to_hub }
    }

    /// A local note (a deliberate drop) into the log and to the hub as a lost connection.
    pub fn local_drop(&self, note: &str) {
        let rec = RawRecord {
            t_us: now_us(),
            kind: RecordKind::ConnectionLost,
            topic: String::new(),
            payload: note.as_bytes().to_vec(),
        };
        self.raw.put_record(&rec);
        (self.to_hub)(LiveIn::Record(rec));
    }

    /// Flushes the raw log.
    pub fn flush(&self) {
        self.raw.flush();
    }
}

impl LiveSink for HubSink {
    fn on_message(&self, m: &LiveMessage<'_>) {
        self.raw.on_message(m);
        (self.to_hub)(LiveIn::Record(RawRecord::message(
            m.received_at_us,
            m.topic,
            m.payload,
        )));
    }

    fn on_event(&self, ep: &qp_live::EndpointKey, ev: &LiveEvent) {
        self.raw.on_event(ep, ev);
        let kind = match ev {
            LiveEvent::Connected { session } => {
                Some((RecordKind::ConnectionOpened, format!("session {session}")))
            }
            LiveEvent::Disconnected { .. } => {
                Some((RecordKind::ConnectionLost, "disconnected".to_owned()))
            }
            _ => None,
        };
        if let Some((kind, note)) = kind {
            (self.to_hub)(LiveIn::Record(RawRecord {
                t_us: now_us(),
                kind,
                topic: String::new(),
                payload: note.into_bytes(),
            }));
        }
    }
}

/// A command to the connection thread.
#[derive(Debug)]
pub enum ConnCtl {
    /// Close (DISCONNECT), wait this long, open again.
    Drop(Duration),
    /// Close and end the thread (answer when closed).
    Stop(Sender<()>),
}

/// What the connection thread needs to open the connection.
pub struct ConnSpec {
    pub cfg: ConnConfig,
    pub auth: Arc<qp_live::AuthClient>,
    pub topics: Vec<String>,
    pub lock_dir: std::path::PathBuf,
}

/// Starts the connection thread: opens the one connection, serves `drop` and `stop`.
/// Returns its control channel.
///
/// # Errors
/// The first open failed (lock held elsewhere, halt marker, …).
pub fn spawn_connection(
    spec: ConnSpec,
    sink: Arc<HubSink>,
    to_hub: Arc<dyn Fn(LiveIn) + Send + Sync>,
) -> Result<Sender<ConnCtl>, String> {
    let open = |spec: &ConnSpec, sink: &Arc<HubSink>| {
        let mut opts = qp_live::ConnectOptions::new(spec.topics.clone());
        opts.classifier = Some(crate::live::classifier());
        opts.lock_dir = spec.lock_dir.clone();
        LiveConnection::open(
            &spec.cfg,
            spec.cfg.market_endpoint(),
            spec.auth.clone(),
            opts,
            sink.clone() as Arc<dyn LiveSink>,
        )
        .map_err(|e| e.to_string())
    };
    let first = open(&spec, &sink)?;
    let (tx, rx): (Sender<ConnCtl>, Receiver<ConnCtl>) = mpsc::channel();
    std::thread::spawn(move || {
        let mut conn = Some(first);
        loop {
            match rx.recv_timeout(Duration::from_millis(500)) {
                Ok(ConnCtl::Drop(wait)) => {
                    let Some(c) = conn.take() else { continue };
                    sink.local_drop("local drop");
                    let _ = c.close();
                    // Interruptible wait: a stop during it ends the thread.
                    match rx.recv_timeout(wait) {
                        Ok(ConnCtl::Stop(done)) => {
                            sink.flush();
                            let _ = done.send(());
                            return;
                        }
                        Ok(ConnCtl::Drop(_)) | Err(RecvTimeoutError::Timeout) => {}
                        Err(RecvTimeoutError::Disconnected) => return,
                    }
                    match open(&spec, &sink) {
                        Ok(c) => conn = Some(c),
                        Err(e) => to_hub(LiveIn::Halted(format!("reopen failed: {e}"))),
                    }
                }
                Ok(ConnCtl::Stop(done)) => {
                    if let Some(c) = conn.take() {
                        let _ = c.close();
                    }
                    sink.flush();
                    let _ = done.send(());
                    return;
                }
                Err(RecvTimeoutError::Timeout) => {
                    if let Some(c) = &conn {
                        if let qp_live::ConnState::Halted(r) = c.state() {
                            to_hub(LiveIn::Halted(r.to_string()));
                            if let Some(c) = conn.take() {
                                let _ = c.close();
                            }
                        }
                    }
                }
                Err(RecvTimeoutError::Disconnected) => {
                    if let Some(c) = conn.take() {
                        let _ = c.close();
                    }
                    sink.flush();
                    return;
                }
            }
        }
    });
    Ok(tx)
}

// ---------------------------------------------------------------------------------------
// The feed: `qp_matriks::LiveFeed` (the one pipeline shared with raw-log replay).

/// [`FeedEngine`] over `qp_matriks::LiveFeed` on one interval.
pub struct MatriksFeed {
    cfg: qp_matriks::LiveFeedConfig,
    cal: ExchangeCalendar,
    interval: String,
    inner: qp_matriks::LiveFeed,
    /// Stats of the feeds replaced by a repair (records, events).
    pub retired: (u64, u64),
}

impl MatriksFeed {
    /// A feed building `interval` bars (other settings: `LiveFeedConfig` defaults —
    /// ticks, TL value, `dailyVolume`, `updateDate` time).
    #[must_use]
    pub fn new(cal: ExchangeCalendar, interval: &str) -> Self {
        let cfg = qp_matriks::LiveFeedConfig {
            intervals: vec![interval.to_owned()],
            ..Default::default()
        };
        Self {
            inner: qp_matriks::LiveFeed::new(cfg.clone(), cal.clone()),
            cfg,
            cal,
            interval: interval.to_owned(),
            retired: (0, 0),
        }
    }
}

impl FeedEngine for MatriksFeed {
    fn seed_all(&mut self, seeds: &[Seed]) -> Result<(), String> {
        let started = self.inner.feeds().any(|(_, lb)| lb.stats().ticks > 0);
        if started {
            // After a repair: a new feed seeded at the repaired bars (`LiveFeed` seeds
            // only before a feed's first tick); each day's counter goes on from the
            // card's last value; the next message per symbol is its initial (as after any
            // reconnect).
            let old = std::mem::replace(
                &mut self.inner,
                qp_matriks::LiveFeed::new(self.cfg.clone(), self.cal.clone()),
            );
            let st = old.stats();
            self.retired.0 += st.records;
            self.retired.1 += st.events;
            for (sym, last, seq, _) in seeds {
                let cum = old.book().state(sym).and_then(|c| c.daily_volume);
                self.inner
                    .seed_with_counter(sym, &self.interval, last.clone(), *seq, cum)?;
            }
            return Ok(());
        }
        for (sym, last, seq, cum) in seeds {
            self.inner
                .seed_with_counter(sym, &self.interval, last.clone(), *seq, *cum)?;
        }
        Ok(())
    }

    fn push(&mut self, rec: &RawRecord) -> FeedOut {
        self.inner.push(rec)
    }

    fn advance_clock(&mut self, now_ms: i64) -> FeedOut {
        self.inner.advance_clock(now_ms)
    }

    fn stats_json(&self) -> Value {
        let s = self.inner.stats();
        json!({
            "records": s.records + self.retired.0, "events": s.events + self.retired.1,
            "current": {
                "records": s.records, "symbol_messages": s.symbol_messages,
                "initials": s.initials, "decode_errors": s.decode_errors,
                "no_bar_fields": s.no_bar_fields, "time_direct": s.time_direct,
                "time_derived": s.time_derived, "time_unknown": s.time_unknown,
                "behind_clock": s.behind_clock, "max_behind_ms": s.max_behind_ms,
                "bad_update_date": s.bad_update_date,
                "clock_lead_rejected": s.clock_lead_rejected, "ticks": s.ticks,
                "connections_opened": s.connections_opened,
                "connections_lost": s.connections_lost, "resubscribes": s.resubscribes,
                "feeds": s.feeds, "finalized": s.finalized, "off_session": s.off_session,
                "late": s.late, "no_price": s.no_price, "invalid_fields": s.invalid_fields,
                "volume_regressions": s.volume_regressions,
                "volume_rebases": s.volume_rebases, "bars_opened": s.bars_opened,
                "events": s.events,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    //! VERIFY 10 (M23, no network): a simulated drop on a recorded-like stream — stale
    //! marking, reconnect, REST repair (a fake source), no capsule while stale, repaired
    //! bars equal to the settled ones.
    use super::*;
    use qp_schema::protocol::OutboundLine;

    fn cal() -> ExchangeCalendar {
        ExchangeCalendar::new(&qp_calendar::builtin("XIST").expect("XIST")).expect("cal")
    }

    /// `n` daily bars on XIST trading days from 2025-01-02 (a gentle walk).
    fn truth(n: usize, seed: f64) -> Vec<Bar> {
        let c = cal();
        let mut out = Vec::new();
        let mut day = qp_calendar::days_from_civil(2025, 1, 2);
        let mut p = 100.0 + seed;
        while out.len() < n {
            if let Some(s) = c.sessions_on(day).first() {
                #[allow(clippy::cast_precision_loss)]
                let k = out.len() as f64;
                let o = p;
                p += ((k * 0.7 + seed).sin()) * 2.0;
                out.push(Bar {
                    t: s.open,
                    o,
                    h: o.max(p) + 1.0,
                    l: o.min(p) - 1.0,
                    c: p,
                    v: 1000.0 + k,
                });
            }
            day += 1;
        }
        out
    }

    /// A fake feed: a message's payload is a JSON bar of its topic's symbol; a later `t`
    /// opens a bar, the same `t` patches it.
    #[derive(Default)]
    struct FakeFeed {
        last: BTreeMap<String, (Option<Bar>, u64)>,
    }

    impl FeedEngine for FakeFeed {
        fn seed_all(&mut self, seeds: &[Seed]) -> Result<(), String> {
            for (symbol, last, next_seq, _) in seeds {
                self.last.insert(symbol.clone(), (last.clone(), *next_seq));
            }
            Ok(())
        }
        fn push(&mut self, rec: &RawRecord) -> FeedOut {
            let mut out = FeedOut::default();
            match rec.kind {
                RecordKind::ConnectionOpened => {
                    out.connection = Some(ConnChange::Opened { session: 2 });
                }
                RecordKind::ConnectionLost => out.connection = Some(ConnChange::Lost),
                RecordKind::Message => {
                    let b: Bar = serde_json::from_slice(&rec.payload).expect("bar");
                    let Some((last, seq)) = self.last.get_mut(&rec.topic) else {
                        return out;
                    };
                    let feed = FeedKey {
                        symbol: rec.topic.clone(),
                        interval: "1d".to_owned(),
                        feed: None,
                    };
                    let change = match last {
                        Some(l) if l.t == b.t => {
                            let f = qp_source::bar_patch_diff(l, &b);
                            if f.is_empty() {
                                return out;
                            }
                            DataChange::Patch { t: b.t, fields: f }
                        }
                        Some(l) if l.t > b.t => return out,
                        _ => DataChange::Bar { bar: b.clone() },
                    };
                    out.clock_ms = Some(b.t);
                    out.events.push(DataEvent {
                        feed,
                        seq: *seq,
                        change,
                    });
                    *seq += 1;
                    *last = Some(b);
                }
                _ => {}
            }
            out
        }
        fn advance_clock(&mut self, _now_ms: i64) -> FeedOut {
            FeedOut::default()
        }
    }

    struct FakeSource(BTreeMap<String, Vec<Bar>>);

    impl BarSource for FakeSource {
        fn fetch(
            &self,
            symbols: &[String],
            _interval: &str,
            from_ms: i64,
            to_ms: i64,
        ) -> Vec<(String, Result<Vec<Bar>, String>)> {
            symbols
                .iter()
                .map(|s| {
                    let b = self.0.get(s).map(|b| {
                        b.iter()
                            .filter(|x| x.t >= from_ms && x.t <= to_ms)
                            .cloned()
                            .collect()
                    });
                    (s.clone(), b.ok_or_else(|| "no such symbol".to_owned()))
                })
                .collect()
        }
    }

    /// A message arriving a day after its bar opened (the exchange clock lags the wall
    /// clock by about a year; bars are closed by that clock).
    fn msg(sym: &str, b: &Bar) -> RawRecord {
        RawRecord::message(
            (b.t + 86_400_000) * 1000,
            sym,
            serde_json::to_vec(b).expect("json"),
        )
    }

    fn conn(kind: RecordKind) -> RawRecord {
        RawRecord {
            t_us: now_us(),
            kind,
            topic: String::new(),
            payload: b"session 1".to_vec(),
        }
    }

    fn live_host(hist: &BTreeMap<String, Vec<Bar>>) -> ReplayHost {
        let feeds = hist
            .iter()
            .map(|(s, b)| qp_source::ReplayFeed {
                key: FeedKey {
                    symbol: s.clone(),
                    interval: "1d".to_owned(),
                    feed: None,
                },
                bars: b.clone(),
                fine: Vec::new(),
            })
            .collect();
        let init = qp_session::demo_init(qp_schema::protocol::ReplayInit {
            interval: Some("1d".to_owned()),
            ..Default::default()
        });
        ReplayHost::live(init, std::path::Path::new("."), feeds).expect("live host")
    }

    #[test]
    #[allow(clippy::needless_range_loop)]
    fn drop_marks_stale_and_repair_matches_settled_bars() {
        let syms = ["AAA", "BBB"];
        let full: BTreeMap<String, Vec<Bar>> = syms
            .iter()
            .enumerate()
            .map(|(i, s)| {
                #[allow(clippy::cast_precision_loss)]
                let seed = i as f64 * 3.0;
                ((*s).to_owned(), truth(80, seed))
            })
            .collect();
        let hist: BTreeMap<String, Vec<Bar>> = full
            .iter()
            .map(|(s, b)| (s.clone(), b[..40].to_vec()))
            .collect();
        let mut host = live_host(&hist);
        let (tx, rx) = mpsc::channel::<LiveIn>();
        let tx = std::sync::Mutex::new(tx);
        let to_hub: Arc<dyn Fn(LiveIn) + Send + Sync> = Arc::new(move |i| {
            let _ = tx
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .send(i);
        });
        // The settled source differs from what live delivered on bar 44 (a correction).
        let mut settled = full.clone();
        settled.get_mut("AAA").expect("AAA")[44].c += 0.5;
        settled.get_mut("AAA").expect("AAA")[44].h += 0.5;
        let mut rt = LiveRuntime::new(
            &host,
            Box::new(FakeFeed::default()),
            Arc::new(FakeSource(settled.clone())),
            to_hub,
            cal(),
            "1d",
            0,
        )
        .expect("runtime");
        rt.baseline_capsules(&host);
        let mut lines: Vec<OutboundLine> = Vec::new();
        lines.extend(rt.on_in(
            &mut host,
            LiveIn::Record(conn(RecordKind::ConnectionOpened)),
        ));
        assert_eq!(rt.link(), "live");
        // Live: bars 40..46 arrive.
        for i in 40..46 {
            for s in syms {
                lines.extend(rt.on_in(&mut host, LiveIn::Record(msg(s, &full[s][i]))));
            }
        }
        lines.extend(host.live_flush());
        rt.watch_capsules(&host);
        assert_eq!(host.live_not_ready(), 0);
        assert_eq!(rt.events, 12);
        // Drop.
        lines.extend(rt.on_in(&mut host, LiveIn::Record(conn(RecordKind::ConnectionLost))));
        assert_eq!(rt.link(), "stale");
        assert_eq!(host.live_not_ready(), 2, "every live feed is stale");
        lines.extend(rt.tick(&mut host));
        let m = host.session().market_for("arkin").expect("market status");
        assert!(m.stale, "the market status says stale");
        // A late message while stale is not applied.
        lines.extend(rt.on_in(&mut host, LiveIn::Record(msg("AAA", &full["AAA"][46]))));
        assert_eq!(rt.dropped_while_stale, 1);
        rt.watch_capsules(&host);
        // Reconnect: repair starts; messages wait.
        lines.extend(rt.on_in(
            &mut host,
            LiveIn::Record(conn(RecordKind::ConnectionOpened)),
        ));
        assert_eq!(rt.link(), "repairing");
        for s in syms {
            lines.extend(rt.on_in(&mut host, LiveIn::Record(msg(s, &full[s][70]))));
        }
        let repaired = rx.recv_timeout(Duration::from_secs(10)).expect("repair");
        assert!(matches!(repaired, LiveIn::Repaired { .. }));
        lines.extend(rt.on_in(&mut host, repaired));
        lines.extend(host.live_flush());
        rt.watch_capsules(&host);
        assert_eq!(rt.link(), "live");
        assert_eq!(host.live_not_ready(), 0, "resolved again");
        lines.extend(rt.tick(&mut host));
        assert!(!host.session().market_for("arkin").expect("market").stale);
        // The repaired series: the settled bars (all complete by the exchange clock); the
        // buffered message for an already settled bar changes nothing.
        for s in syms {
            let k = FeedKey {
                symbol: s.to_owned(),
                interval: "1d".to_owned(),
                feed: None,
            };
            let (bars, _) = host.live_feed(&k).expect("feed");
            let want: Vec<Bar> = settled[s].clone();
            assert_eq!(bars, want, "{s}: repaired bars equal the settled bars");
        }
        // No capsule was born or changed while stale.
        assert_eq!(rt.capsule_counts().1, 0);
        let st = rt.stats(&host);
        assert_eq!(st["repairs"].as_array().map(Vec::len), Some(1));
        assert_eq!(st["repairs"][0]["patched"], 1);
        // Devices: the recorded events replay to the same bars (no gap: the stale mark
        // was never recorded).
        let (f0, evs) = host
            .feed_data(&FeedKey {
                symbol: "AAA".to_owned(),
                interval: "1d".to_owned(),
                feed: None,
            })
            .expect("feed data");
        let mut store =
            qp_source::FeedStore::with_bars(f0.feed.clone(), f0.bars.clone().expect("bars"), 0)
                .expect("store");
        for e in &evs {
            let a = store.apply(e).expect("apply");
            assert!(
                !matches!(
                    a,
                    qp_source::Applied::Gap { .. } | qp_source::Applied::Stale(_)
                ),
                "device sees no gap"
            );
        }
        assert_eq!(store.bars(), &settled["AAA"][..]);
        let _ = lines;
    }

    /// The connection path end to end on the loopback mock broker and token service: the
    /// sink feeds the runtime, a local drop closes the one connection (DISCONNECT) before
    /// opening again (never two at once), the feeds go stale and are repaired.
    #[test]
    fn local_drop_on_mock_broker_keeps_one_connection_and_repairs() {
        use qp_live::mock::{mock_config, BrokerScript, MockBroker, MockTokenService};
        let dir = tempfile::tempdir().expect("tmp");
        let tokens = MockTokenService::start_default(3600);
        let broker = MockBroker::start(BrokerScript::default());
        let cfg = mock_config(&tokens, &broker.url("market")).expect("cfg");
        let full: BTreeMap<String, Vec<Bar>> = [
            ("AAA".to_owned(), truth(60, 0.0)),
            ("BBB".to_owned(), truth(60, 2.0)),
        ]
        .into_iter()
        .collect();
        let hist: BTreeMap<String, Vec<Bar>> = full
            .iter()
            .map(|(s, b)| (s.clone(), b[..30].to_vec()))
            .collect();
        let mut host = live_host(&hist);
        let (tx, rx) = mpsc::channel::<LiveIn>();
        let tx = std::sync::Mutex::new(tx);
        let to_hub: Arc<dyn Fn(LiveIn) + Send + Sync> = Arc::new(move |i| {
            let _ = tx
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .send(i);
        });
        let mut rt = LiveRuntime::new(
            &host,
            Box::new(FakeFeed::default()),
            Arc::new(FakeSource(full.clone())),
            to_hub.clone(),
            cal(),
            "1d",
            0,
        )
        .expect("runtime");
        let sink = Arc::new(HubSink::new(
            RawSink::create(&dir.path().join("raw.qpraw")).expect("raw"),
            to_hub.clone(),
        ));
        let auth = Arc::new(qp_live::AuthClient::new(
            &cfg,
            qp_live::AuthPolicy::default(),
        ));
        let spec = ConnSpec {
            cfg,
            auth,
            topics: vec!["AAA".to_owned(), "BBB".to_owned()],
            lock_dir: dir.path().to_owned(),
        };
        let ctl = spawn_connection(spec, sink, to_hub).expect("open");
        let pump =
            |rt: &mut LiveRuntime, host: &mut ReplayHost, until: &dyn Fn(&LiveRuntime) -> bool| {
                let deadline = Instant::now() + Duration::from_secs(20);
                while Instant::now() < deadline {
                    if let Ok(i) = rx.recv_timeout(Duration::from_millis(50)) {
                        let _ = rt.on_in(host, i);
                    }
                    if until(rt) {
                        return true;
                    }
                }
                false
            };
        assert!(
            pump(&mut rt, &mut host, &|r| r.link() == "live"),
            "connected"
        );
        assert!(
            broker.wait_until(Duration::from_secs(5), |s| s.subscribe_packets.len() >= 2
                || s.subscribe_packets.iter().map(Vec::len).sum::<usize>() >= 2)
        );
        #[allow(clippy::needless_range_loop)]
        for i in 30..35 {
            for s in ["AAA", "BBB"] {
                broker.publish(s, &serde_json::to_vec(&full[s][i]).expect("json"));
            }
        }
        assert!(pump(&mut rt, &mut host, &|r| r.events >= 10), "live events");
        ctl.send(ConnCtl::Drop(Duration::from_millis(300)))
            .expect("drop");
        assert!(
            pump(&mut rt, &mut host, &|r| r.is_stale()),
            "stale after drop"
        );
        assert_eq!(host.live_not_ready(), 2);
        assert!(
            pump(&mut rt, &mut host, &|r| r.link() == "live"
                && !r.repairs.is_empty()),
            "reconnected and repaired"
        );
        assert_eq!(host.live_not_ready(), 0);
        let st = broker.stats();
        assert_eq!(st.max_concurrent, 1, "never two connections at once");
        assert!(st.disconnects >= 1, "the drop sent DISCONNECT");
        for s in ["AAA", "BBB"] {
            let k = FeedKey {
                symbol: s.to_owned(),
                interval: "1d".to_owned(),
                feed: None,
            };
            // Bars here open in 2025 and arrive now: the exchange clock (arrival minus
            // the observed lag) closes only the bars up to the last one seen, so the
            // repair stops there; what it wrote equals the settled bars.
            let bars = host.live_feed(&k).expect("feed").0;
            assert!(bars.len() >= 35, "{s}: live bars kept");
            assert_eq!(bars[..], full[s][..bars.len()], "{s} repaired");
        }
        assert_eq!(rt.capsule_counts().1, 0);
        let (done_tx, done_rx) = mpsc::channel();
        ctl.send(ConnCtl::Stop(done_tx)).expect("stop");
        done_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("closed");
        assert!(broker.wait_until(Duration::from_secs(5), |s| s.current == 0));
    }

    #[test]
    fn rest_bars_are_aligned_and_closing_session_merged() {
        let c = cal();
        let day = qp_calendar::days_from_civil(2025, 3, 4);
        let s = c.sessions_on(day);
        let open = s[0].open;
        let close = s.last().expect("session").close;
        let b = |t: i64, p: f64| {
            (
                t,
                Bar {
                    t,
                    o: p,
                    h: p + 1.0,
                    l: p - 1.0,
                    c: p,
                    v: 10.0,
                },
            )
        };
        let rows = vec![
            b(open - 5 * 60_000, 9.0), // opening auction
            b(open, 10.0),
            b(open + 5 * 60_000, 11.0),
            b(close, 12.0), // closing session
        ];
        let out = normalize_rest(&rows, Interval::Minutes(5), &c);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].t, open);
        assert!((out[0].o - 9.0).abs() < 1e-12 && (out[0].v - 20.0).abs() < 1e-12);
        assert!((out[1].c - 12.0).abs() < 1e-12 && (out[1].v - 20.0).abs() < 1e-12);
        // Daily: local midnight → the session open.
        let mid = open - 10 * 3_600_000;
        let d = normalize_rest(&[b(mid, 5.0)], Interval::Day, &c);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].t, c.bar_at(Interval::Day, open).expect("day").open);
    }
}
