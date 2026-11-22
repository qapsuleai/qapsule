//! `qp live` (R6): the live Matriks connection on the command line.
//!
//! - `qp live record`: one connection to the market endpoint (`qp-live`: token, MQTT over
//!   WebSocket, one-connection lock, multi-connect stop, backoff), subscribing
//!   `mx/timestamp` and the price card `mx/symbol/<SYM>@lvl2` of each symbol; every
//!   message and connection event goes to a raw log (`qp-matriks::rawlog`) under
//!   `fixtures/local/` (real data never leaves the machine, K23). Prints only counts.
//! - `qp live inspect`: aggregate statistics of a raw log (field presence, time-field
//!   formats, arrival lags, session phases); no market values are printed.
//!
//! Connection values come from the environment only (`scripts/live.sh` loads
//! `.env.local`); nothing here prints a secret or an endpoint address.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use clap::{Args, Subcommand};
use qp_live::{
    AuthClient, AuthPolicy, ConnState, ConnectOptions, LiveConfig, LiveConnection, LiveEvent,
    LiveMessage, LiveSink, Notice, NoticeClassifier,
};
use qp_matriks::{RawReader, RawRecord, RawWriter, RecordKind};

mod compare;
mod inspect;
mod record;
mod rest;

pub(crate) use compare::*;
pub(crate) use inspect::*;
pub(crate) use record::*;
pub(crate) use rest::*;

/// `qp live …`.
#[derive(Debug, Subcommand)]
pub enum LiveCommand {
    /// Connect once (environment variables, `docs/DATA_ACCESS.md`), subscribe the clock
    /// and the price cards of `--symbols`, record every raw message under
    /// `fixtures/local/live/raw/` and print counts. Ends after `--seconds` or on halt.
    Record(RecordArgs),
    /// Fetch the symbol list (REST meta) into `fixtures/local/live/meta.json` and write
    /// universe lists `u30.txt` (BIST30), `u100.txt` (BIST100), `utum.txt` (BISTTUM) and
    /// `uall.txt` (all BIST equities); prints only counts.
    Symbols,
    /// Fetch settled REST bars into `fixtures/local/live/rest/` (sequential, paced).
    Bars {
        /// Comma-separated symbols or `@file`.
        #[arg(long)]
        symbols: String,
        /// REST period (`1min`, `5min`, `1hour`, `1day`, `15min`, …).
        #[arg(long)]
        period: String,
        /// First local date `YYYY-MM-DD`.
        #[arg(long)]
        start: String,
        /// Last local date (default: `start`).
        #[arg(long)]
        end: Option<String>,
        /// Unadjusted prices (`divide=false`).
        #[arg(long)]
        unadjusted: bool,
    },
    /// (RP4/T1) Fetch historical trades (`trade.gz`, level 1 only) into
    /// `fixtures/local/trades/<SYM>/<start>_<end>.csv.gz`, in windows of at most
    /// `--window-days` calendar days (≤ 15, the endpoint's limit). A window already in the
    /// cache is never fetched again; every request is appended to
    /// `fixtures/local/trades/requests.jsonl` (counts only).
    Trades {
        /// Comma-separated symbols or `@file`.
        #[arg(long)]
        symbols: String,
        /// First local date `YYYY-MM-DD`.
        #[arg(long)]
        start: String,
        /// Last local date.
        #[arg(long)]
        end: String,
        /// Window length in calendar days (1–15).
        #[arg(long, default_value_t = 14)]
        window_days: u32,
        /// Special (reported) trades (`trade_ex`) into `fixtures/local/trades_ex/` instead.
        #[arg(long)]
        ex: bool,
    },
    /// M22: build bars from raw logs with the live feed and compare them with REST bars
    /// (`qp live bars` files); prints only an aggregate table.
    Compare(CompareArgs),
    /// (RP4/T2, M27) Bars from the cached trades through the live bar builder
    /// (`BarMethod::Trades`) against settled REST bars; with `--raw`, also the card /
    /// trades / mixed source table. Prints only an aggregate table.
    CompareTrades(crate::live_trades::CompareTradesArgs),
    /// (RP4/T2) The price card's `priceStep` against the BIST step table, and trade prices
    /// on that grid (aggregate counts only).
    PriceSteps(crate::live_trades::PriceStepsArgs),
    /// (RP4/T2) Bars from the cached trades (M27 rules) as RP4 harness series
    /// (`fixtures/local/rp4/tradebars/full/<period>/<SYM>.bars`, `manifest.json`), with an
    /// optional check against the harness REST series (aggregate counts only).
    ExportTradeBars(crate::live_trades::ExportTradeBarsArgs),
    /// Aggregate statistics of a raw log (no market values).
    Inspect {
        /// Raw log file (`*.qpraw`).
        file: PathBuf,
    },
    /// (RP4/T5, K64) A person's reset of an endpoint halt — a multi-connect notice or
    /// unexplained server closes (A52) — after they checked why. Removes
    /// `fixtures/local/live-<key>.halted` only with a non-empty `--note`, which goes with
    /// the time into `live-<key>.halted.log`. Asks to type the key on a terminal; without
    /// one it needs `--i-am-human`. An agent never runs this on its own
    /// (`docs/matriks/LIVE.md`). No network.
    Reset(ResetArgs),
}

