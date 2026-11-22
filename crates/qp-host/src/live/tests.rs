use super::*;

#[test]
fn iso_times_parse_with_and_without_offset() {
    // 2026-11-19 14:02:03.123 Istanbul = 11:02:03.123 UTC.
    let utc =
        (qp_calendar::days_from_civil(2026, 11, 19) * 86_400 + 11 * 3600 + 2 * 60 + 3) * 1000 + 123;
    assert_eq!(
        parse_iso_ms("2026-11-19T14:02:03.123+03"),
        Some((utc, true))
    );
    assert_eq!(
        parse_iso_ms("2026-11-19T14:02:03.123+03:00"),
        Some((utc, true))
    );
    assert_eq!(parse_iso_ms("2026-11-19T11:02:03.123Z"), Some((utc, true)));
    assert_eq!(parse_iso_ms("2026-11-19T14:02:03.123"), Some((utc, false)));
    assert_eq!(parse_iso_ms("19/11/26"), None);
    assert_eq!(
        shape("2026-11-19T14:02:03.123+03"),
        "dddd-dd-ddTdd:dd:dd.ddd+dd"
    );
}

#[test]
fn identity_lock_is_exclusive() {
    let dir = tempfile::tempdir().unwrap();
    std::env::set_var("QP_LIVE_LOCK_DIR", dir.path());
    let first = identity_lock().expect("first holder");
    let second = identity_lock();
    assert!(second.is_err(), "a second token holder must be refused");
    assert!(second.unwrap_err().contains("identity"));
    drop(first);
    assert!(identity_lock().is_ok(), "released with its holder");
    std::env::remove_var("QP_LIVE_LOCK_DIR");
}

#[test]
fn reset_needs_a_note_and_a_person() {
    let dir = tempfile::tempdir().unwrap();
    let key = qp_live::EndpointKey::new("delayed-market");
    let marker = key.halt_marker(dir.path());
    std::fs::write(
        &marker,
        "halted=unexplained-server-close\ncloses=2\nwindow_secs=600\n",
    )
    .unwrap();
    let args = |note: &str, human: bool| ResetArgs {
        endpoint: "delayed-market".into(),
        note: note.into(),
        i_am_human: human,
    };
    let e = reset(&args("  ", true), dir.path()).unwrap_err();
    assert!(e.contains("--note is required"), "{e}");
    assert!(marker.exists());
    // Tests run without a terminal on stdin: the flag is required.
    if !std::io::IsTerminal::is_terminal(&std::io::stdin()) {
        let e = reset(&args("checked", false), dir.path()).unwrap_err();
        assert!(e.contains("--i-am-human"), "{e}");
        assert!(marker.exists());
    }
    reset(&args("Arkin: no other session", true), dir.path()).unwrap();
    assert!(!marker.exists());
    let log = std::fs::read_to_string(qp_live::halt::halt_log(dir.path(), &key)).unwrap();
    assert!(
        log.contains("RESET previous=unexplained-server-close"),
        "{log}"
    );
    assert!(log.contains("Arkin: no other session"), "{log}");
    // Not halted: nothing to do.
    reset(&args("again", true), dir.path()).unwrap();
}

#[test]
fn symbols_are_checked() {
    assert_eq!(
        parse_symbols("thyao, garan").unwrap(),
        vec!["THYAO", "GARAN"]
    );
    assert!(parse_symbols("a/b").is_err());
    assert!(parse_symbols(" , ").is_err());
}

