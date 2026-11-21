//! Synthetic REST files (tests and smoke runs without real data): XIST-calendar 5-minute
//! and 1-hour bars with a separate closing-auction bar, daily bars at local midnight,
//! in the REST file layout `<SYM>.<period>.<start>.<end>.adj.json`.

use std::path::Path;

use qp_schema::Bar;

use crate::common::{day_str, xist, Lcg};

fn row(b: &Bar) -> serde_json::Value {
    serde_json::json!({
        "time": b.t, "open": b.o, "high": b.h, "low": b.l, "close": b.c,
        "volume": b.v, "quantity": (b.v / b.c).round()
    })
}

fn step(rng: &mut Lcg, p: f64, vol: f64) -> f64 {
    let mut z = 0.0;
    for _ in 0..6 {
        z += rng.u01() * 2.0 - 1.0;
    }
    (p * (1.0 + vol * z / std::f64::consts::SQRT_2)).max(0.5)
}

/// Raw (REST-shaped) 5-minute bars of one symbol from `first` to `last` (local dates),
/// including a closing-auction bar at each session close.
#[must_use]
pub fn five_minute(first: &str, last: &str, seed: u64, p0: f64) -> Vec<Bar> {
    let cal = xist();
    let (d0, d1) = (
        qp_calendar::parse_date(first).expect("date"),
        qp_calendar::parse_date(last).expect("date"),
    );
    let mut rng = Lcg::new(seed);
    let mut p = p0;
    let mut out = Vec::new();
    for day in d0..=d1 {
        let Some(span) = cal.day_span(day) else {
            continue;
        };
        let mut t = span.open;
        while t < span.close {
            let o = p;
            let mut hi = o;
            let mut lo = o;
            for _ in 0..5 {
                p = step(&mut rng, p, 0.0012);
                hi = hi.max(p);
                lo = lo.min(p);
            }
            let c = (p * 100.0).round() / 100.0;
            p = c;
            out.push(Bar {
                t,
                o,
                h: hi.max(c),
                l: lo.min(c),
                c,
                v: (1.0 + rng.u01() * 4.0) * 1.0e5,
            });
            t += 300_000;
        }
        // Closing auction: one price.
        let c = (step(&mut rng, p, 0.0008) * 100.0).round() / 100.0;
        p = c;
        out.push(Bar {
            t: span.close,
            o: c,
            h: c,
            l: c,
            c,
            v: 5.0e5,
        });
    }
    out
}

/// REST-shaped 1-hour bars from raw 5-minute bars (hour starts; the closing bar stays a
/// separate bar at the session close, as REST sends it).
#[must_use]
pub fn hourly(five: &[Bar]) -> Vec<Bar> {
    let cal = xist();
    let mut out: Vec<Bar> = Vec::new();
    for b in five {
        let day = cal.local_day(b.t);
        let span = cal.day_span(day).expect("trading day");
        let key = if b.t >= span.close {
            b.t
        } else {
            span.open + (b.t - span.open) / 3_600_000 * 3_600_000
        };
        match out.last_mut() {
            Some(l) if l.t == key => {
                l.h = l.h.max(b.h);
                l.l = l.l.min(b.l);
                l.c = b.c;
                l.v += b.v;
            }
            _ => out.push(Bar {
                t: key,
                ..b.clone()
            }),
        }
    }
    out
}

/// REST-shaped daily bars (time = local midnight) of a random walk.
#[must_use]
pub fn daily(first: &str, last: &str, seed: u64, p0: f64) -> Vec<Bar> {
    let cal = xist();
    let (d0, d1) = (
        qp_calendar::parse_date(first).expect("date"),
        qp_calendar::parse_date(last).expect("date"),
    );
    let mut rng = Lcg::new(seed);
    let mut p = p0;
    let mut out = Vec::new();
    for day in d0..=d1 {
        let Some(span) = cal.day_span(day) else {
            continue;
        };
        let o = p;
        let mut hi = o;
        let mut lo = o;
        for _ in 0..8 {
            p = step(&mut rng, p, 0.007);
            hi = hi.max(p);
            lo = lo.min(p);
        }
        let c = (p * 100.0).round() / 100.0;
        p = c;
        // Local midnight = session open − 10 h (Istanbul, no DST since 2016).
        out.push(Bar {
            t: span.open - 10 * 3_600_000,
            o,
            h: hi.max(c),
            l: lo.min(c),
            c,
            v: 1.0e8,
        });
    }
    out
}

fn write_rest(
    dir: &Path,
    sym: &str,
    token: &str,
    bars: &[Bar],
    windows: usize,
) -> std::io::Result<()> {
    let cal = xist();
    let n = bars.len();
    let w = windows.max(1);
    for k in 0..w {
        // Windows overlap by a few rows (the merger must drop duplicates).
        let a = (k * n / w).saturating_sub(if k > 0 { 3 } else { 0 });
        let b = ((k + 1) * n / w).min(n);
        if a >= b {
            continue;
        }
        let part = &bars[a..b];
        let name = format!(
            "{sym}.{token}.{}.{}.adj.json",
            day_str(cal.local_day(part[0].t)),
            day_str(cal.local_day(part[part.len() - 1].t))
        );
        let rows: Vec<serde_json::Value> = part.iter().map(row).collect();
        std::fs::write(dir.join(name), serde_json::to_string(&rows)?)?;
    }
    Ok(())
}

/// Write a synthetic REST directory: `symbols` symbols, 5-minute / 1-hour bars from
/// `intraday_first` to `last`, daily bars from `daily_first` to `last`.
///
/// # Errors
/// I/O.
pub fn write_rest_dir(
    dir: &Path,
    symbols: usize,
    intraday_first: &str,
    daily_first: &str,
    last: &str,
    seed: u64,
) -> std::io::Result<Vec<String>> {
    std::fs::create_dir_all(dir)?;
    let mut names = Vec::new();
    for i in 0..symbols {
        let sym = format!("SYN{:02}", i + 1);
        #[allow(clippy::cast_precision_loss)]
        let p0 = 15.0 + 60.0 * i as f64;
        let five = five_minute(intraday_first, last, seed * 1000 + i as u64, p0);
        write_rest(dir, &sym, "5min", &five, 3)?;
        write_rest(dir, &sym, "1hour", &hourly(&five), 1)?;
        let d = daily(daily_first, last, seed * 7000 + i as u64, p0);
        write_rest(dir, &sym, "1day", &d, 1)?;
        names.push(sym);
    }
    Ok(names)
}
