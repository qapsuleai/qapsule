//! VERIFY 10 (M23, no network): a simulated drop on a recorded-like stream — stale
//! marking, reconnect, REST repair (a fake source), no capsule while stale, repaired
//! bars equal to the settled ones.
use super::*;
use qp_schema::protocol::OutboundLine;

mod replica;

fn cal() -> ExchangeCalendar {
    ExchangeCalendar::new(&qp_calendar::builtin("XIST").expect("XIST")).expect("cal")
}

/// `n` daily bars on XIST trading days from 2025-01-02 (a gentle walk).
fn truth(n: usize, seed: f64) -> Vec<Bar> {
    let c = cal();
    let mut out = Vec::new();
    let mut day = qp_calendar::days_from_civil(2025, 1, 2);
    let mut p = 100.0 + seed;
    while out.len() < n {
        if let Some(s) = c.sessions_on(day).first() {
            #[allow(clippy::cast_precision_loss)]
            let k = out.len() as f64;
            let o = p;
            p += ((k * 0.7 + seed).sin()) * 2.0;
            out.push(Bar {
                t: s.open,
                o,
                h: o.max(p) + 1.0,
                l: o.min(p) - 1.0,
                c: p,
                v: 1000.0 + k,
            });
        }
        day += 1;
    }
    out
}

/// A fake feed: a message's payload is a JSON bar of its topic's symbol; a later `t`
/// opens a bar, the same `t` patches it.
#[derive(Default)]
struct FakeFeed {
    last: BTreeMap<String, (Option<Bar>, u64)>,
}

impl FeedEngine for FakeFeed {
    fn seed_all(&mut self, seeds: &[Seed]) -> Result<(), String> {
        for (symbol, last, next_seq, _) in seeds {
            self.last.insert(symbol.clone(), (last.clone(), *next_seq));
        }
        Ok(())
    }
    fn push(&mut self, rec: &RawRecord) -> FeedOut {
        let mut out = FeedOut::default();
        match rec.kind {
            RecordKind::ConnectionOpened => {
                out.connection = Some(ConnChange::Opened { session: 2 });
            }
            RecordKind::ConnectionLost => out.connection = Some(ConnChange::Lost),
            RecordKind::Message => {
                let b: Bar = serde_json::from_slice(&rec.payload).expect("bar");
                let Some((last, seq)) = self.last.get_mut(&rec.topic) else {
                    return out;
                };
                let feed = FeedKey {
                    symbol: rec.topic.clone(),
                    interval: "1d".to_owned(),
                    feed: None,
                };
                let change = match last {
                    Some(l) if l.t == b.t => {
                        let f = qp_source::bar_patch_diff(l, &b);
                        if f.is_empty() {
                            return out;
                        }
                        DataChange::Patch { t: b.t, fields: f }
                    }
                    Some(l) if l.t > b.t => return out,
                    _ => DataChange::Bar { bar: b.clone() },
                };
                out.clock_ms = Some(b.t);
                out.events.push(DataEvent {
                    feed,
                    seq: *seq,
                    change,
                });
                *seq += 1;
                *last = Some(b);
            }
            _ => {}
        }
        out
    }
    fn advance_clock(&mut self, _now_ms: i64) -> FeedOut {
        FeedOut::default()
    }
}

struct FakeSource(BTreeMap<String, Vec<Bar>>);

impl BarSource for FakeSource {
    fn fetch(
        &self,
        symbols: &[String],
        _interval: &str,
        from_ms: i64,
        to_ms: i64,
    ) -> Vec<(String, Result<Vec<Bar>, String>)> {
        symbols
            .iter()
            .map(|s| {
                let b = self.0.get(s).map(|b| {
                    b.iter()
                        .filter(|x| x.t >= from_ms && x.t <= to_ms)
                        .cloned()
                        .collect()
                });
                (s.clone(), b.ok_or_else(|| "no such symbol".to_owned()))
            })
            .collect()
    }
}