/// Hand-built records (made-up symbol and prices): a connection, a snapshot, trades
/// with ask-only updates in between, a lost connection and a reconnect.
pub(crate) fn fake_records() -> Vec<RawRecord> {
    use prost::Message as _;
    use qp_matriks::messages::SymbolMessage;
    let base = qp_calendar::days_from_civil(2026, 11, 18) * 86_400_000 - 3 * 3_600_000;
    let at = |h: i64, m: i64, s: i64| base + ((h * 60 + m) * 60 + s) * 1000;
    let ud = |t: i64| {
        let l = t + 3 * 3_600_000 - qp_calendar::days_from_civil(2026, 11, 18) * 86_400_000;
        format!(
            "2026-11-18T{:02}:{:02}:{:02}.{:03}+03",
            l / 3_600_000,
            l / 60_000 % 60,
            l / 1000 % 60,
            l % 1000
        )
    };
    let topic = "mx/symbol/TEST1@lvl2";
    let delay = 900_000;
    let r = |kind, t: i64| RawRecord {
        t_us: t * 1000,
        kind,
        topic: String::new(),
        payload: Vec::new(),
    };
    let mut out = vec![r(RecordKind::ConnectionOpened, at(10, 0, 0))];
    let snap = SymbolMessage {
        symbol_id: 7,
        update_date: Some(ud(at(9, 58, 0))),
        last: Some(10.0),
        open: Some(10.0),
        daily_volume: Some(0.0),
        daily_quantity: Some(0.0),
        stock_status: Some(2),
        ..SymbolMessage::default()
    };
    out.push(RawRecord::message(
        at(10, 0, 1) * 1000,
        topic,
        snap.encode_to_vec(),
    ));
    let mut cum = 0.0;
    for i in 0..40i32 {
        let t = at(10, 0, 5) + i64::from(i) * 17_000;
        cum += 100.0 + f64::from(i);
        let m = SymbolMessage {
            symbol_id: 7,
            update_date: Some(ud(t)),
            last: Some(10.0 + f64::from(i % 7) * 0.05),
            daily_volume: Some(cum),
            volume: Some(cum * 2.0),
            daily_quantity: Some(cum / 10.0),
            ..SymbolMessage::default()
        };
        out.push(RawRecord::message(
            (t + delay) * 1000,
            topic,
            m.encode_to_vec(),
        ));
        let ask = SymbolMessage {
            symbol_id: 7,
            ask_size: Some(i64::from(i)),
            ..SymbolMessage::default()
        };
        out.push(RawRecord::message(
            (t + delay + 200) * 1000,
            topic,
            ask.encode_to_vec(),
        ));
        if i == 20 {
            let lost = (t + delay + 500) * 1000;
            out.push(RawRecord {
                t_us: lost,
                kind: RecordKind::ConnectionLost,
                topic: String::new(),
                payload: b"test".to_vec(),
            });
            out.push(RawRecord {
                t_us: lost + 1000,
                kind: RecordKind::ConnectionOpened,
                topic: "market".into(),
                payload: Vec::new(),
            });
            out.push(RawRecord {
                t_us: lost + 2000,
                kind: RecordKind::Subscribed,
                topic: topic.into(),
                payload: Vec::new(),
            });
        }
    }
    out
}

pub(crate) fn write_raw(path: &Path, recs: &[RawRecord]) {
    let mut w = RawWriter::new(File::create(path).unwrap()).unwrap();
    for r in recs {
        w.write(r).unwrap();
    }
    w.flush().unwrap();
}

fn feed_args() -> FeedArgs {
    FeedArgs {
        intervals: "1m,5m,1d".into(),
        method: "ticks".into(),
        volume_field: "daily".into(),
        volume_unit: "value".into(),
        time: "update_date".into(),
        finalize_grace_ms: qp_matriks::feed::DEFAULT_FINALIZE_GRACE_MS,
    }
}

/// Replay output = the live path (`LiveFeed::push` record by record), byte for byte,
/// and two replays are byte-equal.
#[test]
fn replay_raw_equals_the_live_path() {
    let dir = tempfile::tempdir().unwrap();
    let raw = dir.path().join("fake.qpraw");
    let recs = fake_records();
    write_raw(&raw, &recs);
    let out1 = dir.path().join("a.jsonl");
    let out2 = dir.path().join("b.jsonl");
    replay_raw(&raw, &feed_args(), Some(&out1), false).unwrap();
    replay_raw(&raw, &feed_args(), Some(&out2), false).unwrap();
    let a = std::fs::read(&out1).unwrap();
    assert_eq!(a, std::fs::read(&out2).unwrap());

    let mut feed = feed_args().build().unwrap();
    let mut live = String::new();
    for r in &recs {
        for l in feed_out_lines(&feed.push(r)) {
            live.push_str(&l);
            live.push('\n');
        }
    }
    assert_eq!(String::from_utf8(a).unwrap(), live);
    assert!(live.contains("\"conn\":\"lost\""));
    assert!(live.contains("\"final\""));
    assert!(live.contains("\"clock\""));
    assert!(live.contains("\"phase\""));
    assert_eq!(feed.stats().resubscribes, 1);
}

