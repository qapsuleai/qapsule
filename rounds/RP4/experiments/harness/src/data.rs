//! Bar data: REST files → K14-aligned series (`prepare`), the binary series format, the
//! calibration / test split (SPLIT.md) and the loader that cuts at the split.
//!
//! # Series format (`<dir>/<period>/<SYM>.bars`)
//!
//! ```text
//! magic "QPRP4BR1" (8 bytes) | count u64 LE | count × record
//! record = t i64 | o f64 | h f64 | l f64 | c f64 | v f64     (all little-endian, 48 bytes)
//! ```
//!
//! `t` is the bar open in UTC milliseconds, already on the XIST calendar (K14): intraday
//! bars start inside `[10:00, 18:00)` Istanbul with the closing-auction bar merged into the
//! day's last bar; daily bars carry `t` = the day's session open (10:00 Istanbul). Bars are
//! strictly ascending. `<dir>/manifest.json` holds aggregate counts only.

use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use qp_calendar::{ExchangeCalendar, Interval};
use qp_schema::Bar;
use serde::{Deserialize, Serialize};

use crate::common::{istanbul_midnight_ms, to_json, write_file, xist};

/// File magic of a series file.
pub const MAGIC: [u8; 8] = *b"QPRP4BR1";

/// A measured period.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Period {
    /// 5 minutes (REST).
    M5,
    /// 15 minutes (aggregated from 5 minutes on the XIST calendar).
    M15,
    /// 1 hour (REST).
    H1,
    /// 1 day (REST).
    D1,
}

impl Period {
    /// All periods, in order.
    pub const ALL: [Self; 4] = [Self::M5, Self::M15, Self::H1, Self::D1];

    /// Parse `5m`, `15m`, `1h`, `1d`.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "5m" => Some(Self::M5),
            "15m" => Some(Self::M15),
            "1h" => Some(Self::H1),
            "1d" => Some(Self::D1),
            _ => None,
        }
    }

    /// Short name (directory name, series id).
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::M5 => "5m",
            Self::M15 => "15m",
            Self::H1 => "1h",
            Self::D1 => "1d",
        }
    }

    /// Calendar interval.
    #[must_use]
    pub fn interval(self) -> Interval {
        match self {
            Self::M5 => Interval::Minutes(5),
            Self::M15 => Interval::Minutes(15),
            Self::H1 => Interval::Minutes(60),
            Self::D1 => Interval::Day,
        }
    }

    /// REST file period token (`None`: derived, not fetched).
    #[must_use]
    pub fn rest_token(self) -> Option<&'static str> {
        match self {
            Self::M5 => Some("5min"),
            Self::M15 => None,
            Self::H1 => Some("1hour"),
            Self::D1 => Some("1day"),
        }
    }

    /// Intraday?
    #[must_use]
    pub fn intraday(self) -> bool {
        self != Self::D1
    }
}

/// Which range a run measures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Split {
    /// Calibration (default): only bars before the cut are read.
    Calib,
    /// Test: everything is read; only stories born in the test range count.
    Test,
}

impl Split {
    /// Parse `calib` / `test`.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "calib" => Some(Self::Calib),
            "test" => Some(Self::Test),
            _ => None,
        }
    }
    /// Name.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Calib => "calib",
            Self::Test => "test",
        }
    }
}

/// The SPLIT.md ranges of a period, as UTC milliseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ranges {
    /// Calibration starts (local midnight of the earliest allowed day).
    pub calib_start: i64,
    /// The cut: test range starts; calibration reads only `t < cut`.
    pub cut: i64,
    /// Test range ends (exclusive): local midnight after the last test day.
    pub test_end: i64,
}

/// SPLIT.md: intraday calibration 2025-11-03 → 2026-10-16, test 2026-10-19 → 2026-11-20;
/// daily calibration 2018-01-02 → 2024-12-31, test 2025-01-02 → 2026-11-20.
#[must_use]
pub fn ranges(p: Period) -> Ranges {
    if p.intraday() {
        Ranges {
            calib_start: istanbul_midnight_ms("2025-11-03"),
            cut: istanbul_midnight_ms("2026-10-19"),
            test_end: istanbul_midnight_ms("2026-11-21"),
        }
    } else {
        Ranges {
            calib_start: istanbul_midnight_ms("2018-01-02"),
            cut: istanbul_midnight_ms("2025-01-02"),
            test_end: istanbul_midnight_ms("2026-11-21"),
        }
    }
}

