//! The REST side of `qp live`: symbols, settled bars and trades ([`bars`], [`trades`]; RG1: split out of `live.rs`).

use super::*;

fn rest_ctx() -> Result<(LiveConfig, AuthClient, qp_live::lock::MachineLock), String> {
    let lock = identity_lock()?;
    let cfg = LiveConfig::from_env().map_err(|e| e.to_string())?;
    let auth = AuthClient::new(&cfg, AuthPolicy::default());
    Ok((cfg, auth, lock))
}

pub(super) fn symbols() -> Result<(), String> {
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

pub(super) fn bars(
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

#[allow(clippy::too_many_lines)]
pub(super) fn trades(
    symbols: &str,
    start: &str,
    end: &str,
    window_days: u32,
    ex: bool,
) -> Result<(), String> {
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
