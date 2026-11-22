//! `qp sim` and `qp demo --sim` (RP4/T4, K65): the accelerated simulation.
//!
//! Cached historical trades (`fixtures/local/trades/<SYM>/<start>_<end>.csv.gz`, level 1,
//! exchange time) are put into exchange order and sent through **the live path itself**:
//! each trade becomes the wire message the trade endpoint would send (`TradeMessage` on
//! `mx/trade/<SYM>`) inside a [`RawRecord`] whose arrival is the trade's own time, and goes
//! in as [`LiveIn::Trade`] to the same [`LiveRuntime`] (`qp_matriks::LiveFeed` with trades →
//! the authority [`ReplayHost::live`] → the capsule engine) that `qp demo --live` runs. No
//! separate pipeline: the only simulation-specific parts are where the records come from,
//! the clock ([`LiveRuntime::use_sim_clock`], moved by [`LiveIn::Clock`]) and the pace.
//!
//! - **History (first resolution, warm-up).** The last `--history` settled bars before the
//!   start, per symbol from the cached REST bars (`fixtures/local/live/rest/
//!   <SYM>.<period>.<start>.<end>.unadj.json`, normalised like the live REST source), or —
//!   when the cache has fewer bars there — built from the cached trades of the days
//!   before the start with the same bar builder (`BarMethod::Trades`). Which source each
//!   symbol took is counted.
//! - **Trade connection.** At the start the trade endpoint "opens"; the gap from the start
//!   (local midnight of `--start`) is filled from the cache ([`CacheHistory`], the
//!   simulation's [`TradeHistory`]; live uses REST), as on a live connection.
//! - **Clock.** Exchange time. The simulation moves the runtime's clock to every trade's
//!   time and to every bar close, session open and close and day end of the XIST calendar
//!   ([`schedule`]), so bars close without trades too and days follow each other
//!   (close → market closed → next trading day's open; holidays and weekends skipped).
//! - **Session phase.** There is no price card in the simulation, so no per-symbol
//!   `stockStatus`: the market status (open / closed / holiday) comes from the XIST
//!   calendar ([`LiveRuntime::market_status`], `source: calendar`), as live does when the
//!   card is silent.
//! - **Pace ([`Speed`], [`Pacer`]).** `N`: N× real time inside sessions; closed hours
//!   (night, weekend, before the open) take at most [`CLOSED_PAUSE`] of real time each.
//!   `max`: no waiting. The pace never changes the inputs, only when they are sent: the
//!   same cache and arguments give a byte-equal event stream at any speed (VERIFY 7).
//!
//! Real data stays on this machine: `--out` only under `fixtures/local/`, standard output
//! gets counts and a digest only; `qp demo --sim` listens on 127.0.0.1 only.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use prost::Message as _;
use qp_calendar::{ExchangeCalendar, Interval};
use qp_matriks::messages::TradeMessage;
use qp_matriks::{RawRecord, RecordKind, Trade, TradeDecodeConfig, TradeHistory, TradeTimeUnit};
use qp_schema::data::{DataChange, FeedKey};
use qp_schema::Bar;
use qp_session::ReplayHost;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::livehost::{BarSource, LiveIn, LiveRuntime, MatriksFeed};

/// Real time a closed stretch (night, weekend, pre-open) takes at most at speed `N`.
pub(crate) const CLOSED_PAUSE: Duration = Duration::from_secs(2);

/// Named capsule constant records that `--rule` accepts (`docs/CAPSULE.md` §9.9):
/// `qp_capsule::ENGINES` (`r3`: `EngineConfig::r3` and the R3 detectors, the protocol
/// default; `rp4`: the RP4 calibration's outcome rule and detector set). Old records stay
/// selectable by name; the choice goes to the authority as `init.capsules.engine`.
pub(crate) const RULES: &[&str] = qp_capsule::ENGINES;

/// The session init's `capsules.engine` for `--rule`: `None` for the protocol default
/// (`r3`: nothing written, so the run is byte-identical to one without `--rule`).
#[must_use]
pub(crate) fn engine_field(rule: &str) -> Option<String> {
    (rule != qp_capsule::DEFAULT_ENGINE).then(|| rule.to_owned())
}

/// Checks `--rule` against [`RULES`].
///
/// # Errors
/// An unknown rule name.
pub(crate) fn check_rule(name: &str) -> Result<(), String> {
    if RULES.contains(&name) {
        Ok(())
    } else {
        Err(format!(
            "--rule `{name}`: unknown (known: {})",
            RULES.join(", ")
        ))
    }
}

/// [`check_rule`] as a `clap` value parser (`qp demo --rule`).
///
/// # Errors
/// An unknown rule name.
pub(crate) fn check_rule_arg(name: &str) -> Result<String, String> {
    check_rule(name).map(|()| name.to_owned())
}

/// `qp sim` arguments.
#[derive(Debug, Clone, clap::Args)]
pub struct SimArgs {
    /// Universe: comma-separated symbols or `@file` (one per line).
    #[arg(long, default_value = "@fixtures/local/live/u100.txt")]
    pub symbols: String,
    /// First day (`YYYY-MM-DD`, exchange time zone).
    #[arg(long)]
    pub start: String,
    /// Last day (inclusive; default: `--start`).
    #[arg(long)]
    pub end: Option<String>,
    /// (RP4/T6) Start time on the first day (`HH:MM`, exchange time zone; default:
    /// midnight). That day's earlier trades are history.
    #[arg(long, value_name = "HH:MM")]
    pub at: Option<String>,
    /// Bar interval (`1m`, `5m`, `15m`, `30m`, `1h`, `1d`).
    #[arg(long, default_value = "5m")]
    pub interval: String,
    /// Pace: `N` (N× real time inside sessions) or `max` (no waiting).
    #[arg(long, default_value = "max")]
    pub speed: String,
    /// Settled bars of history before the start (the first resolution).
    #[arg(long, default_value_t = 120)]
    pub history: u32,
    /// Event stream (JSON lines) — only under `fixtures/local/` (real data).
    #[arg(long)]
    pub out: Option<PathBuf>,
    /// Capsule constant record by name (`r3`, the default; `rp4`).
    #[arg(long, default_value = "r3")]
    pub rule: String,
    /// The host's feed rule (as `qp demo --feed-rule`).
    #[arg(long, default_value = "rate55")]
    pub feed_rule: String,
    /// Cached trades (`<dir>/<SYM>/<start>_<end>.csv.gz`).
    #[arg(long, default_value = "fixtures/local/trades")]
    pub trades_dir: PathBuf,
    /// Cached REST bars (`<SYM>.<period>.<start>.<end>.unadj.json`).
    #[arg(long, default_value = "fixtures/local/live/rest")]
    pub rest_dir: PathBuf,
    /// Repository root (relative paths of the session init; `--out` must be under its
    /// `fixtures/local/`).
    #[arg(long, default_value = ".")]
    pub root: PathBuf,
}

