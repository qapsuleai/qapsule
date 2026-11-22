//! `qp live compare-trades` and `qp live price-steps` (RP4/T2, M27).
//!
//! - `compare-trades`: bars built from the cached REST trades (`fixtures/local/trades/`,
//!   level 1) with the live path's own bar builder ([`qp_source::LiveBars`],
//!   [`BarMethod::Trades`], events applied to a [`FeedStore`]; no separate bar code)
//!   against the settled, unadjusted REST bars (`qp live bars`), in the columns of the R6
//!   M22 table. Variants measured: the time field (`timestamp_ns` or `date_time`), the
//!   session edges (opening auction and closing session into the first / last bar, or
//!   dropped), special trades (`trade_ex`) in or out, the volume unit (TL or lots) and the
//!   REST price form (as sent, or rounded to the cent). With `--raw` it also compares the
//!   three bar sources of M27 (price card, trades, mixed) on the bars a card recording
//!   covers.
//! - `price-steps`: the price card's `priceStep` against the BIST price-step table, and
//!   how many trade prices sit on that table's grid.
//!
//! Only aggregate counts are printed (K23): no symbol, no price, no volume.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Read as _;
use std::path::{Path, PathBuf};

use clap::Args;
use qp_calendar::{ExchangeCalendar, Interval};
use qp_matriks::trade::{parse_rest_csv, Trade, TradeKey};
use qp_schema::data::FeedKey;
use qp_schema::Bar;
use qp_source::{
    compare_bars_with, BarDiff, BarMethod, DiffTolerance, FeedStore, LiveBars, LiveConfig,
    LiveStats, SpecialTrades, VolumeUnit,
};

use crate::live::{
    build_live, load_rest_with, merge_diff, parse_symbols, read_recordings, split_list, xist,
    FeedArgs,
};

mod compare;
mod export;

pub use compare::*;
pub use export::*;

const NS_PER_S: i64 = 1_000_000_000;

/// `qp live compare-trades` (M27).
#[derive(Debug, Args)]
pub struct CompareTradesArgs {
    /// Symbols: comma-separated or `@file`.
    #[arg(long, default_value = "@fixtures/local/live/u30.txt")]
    pub symbols: String,
    /// Trade cache (`<dir>/<SYM>/<a>_<b>.csv.gz`).
    #[arg(long, default_value = "fixtures/local/trades")]
    pub trades_dir: PathBuf,
    /// Special-trade cache (`trade_ex`, same layout); a missing directory means none.
    #[arg(long, default_value = "fixtures/local/trades_ex")]
    pub trades_ex_dir: PathBuf,
    /// REST bars directory (`<SYM>.<period>.<start>.<end>.unadj.json`).
    #[arg(long, default_value = "fixtures/local/live/rest")]
    pub rest_dir: PathBuf,
    /// First local trading day compared (`YYYY-MM-DD`).
    #[arg(long, default_value = "2026-10-05")]
    pub from: String,
    /// Last local trading day compared.
    #[arg(long, default_value = "2026-11-20")]
    pub to: String,
    /// Intervals.
    #[arg(long, default_value = "1m,5m,1h,1d")]
    pub intervals: String,
    /// Trade time: `ns` (`date` + `timestamp_ns`, nanoseconds since local midnight) and/or
    /// `date_time` (the whole-second `date_time` column).
    #[arg(long, default_value = "ns")]
    pub time: String,
    /// Session edges: `session` (the live rule: opening auction into the first bar,
    /// closing session into the last bar) and/or `none` (trades outside the session
    /// dropped).
    #[arg(long, default_value = "session")]
    pub edges: String,
    /// Special trades: `day_only` (the live default: the `1d` bar only), `exclude`,
    /// `include` (every interval).
    #[arg(long, default_value = "day_only")]
    pub special: String,
    /// Volume units: `value` (TL, Σ price·quantity vs REST `volume`) and/or `quantity`
    /// (lots vs REST `quantity`).
    #[arg(long, default_value = "value,quantity")]
    pub volume_units: String,
    /// REST prices: `raw` (as sent) and/or `mill` (rounded to 0.001, see [`to_mill`]).
    #[arg(long, default_value = "raw,mill")]
    pub rest_price: String,
    /// Where REST's closing-session bar (18:00+) goes: `last` (the day's last session
    /// bar, the live rule; created if REST has none there) or `prev` (the previous REST
    /// bar, `qp live compare` / R3).
    #[arg(long, default_value = "last")]
    pub rest_close: String,
    /// Absolute price tolerance ("within tolerance" columns).
    #[arg(long, default_value_t = 1e-6)]
    pub price_tol: f64,
    /// Relative volume tolerance ("within tolerance" columns).
    #[arg(long, default_value_t = 0.01)]
    pub volume_tol: f64,
    /// Also print, per row, matched and all-exact bars per trading day.
    #[arg(long)]
    pub by_day: bool,
    /// Also print, per row, where the bars that are not all-exact sit (first / last /
    /// other bar of the day) and which fields differ.
    #[arg(long)]
    pub detail: bool,
    /// Price-card raw logs (comma-separated): adds the M27 source table (card, trades,
    /// mixed) on the bars these recordings cover.
    #[arg(long)]
    pub raw: Option<String>,
    /// Card bar method for the source table (`ticks` or `mixed`).
    #[arg(long, default_value = "mixed")]
    pub card_method: String,
}

