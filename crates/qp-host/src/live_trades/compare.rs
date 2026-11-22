//! `qp live compare-trades`: trade-built bars against REST bars ([`compare_trades`]; RG1: split out of `live_trades.rs`).

use super::*;

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

pub(super) fn print_header(first: &str) {
    println!(
        "{first} | {:>7} {:>6} {:>6} | {:>6} {:>6} | exact% o/h/l/c/v               | v≤tol% | max rel price / v | v≤1e-6% all=p,v≤1e-6% v=f32%",
        "matched", "l-only", "r-only", "all=", "all~"
    );
}

/// Volume agreement beyond bit equality: REST's TL volume carries about seven
/// significant digits, so a Σ price·quantity in `f64` rarely equals it bit for bit.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct VolExtra {
    /// `|v − rest| ≤ 1e-6 · rest`.
    rel6: u64,
    /// All four prices bit-equal and `rel6`.
    all6: u64,
    /// `f32(v) == rest` (REST's number is the single-precision value).
    f32_eq: u64,
}

impl VolExtra {
    pub(super) fn add(&mut self, live: &[Bar], rest: &[Bar]) {
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

pub(super) fn print_row(label: &str, d: &BarDiff, x: &VolExtra) {
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

#[allow(clippy::cognitive_complexity)]
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

#[allow(clippy::too_many_lines)]
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