/// The pace.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Speed {
    /// N× real time inside sessions.
    Factor(f64),
    /// No waiting.
    Max,
}

impl Speed {
    /// Reads `N` (> 0) or `max`.
    ///
    /// # Errors
    /// Anything else.
    pub fn parse(s: &str) -> Result<Self, String> {
        if s.eq_ignore_ascii_case("max") {
            return Ok(Self::Max);
        }
        match s.trim_end_matches(['x', 'X']).parse::<f64>() {
            Ok(f) if f.is_finite() && f > 0.0 => Ok(Self::Factor(f)),
            _ => Err(format!("--speed `{s}`: a number > 0 or `max`")),
        }
    }

    /// Short text (`60x`, `max`).
    #[must_use]
    pub fn label(self) -> String {
        match self {
            Self::Factor(f) => format!("{f}x"),
            Self::Max => "max".to_owned(),
        }
    }
}

/// What a simulation runs on (from `qp sim` or `qp demo --sim`).
#[derive(Debug, Clone)]
pub(crate) struct SimSpec {
    /// Symbols.
    pub symbols: Vec<String>,
    /// First and last local day (day numbers).
    pub first_day: i64,
    /// Last local day (inclusive).
    pub last_day: i64,
    /// (RP4/T6) Start time on the first day (minutes after local midnight; default 0).
    /// The trades of that day before it are history (bars built from them), as if the
    /// host had started then.
    pub at_min: u32,
    /// Bar interval.
    pub interval: String,
    /// Settled bars of history.
    pub history: u32,
    /// The host's feed rule.
    pub feed_rule: String,
    /// (RP4) The capsule constant record (`--rule`).
    pub rule: String,
    /// Cached trades directory.
    pub trades_dir: PathBuf,
    /// Cached REST bars directory.
    pub rest_dir: PathBuf,
    /// Repository root.
    pub root: PathBuf,
}

impl SimSpec {
    /// From `qp sim` arguments.
    ///
    /// # Errors
    /// Bad symbols, dates, interval or rule.
    pub fn from_args(a: &SimArgs) -> Result<Self, String> {
        Self::new(
            &a.symbols,
            &a.start,
            a.end.as_deref(),
            &a.interval,
            a.history,
            &a.feed_rule,
            &a.rule,
            (&a.trades_dir, &a.rest_dir, &a.root),
        )?
        .with_at(a.at.as_deref())
    }

    /// From the parts.
    ///
    /// # Errors
    /// Bad symbols, dates, interval or rule.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        symbols: &str,
        start: &str,
        end: Option<&str>,
        interval: &str,
        history: u32,
        feed_rule: &str,
        rule: &str,
        dirs: (&Path, &Path, &Path),
    ) -> Result<Self, String> {
        check_rule(rule)?;
        let symbols = crate::live::parse_symbols(symbols).map_err(|e| format!("--symbols: {e}"))?;
        let first_day = qp_calendar::parse_date(start).map_err(|e| format!("--start: {e}"))?;
        let last_day = match end {
            Some(e) => qp_calendar::parse_date(e).map_err(|e| format!("--end: {e}"))?,
            None => first_day,
        };
        if last_day < first_day {
            return Err("--end is before --start".to_owned());
        }
        Interval::parse(interval).map_err(|e| format!("--interval: {e}"))?;
        crate::livehost::rest_period(interval)
            .ok_or_else(|| format!("--interval `{interval}`: not a live interval"))?;
        let rel = |p: &Path| {
            if p.is_absolute() {
                p.to_owned()
            } else {
                dirs.2.join(p)
            }
        };
        Ok(Self {
            symbols,
            first_day,
            last_day,
            at_min: 0,
            interval: interval.to_owned(),
            history,
            feed_rule: feed_rule.to_owned(),
            rule: rule.to_owned(),
            trades_dir: rel(dirs.0),
            rest_dir: rel(dirs.1),
            root: dirs.2.to_owned(),
        })
    }
}

/// XIST.
///
/// # Errors
/// The built-in calendar is missing (never).
pub(crate) fn xist() -> Result<ExchangeCalendar, String> {
    crate::live::xist()
}

/// UTC ms of local midnight starting local day `day`.
fn local_midnight(cal: &ExchangeCalendar, day: i64) -> i64 {
    // The zone's offset near that day (XIST: +03 all year since 2016).
    let guess = day * 86_400_000;
    let off = i64::from(cal.zone().offset_at(guess.div_euclid(1000))) * 1000;
    guess - off
}

fn date_text(day: i64) -> String {
    let (y, m, d) = qp_calendar::civil_from_days(day);
    format!("{y:04}-{m:02}-{d:02}")
}

impl SimSpec {
    /// (RP4/T6) Starts at local time `hhmm` (`HH:MM`) of the first day instead of its
    /// midnight (`None`: midnight).
    ///
    /// # Errors
    /// Not `HH:MM` within the day.
    pub fn with_at(mut self, hhmm: Option<&str>) -> Result<Self, String> {
        let Some(t) = hhmm else {
            return Ok(self);
        };
        let bad = || format!("--at `{t}`: HH:MM");
        let (h, m) = t.split_once(':').ok_or_else(bad)?;
        let (h, m): (u32, u32) = (h.parse().map_err(|_| bad())?, m.parse().map_err(|_| bad())?);
        if h > 23 || m > 59 {
            return Err(bad());
        }
        self.at_min = h * 60 + m;
        Ok(self)
    }
}

// ---------------------------------------------------------------------------------------
// The trade cache.

/// Counters of a cache read (no values).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct LoadStats {
    /// Files read.
    pub files: u64,
    /// Rows kept (in the range, after de-duplication).
    pub trades: u64,
    /// Rows that did not parse.
    pub bad_rows: u64,
    /// Rows dropped as duplicates (same trade number; overlapping windows).
    pub duplicates: u64,
}

/// The cache files of `sym` whose window overlaps local days `[d0, d1]`, sorted.
fn trade_files(dir: &Path, sym: &str, d0: i64, d1: i64) -> Vec<PathBuf> {
    let Ok(rd) = std::fs::read_dir(dir.join(sym)) else {
        return Vec::new();
    };
    let mut v: Vec<PathBuf> = rd
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let stem = name.strip_suffix(".csv.gz")?;
            let (a, b) = stem.split_once('_')?;
            let (a, b) = (
                qp_calendar::parse_date(a).ok()?,
                qp_calendar::parse_date(b).ok()?,
            );
            (a <= d1 && b >= d0).then(|| e.path())
        })
        .collect();
    v.sort();
    v
}