/// `qp live reset`.
#[derive(Debug, Args)]
pub struct ResetArgs {
    /// Endpoint key as the halt names it (e.g. `delayed-market`).
    #[arg(long)]
    pub endpoint: String,
    /// Who resets and what was checked (required, non-empty).
    #[arg(long, default_value = "")]
    pub note: String,
    /// Confirms a person runs this when there is no terminal to ask on.
    #[arg(long)]
    pub i_am_human: bool,
}

/// `qp live record`.
#[derive(Debug, Args)]
pub struct RecordArgs {
    /// Comma-separated symbols (e.g. `THYAO,GARAN,ASELS`), or `@file` (one per line).
    #[arg(long)]
    pub symbols: String,
    /// How long to stay connected.
    #[arg(long, default_value_t = 60)]
    pub seconds: u64,
    /// Output raw log; default `fixtures/local/live/raw/<UTC time>.qpraw`.
    #[arg(long)]
    pub out: Option<PathBuf>,
    /// Also subscribe `mx/messenger` (broker notices).
    #[arg(long)]
    pub messenger: bool,
    /// Topic suffix: for the price card `@lvl2` (default) or empty for the bare topic;
    /// with `--trades` empty (default, level 1) or `@lvl0` — `@lvl2` is refused there.
    #[arg(long)]
    pub level: Option<String>,
    /// (RP4, K66) Connect to the trade endpoint (`/trade`, its own lock) instead of the
    /// market endpoint and subscribe `mx/trade/<SYM>` (level 1, no suffix). The raw log
    /// (`<UTC time>-trade.qpraw`) keeps no buyer / seller field.
    #[arg(long)]
    pub trades: bool,
    /// (RP4) With `--trades`: also the special (reported) trades `mx/tradeex/<SYM>`.
    #[arg(long, requires = "trades")]
    pub tradeex: bool,
    /// Print a status line every this many seconds.
    #[arg(long, default_value_t = 10)]
    pub every: u64,
}

pub fn run(cmd: &LiveCommand) -> Result<(), String> {
    match cmd {
        LiveCommand::Record(a) => record(a),
        LiveCommand::Inspect { file } => inspect(file),
        LiveCommand::Reset(a) => reset(a, &qp_live::lock::default_lock_dir()),
        LiveCommand::Compare(a) => compare(a),
        LiveCommand::CompareTrades(a) => crate::live_trades::compare_trades(a),
        LiveCommand::PriceSteps(a) => crate::live_trades::price_steps(a),
        LiveCommand::ExportTradeBars(a) => crate::live_trades::export_trade_bars(a),
        LiveCommand::Symbols => symbols(),
        LiveCommand::Trades {
            symbols,
            start,
            end,
            window_days,
            ex,
        } => trades(symbols, start, end, *window_days, *ex),
        LiveCommand::Bars {
            symbols,
            period,
            start,
            end,
            unadjusted,
        } => bars(
            symbols,
            period,
            start,
            end.as_deref().unwrap_or(start),
            *unadjusted,
        ),
    }
}