/// `qp live export-trade-bars`.
#[derive(Debug, Args)]
pub struct ExportTradeBarsArgs {
    /// Symbols: comma-separated or `@file`.
    #[arg(long, default_value = "@fixtures/local/live/u100.txt")]
    pub symbols: String,
    /// Trade cache.
    #[arg(long, default_value = "fixtures/local/trades")]
    pub trades_dir: PathBuf,
    /// Special-trade cache (`trade_ex`).
    #[arg(long, default_value = "fixtures/local/trades_ex")]
    pub trades_ex_dir: PathBuf,
    /// First local day.
    #[arg(long, default_value = "2026-10-05")]
    pub from: String,
    /// Last local day.
    #[arg(long, default_value = "2026-11-20")]
    pub to: String,
    /// Output directory (`<out>/<period>/<SYM>.bars`, `<out>/manifest.json`).
    #[arg(long, default_value = "fixtures/local/rp4/tradebars/full")]
    pub out: PathBuf,
    /// Harness series to check against (`<dir>/<period>/<SYM>.bars`, REST `full/`).
    #[arg(long)]
    pub check: Option<PathBuf>,
    /// First local day of the check (the test range).
    #[arg(long, default_value = "2026-10-19")]
    pub check_from: String,
}

/// `qp live price-steps`.
#[derive(Debug, Args)]
pub struct PriceStepsArgs {
    /// Price-card raw logs (comma-separated).
    #[arg(long)]
    pub raw: String,
    /// Symbols whose trades are checked against the grid (comma-separated or `@file`).
    #[arg(long, default_value = "@fixtures/local/live/u30.txt")]
    pub symbols: String,
    /// Trade cache.
    #[arg(long, default_value = "fixtures/local/trades")]
    pub trades_dir: PathBuf,
}

/// Which clock a trade gets (`--time`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum TimeField {
    Ns,
    DateTime,
}

impl TimeField {
    fn parse(s: &str) -> Result<Self, String> {
        match s {
            "ns" | "timestamp_ns" => Ok(Self::Ns),
            "date_time" | "dt" => Ok(Self::DateTime),
            _ => Err(format!("time `{s}`: expected ns or date_time")),
        }
    }
    fn name(self) -> &'static str {
        match self {
            Self::Ns => "ns",
            Self::DateTime => "date_time",
        }
    }
}

/// BIST equity price step for a price level (`docs/matriks`; RP4 T2 table). The upper
/// bound of each band is exclusive (a price of exactly 20 trades on the 0.02 grid, which
/// also is a 0.01 grid point).
#[must_use]
pub fn bist_price_step(price: f64) -> f64 {
    const TABLE: [(f64, f64); 7] = [
        (20.0, 0.01),
        (50.0, 0.02),
        (100.0, 0.05),
        (250.0, 0.10),
        (500.0, 0.25),
        (1000.0, 0.50),
        (2500.0, 1.00),
    ];
    TABLE
        .iter()
        .find(|(hi, _)| price < *hi)
        .map_or(2.50, |(_, s)| *s)
}

/// Whether `price` is a whole multiple of `step` (to 1e-6 of a step).
#[must_use]
pub fn on_grid(price: f64, step: f64) -> bool {
    let k = price / step;
    (k - k.round()).abs() < 1e-6
}