/// `sym`'s cached trades with time in `[from_ms, to_ms)`, de-duplicated by trade number,
/// in exchange order. Rows of other local days are skipped before parsing.
///
/// # Errors
/// An unreadable file.
pub(crate) fn load_trades(
    dir: &Path,
    sym: &str,
    from_ms: i64,
    to_ms: i64,
    cal: &ExchangeCalendar,
) -> Result<(Vec<Trade>, LoadStats), String> {
    let (d0, d1) = (cal.local_day(from_ms), cal.local_day(to_ms - 1));
    let days: BTreeSet<String> = (d0..=d1).map(date_text).collect();
    let mut st = LoadStats::default();
    let mut all = Vec::new();
    for f in trade_files(dir, sym, d0, d1) {
        st.files += 1;
        let file = std::fs::File::open(&f).map_err(|e| format!("{}: {e}", f.display()))?;
        let mut r = BufReader::new(flate2::read::GzDecoder::new(file));
        // Header, then only the rows of the wanted local days (the `date_time` column,
        // the second, starts with the local date), parsed in pieces of about
        // `CHUNK` bytes (RP4/T6: the loading peak; rows parse one by one).
        const CHUNK: usize = 4 << 20;
        let mut header: Vec<u8> = Vec::new();
        let mut text = Vec::new();
        let mut line = Vec::new();
        let mut first = true;
        let mut flush = |text: &mut Vec<u8>, header: &[u8], all: &mut Vec<Trade>| {
            if text.len() > header.len() {
                let (rows, bad) = qp_matriks::trade::parse_rest_csv(text);
                st.bad_rows += bad as u64;
                all.extend(
                    rows.into_iter()
                        .filter(|t| t.symbol == sym && (from_ms..to_ms).contains(&t.t_ms())),
                );
            }
            text.clear();
            text.extend_from_slice(header);
        };
        loop {
            line.clear();
            let n = r
                .read_until(b'\n', &mut line)
                .map_err(|e| format!("{}: {e}", f.display()))?;
            if n == 0 {
                break;
            }
            if first {
                first = false;
                header.clone_from(&line);
                text.extend_from_slice(&line);
                continue;
            }
            let keep = line
                .iter()
                .position(|&b| b == b',')
                .and_then(|i| line.get(i + 1..i + 11))
                .and_then(|d| std::str::from_utf8(d).ok())
                .is_some_and(|d| days.contains(d));
            if keep {
                text.extend_from_slice(&line);
                if text.len() >= CHUNK {
                    flush(&mut text, &header, &mut all);
                }
            }
        }
        flush(&mut text, &header, &mut all);
    }
    all.sort_by(Trade::exchange_cmp);
    let mut seen = BTreeSet::new();
    let before = all.len();
    all.retain(|t| seen.insert(t.dedup_key()));
    st.duplicates = (before - all.len()) as u64;
    st.trades = all.len() as u64;
    Ok((all, st))
}

/// Loads several symbols in parallel (results in symbol order).
fn load_many<T: Send>(
    dir: &Path,
    symbols: &[String],
    from_ms: i64,
    to_ms: i64,
    cal: &ExchangeCalendar,
    keep: &(dyn Fn(&str, Vec<Trade>) -> T + Sync),
) -> Result<Vec<(String, T, LoadStats)>, String> {
    let n = std::thread::available_parallelism().map_or(4, std::num::NonZero::get);
    let chunks: Vec<&[String]> = symbols.chunks(symbols.len().div_ceil(n).max(1)).collect();
    let results: Vec<Result<Vec<_>, String>> = std::thread::scope(|s| {
        let hs: Vec<_> = chunks
            .iter()
            .map(|c| {
                s.spawn(move || {
                    c.iter()
                        .map(|sym| {
                            load_trades(dir, sym, from_ms, to_ms, cal)
                                .map(|(t, st)| (sym.clone(), keep(sym, t), st))
                        })
                        .collect::<Result<Vec<_>, String>>()
                })
            })
            .collect();
        hs.into_iter()
            .map(|h| h.join().unwrap_or_else(|_| Err("loader panicked".into())))
            .collect()
    });
    let mut out = Vec::new();
    for r in results {
        out.extend(r?);
    }
    Ok(out)
}

/// (RP4/T6) One symbol's cached trades, packed (32 bytes a trade, no allocation each; a
/// trade number that is not a plain decimal and an unusual side text are kept aside): a
/// day of 580 symbols' trades in memory once and small. [`Packed::get`] gives back the
/// very [`Trade`] that was packed.
#[derive(Debug, Default)]
pub(crate) struct Packed {
    symbol: String,
    rows: Vec<Row>,
    odd_order: BTreeMap<u32, String>,
    odd_side: BTreeMap<u32, String>,
}

#[derive(Debug, Clone, Copy)]
struct Row {
    ts_ns: i64,
    price: f64,
    quantity: i64,
    /// The trade number (below 2^56) in the low 56 bits; the top byte: bit 0 quantity
    /// given, bit 1 trade number given, bit 2 special trade, bit 3 the trade number is in
    /// `odd_order`, bits 4–5 the side (0 none, 1 bid, 2 ask, 3 other: `odd_side`).
    packed: u64,
}

/// Trade numbers kept in a row (below 2^56).
const NUMBER_BITS: u32 = 56;

/// A trade number written back exactly by `u64::to_string` and small enough for a row.
fn plain_number(s: &str) -> Option<u64> {
    let ok = !s.is_empty()
        && s.len() <= 19
        && s.bytes().all(|b| b.is_ascii_digit())
        && (s == "0" || !s.starts_with('0'));
    ok.then(|| s.parse().ok())
        .flatten()
        .filter(|n: &u64| *n < 1 << NUMBER_BITS)
}

impl Packed {
    /// Packs `trades` (all of `symbol`, in order).
    #[must_use]
    pub fn new(symbol: &str, trades: &[Trade]) -> Self {
        let mut odd_order = BTreeMap::new();
        let mut odd_side = BTreeMap::new();
        let rows = trades
            .iter()
            .enumerate()
            .map(|(i, t)| {
                debug_assert_eq!(t.symbol, symbol);
                let i = u32::try_from(i).unwrap_or(u32::MAX);
                let (order_no, odd) = match t.order_no.as_deref() {
                    None => (0, false),
                    Some(o) => match plain_number(o) {
                        Some(n) => (n, false),
                        None => {
                            odd_order.insert(i, o.to_owned());
                            (0, true)
                        }
                    },
                };
                let side: u8 = match &t.side {
                    None => 0,
                    Some(qp_matriks::Aggressor::Bid) => 1,
                    Some(qp_matriks::Aggressor::Ask) => 2,
                    Some(qp_matriks::Aggressor::Other(x)) => {
                        odd_side.insert(i, x.clone());
                        3
                    }
                };
                let flags = u64::from(t.quantity.is_some())
                    | u64::from(t.order_no.is_some()) << 1
                    | u64::from(t.is_trade_ex) << 2
                    | u64::from(odd) << 3
                    | u64::from(side) << 4;
                Row {
                    ts_ns: t.timestamp_ns,
                    price: t.price,
                    quantity: t.quantity.unwrap_or(0),
                    packed: order_no | flags << NUMBER_BITS,
                }
            })
            .collect();
        Self {
            symbol: symbol.to_owned(),
            rows,
            odd_order,
            odd_side,
        }
    }