/// `qp live reset` against the marker directory `dir`.
fn reset(a: &ResetArgs, dir: &Path) -> Result<(), String> {
    use std::io::{BufRead as _, IsTerminal as _, Write as _};
    let key = qp_live::EndpointKey::new(&a.endpoint);
    if a.note.trim().is_empty() {
        return Err(format!(
            "reset {key}: --note is required (who resets, what was checked); nothing changed"
        ));
    }
    let Some(reason) = qp_live::halt::halt_status(dir, &key) else {
        println!(
            "reset {key}: not halted ({} absent); nothing changed",
            key.halt_marker(dir).display()
        );
        return Ok(());
    };
    println!("{key} halted: {reason}");
    if !a.i_am_human {
        let stdin = std::io::stdin();
        if !stdin.is_terminal() {
            return Err(format!(
                "reset {key}: no terminal to confirm on; a person adds --i-am-human \
                 (an agent never resets a halt on its own); nothing changed"
            ));
        }
        print!("Type the endpoint key `{key}` to confirm that you, a person, reset it: ");
        let _ = std::io::stdout().flush();
        let mut line = String::new();
        stdin
            .lock()
            .read_line(&mut line)
            .map_err(|e| e.to_string())?;
        if line.trim() != key.as_str() {
            return Err(format!("reset {key}: not confirmed; nothing changed"));
        }
    }
    match qp_live::halt::reset_halt(dir, &key, &a.note) {
        Ok(_) => {
            println!(
                "reset {key}: halt lifted; note logged in {}",
                qp_live::halt::halt_log(dir, &key).display()
            );
            Ok(())
        }
        Err(e) => Err(format!("reset {key}: {e}; nothing changed")),
    }
}

/// Repository root: the directory holding `fixtures/` (walks up from the current dir).
pub(crate) fn repo_root() -> PathBuf {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut d = cwd.as_path();
    loop {
        if d.join("fixtures").is_dir() && d.join("Cargo.toml").is_file() {
            return d.to_path_buf();
        }
        match d.parent() {
            Some(p) => d = p,
            None => return cwd,
        }
    }
}

pub(crate) fn parse_symbols(s: &str) -> Result<Vec<String>, String> {
    let text = match s.strip_prefix('@') {
        Some(path) => std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?,
        None => s.replace(',', "\n"),
    };
    let mut out: Vec<String> = text
        .lines()
        .map(|l| {
            l.split('#')
                .next()
                .unwrap_or("")
                .trim()
                .to_ascii_uppercase()
        })
        .filter(|l| !l.is_empty())
        .collect();
    out.dedup();
    if out.is_empty() {
        return Err("no symbols".to_owned());
    }
    if let Some(bad) = out.iter().find(|s| {
        !s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_')
    }) {
        return Err(format!("symbol `{bad}`: only letters, digits, `.`, `_`"));
    }
    Ok(out)
}

pub(crate) fn utc_stamp(us: i64) -> String {
    let s = us.div_euclid(1_000_000);
    let day = s.div_euclid(86_400);
    let (y, m, d) = qp_calendar::civil_from_days(day);
    let r = s.rem_euclid(86_400);
    format!(
        "{y:04}{m:02}{d:02}T{:02}{:02}{:02}Z",
        r / 3600,
        (r / 60) % 60,
        r % 60
    )
}