/// A message arriving a day after its bar opened (the exchange clock lags the wall
/// clock by about a year; bars are closed by that clock).
fn msg(sym: &str, b: &Bar) -> RawRecord {
    RawRecord::message(
        (b.t + 86_400_000) * 1000,
        sym,
        serde_json::to_vec(b).expect("json"),
    )
}

fn conn(kind: RecordKind) -> RawRecord {
    RawRecord {
        t_us: now_us(),
        kind,
        topic: String::new(),
        payload: b"session 1".to_vec(),
    }
}

fn live_host(hist: &BTreeMap<String, Vec<Bar>>) -> ReplayHost {
    let feeds = hist
        .iter()
        .map(|(s, b)| qp_source::ReplayFeed {
            key: FeedKey {
                symbol: s.clone(),
                interval: "1d".to_owned(),
                feed: None,
            },
            bars: b.clone(),
            fine: Vec::new(),
        })
        .collect();
    let init = qp_session::demo_init(qp_schema::protocol::ReplayInit {
        interval: Some("1d".to_owned()),
        ..Default::default()
    });
    ReplayHost::live(init, std::path::Path::new("."), feeds).expect("live host")
}

#[test]
#[allow(clippy::needless_range_loop)]
fn drop_marks_stale_and_repair_matches_settled_bars() {
    let syms = ["AAA", "BBB"];
    let full: BTreeMap<String, Vec<Bar>> = syms
        .iter()
        .enumerate()
        .map(|(i, s)| {
            #[allow(clippy::cast_precision_loss)]
            let seed = i as f64 * 3.0;
            ((*s).to_owned(), truth(80, seed))
        })
        .collect();
    let hist: BTreeMap<String, Vec<Bar>> = full
        .iter()
        .map(|(s, b)| (s.clone(), b[..40].to_vec()))
        .collect();
    let mut host = live_host(&hist);
    let (tx, rx) = mpsc::channel::<LiveIn>();
    let tx = std::sync::Mutex::new(tx);
    let to_hub: Arc<dyn Fn(LiveIn) + Send + Sync> = Arc::new(move |i| {
        let _ = tx
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .send(i);
    });
    // The settled source differs from what live delivered on bar 44 (a correction).
    let mut settled = full.clone();
    settled.get_mut("AAA").expect("AAA")[44].c += 0.5;
    settled.get_mut("AAA").expect("AAA")[44].h += 0.5;
    let mut rt = LiveRuntime::new(
        &host,
        Box::new(FakeFeed::default()),
        Arc::new(FakeSource(settled.clone())),
        to_hub,
        cal(),
        "1d",
        0,
    )
    .expect("runtime");
    rt.baseline_capsules(&host);
    let mut lines: Vec<OutboundLine> = Vec::new();
    lines.extend(rt.on_in(
        &mut host,
        LiveIn::Record(conn(RecordKind::ConnectionOpened)),
    ));
    assert_eq!(rt.link(), "live");
    // Live: bars 40..46 arrive.
    for i in 40..46 {
        for s in syms {
            lines.extend(rt.on_in(&mut host, LiveIn::Record(msg(s, &full[s][i]))));
        }
    }
    lines.extend(host.live_flush());
    rt.watch_capsules(&host);
    assert_eq!(host.live_not_ready(), 0);
    assert_eq!(rt.events, 12);
    // Drop.
    lines.extend(rt.on_in(&mut host, LiveIn::Record(conn(RecordKind::ConnectionLost))));
    assert_eq!(rt.link(), "stale");
    assert_eq!(host.live_not_ready(), 2, "every live feed is stale");
    lines.extend(rt.tick(&mut host));
    let m = host.session().market_for("arkin").expect("market status");
    assert!(m.stale, "the market status says stale");
    // A late message while stale is not applied.
    lines.extend(rt.on_in(&mut host, LiveIn::Record(msg("AAA", &full["AAA"][46]))));
    assert_eq!(rt.dropped_while_stale, 1);
    rt.watch_capsules(&host);
    // Reconnect: repair starts; messages wait.
    lines.extend(rt.on_in(
        &mut host,
        LiveIn::Record(conn(RecordKind::ConnectionOpened)),
    ));
    assert_eq!(rt.link(), "repairing");
    for s in syms {
        lines.extend(rt.on_in(&mut host, LiveIn::Record(msg(s, &full[s][70]))));
    }
    let repaired = rx.recv_timeout(Duration::from_secs(10)).expect("repair");
    assert!(matches!(repaired, LiveIn::Repaired { .. }));
    lines.extend(rt.on_in(&mut host, repaired));
    lines.extend(host.live_flush());
    rt.watch_capsules(&host);
    assert_eq!(rt.link(), "live");
    assert_eq!(host.live_not_ready(), 0, "resolved again");
    lines.extend(rt.tick(&mut host));
    assert!(!host.session().market_for("arkin").expect("market").stale);
    // The repaired series: the settled bars (all complete by the exchange clock); the
    // buffered message for an already settled bar changes nothing.
    for s in syms {
        let k = FeedKey {
            symbol: s.to_owned(),
            interval: "1d".to_owned(),
            feed: None,
        };
        let (bars, _) = host.live_feed(&k).expect("feed");
        let want: Vec<Bar> = settled[s].clone();
        assert_eq!(bars, want, "{s}: repaired bars equal the settled bars");
    }
    // No capsule was born or changed while stale.
    assert_eq!(rt.capsule_counts().1, 0);
    let st = rt.stats(&host);
    assert_eq!(st["repairs"].as_array().map(Vec::len), Some(1));
    assert_eq!(st["repairs"][0]["patched"], 1);
    // Devices: the recorded events replay to the same bars (no gap: the stale mark
    // was never recorded).
    let (f0, evs) = host
        .feed_data(&FeedKey {
            symbol: "AAA".to_owned(),
            interval: "1d".to_owned(),
            feed: None,
        })
        .expect("feed data");
    let mut store =
        qp_source::FeedStore::with_bars(f0.feed.clone(), f0.bars.clone().expect("bars"), 0)
            .expect("store");
    for e in &evs {
        let a = store.apply(e).expect("apply");
        assert!(
            !matches!(
                a,
                qp_source::Applied::Gap { .. } | qp_source::Applied::Stale(_)
            ),
            "device sees no gap"
        );
    }
    assert_eq!(store.bars(), &settled["AAA"][..]);
    let _ = lines;
}

