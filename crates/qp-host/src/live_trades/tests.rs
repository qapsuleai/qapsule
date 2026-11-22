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