pub(crate) fn now_us() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| i64::try_from(d.as_micros()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

/// (R6 audit B1/B2) The machine-wide "Matriks identity" lock: every `qp` process that
/// requests a token (record, symbols, bars, demo --live) takes it first and holds it for its
/// whole life, so no second process can ask for a token with the same identity while a
/// live connection is open. The directory is `qp_live::lock::default_lock_dir()` (the main
/// tree's `fixtures/local`, also from a worktree; `QP_LIVE_LOCK_DIR` overrides), never the
/// current working root.
pub(crate) fn identity_lock() -> Result<qp_live::lock::MachineLock, String> {
    let dir = qp_live::lock::default_lock_dir();
    qp_live::lock::acquire_machine_lock(&dir, &qp_live::EndpointKey::new("identity"))
        .map_err(|e| format!("live: {e} (another qp process holds the Matriks identity)"))
}

/// The classifier wired into the connection: Matriks `MessengerMessage` decoding.
pub(crate) fn classifier() -> NoticeClassifier {
    Arc::new(|topic: &str, payload: &[u8]| {
        let n = qp_matriks::classify_notice(topic, payload)?;
        Some(if n.is_multi_connect() {
            Notice::MultiConnect
        } else {
            Notice::Other(format!("{n:?}"))
        })
    })
}

/// Whether `topic` is a trade topic (`mx/trade/…`, `mx/tradeex/…`, also mirrored).
#[must_use]
pub(crate) fn is_trade_topic(topic: &str) -> bool {
    // The strict parser, and a prefix check as a floor: any topic naming a trade stream
    // (also inside a mirror, or in a form the parser does not know) is treated as a trade,
    // so its payload is sanitized rather than kept as is (security review, RP4).
    let t = topic.trim();
    matches!(
        qp_matriks::parse_topic(t).unwrap_mirror(),
        qp_matriks::Topic::Trade { .. }
    ) || t.starts_with("mx/trade")
        || t.contains("/mx/trade")
}

/// (RP4, K66) What may be kept of a received payload: a trade payload is re-encoded with
/// level-1 fields only (`sanitize_trade_payload`: buyer and seller dropped); any other
/// payload is unchanged. A trade payload that does not decode is kept as nothing (empty):
/// its bytes might hold the level-2 fields. Returns the payload and, for a trade, whether
/// buyer / seller were on the wire (tags only; their values are never read).
#[must_use]
///
/// With `trade_endpoint`, every payload received on the trade endpoint is treated as a
/// trade whatever its topic (sanitized, or kept as nothing when it does not decode).
pub(crate) fn keepable_payload_on<'a>(
    topic: &str,
    payload: &'a [u8],
    trade_endpoint: bool,
) -> (std::borrow::Cow<'a, [u8]>, Option<(bool, bool)>) {
    use std::borrow::Cow;
    if !trade_endpoint && !is_trade_topic(topic) {
        return (Cow::Borrowed(payload), None);
    }
    let l2 = qp_matriks::level2_presence(payload);
    match qp_matriks::sanitize_trade_payload(payload) {
        Ok(clean) => (Cow::Owned(clean), Some(l2.unwrap_or((false, false)))),
        Err(_) => (Cow::Owned(Vec::new()), None),
    }
}

/// Prefix of the raw log note with the trade sanitizing counts (`qp live inspect`).
pub(crate) const TRADE_SANITIZE_NOTE: &str = "trade-sanitize";

/// Live feed settings shared by `qp replay --raw` and `qp live compare` (R6/T5).
#[derive(Debug, Clone, Args)]
pub struct FeedArgs {
    /// Bar intervals (comma-separated).
    #[arg(long, default_value = "1m,5m,1h,1d")]
    pub intervals: String,
    /// Bar method: `ticks`, `card` (card day fields on `1d`, ticks elsewhere) or `mixed`.
    #[arg(long, default_value = "ticks")]
    pub method: String,
    /// Volume counters: `daily` (`dailyVolume`/`dailyQuantity`) or `volume`
    /// (`volume`/`quantity`).
    #[arg(long, default_value = "daily")]
    pub volume_field: String,
    /// Volume unit: `value` (TL, like REST `volume`) or `quantity` (lots).
    #[arg(long, default_value = "value")]
    pub volume_unit: String,
    /// Tick time: `update_date` (exchange time from `updateDate`) or `arrival`.
    #[arg(long, default_value = "update_date")]
    pub time: String,
    /// Bars are closed this long after the clock passes their end (ms).
    #[arg(long, default_value_t = qp_matriks::feed::DEFAULT_FINALIZE_GRACE_MS)]
    pub finalize_grace_ms: i64,
}

#[cfg(test)]
mod trade_keep_tests {
    use super::*;