/// The price at three decimal places (`round(x·1000)/1000`, which is the `f64` the
/// decimal text parses to). REST sends some prices as a single-precision value printed
/// with extra digits (e.g. `…9.89999` for `…9.9`, at most 1e-5 away); trade prices have
/// at most three decimals, so this recovers REST's decimal price without touching any
/// real price.
#[must_use]
pub fn to_mill(x: f64) -> f64 {
    (x * 1000.0).round() / 1000.0
}

/// Rewrites a level-1 trade CSV so that `timestamp_ns` is the whole-second clock of the
/// `date_time` column (for the time-field measurement). Also returns the distribution of
/// `date_time − floor(timestamp_ns)` in seconds (clamped to ±5; `i64::MAX` = unparsable),
/// and how many rows have `timestamp_ns` inside one day of nanoseconds.
fn date_time_clock(csv: &[u8]) -> (Vec<u8>, BTreeMap<i64, u64>, u64) {
    let text = String::from_utf8_lossy(csv);
    let mut lines = text.lines();
    let Some(header) = lines.next() else {
        return (Vec::new(), BTreeMap::new(), 0);
    };
    let cols: Vec<&str> = header.split(',').map(str::trim).collect();
    let (Some(i_dt), Some(i_ns)) = (
        cols.iter().position(|c| *c == "date_time"),
        cols.iter().position(|c| *c == "timestamp_ns"),
    ) else {
        return (csv.to_vec(), BTreeMap::new(), 0);
    };
    let mut lag: BTreeMap<i64, u64> = BTreeMap::new();
    let mut in_day = 0u64;
    let mut out = String::with_capacity(text.len());
    out.push_str(header);
    out.push('\n');
    for l in lines {
        let mut f: Vec<String> = l.split(',').map(str::to_owned).collect();
        let sec = f.get(i_dt).and_then(|dt| {
            let h: i64 = dt.get(11..13)?.parse().ok()?;
            let m: i64 = dt.get(14..16)?.parse().ok()?;
            let s: i64 = dt.get(17..19)?.parse().ok()?;
            Some(h * 3600 + m * 60 + s)
        });
        let ns = f.get(i_ns).and_then(|s| s.trim().parse::<i64>().ok());
        if let Some(ns) = ns {
            if (0..86_400 * NS_PER_S).contains(&ns) {
                in_day += 1;
            }
        }
        let key = match (sec, ns) {
            (Some(s), Some(n)) => (s - n.div_euclid(NS_PER_S)).clamp(-5, 5),
            _ => i64::MAX,
        };
        *lag.entry(key).or_default() += 1;
        if let (Some(s), Some(slot)) = (sec, f.get_mut(i_ns)) {
            *slot = (s * NS_PER_S).to_string();
        }
        out.push_str(&f.join(","));
        out.push('\n');
    }
    (out.into_bytes(), lag, in_day)
}

/// Counters of the trade loading (no values).
#[derive(Debug, Default, Clone)]
struct LoadStats {
    files: u64,
    rows: u64,
    bad: u64,
    duplicates: u64,
    special: u64,
    special_also_regular: u64,
    out_of_range: u64,
    ns_in_day: u64,
    lag: BTreeMap<i64, u64>,
}

fn read_gz(path: &Path) -> Result<Vec<u8>, String> {
    let f = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut out = Vec::new();
    flate2::read::GzDecoder::new(f)
        .read_to_end(&mut out)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(out)
}

fn csv_files(dir: &Path) -> Vec<PathBuf> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut v: Vec<PathBuf> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.to_string_lossy().ends_with(".csv.gz"))
        .collect();
    v.sort();
    v
}

/// Whether a cache file `<a>_<b>.csv.gz` can hold days in `[d0, d1]` (an unrecognized
/// name is read).
fn window_overlaps(path: &Path, days: (i64, i64)) -> bool {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let stem = name.trim_end_matches(".csv.gz");
    let Some((a, b)) = stem.split_once('_') else {
        return true;
    };
    match (parse_day(a), parse_day(b)) {
        (Ok(a), Ok(b)) => b >= days.0 && a <= days.1,
        _ => true,
    }
}

