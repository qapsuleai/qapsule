//! REST bars and trades, the history and the repair events ([`RestSource`], [`RestTradeHistory`], [`history`]; RG1: split out of `livehost.rs`).

use super::*;

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

/// One trade history request: a symbol's level-1 trades over local dates
/// `start..=end` (`YYYY-MM-DD`, at most [`RestTradeHistory::MAX_WINDOW_DAYS`] days) and
/// the number of rows that did not parse. REST live; local files or a fake in tests.
pub type TradeFetch = dyn Fn(&str, &str, &str) -> Result<(Vec<Trade>, usize), String> + Send + Sync;

/// (RP4, L-prep) The live [`TradeHistory`] over the REST trade history
/// (`RestClient::trades_csv` + `qp_matriks::trade::parse_rest_csv`, level 1 only): a gap
/// `[start, end]` becomes requests of at most 15 local calendar days (the endpoint's
/// limit), the rows outside the gap are dropped, duplicates are removed by trade number
/// and the result is in exchange order. The network is used only here, i.e. only to
/// repair a trade gap (opening, reconnect). Counts only in its counters.
pub struct RestTradeHistory {
    fetch: Box<TradeFetch>,
    cal: ExchangeCalendar,
    window_days: i64,
    /// Requests made.
    pub calls: std::sync::atomic::AtomicU64,
    /// Rows received (before the gap filter) and rows that did not parse.
    pub rows: std::sync::atomic::AtomicU64,
    pub bad_rows: std::sync::atomic::AtomicU64,
}

impl RestTradeHistory {
    /// The endpoint's longest window (calendar days).
    pub const MAX_WINDOW_DAYS: i64 = 15;

