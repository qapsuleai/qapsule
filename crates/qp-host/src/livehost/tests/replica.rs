//! Late and incremental replicas against the live host (RG1: split out of `livehost/tests.rs`).

use super::*;

/// What `qp demo` gives a full replica (`mode=replica`) that connects now: the
/// start frame's data (the initial bars) and the events recorded since.
fn replica_core(host: &ReplayHost) -> qp_shellcore::shell::ShellCore {
    let data = host.init().data.clone().expect("init data");
    qp_shellcore::shell::ShellCore::new("arkin", "tr", &data, &host.recorded_events())
        .expect("core")
}

/// Passes the core's requests to the host as connection `conn` until it is idle.
fn settle(core: &mut qp_shellcore::shell::ShellCore, host: &mut ReplayHost, conn: &str) {
    for _ in 0..1000 {
        let out = core.take_outbox();
        if out.is_empty() {
            break;
        }
        for m in out {
            deliver(core, conn, host.handle_raw(conn, &m.to_string()));
        }
    }
}

fn deliver(core: &mut qp_shellcore::shell::ShellCore, conn: &str, lines: Vec<OutboundLine>) {
    for l in lines {
        if l.conn == conn {
            core.on_message(&l.msg);
        }
    }
}

/// RP4/T5 (R6 observation, REPORT §10): a full replica that joins a live host late
/// holds the same series as one connected from the start — the history and every
/// live bar since, also over a drop and its repair.
#[test]
#[allow(clippy::needless_range_loop)]
fn late_full_replica_has_the_same_series_as_an_early_one() {
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
    let mut rt = LiveRuntime::new(
        &host,
        Box::new(FakeFeed::default()),
        Arc::new(FakeSource(full.clone())),
        to_hub,
        cal(),
        "1d",
        0,
    )
    .expect("runtime");
    // The early replica connects before any live bar.
    let mut early = replica_core(&host);
    settle(&mut early, &mut host, "c1");
    let mut sent = host.recorded_len();
    // What the hub does after each batch: new events as a data frame to the replica,
    // the authority's notifications routed to it.
    let mut push = |early: &mut qp_shellcore::shell::ShellCore,
                    host: &mut ReplayHost,
                    lines: Vec<OutboundLine>| {
        let mut lines = lines;
        lines.extend(host.live_flush());
        let evs = host.recorded_events();
        if evs.len() > sent {
            let frame = serde_json::json!({"data": &evs[sent..]}).to_string();
            early.on_frame(&frame).expect("data frame");
            sent = evs.len();
        }
        deliver(early, "c1", lines);
        settle(early, host, "c1");
    };
    let l = rt.on_in(
        &mut host,
        LiveIn::Record(conn(RecordKind::ConnectionOpened)),
    );
    push(&mut early, &mut host, l);
    for i in 40..46 {
        for s in syms {
            let l = rt.on_in(&mut host, LiveIn::Record(msg(s, &full[s][i])));
            push(&mut early, &mut host, l);
        }
    }
    // A drop and its repair (REST patches and bars go in as recorded events).
    let l = rt.on_in(&mut host, LiveIn::Record(conn(RecordKind::ConnectionLost)));
    push(&mut early, &mut host, l);
    let l = rt.on_in(
        &mut host,
        LiveIn::Record(conn(RecordKind::ConnectionOpened)),
    );
    push(&mut early, &mut host, l);
    let repaired = rx.recv_timeout(Duration::from_secs(10)).expect("repair");
    let l = rt.on_in(&mut host, repaired);
    push(&mut early, &mut host, l);
    assert_eq!(rt.link(), "live");
    for i in 70..73 {
        for s in syms {
            let l = rt.on_in(&mut host, LiveIn::Record(msg(s, &full[s][i])));
            push(&mut early, &mut host, l);
        }
    }
    // The late replica joins now.
    let mut late = replica_core(&host);
    settle(&mut late, &mut host, "c2");
    for s in syms {
        let id = s.to_lowercase();
        let (want, _) = host
            .live_feed(&FeedKey {
                symbol: s.to_owned(),
                interval: "1d".to_owned(),
                feed: None,
            })
            .expect("feed");
        assert!(want.len() > 40, "{s}: live bars were added");
        let e = early.replica_mut().bars(&id).expect("early bars").to_vec();
        let l = late.replica_mut().bars(&id).expect("late bars").to_vec();
        let enc = |b: &[Bar]| serde_json::to_vec(b).expect("json");
        assert_eq!(
            enc(&e),
            enc(&want),
            "{s}: the early replica follows the host"
        );
        assert_eq!(
            enc(&l),
            enc(&e),
            "{s}: late replica byte-equal to the early one"
        );
    }
    // What the page shows: the document, every face in the feed, the open scene.
    early.requery_feed();
    late.requery_feed();
    settle(&mut early, &mut host, "c1");
    settle(&mut late, &mut host, "c2");
    let (de, dl) = (early.digests(), late.digests());
    assert!(
        de["faces"].as_array().is_some_and(|f| !f.is_empty()),
        "capsules on screen: {de}"
    );
    assert_eq!(dl, de, "late replica shows what the early one shows");
}

