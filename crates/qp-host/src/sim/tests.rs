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
    assert!(check_rule("r3").is_ok() && check_rule("rp4").is_ok() && check_rule("r4").is_err());
    // (RP4) The default writes nothing (byte-identical runs); another record is named.
    assert_eq!(engine_field("r3"), None);
    assert_eq!(engine_field("rp4").as_deref(), Some("rp4"));
}

/// VERIFY 8: the same trades through the loopback mock broker's trade endpoint (a real
/// `LiveConnection`, the live hub sink, the live runtime on the wall clock) and through
/// the simulation give byte-equal data events in the live host.
#[test]
fn simulation_path_is_the_live_path() {
    let _serial = crate::livehost::MOCK_ENDPOINTS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
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

// ---------------------------------------------------------------------------------------
// (RP4, L-prep) The live trade link: market and trade endpoints together.

/// Bars of every feed of `h` (the host's store), bit by bit.
fn store_bits(h: &ReplayHost) -> Vec<(String, Vec<[u64; 6]>)> {
    h.init()
        .data
        .iter()
        .flat_map(|d| d.feeds.iter().map(|f| f.feed.clone()))
        .map(|k| {
            let (bars, _) = h.live_feed(&k).expect("feed");
            let bits = bars
                .iter()
                .map(|b| {
                    [
                        u64::try_from(b.t).expect("t"),
                        b.o.to_bits(),
                        b.h.to_bits(),
                        b.l.to_bits(),
                        b.c.to_bits(),
                        b.v.to_bits(),
                    ]
                })
                .collect();
            (k.symbol.clone(), bits)
        })
        .collect()
}

/// Feeds the channel's inputs to the runtime until `until` holds (30 s at most).
fn pump_until(
    rx: &mpsc::Receiver<LiveIn>,
    rt: &mut LiveRuntime,
    h: &mut ReplayHost,
    until: &dyn Fn(&LiveRuntime) -> bool,
) -> bool {
    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline {
        if until(rt) {
            return true;
        }
        if let Ok(i) = rx.recv_timeout(Duration::from_millis(20)) {
            let _ = rt.on_in(h, i);
        }
    }
    until(rt)
}

/// The trade link drops and comes back while the market link drops and comes back: the
/// market's loss stales nothing (bars go on from trades), the trade's loss stales the
/// feeds, its reconnect fills the gap from the REST trade history (a fake REST: the cached
/// CSV through `RestTradeHistory`, off the hub thread) and the final bars are bit-equal
/// to those built from the REST trade history alone. No bar repair replaces the feed.
#[test]
fn trade_link_drops_repair_from_rest_bit_equal() {
    let tmp = tempfile::tempdir().expect("tmp");
    let root = tmp.path();
    write_cache(root, &["2026-11-17", "2026-11-18"]);
    let s = spec(root, "2026-11-18", "2026-11-18");
    let input = load(&s).expect("load");
    let decode = TradeDecodeConfig::default();
    let cal = input.cal.clone();
    let d = cal.local_day(input.from_ms + 12 * 3_600_000);
    let sess = cal.sessions_on(d);
    let (open, close) = (sess[0].open, sess[sess.len() - 1].close);
    let rest = || -> Arc<dyn TradeHistory + Send + Sync> {
        Arc::new(crate::livehost::RestTradeHistory::dir(
            root.join("fixtures/local/trades"),
            cal.clone(),
        ))
    };

    // Truth: every bar from the REST trade history alone (the trade link opens after the
    // close; the simulation runtime fills on the hub thread).
    let (mut th, mut trt) = runtime(&s, &input, Arc::new(|_| {}), decode).expect("truth");
    trt.set_trade_history(rest());
    let _ = trt.on_in(&mut th, LiveIn::Clock((close + 3_600_000) * 1000));
    let _ = trt.on_in(
        &mut th,
        LiveIn::Trade(conn_record(RecordKind::ConnectionOpened, close + 3_600_000)),
    );
    let _ = trt.on_in(&mut th, LiveIn::Clock(input.to_ms * 1000));
    let truth = store_bits(&th);
    assert!(
        truth.iter().all(|(_, b)| b.len() > 90),
        "a whole day of 5m bars"
    );

    // Live: the wall-clock path's asynchronous fill, on the simulated clock.
    let mut h = host(&s, &input).expect("host");
    let (tx, rx) = mpsc::channel::<LiveIn>();
    let tx = Mutex::new(tx);
    let to_hub: Arc<dyn Fn(LiveIn) + Send + Sync> = Arc::new(move |i| {
        let _ = tx
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .send(i);
    });
    let mut rt = LiveRuntime::new(
        &h,
        Box::new(MatriksFeed::with_trades(
            cal.clone(),
            "5m",
            qp_matriks::TradeFeedConfig {
                decode,
                ..qp_matriks::TradeFeedConfig::default()
            },
        )),
        input.source.clone(),
        to_hub,
        cal.clone(),
        "5m",
        0,
    )
    .expect("runtime");
    assert!(rt.trade_mode());
    rt.use_sim_clock((open - 3_600_000) * 1000);
    rt.set_trade_fill_sync(false);
    rt.set_trade_history(rest());
    rt.watch_trades_after_seams(input.from_ms);
    rt.baseline_capsules(&h);

    let (t1, t2, t3, t4, t5) = (
        open + 3_600_000,
        open + 7_200_000,
        open + 9_000_000,
        open + 10_800_000,
        open + 14_400_000,
    );
    let step = |rt: &mut LiveRuntime, h: &mut ReplayHost, i: LiveIn| {
        let _ = rt.on_in(h, i);
    };
    step(
        &mut rt,
        &mut h,
        LiveIn::Record(conn_record(RecordKind::ConnectionOpened, open - 3_600_000)),
    );
    assert_eq!(rt.link(), "connecting", "bars wait for the trade link");
    step(&mut rt, &mut h, LiveIn::Clock(t1 * 1000));
    step(
        &mut rt,
        &mut h,
        LiveIn::Trade(conn_record(RecordKind::ConnectionOpened, t1)),
    );
    assert_eq!(
        rt.link(),
        "repairing",
        "the opening gap is fetched off the hub"
    );
    assert!(pump_until(&rx, &mut rt, &mut h, &|r| r.link() == "live"));
    assert_eq!(rt.trade_fill_count(), 1);

    let (mut market_lost, mut market_back, mut trade_lost, mut trade_back) =
        (false, false, false, false);
    let mut events_at_market_loss = 0;
    for t in input
        .trades()
        .collect::<Vec<_>>()
        .iter()
        .filter(|t| t.t_ms() > t1)
    {
        let ms = t.t_ms();
        if !market_lost && ms > t2 {
            market_lost = true;
            step(
                &mut rt,
                &mut h,
                LiveIn::Record(conn_record(RecordKind::ConnectionLost, t2)),
            );
            assert_eq!(rt.link(), "live", "the market link's loss stales no bar");
            events_at_market_loss = rt.events;
        }
        if !market_back && ms > t3 {
            market_back = true;
            assert!(
                rt.events > events_at_market_loss,
                "bars go on while the market link is down"
            );
            step(
                &mut rt,
                &mut h,
                LiveIn::Record(conn_record(RecordKind::ConnectionOpened, t3)),
            );
            assert_eq!(rt.link(), "live");
        }
        if !trade_lost && ms > t4 {
            trade_lost = true;
            step(
                &mut rt,
                &mut h,
                LiveIn::Trade(conn_record(RecordKind::ConnectionLost, t4)),
            );
            assert_eq!(rt.link(), "stale", "the trade link's loss stales the feeds");
        }
        if trade_lost && !trade_back {
            if ms <= t5 {
                continue; // Sent while the trade link was down: only REST has them.
            }
            trade_back = true;
            step(&mut rt, &mut h, LiveIn::Clock(t5 * 1000));
            step(
                &mut rt,
                &mut h,
                LiveIn::Trade(conn_record(RecordKind::ConnectionOpened, t5)),
            );
            assert!(pump_until(&rx, &mut rt, &mut h, &|r| r.link() == "live"));
            assert_eq!(rt.trade_fill_count(), 2);
        }
        step(&mut rt, &mut h, LiveIn::Clock(ms * 1000));
        step(&mut rt, &mut h, LiveIn::Trade(record(t, &decode)));
    }
    assert!(market_lost && market_back && trade_lost && trade_back);
    step(&mut rt, &mut h, LiveIn::Clock((close + 3_600_000) * 1000));
    step(&mut rt, &mut h, LiveIn::Clock(input.to_ms * 1000));
    let st = rt.stats(&h);
    assert_eq!(
        st["repairs"].as_array().map(Vec::len),
        Some(0),
        "no bar repair"
    );
    assert_eq!(
        st["trade_fills"][1]["after_loss"], true,
        "{}",
        st["trade_fills"]
    );
    assert_eq!(st["trade_fills"][1]["failed"], 0);
    for i in 0..2 {
        assert!(
            st["trade_fills"][i]["trades"]
                .as_u64()
                .is_some_and(|n| n > 100),
            "the gaps were filled from REST: {}",
            st["trade_fills"]
        );
    }
    assert_eq!(st["feed"]["trades"]["connections_lost"], 1);
    assert_eq!(rt.capsule_counts().1, 0, "no capsule while stale");
    assert_eq!(h.live_not_ready(), 0);
    assert!(
        store_bits(&h) == truth,
        "bars after the repair are bit-equal to REST's"
    );
}

/// T4's known limit: after a REST bar repair the feed is replaced; with trades the new
/// one keeps the watched symbols (a gap from the new seam) and the trade connection's
/// state, and the trades after the seam come back from history: bit-equal bars.
#[test]
fn reseeded_trade_feed_keeps_watched_symbols_and_trade_link() {
    use crate::livehost::FeedEngine as _;
    let tmp = tempfile::tempdir().expect("tmp");
    let root = tmp.path();
    write_cache(root, &["2026-11-17", "2026-11-18"]);
    let s = spec(root, "2026-11-18", "2026-11-18");
    let input = load(&s).expect("load");
    let cal = input.cal.clone();
    let d = cal.local_day(input.from_ms + 12 * 3_600_000);
    let open = cal.sessions_on(d)[0].open;
    let h = host(&s, &input).expect("host");
    let mut feed =
        MatriksFeed::with_trades(cal.clone(), "5m", qp_matriks::TradeFeedConfig::default());
    let keys: Vec<FeedKey> = h
        .init()
        .data
        .iter()
        .flat_map(|d| d.feeds.iter().map(|f| f.feed.clone()))
        .collect();
    let seeds: Vec<crate::livehost::Seed> = keys
        .iter()
        .map(|k| {
            let (bars, seq) = h.live_feed(k).expect("feed");
            (k.symbol.clone(), bars.last().cloned(), seq + 1, None)
        })
        .collect();
    feed.seed_all(&seeds).expect("seed");
    let syms: Vec<String> = keys.iter().map(|k| k.symbol.clone()).collect();
    feed.watch_trades_after_seams(&syms, input.from_ms);
    let (t1, t2) = (open + 3_600_000, open + 7_200_000);
    let _ = feed.push_from(
        qp_matriks::FeedSource::Trade,
        &conn_record(RecordKind::ConnectionOpened, t1),
    );
    let (_, gaps) = feed.trade_gaps(t1);
    assert_eq!(gaps.len(), syms.len());
    for g in &gaps {
        let hist = input
            .history
            .trades(&g.symbol, g.from_ms, g.to_ms)
            .expect("h");
        let _ = feed.apply_trade_history(g, hist);
    }
    let decode = TradeDecodeConfig::default();
    for t in input
        .trades()
        .collect::<Vec<_>>()
        .iter()
        .filter(|t| t.t_ms() > t1 && t.t_ms() <= t2)
    {
        let _ = feed.push_from(qp_matriks::FeedSource::Trade, &record(t, &decode));
    }
    let _ = feed.advance_clock(t2);
    assert!(feed.inner().trade_stale_symbols().is_empty());

    // A REST bar repair reseeds at the last bar complete by `cut`.
    let cut = t2 - 30 * 60_000;
    let by_sym = |sym: &str, until: i64| -> Vec<Trade> {
        input
            .trades()
            .collect::<Vec<_>>()
            .iter()
            .filter(|t| t.symbol == sym && t.t_ms() < until)
            .cloned()
            .collect()
    };
    let mut seam: BTreeMap<String, Bar> = BTreeMap::new();
    let reseeds: Vec<crate::livehost::Seed> = syms
        .iter()
        .map(|sym| {
            let bars = trade_bars(sym, "5m", &by_sym(sym, cut), &cal, cut);
            let last = bars.last().cloned().expect("a bar");
            seam.insert(sym.clone(), last.clone());
            (sym.clone(), Some(last), 1_000, None)
        })
        .collect();
    feed.seed_all(&reseeds).expect("reseed a trade feed");
    let inner = feed.inner();
    assert!(
        inner.trades_enabled() && inner.trade_connected(),
        "trade link kept"
    );
    let mut stale = inner.trade_stale_symbols();
    stale.sort();
    let mut want = syms.clone();
    want.sort();
    assert_eq!(stale, want, "every watched symbol kept, with a gap");
    let (_, gaps) = feed.trade_gaps(t2);
    for g in &gaps {
        assert_eq!(
            Some(g.from_ms),
            feed.inner().seam_end(&g.symbol),
            "gap from the seam"
        );
        let hist = input
            .history
            .trades(&g.symbol, g.from_ms, g.to_ms)
            .expect("h");
        let o = feed.apply_trade_history(g, hist);
        // Rebuild the feed's bars after the seam from its events.
        let k = feed.inner().key(&g.symbol, "5m");
        let mut store =
            qp_source::FeedStore::with_bars(k, vec![seam[&g.symbol].clone()], 999).expect("st");
        for e in o.events.iter().filter(|e| e.feed.symbol == g.symbol) {
            store.apply(e).expect("gapless");
        }
        let truth = trade_bars(&g.symbol, "5m", &by_sym(&g.symbol, t2 + 1), &cal, t2);
        let first = truth
            .iter()
            .position(|b| b.t == seam[&g.symbol].t)
            .expect("seam in truth");
        let want: Vec<&Bar> = truth[first..].iter().collect();
        let got: Vec<&Bar> = store.bars().iter().take(want.len()).collect();
        assert!(got.len() > 3);
        assert!(
            got.iter().zip(&want).all(|(a, b)| a.t == b.t
                && a.o.to_bits() == b.o.to_bits()
                && a.h.to_bits() == b.h.to_bits()
                && a.l.to_bits() == b.l.to_bits()
                && a.c.to_bits() == b.c.to_bits()
                && a.v.to_bits() == b.v.to_bits())
                && got.len() == want.len(),
            "{} bars after the seam rebuilt from history",
            g.symbol
        );
    }
    assert!(feed.inner().trade_stale_symbols().is_empty());
}

/// Both endpoints at once on the loopback mock broker, through the demo's own wiring
/// (`spawn_connection`, `HubSink`, the live runtime): one connection per endpoint
/// (`max_concurrent == 1` on each path), no `@lvl2` trade topic ever reaches the broker
/// (one is asked for and refused locally), the trade link drops while the market link
/// stays and the other way round, and the trade raw log keeps no buyer / seller byte.
#[test]
fn market_and_trade_endpoints_together_on_the_mock_broker() {
    let _serial = crate::livehost::MOCK_ENDPOINTS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    use crate::livehost::{spawn_connection, ConnCtl, ConnSpec, HubSink};
    use qp_live::mock::{mock_config, BrokerScript, MockBroker, MockTokenService};
    use std::sync::atomic::{AtomicU64, Ordering};
    let tmp = tempfile::tempdir().expect("tmp");
    let root = tmp.path();
    write_cache(root, &["2026-11-17", "2026-11-18"]);
    let s = spec(root, "2026-11-18", "2026-11-18");
    let input = load(&s).expect("load");
    let cal = input.cal.clone();
    let decode = TradeDecodeConfig {
        unit: TradeTimeUnit::EpochMillis,
        ..TradeDecodeConfig::default()
    };
    let tokens = MockTokenService::start_default(3600);
    let broker = MockBroker::start(BrokerScript::default());
    let cfg = mock_config(&tokens, &broker.url("market")).expect("cfg");
    let auth = Arc::new(qp_live::AuthClient::new(
        &cfg,
        qp_live::AuthPolicy::default(),
    ));
    let mut h = host(&s, &input).expect("host");
    let (tx, rx) = mpsc::channel::<LiveIn>();
    let tx = Mutex::new(tx);
    let market_msgs = Arc::new(AtomicU64::new(0));
    let mm = market_msgs.clone();
    let to_hub: Arc<dyn Fn(LiveIn) + Send + Sync> = Arc::new(move |i| {
        if let LiveIn::Record(r) = &i {
            if r.kind == RecordKind::Message {
                mm.fetch_add(1, Ordering::SeqCst);
            }
        }
        let _ = tx
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .send(i);
    });
    let delay = crate::live::now_us() / 1000 - input.from_ms;
    let mut rt = LiveRuntime::new(
        &h,
        Box::new(MatriksFeed::with_trades(
            cal.clone(),
            "5m",
            qp_matriks::TradeFeedConfig {
                decode,
                ..qp_matriks::TradeFeedConfig::default()
            },
        )),
        input.source.clone(),
        to_hub.clone(),
        cal.clone(),
        "5m",
        delay,
    )
    .expect("runtime");
    rt.set_trade_history(Arc::new(crate::livehost::RestTradeHistory::dir(
        root.join("fixtures/local/trades"),
        cal.clone(),
    )));
    rt.watch_trades_after_seams(input.from_ms);
    let dir = tempfile::tempdir().expect("lock dir");
    let mut trade_topics = vec!["mx/trade/TEST1@lvl2".to_owned()];
    for sym in SYMS {
        trade_topics.extend(qp_matriks::trade_topics(sym, true).expect("topics"));
    }
    let market = spawn_connection(
        ConnSpec {
            cfg: cfg.clone(),
            auth: auth.clone(),
            topics: SYMS.iter().map(|s| format!("mx/symbol/{s}@lvl2")).collect(),
            lock_dir: dir.path().to_owned(),
            trade: false,
        },
        Arc::new(HubSink::new(
            crate::live::RawSink::create(&dir.path().join("market.qpraw")).expect("raw"),
            to_hub.clone(),
        )),
        to_hub.clone(),
    )
    .expect("market endpoint");
    let trade_raw = dir.path().join("trade.qpraw");
    let trade = spawn_connection(
        ConnSpec {
            cfg,
            auth,
            topics: trade_topics,
            lock_dir: dir.path().to_owned(),
            trade: true,
        },
        Arc::new(HubSink::for_trades(
            crate::live::RawSink::create(&trade_raw).expect("raw"),
            to_hub.clone(),
        )),
        to_hub,
    )
    .expect("trade endpoint");
    assert!(pump_until(&rx, &mut rt, &mut h, &|r| r.link() == "live"));
    assert!(broker.wait_until(Duration::from_secs(5), |st| {
        st.path("market").current == 1
            && st.path("trade").current == 1
            && st.path("trade").subscribed_topics.len() >= 2 * SYMS.len()
    }));

    // The trade link drops (server side); the market link stays and delivers.
    let decode_wire = |t: &Trade| {
        let mut m =
            <TradeMessage as prost::Message>::decode(wire(t, &decode).as_slice()).expect("wire");
        m.buyer = Some("BUYERX".to_owned());
        m.seller = Some("SELLERY".to_owned());
        m.encode_to_vec()
    };
    let all_trades: Vec<Trade> = input.trades().collect();
    let morning: Vec<&Trade> = all_trades.iter().take(40).collect();
    for t in &morning[..20] {
        broker.publish(&format!("mx/trade/{}", t.symbol), &decode_wire(t));
    }
    assert!(pump_until(&rx, &mut rt, &mut h, &|r| r.messages >= 20));
    broker.drop_path("trade");
    assert!(pump_until(&rx, &mut rt, &mut h, &|r| r.link() == "stale"));
    let before = market_msgs.load(Ordering::SeqCst);
    broker.publish("mx/symbol/TEST1@lvl2", b"\x08\x01");
    assert!(pump_until(&rx, &mut rt, &mut h, &|_| {
        market_msgs.load(Ordering::SeqCst) > before
    }));
    assert!(
        pump_until(&rx, &mut rt, &mut h, &|r| r.link() == "live"),
        "trade reconnect, then the REST trade fill"
    );
    let st = broker.stats();
    assert_eq!(
        st.path("market").connections_total,
        1,
        "market link never dropped"
    );
    assert_eq!(st.path("trade").connections_total, 2);

    // The market link drops; the trade link stays, the bars stay live.
    broker.drop_path("market");
    assert!(pump_until(&rx, &mut rt, &mut h, &|r| {
        r.stats_market_connected() == Some(false)
    }));
    assert_eq!(rt.link(), "live", "the market link's loss stales no bar");
    for t in &morning[20..] {
        broker.publish(&format!("mx/trade/{}", t.symbol), &decode_wire(t));
    }
    assert!(pump_until(&rx, &mut rt, &mut h, &|r| r.messages >= 41));
    assert!(broker.wait_until(Duration::from_secs(10), |st| st.path("market").current == 1));
    let st = broker.stats();
    assert_eq!(
        st.path("trade").connections_total,
        2,
        "trade link not dropped"
    );
    assert_eq!(st.path("market").max_concurrent, 1, "one market connection");
    assert_eq!(st.path("trade").max_concurrent, 1, "one trade connection");
    for p in ["market", "trade"] {
        assert!(
            st.path(p)
                .subscribed_topics
                .iter()
                .all(|t| !t.starts_with("mx/trade/") || !t.to_ascii_lowercase().ends_with("@lvl2")),
            "no level-2 trade topic on {p}"
        );
    }
    assert!(st
        .path("trade")
        .subscribed_topics
        .iter()
        .any(|t| t == "mx/tradeex/TEST1"));
    assert!(st
        .path("market")
        .subscribed_topics
        .iter()
        .all(|t| t.starts_with("mx/symbol/")));
    assert_eq!(rt.stats(&h)["repairs"].as_array().map(Vec::len), Some(0));

    for ctl in [market, trade] {
        let (done, wait) = mpsc::channel();
        let _ = ctl.send(ConnCtl::Stop(done));
        let _ = wait.recv_timeout(Duration::from_secs(10));
    }
    // The trade raw log: every trade, none with a buyer / seller byte, and the
    // sanitizing note counting what the wire carried.
    let bytes = std::fs::read(&trade_raw).expect("raw");
    assert!(!bytes.windows(6).any(|w| w == b"BUYERX"));
    assert!(!bytes.windows(7).any(|w| w == b"SELLERY"));
    let (recs, _) = crate::live::read_all(&trade_raw).expect("read");
    let ti = crate::live::inspect_trades(&recs);
    assert_eq!(ti.messages(), 40);
    assert_eq!(ti.fields.get("buyer"), None);
    assert_eq!(ti.fields.get("seller"), None);
    assert_eq!(ti.fields.get("price"), Some(&40));
    // (Arrival is today, the trades are of the cached day: `Auto` picks by arrival.)
    assert_eq!(ti.auto_unit.values().sum::<u64>(), 40);
    assert_eq!(ti.lag_auto_ms.len(), 40);
    assert_eq!(ti.notes.len(), 1);
    assert!(ti.notes[0].contains("buyer_present=40 seller_present=40 unreadable=0"));
}