/// The connection path end to end on the loopback mock broker and token service: the
/// sink feeds the runtime, a local drop closes the one connection (DISCONNECT) before
/// opening again (never two at once), the feeds go stale and are repaired.
#[test]
fn local_drop_on_mock_broker_keeps_one_connection_and_repairs() {
    let _serial = MOCK_ENDPOINTS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    use qp_live::mock::{mock_config, BrokerScript, MockBroker, MockTokenService};
    let dir = tempfile::tempdir().expect("tmp");
    let tokens = MockTokenService::start_default(3600);
    let broker = MockBroker::start(BrokerScript::default());
    let cfg = mock_config(&tokens, &broker.url("market")).expect("cfg");
    let full: BTreeMap<String, Vec<Bar>> = [
        ("AAA".to_owned(), truth(60, 0.0)),
        ("BBB".to_owned(), truth(60, 2.0)),
    ]
    .into_iter()
    .collect();
    let hist: BTreeMap<String, Vec<Bar>> = full
        .iter()
        .map(|(s, b)| (s.clone(), b[..30].to_vec()))
        .collect();
    let mut host = live_host(&hist);
    let (tx, rx) = mpsc::channel::<LiveIn>();
    let tx = std::sync::Mutex::new(tx);
    let to_hub: Arc<dyn Fn(LiveIn) + Send + Sync> = Arc::new(move |i| {
        let _ = tx
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .send(i);
    });
    let mut rt = LiveRuntime::new(
        &host,
        Box::new(FakeFeed::default()),
        Arc::new(FakeSource(full.clone())),
        to_hub.clone(),
        cal(),
        "1d",
        0,
    )
    .expect("runtime");
    let sink = Arc::new(HubSink::new(
        RawSink::create(&dir.path().join("raw.qpraw")).expect("raw"),
        to_hub.clone(),
    ));
    let auth = Arc::new(qp_live::AuthClient::new(
        &cfg,
        qp_live::AuthPolicy::default(),
    ));
    let spec = ConnSpec {
        cfg,
        auth,
        topics: vec!["AAA".to_owned(), "BBB".to_owned()],
        lock_dir: dir.path().to_owned(),
        trade: false,
    };
    let ctl = spawn_connection(spec, sink, to_hub).expect("open");
    let pump =
        |rt: &mut LiveRuntime, host: &mut ReplayHost, until: &dyn Fn(&LiveRuntime) -> bool| {
            let deadline = Instant::now() + Duration::from_secs(20);
            while Instant::now() < deadline {
                if let Ok(i) = rx.recv_timeout(Duration::from_millis(50)) {
                    let _ = rt.on_in(host, i);
                }
                if until(rt) {
                    return true;
                }
            }
            false
        };
    assert!(
        pump(&mut rt, &mut host, &|r| r.link() == "live"),
        "connected"
    );
    assert!(
        broker.wait_until(Duration::from_secs(5), |s| s.subscribe_packets.len() >= 2
            || s.subscribe_packets.iter().map(Vec::len).sum::<usize>() >= 2)
    );
    #[allow(clippy::needless_range_loop)]
    for i in 30..35 {
        for s in ["AAA", "BBB"] {
            broker.publish(s, &serde_json::to_vec(&full[s][i]).expect("json"));
        }
    }
    assert!(pump(&mut rt, &mut host, &|r| r.events >= 10), "live events");
    ctl.send(ConnCtl::Drop(Duration::from_millis(300)))
        .expect("drop");
    assert!(
        pump(&mut rt, &mut host, &|r| r.is_stale()),
        "stale after drop"
    );
    assert_eq!(host.live_not_ready(), 2);
    assert!(
        pump(&mut rt, &mut host, &|r| r.link() == "live"
            && !r.repairs.is_empty()),
        "reconnected and repaired"
    );
    assert_eq!(host.live_not_ready(), 0);
    let st = broker.stats();
    assert_eq!(st.max_concurrent, 1, "never two connections at once");
    assert!(st.disconnects >= 1, "the drop sent DISCONNECT");
    for s in ["AAA", "BBB"] {
        let k = FeedKey {
            symbol: s.to_owned(),
            interval: "1d".to_owned(),
            feed: None,
        };
        // Bars here open in 2025 and arrive now: the exchange clock (arrival minus
        // the observed lag) closes only the bars up to the last one seen, so the
        // repair stops there; what it wrote equals the settled bars.
        let bars = host.live_feed(&k).expect("feed").0;
        assert!(bars.len() >= 35, "{s}: live bars kept");
        assert_eq!(bars[..], full[s][..bars.len()], "{s} repaired");
    }
    assert_eq!(rt.capsule_counts().1, 0);
    let (done_tx, done_rx) = mpsc::channel();
    ctl.send(ConnCtl::Stop(done_tx)).expect("stop");
    done_rx
        .recv_timeout(Duration::from_secs(10))
        .expect("closed");
    assert!(broker.wait_until(Duration::from_secs(5), |s| s.current == 0));
}