/// REST mapping (R3 rule): daily local midnight → session open; a closing-session
/// bar merges into the previous bar; adjusted files are used only without unadjusted.
#[test]
fn rest_bars_map_to_k14() {
    let dir = tempfile::tempdir().unwrap();
    let cal = xist().unwrap();
    let day = qp_calendar::days_from_civil(2026, 11, 18);
    let local = |h: i64, m: i64| day * 86_400_000 + (h * 60 + m) * 60_000 - 3 * 3_600_000;
    let row = |t: i64, o: f64, h: f64, l: f64, c: f64, v: f64| serde_json::json!({"time": t, "open": o, "high": h, "low": l, "close": c, "volume": v, "quantity": v / 10.0});
    std::fs::write(
        dir.path()
            .join("TEST1.1day.2026-11-18.2026-11-18.unadj.json"),
        serde_json::json!([row(local(0, 0), 1.0, 2.0, 0.5, 1.5, 100.0)]).to_string(),
    )
    .unwrap();
    std::fs::write(
        dir.path().join("TEST1.1day.2026-11-18.2026-11-18.adj.json"),
        serde_json::json!([row(local(0, 0), 9.0, 9.0, 9.0, 9.0, 9.0)]).to_string(),
    )
    .unwrap();
    std::fs::write(
        dir.path()
            .join("TEST1.1min.2026-11-18.2026-11-18.unadj.json"),
        serde_json::json!([
            row(local(17, 58), 1.0, 1.2, 0.9, 1.1, 10.0),
            row(local(17, 59), 1.1, 1.3, 1.0, 1.2, 20.0),
            row(local(18, 4), 1.4, 1.4, 1.4, 1.4, 5.0)
        ])
        .to_string(),
    )
    .unwrap();
    let d = load_rest(dir.path(), "TEST1", "1d", false, false, &cal)
        .unwrap()
        .unwrap();
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].t, local(10, 0));
    assert_eq!(d[0].o.to_bits(), 1.0f64.to_bits());
    let adj = load_rest(dir.path(), "TEST1", "1d", true, false, &cal)
        .unwrap()
        .unwrap();
    assert_eq!(adj[0].o.to_bits(), 9.0f64.to_bits());
    let m = load_rest(dir.path(), "TEST1", "1m", false, false, &cal)
        .unwrap()
        .unwrap();
    assert_eq!(m.len(), 2);
    assert_eq!(m[1].t, local(17, 59));
    assert_eq!((m[1].h, m[1].c, m[1].v), (1.4, 1.4, 25.0));
    let q = load_rest(dir.path(), "TEST1", "1m", false, true, &cal)
        .unwrap()
        .unwrap();
    assert_eq!(q[0].v, 1.0);
    assert!(load_rest(dir.path(), "OTHER", "1m", false, false, &cal)
        .unwrap()
        .is_none());
}

/// The compare command runs on a hand-built log and REST files made from its own live
/// bars, and fails clearly without REST files.
#[test]
fn compare_runs_and_requires_rest_files() {
    let dir = tempfile::tempdir().unwrap();
    let raw = dir.path().join("fake.qpraw");
    let recs = fake_records();
    write_raw(&raw, &recs);
    let rest = dir.path().join("rest");
    std::fs::create_dir_all(&rest).unwrap();
    let args = |rest_dir: PathBuf| CompareArgs {
        raw: raw.display().to_string(),
        rest_dir,
        intervals: "1m".into(),
        methods: "ticks,mixed".into(),
        volume_fields: "daily,volume".into(),
        volume_unit: "value".into(),
        time: "update_date".into(),
        finalize_grace_ms: 2000,
        price_tol: 1e-6,
        volume_tol: 0.01,
        adjusted: false,
    };
    let err = compare(&args(rest.clone())).unwrap_err();
    assert!(err.contains("no REST bars for 1m"), "{err}");
    let mut fa = feed_args();
    fa.intervals = "1m".into();
    let built = build_live(&recs, &fa).unwrap();
    assert_eq!(built.gaps.len(), 1);
    let bars = built.stores.values().next().unwrap().bars().to_vec();
    let rows: Vec<serde_json::Value> = bars
        .iter()
        .map(|b| serde_json::json!({"time": b.t, "open": b.o, "high": b.h, "low": b.l, "close": b.c, "volume": b.v, "quantity": 0.0}))
        .collect();
    std::fs::write(
        rest.join("TEST1.1min.2026-11-18.2026-11-18.unadj.json"),
        serde_json::Value::Array(rows).to_string(),
    )
    .unwrap();
    compare(&args(rest)).unwrap();
}
