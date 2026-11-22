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

mod cache;
mod pace;
mod stream;

pub(crate) use cache::*;
pub(crate) use pace::*;
pub(crate) use stream::*;

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