    /// Over `fetch` with windows of at most `window_days` (clamped to 1..=15) days.
    #[must_use]
    pub fn new(fetch: Box<TradeFetch>, cal: ExchangeCalendar, window_days: i64) -> Self {
        Self {
            fetch,
            cal,
            window_days: window_days.clamp(1, Self::MAX_WINDOW_DAYS),
            calls: std::sync::atomic::AtomicU64::new(0),
            rows: std::sync::atomic::AtomicU64::new(0),
            bad_rows: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Over REST (`GET /dumrul/v1/tick/trade.gz`, the service token of `auth`), paced
    /// like every REST call, each request logged (counts only) to `log`.
    #[must_use]
    pub fn rest(
        cfg: ConnConfig,
        auth: Arc<qp_live::AuthClient>,
        cal: ExchangeCalendar,
        log: Option<std::path::PathBuf>,
    ) -> Self {
        let last: std::sync::Mutex<Option<Instant>> = std::sync::Mutex::new(None);
        let fetch = move |sym: &str, a: &str, b: &str| {
            {
                // One client per request (it borrows the token client); the pause
                // between requests is kept here.
                let mut l = last
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                if let Some(t) = *l {
                    let gap = Duration::from_millis(700);
                    if t.elapsed() < gap {
                        std::thread::sleep(gap - t.elapsed());
                    }
                }
                *l = Some(Instant::now());
            }
            let rest = qp_live::rest::RestClient::new(&auth, cfg.redactor().clone());
            let t0 = Instant::now();
            let got = rest
                .trades_csv(sym, a, b)
                .map(|csv| qp_matriks::trade::parse_rest_csv(&csv))
                .map_err(|e| e.to_string());
            if let Some(log) = &log {
                crate::live::log_request(
                    log,
                    "trades-repair",
                    sym,
                    &format!("{a}:{b}"),
                    got.as_ref().map(|(t, _)| t.len()).map_err(Clone::clone),
                    t0.elapsed(),
                );
            }
            got
        };
        Self::new(Box::new(fetch), cal, Self::MAX_WINDOW_DAYS)
    }

    /// Over cached trade files (`<dir>/<SYM>/*.csv.gz`, the `qp live trades` cache) instead
    /// of REST: offline tests, a replayed day. Every file of the symbol is read; the gap
    /// filter keeps the right rows.
    #[must_use]
    pub fn dir(dir: std::path::PathBuf, cal: ExchangeCalendar) -> Self {
        let fetch = move |sym: &str, _a: &str, _b: &str| {
            let d = dir.join(sym);
            let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(&d)
                .map_err(|e| format!("{}: {e}", d.display()))?
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.to_string_lossy().ends_with(".csv.gz"))
                .collect();
            files.sort();
            let (mut all, mut bad) = (Vec::new(), 0);
            for f in files {
                let raw = std::fs::read(&f).map_err(|e| format!("{}: {e}", f.display()))?;
                let mut csv = Vec::new();
                std::io::Read::read_to_end(&mut flate2::read::GzDecoder::new(&raw[..]), &mut csv)
                    .map_err(|e| format!("{}: {e}", f.display()))?;
                let (t, b) = qp_matriks::trade::parse_rest_csv(&csv);
                all.extend(t);
                bad += b;
            }
            Ok((all, bad))
        };
        Self::new(Box::new(fetch), cal, Self::MAX_WINDOW_DAYS)
    }

    /// The request windows (local dates, inclusive) covering `[start_ms, end_ms]`.
    #[must_use]
    pub fn windows(&self, start_ms: i64, end_ms: i64) -> Vec<(String, String)> {
        let date = |d: i64| {
            let (y, m, dd) = qp_calendar::civil_from_days(d);
            format!("{y:04}-{m:02}-{dd:02}")
        };
        let (mut a, z) = (self.cal.local_day(start_ms), self.cal.local_day(end_ms));
        let mut out = Vec::new();
        while a <= z {
            let b = (a + self.window_days - 1).min(z);
            out.push((date(a), date(b)));
            a = b + 1;
        }
        out
    }
}

impl TradeHistory for RestTradeHistory {
    fn trades(&self, symbol: &str, start_ms: i64, end_ms: i64) -> Result<Vec<Trade>, String> {
        use std::sync::atomic::Ordering::Relaxed;
        if end_ms < start_ms {
            return Ok(Vec::new());
        }
        let mut keys = std::collections::BTreeSet::new();
        let mut out = Vec::new();
        for (a, b) in self.windows(start_ms, end_ms) {
            self.calls.fetch_add(1, Relaxed);
            let (rows, bad) = (self.fetch)(symbol, &a, &b)?;
            self.rows.fetch_add(rows.len() as u64, Relaxed);
            self.bad_rows.fetch_add(bad as u64, Relaxed);
            for mut t in rows {
                let ms = t.t_ms();
                if ms < start_ms || ms > end_ms {
                    continue;
                }
                if keys.insert(t.dedup_key()) {
                    symbol.clone_into(&mut t.symbol);
                    out.push(t);
                }
            }
        }
        out.sort_by(Trade::exchange_cmp);
        Ok(out)
    }
}

/// Settled bars from local files instead of REST (`<dir>/<SYMBOL>.<period>.unadj.json`,
/// the `fetch-bars.sh` / `qp_source::load_local_bars_with` form): for tests without the
/// network and for a cached history. History and repair both read it.
pub struct DirSource {
    pub dir: std::path::PathBuf,
}

impl BarSource for DirSource {
    fn fetch(
        &self,
        symbols: &[String],
        interval: &str,
        from_ms: i64,
        to_ms: i64,
    ) -> Vec<(String, Result<Vec<Bar>, String>)> {
        let period = rest_period(interval).unwrap_or("?");
        let all = qp_source::load_local_bars_with(&self.dir, period, false);
        symbols
            .iter()
            .map(|s| {
                let r = all
                    .iter()
                    .find(|(x, _)| x == s)
                    .map(|(_, b)| {
                        b.iter()
                            .filter(|x| x.t >= from_ms && x.t <= to_ms)
                            .cloned()
                            .collect()
                    })
                    .ok_or_else(|| format!("no local {period} file"));
                (s.clone(), r)
            })
            .collect()
    }
}

/// Bars complete by exchange time `cutoff` (their span closed).
pub(super) fn complete_by(
    bars: Vec<Bar>,
    iv: Interval,
    cal: &ExchangeCalendar,
    cutoff: i64,
) -> Vec<Bar> {
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

/// The live host over an initial history (one feed per symbol on `interval`): the
/// session init of the demo (every feed bound and run by the capsule engine, the host's
/// feed rule) and [`ReplayHost::live`]. Shared by `qp demo --live`, `qp demo --sim` and
/// `qp sim` (one construction path).
///
/// # Errors
/// No history, or the host's own errors.
pub fn host_from_history(
    hist: Vec<(String, Vec<Bar>)>,
    interval: &str,
    opts: &qp_shellcore::shell::DemoOptions,
    root: &std::path::Path,
) -> Result<ReplayHost, String> {
    if hist.is_empty() {
        return Err("no symbol has history".to_owned());
    }
    let n = hist.len();
    let feeds: Vec<qp_source::ReplayFeed> = hist
        .into_iter()
        .map(|(s, bars)| qp_source::ReplayFeed {
            key: FeedKey {
                symbol: s,
                interval: interval.to_owned(),
                feed: None,
            },
            bars,
            fine: Vec::new(),
        })
        .collect();
    let mut init = qp_shellcore::shell::demo_session_init(&qp_shellcore::shell::DemoOptions {
        local: true,
        instruments: u32::try_from(n).unwrap_or(u32::MAX),
        from_t: None,
        running: true,
        ..opts.clone()
    });
    if let Some(r) = init.replay.as_mut() {
        r.interval = Some(interval.to_owned());
        r.fine = None;
    }
    ReplayHost::live(init, root, feeds)
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
