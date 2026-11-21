//! (RP4/T4) The accelerated simulation: byte-equal stream on the same input (VERIFY 7),
//! the simulation path is the live path (VERIFY 8: the same trades through the loopback
//! mock broker's trade endpoint and the live runtime give the same data events, byte for
//! byte), several days (close → closed → next open), the pace on a fake clock (no real
//! waiting). Made-up symbols and prices in a temporary cache; no network beyond
//! 127.0.0.1.

use std::cell::RefCell;
use std::io::Write as _;
use std::sync::mpsc;

use super::*;

const SYMS: [&str; 2] = ["TEST1", "TEST2"];

/// A fake real-time clock: `sleep` only moves it (and is recorded).
#[derive(Default)]
struct FakePace {
    now: RefCell<Duration>,
    sleeps: RefCell<Vec<Duration>>,
}

impl PaceClock for FakePace {
    fn now(&self) -> Duration {
        *self.now.borrow()
    }
    fn sleep(&self, d: Duration) {
        *self.now.borrow_mut() += d;
        self.sleeps.borrow_mut().push(d);
    }
}

fn day(s: &str) -> i64 {
    qp_calendar::parse_date(s).expect("date")
}

/// The made-up trades of `sym` on local day `d`: an opening-auction match at 09:55, then
/// one trade every 47 s (plus a symbol offset) from 10:00 to 17:59, price walking.
fn day_rows(sym: &str, d: i64, sym_i: i64) -> Vec<String> {
    let date = date_text(d);
    let mut rows = Vec::new();
    let mut k = 0i64;
    let mut push = |sec: i64, ms: i64, side: &str, k: &mut i64| {
        let ns = (sec * 1000 + ms) * 1_000_000 + 417;
        #[allow(clippy::cast_precision_loss)]
        let price = 20.0 + sym_i as f64 * 5.0 + ((*k * 37 + d * 11) % 41) as f64 * 0.02;
        let (h, m, s) = (sec / 3600, sec / 60 % 60, sec % 60);
        rows.push(format!(
            "{sym},{date}T{h:02}:{m:02}:{s:02}.000+0300,{ns},{side},{},{price:.2},{}",
            1_000_000 + d * 10_000 + *k,
            1 + (*k * 13 + sym_i) % 90
        ));
        *k += 1;
    };
    push(9 * 3600 + 55 * 60, 0, "null", &mut k);
    let mut sec = 10 * 3600 + 3 + sym_i * 5;
    while sec < 18 * 3600 {
        push(sec, 123, if k % 3 == 0 { "97" } else { "98" }, &mut k);
        sec += 47;
    }
    rows
}

/// A trade cache under `root/fixtures/local/trades` for `days` (one file per symbol).
fn write_cache(root: &Path, days: &[&str]) {
    for (i, sym) in SYMS.iter().enumerate() {
        let dir = root.join("fixtures/local/trades").join(sym);
        std::fs::create_dir_all(&dir).expect("dir");
        let ds: Vec<i64> = days.iter().map(|d| day(d)).collect();
        let name = format!(
            "{}_{}.csv.gz",
            date_text(*ds.iter().min().expect("days")),
            date_text(*ds.iter().max().expect("days"))
        );
        let f = std::fs::File::create(dir.join(name)).expect("file");
        let mut gz = flate2::write::GzEncoder::new(f, flate2::Compression::fast());
        writeln!(
            gz,
            "symbol,date_time,timestamp_ns,bidask,mx_id,price,quantity"
        )
        .expect("w");
        for d in &ds {
            #[allow(clippy::cast_possible_wrap)]
            for r in day_rows(sym, *d, i as i64) {
                writeln!(gz, "{r}").expect("w");
            }
        }
        gz.finish().expect("gz");
    }
    std::fs::create_dir_all(root.join("fixtures/local/live/rest")).expect("rest dir");
}

fn spec(root: &Path, start: &str, end: &str) -> SimSpec {
    SimSpec::new(
        &SYMS.join(","),
        start,
        Some(end),
        "5m",
        40,
        "rate55",
        "r3",
        (
            Path::new("fixtures/local/trades"),
            Path::new("fixtures/local/live/rest"),
            root,
        ),
    )
    .expect("spec")
}