    fn len(&self) -> usize {
        self.rows.len()
    }

    fn ts_ns(&self, i: usize) -> i64 {
        self.rows[i].ts_ns
    }

    fn t_ms(&self, i: usize) -> i64 {
        self.rows[i].ts_ns.div_euclid(1_000_000)
    }

    /// Trade `i`.
    #[must_use]
    pub fn get(&self, i: usize) -> Trade {
        let r = self.rows[i];
        let k = u32::try_from(i).unwrap_or(u32::MAX);
        let flags = r.packed >> NUMBER_BITS;
        let order_no = r.packed & ((1 << NUMBER_BITS) - 1);
        Trade {
            symbol: self.symbol.clone(),
            price: r.price,
            quantity: (flags & 1 != 0).then_some(r.quantity),
            side: match (flags >> 4) & 3 {
                1 => Some(qp_matriks::Aggressor::Bid),
                2 => Some(qp_matriks::Aggressor::Ask),
                3 => Some(qp_matriks::Aggressor::Other(
                    self.odd_side.get(&k).cloned().unwrap_or_default(),
                )),
                _ => None,
            },
            timestamp_ns: r.ts_ns,
            order_no: (flags & 2 != 0).then(|| {
                if flags & 8 != 0 {
                    self.odd_order.get(&k).cloned().unwrap_or_default()
                } else {
                    order_no.to_string()
                }
            }),
            is_trade_ex: flags & 4 != 0,
        }
    }
}

/// (RP4/T6) The trades of several [`Packed`] symbols in exchange order (time, symbol,
/// trade order), merged as they are read: a symbol's trades are in trade order already,
/// so (time, symbol, position) is that order.
pub(crate) struct Merge<'a> {
    syms: Vec<&'a Packed>,
    heap: std::collections::BinaryHeap<std::cmp::Reverse<(i64, usize, usize)>>,
}

impl Merge<'_> {
    /// The next trade's time (UTC ms).
    #[must_use]
    pub fn peek_t_ms(&self) -> Option<i64> {
        self.heap.peek().map(|r| self.syms[r.0 .1].t_ms(r.0 .2))
    }
}

impl Iterator for Merge<'_> {
    type Item = Trade;
    fn next(&mut self) -> Option<Trade> {
        let std::cmp::Reverse((_, k, i)) = self.heap.pop()?;
        let p = self.syms[k];
        if i + 1 < p.len() {
            self.heap
                .push(std::cmp::Reverse((p.ts_ns(i + 1), k, i + 1)));
        }
        Some(p.get(i))
    }
}

/// The simulation's [`TradeHistory`]: the cached trades (what REST answers live).
pub(crate) struct CacheHistory {
    by_symbol: BTreeMap<String, Packed>,
}

impl TradeHistory for CacheHistory {
    fn trades(&self, symbol: &str, start_ms: i64, end_ms: i64) -> Result<Vec<Trade>, String> {
        let v = self
            .by_symbol
            .get(symbol)
            .ok_or_else(|| "no cached trades".to_owned())?;
        let pp = |f: &dyn Fn(i64) -> bool| {
            let (mut lo, mut hi) = (0, v.len());
            while lo < hi {
                let mid = lo + (hi - lo) / 2;
                if f(v.t_ms(mid)) {
                    lo = mid + 1;
                } else {
                    hi = mid;
                }
            }
            lo
        };
        let a = pp(&|t| t < start_ms);
        let b = pp(&|t| t <= end_ms);
        Ok((a..b.max(a)).map(|i| v.get(i)).collect())
    }
}

// ---------------------------------------------------------------------------------------
// History (first resolution).

/// Which source a symbol's history came from (counts).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct HistoryFrom {
    /// From the cached REST bars.
    pub rest: Vec<String>,
    /// Built from the cached trades.
    pub trades: Vec<String>,
}

/// Settled bars from the caches: REST bars (unadjusted, normalised like the live REST
/// source), or bars built from the cached trades when that gives more bars in the range.
pub(crate) struct CacheBars {
    /// REST bars directory.
    pub rest_dir: PathBuf,
    /// Trades directory.
    pub trades_dir: PathBuf,
    /// Calendar.
    pub cal: ExchangeCalendar,
    /// Which source each symbol took (last fetch).
    pub from: Mutex<HistoryFrom>,
}

impl CacheBars {
    /// REST rows of `sym` × `period` from the unadjusted cache files (later file wins).
    fn rest_rows(&self, sym: &str, period: &str) -> Result<Vec<(i64, Bar)>, String> {
        let prefix = format!("{sym}.{period}.");
        let mut files: Vec<PathBuf> = std::fs::read_dir(&self.rest_dir)
            .map(|rd| {
                rd.flatten()
                    .filter(|e| {
                        let n = e.file_name().to_string_lossy().into_owned();
                        n.starts_with(&prefix) && n.ends_with(".unadj.json")
                    })
                    .map(|e| e.path())
                    .collect()
            })
            .unwrap_or_default();
        files.sort();
        let mut rows: BTreeMap<i64, Bar> = BTreeMap::new();
        for p in files {
            let text = std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?;
            let v: Vec<Value> =
                serde_json::from_str(&text).map_err(|e| format!("{}: {e}", p.display()))?;
            for r in v {
                let num = |k: &str| r.get(k).and_then(Value::as_f64);
                let (Some(t), Some(o), Some(h), Some(l), Some(c)) = (
                    r.get("time").and_then(Value::as_i64),
                    num("open"),
                    num("high"),
                    num("low"),
                    num("close"),
                ) else {
                    continue;
                };
                let v = num("volume").unwrap_or(0.0);
                rows.insert(t, Bar { t, o, h, l, c, v });
            }
        }
        Ok(rows.into_iter().collect())
    }

    /// Bars of `sym` on `iv` built from the cached trades in `[from_ms, to_ms)` with the
    /// live bar builder (`BarMethod::Trades`), final ones only.
    fn trade_bars(&self, sym: &str, iv: &str, from_ms: i64, to_ms: i64) -> Vec<Bar> {
        let Ok((trades, _)) = load_trades(&self.trades_dir, sym, from_ms, to_ms, &self.cal) else {
            return Vec::new();
        };
        trade_bars(sym, iv, &trades, &self.cal, to_ms)
    }
}