/// The counting window of a run: stories born in `[from, to)` count.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Window {
    /// First counted birth time.
    pub from: i64,
    /// End (exclusive).
    pub to: i64,
}

impl Window {
    /// The window of `split` for `period`.
    #[must_use]
    pub fn of(period: Period, split: Split) -> Self {
        let r = ranges(period);
        match split {
            Split::Calib => Self {
                from: r.calib_start,
                to: r.cut,
            },
            Split::Test => Self {
                from: r.cut,
                to: r.test_end,
            },
        }
    }
    /// Whether `t` is inside.
    #[must_use]
    pub fn contains(&self, t: i64) -> bool {
        self.from <= t && t < self.to
    }
}

// ---------------------------------------------------------------------------
// Binary series
// ---------------------------------------------------------------------------

/// Encode bars.
#[must_use]
pub fn encode(bars: &[Bar]) -> Vec<u8> {
    let mut out = Vec::with_capacity(16 + bars.len() * 48);
    out.extend_from_slice(&MAGIC);
    out.extend_from_slice(&(bars.len() as u64).to_le_bytes());
    for b in bars {
        out.extend_from_slice(&b.t.to_le_bytes());
        for x in [b.o, b.h, b.l, b.c, b.v] {
            out.extend_from_slice(&x.to_le_bytes());
        }
    }
    out
}

/// Read a series file, keeping only bars with `t < cut` (when given). Reading stops at
/// the first bar at or after the cut: later records are never decoded.
///
/// # Errors
/// I/O, bad magic, truncated file, bars not ascending.
pub fn read_series(path: &Path, cut: Option<i64>) -> Result<Vec<Bar>, String> {
    let mut f = std::io::BufReader::new(
        std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?,
    );
    let mut head = [0u8; 16];
    f.read_exact(&mut head)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    if head[..8] != MAGIC {
        return Err(format!("{}: bad magic", path.display()));
    }
    let n = u64::from_le_bytes(head[8..16].try_into().expect("8 bytes"));
    let mut bars = Vec::new();
    let mut rec = [0u8; 48];
    let rd =
        |r: &[u8], k: usize| f64::from_le_bytes(r[8 + 8 * k..16 + 8 * k].try_into().expect("8"));
    for _ in 0..n {
        f.read_exact(&mut rec)
            .map_err(|e| format!("{}: truncated: {e}", path.display()))?;
        let t = i64::from_le_bytes(rec[..8].try_into().expect("8 bytes"));
        if cut.is_some_and(|c| t >= c) {
            break;
        }
        if bars.last().is_some_and(|b: &Bar| b.t >= t) {
            return Err(format!("{}: bars not ascending", path.display()));
        }
        bars.push(Bar {
            t,
            o: rd(&rec, 0),
            h: rd(&rec, 1),
            l: rd(&rec, 2),
            c: rd(&rec, 3),
            v: rd(&rec, 4),
        });
    }
    Ok(bars)
}

/// One symbol's series.
#[derive(Debug, Clone, PartialEq)]
pub struct SymbolSeries {
    /// Symbol.
    pub symbol: String,
    /// Bars (ascending).
    pub bars: Vec<Bar>,
}

/// Load every `<dir>/<period>/*.bars`, sorted by symbol, cut at `cut` (when given).
/// Symbols with no bar left are dropped.
///
/// # Errors
/// Missing directory, unreadable file.
pub fn load(dir: &Path, period: Period, cut: Option<i64>) -> Result<Vec<SymbolSeries>, String> {
    let pdir = dir.join(period.name());
    let rd = std::fs::read_dir(&pdir).map_err(|e| format!("{}: {e}", pdir.display()))?;
    let mut files: Vec<PathBuf> = rd
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "bars"))
        .collect();
    files.sort();
    let mut out = Vec::new();
    for p in files {
        let symbol = p
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_owned();
        let bars = read_series(&p, cut)?;
        if !bars.is_empty() {
            out.push(SymbolSeries { symbol, bars });
        }
    }
    Ok(out)
}