/// The same check on the real feed engine (`MatriksFeed`, made-up protobuf ticks of
/// `live::tests::fake_records`: a snapshot, trades, ask-only updates, a drop and a
/// reconnect), 5-minute bars, and the bars closed by the exchange clock.
#[test]
fn late_full_replica_matches_on_the_matriks_feed() {
    let c = cal();
    let day = qp_calendar::days_from_civil(2026, 11, 17);
    let open = day * 86_400_000 - 3 * 3_600_000 + 10 * 3_600_000;
    let hist_bars: Vec<Bar> = (0..40)
        .map(|i| {
            #[allow(clippy::cast_precision_loss)]
            let p = 10.0 + (i as f64 * 0.4).sin() * 0.2;
            Bar {
                t: open + i * 300_000,
                o: p,
                h: p + 0.05,
                l: p - 0.05,
                c: p,
                v: 1000.0,
            }
        })
        .collect();
    let feeds = vec![qp_source::ReplayFeed {
        key: FeedKey {
            symbol: "TEST1".to_owned(),
            interval: "5m".to_owned(),
            feed: None,
        },
        bars: hist_bars.clone(),
        fine: Vec::new(),
    }];
    let init = qp_session::demo_init(qp_schema::protocol::ReplayInit {
        interval: Some("5m".to_owned()),
        ..Default::default()
    });
    let mut host = ReplayHost::live(init, std::path::Path::new("."), feeds).expect("live host");
    let (tx, rx) = mpsc::channel::<LiveIn>();
    let tx = std::sync::Mutex::new(tx);
    let to_hub: Arc<dyn Fn(LiveIn) + Send + Sync> = Arc::new(move |i| {
        let _ = tx
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .send(i);
    });
    let source: BTreeMap<String, Vec<Bar>> =
        [("TEST1".to_owned(), hist_bars)].into_iter().collect();
    let mut rt = LiveRuntime::new(
        &host,
        Box::new(MatriksFeed::new(c, "5m")),
        Arc::new(FakeSource(source)),
        to_hub,
        cal(),
        "5m",
        900_000,
    )
    .expect("runtime");
    let mut early = replica_core(&host);
    settle(&mut early, &mut host, "c1");
    let mut sent = host.recorded_len();
    let mut push = |early: &mut qp_shellcore::shell::ShellCore,
                    host: &mut ReplayHost,
                    lines: Vec<OutboundLine>| {
        let mut lines = lines;
        lines.extend(host.live_flush());
        let evs = host.recorded_events();
        if evs.len() > sent {
            let frame = serde_json::json!({"data": &evs[sent..]}).to_string();
            early.on_frame(&frame).expect("data frame");
            sent = evs.len();
        }
        deliver(early, "c1", lines);
        settle(early, host, "c1");
    };
    let recs = crate::live::tests::fake_records();
    let last_t = recs.last().map_or(0, |r| r.t_us / 1000);
    for r in recs {
        let l = rt.on_in(&mut host, LiveIn::Record(r));
        push(&mut early, &mut host, l);
        if rt.link() == "repairing" {
            if let Ok(i) = rx.recv_timeout(Duration::from_secs(10)) {
                let l = rt.on_in(&mut host, i);
                push(&mut early, &mut host, l);
            }
        }
    }
    let out = rt.feed.advance_clock(last_t + 30 * 60_000);
    let l = rt.apply(&mut host, out, None);
    push(&mut early, &mut host, l);
    assert!(rt.events > 0, "live events");
    let mut late = replica_core(&host);
    settle(&mut late, &mut host, "c2");
    let k = FeedKey {
        symbol: "TEST1".to_owned(),
        interval: "5m".to_owned(),
        feed: None,
    };
    let (want, _) = host.live_feed(&k).expect("feed");
    assert!(want.len() > 40, "live bars were added: {}", want.len());
    let enc = |b: &[Bar]| serde_json::to_vec(b).expect("json");
    let e = early.replica_mut().bars("test1").expect("early").to_vec();
    let l = late.replica_mut().bars("test1").expect("late").to_vec();
    assert_eq!(enc(&e), enc(&want), "the early replica follows the host");
    assert_eq!(enc(&l), enc(&e), "late replica byte-equal to the early one");
}