/// Final bars of `sym` on `iv` built from `trades` (exchange order) with the live bar
/// builder (`BarMethod::Trades`, special trades excluded), closed through `until_ms`.
#[must_use]
pub(crate) fn trade_bars(
    sym: &str,
    iv: &str,
    trades: &[Trade],
    cal: &ExchangeCalendar,
    until_ms: i64,
) -> Vec<Bar> {
    let key = FeedKey {
        symbol: sym.to_owned(),
        interval: iv.to_owned(),
        feed: None,
    };
    let Ok(mut lb) = qp_source::LiveBars::new(
        key.clone(),
        cal.clone(),
        qp_source::LiveConfig {
            method: qp_source::BarMethod::Trades,
            ..qp_source::LiveConfig::default()
        },
    ) else {
        return Vec::new();
    };
    let mut store = qp_source::FeedStore::new(key);
    for t in trades {
        for e in lb.push_trade(&t.tick()) {
            let _ = store.apply(&e);
        }
    }
    let _ = lb.close_until(until_ms);
    let last_final = lb.last_final_t();
    store
        .bars()
        .iter()
        .filter(|b| last_final.is_some_and(|f| b.t <= f))
        .cloned()
        .collect()
}

impl BarSource for CacheBars {
    fn fetch(
        &self,
        symbols: &[String],
        interval: &str,
        from_ms: i64,
        to_ms: i64,
    ) -> Vec<(String, Result<Vec<Bar>, String>)> {
        let (Some(period), Ok(iv)) = (
            crate::livehost::rest_period(interval),
            Interval::parse(interval),
        ) else {
            return Vec::new();
        };
        let mut from = HistoryFrom::default();
        let out = symbols
            .iter()
            .map(|s| {
                let rest: Vec<Bar> = match self.rest_rows(s, period) {
                    Ok(rows) => crate::livehost::normalize_rest(&rows, iv, &self.cal)
                        .into_iter()
                        .filter(|b| b.t >= from_ms && b.t <= to_ms)
                        .collect(),
                    Err(e) => return (s.clone(), Err(e)),
                };
                let built = self.trade_bars(s, interval, from_ms, to_ms + 1);
                if built.len() > rest.len() {
                    from.trades.push(s.clone());
                    (s.clone(), Ok(built))
                } else {
                    if !rest.is_empty() {
                        from.rest.push(s.clone());
                    }
                    (s.clone(), Ok(rest))
                }
            })
            .collect();
        *self
            .from
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = from;
        out
    }
}

// ---------------------------------------------------------------------------------------
// Inputs, schedule, records.

/// Everything a simulation needs, loaded.
pub(crate) struct SimInput {
    /// Calendar.
    pub cal: ExchangeCalendar,
    /// Start (local midnight of the first day, UTC ms) and end (local midnight after the
    /// last day).
    pub from_ms: i64,
    /// See `from_ms`.
    pub to_ms: i64,
    /// (RP4/T6) The symbols whose trades are sent (those with history), in order.
    sent: Vec<String>,
    /// Their number of trades.
    count: usize,
    /// The trades by symbol (the trade history).
    pub history: Arc<CacheHistory>,
    /// The first resolution: settled bars per symbol.
    pub hist: Vec<(String, Vec<Bar>)>,
    /// Symbols without history (left out).
    pub missing: Vec<String>,
    /// Where the history came from.
    pub hist_from: HistoryFrom,
    /// Cache read counters.
    pub load: LoadStats,
    /// The bar source (history; also the runtime's repair source).
    pub source: Arc<CacheBars>,
}

impl SimInput {
    /// Number of trades sent.
    #[must_use]
    pub fn trade_count(&self) -> usize {
        self.count
    }

    /// Every trade sent, in exchange order (time, symbol, number).
    #[must_use]
    pub fn trades(&self) -> Merge<'_> {
        let syms: Vec<&Packed> = self
            .sent
            .iter()
            .filter_map(|s| self.history.by_symbol.get(s))
            .collect();
        let heap = syms
            .iter()
            .enumerate()
            .filter(|(_, p)| p.len() > 0)
            .map(|(k, p)| std::cmp::Reverse((p.ts_ns(0), k, 0)))
            .collect();
        Merge { syms, heap }
    }
}

/// Loads the caches for `spec`.
///
/// # Errors
/// Unreadable cache files; no symbol with history.
pub(crate) fn load(spec: &SimSpec) -> Result<SimInput, String> {
    let cal = xist()?;
    let from_ms = local_midnight(&cal, spec.first_day) + i64::from(spec.at_min) * 60_000;
    let to_ms = local_midnight(&cal, spec.last_day + 1);
    let per = load_many(
        &spec.trades_dir,
        &spec.symbols,
        from_ms,
        to_ms,
        &cal,
        &|s, t| Packed::new(s, &t),
    )?;
    let mut load = LoadStats::default();
    let mut by_symbol = BTreeMap::new();
    for (s, t, st) in per {
        load.files += st.files;
        load.trades += st.trades;
        load.bad_rows += st.bad_rows;
        load.duplicates += st.duplicates;
        by_symbol.insert(s, t);
    }
    let source = Arc::new(CacheBars {
        rest_dir: spec.rest_dir.clone(),
        trades_dir: spec.trades_dir.clone(),
        cal: cal.clone(),
        from: Mutex::new(HistoryFrom::default()),
    });
    let (hist, missing) = crate::livehost::history(
        source.as_ref(),
        &spec.symbols,
        &spec.interval,
        spec.history as usize,
        from_ms,
        &cal,
    )?;
    let hist_from = source
        .from
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone();
    let keep: BTreeSet<&str> = hist.iter().map(|(s, _)| s.as_str()).collect();
    let sent: Vec<String> = by_symbol
        .keys()
        .filter(|s| keep.contains(s.as_str()))
        .cloned()
        .collect();
    let count = sent.iter().map(|s| by_symbol[s].len()).sum();
    Ok(SimInput {
        cal,
        from_ms,
        to_ms,
        sent,
        count,
        history: Arc::new(CacheHistory { by_symbol }),
        hist,
        missing,
        hist_from,
        load,
        source,
    })
}

/// The clock points besides the trades (UTC ms, ascending): for every trading day in
/// `[from_ms, to_ms)` the session opens, every bar close of `interval` and the session
/// closes; the local midnight after each day; and `to_ms`.
#[must_use]
pub(crate) fn schedule(cal: &ExchangeCalendar, interval: Interval, from_ms: i64, to_ms: i64) -> Vec<i64> {
    let mut v = BTreeSet::new();
    let (d0, d1) = (cal.local_day(from_ms), cal.local_day(to_ms - 1));
    for day in d0..=d1 {
        for s in cal.sessions_on(day) {
            v.insert(s.open);
            v.insert(s.close);
        }
        v.insert(local_midnight(cal, day + 1));
    }
    for b in cal.bars_between(interval, from_ms, to_ms) {
        v.insert(b.close);
    }
    v.insert(to_ms);
    v.into_iter()
        .filter(|&t| t > from_ms && t <= to_ms)
        .collect()
}