/// Load for a run: calibration cuts at the split (whatever directory it is given);
/// test reads everything and requires bars in the test range.
///
/// # Errors
/// As [`load`]; test split without test bars.
pub fn load_for(dir: &Path, period: Period, split: Split) -> Result<Vec<SymbolSeries>, String> {
    let r = ranges(period);
    match split {
        Split::Calib => load(dir, period, Some(r.cut)),
        Split::Test => {
            let v = load(dir, period, Some(r.test_end))?;
            if !v
                .iter()
                .any(|s| s.bars.last().is_some_and(|b| b.t >= r.cut))
            {
                return Err(format!(
                    "{}: sınama aralığında bar yok (--split test tam kopyayı ister)",
                    dir.display()
                ));
            }
            Ok(v)
        }
    }
}

// ---------------------------------------------------------------------------
// prepare
// ---------------------------------------------------------------------------

/// One REST row.
#[derive(Debug, Clone, Deserialize)]
struct RestRow {
    time: i64,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: f64,
}

/// Aggregate counts of one period's preparation (no symbol names, no prices).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PrepStats {
    /// REST files read.
    pub files: u64,
    /// Symbols written.
    pub symbols: u64,
    /// Raw rows read.
    pub rows: u64,
    /// Duplicate rows (same `time`) dropped.
    pub duplicates: u64,
    /// Of those, with different values (first file in name order kept).
    pub duplicate_conflicts: u64,
    /// Closing-auction bars merged into the day's previous bar.
    pub closing_merged: u64,
    /// Rows dropped: outside every session / not a trading day / not aligned / invalid.
    pub dropped: u64,
    /// Bars written to the full copy.
    pub bars_full: u64,
    /// Bars written to the calibration copy.
    pub bars_calib: u64,
    /// Trading days with at least one bar (full).
    pub days_full: u64,
    /// Same, calibration copy.
    pub days_calib: u64,
    /// First and last local day (full), `YYYY-MM-DD`.
    pub first_day: String,
    /// Last local day (full).
    pub last_day: String,
}

/// `prepare` manifest.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    /// Format note.
    pub format: String,
    /// Which copy this is (`full` / `calib`).
    pub copy: String,
    /// Per period.
    pub periods: BTreeMap<String, PrepStats>,
    /// Universe size, when a universe file was given.
    pub universe: Option<u64>,
    /// Per period: universe symbols without data (count).
    pub missing: BTreeMap<String, u64>,
}

fn valid(r: &RestRow) -> bool {
    [r.open, r.high, r.low, r.close, r.volume]
        .iter()
        .all(|x| x.is_finite())
        && r.open > 0.0
        && r.high > 0.0
        && r.low > 0.0
        && r.close > 0.0
}

/// Parse a REST file name `<SYM>.<period>.<start>.<end>.adj.json`.
#[must_use]
pub fn parse_rest_name(name: &str) -> Option<(String, String)> {
    let stem = name.strip_suffix(".adj.json")?;
    let parts: Vec<&str> = stem.split('.').collect();
    if parts.len() != 4 {
        return None;
    }
    Some((parts[0].to_owned(), parts[1].to_owned()))
}

