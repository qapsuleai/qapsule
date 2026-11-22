//! The trade cache and the first resolution's history ([`load_trades`], [`CacheBars`]; RG1: split out of `sim.rs`).

use super::*;

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
pub(super) fn load_many<T: Send>(
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
pub(super) struct Row {
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

    pub(super) fn len(&self) -> usize {
        self.rows.len()
    }

    pub(super) fn ts_ns(&self, i: usize) -> i64 {
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
    pub(super) syms: Vec<&'a Packed>,
    pub(super) heap: std::collections::BinaryHeap<std::cmp::Reverse<(i64, usize, usize)>>,
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
    pub(super) by_symbol: BTreeMap<String, Packed>,
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