/// The wire message of `t` as the trade endpoint sends it (level 1; buyer and seller
/// absent), its `timestamp` in `cfg.unit` (since-midnight: of the trade's local day).
#[must_use]
pub(crate) fn wire(t: &Trade, cfg: &TradeDecodeConfig) -> Vec<u8> {
    let ns = t.timestamp_ns;
    let off_ns = i64::from(cfg.utc_offset_min) * 60_000_000_000;
    let day_ns = 86_400_000_000_000i64;
    let ts = match cfg.unit {
        TradeTimeUnit::NanosFromMidnight | TradeTimeUnit::Auto => (ns + off_ns).rem_euclid(day_ns),
        TradeTimeUnit::EpochMillis => ns.div_euclid(1_000_000),
        TradeTimeUnit::EpochMicros => ns.div_euclid(1000),
        TradeTimeUnit::EpochNanos => ns,
    };
    #[allow(clippy::cast_possible_truncation)]
    let price = t.price as f32;
    TradeMessage {
        symbol: t.symbol.clone(),
        order_no: t.order_no.clone(),
        price: Some(price),
        quantity: t.quantity,
        active_bid_or_ask: t.side.as_ref().map(|s| match s {
            qp_matriks::Aggressor::Bid => "B".to_owned(),
            qp_matriks::Aggressor::Ask => "A".to_owned(),
            qp_matriks::Aggressor::Other(x) => x.clone(),
        }),
        timestamp: Some(ts),
        buyer: None,
        seller: None,
        is_trade_ex: Some(t.is_trade_ex),
    }
    .encode_to_vec()
}

/// The raw record of `t` arriving at its own exchange time (what the trade endpoint's
/// sink hands the hub).
#[must_use]
pub(crate) fn record(t: &Trade, cfg: &TradeDecodeConfig) -> RawRecord {
    let topic = if t.is_trade_ex {
        format!("mx/tradeex/{}", t.symbol)
    } else {
        format!("mx/trade/{}", t.symbol)
    };
    RawRecord::message(t.timestamp_ns.div_euclid(1000), topic, wire(t, cfg))
}

/// The trade endpoint's connection record at `t_ms`.
#[must_use]
pub(crate) fn conn_record(kind: RecordKind, t_ms: i64) -> RawRecord {
    RawRecord {
        t_us: t_ms * 1000,
        kind,
        topic: String::new(),
        payload: b"session 1".to_vec(),
    }
}

// ---------------------------------------------------------------------------------------
// Pace.

/// Real time for the pacer (a fake one in tests).
pub(crate) trait PaceClock {
    /// Real time since an arbitrary origin.
    fn now(&self) -> Duration;
    /// Waits `d`.
    fn sleep(&self, d: Duration);
}

/// The wall clock.
pub(crate) struct WallPace(Instant);

impl WallPace {
    /// Starting now.
    #[must_use]
    pub fn new() -> Self {
        Self(Instant::now())
    }
}

impl Default for WallPace {
    fn default() -> Self {
        Self::new()
    }
}

impl PaceClock for WallPace {
    fn now(&self) -> Duration {
        self.0.elapsed()
    }
    fn sleep(&self, d: Duration) {
        std::thread::sleep(d);
    }
}

/// Sends simulation time at a pace ([`Speed`]): inside a session N× real time, a closed
/// stretch at most [`CLOSED_PAUSE`].
pub(crate) struct Pacer<'a> {
    speed: Speed,
    clock: &'a dyn PaceClock,
    cal: ExchangeCalendar,
    origin: Duration,
    /// Real time the simulation should have reached.
    target: Duration,
    last_ms: Option<i64>,
}

impl<'a> Pacer<'a> {
    /// A pacer starting at `clock.now()`.
    #[must_use]
    pub fn new(speed: Speed, clock: &'a dyn PaceClock, cal: ExchangeCalendar) -> Self {
        Self {
            speed,
            origin: clock.now(),
            clock,
            cal,
            target: Duration::ZERO,
            last_ms: None,
        }
    }

    /// Waits until simulation time `t_ms` is due.
    pub fn wait_until(&mut self, t_ms: i64) {
        let Speed::Factor(f) = self.speed else {
            return;
        };
        let Some(prev) = self.last_ms.replace(t_ms) else {
            return;
        };
        if t_ms <= prev {
            return;
        }
        #[allow(clippy::cast_precision_loss)]
        let real = Duration::from_secs_f64((t_ms - prev) as f64 / 1000.0 / f);
        let real = if self.cal.in_session(prev) {
            real
        } else {
            real.min(CLOSED_PAUSE)
        };
        self.target += real;
        let elapsed = self.clock.now().saturating_sub(self.origin);
        if self.target > elapsed {
            self.clock.sleep(self.target - elapsed);
        }
    }
}

// ---------------------------------------------------------------------------------------
// Driving.

/// Counts of a drive (no values).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct DriveStats {
    /// Trade records sent.
    pub trades: u64,
    /// Clock moves sent.
    pub clock_moves: u64,
    /// Local days the clock went through.
    pub days: u64,
}

/// Sends the simulation's inputs to `sink` in order: the clock to the start, the trade
/// connection opening, then every trade (as its wire record) and clock point
/// ([`schedule`]) in time order, each clock move paced by `pacer`. `sink` returns
/// `false` to stop early.
pub(crate) fn drive(
    input: &SimInput,
    interval: Interval,
    decode: &TradeDecodeConfig,
    pacer: &mut Pacer<'_>,
    sink: &mut dyn FnMut(LiveIn) -> bool,
) -> DriveStats {
    let mut st = DriveStats::default();
    let ticks = schedule(&input.cal, interval, input.from_ms, input.to_ms);
    let mut clock = input.from_ms;
    let mut day = input.cal.local_day(clock);
    st.days = 1;
    pacer.wait_until(clock);
    if !sink(LiveIn::Clock(clock * 1000)) {
        return st;
    }
    if !sink(LiveIn::Trade(conn_record(
        RecordKind::ConnectionOpened,
        clock,
    ))) {
        return st;
    }
    let mut j = 0usize;
    let mut trades = input.trades();
    loop {
        let trade_t = trades.peek_t_ms();
        let tick_t = ticks.get(j).copied();
        let (t, is_tick) = match (trade_t, tick_t) {
            (Some(a), Some(b)) if b <= a => (b, true),
            (Some(a), _) => (a, false),
            (None, Some(b)) => (b, true),
            (None, None) => break,
        };
        if t > clock {
            pacer.wait_until(t);
            clock = t;
            st.clock_moves += 1;
            let d = input.cal.local_day(t.min(input.to_ms - 1));
            if d != day {
                day = d;
                st.days += 1;
            }
            if !sink(LiveIn::Clock(t * 1000)) {
                return st;
            }
        }
        if is_tick {
            j += 1;
        } else {
            let Some(t) = trades.next() else { break };
            if !sink(LiveIn::Trade(record(&t, decode))) {
                return st;
            }
            st.trades += 1;
        }
    }
    st
}