/// REST rows (ascending, deduplicated) → K14 bars for `period` (5m, 1h, 1d).
pub fn align_rest(
    cal: &ExchangeCalendar,
    period: Period,
    rows: &[(i64, Bar)],
    st: &mut PrepStats,
) -> Vec<Bar> {
    let mut out: Vec<Bar> = Vec::new();
    for (_, raw) in rows {
        let day = cal.local_day(raw.t);
        let Some(span) = cal.day_span(day) else {
            st.dropped += 1;
            continue;
        };
        if period == Period::D1 {
            out.push(Bar {
                t: span.open,
                ..raw.clone()
            });
            continue;
        }
        if raw.t >= span.close {
            // Closing auction (18:00 bar, or after a half day's close): joins the day's
            // previous bar (DATA_ACCESS.md, K14).
            match out.last_mut() {
                Some(last) if cal.local_day(last.t) == day => {
                    last.h = last.h.max(raw.h);
                    last.l = last.l.min(raw.l);
                    last.c = raw.c;
                    last.v += raw.v;
                    st.closing_merged += 1;
                }
                _ => st.dropped += 1,
            }
            continue;
        }
        match cal.bar_at(period.interval(), raw.t) {
            Some(s) if s.open == raw.t => out.push(raw.clone()),
            _ => st.dropped += 1,
        }
    }
    // Daily: two rows mapping to one day cannot happen after dedup by `time`, but guard.
    out.dedup_by(|b, a| {
        if a.t == b.t {
            st.dropped += 1;
            true
        } else {
            false
        }
    });
    out
}

/// Read every REST file of `token` under `rest`, grouped by symbol: rows ascending,
/// duplicates by `time` removed (first file in name order wins).
fn read_rest(
    rest: &Path,
    token: &str,
    st: &mut PrepStats,
) -> Result<BTreeMap<String, Vec<(i64, Bar)>>, String> {
    let mut names: Vec<String> = std::fs::read_dir(rest)
        .map_err(|e| format!("{}: {e}", rest.display()))?
        .filter_map(Result::ok)
        .filter_map(|e| e.file_name().to_str().map(str::to_owned))
        .filter(|n| parse_rest_name(n).is_some_and(|(_, p)| p == token))
        .collect();
    names.sort();
    let mut by: BTreeMap<String, BTreeMap<i64, Bar>> = BTreeMap::new();
    for n in names {
        let (sym, _) = parse_rest_name(&n).expect("filtered");
        let text = std::fs::read_to_string(rest.join(&n)).map_err(|e| format!("{n}: {e}"))?;
        let rows: Vec<RestRow> = serde_json::from_str(&text).map_err(|e| format!("{n}: {e}"))?;
        st.files += 1;
        let m = by.entry(sym).or_default();
        for r in rows {
            st.rows += 1;
            if !valid(&r) {
                st.dropped += 1;
                continue;
            }
            let b = Bar {
                t: r.time,
                o: r.open,
                h: r.high.max(r.open).max(r.close),
                l: r.low.min(r.open).min(r.close),
                c: r.close,
                v: r.volume,
            };
            if let Some(prev) = m.get(&r.time) {
                st.duplicates += 1;
                if *prev != b {
                    st.duplicate_conflicts += 1;
                }
                continue;
            }
            m.insert(r.time, b);
        }
    }
    Ok(by
        .into_iter()
        .map(|(s, m)| (s, m.into_iter().collect()))
        .collect())
}

fn count_days(
    cal: &ExchangeCalendar,
    series: &BTreeMap<String, Vec<Bar>>,
) -> (u64, Option<i64>, Option<i64>) {
    let mut days = std::collections::BTreeSet::new();
    for bars in series.values() {
        for b in bars {
            days.insert(cal.local_day(b.t));
        }
    }
    (
        days.len() as u64,
        days.first().copied(),
        days.last().copied(),
    )
}

fn write_period(
    dir: &Path,
    period: Period,
    series: &BTreeMap<String, Vec<Bar>>,
) -> Result<u64, String> {
    let pdir = dir.join(period.name());
    if pdir.exists() {
        std::fs::remove_dir_all(&pdir).map_err(|e| format!("{}: {e}", pdir.display()))?;
    }
    std::fs::create_dir_all(&pdir).map_err(|e| format!("{}: {e}", pdir.display()))?;
    let mut n = 0u64;
    for (sym, bars) in series {
        if bars.is_empty() {
            continue;
        }
        std::fs::write(pdir.join(format!("{sym}.bars")), encode(bars))
            .map_err(|e| format!("{sym}: {e}"))?;
        n += bars.len() as u64;
    }
    Ok(n)
}