#[test]
fn rest_bars_are_aligned_and_closing_session_merged() {
    let c = cal();
    let day = qp_calendar::days_from_civil(2025, 3, 4);
    let s = c.sessions_on(day);
    let open = s[0].open;
    let close = s.last().expect("session").close;
    let b = |t: i64, p: f64| {
        (
            t,
            Bar {
                t,
                o: p,
                h: p + 1.0,
                l: p - 1.0,
                c: p,
                v: 10.0,
            },
        )
    };
    let rows = vec![
        b(open - 5 * 60_000, 9.0), // opening auction
        b(open, 10.0),
        b(open + 5 * 60_000, 11.0),
        b(close, 12.0), // closing session
    ];
    let out = normalize_rest(&rows, Interval::Minutes(5), &c);
    assert_eq!(out.len(), 2);
    assert_eq!(out[0].t, open);
    assert!((out[0].o - 9.0).abs() < 1e-12 && (out[0].v - 20.0).abs() < 1e-12);
    assert!((out[1].c - 12.0).abs() < 1e-12 && (out[1].v - 20.0).abs() < 1e-12);
    // Daily: local midnight → the session open.
    let mid = open - 10 * 3_600_000;
    let d = normalize_rest(&[b(mid, 5.0)], Interval::Day, &c);
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].t, c.bar_at(Interval::Day, open).expect("day").open);
}