/// A simulation's runtime: the live host over the history and the live runtime on the
/// simulated clock, watching every symbol's trades from the start.
///
/// # Errors
/// The host or the runtime refuses the history.
pub(crate) fn runtime(
    spec: &SimSpec,
    input: &SimInput,
    to_hub: Arc<dyn Fn(LiveIn) + Send + Sync>,
    decode: TradeDecodeConfig,
) -> Result<(ReplayHost, LiveRuntime), String> {
    let host = host(spec, input)?;
    let feed = MatriksFeed::with_trades(
        input.cal.clone(),
        &spec.interval,
        qp_matriks::TradeFeedConfig {
            decode,
            ..qp_matriks::TradeFeedConfig::default()
        },
    );
    let mut rt = LiveRuntime::new(
        &host,
        Box::new(feed),
        input.source.clone(),
        to_hub,
        input.cal.clone(),
        &spec.interval,
        0,
    )?;
    rt.use_sim_clock(input.from_ms * 1000);
    rt.set_trade_history(input.history.clone());
    rt.watch_trades(input.from_ms);
    rt.baseline_capsules(&host);
    Ok((host, rt))
}

/// The live host of a simulation (`crate::livehost::host_from_history`, as `qp demo
/// --live` builds it).
///
/// # Errors
/// The host refuses the history.
pub(crate) fn host(spec: &SimSpec, input: &SimInput) -> Result<ReplayHost, String> {
    let mut host = crate::livehost::host_from_history(
        input.hist.clone(),
        &spec.interval,
        &qp_shellcore::shell::DemoOptions {
            local: true,
            instruments: 0,
            history: spec.history,
            days: 260,
            speed: 4.0,
            from_t: None,
            running: true,
            feed_rule: Some(spec.feed_rule.clone()),
            engine: engine_field(&spec.rule),
        },
        &spec.root,
    )?;
    host.set_record_events(true);
    Ok(host)
}

/// The event stream of a run: the data events the authority took, capsule births and
/// state changes, market status changes and the authority's messages, one JSON line each,
/// with a running SHA-256 (the digest compares two runs without printing data).
pub(crate) struct Stream {
    w: Option<BufWriter<std::fs::File>>,
    hash: Sha256,
    sent: usize,
    capsules: BTreeMap<String, (String, u32)>,
    market: Option<Value>,
    cal: ExchangeCalendar,
    iv: Interval,
    /// Capsules are compared again after a bar event or once the clock reaches this.
    next_check: i64,
    /// Lines written.
    pub lines: u64,
    /// Data events.
    pub events: u64,
    /// Capsules born.
    pub born: u64,
    /// Capsule state changes.
    pub changed: u64,
    /// Market status changes.
    pub market_changes: u64,
    /// Authority messages.
    pub messages: u64,
}

impl Stream {
    /// A stream (to `out` if given) after `host`'s initial state.
    ///
    /// # Errors
    /// The file cannot be created.
    pub fn new(
        out: Option<&Path>,
        host: &ReplayHost,
        cal: ExchangeCalendar,
        iv: Interval,
    ) -> Result<Self, String> {
        let w = match out {
            Some(p) => Some(BufWriter::new(
                std::fs::File::create(p).map_err(|e| format!("{}: {e}", p.display()))?,
            )),
            None => None,
        };
        let capsules = host
            .session()
            .document()
            .capsules
            .iter()
            .map(|c| (c.id.clone(), (format!("{:?}", c.state), c.revision)))
            .collect();
        Ok(Self {
            w,
            hash: Sha256::new(),
            sent: host.recorded_len(),
            capsules,
            market: None,
            cal,
            iv,
            next_check: i64::MIN,
            lines: 0,
            events: 0,
            born: 0,
            changed: 0,
            market_changes: 0,
            messages: 0,
        })
    }

