//! (RP4, L-prep) The live trade link: market and trade endpoints together (RG1: split out of `sim/tests.rs`).

use super::*;

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

#[allow(clippy::too_many_lines, clippy::cognitive_complexity)]
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

#[allow(clippy::too_many_lines)]
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

#[allow(clippy::too_many_lines, clippy::cognitive_complexity)]
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