/// (RP4, L-prep) The REST trade history adapter: windows of at most 15 days, only the
/// gap's rows, de-duplicated by trade number, exchange order, the asked symbol; a
/// fake REST (no network).
#[test]
fn rest_trade_history_windows_filter_and_dedup() {
    use std::sync::Mutex;
    let c = cal();
    let day0 = qp_calendar::parse_date("2026-10-01").expect("date");
    let midnight = |d: i64| c.sessions_on(d).first().map(|s| s.open);
    let calls: Arc<Mutex<Vec<(String, String, String)>>> = Arc::default();
    let calls2 = calls.clone();
    let t0 = midnight(day0 + 1).expect("trading day");
    let mk = |ms: i64, no: &str| Trade {
        symbol: "AAA.E".to_owned(),
        price: 10.5,
        quantity: Some(3),
        side: None,
        timestamp_ns: ms * 1_000_000,
        order_no: Some(no.to_owned()),
        is_trade_ex: false,
    };
    let rows = vec![
        mk(t0 + 2_000, "2"),
        mk(t0 + 1_000, "1"),
        mk(t0 + 1_000, "1"),           // duplicate
        mk(t0 - 86_400_000 * 2, "0"),  // before the gap
        mk(t0 + 86_400_000 * 45, "9"), // after the gap
    ];
    let fetch = move |sym: &str, a: &str, b: &str| {
        calls2
            .lock()
            .expect("calls")
            .push((sym.to_owned(), a.to_owned(), b.to_owned()));
        Ok((rows.clone(), 1))
    };
    let h = RestTradeHistory::new(Box::new(fetch), c.clone(), 99);
    let from = t0 - 3_600_000;
    let to = from + 40 * 86_400_000;
    let got = h.trades("AAA", from, to).expect("trades");
    let w = h.windows(from, to);
    assert_eq!(w.len(), 3, "41 local days in windows of at most 15: {w:?}");
    assert!(w.iter().all(|(a, b)| {
        let (a, b) = (
            qp_calendar::parse_date(a).expect("a"),
            qp_calendar::parse_date(b).expect("b"),
        );
        b >= a && b - a < RestTradeHistory::MAX_WINDOW_DAYS
    }));
    assert_eq!(calls.lock().expect("calls").len(), 3);
    assert!(calls
        .lock()
        .expect("calls")
        .iter()
        .all(|(s, _, _)| s == "AAA"));
    let nos: Vec<_> = got
        .iter()
        .map(|t| t.order_no.clone().unwrap_or_default())
        .collect();
    assert_eq!(nos, vec!["1", "2"]);
    assert!(got.iter().all(|t| t.symbol == "AAA"));
    assert_eq!(h.bad_rows.load(std::sync::atomic::Ordering::Relaxed), 3);
    // An empty or reversed gap asks nothing.
    assert!(h.trades("AAA", to, from).expect("empty").is_empty());
    assert_eq!(h.calls.load(std::sync::atomic::Ordering::Relaxed), 3);
    // A failed request is the gap's error.
    let bad = RestTradeHistory::new(
        Box::new(|_: &str, _: &str, _: &str| Err("rest: HTTP 503".to_owned())),
        c,
        15,
    );
    assert!(bad.trades("AAA", from, from + 1).is_err());
}