#[test]
fn same_input_twice_gives_a_byte_equal_stream_at_any_speed() {
    let tmp = tempfile::tempdir().expect("tmp");
    let root = tmp.path();
    write_cache(root, &["2026-11-16", "2026-11-17", "2026-11-18"]);
    let s = spec(root, "2026-11-17", "2026-11-18");
    let out = |name: &str| root.join("fixtures/local").join(name);
    let fake = FakePace::default();
    let a = run(&s, Speed::Max, Some(&out("a.jsonl")), &fake).expect("run a");
    let b = run(&s, Speed::Max, Some(&out("b.jsonl")), &fake).expect("run b");
    assert!(fake.sleeps.borrow().is_empty(), "max never waits");
    let (fa, fb) = (
        std::fs::read(out("a.jsonl")).expect("a"),
        std::fs::read(out("b.jsonl")).expect("b"),
    );
    assert!(!fa.is_empty());
    assert_eq!(fa, fb, "byte-equal stream");
    assert_eq!(a.digest, b.digest);
    // History from the day before (built from the cached trades: no REST bars here).
    assert_eq!(a.json["history_from_trades"], 2);
    assert_eq!(a.json["symbols_with_history"], 2);
    assert_eq!(a.json["events_rejected"], 0);
    assert_eq!(a.json["notices"], 0);
    assert!(a.json["events"].as_u64().expect("events") > 1000);
    // The pace changes when inputs are sent, never what they are.
    let paced = FakePace::default();
    let c = run(&s, Speed::Factor(600.0), None, &paced).expect("run c");
    assert_eq!(c.digest, a.digest, "same stream at 600x");
    assert!(!paced.sleeps.borrow().is_empty(), "600x waits (fake clock)");
    // `--out` only under fixtures/local.
    assert!(check_out(&root.join("x.jsonl"), root).is_err());
    assert!(check_out(&out("y.jsonl"), root).is_ok());
}

#[test]
fn several_days_close_then_closed_then_next_open() {
    let tmp = tempfile::tempdir().expect("tmp");
    let root = tmp.path();
    // Friday → Monday (weekend in between).
    write_cache(root, &["2026-11-19", "2026-11-20", "2026-11-23"]);
    let s = spec(root, "2026-11-20", "2026-11-23");
    let p = root.join("fixtures/local/days.jsonl");
    let r = run(&s, Speed::Max, Some(&p), &FakePace::default()).expect("run");
    assert_eq!(r.json["days"], 4);
    assert_eq!(r.json["trading_days"], 2);
    let text = std::fs::read_to_string(&p).expect("stream");
    let lines: Vec<Value> = text
        .lines()
        .map(|l| serde_json::from_str(l).expect("json"))
        .collect();
    let opens: Vec<bool> = lines
        .iter()
        .filter_map(|v| v["market"]["open"].as_bool())
        .collect();
    let mut flips = opens.clone();
    flips.dedup();
    assert_eq!(
        flips,
        vec![false, true, false, true, false],
        "closed → Friday open → closed (weekend) → Monday open → closed"
    );
    // Bars of both days, no event on the weekend, in order per feed.
    let cal = xist().expect("cal");
    let mut by_day: BTreeMap<i64, usize> = BTreeMap::new();
    let mut last: BTreeMap<(String, String), u64> = BTreeMap::new();
    for v in &lines {
        let Some(ev) = v.get("ev") else { continue };
        let ev: qp_schema::data::DataEvent = serde_json::from_value(ev.clone()).expect("event");
        let t = match &ev.change {
            DataChange::Bar { bar } => bar.t,
            DataChange::Patch { t, .. } => *t,
        };
        *by_day.entry(cal.local_day(t)).or_default() += 1;
        let k = (ev.feed.symbol.clone(), ev.feed.interval.clone());
        let prev = last.insert(k, ev.seq);
        assert!(prev.is_none_or(|p| p + 1 == ev.seq), "gapless seq");
    }
    assert_eq!(
        by_day.keys().copied().collect::<Vec<_>>(),
        vec![day("2026-11-20"), day("2026-11-23")]
    );
    // The schedule has no session on the weekend.
    let iv = Interval::parse("5m").expect("iv");
    let sched = schedule(
        &cal,
        iv,
        local_midnight(&cal, day("2026-11-21")),
        local_midnight(&cal, day("2026-11-23")),
    );
    assert!(sched.iter().all(|&t| !cal.in_session(t - 1)), "weekend");
}