    fn put(&mut self, v: &Value) -> Result<(), String> {
        let s = v.to_string();
        self.hash.update(s.as_bytes());
        self.hash.update(b"\n");
        self.lines += 1;
        if let Some(w) = self.w.as_mut() {
            writeln!(w, "{s}").map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// Takes what one input caused: new data events, the authority's messages, the
    /// market status and capsule changes (compared after a bar event, and when the
    /// authority's clock passes the end of the bar it was in at the last comparison).
    ///
    /// # Errors
    /// The file cannot be written.
    pub fn after(
        &mut self,
        host: &ReplayHost,
        rt: &LiveRuntime,
        lines: &[qp_schema::protocol::OutboundLine],
    ) -> Result<(), String> {
        let total = host.recorded_len();
        let mut bar_event = false;
        if total > self.sent {
            let new: Vec<Value> = host
                .recorded_since(self.sent)
                .iter()
                .map(|e| {
                    bar_event |= matches!(e.change, DataChange::Bar { .. });
                    json!({"ev": e})
                })
                .collect();
            self.sent = total;
            for v in new {
                self.events += 1;
                self.put(&v)?;
            }
        }
        for l in lines {
            self.messages += 1;
            self.put(&json!({"conn": l.conn, "msg": l.msg}))?;
        }
        if let Some(m) = rt.market() {
            let v = json!({"open": m.open, "holiday": m.holiday, "stale": m.stale,
                           "phases": m.phases.len()});
            if self.market.as_ref() != Some(&v) {
                self.market_changes += 1;
                self.market = Some(v.clone());
                self.put(&json!({"t": m.exchange_time, "market": v}))?;
            }
        }
        let t = host.session().clock();
        if bar_event || t >= self.next_check {
            self.capsule_changes(host)?;
            self.next_check = self
                .cal
                .bar_at(self.iv, t)
                .map_or(t + 60_000, |s| s.close.max(t + 1));
        }
        Ok(())
    }

    fn capsule_changes(&mut self, host: &ReplayHost) -> Result<(), String> {
        let doc = host.session().document();
        let t = host.session().clock();
        let mut out = Vec::new();
        for c in &doc.capsules {
            let st = format!("{:?}", c.state);
            match self.capsules.get(&c.id) {
                Some((s, r)) if *s == st && *r == c.revision => continue,
                Some((s, _)) => {
                    if *s != st {
                        self.changed += 1;
                    }
                }
                None => self.born += 1,
            }
            out.push(json!({"t": t, "capsule": c.id, "state": st.to_lowercase(),
                            "rev": c.revision}));
            self.capsules.insert(c.id.clone(), (st, c.revision));
        }
        for v in out {
            self.put(&v)?;
        }
        Ok(())
    }

    /// Flushes and returns the digest (hex).
    ///
    /// # Errors
    /// The file cannot be written.
    pub fn finish(mut self) -> Result<String, String> {
        if let Some(w) = self.w.as_mut() {
            w.flush().map_err(|e| e.to_string())?;
        }
        Ok(self
            .hash
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect())
    }
}

/// Checks that `out` is under `root/fixtures/local/` (real data stays there).
///
/// # Errors
/// Anywhere else.
pub(crate) fn check_out(out: &Path, root: &Path) -> Result<PathBuf, String> {
    let abs = |p: &Path| {
        if p.is_absolute() {
            p.to_owned()
        } else {
            std::env::current_dir().unwrap_or_default().join(p)
        }
    };
    let local = abs(&root.join("fixtures/local"));
    let local = local.canonicalize().map_err(|e| {
        format!(
            "--out: {} must exist ({e}); real data is written only there",
            local.display()
        )
    })?;
    let out = abs(out);
    let parent = out
        .parent()
        .ok_or("--out: no directory")?
        .canonicalize()
        .map_err(|e| format!("--out: {e}"))?;
    let name = out.file_name().ok_or("--out: no file name")?;
    if !parent.starts_with(&local) {
        return Err(format!(
            "--out: real data is written only under {} (got {})",
            local.display(),
            parent.display()
        ));
    }
    Ok(parent.join(name))
}

/// Result counts of a run (printed; no values).
#[derive(Debug, Clone)]
pub(crate) struct RunSummary {
    /// The summary as JSON.
    pub json: Value,
    /// The stream digest.
    pub digest: String,
}

/// Runs a simulation to the end, in this thread: every input goes straight to the
/// runtime (`LiveRuntime::on_in`), then the viewers' flush, as the demo hub does.
///
/// # Errors
/// Loading, the host, or the output file.
pub(crate) fn run(
    spec: &SimSpec,
    speed: Speed,
    out: Option<&Path>,
    pace: &dyn PaceClock,
) -> Result<RunSummary, String> {
    let t0 = Instant::now();
    let input = load(spec)?;
    let load_ms = t0.elapsed().as_millis();
    let decode = TradeDecodeConfig::default();
    let (mut host, mut rt) = runtime(spec, &input, Arc::new(|_| {}), decode)?;
    let iv = Interval::parse(&spec.interval).map_err(|e| e.to_string())?;
    let mut stream = Stream::new(out, &host, input.cal.clone(), iv)?;
    let mut pacer = Pacer::new(speed, pace, input.cal.clone());
    let t1 = Instant::now();
    let mut err = None;
    let ds = drive(&input, iv, &decode, &mut pacer, &mut |i| {
        let clock = matches!(i, LiveIn::Clock(_));
        let mut lines = rt.on_in(&mut host, i);
        if clock {
            lines.extend(host.live_flush());
        }
        match stream.after(&host, &rt, &lines) {
            Ok(()) => true,
            Err(e) => {
                err = Some(e);
                false
            }
        }
    });
    if let Some(e) = err {
        return Err(e);
    }
    let run_ms = t1.elapsed().as_millis();
    let rt_stats = rt.stats(&host);
    let (events, born, changed, market_changes, lines) = (
        stream.events,
        stream.born,
        stream.changed,
        stream.market_changes,
        stream.lines,
    );
    let digest = stream.finish()?;
    let json = json!({
        "symbols": spec.symbols.len(),
        "symbols_with_history": input.hist.len(),
        "history_from_rest": input.hist_from.rest.len(),
        "history_from_trades": input.hist_from.trades.len(),
        "symbols_without_history": input.missing.len(),
        "days": ds.days,
        "trading_days": (spec.first_day..=spec.last_day).filter(|&d| input.cal.is_trading_day(d)).count(),
        "interval": spec.interval,
        "speed": speed.label(),
        "cache": {"files": input.load.files, "trades": input.load.trades,
                  "bad_rows": input.load.bad_rows, "duplicates": input.load.duplicates},
        "trade_records": ds.trades,
        "clock_moves": ds.clock_moves,
        "events": events,
        "capsules_born": born,
        "capsule_state_changes": changed,
        "market_changes": market_changes,
        "stream_lines": lines,
        "finalized_bars": rt_stats["finalized_bars"],
        "events_rejected": rt_stats["events_rejected"],
        "feed_trades": rt_stats["feed"]["trades"],
        "trade_repairs": rt.trade_repairs.0,
        "trade_repair_errors": rt.trade_repairs.1,
        "load_ms": load_ms,
        "run_ms": run_ms,
        "notices": host.notices().len(),
    });
    Ok(RunSummary { json, digest })
}

/// `qp sim`.
///
/// # Errors
/// Arguments, loading, the host, or the output file.
pub fn sim(a: &SimArgs) -> Result<(), String> {
    let speed = Speed::parse(&a.speed)?;
    let spec = SimSpec::from_args(a)?;
    let out = match &a.out {
        Some(p) => Some(check_out(p, &a.root)?),
        None => None,
    };
    let pace = WallPace::new();
    let t0 = Instant::now();
    let r = run(&spec, speed, out.as_deref(), &pace)?;
    let mut j = r.json;
    j["wall_ms"] = json!(t0.elapsed().as_millis());
    j["digest"] = json!(r.digest);
    println!("benzetim / sim: {j}");
    Ok(())
}

/// The demo side: sends the inputs to the hub as `qp demo` commands, paced, waiting for
/// the hub every `SYNC_EVERY` inputs (so `max` does not run ahead of the hub).
pub(crate) fn spawn_feeder(
    input: Arc<SimInput>,
    interval: Interval,
    speed: Speed,
    to_hub: Arc<dyn Fn(LiveIn) + Send + Sync>,
    done: Box<dyn FnOnce(DriveStats) + Send>,
) {
    const SYNC_EVERY: u64 = 20_000;
    std::thread::spawn(move || {
        let pace = WallPace::new();
        let mut pacer = Pacer::new(speed, &pace, input.cal.clone());
        let decode = TradeDecodeConfig::default();
        let mut n = 0u64;
        let paced = matches!(speed, Speed::Factor(_));
        let st = drive(&input, interval, &decode, &mut pacer, &mut |i| {
            // (RP4/T6) At a paced speed the wall-clock send time is the trades' arrival
            // (tick-to-frame).
            let clock = paced && matches!(i, LiveIn::Clock(_));
            to_hub(i);
            if clock {
                to_hub(LiveIn::Arrival(crate::live::now_us()));
            }
            n += 1;
            if n.is_multiple_of(SYNC_EVERY) {
                let (tx, rx) = std::sync::mpsc::channel();
                to_hub(LiveIn::Sync(tx));
                return rx.recv().is_ok();
            }
            true
        });
        done(st);
    });
}

#[cfg(test)]
mod tests;