/// Trades of `sym` on local days `[d0, d1]`, regular then special, de-duplicated by
/// (local day, trade number), in exchange order.
fn load_trades(
    a: &CompareTradesArgs,
    cal: &ExchangeCalendar,
    sym: &str,
    days: (i64, i64),
    time: TimeField,
    st: &mut LoadStats,
) -> Result<Vec<Trade>, String> {
    let mut seen: BTreeSet<(i64, TradeKey)> = BTreeSet::new();
    let mut out: Vec<Trade> = Vec::new();
    for (special, dir) in [(false, &a.trades_dir), (true, &a.trades_ex_dir)] {
        for p in csv_files(&dir.join(sym)) {
            if !window_overlaps(&p, days) {
                continue;
            }
            st.files += 1;
            let raw = read_gz(&p)?;
            let csv = match time {
                TimeField::Ns => {
                    // Measure the date_time ↔ timestamp_ns relation on the side.
                    let (_, lag, in_day) = date_time_clock(&raw);
                    for (k, n) in lag {
                        *st.lag.entry(k).or_default() += n;
                    }
                    st.ns_in_day += in_day;
                    raw
                }
                TimeField::DateTime => date_time_clock(&raw).0,
            };
            let (trades, bad) = parse_rest_csv(&csv);
            st.bad += bad as u64;
            for mut t in trades {
                st.rows += 1;
                let day = cal.local_day(t.t_ms());
                if day < days.0 || day > days.1 {
                    st.out_of_range += 1;
                    continue;
                }
                t.is_trade_ex = special;
                if special {
                    st.special += 1;
                }
                if !seen.insert((day, t.dedup_key())) {
                    if special {
                        st.special_also_regular += 1;
                    } else {
                        st.duplicates += 1;
                    }
                    continue;
                }
                out.push(t);
            }
        }
    }
    out.sort_by(Trade::exchange_cmp);
    Ok(out)
}

/// Bars of `trades` through the live builder ([`BarMethod::Trades`]) and a [`FeedStore`].
fn build_trade_bars(
    trades: &[Trade],
    sym: &str,
    iv: &str,
    cal: &ExchangeCalendar,
    cfg: LiveConfig,
) -> Result<(Vec<Bar>, LiveStats), String> {
    let key = FeedKey {
        symbol: sym.to_owned(),
        interval: iv.to_owned(),
        feed: None,
    };
    let mut lb = LiveBars::new(key.clone(), cal.clone(), cfg).map_err(|e| e.to_string())?;
    let mut store = FeedStore::new(key);
    for t in trades {
        for e in lb.push_trade(&t.tick()) {
            store
                .apply(&e)
                .map_err(|err| format!("trade event does not apply: {err}"))?;
        }
    }
    Ok((store.bars().to_vec(), lb.stats().clone()))
}

fn trade_config(unit: VolumeUnit, special: SpecialTrades, edges: &str) -> LiveConfig {
    let mut cfg = LiveConfig {
        method: BarMethod::Trades,
        volume: unit,
        special_trades: special,
        ..LiveConfig::default()
    };
    if edges == "none" {
        cfg.pre_open_ms = 0;
        cfg.post_close_ms = 0;
    }
    cfg
}

fn parse_day(s: &str) -> Result<i64, String> {
    let p: Vec<&str> = s.split('-').collect();
    let bad = || format!("date `{s}`: expected YYYY-MM-DD");
    if p.len() != 3 {
        return Err(bad());
    }
    let y: i64 = p[0].parse().map_err(|_| bad())?;
    let m: u32 = p[1].parse().map_err(|_| bad())?;
    let d: u32 = p[2].parse().map_err(|_| bad())?;
    Ok(qp_calendar::days_from_civil(y, m, d))
}

fn day_name(day: i64) -> String {
    let (y, m, d) = qp_calendar::civil_from_days(day);
    format!("{y:04}-{m:02}-{d:02}")
}

/// REST bars of one symbol × interval × volume unit, limited to `days`.
fn rest_bars(
    a: &CompareTradesArgs,
    cal: &ExchangeCalendar,
    sym: &str,
    iv: &str,
    quantity: bool,
    days: (i64, i64),
) -> Result<Option<Vec<Bar>>, String> {
    Ok(load_rest_with(
        &a.rest_dir,
        sym,
        iv,
        false,
        quantity,
        cal,
        a.rest_close == "last",
    )?
    .map(|v| {
        v.into_iter()
            .filter(|b| {
                let d = cal.local_day(b.t);
                d >= days.0 && d <= days.1
            })
            .collect()
    }))
}

fn mill_bars(bars: &[Bar]) -> Vec<Bar> {
    bars.iter()
        .map(|b| Bar {
            o: to_mill(b.o),
            h: to_mill(b.h),
            l: to_mill(b.l),
            c: to_mill(b.c),
            ..b.clone()
        })
        .collect()
}

#[cfg(test)]
mod tests;