/// (RP4/T6) Incremental = from scratch on the live path: two identical live hosts take
/// the same data events (a fixed, uneven order: symbol `k` moves every `k + 1`-th
/// round, patches inside a bar and new bars), clocks, market statuses and messages;
/// one keeps its `capsules` item memo, the other forgets it before every flush and
/// query. Every line both send is byte-equal; the memo is reused across rounds (the
/// symbols that did not move).
#[test]
fn incremental_viewer_equals_from_scratch_on_live_events() {
    let syms = ["AAA", "BBB", "CCC", "DDD", "EEE", "FFF"];
    let full: BTreeMap<String, Vec<Bar>> = syms
        .iter()
        .enumerate()
        .map(|(i, s)| {
            #[allow(clippy::cast_precision_loss)]
            let seed = i as f64 * 1.7;
            ((*s).to_owned(), truth(200, seed))
        })
        .collect();
    let hist: BTreeMap<String, Vec<Bar>> = full
        .iter()
        .map(|(s, b)| (s.clone(), b[..120].to_vec()))
        .collect();
    let mut a = live_host(&hist);
    let mut b = live_host(&hist);
    let text = |l: &[OutboundLine]| serde_json::to_string(l).expect("json");
    let setup = [
        (
            "h",
            r#"{"type":"hello","protocol":"qp/0","principal":"host","surface":{"kind":"phone","lang":"tr"}}"#,
        ),
        (
            "h",
            r#"{"type":"input","id":"s","gesture":{"kind":"style","voice":"marla","clock":true}}"#,
        ),
        (
            "h",
            r#"{"type":"subscribe","id":"v","topic":{"kind":"viewer"}}"#,
        ),
        (
            "e",
            r#"{"type":"hello","protocol":"qp/0","principal":"host","surface":{"kind":"phone","lang":"en"}}"#,
        ),
        (
            "e",
            r#"{"type":"subscribe","id":"v","topic":{"kind":"viewer","limit":30}}"#,
        ),
    ];
    for (c, m) in setup {
        assert_eq!(text(&a.handle_raw(c, m)), text(&b.handle_raw(c, m)), "{m}");
    }
    let query = r#"{"type":"query","id":"q","query":{"kind":"capsules"}}"#;
    // Per symbol: the next bar index and how far into it (patches before the close).
    let mut at: BTreeMap<&str, (usize, u32)> = syms.iter().map(|s| (*s, (120, 0))).collect();
    let mut clock = 0;
    for round in 0..240usize {
        for (k, s) in syms.iter().enumerate() {
            if round % (k + 1) != 0 {
                continue;
            }
            let (i, part) = at[s];
            if i >= 200 {
                continue;
            }
            let truth_bar = &full[*s][i];
            // Three partial looks at the bar (patches), then it is whole.
            let f = f64::from(part + 1) / 4.0;
            let bar = Bar {
                c: truth_bar.o + (truth_bar.c - truth_bar.o) * f,
                v: truth_bar.v * f,
                ..truth_bar.clone()
            };
            let key = FeedKey {
                symbol: (*s).to_owned(),
                interval: "1d".to_owned(),
                feed: None,
            };
            let (store, seq) = a.live_feed(&key).expect("feed");
            let change = match store.last() {
                Some(l) if l.t == bar.t => DataChange::Patch {
                    t: bar.t,
                    fields: qp_source::bar_patch_diff(l, &bar),
                },
                _ => DataChange::Bar { bar: bar.clone() },
            };
            let ev = DataEvent {
                feed: key,
                seq: seq + 1,
                change,
            };
            assert_eq!(
                text(&a.live_event(&ev).expect("a")),
                text(&b.live_event(&ev).expect("b")),
                "round {round} {s}"
            );
            at.insert(s, if part == 3 { (i + 1, 0) } else { (i, part + 1) });
            if bar.t > clock {
                clock = bar.t;
                let la = a.session_mut().host_clock(clock).expect("clock a");
                let lb = b.session_mut().host_clock(clock).expect("clock b");
                assert_eq!(text(&la), text(&lb), "round {round}: clock");
            }
        }
        if round % 25 == 5 {
            let m = qp_schema::market::MarketStatus {
                exchange_time: clock,
                open: Some(round % 50 == 5),
                holiday: Some(false),
                source: qp_schema::market::MarketSource::Calendar,
                delayed: false,
                stale: false,
                phases: BTreeMap::new(),
            };
            b.session().forget_item_memo();
            assert_eq!(
                text(&a.set_market_status(m.clone())),
                text(&b.set_market_status(m)),
                "round {round}: market"
            );
        }
        b.session().forget_item_memo();
        assert_eq!(
            text(&a.live_flush()),
            text(&b.live_flush()),
            "round {round}: flush"
        );
        if round % 9 == 4 {
            for c in ["h", "e"] {
                b.session().forget_item_memo();
                assert_eq!(
                    text(&a.handle_raw(c, query)),
                    text(&b.handle_raw(c, query)),
                    "round {round}: query {c}"
                );
            }
        }
    }
    let caps = a.session().document().capsules.len();
    let (hits, misses) = a.session().item_memo_counts();
    assert!(caps > 10, "capsules were born ({caps})");
    assert!(
        hits > misses,
        "items were reused across rounds: {hits} reused, {misses} written"
    );
}
