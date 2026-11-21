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
pub fn repo_root() -> PathBuf {
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

pub fn parse_symbols(s: &str) -> Result<Vec<String>, String> {
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

pub fn utc_stamp(us: i64) -> String {
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

pub fn now_us() -> i64 {
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
pub fn identity_lock() -> Result<qp_live::lock::MachineLock, String> {
    let dir = qp_live::lock::default_lock_dir();
    qp_live::lock::acquire_machine_lock(&dir, &qp_live::EndpointKey::new("identity"))
        .map_err(|e| format!("live: {e} (another qp process holds the Matriks identity)"))
}

/// The classifier wired into the connection: Matriks `MessengerMessage` decoding.
pub fn classifier() -> NoticeClassifier {
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
pub fn is_trade_topic(topic: &str) -> bool {
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
pub fn keepable_payload_on<'a>(
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
pub const TRADE_SANITIZE_NOTE: &str = "trade-sanitize";

/// A sink that appends every message and event to a raw log and counts. Trade payloads are
/// written without buyer and seller ([`keepable_payload_on`]).
pub struct RawSink {
    w: Mutex<RawWriter<BufWriter<File>>>,
    pub messages: AtomicU64,
    pub bytes: AtomicU64,
    pub events: Mutex<Vec<String>>,
    pub errors: AtomicU64,
    /// Trade messages (sanitized).
    pub trade_messages: AtomicU64,
    /// Trade messages that carried a buyer / a seller field on the wire (dropped).
    pub level2_buyer: AtomicU64,
    pub level2_seller: AtomicU64,
    /// Trade payloads that did not decode (kept empty).
    pub unreadable_trades: AtomicU64,
    finished: std::sync::atomic::AtomicBool,
    /// Sink of the trade endpoint: every payload is sanitized as a trade.
    trade_endpoint: bool,
}

impl RawSink {
    pub fn create(path: &Path) -> Result<Self, String> {
        if let Some(d) = path.parent() {
            std::fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
        }
        let f = File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let w = RawWriter::new(BufWriter::new(f)).map_err(|e| e.to_string())?;
        Ok(Self {
            w: Mutex::new(w),
            messages: AtomicU64::new(0),
            bytes: AtomicU64::new(0),
            events: Mutex::new(Vec::new()),
            errors: AtomicU64::new(0),
            trade_messages: AtomicU64::new(0),
            level2_buyer: AtomicU64::new(0),
            level2_seller: AtomicU64::new(0),
            unreadable_trades: AtomicU64::new(0),
            finished: std::sync::atomic::AtomicBool::new(false),
            trade_endpoint: false,
        })
    }

    /// The sink of a trade-endpoint connection: every received payload goes through the
    /// trade sanitizer, whatever its topic.
    #[must_use]
    pub fn for_trade_endpoint(mut self) -> Self {
        self.trade_endpoint = true;
        self
    }

    /// Writes one received message (trade payloads sanitized, counted) and returns the
    /// payload as kept, for the hub.
    pub fn take_message<'a>(&self, m: &LiveMessage<'a>) -> std::borrow::Cow<'a, [u8]> {
        self.messages.fetch_add(1, Ordering::Relaxed);
        self.bytes
            .fetch_add(m.payload.len() as u64, Ordering::Relaxed);
        let (kept, l2) = keepable_payload_on(m.topic, m.payload, self.trade_endpoint);
        if self.trade_endpoint || is_trade_topic(m.topic) {
            self.trade_messages.fetch_add(1, Ordering::Relaxed);
            match l2 {
                Some((b, s)) => {
                    self.level2_buyer.fetch_add(u64::from(b), Ordering::Relaxed);
                    self.level2_seller
                        .fetch_add(u64::from(s), Ordering::Relaxed);
                }
                None => {
                    self.unreadable_trades.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
        self.put(&RawRecord::message(
            m.received_at_us,
            m.topic,
            kept.as_ref().to_vec(),
        ));
        kept
    }

    /// The trade sanitizing counts (counts only).
    #[must_use]
    pub fn trade_note(&self) -> String {
        format!(
            "{TRADE_SANITIZE_NOTE} messages={} buyer_present={} seller_present={} unreadable={}",
            self.trade_messages.load(Ordering::Relaxed),
            self.level2_buyer.load(Ordering::Relaxed),
            self.level2_seller.load(Ordering::Relaxed),
            self.unreadable_trades.load(Ordering::Relaxed)
        )
    }

    /// Ends the log: the trade sanitizing note (once, if trades were seen), then a flush.
    pub fn finish(&self) {
        if self.trade_messages.load(Ordering::Relaxed) > 0
            && !self.finished.swap(true, Ordering::Relaxed)
        {
            self.put(&RawRecord {
                t_us: now_us(),
                kind: RecordKind::Note,
                topic: String::new(),
                payload: self.trade_note().into_bytes(),
            });
        }
        self.flush();
    }

    fn put(&self, rec: &RawRecord) {
        let mut w = self
            .w
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if w.write(rec).is_err() {
            self.errors.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Appends one record (a local note, e.g. a deliberate drop).
    pub fn put_record(&self, rec: &RawRecord) {
        self.put(rec);
    }

    pub fn flush(&self) {
        let mut w = self
            .w
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _ = w.flush();
    }
}

impl LiveSink for RawSink {
    fn on_message(&self, m: &LiveMessage<'_>) {
        let _ = self.take_message(m);
    }

    fn on_event(&self, _ep: &qp_live::EndpointKey, ev: &LiveEvent) {
        let t = now_us();
        let (kind, topic, note) = match ev {
            LiveEvent::Connected { session } => (
                RecordKind::ConnectionOpened,
                String::new(),
                format!("session {session}"),
            ),
            LiveEvent::Disconnected { .. } => {
                (RecordKind::ConnectionLost, String::new(), ev.to_string())
            }
            LiveEvent::Subscribed { topic } => {
                (RecordKind::Subscribed, topic.clone(), String::new())
            }
            LiveEvent::SubscribeRejected { topic, code } => (
                RecordKind::SubscribeRejected,
                topic.clone(),
                format!("0x{code:02x}"),
            ),
            _ => (RecordKind::Note, String::new(), ev.to_string()),
        };
        self.put(&RawRecord {
            t_us: t,
            kind,
            topic,
            payload: note.into_bytes(),
        });
        if !matches!(ev, LiveEvent::Subscribed { .. }) {
            let mut e = self
                .events
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            e.push(format!("{} {ev}", utc_stamp(t)));
        }
    }
}

/// The topics of `qp live record`: the market endpoint's (clock, notices, price cards) or,
/// with `--trades`, the trade endpoint's (level 1 trade topics; `@lvl2` refused).
///
/// # Errors
/// A bad symbol, or a refused level.
pub fn record_topics(a: &RecordArgs, symbols: &[String]) -> Result<Vec<String>, String> {
    let mut topics = Vec::new();
    if a.trades {
        if a.messenger {
            topics.push("mx/messenger".to_owned());
        }
        for s in symbols {
            let mut streams = vec![qp_matriks::TradeStream::Regular];
            if a.tradeex {
                streams.push(qp_matriks::TradeStream::Special);
            }
            for st in streams {
                let t = qp_matriks::trade_topic_at(st, s, a.level.as_deref())
                    .map_err(|e| format!("--level / --symbols: {e} ({s})"))?;
                topics.push(t);
            }
        }
        if let Some(t) = topics.iter().find(|t| qp_live::is_level2_trade_topic(t)) {
            return Err(format!("refused level-2 trade topic {t}"));
        }
        return Ok(topics);
    }
    topics.push("mx/timestamp".to_owned());
    if a.messenger {
        topics.push("mx/messenger".to_owned());
    }
    let level = a.level.as_deref().unwrap_or("@lvl2");
    topics.extend(symbols.iter().map(|s| format!("mx/symbol/{s}{level}")));
    Ok(topics)
}

fn record(a: &RecordArgs) -> Result<(), String> {
    let symbols = parse_symbols(&a.symbols)?;
    let topics = record_topics(a, &symbols)?;
    let _identity = identity_lock()?;
    let cfg = LiveConfig::from_env().map_err(|e| e.to_string())?;
    if cfg.hardware_id_warning() {
        eprintln!("live: warning: MATRIKS_HARDWARE_ID does not start with `qapsule-`");
    }
    let root = repo_root();
    let start_us = now_us();
    let out = a.out.clone().unwrap_or_else(|| {
        root.join("fixtures/local/live/raw").join(format!(
            "{}{}.qpraw",
            utc_stamp(start_us),
            if a.trades { "-trade" } else { "" }
        ))
    });
    // A relative `--out` is taken from the current directory; `..` is refused so the
    // prefix check below cannot be walked around.
    let out = if out.is_relative() {
        std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(out)
    } else {
        out
    };
    if out
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
        || !out.starts_with(root.join("fixtures/local"))
    {
        return Err("live: the raw log must stay under fixtures/local/ (K23)".to_owned());
    }
    let endpoint = if a.trades {
        cfg.trade_endpoint()
    } else {
        cfg.market_endpoint()
    };
    let sink = if a.trades {
        Arc::new(RawSink::create(&out)?.for_trade_endpoint())
    } else {
        Arc::new(RawSink::create(&out)?)
    };
    let auth = Arc::new(AuthClient::new(&cfg, AuthPolicy::default()));
    let mut opts = ConnectOptions::new(topics.clone());
    opts.classifier = Some(classifier());
    opts.lock_dir = qp_live::lock::default_lock_dir();
    println!(
        "live: mode {:?}, endpoint {}, {} symbols, {} topics, {} s",
        cfg.mode(),
        endpoint.key(),
        symbols.len(),
        topics.len(),
        a.seconds
    );
    let t0 = Instant::now();
    let conn = LiveConnection::open(
        &cfg,
        endpoint,
        auth,
        opts,
        sink.clone() as Arc<dyn LiveSink>,
    )
    .map_err(|e| e.to_string())?;
    let deadline = t0 + Duration::from_secs(a.seconds);
    let mut next = t0 + Duration::from_secs(a.every.max(1));
    let mut seen_events = 0usize;
    let halted = loop {
        std::thread::sleep(Duration::from_millis(200));
        {
            let ev = sink
                .events
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            for e in &ev[seen_events..] {
                println!("live: {e}");
            }
            seen_events = ev.len();
        }
        let st = conn.state();
        if let ConnState::Halted(r) = &st {
            break Some(r.to_string());
        }
        let now = Instant::now();
        if now >= next {
            println!(
                "live: {:>4} s  state {}  messages {}",
                t0.elapsed().as_secs(),
                state_name(&st),
                sink.messages.load(Ordering::Relaxed)
            );
            next = now + Duration::from_secs(a.every.max(1));
        }
        if now >= deadline {
            break None;
        }
    };
    let close = conn.close();
    sink.finish();
    let ev = sink
        .events
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    for e in &ev[seen_events..] {
        println!("live: {e}");
    }
    println!(
        "live: done after {} s: {} messages, {} payload bytes, {} write errors; raw log {}",
        t0.elapsed().as_secs(),
        sink.messages.load(Ordering::Relaxed),
        sink.bytes.load(Ordering::Relaxed),
        sink.errors.load(Ordering::Relaxed),
        out.strip_prefix(&root).unwrap_or(&out).display()
    );
    if a.trades {
        println!(
            "live: trades: {} messages; level-2 fields on the wire (dropped, not kept): \
             buyer {}, seller {}; unreadable {}",
            sink.trade_messages.load(Ordering::Relaxed),
            sink.level2_buyer.load(Ordering::Relaxed),
            sink.level2_seller.load(Ordering::Relaxed),
            sink.unreadable_trades.load(Ordering::Relaxed)
        );
    }
    if let Some(r) = halted {
        return Err(format!(
            "live: HALTED ({r}); not retrying. Tell Arkin before any further live attempt."
        ));
    }
    close.map_err(|e| e.to_string())
}

fn state_name(s: &ConnState) -> String {
    match s {
        ConnState::Starting => "starting".into(),
        ConnState::Connecting { attempt } => format!("connecting#{attempt}"),
        ConnState::Connected { session } => format!("connected#{session}"),
        ConnState::BackingOff { failures } => format!("backoff({failures})"),
        ConnState::Halted(r) => format!("halted({r})"),
        ConnState::Closed => "closed".into(),
    }
}

/// Reads every record of a raw log (a truncated last record ends the read).
pub fn read_all(path: &Path) -> Result<(Vec<RawRecord>, Option<String>), String> {
    let f = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut r = RawReader::new(BufReader::new(f)).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    loop {
        match r.next_record() {
            Ok(Some(rec)) => out.push(rec),
            Ok(None) => return Ok((out, None)),
            Err(e) => return Ok((out, Some(e.to_string()))),
        }
    }
}

fn pct(v: &mut [i64], p: f64) -> i64 {
    if v.is_empty() {
        return 0;
    }
    v.sort_unstable();
    let i = ((v.len() - 1) as f64 * p).round() as usize;
    v[i]
}

/// Shape of a time string without its digits: `2026-11-19T14:02:03.123+03:00` →
/// `dddd-dd-ddTdd:dd:dd.ddd+dd:dd`.
fn shape(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_digit() { 'd' } else { c })
        .collect()
}

/// Parses an ISO-like time (`YYYY-MM-DDTHH:MM:SS[.f][Z|±HH:MM]`) to UTC ms; a time
/// without offset is read as Istanbul (UTC+3). Returns (ms, had_offset).
pub fn parse_iso_ms(s: &str) -> Option<(i64, bool)> {
    let b = s.as_bytes();
    if b.len() < 19 || b[4] != b'-' || b[7] != b'-' || (b[10] != b'T' && b[10] != b' ') {
        return None;
    }
    let num = |a: usize, n: usize| -> Option<i64> { s.get(a..a + n)?.parse().ok() };
    let (y, mo, d, h, mi, se) = (
        num(0, 4)?,
        num(5, 2)?,
        num(8, 2)?,
        num(11, 2)?,
        num(14, 2)?,
        num(17, 2)?,
    );
    let mut i = 19;
    let mut frac_ms = 0i64;
    if b.get(i) == Some(&b'.') {
        let start = i + 1;
        i = start;
        while b.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
        let digits = &s[start..i];
        let mut v: String = digits.chars().take(3).collect();
        while v.len() < 3 {
            v.push('0');
        }
        frac_ms = v.parse().ok()?;
    }
    let rest = &s[i..];
    let (off_min, had) = if rest.is_empty() {
        (180, false)
    } else if rest == "Z" {
        (0, true)
    } else {
        let sign = match rest.as_bytes()[0] {
            b'+' => 1,
            b'-' => -1,
            _ => return None,
        };
        let r = rest[1..].replace(':', "");
        let oh: i64 = r.get(0..2)?.parse().ok()?;
        let om: i64 = r.get(2..4).unwrap_or("00").parse().ok()?;
        (sign * (oh * 60 + om), true)
    };
    let day = qp_calendar::days_from_civil(y, u32::try_from(mo).ok()?, u32::try_from(d).ok()?);
    let ms = ((day * 86_400 + h * 3600 + mi * 60 + se) - off_min * 60) * 1000 + frac_ms;
    Some((ms, had))
}

/// (RP4) Aggregate statistics of the trade messages of a raw log (no values: counts,
/// field presence, time-unit estimate, arrival lags, the side value set).
#[derive(Debug, Default, Clone, PartialEq)]
pub struct TradeInspect {
    /// `mx/trade/…` messages.
    pub regular: u64,
    /// `mx/tradeex/…` messages.
    pub special: u64,
    /// Messages per symbol.
    pub per_symbol: BTreeMap<String, u64>,
    /// Payloads that are not a `TradeMessage` (or were kept empty).
    pub decode_errors: u64,
    /// Payload symbol differs from the topic's.
    pub symbol_mismatch: u64,
    /// Messages carrying each field (wire tags; buyer / seller: presence only).
    pub fields: BTreeMap<&'static str, u64>,
    /// `TradeTimeUnit::Auto`'s pick per message (arrival as reference).
    pub auto_unit: BTreeMap<&'static str, u64>,
    /// Messages whose `timestamp` is a valid since-midnight nanosecond value.
    pub since_midnight_ns: u64,
    /// Messages without `timestamp`.
    pub no_timestamp: u64,
    /// Arrival − trade time (ms) with the `Auto` reading.
    pub lag_auto_ms: Vec<i64>,
    /// Arrival − trade time (ms) read as since-midnight ns (documented unit).
    pub lag_since_midnight_ms: Vec<i64>,
    /// `activeBidOrAsk` values as sent (at most 8 characters) and their counts.
    pub sides: BTreeMap<String, u64>,
    /// `isTradeEx = true`.
    pub trade_ex_flag: u64,
    /// Trade numbers seen twice for the same symbol.
    pub repeated_trade_no: u64,
    /// The recorder's sanitizing notes (`trade-sanitize …`).
    pub notes: Vec<String>,
}

impl TradeInspect {
    /// Trade messages.
    #[must_use]
    pub fn messages(&self) -> u64 {
        self.regular + self.special
    }
}

/// (RP4) [`TradeInspect`] of raw records.
#[must_use]
pub fn inspect_trades(recs: &[RawRecord]) -> TradeInspect {
    use qp_matriks::trade::resolve_trade_time;
    use qp_matriks::{
        decode_trade_raw, parse_topic, Topic, TradeDecodeConfig, TradeTimeUnit, TRADE_FIELDS,
    };
    let mut out = TradeInspect::default();
    let mut seen_no: std::collections::BTreeSet<(String, String)> =
        std::collections::BTreeSet::new();
    let auto = TradeDecodeConfig {
        unit: TradeTimeUnit::Auto,
        ..TradeDecodeConfig::default()
    };
    let midnight = TradeDecodeConfig::default();
    for r in recs {
        if r.kind == RecordKind::Note {
            let n = String::from_utf8_lossy(&r.payload);
            if n.starts_with(TRADE_SANITIZE_NOTE) {
                out.notes.push(n.into_owned());
            }
            continue;
        }
        if r.kind != RecordKind::Message {
            continue;
        }
        let Topic::Trade {
            symbol, special, ..
        } = parse_topic(&r.topic).unwrap_mirror().clone()
        else {
            continue;
        };
        if special {
            out.special += 1;
        } else {
            out.regular += 1;
        }
        *out.per_symbol.entry(symbol.clone()).or_default() += 1;
        let Ok(tags) = qp_matriks::wire::field_numbers(&r.payload) else {
            out.decode_errors += 1;
            continue;
        };
        let Ok(v) = decode_trade_raw(&r.payload) else {
            out.decode_errors += 1;
            continue;
        };
        if r.payload.is_empty() {
            out.decode_errors += 1;
            continue;
        }
        for (n, name) in TRADE_FIELDS {
            if tags.contains(&n) {
                *out.fields.entry(name).or_default() += 1;
            }
        }
        if v.symbol != symbol {
            out.symbol_mismatch += 1;
        }
        if let Some(sd) = &v.side {
            *out.sides
                .entry(sd.trim().chars().take(8).collect())
                .or_default() += 1;
        }
        if v.is_trade_ex == Some(true) {
            out.trade_ex_flag += 1;
        }
        // Repeats of a trade number (kept in memory only, never printed).
        if let Some(no) = v.order_no.clone() {
            if !seen_no.insert((symbol.clone(), no)) {
                out.repeated_trade_no += 1;
            }
        }
        let arrival_ms = r.t_us.div_euclid(1000);
        let Some(raw) = v.timestamp else {
            out.no_timestamp += 1;
            continue;
        };
        if let Some((ns, u)) = resolve_trade_time(raw, &auto, arrival_ms) {
            *out.auto_unit.entry(u.name()).or_default() += 1;
            out.lag_auto_ms.push(arrival_ms - ns.div_euclid(1_000_000));
        }
        if let Some((ns, _)) = resolve_trade_time(raw, &midnight, arrival_ms) {
            out.since_midnight_ns += 1;
            out.lag_since_midnight_ms
                .push(arrival_ms - ns.div_euclid(1_000_000));
        }
    }
    out
}

fn print_trade_inspect(t: &mut TradeInspect) {
    let total = t.messages();
    println!(
        "trades: {total} messages ({} mx/trade, {} mx/tradeex), {} symbols, {} decode errors, \
         {} symbol mismatches",
        t.regular,
        t.special,
        t.per_symbol.len(),
        t.decode_errors,
        t.symbol_mismatch
    );
    let mut per: Vec<i64> = t
        .per_symbol
        .values()
        .map(|&n| i64::try_from(n).unwrap_or(i64::MAX))
        .collect();
    let (mn, md, mx) = (
        per.iter().min().copied().unwrap_or(0),
        pct(&mut per, 0.5),
        per.iter().max().copied().unwrap_or(0),
    );
    println!("trades per symbol: min {mn}, p50 {md}, max {mx}");
    println!("trade fields present (messages):");
    for (_, name) in qp_matriks::TRADE_FIELDS {
        let c = t.fields.get(name).copied().unwrap_or(0);
        let tag = if matches!(name, "buyer" | "seller") {
            " (level 2: presence in the kept log; must be 0)"
        } else {
            ""
        };
        println!("  {name}: {c}{tag}");
    }
    for n in &t.notes {
        println!("recorder: {n} (level-2 fields seen on the wire and dropped; counts only)");
    }
    println!(
        "trade time: auto unit picks {:?}; valid as ns since local midnight: {} of {}; \
         no timestamp: {}",
        t.auto_unit,
        t.since_midnight_ns,
        total - t.decode_errors,
        t.no_timestamp
    );
    for (label, v) in [
        ("auto", &mut t.lag_auto_ms),
        ("ns_from_midnight", &mut t.lag_since_midnight_ms),
    ] {
        let n = v.len();
        let (a, b) = (pct(v, 0.5), pct(v, 0.95));
        println!(
            "arrival - trade time [{label}] (ms): n {n}, min {}, p50 {a}, p95 {b}, max {}",
            v.first().copied().unwrap_or(0),
            v.last().copied().unwrap_or(0)
        );
    }
    println!("activeBidOrAsk values (count): {:?}", t.sides);
    println!(
        "isTradeEx=true: {}; repeated trade numbers: {}",
        t.trade_ex_flag, t.repeated_trade_no
    );
}

fn inspect(path: &Path) -> Result<(), String> {
    use qp_matriks::{parse_topic, SymbolBook, Topic, UpdateKind};
    let (recs, tail) = read_all(path)?;
    let mut kinds: BTreeMap<&'static str, u64> = BTreeMap::new();
    let mut book = SymbolBook::new();
    let mut per_symbol: BTreeMap<String, (u64, u64)> = BTreeMap::new();
    let mut field_upd: BTreeMap<&'static str, u64> = BTreeMap::new();
    let mut field_init: BTreeMap<&'static str, u64> = BTreeMap::new();
    let mut shapes: BTreeMap<String, u64> = BTreeMap::new();
    let mut lag_upd: Vec<i64> = Vec::new();
    let mut lag_time: BTreeMap<String, Vec<i64>> = BTreeMap::new();
    let mut phases: BTreeMap<i32, u64> = BTreeMap::new();
    let mut sources: BTreeMap<String, u64> = BTreeMap::new();
    let mut clock = qp_matriks::MarketClock::new();
    let mut clock_events: Vec<String> = Vec::new();
    let mut decode_err = 0u64;
    let mut pub_reason: BTreeMap<i32, u64> = BTreeMap::new();
    let (mut first, mut last) = (i64::MAX, i64::MIN);
    for r in &recs {
        first = first.min(r.t_us);
        last = last.max(r.t_us);
        let k = match r.kind {
            RecordKind::Message => "message",
            RecordKind::ConnectionOpened => "connection_opened",
            RecordKind::ConnectionLost => "connection_lost",
            RecordKind::Subscribed => "subscribed",
            RecordKind::SubscribeRejected => "subscribe_rejected",
            RecordKind::Note => "note",
        };
        *kinds.entry(k).or_default() += 1;
        if r.kind == RecordKind::Subscribed {
            if let Topic::Symbol { symbol, .. } = parse_topic(&r.topic) {
                book.resubscribed(&symbol);
            }
        }
        if r.kind != RecordKind::Message {
            continue;
        }
        let t = parse_topic(&r.topic);
        match t.unwrap_mirror() {
            Topic::Symbol { symbol, .. } => match book.apply(symbol, &r.payload) {
                Ok(u) => {
                    let e = per_symbol.entry(symbol.clone()).or_default();
                    let target = if u.kind == UpdateKind::Initial {
                        e.0 += 1;
                        &mut field_init
                    } else {
                        e.1 += 1;
                        &mut field_upd
                    };
                    for n in u.fields.names() {
                        *target.entry(n).or_default() += 1;
                    }
                    if let Some(p) = u.stock_status {
                        *phases.entry(p).or_default() += 1;
                    }
                    if let Some(p) = u.publish_reason {
                        *pub_reason.entry(p).or_default() += 1;
                    }
                    if u.fields.contains_name("updateDate") {
                        if let Some(ud) = book.state(symbol).and_then(|s| s.update_date.clone()) {
                            *shapes.entry(shape(&ud)).or_default() += 1;
                            if let Some((ms, _)) = parse_iso_ms(&ud) {
                                lag_upd.push(r.t_us / 1000 - ms);
                            }
                        }
                    }
                }
                Err(_) => decode_err += 1,
            },
            Topic::Timestamp => match qp_matriks::decode_time(&r.payload) {
                Ok(tt) => {
                    *sources.entry(tt.source.clone()).or_default() += 1;
                    lag_time
                        .entry(tt.source.clone())
                        .or_default()
                        .push(r.t_us / 1000 - tt.millis());
                    for ev in clock.apply(&tt) {
                        clock_events.push(format!("{} {ev:?}", utc_stamp(r.t_us)));
                    }
                }
                Err(_) => decode_err += 1,
            },
            _ => {}
        }
    }
    println!("records: {} ({kinds:?})", recs.len());
    if let Some(t) = tail {
        println!("tail: {t}");
    }
    if first <= last {
        println!(
            "span: {} .. {} ({} s)",
            utc_stamp(first),
            utc_stamp(last),
            (last - first) / 1_000_000
        );
    }
    println!("decode errors: {decode_err}");
    println!("symbols: {} (initial, updates):", per_symbol.len());
    for (s, (i, u)) in &per_symbol {
        println!("  {s}: {i} initial, {u} updates");
    }
    println!("fields in initial messages (count):");
    for (n, c) in &field_init {
        println!("  {n}: {c}");
    }
    println!("fields in update messages (count):");
    let mut fu: Vec<_> = field_upd.iter().collect();
    fu.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    for (n, c) in fu {
        println!("  {n}: {c}");
    }
    println!("updateDate shapes: {shapes:?}");
    let n = lag_upd.len();
    let (p50, p95, mn, mx) = (
        pct(&mut lag_upd, 0.5),
        pct(&mut lag_upd, 0.95),
        lag_upd.first().copied().unwrap_or(0),
        lag_upd.last().copied().unwrap_or(0),
    );
    println!("arrival - updateDate (ms): n {n}, min {mn}, p50 {p50}, p95 {p95}, max {mx}");
    println!("timestamp sources: {sources:?}");
    for (s, v) in &mut lag_time {
        let n = v.len();
        let (a, b) = (pct(v, 0.5), pct(v, 0.95));
        println!(
            "arrival - timestamp[{s}] (ms): n {n}, min {}, p50 {a}, p95 {b}, max {}",
            v.first().copied().unwrap_or(0),
            v.last().copied().unwrap_or(0)
        );
    }
    println!("clock events: {clock_events:?}");
    println!("stockStatus codes seen (count): {phases:?}");
    println!("publishReason seen (count): {pub_reason:?}");
    let mut tr = inspect_trades(&recs);
    if tr.messages() > 0 || !tr.notes.is_empty() {
        print_trade_inspect(&mut tr);
    }
    Ok(())
}

fn rest_ctx() -> Result<(LiveConfig, AuthClient, qp_live::lock::MachineLock), String> {
    let lock = identity_lock()?;
    let cfg = LiveConfig::from_env().map_err(|e| e.to_string())?;
    let auth = AuthClient::new(&cfg, AuthPolicy::default());
    Ok((cfg, auth, lock))
}

fn symbols() -> Result<(), String> {
    let (cfg, auth, _identity) = rest_ctx()?;
    let rest = qp_live::rest::RestClient::new(&auth, cfg.redactor().clone());
    let v = rest.symbols().map_err(|e| e.to_string())?;
    let dir = repo_root().join("fixtures/local/live");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("meta.json"), v.to_string()).map_err(|e| e.to_string())?;
    let rows: Vec<&serde_json::Value> = match &v {
        serde_json::Value::Array(a) => a.iter().collect(),
        serde_json::Value::Object(o) => o
            .values()
            .find_map(|x| x.as_array())
            .map(|a| a.iter().collect())
            .unwrap_or_default(),
        _ => Vec::new(),
    };
    let s =
        |r: &serde_json::Value, k: &str| r.get(k).and_then(|x| x.as_str()).unwrap_or("").to_owned();
    let mut by_type: BTreeMap<String, u64> = BTreeMap::new();
    let mut by_ex: BTreeMap<String, u64> = BTreeMap::new();
    let (mut u30, mut u100, mut utum, mut uall) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for r in &rows {
        if r.get("d").and_then(serde_json::Value::as_bool) == Some(true) {
            continue;
        }
        let (t, ex, code) = (s(r, "sT"), s(r, "eC"), s(r, "sC"));
        *by_type.entry(t.clone()).or_default() += 1;
        *by_ex.entry(ex.clone()).or_default() += 1;
        let idx: Vec<String> = r
            .get("ind")
            .and_then(|x| x.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default();
        if t == "S" && ex.contains("BIST") {
            uall.push(code.clone());
            if idx.iter().any(|i| i == "BIST30") {
                u30.push(code.clone());
            }
            if idx.iter().any(|i| i == "BISTTUM") {
                utum.push(code.clone());
            }
            if idx.iter().any(|i| i == "BIST100") {
                u100.push(code);
            }
        }
    }
    for (name, list) in [
        ("u30.txt", &mut u30),
        ("u100.txt", &mut u100),
        ("utum.txt", &mut utum),
        ("uall.txt", &mut uall),
    ] {
        list.sort();
        std::fs::write(dir.join(name), list.join("\n") + "\n").map_err(|e| e.to_string())?;
    }
    println!("meta rows {} ; by type {by_type:?}", rows.len());
    println!("by exchange {by_ex:?}");
    println!(
        "u30 {} ; u100 {} ; utum {} ; uall (BIST equities) {}",
        u30.len(),
        u100.len(),
        utum.len(),
        uall.len()
    );
    Ok(())
}

fn bars(
    symbols: &str,
    period: &str,
    start: &str,
    end: &str,
    unadjusted: bool,
) -> Result<(), String> {
    let syms = parse_symbols(symbols)?;
    let (cfg, auth, _identity) = rest_ctx()?;
    let rest = qp_live::rest::RestClient::new(&auth, cfg.redactor().clone());
    let dir = repo_root().join("fixtures/local/live/rest");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let tag = if unadjusted { "unadj" } else { "adj" };
    let mut total = 0usize;
    for sym in &syms {
        let path = dir.join(format!("{sym}.{period}.{start}.{end}.{tag}.json"));
        if path.exists() {
            println!("bars {sym} {period}: cached");
            continue;
        }
        let t0 = Instant::now();
        let got = rest.bars(sym, period, start, end, !unadjusted);
        log_request(
            &repo_root().join("fixtures/local/live/rest/requests.jsonl"),
            "bars",
            sym,
            &format!("{period}:{start}:{end}:{tag}"),
            got.as_ref().map(Vec::len).map_err(ToString::to_string),
            t0.elapsed(),
        );
        match got {
            Ok(b) => {
                total += b.len();
                let rows: Vec<serde_json::Value> = b
                    .iter()
                    .map(|x| {
                        serde_json::json!({"time": x.time, "open": x.open, "high": x.high,
                        "low": x.low, "close": x.close, "volume": x.volume, "quantity": x.quantity})
                    })
                    .collect();
                std::fs::write(&path, serde_json::Value::Array(rows).to_string())
                    .map_err(|e| e.to_string())?;
                println!("bars {sym} {period}: {} rows", b.len());
            }
            Err(e) => println!("bars {sym} {period}: error {e}"),
        }
    }
    println!(
        "bars: {} symbols, {total} rows, {} calls",
        syms.len(),
        rest.calls.load(Ordering::Relaxed)
    );
    Ok(())
}

/// Append one request line (counts only; no values, no addresses) to a JSONL log.
pub(crate) fn log_request(
    log: &Path,
    what: &str,
    sym: &str,
    range: &str,
    rows: Result<usize, String>,
    took: Duration,
) {
    let line = serde_json::json!({
        "what": what, "symbol": sym, "range": range,
        "ok": rows.is_ok(), "rows": rows.as_ref().ok(),
        "error": rows.as_ref().err().map(|e| e.chars().take(80).collect::<String>()),
        "ms": u64::try_from(took.as_millis()).unwrap_or(u64::MAX),
    });
    if let Some(d) = log.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log)
    {
        use std::io::Write as _;
        let _ = writeln!(f, "{line}");
    }
}

/// Calendar-day windows `[a, b]` of at most `days` days covering `start..=end`.
fn date_windows(start: &str, end: &str, days: u32) -> Result<Vec<(String, String)>, String> {
    if !(1..=15).contains(&days) {
        return Err("window-days must be 1..=15".into());
    }
    let (mut a, z) = (civil_days(start)?, civil_days(end)?);
    let mut out = Vec::new();
    while a <= z {
        let b = (a + i64::from(days) - 1).min(z);
        out.push((civil_date(a), civil_date(b)));
        a = b + 1;
    }
    Ok(out)
}

/// Days since 1970-01-01 of a `YYYY-MM-DD` date (proleptic Gregorian).
fn civil_days(s: &str) -> Result<i64, String> {
    let bad = || format!("date `{s}`: expected YYYY-MM-DD");
    let mut it = s.split('-');
    let (Some(y), Some(m), Some(d), None) = (it.next(), it.next(), it.next(), it.next()) else {
        return Err(bad());
    };
    let (y, m, d): (i64, i64, i64) = (
        y.parse().map_err(|_| bad())?,
        m.parse().map_err(|_| bad())?,
        d.parse().map_err(|_| bad())?,
    );
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return Err(bad());
    }
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Ok(era * 146_097 + doe - 719_468)
}

fn civil_date(z: i64) -> String {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

fn trades(symbols: &str, start: &str, end: &str, window_days: u32, ex: bool) -> Result<(), String> {
    let syms = parse_symbols(symbols)?;
    let windows = date_windows(start, end, window_days)?;
    let root = repo_root().join(if ex {
        "fixtures/local/trades_ex"
    } else {
        "fixtures/local/trades"
    });
    let log = root.join("requests.jsonl");
    // Only open the identity (token) when something is missing from the cache.
    let missing: Vec<(&String, &(String, String))> = syms
        .iter()
        .flat_map(|s| windows.iter().map(move |w| (s, w)))
        .filter(|(s, (a, b))| {
            !root
                .join(s.as_str())
                .join(format!("{a}_{b}.csv.gz"))
                .exists()
        })
        .collect();
    println!(
        "trades: {} symbols x {} windows; {} cached, {} to fetch",
        syms.len(),
        windows.len(),
        syms.len() * windows.len() - missing.len(),
        missing.len()
    );
    if missing.is_empty() {
        return Ok(());
    }
    let (cfg, auth, _identity) = rest_ctx()?;
    let rest = qp_live::rest::RestClient::new(&auth, cfg.redactor().clone());
    let (mut rows_total, mut bytes_total, mut errors) = (0usize, 0usize, 0usize);
    for (n, (sym, (a, b))) in missing.iter().enumerate() {
        let dir = root.join(sym.as_str());
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let t0 = Instant::now();
        let got = if ex {
            rest.trades_ex_csv(sym, a, b)
        } else {
            rest.trades_csv(sym, a, b)
        };
        let rows = got
            .as_ref()
            .map(|c| c.iter().filter(|&&x| x == b'\n').count().saturating_sub(1))
            .map_err(ToString::to_string);
        log_request(
            &log,
            "trades",
            sym,
            &format!("{a}:{b}"),
            rows.clone(),
            t0.elapsed(),
        );
        match got {
            Ok(csv) => {
                let path = dir.join(format!("{a}_{b}.csv.gz"));
                let tmp = path.with_extension("part");
                let f = File::create(&tmp).map_err(|e| e.to_string())?;
                let mut gz = flate2::write::GzEncoder::new(f, flate2::Compression::default());
                std::io::Write::write_all(&mut gz, &csv).map_err(|e| e.to_string())?;
                gz.finish().map_err(|e| e.to_string())?;
                std::fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
                rows_total += rows.unwrap_or(0);
                bytes_total += csv.len();
            }
            Err(e) => {
                errors += 1;
                println!("trades {sym} {a}..{b}: error {e}");
            }
        }
        if (n + 1) % 25 == 0 {
            println!(
                "trades: {}/{} requests, {rows_total} rows, {} MB csv, {errors} errors",
                n + 1,
                missing.len(),
                bytes_total / 1_000_000
            );
        }
    }
    println!(
        "trades: done; {} calls, {rows_total} rows, {} MB csv, {errors} errors",
        rest.calls.load(Ordering::Relaxed),
        bytes_total / 1_000_000
    );
    Ok(())
}

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

#[cfg(test)]
mod window_tests {
    use super::*;

    #[test]
    fn windows_cover_the_range_without_overlap() {
        let w = date_windows("2026-10-05", "2026-11-20", 14).unwrap();
        assert_eq!(
            w,
            vec![
                ("2026-10-05".into(), "2026-10-18".into()),
                ("2026-10-19".into(), "2026-11-01".into()),
                ("2026-11-02".into(), "2026-11-15".into()),
                ("2026-11-16".into(), "2026-11-20".into()),
            ]
        );
        assert!(date_windows("2026-01-01", "2026-01-02", 16).is_err());
        assert_eq!(civil_date(civil_days("2024-02-29").unwrap()), "2024-02-29");
        assert_eq!(civil_days("1970-01-01").unwrap(), 0);
    }
}

pub fn parse_method(s: &str) -> Result<qp_source::BarMethod, String> {
    use qp_source::BarMethod;
    Ok(match s {
        "ticks" => BarMethod::Ticks,
        "card" | "card_daily" => BarMethod::CardDaily,
        "mixed" => BarMethod::Mixed,
        _ => return Err(format!("method `{s}`: expected ticks, card or mixed")),
    })
}

pub fn parse_volume_field(s: &str) -> Result<qp_matriks::VolumeField, String> {
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
pub fn xist() -> Result<qp_calendar::ExchangeCalendar, String> {
    let cal = qp_calendar::builtin("XIST").ok_or("no XIST calendar")?;
    qp_calendar::ExchangeCalendar::new(&cal).map_err(|e| e.to_string())
}

/// JSON lines of one [`qp_matriks::FeedOut`], in a fixed order: connection, phases, data
/// events, clock, final bars.
pub fn feed_out_lines(out: &qp_matriks::FeedOut) -> Vec<String> {
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

/// REST period name of a Qapsule interval.
fn rest_period(iv: &str) -> Option<&'static str> {
    Some(match iv {
        "1m" => "1min",
        "5m" => "5min",
        "15m" => "15min",
        "30m" => "30min",
        "1h" => "1hour",
        "1d" => "1day",
        _ => return None,
    })
}

/// REST bars of `sym` × `iv` mapped to K14 (`docs/DATA_ACCESS.md` R3): daily `time` (local
/// midnight) → the day's session open; intraday `time` = bar open, bars outside the
/// session (closing session, 18:00+) merged into the day's previous bar. Several files are
/// merged by `t` (later file wins). `None`: no file.
pub(crate) fn load_rest(
    dir: &Path,
    sym: &str,
    iv: &str,
    prefer_adj: bool,
    quantity: bool,
    cal: &qp_calendar::ExchangeCalendar,
) -> Result<Option<Vec<qp_schema::Bar>>, String> {
    load_rest_with(dir, sym, iv, prefer_adj, quantity, cal, false)
}

/// [`load_rest`]; with `last_span`, a bar outside the session merges into the day's
/// **last session bar** (`bar_at(interval, last close − 1)`, the live rule 1 of
/// `qp_source::livebar`), which is created from it when REST has no bar there (no
/// continuous trade in the last minute), instead of into the previous REST bar.
pub(crate) fn load_rest_with(
    dir: &Path,
    sym: &str,
    iv: &str,
    prefer_adj: bool,
    quantity: bool,
    cal: &qp_calendar::ExchangeCalendar,
    last_span: bool,
) -> Result<Option<Vec<qp_schema::Bar>>, String> {
    use qp_schema::Bar;
    let Some(period) = rest_period(iv) else {
        return Err(format!("interval `{iv}` has no REST period"));
    };
    let interval = qp_calendar::Interval::parse(iv).map_err(|e| e.to_string())?;
    let prefix = format!("{sym}.{period}.");
    let mut files: Vec<(bool, PathBuf)> = Vec::new();
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Ok(None);
    };
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if !name.starts_with(&prefix) {
            continue;
        }
        if name.ends_with(".unadj.json") {
            files.push((false, e.path()));
        } else if name.ends_with(".adj.json") {
            files.push((true, e.path()));
        }
    }
    let want: Vec<PathBuf> = {
        let pick: Vec<_> = files.iter().filter(|(adj, _)| *adj == prefer_adj).collect();
        let pick = if pick.is_empty() {
            files.iter().collect()
        } else {
            pick
        };
        let mut v: Vec<PathBuf> = pick.into_iter().map(|(_, p)| p.clone()).collect();
        v.sort();
        v
    };
    if want.is_empty() {
        return Ok(None);
    }
    let mut raw: BTreeMap<i64, Bar> = BTreeMap::new();
    for p in &want {
        let text = std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))?;
        let rows: Vec<serde_json::Value> =
            serde_json::from_str(&text).map_err(|e| format!("{}: {e}", p.display()))?;
        for r in rows {
            let num = |k: &str| r.get(k).and_then(serde_json::Value::as_f64);
            let (Some(time), Some(o), Some(h), Some(l), Some(c)) = (
                r.get("time").and_then(serde_json::Value::as_i64),
                num("open"),
                num("high"),
                num("low"),
                num("close"),
            ) else {
                continue;
            };
            let v = num(if quantity { "quantity" } else { "volume" }).unwrap_or(0.0);
            raw.insert(
                time,
                Bar {
                    t: time,
                    o,
                    h,
                    l,
                    c,
                    v,
                },
            );
        }
    }
    let mut out: Vec<Bar> = Vec::new();
    for (_, mut b) in raw {
        if interval == qp_calendar::Interval::Day {
            let Some(span) = cal.day_span(cal.local_day(b.t)) else {
                continue;
            };
            b.t = span.open;
            out.push(b);
            continue;
        }
        if cal.in_session(b.t) {
            out.push(b);
            continue;
        }
        let day = cal.local_day(b.t);
        let target = cal
            .sessions_on(day)
            .last()
            .and_then(|s| cal.bar_at(interval, s.close - 1))
            .map(|s| s.open);
        if let (true, Some(tt)) = (last_span, target) {
            if b.t >= tt {
                match out.last_mut() {
                    Some(prev) if prev.t == tt => {
                        prev.h = prev.h.max(b.h);
                        prev.l = prev.l.min(b.l);
                        prev.c = b.c;
                        prev.v += b.v;
                    }
                    _ => out.push(Bar { t: tt, ..b }),
                }
                continue;
            }
        }
        // Outside the session: merge into the same day's previous bar.
        match out.last_mut() {
            Some(prev) if cal.local_day(prev.t) == cal.local_day(b.t) && prev.t < b.t => {
                prev.h = prev.h.max(b.h);
                prev.l = prev.l.min(b.l);
                prev.c = b.c;
                prev.v += b.v;
            }
            _ => {}
        }
    }
    Ok(Some(out))
}

/// Which part of the recording a live bar belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Bucket {
    /// Complete: finalized, after the feed's first bar, outside every gap.
    Full,
    /// The feed's first bar (subscription mid-bar: partial).
    First,
    /// Overlaps a connection gap.
    Gap,
    /// Not final when the recording ended.
    Open,
}

impl Bucket {
    fn name(self) -> &'static str {
        match self {
            Bucket::Full => "full",
            Bucket::First => "first",
            Bucket::Gap => "gap",
            Bucket::Open => "open",
        }
    }
}

/// Records of several raw logs as one recording (rule in [`CompareArgs::raw`]).
pub(crate) fn read_recordings(list: &str) -> Result<Vec<RawRecord>, String> {
    let mut all: Vec<RawRecord> = Vec::new();
    for (i, f) in split_list(list).iter().enumerate() {
        let path = Path::new(f);
        warn_outside_local(path);
        let (recs, tail) = read_all(path)?;
        if let Some(t) = tail {
            eprintln!("{f}: ends early ({t}); using the records before it");
        }
        if i > 0
            && all
                .last()
                .is_some_and(|r| r.kind != RecordKind::ConnectionLost)
        {
            if let Some(first) = recs.first() {
                all.push(RawRecord {
                    t_us: first.t_us,
                    kind: RecordKind::ConnectionLost,
                    topic: String::new(),
                    payload: b"file boundary".to_vec(),
                });
            }
        }
        all.extend(recs);
    }
    if all.is_empty() {
        return Err("no records".into());
    }
    Ok(all)
}

pub(crate) struct Built {
    pub(crate) stores: BTreeMap<qp_schema::data::FeedKey, qp_source::FeedStore>,
    pub(crate) finals: std::collections::BTreeSet<(qp_schema::data::FeedKey, i64)>,
    pub(crate) gaps: Vec<(i64, i64)>,
    pub(crate) stats: qp_matriks::FeedStats,
}

pub(crate) fn build_live(recs: &[RawRecord], args: &FeedArgs) -> Result<Built, String> {
    let mut feed = args.build()?;
    let mut stores: BTreeMap<qp_schema::data::FeedKey, qp_source::FeedStore> = BTreeMap::new();
    let mut finals = std::collections::BTreeSet::new();
    let mut gaps = Vec::new();
    let mut gap_start: Option<i64> = None;
    let mut waiting = false;
    for r in recs {
        let before = feed.clock_ms();
        let out = feed.push(r);
        if out.connection == Some(qp_matriks::ConnChange::Lost) {
            gap_start = before.or(gap_start);
            waiting = true;
        }
        if waiting {
            if let (Some(c), Some(s)) = (out.clock_ms, gap_start) {
                gaps.push((s, c));
                waiting = false;
                gap_start = None;
            }
        }
        for e in &out.events {
            let store = stores
                .entry(e.feed.clone())
                .or_insert_with(|| qp_source::FeedStore::new(e.feed.clone()));
            store
                .apply(e)
                .map_err(|err| format!("live event does not apply: {err}"))?;
        }
        finals.extend(out.finalized);
    }
    if let (true, Some(s), Some(c)) = (waiting, gap_start, feed.clock_ms()) {
        gaps.push((s, c.max(s)));
    }
    Ok(Built {
        stores,
        finals,
        gaps,
        stats: feed.stats(),
    })
}

#[allow(clippy::too_many_lines)]
fn compare(a: &CompareArgs) -> Result<(), String> {
    use qp_source::{compare_bars_with, BarDiff, DiffTolerance};
    let cal = xist()?;
    let recs = read_recordings(&a.raw)?;
    let intervals = split_list(&a.intervals);
    let methods = split_list(&a.methods);
    let vfields = split_list(&a.volume_fields);
    for m in &methods {
        parse_method(m)?;
    }
    for v in &vfields {
        parse_volume_field(v)?;
    }
    let quantity = a.volume_unit == "quantity";
    let tol = DiffTolerance {
        price_abs: a.price_tol,
        volume_rel: a.volume_tol,
    };
    // REST bars, per symbol × interval, loaded once.
    let mut rest: BTreeMap<(String, String), Vec<qp_schema::Bar>> = BTreeMap::new();
    let mut symbols = std::collections::BTreeSet::new();
    for r in &recs {
        if let qp_matriks::Topic::Symbol { symbol, .. } =
            qp_matriks::parse_topic(&r.topic).unwrap_mirror()
        {
            if r.kind == RecordKind::Message {
                symbols.insert(symbol.clone());
            }
        }
    }
    let mut missing: BTreeMap<String, usize> = BTreeMap::new();
    for iv in &intervals {
        qp_calendar::Interval::parse(iv).map_err(|e| format!("interval `{iv}`: {e}"))?;
        let mut found = 0;
        for sym in &symbols {
            match load_rest(&a.rest_dir, sym, iv, a.adjusted, quantity, &cal)? {
                Some(b) => {
                    found += 1;
                    rest.insert((sym.clone(), iv.clone()), b);
                }
                None => *missing.entry(iv.clone()).or_default() += 1,
            }
        }
        if found == 0 {
            return Err(format!(
                "no REST bars for {iv} in {} (expected <SYM>.{}.<start>.<end>.<adj|unadj>.json; fetch with `qp live bars`)",
                a.rest_dir.display(),
                rest_period(iv).unwrap_or("?")
            ));
        }
    }
    println!(
        "M22 compare: {} records, {} symbols; REST missing per interval {:?}; tol price {} abs, volume {} rel; time {}",
        recs.len(),
        symbols.len(),
        missing,
        a.price_tol,
        a.volume_tol,
        a.time
    );
    println!(
        "{:<4} {:<6} {:<6} {:<5} {:>7} {:>6} {:>6} | {:>6} {:>6} | exact% o/h/l/c/v               | tol% o/h/l/c/v                  | max rel o/h/l/c/v",
        "iv", "method", "vol", "part", "matched", "l-only", "r-only", "all=", "all~"
    );
    for m in &methods {
        for vf in &vfields {
            let args = FeedArgs {
                intervals: intervals.join(","),
                method: m.clone(),
                volume_field: vf.clone(),
                volume_unit: a.volume_unit.clone(),
                time: a.time.clone(),
                finalize_grace_ms: a.finalize_grace_ms,
            };
            let built = build_live(&recs, &args)?;
            for iv in &intervals {
                if m == "card" && iv != "1d" {
                    continue;
                }
                let interval = qp_calendar::Interval::parse(iv).map_err(|e| e.to_string())?;
                let mut per: BTreeMap<Bucket, BarDiff> = BTreeMap::new();
                for (key, store) in built.stores.iter().filter(|(k, _)| &k.interval == iv) {
                    let Some(rbars) = rest.get(&(key.symbol.clone(), iv.clone())) else {
                        continue;
                    };
                    let bars = store.bars();
                    let mut by: BTreeMap<Bucket, Vec<qp_schema::Bar>> = BTreeMap::new();
                    for (i, b) in bars.iter().enumerate() {
                        let span = cal.bar_at(interval, b.t);
                        let in_gap = span.is_some_and(|s| {
                            built
                                .gaps
                                .iter()
                                .any(|&(g0, g1)| s.open <= g1 && g0 < s.close)
                        });
                        let bucket = if in_gap {
                            Bucket::Gap
                        } else if i == 0 {
                            Bucket::First
                        } else if !built.finals.contains(&(key.clone(), b.t)) {
                            Bucket::Open
                        } else {
                            Bucket::Full
                        };
                        by.entry(bucket).or_default().push(b.clone());
                    }
                    let owner: BTreeMap<i64, Bucket> = by
                        .iter()
                        .flat_map(|(k, v)| v.iter().map(move |b| (b.t, *k)))
                        .collect();
                    for (bucket, lb) in by {
                        // REST-only counts only REST bars inside this part's range that no
                        // other part holds (a missing live bar).
                        let (lo, hi) = (lb[0].t, lb[lb.len() - 1].t);
                        let rb: Vec<qp_schema::Bar> = rbars
                            .iter()
                            .filter(|r| {
                                r.t >= lo
                                    && r.t <= hi
                                    && owner.get(&r.t).is_none_or(|&o| o == bucket)
                            })
                            .cloned()
                            .collect();
                        let d = compare_bars_with(&lb, &rb, &tol);
                        let acc = per.entry(bucket).or_default();
                        merge_diff(acc, &d);
                    }
                }
                for (bucket, d) in &per {
                    let pc = |n: u64| 100.0 * d.ratio(n);
                    let f = [d.o, d.h, d.l, d.c, d.v];
                    println!(
                        "{:<4} {:<6} {:<6} {:<5} {:>7} {:>6} {:>6} | {:>5.1}% {:>5.1}% | {} | {} | {}",
                        iv,
                        m,
                        vf,
                        bucket.name(),
                        d.matched,
                        d.live_only,
                        d.rest_only,
                        pc(d.all_exact),
                        pc(d.all_within_tol),
                        f.iter().map(|x| format!("{:>5.1}", pc(x.exact))).collect::<Vec<_>>().join(" "),
                        f.iter().map(|x| format!("{:>5.1}", pc(x.within_tol))).collect::<Vec<_>>().join(" "),
                        f.iter().map(|x| format!("{:.1e}", x.max_rel)).collect::<Vec<_>>().join(" "),
                    );
                }
            }
            let s = &built.stats;
            println!(
                "     {m}/{vf}: ticks {} (derived time {}), off-session {}, late {}, behind clock {} (max {} ms), regressions {}, rebases {}, gaps {}",
                s.ticks,
                s.time_derived,
                s.off_session,
                s.late,
                s.behind_clock,
                s.max_behind_ms,
                s.volume_regressions,
                s.volume_rebases,
                built.gaps.len()
            );
        }
    }
    Ok(())
}

/// Adds `d` into `acc` (counts summed, maxima kept).
pub(crate) fn merge_diff(acc: &mut qp_source::BarDiff, d: &qp_source::BarDiff) {
    let f = |a: &mut qp_source::FieldDiff, b: &qp_source::FieldDiff| {
        a.exact += b.exact;
        a.within_tol += b.within_tol;
        a.max_abs = a.max_abs.max(b.max_abs);
        a.max_rel = a.max_rel.max(b.max_rel);
    };
    acc.matched += d.matched;
    acc.live_only += d.live_only;
    acc.rest_only += d.rest_only;
    acc.all_exact += d.all_exact;
    acc.all_within_tol += d.all_within_tol;
    f(&mut acc.o, &d.o);
    f(&mut acc.h, &d.h);
    f(&mut acc.l, &d.l);
    f(&mut acc.c, &d.c);
    f(&mut acc.v, &d.v);
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    #[test]
    fn iso_times_parse_with_and_without_offset() {
        // 2026-11-19 14:02:03.123 Istanbul = 11:02:03.123 UTC.
        let utc = (qp_calendar::days_from_civil(2026, 11, 19) * 86_400 + 11 * 3600 + 2 * 60 + 3)
            * 1000
            + 123;
        assert_eq!(
            parse_iso_ms("2026-11-19T14:02:03.123+03"),
            Some((utc, true))
        );
        assert_eq!(
            parse_iso_ms("2026-11-19T14:02:03.123+03:00"),
            Some((utc, true))
        );
        assert_eq!(parse_iso_ms("2026-11-19T11:02:03.123Z"), Some((utc, true)));
        assert_eq!(parse_iso_ms("2026-11-19T14:02:03.123"), Some((utc, false)));
        assert_eq!(parse_iso_ms("19/11/26"), None);
        assert_eq!(
            shape("2026-11-19T14:02:03.123+03"),
            "dddd-dd-ddTdd:dd:dd.ddd+dd"
        );
    }

    #[test]
    fn identity_lock_is_exclusive() {
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("QP_LIVE_LOCK_DIR", dir.path());
        let first = identity_lock().expect("first holder");
        let second = identity_lock();
        assert!(second.is_err(), "a second token holder must be refused");
        assert!(second.unwrap_err().contains("identity"));
        drop(first);
        assert!(identity_lock().is_ok(), "released with its holder");
        std::env::remove_var("QP_LIVE_LOCK_DIR");
    }

    #[test]
    fn reset_needs_a_note_and_a_person() {
        let dir = tempfile::tempdir().unwrap();
        let key = qp_live::EndpointKey::new("delayed-market");
        let marker = key.halt_marker(dir.path());
        std::fs::write(
            &marker,
            "halted=unexplained-server-close\ncloses=2\nwindow_secs=600\n",
        )
        .unwrap();
        let args = |note: &str, human: bool| ResetArgs {
            endpoint: "delayed-market".into(),
            note: note.into(),
            i_am_human: human,
        };
        let e = reset(&args("  ", true), dir.path()).unwrap_err();
        assert!(e.contains("--note is required"), "{e}");
        assert!(marker.exists());
        // Tests run without a terminal on stdin: the flag is required.
        if !std::io::IsTerminal::is_terminal(&std::io::stdin()) {
            let e = reset(&args("checked", false), dir.path()).unwrap_err();
            assert!(e.contains("--i-am-human"), "{e}");
            assert!(marker.exists());
        }
        reset(&args("Arkin: no other session", true), dir.path()).unwrap();
        assert!(!marker.exists());
        let log = std::fs::read_to_string(qp_live::halt::halt_log(dir.path(), &key)).unwrap();
        assert!(
            log.contains("RESET previous=unexplained-server-close"),
            "{log}"
        );
        assert!(log.contains("Arkin: no other session"), "{log}");
        // Not halted: nothing to do.
        reset(&args("again", true), dir.path()).unwrap();
    }

    #[test]
    fn symbols_are_checked() {
        assert_eq!(
            parse_symbols("thyao, garan").unwrap(),
            vec!["THYAO", "GARAN"]
        );
        assert!(parse_symbols("a/b").is_err());
        assert!(parse_symbols(" , ").is_err());
    }

    /// Hand-built records (made-up symbol and prices): a connection, a snapshot, trades
    /// with ask-only updates in between, a lost connection and a reconnect.
    pub(crate) fn fake_records() -> Vec<RawRecord> {
        use prost::Message as _;
        use qp_matriks::messages::SymbolMessage;
        let base = qp_calendar::days_from_civil(2026, 11, 18) * 86_400_000 - 3 * 3_600_000;
        let at = |h: i64, m: i64, s: i64| base + ((h * 60 + m) * 60 + s) * 1000;
        let ud = |t: i64| {
            let l = t + 3 * 3_600_000 - qp_calendar::days_from_civil(2026, 11, 18) * 86_400_000;
            format!(
                "2026-11-18T{:02}:{:02}:{:02}.{:03}+03",
                l / 3_600_000,
                l / 60_000 % 60,
                l / 1000 % 60,
                l % 1000
            )
        };
        let topic = "mx/symbol/TEST1@lvl2";
        let delay = 900_000;
        let r = |kind, t: i64| RawRecord {
            t_us: t * 1000,
            kind,
            topic: String::new(),
            payload: Vec::new(),
        };
        let mut out = vec![r(RecordKind::ConnectionOpened, at(10, 0, 0))];
        let snap = SymbolMessage {
            symbol_id: 7,
            update_date: Some(ud(at(9, 58, 0))),
            last: Some(10.0),
            open: Some(10.0),
            daily_volume: Some(0.0),
            daily_quantity: Some(0.0),
            stock_status: Some(2),
            ..SymbolMessage::default()
        };
        out.push(RawRecord::message(
            at(10, 0, 1) * 1000,
            topic,
            snap.encode_to_vec(),
        ));
        let mut cum = 0.0;
        for i in 0..40i32 {
            let t = at(10, 0, 5) + i64::from(i) * 17_000;
            cum += 100.0 + f64::from(i);
            let m = SymbolMessage {
                symbol_id: 7,
                update_date: Some(ud(t)),
                last: Some(10.0 + f64::from(i % 7) * 0.05),
                daily_volume: Some(cum),
                volume: Some(cum * 2.0),
                daily_quantity: Some(cum / 10.0),
                ..SymbolMessage::default()
            };
            out.push(RawRecord::message(
                (t + delay) * 1000,
                topic,
                m.encode_to_vec(),
            ));
            let ask = SymbolMessage {
                symbol_id: 7,
                ask_size: Some(i64::from(i)),
                ..SymbolMessage::default()
            };
            out.push(RawRecord::message(
                (t + delay + 200) * 1000,
                topic,
                ask.encode_to_vec(),
            ));
            if i == 20 {
                let lost = (t + delay + 500) * 1000;
                out.push(RawRecord {
                    t_us: lost,
                    kind: RecordKind::ConnectionLost,
                    topic: String::new(),
                    payload: b"test".to_vec(),
                });
                out.push(RawRecord {
                    t_us: lost + 1000,
                    kind: RecordKind::ConnectionOpened,
                    topic: "market".into(),
                    payload: Vec::new(),
                });
                out.push(RawRecord {
                    t_us: lost + 2000,
                    kind: RecordKind::Subscribed,
                    topic: topic.into(),
                    payload: Vec::new(),
                });
            }
        }
        out
    }

    pub(crate) fn write_raw(path: &Path, recs: &[RawRecord]) {
        let mut w = RawWriter::new(File::create(path).unwrap()).unwrap();
        for r in recs {
            w.write(r).unwrap();
        }
        w.flush().unwrap();
    }

    fn feed_args() -> FeedArgs {
        FeedArgs {
            intervals: "1m,5m,1d".into(),
            method: "ticks".into(),
            volume_field: "daily".into(),
            volume_unit: "value".into(),
            time: "update_date".into(),
            finalize_grace_ms: qp_matriks::feed::DEFAULT_FINALIZE_GRACE_MS,
        }
    }

    /// Replay output = the live path (`LiveFeed::push` record by record), byte for byte,
    /// and two replays are byte-equal.
    #[test]
    fn replay_raw_equals_the_live_path() {
        let dir = tempfile::tempdir().unwrap();
        let raw = dir.path().join("fake.qpraw");
        let recs = fake_records();
        write_raw(&raw, &recs);
        let out1 = dir.path().join("a.jsonl");
        let out2 = dir.path().join("b.jsonl");
        replay_raw(&raw, &feed_args(), Some(&out1), false).unwrap();
        replay_raw(&raw, &feed_args(), Some(&out2), false).unwrap();
        let a = std::fs::read(&out1).unwrap();
        assert_eq!(a, std::fs::read(&out2).unwrap());

        let mut feed = feed_args().build().unwrap();
        let mut live = String::new();
        for r in &recs {
            for l in feed_out_lines(&feed.push(r)) {
                live.push_str(&l);
                live.push('\n');
            }
        }
        assert_eq!(String::from_utf8(a).unwrap(), live);
        assert!(live.contains("\"conn\":\"lost\""));
        assert!(live.contains("\"final\""));
        assert!(live.contains("\"clock\""));
        assert!(live.contains("\"phase\""));
        assert_eq!(feed.stats().resubscribes, 1);
    }

    /// REST mapping (R3 rule): daily local midnight → session open; a closing-session
    /// bar merges into the previous bar; adjusted files are used only without unadjusted.
    #[test]
    fn rest_bars_map_to_k14() {
        let dir = tempfile::tempdir().unwrap();
        let cal = xist().unwrap();
        let day = qp_calendar::days_from_civil(2026, 11, 18);
        let local = |h: i64, m: i64| day * 86_400_000 + (h * 60 + m) * 60_000 - 3 * 3_600_000;
        let row = |t: i64, o: f64, h: f64, l: f64, c: f64, v: f64| serde_json::json!({"time": t, "open": o, "high": h, "low": l, "close": c, "volume": v, "quantity": v / 10.0});
        std::fs::write(
            dir.path()
                .join("TEST1.1day.2026-11-18.2026-11-18.unadj.json"),
            serde_json::json!([row(local(0, 0), 1.0, 2.0, 0.5, 1.5, 100.0)]).to_string(),
        )
        .unwrap();
        std::fs::write(
            dir.path().join("TEST1.1day.2026-11-18.2026-11-18.adj.json"),
            serde_json::json!([row(local(0, 0), 9.0, 9.0, 9.0, 9.0, 9.0)]).to_string(),
        )
        .unwrap();
        std::fs::write(
            dir.path()
                .join("TEST1.1min.2026-11-18.2026-11-18.unadj.json"),
            serde_json::json!([
                row(local(17, 58), 1.0, 1.2, 0.9, 1.1, 10.0),
                row(local(17, 59), 1.1, 1.3, 1.0, 1.2, 20.0),
                row(local(18, 4), 1.4, 1.4, 1.4, 1.4, 5.0)
            ])
            .to_string(),
        )
        .unwrap();
        let d = load_rest(dir.path(), "TEST1", "1d", false, false, &cal)
            .unwrap()
            .unwrap();
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].t, local(10, 0));
        assert_eq!(d[0].o.to_bits(), 1.0f64.to_bits());
        let adj = load_rest(dir.path(), "TEST1", "1d", true, false, &cal)
            .unwrap()
            .unwrap();
        assert_eq!(adj[0].o.to_bits(), 9.0f64.to_bits());
        let m = load_rest(dir.path(), "TEST1", "1m", false, false, &cal)
            .unwrap()
            .unwrap();
        assert_eq!(m.len(), 2);
        assert_eq!(m[1].t, local(17, 59));
        assert_eq!((m[1].h, m[1].c, m[1].v), (1.4, 1.4, 25.0));
        let q = load_rest(dir.path(), "TEST1", "1m", false, true, &cal)
            .unwrap()
            .unwrap();
        assert_eq!(q[0].v, 1.0);
        assert!(load_rest(dir.path(), "OTHER", "1m", false, false, &cal)
            .unwrap()
            .is_none());
    }

    /// The compare command runs on a hand-built log and REST files made from its own live
    /// bars, and fails clearly without REST files.
    #[test]
    fn compare_runs_and_requires_rest_files() {
        let dir = tempfile::tempdir().unwrap();
        let raw = dir.path().join("fake.qpraw");
        let recs = fake_records();
        write_raw(&raw, &recs);
        let rest = dir.path().join("rest");
        std::fs::create_dir_all(&rest).unwrap();
        let args = |rest_dir: PathBuf| CompareArgs {
            raw: raw.display().to_string(),
            rest_dir,
            intervals: "1m".into(),
            methods: "ticks,mixed".into(),
            volume_fields: "daily,volume".into(),
            volume_unit: "value".into(),
            time: "update_date".into(),
            finalize_grace_ms: 2000,
            price_tol: 1e-6,
            volume_tol: 0.01,
            adjusted: false,
        };
        let err = compare(&args(rest.clone())).unwrap_err();
        assert!(err.contains("no REST bars for 1m"), "{err}");
        let mut fa = feed_args();
        fa.intervals = "1m".into();
        let built = build_live(&recs, &fa).unwrap();
        assert_eq!(built.gaps.len(), 1);
        let bars = built.stores.values().next().unwrap().bars().to_vec();
        let rows: Vec<serde_json::Value> = bars
            .iter()
            .map(|b| serde_json::json!({"time": b.t, "open": b.o, "high": b.h, "low": b.l, "close": b.c, "volume": b.v, "quantity": 0.0}))
            .collect();
        std::fs::write(
            rest.join("TEST1.1min.2026-11-18.2026-11-18.unadj.json"),
            serde_json::Value::Array(rows).to_string(),
        )
        .unwrap();
        compare(&args(rest)).unwrap();
    }
}