/// `prepare`: REST files → `<out>/full/<period>/` and the physically separate
/// calibration copy `<out>/calib/<period>/` (no bar at or after the cut).
///
/// # Errors
/// I/O, malformed REST files.
pub fn prepare(rest: &Path, out: &Path, universe: Option<&Path>) -> Result<Manifest, String> {
    let cal = xist();
    let uni: Option<Vec<String>> = match universe {
        Some(p) => Some(
            std::fs::read_to_string(p)
                .map_err(|e| format!("{}: {e}", p.display()))?
                .lines()
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .map(str::to_owned)
                .collect(),
        ),
        None => None,
    };
    let mut full_m = Manifest {
        format: "rp4-harness series v1 (data.rs)".to_owned(),
        copy: "full".to_owned(),
        universe: uni.as_ref().map(|u| u.len() as u64),
        ..Manifest::default()
    };
    let mut calib_m = Manifest {
        copy: "calib".to_owned(),
        ..full_m.clone()
    };
    let mut five: BTreeMap<String, Vec<Bar>> = BTreeMap::new();
    for period in Period::ALL {
        let mut st = PrepStats::default();
        let series: BTreeMap<String, Vec<Bar>> = if let Some(token) = period.rest_token() {
            read_rest(rest, token, &mut st)?
                .into_iter()
                .map(|(s, rows)| {
                    let bars = align_rest(&cal, period, &rows, &mut st);
                    (s, bars)
                })
                .filter(|(_, b)| !b.is_empty())
                .collect()
        } else {
            // 15 minutes from the aligned 5-minute bars (calendar resample, K14).
            let mut m = BTreeMap::new();
            for (s, b) in &five {
                st.rows += b.len() as u64;
                let (bars, dropped) = cal
                    .resample(Interval::Minutes(15), b)
                    .map_err(|e| format!("{s}: {e}"))?;
                st.dropped += dropped as u64;
                m.insert(s.clone(), bars);
            }
            m
        };
        if period == Period::M5 {
            five = series.clone();
        }
        let cut = ranges(period).cut;
        let calib: BTreeMap<String, Vec<Bar>> = series
            .iter()
            .map(|(s, b)| {
                (
                    s.clone(),
                    b.iter().filter(|x| x.t < cut).cloned().collect::<Vec<_>>(),
                )
            })
            .filter(|(_, b)| !b.is_empty())
            .collect();
        st.symbols = series.len() as u64;
        st.bars_full = write_period(&out.join("full"), period, &series)?;
        let (d, first, last) = count_days(&cal, &series);
        st.days_full = d;
        st.first_day = first.map(crate::common::day_str).unwrap_or_default();
        st.last_day = last.map(crate::common::day_str).unwrap_or_default();
        let mut cst = st.clone();
        cst.bars_calib = write_period(&out.join("calib"), period, &calib)?;
        st.bars_calib = cst.bars_calib;
        let (dc, firstc, lastc) = count_days(&cal, &calib);
        st.days_calib = dc;
        cst.days_calib = dc;
        cst.symbols = calib.len() as u64;
        cst.bars_full = 0;
        cst.days_full = 0;
        cst.first_day = firstc.map(crate::common::day_str).unwrap_or_default();
        cst.last_day = lastc.map(crate::common::day_str).unwrap_or_default();
        if let Some(u) = &uni {
            let miss = u.iter().filter(|s| !series.contains_key(*s)).count() as u64;
            full_m.missing.insert(period.name().to_owned(), miss);
            let missc = u.iter().filter(|s| !calib.contains_key(*s)).count() as u64;
            calib_m.missing.insert(period.name().to_owned(), missc);
        }
        full_m.periods.insert(period.name().to_owned(), st);
        calib_m.periods.insert(period.name().to_owned(), cst);
    }
    write_file(&out.join("full/manifest.json"), &to_json(&full_m)).map_err(|e| e.to_string())?;
    write_file(&out.join("calib/manifest.json"), &to_json(&calib_m)).map_err(|e| e.to_string())?;
    Ok(full_m)
}
