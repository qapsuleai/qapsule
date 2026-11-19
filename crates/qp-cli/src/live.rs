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
    /// universe lists `u100.txt` (XU100 members) and `uall.txt` (BIST equities); prints
    /// only counts.
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
    /// Aggregate statistics of a raw log (no market values).
    Inspect {
        /// Raw log file (`*.qpraw`).
        file: PathBuf,
    },
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
    /// Topic suffix for the price card (`@lvl2`; empty for the bare topic).
    #[arg(long, default_value = "@lvl2")]
    pub level: String,
    /// Print a status line every this many seconds.
    #[arg(long, default_value_t = 10)]
    pub every: u64,
}

pub fn run(cmd: &LiveCommand) -> Result<(), String> {
    match cmd {
        LiveCommand::Record(a) => record(a),
        LiveCommand::Inspect { file } => inspect(file),
        LiveCommand::Symbols => symbols(),
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

/// A sink that appends every message and event to a raw log and counts.
pub struct RawSink {
    w: Mutex<RawWriter<BufWriter<File>>>,
    pub messages: AtomicU64,
    pub bytes: AtomicU64,
    pub events: Mutex<Vec<String>>,
    pub errors: AtomicU64,
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
        })
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
        self.messages.fetch_add(1, Ordering::Relaxed);
        self.bytes
            .fetch_add(m.payload.len() as u64, Ordering::Relaxed);
        self.put(&RawRecord::message(m.received_at_us, m.topic, m.payload));
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

fn record(a: &RecordArgs) -> Result<(), String> {
    let symbols = parse_symbols(&a.symbols)?;
    let cfg = LiveConfig::from_env().map_err(|e| e.to_string())?;
    if cfg.hardware_id_warning() {
        eprintln!("live: warning: MATRIKS_HARDWARE_ID does not start with `qapsule-`");
    }
    let root = repo_root();
    let start_us = now_us();
    let out = a.out.clone().unwrap_or_else(|| {
        root.join("fixtures/local/live/raw")
            .join(format!("{}.qpraw", utc_stamp(start_us)))
    });
    if !out.starts_with(root.join("fixtures/local")) {
        return Err("live: the raw log must stay under fixtures/local/ (K23)".to_owned());
    }
    let mut topics = vec!["mx/timestamp".to_owned()];
    if a.messenger {
        topics.push("mx/messenger".to_owned());
    }
    topics.extend(symbols.iter().map(|s| format!("mx/symbol/{s}{}", a.level)));
    let sink = Arc::new(RawSink::create(&out)?);
    let auth = Arc::new(AuthClient::new(&cfg, AuthPolicy::default()));
    let mut opts = ConnectOptions::new(topics.clone());
    opts.classifier = Some(classifier());
    opts.lock_dir = root.join("fixtures/local");
    println!(
        "live: mode {:?}, endpoint {}, {} symbols, {} topics, {} s",
        cfg.mode(),
        cfg.market_endpoint().key(),
        symbols.len(),
        topics.len(),
        a.seconds
    );
    let t0 = Instant::now();
    let conn = LiveConnection::open(
        &cfg,
        cfg.market_endpoint(),
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
    sink.flush();
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
    Ok(())
}

fn rest_ctx() -> Result<(LiveConfig, AuthClient), String> {
    let cfg = LiveConfig::from_env().map_err(|e| e.to_string())?;
    let auth = AuthClient::new(&cfg, AuthPolicy::default());
    Ok((cfg, auth))
}

fn symbols() -> Result<(), String> {
    let (cfg, auth) = rest_ctx()?;
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
    let (mut u100, mut uall) = (Vec::new(), Vec::new());
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
            if idx.iter().any(|i| i == "BIST100") {
                u100.push(code);
            }
        }
    }
    u100.sort();
    uall.sort();
    std::fs::write(dir.join("u100.txt"), u100.join("\n") + "\n").map_err(|e| e.to_string())?;
    std::fs::write(dir.join("uall.txt"), uall.join("\n") + "\n").map_err(|e| e.to_string())?;
    println!("meta rows {} ; by type {by_type:?}", rows.len());
    println!("by exchange {by_ex:?}");
    println!("u100 {} ; uall (BIST equities) {}", u100.len(), uall.len());
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
    let (cfg, auth) = rest_ctx()?;
    let rest = qp_live::rest::RestClient::new(&auth, cfg.redactor().clone());
    let dir = repo_root().join("fixtures/local/live/rest");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let tag = if unadjusted { "unadj" } else { "adj" };
    let mut total = 0usize;
    for sym in &syms {
        let path = dir.join(format!("{sym}.{period}.{start}.{end}.{tag}.json"));
        match rest.bars(sym, period, start, end, !unadjusted) {
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

#[cfg(test)]
mod tests {
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
    fn symbols_are_checked() {
        assert_eq!(
            parse_symbols("thyao, garan").unwrap(),
            vec!["THYAO", "GARAN"]
        );
        assert!(parse_symbols("a/b").is_err());
        assert!(parse_symbols(" , ").is_err());
    }
}
