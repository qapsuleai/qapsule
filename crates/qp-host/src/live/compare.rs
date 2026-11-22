//! `qp live compare`: REST bars against bars built from raw recordings ([`compare`]; RG1: split out of `live.rs`).

use super::*;

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
pub(super) fn compare(a: &CompareArgs) -> Result<(), String> {
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