#[test]
fn pace_follows_the_speed_factor_on_a_fake_clock() {
    let cal = xist().expect("cal");
    let d = day("2026-11-18");
    let at = |h: i64, m: i64| local_midnight(&cal, d) + (h * 60 + m) * 60_000;
    let fake = FakePace::default();
    let mut p = Pacer::new(Speed::Factor(60.0), &fake, cal.clone());
    p.wait_until(at(10, 0));
    p.wait_until(at(10, 5)); // 5 min in session at 60x: 5 s
    p.wait_until(at(10, 5)); // same time: no wait
    assert_eq!(*fake.now.borrow(), Duration::from_secs(5));
    p.wait_until(at(18, 30)); // the session ended 18:00: 8 h 25 min… mostly in session
    let after_close = *fake.now.borrow();
    p.wait_until(local_midnight(&cal, d + 1) + 10 * 3_600_000); // night: capped
    assert_eq!(*fake.now.borrow() - after_close, CLOSED_PAUSE);
    // Real time already spent counts: no sleep when behind.
    *fake.now.borrow_mut() += Duration::from_secs(3600);
    let n = fake.sleeps.borrow().len();
    p.wait_until(local_midnight(&cal, d + 1) + 10 * 3_600_000 + 60_000);
    assert_eq!(fake.sleeps.borrow().len(), n, "behind schedule: no wait");

    let fake = FakePace::default();
    let mut p = Pacer::new(Speed::Max, &fake, cal.clone());
    p.wait_until(at(10, 0));
    p.wait_until(at(17, 0));
    assert!(fake.sleeps.borrow().is_empty());
    assert_eq!(Speed::parse("max"), Ok(Speed::Max));
    assert_eq!(Speed::parse("60"), Ok(Speed::Factor(60.0)));
    assert_eq!(Speed::parse("8x"), Ok(Speed::Factor(8.0)));
    assert!(Speed::parse("0").is_err() && Speed::parse("fast").is_err());
    assert!(check_rule("r3").is_ok() && check_rule("r4").is_err());
}

/// VERIFY 8: the same trades through the loopback mock broker's trade endpoint (a real
/// `LiveConnection`, the live hub sink, the live runtime on the wall clock) and through
/// the simulation give byte-equal data events in the live host.
#[test]
fn simulation_path_is_the_live_path() {
    use qp_live::mock::{mock_config, BrokerScript, MockBroker, MockTokenService};
    let tmp = tempfile::tempdir().expect("tmp");
    let root = tmp.path();
    write_cache(root, &["2026-11-17", "2026-11-18"]);
    let s = spec(root, "2026-11-18", "2026-11-18");
    let input = load(&s).expect("load");
    // Epoch milliseconds on the wire: a since-midnight value would take today's date from
    // the wall-clock arrival (the unit is a feed setting; both sides use it).
    let decode = TradeDecodeConfig {
        unit: TradeTimeUnit::EpochMillis,
        ..TradeDecodeConfig::default()
    };

    // The simulation.
    let (mut sh, mut srt) = runtime(&s, &input, Arc::new(|_| {}), decode).expect("sim");
    let fake = FakePace::default();
    let mut pacer = Pacer::new(Speed::Max, &fake, input.cal.clone());
    let iv = Interval::parse("5m").expect("iv");
    drive(&input, iv, &decode, &mut pacer, &mut |i| {
        let _ = srt.on_in(&mut sh, i);
        true
    });
    let sim_events = serde_json::to_string(&sh.recorded_events()).expect("json");
    assert!(sh.recorded_len() > 500);

    // Live: the mock broker's trade endpoint, a real connection, the hub sink.
    let tokens = MockTokenService::start_default(3600);
    let broker = MockBroker::start(BrokerScript::default());
    let cfg = mock_config(&tokens, &broker.url("market")).expect("cfg");
    let mut lh = host(&s, &input).expect("host");
    let (tx, rx) = mpsc::channel::<LiveIn>();
    let tx = Mutex::new(tx);
    let to_hub: Arc<dyn Fn(LiveIn) + Send + Sync> = Arc::new(move |i| {
        let _ = tx
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .send(i);
    });
    // Delayed by the distance to the simulated day: the exchange clock is the trades'.
    let delay = crate::live::now_us() / 1000 - input.from_ms;
    let mut lrt = LiveRuntime::new(
        &lh,
        Box::new(MatriksFeed::with_trades(
            input.cal.clone(),
            "5m",
            qp_matriks::TradeFeedConfig {
                decode,
                ..qp_matriks::TradeFeedConfig::default()
            },
        )),
        input.source.clone(),
        to_hub.clone(),
        input.cal.clone(),
        "5m",
        delay,
    )
    .expect("runtime");
    lrt.set_trade_history(input.history.clone());
    lrt.watch_trades(input.from_ms);
    let dir = tempfile::tempdir().expect("lock dir");
    let sink = Arc::new(crate::livehost::HubSink::for_trades(
        crate::live::RawSink::create(&dir.path().join("trade.qpraw")).expect("raw"),
        to_hub,
    ));
    let mut topics = Vec::new();
    for sym in SYMS {
        topics.extend(qp_matriks::trade_topics(sym, false).expect("topics"));
    }
    let mut opts = qp_live::ConnectOptions::new(topics);
    opts.lock_dir = dir.path().to_owned();
    let auth = Arc::new(qp_live::AuthClient::new(
        &cfg,
        qp_live::AuthPolicy::default(),
    ));
    let conn = qp_live::LiveConnection::open(
        &cfg,
        cfg.trade_endpoint(),
        auth,
        opts,
        sink as Arc<dyn qp_live::LiveSink>,
    )
    .expect("trade connection");
    let pump = |rt: &mut LiveRuntime, h: &mut ReplayHost, until: &dyn Fn(&LiveRuntime) -> bool| {
        let deadline = Instant::now() + Duration::from_secs(30);
        while Instant::now() < deadline {
            if let Ok(i) = rx.recv_timeout(Duration::from_millis(20)) {
                let _ = rt.on_in(h, i);
            }
            if until(rt) {
                return true;
            }
        }
        false
    };
    assert!(
        pump(&mut lrt, &mut lh, &|r| r.link() == "live"),
        "trade endpoint open"
    );
    assert!(broker.wait_until(Duration::from_secs(5), |st| {
        st.subscribe_packets.iter().map(Vec::len).sum::<usize>() >= SYMS.len()
    }));
    for t in input.trades() {
        let topic = format!("mx/trade/{}", t.symbol);
        broker.publish(&topic, &wire(&t, &decode));
    }
    let want = sh.recorded_len();
    assert!(
        pump(&mut lrt, &mut lh, &|r| r.messages
            >= input.trade_count() as u64),
        "every trade arrived"
    );
    let _ = conn.close();
    let live_events = serde_json::to_string(&lh.recorded_events()).expect("json");
    assert_eq!(lh.recorded_len(), want);
    assert!(
        live_events == sim_events,
        "simulation and live give byte-equal data events"
    );
}

