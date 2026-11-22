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

/// One row of the table.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct RowKey {
    iv_rank: u64,
    iv: String,
    time: TimeField,
    edges: String,
    special: String,
    unit: String,
    rest_price: String,
}

/// Where the differing bars sit (`--detail`).
#[derive(Debug, Default, Clone)]
struct Detail {
    /// (position, field set) → bars.
    pos: BTreeMap<(&'static str, String), u64>,
    /// day → (matched, all exact).
    by_day: BTreeMap<i64, (u64, u64, u64)>,
    /// symbols with at least one differing bar → differing bars (anonymous).
    per_symbol: BTreeMap<String, u64>,
}

fn bits(x: f64, y: f64) -> bool {
    x.to_bits() == y.to_bits()
}

fn detail_add(d: &mut Detail, cal: &ExchangeCalendar, sym: &str, live: &[Bar], rest: &[Bar]) {
    let rest: BTreeMap<i64, &Bar> = rest.iter().map(|b| (b.t, b)).collect();
    // First / last bar of each day on the REST side.
    let mut first: BTreeMap<i64, i64> = BTreeMap::new();
    let mut last: BTreeMap<i64, i64> = BTreeMap::new();
    for t in rest.keys() {
        let day = cal.local_day(*t);
        first.entry(day).or_insert(*t);
        last.insert(day, *t);
    }
    for b in live {
        let day = cal.local_day(b.t);
        let e = d.by_day.entry(day).or_default();
        let Some(r) = rest.get(&b.t) else {
            e.2 += 1;
            continue;
        };
        e.0 += 1;
        let f = [
            ('o', bits(b.o, r.o)),
            ('h', bits(b.h, r.h)),
            ('l', bits(b.l, r.l)),
            ('c', bits(b.c, r.c)),
            ('v', bits(b.v, r.v)),
        ];
        if f.iter().all(|(_, ok)| *ok) {
            e.1 += 1;
            continue;
        }
        let pos = if first.get(&day) == Some(&b.t) {
            "first"
        } else if last.get(&day) == Some(&b.t) {
            "last"
        } else {
            "other"
        };
        let fields: String = f.iter().filter(|(_, ok)| !ok).map(|(c, _)| *c).collect();
        *d.pos.entry((pos, fields)).or_default() += 1;
        *d.per_symbol.entry(sym.to_owned()).or_default() += 1;
    }
}

fn iv_rank(iv: &str) -> u64 {
    Interval::parse(iv).map_or(u64::MAX, |i| match i {
        Interval::Minutes(n) => u64::from(n),
        Interval::Day => 1_000_000,
        _ => 2_000_000,
    })
}

fn print_header(first: &str) {
    println!(
        "{first} | {:>7} {:>6} {:>6} | {:>6} {:>6} | exact% o/h/l/c/v               | v≤tol% | max rel price / v | v≤1e-6% all=p,v≤1e-6% v=f32%",
        "matched", "l-only", "r-only", "all=", "all~"
    );
}

/// Volume agreement beyond bit equality: REST's TL volume carries about seven
/// significant digits, so a Σ price·quantity in `f64` rarely equals it bit for bit.
#[derive(Debug, Clone, Copy, Default)]
struct VolExtra {
    /// `|v − rest| ≤ 1e-6 · rest`.
    rel6: u64,
    /// All four prices bit-equal and `rel6`.
    all6: u64,
    /// `f32(v) == rest` (REST's number is the single-precision value).
    f32_eq: u64,
}

impl VolExtra {
    fn add(&mut self, live: &[Bar], rest: &[Bar]) {
        let rest: BTreeMap<i64, &Bar> = rest.iter().map(|b| (b.t, b)).collect();
        for b in live {
            let Some(r) = rest.get(&b.t) else {
                continue;
            };
            let rel6 = (b.v - r.v).abs() <= 1e-6 * r.v.abs();
            if rel6 {
                self.rel6 += 1;
            }
            if rel6 && bits(b.o, r.o) && bits(b.h, r.h) && bits(b.l, r.l) && bits(b.c, r.c) {
                self.all6 += 1;
            }
            #[allow(clippy::cast_possible_truncation)]
            if f64::from(b.v as f32) == r.v {
                self.f32_eq += 1;
            }
        }
    }
}

fn print_row(label: &str, d: &BarDiff, x: &VolExtra) {
    let pc = |n: u64| 100.0 * d.ratio(n);
    let f = [d.o, d.h, d.l, d.c, d.v];
    let max_price = [d.o, d.h, d.l, d.c]
        .iter()
        .map(|x| x.max_rel)
        .fold(0.0_f64, f64::max);
    println!(
        "{label} | {:>7} {:>6} {:>6} | {:>5.2}% {:>5.2}% | {} | {:>6.2} | {:.1e} / {:.1e} | {:>6.2} {:>6.2} {:>6.2}",
        d.matched,
        d.live_only,
        d.rest_only,
        pc(d.all_exact),
        pc(d.all_within_tol),
        f.iter()
            .map(|x| format!("{:>5.2}", pc(x.exact)))
            .collect::<Vec<_>>()
            .join(" "),
        pc(d.v.within_tol),
        max_price,
        d.v.max_rel,
        pc(x.rel6),
        pc(x.all6),
        pc(x.f32_eq),
    );
}

/// `qp live compare-trades`.
#[allow(clippy::too_many_lines)]
pub fn compare_trades(a: &CompareTradesArgs) -> Result<(), String> {
    let cal = xist()?;
    let syms = parse_symbols(&a.symbols)?;
    let days = (parse_day(&a.from)?, parse_day(&a.to)?);
    if days.0 > days.1 {
        return Err("--from is after --to".into());
    }
    if a.rest_close != "last" && a.rest_close != "prev" {
        return Err(format!(
            "rest close `{}`: expected last or prev",
            a.rest_close
        ));
    }
    let intervals = split_list(&a.intervals);
    for iv in &intervals {
        Interval::parse(iv).map_err(|e| format!("interval `{iv}`: {e}"))?;
    }
    let times: Vec<TimeField> = split_list(&a.time)
        .iter()
        .map(|s| TimeField::parse(s))
        .collect::<Result<_, _>>()?;
    let edges = split_list(&a.edges);
    for e in &edges {
        if e != "session" && e != "none" {
            return Err(format!("edges `{e}`: expected session or none"));
        }
    }
    let specials: Vec<(String, SpecialTrades)> = split_list(&a.special)
        .into_iter()
        .map(|s| match s.as_str() {
            "exclude" => Ok((s, SpecialTrades::Exclude)),
            "include" => Ok((s, SpecialTrades::Include)),
            "day_only" => Ok((s, SpecialTrades::DayOnly)),
            _ => Err(format!(
                "special `{s}`: expected day_only, exclude or include"
            )),
        })
        .collect::<Result<_, _>>()?;
    let units: Vec<(String, VolumeUnit)> = split_list(&a.volume_units)
        .into_iter()
        .map(|s| match s.as_str() {
            "value" => Ok((s, VolumeUnit::Value)),
            "quantity" => Ok((s, VolumeUnit::Quantity)),
            _ => Err(format!("volume unit `{s}`: expected value or quantity")),
        })
        .collect::<Result<_, _>>()?;
    let rest_prices = split_list(&a.rest_price);
    for r in &rest_prices {
        if r != "raw" && r != "mill" {
            return Err(format!("rest price `{r}`: expected raw or mill"));
        }
    }
    let tol = DiffTolerance {
        price_abs: a.price_tol,
        volume_rel: a.volume_tol,
    };

    let mut rows: BTreeMap<RowKey, BarDiff> = BTreeMap::new();
    let mut extras: BTreeMap<RowKey, VolExtra> = BTreeMap::new();
    let mut details: BTreeMap<RowKey, Detail> = BTreeMap::new();
    let mut load = LoadStats::default();
    let mut bstats = LiveStats::default();
    let mut used_syms = 0usize;
    let mut trade_days: BTreeSet<i64> = BTreeSet::new();
    let mut rest_off_mill = 0u64;
    let mut rest_prices_n = 0u64;
    let mut rest_off_mill_max = 0.0_f64;
    // Trade bars for the source table: (symbol, interval, unit) → bars (first time,
    // first edges, first special).
    let mut kept: BTreeMap<(String, String, String), Vec<Bar>> = BTreeMap::new();
    for sym in &syms {
        let mut any = false;
        let mut rest_cache: BTreeMap<(String, bool), Option<Vec<Bar>>> = BTreeMap::new();
        for &time in &times {
            let mut st = LoadStats::default();
            let trades = load_trades(a, &cal, sym, days, time, &mut st)?;
            if time == times[0] {
                load.files += st.files;
                load.rows += st.rows;
                load.bad += st.bad;
                load.duplicates += st.duplicates;
                load.special += st.special;
                load.special_also_regular += st.special_also_regular;
                load.out_of_range += st.out_of_range;
                load.ns_in_day += st.ns_in_day;
                for (k, n) in st.lag {
                    *load.lag.entry(k).or_default() += n;
                }
                trade_days.extend(trades.iter().map(|t| cal.local_day(t.t_ms())));
            }
            if trades.is_empty() {
                continue;
            }
            any = true;
            for e in &edges {
                for (sname, special) in &specials {
                    for (uname, unit) in &units {
                        for iv in &intervals {
                            let q = *unit == VolumeUnit::Quantity;
                            let slot = match rest_cache.entry((iv.clone(), q)) {
                                std::collections::btree_map::Entry::Occupied(e) => e.into_mut(),
                                std::collections::btree_map::Entry::Vacant(e) => {
                                    e.insert(rest_bars(a, &cal, sym, iv, q, days)?)
                                }
                            };
                            let Some(rest) = slot.as_ref() else {
                                continue;
                            };
                            let (live, s) = build_trade_bars(
                                &trades,
                                sym,
                                iv,
                                &cal,
                                trade_config(*unit, *special, e),
                            )?;
                            if time == times[0] && e == &edges[0] && sname == &specials[0].0 {
                                if iv == &intervals[0] && uname == &units[0].0 {
                                    add_stats(&mut bstats, &s);
                                }
                                if uname == &units[0].0 {
                                    for b in rest {
                                        for x in [b.o, b.h, b.l, b.c] {
                                            rest_prices_n += 1;
                                            let dx = (x - to_mill(x)).abs();
                                            if dx > 0.0 {
                                                rest_off_mill += 1;
                                                rest_off_mill_max = rest_off_mill_max.max(dx);
                                            }
                                        }
                                    }
                                }
                                kept.insert((sym.clone(), iv.clone(), uname.clone()), live.clone());
                            }
                            for rp in &rest_prices {
                                let r = if rp == "mill" {
                                    mill_bars(rest)
                                } else {
                                    rest.clone()
                                };
                                let key = RowKey {
                                    iv_rank: iv_rank(iv),
                                    iv: iv.clone(),
                                    time,
                                    edges: e.clone(),
                                    special: sname.clone(),
                                    unit: uname.clone(),
                                    rest_price: rp.clone(),
                                };
                                let d = compare_bars_with(&live, &r, &tol);
                                merge_diff(rows.entry(key.clone()).or_default(), &d);
                                extras.entry(key.clone()).or_default().add(&live, &r);
                                if a.detail || a.by_day {
                                    detail_add(
                                        details.entry(key).or_default(),
                                        &cal,
                                        sym,
                                        &live,
                                        &r,
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
        if any {
            used_syms += 1;
        }
    }

    let n_days = trade_days.len();
    println!(
        "M27 compare-trades: {used_syms}/{} symbols with trades, {n_days} trading days ({}..{}), {} files; tol price {} abs, volume {} rel; REST closing bar into the {} bar",
        syms.len(),
        a.from,
        a.to,
        load.files,
        a.price_tol,
        a.volume_tol,
        if a.rest_close == "last" { "last session" } else { "previous REST" }
    );
    println!(
        "  trades: rows {}, unparsed {}, out of range {}, duplicates {}, special {} (also regular {}); builder ({} / first interval): trades {}, special excluded {}, off-session {}, late {}, bars {}",
        load.rows,
        load.bad,
        load.out_of_range,
        load.duplicates,
        load.special,
        load.special_also_regular,
        times[0].name(),
        bstats.trades,
        bstats.special_excluded,
        bstats.off_session,
        bstats.late,
        bstats.bars_opened
    );
    if times[0] == TimeField::Ns {
        let total: u64 = load.lag.values().sum();
        println!(
            "  time: timestamp_ns within [0, 1 day) ns: {}/{}; date_time − floor(timestamp_ns) in s: {}",
            load.ns_in_day,
            total,
            load.lag
                .iter()
                .map(|(k, n)| if *k == i64::MAX {
                    format!("unparsed {n}")
                } else {
                    format!("{k:+}: {n}")
                })
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    println!(
        "  REST prices not on 0.001: {rest_off_mill}/{rest_prices_n} (largest distance {rest_off_mill_max:.1e})"
    );
    print_header(&format!(
        "{:<3} {:<9} {:<7} {:<7} {:<8} {:<4}",
        "iv", "time", "edges", "special", "volume", "rest"
    ));
    for (k, d) in &rows {
        print_row(
            &format!(
                "{:<3} {:<9} {:<7} {:<7} {:<8} {:<4}",
                k.iv,
                k.time.name(),
                k.edges,
                k.special,
                k.unit,
                k.rest_price
            ),
            d,
            &extras.get(k).copied().unwrap_or_default(),
        );
    }
    if a.detail {
        println!("detail: bars not all-exact by position and differing fields; symbols affected");
        for (k, d) in &details {
            let mut syms: Vec<u64> = d.per_symbol.values().copied().collect();
            syms.sort_unstable();
            println!(
                "  {} {} {} {} {} {}: {} | symbols {} (bars per symbol {:?})",
                k.iv,
                k.time.name(),
                k.edges,
                k.special,
                k.unit,
                k.rest_price,
                d.pos
                    .iter()
                    .map(|((p, f), n)| format!("{p}:{f}={n}"))
                    .collect::<Vec<_>>()
                    .join(" "),
                syms.len(),
                syms
            );
        }
    }
    if a.by_day {
        println!("by day (days with a difference): matched / not all-exact / live-only");
        for (k, d) in &details {
            println!(
                "  {} {} {} {} {} {}: {}",
                k.iv,
                k.time.name(),
                k.edges,
                k.special,
                k.unit,
                k.rest_price,
                d.by_day
                    .iter()
                    .filter(|(_, (m, e, lo))| m != e || *lo > 0)
                    .map(|(day, (m, e, lo))| format!("{} {}/{}/{}", day_name(*day), m, m - e, lo))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }
    if let Some(raw) = &a.raw {
        source_table(
            a,
            &cal,
            raw,
            &intervals,
            &units,
            &rest_prices,
            days,
            &kept,
            &tol,
        )?;
    }
    Ok(())
}

fn add_stats(acc: &mut LiveStats, s: &LiveStats) {
    acc.trades += s.trades;
    acc.special_excluded += s.special_excluded;
    acc.off_session += s.off_session;
    acc.late += s.late;
    acc.bars_opened += s.bars_opened;
}

/// M27 source table: price card (R6), trades, mixed (trade bar where trades built one,
/// else the card bar), on the bars the card recording covers (final, outside every
/// connection gap, not a feed's first intraday bar).
#[allow(clippy::too_many_arguments)]
fn source_table(
    a: &CompareTradesArgs,
    cal: &ExchangeCalendar,
    raw: &str,
    intervals: &[String],
    units: &[(String, VolumeUnit)],
    rest_prices: &[String],
    days: (i64, i64),
    kept: &BTreeMap<(String, String, String), Vec<Bar>>,
    tol: &DiffTolerance,
) -> Result<(), String> {
    let recs = read_recordings(raw)?;
    type Key = (u64, String, String, String, &'static str);
    let mut rows: BTreeMap<Key, BarDiff> = BTreeMap::new();
    let mut extras: BTreeMap<Key, VolExtra> = BTreeMap::new();
    let mut cov_days: BTreeMap<i64, u64> = BTreeMap::new();
    // The recording's end: a day bar counts only if the recording saw its close (and the
    // closing session).
    let rec_end_ms = recs.iter().map(|r| r.t_us / 1000).max().unwrap_or(i64::MIN);
    let mut cov_syms: BTreeSet<String> = BTreeSet::new();
    for (uname, _) in units {
        let args = FeedArgs {
            intervals: intervals.join(","),
            method: a.card_method.clone(),
            volume_field: "daily".into(),
            volume_unit: uname.clone(),
            time: "update_date".into(),
            finalize_grace_ms: qp_matriks::feed::DEFAULT_FINALIZE_GRACE_MS,
        };
        let built = build_live(&recs, &args)?;
        for (key, store) in &built.stores {
            let iv = &key.interval;
            let Some(tbars) = kept.get(&(key.symbol.clone(), iv.clone(), uname.clone())) else {
                continue;
            };
            let Some(rest) = rest_bars(a, cal, &key.symbol, iv, uname == "quantity", days)? else {
                continue;
            };
            let interval = Interval::parse(iv).map_err(|e| e.to_string())?;
            let rest_days: BTreeSet<i64> = rest.iter().map(|b| cal.local_day(b.t)).collect();
            let card: Vec<Bar> = store
                .bars()
                .iter()
                .enumerate()
                .filter(|(i, b)| {
                    let in_gap = cal.bar_at(interval, b.t).is_some_and(|s| {
                        built
                            .gaps
                            .iter()
                            .any(|&(g0, g1)| s.open <= g1 && g0 < s.close)
                    });
                    let d = cal.local_day(b.t);
                    let complete = if interval == Interval::Day {
                        cal.day_span(d)
                            .is_some_and(|s| s.close + 30 * 60_000 <= rec_end_ms)
                    } else {
                        built.finals.contains(&(key.clone(), b.t)) && *i > 0
                    };
                    // A day bar from the card's day fields does not depend on gaps
                    // (R6 M22); an intraday bar does.
                    let gap_free = interval == Interval::Day || !in_gap;
                    gap_free && complete && rest_days.contains(&d) && d >= days.0 && d <= days.1
                })
                .map(|(_, b)| b.clone())
                .collect();
            if card.is_empty() {
                continue;
            }
            cov_syms.insert(key.symbol.clone());
            if iv == &intervals[0] && uname == &units[0].0 {
                for b in &card {
                    *cov_days.entry(cal.local_day(b.t)).or_default() += 1;
                }
            }
            let ts: BTreeSet<i64> = card.iter().map(|b| b.t).collect();
            let trades: Vec<Bar> = tbars
                .iter()
                .filter(|b| ts.contains(&b.t))
                .cloned()
                .collect();
            let tmap: BTreeMap<i64, &Bar> = trades.iter().map(|b| (b.t, b)).collect();
            let mixed: Vec<Bar> = card
                .iter()
                .map(|c| tmap.get(&c.t).map_or_else(|| c.clone(), |t| (*t).clone()))
                .collect();
            let rest_on: Vec<Bar> = rest.iter().filter(|b| ts.contains(&b.t)).cloned().collect();
            for rp in rest_prices {
                let r = if rp == "mill" {
                    mill_bars(&rest_on)
                } else {
                    rest_on.clone()
                };
                for (src, bars) in [("card", &card), ("trades", &trades), ("mixed", &mixed)] {
                    let d = compare_bars_with(bars, &r, tol);
                    let k = (iv_rank(iv), iv.clone(), uname.clone(), rp.clone(), src);
                    merge_diff(rows.entry(k.clone()).or_default(), &d);
                    extras.entry(k).or_default().add(bars, &r);
                }
            }
        }
    }
    println!(
        "M27 sources on card-covered bars (with a REST bar that day): {} symbols; {} bars per day {}; card method {}",
        cov_syms.len(),
        intervals.first().map_or("", String::as_str),
        cov_days
            .iter()
            .map(|(d, n)| format!("{} {n}", day_name(*d)))
            .collect::<Vec<_>>()
            .join(","),
        a.card_method
    );
    print_header(&format!(
        "{:<3} {:<8} {:<4} {:<6}",
        "iv", "volume", "rest", "source"
    ));
    for (k, d) in &rows {
        let (_, iv, unit, rp, src) = k;
        print_row(
            &format!("{iv:<3} {unit:<8} {rp:<4} {src:<6}"),
            d,
            &extras.get(k).copied().unwrap_or_default(),
        );
    }
    Ok(())
}

/// RP4 harness series file (`rounds/RP4/experiments/harness/src/data.rs`): magic
/// `QPRP4BR1`, count u64 LE, then `t i64, o h l c v f64` little-endian records.
#[must_use]
pub fn encode_series(bars: &[Bar]) -> Vec<u8> {
    let mut out = Vec::with_capacity(16 + bars.len() * 48);
    out.extend_from_slice(b"QPRP4BR1");
    out.extend_from_slice(&(bars.len() as u64).to_le_bytes());
    for b in bars {
        out.extend_from_slice(&b.t.to_le_bytes());
        for x in [b.o, b.h, b.l, b.c, b.v] {
            out.extend_from_slice(&x.to_le_bytes());
        }
    }
    out
}

/// Decodes [`encode_series`].
///
/// # Errors
/// Bad magic or length.
pub fn decode_series(buf: &[u8]) -> Result<Vec<Bar>, String> {
    if buf.len() < 16 || &buf[..8] != b"QPRP4BR1" {
        return Err("not a QPRP4BR1 series".into());
    }
    let mut n8 = [0u8; 8];
    n8.copy_from_slice(&buf[8..16]);
    let n = usize::try_from(u64::from_le_bytes(n8)).map_err(|e| e.to_string())?;
    if buf.len() != 16 + n * 48 {
        return Err("truncated series".into());
    }
    let word = |i: usize| {
        let mut w = [0u8; 8];
        w.copy_from_slice(&buf[i..i + 8]);
        w
    };
    Ok((0..n)
        .map(|k| {
            let at = 16 + k * 48;
            Bar {
                t: i64::from_le_bytes(word(at)),
                o: f64::from_le_bytes(word(at + 8)),
                h: f64::from_le_bytes(word(at + 16)),
                l: f64::from_le_bytes(word(at + 24)),
                c: f64::from_le_bytes(word(at + 32)),
                v: f64::from_le_bytes(word(at + 40)),
            }
        })
        .collect())
}

/// Per-period counters of the export (no values).
#[derive(Debug, Default, Clone)]
struct ExportStats {
    symbols: u64,
    bars: u64,
    days: u64,
    first_day: String,
    last_day: String,
    resample_dropped: u64,
}

/// `qp live export-trade-bars`: bars from the trades through the live builder
/// (`BarMethod::Trades`, the M27 rules: `timestamp_ns`, opening auction in the first bar,
/// closing session in the last, special trades in `1d` only; volume TL) as RP4 harness
/// series: 5m, 15m (calendar resample of the 5m bars, like the harness), 1h, 1d.
#[allow(clippy::too_many_lines)]
pub fn export_trade_bars(a: &ExportTradeBarsArgs) -> Result<(), String> {
    let cal = xist()?;
    let syms = parse_symbols(&a.symbols)?;
    let days = (parse_day(&a.from)?, parse_day(&a.to)?);
    let check_from = cal
        .day_span(parse_day(&a.check_from)?)
        .map_or(i64::MIN, |s| s.open);
    let load_args = CompareTradesArgs {
        symbols: a.symbols.clone(),
        trades_dir: a.trades_dir.clone(),
        trades_ex_dir: a.trades_ex_dir.clone(),
        rest_dir: PathBuf::new(),
        from: a.from.clone(),
        to: a.to.clone(),
        intervals: String::new(),
        time: "ns".into(),
        edges: "session".into(),
        special: "day_only".into(),
        volume_units: "value".into(),
        rest_price: "raw".into(),
        rest_close: "last".into(),
        price_tol: 1e-6,
        volume_tol: 0.01,
        by_day: false,
        detail: false,
        raw: None,
        card_method: "mixed".into(),
    };
    let periods = ["5m", "15m", "1h", "1d"];
    let mut stats: BTreeMap<&str, ExportStats> = BTreeMap::new();
    let mut day_sets: BTreeMap<&str, BTreeSet<i64>> = BTreeMap::new();
    let mut checks: BTreeMap<(&str, &str), BarDiff> = BTreeMap::new();
    let mut missing_check: BTreeMap<&str, u64> = BTreeMap::new();
    let mut check_extra: BTreeMap<(&str, &str), VolExtra> = BTreeMap::new();
    let tol = DiffTolerance {
        price_abs: 1e-6,
        volume_rel: 0.01,
    };
    let mut load = LoadStats::default();
    for p in periods {
        let dir = a.out.join(p);
        if dir.exists() {
            std::fs::remove_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    for sym in &syms {
        let trades = load_trades(&load_args, &cal, sym, days, TimeField::Ns, &mut load)?;
        if trades.is_empty() {
            continue;
        }
        let cfg = || trade_config(VolumeUnit::Value, SpecialTrades::DayOnly, "session");
        let (five, _) = build_trade_bars(&trades, sym, "5m", &cal, cfg())?;
        let (fifteen, dropped) = cal
            .resample(Interval::Minutes(15), &five)
            .map_err(|e| format!("{sym}: {e}"))?;
        let (hour, _) = build_trade_bars(&trades, sym, "1h", &cal, cfg())?;
        let (day, _) = build_trade_bars(&trades, sym, "1d", &cal, cfg())?;
        for (p, bars) in [
            ("5m", &five),
            ("15m", &fifteen),
            ("1h", &hour),
            ("1d", &day),
        ] {
            if bars.is_empty() {
                continue;
            }
            std::fs::write(
                a.out.join(p).join(format!("{sym}.bars")),
                encode_series(bars),
            )
            .map_err(|e| format!("{sym}: {e}"))?;
            let st = stats.entry(p).or_default();
            st.symbols += 1;
            st.bars += bars.len() as u64;
            if p == "15m" {
                st.resample_dropped += dropped as u64;
            }
            day_sets
                .entry(p)
                .or_default()
                .extend(bars.iter().map(|b| cal.local_day(b.t)));
            if let Some(cdir) = &a.check {
                let path = cdir.join(p).join(format!("{sym}.bars"));
                let Ok(buf) = std::fs::read(&path) else {
                    *missing_check.entry(p).or_default() += 1;
                    continue;
                };
                let rest: Vec<Bar> = decode_series(&buf)
                    .map_err(|e| format!("{}: {e}", path.display()))?
                    .into_iter()
                    .filter(|b| {
                        let d = cal.local_day(b.t);
                        b.t >= check_from && d <= days.1
                    })
                    .collect();
                let mine: Vec<Bar> = bars.iter().filter(|b| b.t >= check_from).cloned().collect();
                for (form, r) in [("raw", rest.clone()), ("mill", mill_bars(&rest))] {
                    let d = compare_bars_with(&mine, &r, &tol);
                    merge_diff(checks.entry((p, form)).or_default(), &d);
                    check_extra.entry((p, form)).or_default().add(&mine, &r);
                }
            }
        }
    }
    for (p, st) in &mut stats {
        let ds = &day_sets[p];
        st.days = ds.len() as u64;
        st.first_day = ds.first().map(|d| day_name(*d)).unwrap_or_default();
        st.last_day = ds.last().map(|d| day_name(*d)).unwrap_or_default();
    }
    let manifest = serde_json::json!({
        "format": "rp4-harness series v1 (QPRP4BR1); bars from trades (qp live export-trade-bars)",
        "copy": "full",
        "source": "trades.gz level 1 + trade_ex; LiveBars BarMethod::Trades; time timestamp_ns; opening auction in the first bar, closing session in the last; special trades in 1d only; v = TL (sum price*quantity); 15m = calendar resample of 5m",
        "from": a.from,
        "to": a.to,
        "universe": syms.len(),
        "trades": {"files": load.files, "rows": load.rows, "unparsed": load.bad, "duplicates": load.duplicates, "special": load.special},
        "periods": stats
            .iter()
            .map(|(p, st)| {
                (
                    (*p).to_owned(),
                    serde_json::json!({
                        "symbols": st.symbols,
                        "bars": st.bars,
                        "days": st.days,
                        "first_day": st.first_day,
                        "last_day": st.last_day,
                        "resample_dropped": st.resample_dropped,
                    }),
                )
            })
            .collect::<serde_json::Map<String, serde_json::Value>>(),
    });
    std::fs::write(
        a.out.join("manifest.json"),
        serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())? + "\n",
    )
    .map_err(|e| e.to_string())?;
    println!(
        "export-trade-bars: {} symbols, {} trades -> {}",
        syms.len(),
        load.rows,
        a.out.display()
    );
    for (p, st) in &stats {
        println!(
            "  {p:<3}: {} symbols, {} bars, {} days ({}..{}), resample dropped {}",
            st.symbols, st.bars, st.days, st.first_day, st.last_day, st.resample_dropped
        );
    }
    if a.check.is_some() {
        println!(
            "check vs harness series from {} (missing there: {:?})",
            a.check_from, missing_check
        );
        print_header(&format!("{:<3} {:<4}", "iv", "rest"));
        for ((p, form), d) in &checks {
            print_row(
                &format!("{p:<3} {form:<4}"),
                d,
                &check_extra.get(&(*p, *form)).copied().unwrap_or_default(),
            );
        }
    }
    Ok(())
}

/// `qp live price-steps`.
pub fn price_steps(a: &PriceStepsArgs) -> Result<(), String> {
    let recs = read_recordings(&a.raw)?;
    let mut book = qp_matriks::SymbolBook::new();
    // Per message carrying priceStep: agree / card larger / card smaller / no reference.
    let (mut n, mut agree, mut larger, mut smaller, mut no_ref) = (0u64, 0u64, 0u64, 0u64, 0u64);
    let mut agree_base = 0u64;
    let mut sym_ok: BTreeMap<String, bool> = BTreeMap::new();
    let mut steps: BTreeSet<u64> = BTreeSet::new();
    for r in &recs {
        if r.kind != qp_matriks::RecordKind::Message {
            continue;
        }
        let topic = qp_matriks::parse_topic(&r.topic);
        let qp_matriks::Topic::Symbol { symbol, .. } = topic.unwrap_mirror() else {
            continue;
        };
        let Ok(up) = book.apply(symbol, &r.payload) else {
            continue;
        };
        // `priceStep` is field 41 of `SymbolMessage`.
        if !up.fields.contains(41) {
            continue;
        }
        let Some(step) = book.state(symbol).and_then(|s| s.price_step) else {
            continue;
        };
        n += 1;
        steps.insert(step.to_bits());
        let state = book.state(symbol);
        let last = state.and_then(|s| s.last).filter(|x| *x > 0.0);
        let base = state.and_then(|s| s.base_price).filter(|x| *x > 0.0);
        let Some(p) = last.or(base) else {
            no_ref += 1;
            continue;
        };
        let want = bist_price_step(p);
        let ok = (step - want).abs() < 1e-9;
        if ok {
            agree += 1;
        } else if step > want {
            larger += 1;
        } else {
            smaller += 1;
        }
        if base.is_some_and(|b| (step - bist_price_step(b)).abs() < 1e-9) {
            agree_base += 1;
        }
        let e = sym_ok.entry(symbol.to_string()).or_insert(true);
        *e &= ok;
    }
    println!(
        "price-steps: {} records; {n} card messages carry priceStep ({} distinct steps); vs table(last, else basePrice): agree {agree}, card larger {larger}, card smaller {smaller}, no reference {no_ref}; vs table(basePrice): agree {agree_base}",
        recs.len(),
        steps.len()
    );
    println!(
        "  symbols: {} with priceStep, {} agree in every message",
        sym_ok.len(),
        sym_ok.values().filter(|x| **x).count()
    );
    let syms = parse_symbols(&a.symbols)?;
    let (mut tn, mut on_table, mut on_cent) = (0u64, 0u64, 0u64);
    for sym in &syms {
        for p in csv_files(&a.trades_dir.join(sym)) {
            let (trades, _) = parse_rest_csv(&read_gz(&p)?);
            for t in trades {
                tn += 1;
                if on_grid(t.price, bist_price_step(t.price)) {
                    on_table += 1;
                }
                if on_grid(t.price, 0.01) {
                    on_cent += 1;
                }
            }
        }
    }
    let pc = |x: u64| {
        if tn == 0 {
            0.0
        } else {
            100.0 * x as f64 / tn as f64
        }
    };
    println!(
        "  trades ({} symbols): {tn}; on the table grid {on_table} ({:.4}%); on a cent {on_cent} ({:.4}%)",
        syms.len(),
        pc(on_table),
        pc(on_cent)
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const MS_PER_DAY: i64 = 86_400_000;

    #[test]
    fn step_table_and_grid() {
        assert_eq!(bist_price_step(19.99), 0.01);
        assert_eq!(bist_price_step(20.0), 0.02);
        assert_eq!(bist_price_step(99.95), 0.05);
        assert_eq!(bist_price_step(249.9), 0.10);
        assert_eq!(bist_price_step(499.75), 0.25);
        assert_eq!(bist_price_step(999.5), 0.50);
        assert_eq!(bist_price_step(2499.0), 1.00);
        assert_eq!(bist_price_step(2500.0), 2.50);
        assert!(on_grid(249.9, 0.10));
        assert!(on_grid(499.75, 0.25));
        assert!(!on_grid(499.70, 0.25));
        assert!(on_grid(20.02, 0.02));
        assert!(!on_grid(20.03, 0.02));
    }

    #[test]
    fn mill_rounding_equals_the_parsed_decimal() {
        assert_eq!(
            to_mill(249.899_99).to_bits(),
            "249.9".parse::<f64>().unwrap().to_bits()
        );
        assert_eq!(to_mill(45.180_000_02).to_bits(), 45.18_f64.to_bits());
        assert_eq!(to_mill(12.345).to_bits(), 12.345_f64.to_bits());
        assert_eq!(to_mill(12.0).to_bits(), 12.0_f64.to_bits());
    }

    #[test]
    fn date_time_clock_rewrites_and_measures() {
        let csv = b"symbol,date_time,timestamp_ns,bidask,mx_id,price,quantity\n\
            AAA,2026-11-16T09:55:20.000+0300,35719182000000,null,1,312.25,4\n\
            AAA,2026-11-16T10:00:00.000+0300,36000000000001,98,2,312.5,10\n";
        let (out, lag, in_day) = date_time_clock(csv);
        assert_eq!(in_day, 2);
        assert_eq!(lag.get(&1), Some(&1));
        assert_eq!(lag.get(&0), Some(&1));
        let (t, bad) = parse_rest_csv(&out);
        assert_eq!(bad, 0);
        assert_eq!(t[0].timestamp_ns % NS_PER_S, 0);
        assert_eq!(
            t[0].timestamp_ns.rem_euclid(MS_PER_DAY * 1_000_000) / NS_PER_S,
            (9 - 3) * 3600 + 55 * 60 + 20
        );
    }

    /// Trades of a hand-made day through the live builder equal REST bars built the
    /// REST way (opening auction in the first bar, closing session in the last).
    #[test]
    fn trade_bars_follow_the_session_edges() {
        let cal = xist().unwrap();
        // 2026-11-16 (Monday) local midnight in UTC ms.
        let mid = (qp_calendar::days_from_civil(2026, 11, 16) * 86_400 - 3 * 3600) * 1000;
        let at = |h: i64, m: i64, s: i64| (mid + ((h * 60 + m) * 60 + s) * 1000) * 1_000_000;
        let tr = |ns: i64, no: &str, p: f64, q: i64| Trade {
            symbol: "TEST".into(),
            price: p,
            quantity: Some(q),
            side: None,
            timestamp_ns: ns,
            order_no: Some(no.into()),
            is_trade_ex: false,
        };
        let trades = vec![
            tr(at(9, 55, 19), "1", 10.0, 5),
            tr(at(10, 0, 30), "2", 10.5, 1),
            tr(at(17, 59, 1), "3", 10.2, 2),
            tr(at(18, 5, 3), "4", 10.3, 4),
            tr(at(18, 8, 0), "5", 10.3, 1),
        ];
        let cfg = trade_config(VolumeUnit::Quantity, SpecialTrades::Exclude, "session");
        let (bars, st) = build_trade_bars(&trades, "TEST", "1m", &cal, cfg).unwrap();
        assert_eq!(bars.len(), 2);
        assert_eq!(
            (bars[0].o, bars[0].h, bars[0].c, bars[0].v),
            (10.0, 10.5, 10.5, 6.0)
        );
        assert_eq!((bars[1].o, bars[1].c, bars[1].v), (10.2, 10.3, 7.0));
        assert_eq!(st.off_session, 0);
        let cfg = trade_config(VolumeUnit::Quantity, SpecialTrades::Exclude, "none");
        let (bars, st) = build_trade_bars(&trades, "TEST", "1m", &cal, cfg).unwrap();
        assert_eq!(st.off_session, 3);
        assert_eq!((bars[0].v, bars[1].v), (1.0, 2.0));
        let (day, _) = build_trade_bars(
            &trades,
            "TEST",
            "1d",
            &cal,
            trade_config(VolumeUnit::Value, SpecialTrades::Exclude, "session"),
        )
        .unwrap();
        assert_eq!(day.len(), 1);
        assert_eq!(
            day[0].t,
            cal.day_span(cal.local_day(day[0].t)).unwrap().open
        );
        assert_eq!(day[0].v, 10.0 * 5.0 + 10.5 + 10.2 * 2.0 + 10.3 * 4.0 + 10.3);
    }

    /// End to end on a hand-built cache: trades of one day and REST bars made the REST
    /// way (opening auction in 10:00, closing session as an 18:04 bar merged by the
    /// loader, a REST price off the cent) compare all-exact after the cent rounding.
    #[test]
    fn compare_trades_runs_on_a_cache() {
        use std::io::Write as _;
        let dir = tempfile::tempdir().unwrap();
        let tdir = dir.path().join("trades/TEST");
        std::fs::create_dir_all(&tdir).unwrap();
        let csv = "symbol,date_time,timestamp_ns,bidask,mx_id,price,quantity\n\
            TEST,2026-11-16T09:55:20.000+0300,35719182000000,null,1,10.5,5\n\
            TEST,2026-11-16T10:00:30.000+0300,36030000000000,98,2,10.6,1\n\
            TEST,2026-11-16T17:59:01.000+0300,64741000000000,97,3,10.4,2\n\
            TEST,2026-11-16T18:05:03.000+0300,65103000000000,null,4,10.5,4\n";
        let f = std::fs::File::create(tdir.join("2026-11-16_2026-11-16.csv.gz")).unwrap();
        let mut gz = flate2::write::GzEncoder::new(f, flate2::Compression::default());
        gz.write_all(csv.as_bytes()).unwrap();
        gz.finish().unwrap();
        let rdir = dir.path().join("rest");
        std::fs::create_dir_all(&rdir).unwrap();
        let mid = (qp_calendar::days_from_civil(2026, 11, 16) * 86_400 - 3 * 3600) * 1000;
        let m = |h: i64, mm: i64| mid + (h * 60 + mm) * 60_000;
        let rest = serde_json::json!([
            {"time": m(10, 0), "open": 10.5, "high": 10.600_000_38, "low": 10.5, "close": 10.6,
             "volume": 10.5 * 5.0 + 10.6, "quantity": 6},
            {"time": m(17, 59), "open": 10.4, "high": 10.4, "low": 10.4, "close": 10.4,
             "volume": 20.8, "quantity": 2},
            {"time": m(18, 4), "open": 10.5, "high": 10.5, "low": 10.5, "close": 10.5,
             "volume": 42.0, "quantity": 4}
        ]);
        std::fs::write(
            rdir.join("TEST.1min.2026-11-16.2026-11-16.unadj.json"),
            rest.to_string(),
        )
        .unwrap();
        let args = CompareTradesArgs {
            symbols: "TEST".into(),
            trades_dir: dir.path().join("trades"),
            trades_ex_dir: dir.path().join("trades_ex"),
            rest_dir: rdir.clone(),
            from: "2026-11-16".into(),
            to: "2026-11-16".into(),
            intervals: "1m".into(),
            time: "ns,date_time".into(),
            edges: "session,none".into(),
            special: "day_only".into(),
            volume_units: "value,quantity".into(),
            rest_price: "raw,mill".into(),
            rest_close: "last".into(),
            price_tol: 1e-6,
            volume_tol: 0.01,
            by_day: true,
            detail: true,
            raw: None,
            card_method: "mixed".into(),
        };
        compare_trades(&args).unwrap();
        let cal = xist().unwrap();
        let mut st = LoadStats::default();
        let days = (
            parse_day("2026-11-16").unwrap(),
            parse_day("2026-11-16").unwrap(),
        );
        let trades = load_trades(&args, &cal, "TEST", days, TimeField::Ns, &mut st).unwrap();
        assert_eq!((trades.len(), st.ns_in_day), (4, 4));
        let cfg = trade_config(VolumeUnit::Quantity, SpecialTrades::DayOnly, "session");
        let (live, _) = build_trade_bars(&trades, "TEST", "1m", &cal, cfg).unwrap();
        let rest = rest_bars(&args, &cal, "TEST", "1m", true, days)
            .unwrap()
            .unwrap();
        let raw = compare_bars_with(&live, &rest, &DiffTolerance::default());
        let cent = compare_bars_with(&live, &mill_bars(&rest), &DiffTolerance::default());
        assert_eq!((raw.matched, raw.all_exact), (2, 1));
        assert_eq!((cent.matched, cent.all_exact), (2, 2));
        // Outside the window: nothing read.
        let mut st = LoadStats::default();
        let none = load_trades(
            &args,
            &cal,
            "TEST",
            (days.0 + 1, days.1 + 1),
            TimeField::Ns,
            &mut st,
        )
        .unwrap();
        assert!(none.is_empty() && st.files == 0);
    }

    /// REST's closing-session bar goes into the day's last session bar (live rule 1),
    /// created when REST has no bar there; `prev` keeps the R3 rule (previous REST bar).
    #[test]
    fn rest_closing_bar_into_the_last_span() {
        let dir = tempfile::tempdir().unwrap();
        let mid = (qp_calendar::days_from_civil(2026, 11, 16) * 86_400 - 3 * 3600) * 1000;
        let m = |h: i64, mm: i64| mid + (h * 60 + mm) * 60_000;
        let rest = serde_json::json!([
            {"time": m(17, 58), "open": 1.0, "high": 1.0, "low": 1.0, "close": 1.0, "volume": 1.0, "quantity": 1},
            {"time": m(18, 4), "open": 2.0, "high": 2.0, "low": 2.0, "close": 2.0, "volume": 4.0, "quantity": 2}
        ]);
        std::fs::write(
            dir.path()
                .join("TEST.1min.2026-11-16.2026-11-16.unadj.json"),
            rest.to_string(),
        )
        .unwrap();
        let cal = xist().unwrap();
        let last = load_rest_with(dir.path(), "TEST", "1m", false, true, &cal, true)
            .unwrap()
            .unwrap();
        assert_eq!(last.len(), 2);
        assert_eq!((last[1].t, last[1].c, last[1].v), (m(17, 59), 2.0, 2.0));
        let prev = load_rest_with(dir.path(), "TEST", "1m", false, true, &cal, false)
            .unwrap()
            .unwrap();
        assert_eq!(prev.len(), 1);
        assert_eq!((prev[0].t, prev[0].c, prev[0].v), (m(17, 58), 2.0, 3.0));
    }

    #[test]
    fn harness_series_round_trip() {
        let bars = vec![
            Bar {
                t: 1,
                o: 1.0,
                h: 2.0,
                l: 0.5,
                c: 1.5,
                v: 10.0,
            },
            Bar {
                t: 2,
                o: 1.5,
                h: 1.5,
                l: 1.5,
                c: 1.5,
                v: 0.0,
            },
        ];
        let buf = encode_series(&bars);
        assert_eq!(&buf[..8], b"QPRP4BR1");
        assert_eq!(buf.len(), 16 + 2 * 48);
        assert_eq!(decode_series(&buf).unwrap(), bars);
        assert!(decode_series(&buf[..20]).is_err());
    }
}