    #[test]
    fn trade_endpoint_sanitizes_every_payload_whatever_the_topic() {
        // Not a trade message: on the trade endpoint it is kept as nothing, never as is.
        let junk = b"\x3a\x03XYZ".to_vec();
        let (kept, _) = keepable_payload_on("odd/topic", &junk, true);
        assert!(kept.is_empty() || kept.as_ref() != junk.as_slice());
        // Off the trade endpoint a non-trade topic is unchanged.
        let (kept, l2) = keepable_payload_on("mx/symbol/AAA@lvl2", &junk, false);
        assert_eq!(kept.as_ref(), junk.as_slice());
        assert!(l2.is_none());
        // Prefix floor: unknown trade-like forms count as trades.
        assert!(is_trade_topic("mx/tradeex/AAA"));
        assert!(is_trade_topic("mx/user/x/mx/trade/AAA"));
        assert!(is_trade_topic("mx/trade/AAA@weird"));
        assert!(!is_trade_topic("mx/symbol/AAA@lvl2"));
    }
}

pub(crate) fn parse_method(s: &str) -> Result<qp_source::BarMethod, String> {
    use qp_source::BarMethod;
    Ok(match s {
        "ticks" => BarMethod::Ticks,
        "card" | "card_daily" => BarMethod::CardDaily,
        "mixed" => BarMethod::Mixed,
        _ => return Err(format!("method `{s}`: expected ticks, card or mixed")),
    })
}

pub(crate) fn parse_volume_field(s: &str) -> Result<qp_matriks::VolumeField, String> {
    match s {
        "daily" | "daily_volume" => Ok(qp_matriks::VolumeField::DailyVolume),
        "volume" => Ok(qp_matriks::VolumeField::Volume),
        _ => Err(format!("volume field `{s}`: expected daily or volume")),
    }
}

pub(crate) fn split_list(s: &str) -> Vec<String> {
    s.split(',')
        .map(str::trim)
        .filter(|x| !x.is_empty())
        .map(str::to_owned)
        .collect()
}

impl FeedArgs {
    /// The feed configuration.
    pub fn config(&self) -> Result<qp_matriks::LiveFeedConfig, String> {
        let intervals = split_list(&self.intervals);
        for iv in &intervals {
            qp_calendar::Interval::parse(iv).map_err(|e| format!("interval `{iv}`: {e}"))?;
        }
        Ok(qp_matriks::LiveFeedConfig {
            intervals,
            method: parse_method(&self.method)?,
            volume: match self.volume_unit.as_str() {
                "value" => qp_source::VolumeUnit::Value,
                "quantity" => qp_source::VolumeUnit::Quantity,
                s => return Err(format!("volume unit `{s}`: expected value or quantity")),
            },
            volume_field: parse_volume_field(&self.volume_field)?,
            time: match self.time.as_str() {
                "update_date" | "updatedate" => qp_matriks::TimeSource::UpdateDate,
                "arrival" => qp_matriks::TimeSource::Arrival,
                s => return Err(format!("time `{s}`: expected update_date or arrival")),
            },
            feed_name: None,
        })
    }
}

impl FeedArgs {
    /// A live feed with these settings on the XIST calendar.
    pub fn build(&self) -> Result<qp_matriks::LiveFeed, String> {
        let mut feed = qp_matriks::LiveFeed::new(self.config()?, xist()?);
        feed.set_finalize_grace(self.finalize_grace_ms);
        Ok(feed)
    }
}

/// The XIST exchange calendar.
pub(crate) fn xist() -> Result<qp_calendar::ExchangeCalendar, String> {
    let cal = qp_calendar::builtin("XIST").ok_or("no XIST calendar")?;
    qp_calendar::ExchangeCalendar::new(&cal).map_err(|e| e.to_string())
}

/// JSON lines of one [`qp_matriks::FeedOut`], in a fixed order: connection, phases, data
/// events, clock, final bars.
pub(crate) fn feed_out_lines(out: &qp_matriks::FeedOut) -> Vec<String> {
    use serde_json::json;
    let mut lines = Vec::new();
    match out.connection {
        Some(qp_matriks::ConnChange::Opened { session }) => {
            lines.push(json!({"conn": {"opened": {"session": session}}}).to_string());
        }
        Some(qp_matriks::ConnChange::Lost) => lines.push(json!({"conn": "lost"}).to_string()),
        None => {}
    }
    for (symbol, code) in &out.phases {
        lines.push(json!({"phase": {"symbol": symbol, "code": code}}).to_string());
    }
    for e in &out.events {
        lines.push(json!({ "data": e }).to_string());
    }
    if let Some(c) = out.clock_ms {
        lines.push(json!({ "clock": c }).to_string());
    }
    for (feed, t) in &out.finalized {
        lines.push(json!({"final": {"feed": feed, "t": t}}).to_string());
    }
    lines
}

