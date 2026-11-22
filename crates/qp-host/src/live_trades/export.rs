//! `qp live export-trade-bars` and `qp live price-steps` ([`export_trade_bars`], [`price_steps`]; RG1: split out of `live_trades.rs`).

use super::*;

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

#[allow(clippy::too_many_lines)]
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