/// (RP4/T6) A packed symbol gives back the very trades packed: plain and odd trade
/// numbers (leading zero, letters, absent), every side, quantity absent, special trades.
#[test]
fn packed_trades_are_the_trades_packed() {
    let t =
        |ns: i64, no: Option<&str>, side: Option<qp_matriks::Aggressor>, q: Option<i64>| Trade {
            symbol: "TEST1".to_owned(),
            price: 12.34 + f64::from(u32::try_from(ns % 7).expect("small")) * 0.01,
            quantity: q,
            side,
            timestamp_ns: ns,
            order_no: no.map(str::to_owned),
            is_trade_ex: ns % 3 == 0,
        };
    use qp_matriks::Aggressor::{Ask, Bid, Other};
    let trades = vec![
        t(1, Some("0"), None, Some(5)),
        t(2, Some("1606"), Some(Bid), Some(0)),
        t(3, Some("0012"), Some(Ask), None),
        t(4, Some("A-7"), Some(Other("97".to_owned())), Some(-3)),
        t(5, None, Some(Other(String::new())), Some(i64::MAX)),
        t(6, Some("18446744073709551615"), None, Some(1)),
        t(7, Some(" 42"), Some(Bid), Some(2)),
        t(8, Some("9999999999999999999"), Some(Ask), Some(3)),
        t(
            9,
            Some("72057594037927935"),
            Some(Other("x".to_owned())),
            None,
        ),
        t(10, Some("72057594037927936"), None, Some(4)),
    ];
    let p = Packed::new("TEST1", &trades);
    let back: Vec<Trade> = (0..trades.len()).map(|i| p.get(i)).collect();
    assert_eq!(back, trades);
    assert_eq!(std::mem::size_of::<Row>(), 32, "a packed trade is 32 bytes");
}

/// (RP4/T6) `--at HH:MM`: the day's earlier trades are history; the run starts then. At
/// midnight it is the run without `--at`, byte for byte.
#[test]
fn start_time_takes_the_days_earlier_trades_as_history() {
    let tmp = tempfile::tempdir().expect("tmp");
    let root = tmp.path();
    write_cache(root, &["2026-11-16", "2026-11-17"]);
    let fake = FakePace::default();
    let base = spec(root, "2026-11-17", "2026-11-17");
    let a = run(&base, Speed::Max, None, &fake).expect("run");
    let midnight = spec(root, "2026-11-17", "2026-11-17")
        .with_at(Some("00:00"))
        .expect("at");
    let b = run(&midnight, Speed::Max, None, &fake).expect("run 00:00");
    assert_eq!(a.digest, b.digest, "--at 00:00 is the default");
    let noon = spec(root, "2026-11-17", "2026-11-17")
        .with_at(Some("12:00"))
        .expect("at");
    let c = run(&noon, Speed::Max, None, &fake).expect("run 12:00");
    let n = |r: &RunSummary, k: &str| r.json[k].as_u64().expect(k);
    assert!(n(&c, "trade_records") < n(&a, "trade_records"));
    assert_eq!(n(&c, "symbols_with_history"), 2);
    assert_eq!(c.json["events_rejected"], 0);
    for bad in ["12", "24:00", "11:60", "a:b"] {
        assert!(spec(root, "2026-11-17", "2026-11-17")
            .with_at(Some(bad))
            .is_err());
    }
}