/// Warns (stderr) when a raw log is outside `fixtures/local/` (real data belongs there).
fn warn_outside_local(path: &Path) {
    let abs = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    if !abs
        .components()
        .collect::<Vec<_>>()
        .windows(2)
        .any(|w| w[0].as_os_str() == "fixtures" && w[1].as_os_str() == "local")
    {
        eprintln!(
            "warning: {} is not under fixtures/local/ (raw logs with real data belong there)",
            path.display()
        );
    }
}

/// `qp replay --raw`: the raw log through [`qp_matriks::LiveFeed`], one JSON line per
/// output. Same records → byte-equal output.
pub fn replay_raw(
    path: &Path,
    args: &FeedArgs,
    out: Option<&Path>,
    stats: bool,
) -> Result<(), String> {
    use std::io::Write;
    warn_outside_local(path);
    let mut feed = args.build()?;
    let f = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut reader = RawReader::new(BufReader::new(f)).map_err(|e| e.to_string())?;
    let mut w: Box<dyn Write> = match out {
        Some(p) => Box::new(BufWriter::new(
            File::create(p).map_err(|e| format!("{}: {e}", p.display()))?,
        )),
        None => Box::new(BufWriter::new(std::io::stdout().lock())),
    };
    let mut tail = None;
    loop {
        match reader.next_record() {
            Ok(Some(rec)) => {
                for line in feed_out_lines(&feed.push(&rec)) {
                    writeln!(w, "{line}").map_err(|e| e.to_string())?;
                }
            }
            Ok(None) => break,
            Err(e) => {
                tail = Some(e.to_string());
                break;
            }
        }
    }
    w.flush().map_err(|e| e.to_string())?;
    if let Some(t) = tail {
        eprintln!("raw log ends early ({t}); records before it were replayed");
    }
    if stats {
        eprintln!("{:#?}", feed.stats());
    }
    Ok(())
}

/// `qp live compare` (M22).
#[derive(Debug, Args)]
pub struct CompareArgs {
    /// Raw logs, comma-separated, replayed in this order as one recording (a file after
    /// the first starts with a connection loss unless the previous one ended with one).
    #[arg(long)]
    pub raw: String,
    /// REST bars directory (`<SYM>.<period>.<start>.<end>.<adj|unadj>.json`).
    #[arg(long, default_value = "fixtures/local/live/rest")]
    pub rest_dir: PathBuf,
    /// Intervals (`1m`, `5m`, `15m`, `30m`, `1h`, `1d`).
    #[arg(long, default_value = "1m,5m,1h,1d")]
    pub intervals: String,
    /// Methods: `ticks`, `card`, `mixed` (`card` is reported on `1d` only: intraday it is
    /// `ticks`).
    #[arg(long, default_value = "ticks,card,mixed")]
    pub methods: String,
    /// Volume fields: `daily`, `volume`.
    #[arg(long, default_value = "daily,volume")]
    pub volume_fields: String,
    /// Volume unit: `value` (REST `volume`) or `quantity` (REST `quantity`).
    #[arg(long, default_value = "value")]
    pub volume_unit: String,
    /// Tick time: `update_date` or `arrival`.
    #[arg(long, default_value = "update_date")]
    pub time: String,
    /// Finalize grace (ms).
    #[arg(long, default_value_t = qp_matriks::feed::DEFAULT_FINALIZE_GRACE_MS)]
    pub finalize_grace_ms: i64,
    /// Absolute price tolerance for the "within tolerance" columns.
    #[arg(long, default_value_t = 1e-6)]
    pub price_tol: f64,
    /// Relative volume tolerance for the "within tolerance" columns.
    #[arg(long, default_value_t = 0.01)]
    pub volume_tol: f64,
    /// Prefer adjusted REST files (default: unadjusted, falling back to adjusted).
    #[arg(long)]
    pub adjusted: bool,
}

#[cfg(test)]
pub(crate) mod tests;
