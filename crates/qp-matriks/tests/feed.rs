//! `LiveFeed` (R6 T5/T4): field presence, the `updateDate` time rule, connection loss and
//! re-subscription, sequence numbers, determinism, and the seam with history (series and
//! indicators continuous). All messages are hand-built with made-up symbols and prices.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::cognitive_complexity
)]

use std::collections::BTreeMap;

use prost::Message as _;
use qp_calendar::{builtin, parse_date, ExchangeCalendar};
use qp_compute::{create, run, Field};
use qp_matriks::messages::SymbolMessage;
use qp_matriks::{
    ConnChange, FeedOut, LiveFeed, LiveFeedConfig, RawRecord, RecordKind, TimeSource, VolumeField,
};
use qp_schema::data::{DataChange, DataEvent, FeedKey};
use qp_schema::Bar;
use qp_source::{Applied, BarMethod, FeedStore};

const DAY: &str = "2026-11-18"; // Wednesday, full trading day
const SYM: &str = "TEST1";
const TOPIC: &str = "mx/symbol/TEST1@lvl2";
/// The delayed endpoint: messages arrive ~15 minutes after their exchange time.
const DELAY_MS: i64 = 900_000;

fn xist() -> ExchangeCalendar {
    ExchangeCalendar::new(&builtin("XIST").expect("XIST")).expect("calendar")
}

/// UTC ms of Istanbul local `DAY hh:mm:ss`.
fn at(hh: i64, mm: i64, ss: i64) -> i64 {
    parse_date(DAY).expect("date") * 86_400_000 + ((hh * 60 + mm) * 60 + ss) * 1000 - 3 * 3_600_000
}

/// `updateDate` text of UTC ms `t` (Istanbul, `+03`, as the delayed endpoint sends it).
fn ud(t: i64) -> String {
    let local = t + 3 * 3_600_000;
    let day = local.div_euclid(86_400_000);
    let ms = local.rem_euclid(86_400_000);
    let (y, m, d) = qp_calendar::civil_from_days(day);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}.{:03}+03",
        ms / 3_600_000,
        ms / 60_000 % 60,
        ms / 1000 % 60,
        ms % 1000
    )
}

fn msg(arrival_ms: i64, m: &SymbolMessage) -> RawRecord {
    RawRecord::message(arrival_ms * 1000, TOPIC, m.encode_to_vec())
}

fn rec(kind: RecordKind, arrival_ms: i64, topic: &str) -> RawRecord {
    RawRecord {
        t_us: arrival_ms * 1000,
        kind,
        topic: topic.into(),
        payload: Vec::new(),
    }
}

/// A full initial picture at exchange time `t`.
fn snapshot(t: i64, last: f64, cum: f64) -> SymbolMessage {
    SymbolMessage {
        symbol_id: 7,
        symbol_code: Some(SYM.into()),
        update_date: Some(ud(t)),
        last: Some(last),
        open: Some(last),
        daily_high: Some(last),
        daily_low: Some(last),
        daily_volume: Some(cum),
        volume: Some(cum),
        daily_quantity: Some(cum / last),
        quantity: Some(cum / last),
        stock_status: Some(2),
        ask_size: Some(100),
        bid_size: Some(100),
        ..SymbolMessage::default()
    }
}

/// A trade update at exchange time `t` (the fields that come with `last`).
fn trade(t: i64, last: f64, cum: f64) -> SymbolMessage {
    SymbolMessage {
        symbol_id: 7,
        update_date: Some(ud(t)),
        last: Some(last),
        daily_volume: Some(cum),
        volume: Some(cum),
        daily_quantity: Some(cum / last),
        quantity: Some(cum / last),
        last_quantity: Some(1.0),
        ..SymbolMessage::default()
    }
}

fn cfg(intervals: &[&str]) -> LiveFeedConfig {
    LiveFeedConfig {
        intervals: intervals.iter().map(|s| (*s).to_owned()).collect(),
        ..LiveFeedConfig::default()
    }
}

fn key(iv: &str) -> FeedKey {
    FeedKey {
        symbol: SYM.into(),
        interval: iv.into(),
        feed: None,
    }
}

fn run_all(feed: &mut LiveFeed, recs: &[RawRecord]) -> Vec<FeedOut> {
    recs.iter().map(|r| feed.push(r)).collect()
}

fn events(outs: &[FeedOut]) -> Vec<DataEvent> {
    outs.iter().flat_map(|o| o.events.clone()).collect()
}

/// Final bars of one feed from its events (applied in a `FeedStore`).
fn bars_of(evs: &[DataEvent], iv: &str) -> Vec<Bar> {
    let mut store = FeedStore::new(key(iv));
    for e in evs.iter().filter(|e| e.feed.interval == iv) {
        let a = store.apply(e).expect("event applies");
        assert!(
            matches!(a, Applied::NewBar { .. } | Applied::LiveUpdate { .. }),
            "{a:?}"
        );
    }
    store.bars().to_vec()
}

fn session(start_ms: i64) -> Vec<RawRecord> {
    // A day: snapshot before the open, trades every 20 s from 10:00:10 to 10:12, with an
    // ask-size-only update between trades. Arrival = exchange time + 15 min.
    let mut out = vec![
        rec(RecordKind::ConnectionOpened, start_ms, "market"),
        msg(start_ms + 5, &snapshot(at(9, 55, 0), 100.0, 0.0)),
        rec(RecordKind::Subscribed, start_ms + 10, TOPIC),
    ];
    let mut cum = 0.0;
    let mut px = 100.0;
    let mut t = at(10, 0, 10);
    let mut i = 0;
    while t < at(10, 12, 0) {
        i += 1;
        px += if i % 3 == 0 { -0.25 } else { 0.5 };
        cum += 1000.0 + f64::from(i) * 10.0;
        out.push(msg(t + DELAY_MS, &trade(t, px, cum)));
        let ask = SymbolMessage {
            symbol_id: 7,
            ask_size: Some(i64::from(i)),
            ..SymbolMessage::default()
        };
        out.push(msg(t + DELAY_MS + 300, &ask));
        t += 20_000;
    }
    out
}

#[test]
fn ask_size_only_update_builds_nothing() {
    let mut feed = LiveFeed::new(cfg(&["1m"]), xist());
    feed.push(&msg(
        at(10, 0, 0) + DELAY_MS,
        &snapshot(at(9, 59, 0), 50.0, 0.0),
    ));
    let t = at(10, 1, 0);
    let first = feed.push(&msg(t + DELAY_MS, &trade(t, 51.0, 500.0)));
    assert_eq!(first.events.len(), 1);
    let before = feed.stats();
    let ask = SymbolMessage {
        symbol_id: 7,
        ask_size: Some(3),
        bid_size: Some(4),
        ..SymbolMessage::default()
    };
    let out = feed.push(&msg(t + DELAY_MS + 100, &ask));
    assert!(out.events.is_empty());
    assert!(out.clock_ms.is_none());
    let after = feed.stats();
    assert_eq!(after.no_bar_fields, before.no_bar_fields + 1);
    assert_eq!(after.ticks, before.ticks);
    // The bar keeps the trade's price (an absent field is not 0).
    let (b, _) = feed.feeds().next().unwrap().1.current().unwrap();
    assert_eq!(b.c.to_bits(), 51.0f64.to_bits());
}

#[test]
fn update_date_times_ticks_and_moves_the_clock() {
    let mut feed = LiveFeed::new(cfg(&["1m"]), xist());
    feed.push(&msg(
        at(10, 0, 0) + DELAY_MS,
        &snapshot(at(9, 59, 0), 50.0, 0.0),
    ));
    // Exchange time 10:01:30, arrival 10:16:30: the tick belongs to the 10:01 bar.
    let t = at(10, 1, 30);
    let out = feed.push(&msg(t + DELAY_MS, &trade(t, 51.0, 500.0)));
    let DataChange::Bar { bar } = &out.events[0].change else {
        panic!("expected a bar: {out:?}");
    };
    assert_eq!(bar.t, at(10, 1, 0));
    assert_eq!(out.clock_ms, Some(t));
    // A trade without updateDate: last known exchange time + arrival difference.
    let mut m = trade(0, 52.0, 900.0);
    m.update_date = None;
    let out = feed.push(&msg(t + DELAY_MS + 45_000, &m));
    assert!(
        out.clock_ms.is_none(),
        "derived times do not move the clock"
    );
    let DataChange::Bar { bar } = &out.events[0].change else {
        panic!("expected a new 10:02 bar: {out:?}");
    };
    assert_eq!(bar.t, at(10, 2, 0));
    assert_eq!(feed.stats().time_derived, 1);
    // The 10:01 bar became final when 10:02 opened.
    assert_eq!(out.finalized, vec![(key("1m"), at(10, 1, 0))]);

    // Arrival mode: the same messages land 15 minutes later.
    let mut arr = LiveFeed::new(
        LiveFeedConfig {
            time: TimeSource::Arrival,
            ..cfg(&["1m"])
        },
        xist(),
    );
    arr.push(&msg(
        at(10, 0, 0) + DELAY_MS,
        &snapshot(at(9, 59, 0), 50.0, 0.0),
    ));
    let out = arr.push(&msg(t + DELAY_MS, &trade(t, 51.0, 500.0)));
    let DataChange::Bar { bar } = &out.events[0].change else {
        panic!("{out:?}");
    };
    assert_eq!(bar.t, at(10, 16, 0));
}

#[test]
fn update_date_parses_offsets() {
    let t = at(14, 2, 3) + 123;
    assert_eq!(qp_matriks::parse_update_date(&ud(t)), Some(t));
    assert_eq!(
        qp_matriks::parse_update_date("2026-11-18T14:02:03.123+03:00"),
        Some(t)
    );
    assert_eq!(
        qp_matriks::parse_update_date("2026-11-18T11:02:03.123Z"),
        Some(t)
    );
    assert_eq!(
        qp_matriks::parse_update_date("2026-11-18T14:02:03.123"),
        Some(t)
    );
    assert_eq!(qp_matriks::parse_update_date("2026-11-18"), None);
    assert_eq!(qp_matriks::parse_update_date("2026-11-18T14:02:0x"), None);
}

#[test]
fn clock_finalizes_bars_with_grace() {
    let mut feed = LiveFeed::new(cfg(&["1m", "5m"]), xist());
    feed.push(&msg(
        at(10, 0, 0) + DELAY_MS,
        &snapshot(at(9, 59, 0), 50.0, 0.0),
    ));
    let t = at(10, 1, 30);
    feed.push(&msg(t + DELAY_MS, &trade(t, 51.0, 500.0)));
    // Another symbol's tick at 10:02:01 moves the clock past 10:02 + grace (2 s)? No.
    let other = |t: i64| {
        RawRecord::message(
            (t + DELAY_MS) * 1000,
            "mx/symbol/TEST2@lvl2",
            SymbolMessage {
                symbol_id: 8,
                update_date: Some(ud(t)),
                ask_size: Some(1),
                ..SymbolMessage::default()
            }
            .encode_to_vec(),
        )
    };
    let out = feed.push(&other(at(10, 2, 1)));
    assert_eq!(out.clock_ms, Some(at(10, 2, 1)));
    assert!(out.finalized.is_empty());
    let out = feed.push(&other(at(10, 2, 2)));
    assert_eq!(out.finalized, vec![(key("1m"), at(10, 1, 0))]);
    // A late tick for 10:01 is not written.
    let out = feed.push(&msg(
        at(10, 1, 59) + DELAY_MS,
        &trade(at(10, 1, 59), 60.0, 600.0),
    ));
    assert!(out.events.iter().all(|e| e.feed.interval == "5m"));
    assert_eq!(feed.stats().late, 1);
    // External clock: exact.
    let out = feed.advance_clock(at(10, 5, 0));
    assert_eq!(out.clock_ms, Some(at(10, 5, 0)));
    assert_eq!(out.finalized, vec![(key("5m"), at(10, 0, 0))]);
    assert!(feed.advance_clock(at(10, 4, 0)).is_empty());
}

#[test]
fn connection_loss_and_resubscription() {
    let mut feed = LiveFeed::new(cfg(&["1m"]), xist());
    let a0 = at(10, 20, 0);
    let out = feed.push(&rec(RecordKind::ConnectionOpened, a0, "market"));
    assert_eq!(out.connection, Some(ConnChange::Opened { session: 1 }));
    // First message before its Subscribed record (seen in the real log order).
    feed.push(&msg(a0 + 10, &snapshot(at(10, 4, 0), 50.0, 1000.0)));
    feed.push(&rec(RecordKind::Subscribed, a0 + 20, TOPIC));
    assert!(!feed.book().awaiting_initial(SYM));
    assert_eq!(feed.stats().resubscribes, 0);
    let t = at(10, 5, 10);
    feed.push(&msg(t + DELAY_MS, &trade(t, 51.0, 1500.0)));
    assert_eq!(feed.stats().initials, 1);

    let out = feed.push(&rec(RecordKind::ConnectionLost, a0 + 60_000, ""));
    assert_eq!(out.connection, Some(ConnChange::Lost));
    assert!(feed.book().awaiting_initial(SYM));
    let out = feed.push(&rec(RecordKind::ConnectionOpened, a0 + 90_000, "market"));
    assert_eq!(out.connection, Some(ConnChange::Opened { session: 2 }));
    feed.push(&rec(RecordKind::Subscribed, a0 + 90_010, TOPIC));
    assert_eq!(feed.stats().resubscribes, 1);
    // The first message after the reconnect is an initial; its counter continues the day.
    let t2 = at(10, 6, 40);
    let out = feed.push(&msg(t2 + DELAY_MS, &snapshot(t2, 52.0, 2000.0)));
    assert_eq!(feed.stats().initials, 2);
    let DataChange::Bar { bar } = &out.events[0].change else {
        panic!("{out:?}");
    };
    assert_eq!(bar.t, at(10, 6, 0));
    assert_eq!(bar.v.to_bits(), 500.0f64.to_bits());
    // A Subscribed after the symbol already had its message is not a re-subscription.
    feed.push(&rec(RecordKind::Subscribed, t2 + DELAY_MS + 5, TOPIC));
    assert_eq!(feed.stats().resubscribes, 1);
    assert!(!feed.book().awaiting_initial(SYM));
}

#[test]
fn phases_are_reported_on_change() {
    let mut feed = LiveFeed::new(cfg(&["1m"]), xist());
    let out = feed.push(&msg(at(10, 0, 0), &snapshot(at(9, 59, 0), 50.0, 0.0)));
    assert_eq!(out.phases, vec![(SYM.to_owned(), 2)]);
    let mut m = trade(at(10, 0, 5), 50.5, 10.0);
    m.stock_status = Some(2);
    assert!(feed.push(&msg(at(10, 0, 6), &m)).phases.is_empty());
    m.stock_status = Some(3);
    assert_eq!(
        feed.push(&msg(at(10, 0, 7), &m)).phases,
        vec![(SYM.to_owned(), 3)]
    );
}

#[test]
fn sequences_are_gapless_per_feed_and_replay_is_deterministic() {
    let recs = session(at(9, 50, 0));
    let run_once = || {
        let mut feed = LiveFeed::new(cfg(&["1m", "5m", "1h", "1d"]), xist());
        let outs = run_all(&mut feed, &recs);
        (format!("{outs:?}"), events(&outs), feed.stats())
    };
    let (a, evs, stats) = run_once();
    let (b, _, stats_b) = run_once();
    assert_eq!(a, b);
    assert_eq!(stats, stats_b);
    let mut next: BTreeMap<FeedKey, u64> = BTreeMap::new();
    for e in &evs {
        let n = next.entry(e.feed.clone()).or_insert(1);
        assert_eq!(e.seq, *n, "{e:?}");
        *n += 1;
    }
    assert_eq!(next.len(), 4);
    assert_eq!(bars_of(&evs, "1m").len(), 12);
    assert_eq!(bars_of(&evs, "5m").len(), 3);
    assert_eq!(bars_of(&evs, "1d").len(), 1);
    assert!(stats.no_bar_fields > 30);
    assert_eq!(stats.late, 0);
    // Volume sums match the counter.
    let day = bars_of(&evs, "1d");
    let one: f64 = bars_of(&evs, "1m").iter().map(|b| b.v).sum();
    assert_eq!(day[0].v, one);
}

#[test]
fn volume_field_choice_reads_the_chosen_counter() {
    let mut recs = vec![msg(at(10, 0, 0), &snapshot(at(9, 59, 0), 50.0, 0.0))];
    let t = at(10, 1, 0);
    let mut m = trade(t, 51.0, 1000.0);
    m.volume = Some(77.0);
    recs.push(msg(t, &m));
    let v = |field| {
        let mut feed = LiveFeed::new(
            LiveFeedConfig {
                volume_field: field,
                ..cfg(&["1m"])
            },
            xist(),
        );
        bars_of(&events(&run_all(&mut feed, &recs)), "1m")[0].v
    };
    assert_eq!(v(VolumeField::DailyVolume), 1000.0);
    assert_eq!(v(VolumeField::Volume), 77.0);
}

#[test]
fn card_daily_falls_back_to_ticks_intraday() {
    let mut feed = LiveFeed::new(
        LiveFeedConfig {
            method: BarMethod::CardDaily,
            ..cfg(&["1m", "1d"])
        },
        xist(),
    );
    let evs = events(&run_all(&mut feed, &session(at(9, 50, 0))));
    assert_eq!(bars_of(&evs, "1m").len(), 12);
    assert_eq!(bars_of(&evs, "1d").len(), 1);
    assert_eq!(feed.stats().bad_intervals, 0);
}

/// T4 seam: REST history (the reference's bars up to a cut, last one partial) + live
/// from the cut equals a series built live from the start; indicators streamed across the
/// seam equal batch computation over the joined series, bit for bit.
#[test]
fn seam_with_history_is_continuous_for_bars_and_indicators() {
    let recs = session(at(9, 50, 0));
    let mut whole = LiveFeed::new(cfg(&["1m"]), xist());
    let all = events(&run_all(&mut whole, &recs));
    let reference = bars_of(&all, "1m");

    // Cut inside the 10:04 bar: records up to exchange time 10:04:30.
    let cut_t = at(10, 4, 30);
    let cut = recs
        .iter()
        .position(|r| r.kind == RecordKind::Message && r.t_us / 1000 > cut_t + DELAY_MS)
        .unwrap();
    let mut head = LiveFeed::new(cfg(&["1m"]), xist());
    let head_events = events(&run_all(&mut head, &recs[..cut]));
    let history = bars_of(&head_events, "1m");
    let hist_seq = head_events.last().unwrap().seq;
    let cum_at_cut: f64 = history.iter().map(|b| b.v).sum();
    assert_eq!(history.last().unwrap().t, at(10, 4, 0));

    // Live from the cut, after a fresh connection: the first message is a full picture.
    let mut live = LiveFeed::new(cfg(&["1m"]), xist());
    live.seed_with_counter(
        SYM,
        "1m",
        history.last().cloned(),
        hist_seq + 1,
        Some(cum_at_cut),
    )
    .unwrap();
    let mut tail = vec![rec(
        RecordKind::ConnectionOpened,
        cut_t + DELAY_MS,
        "market",
    )];
    let mut first = true;
    for r in &recs[cut..] {
        if first && r.kind == RecordKind::Message {
            let m = SymbolMessage::decode(r.payload.as_slice()).unwrap();
            if m.last.is_some() {
                // Turn the first trade into the reconnect snapshot (same values).
                let mut s = snapshot(0, m.last.unwrap(), m.daily_volume.unwrap());
                s.update_date = m.update_date.clone();
                s.open = Some(100.0);
                s.daily_high = None;
                s.daily_low = None;
                tail.push(RawRecord::message(r.t_us, TOPIC, s.encode_to_vec()));
                first = false;
                continue;
            }
        }
        tail.push(r.clone());
    }
    let mut store = FeedStore::with_bars(key("1m"), history.clone(), hist_seq).unwrap();
    let mut studies: Vec<_> = ["ema", "rsi"]
        .iter()
        .map(|n| create(n, &BTreeMap::new(), Field::Close).unwrap())
        .collect();
    let hist_close = Field::Close.column(&history);
    let mut outs: Vec<Vec<Option<f64>>> = studies
        .iter()
        .map(|s| vec![None; s.outputs().len()])
        .collect();
    let mut streamed: Vec<Vec<Option<f64>>> = vec![Vec::new(); studies.len()];
    for (s, o) in studies.iter_mut().zip(outs.iter_mut()) {
        s.extend(&[&hist_close]);
        s.update_last(&[history.last().unwrap().c], o);
    }
    for r in &tail {
        for e in live.push(r).events {
            match store.apply(&e).unwrap() {
                Applied::NewBar { index } => {
                    for ((s, o), col) in studies.iter_mut().zip(outs.iter_mut()).zip(&mut streamed)
                    {
                        col.push(o[0]);
                        s.push(&[store.bars()[index].c], o);
                    }
                }
                Applied::LiveUpdate { index } => {
                    for (s, o) in studies.iter_mut().zip(outs.iter_mut()) {
                        s.update_last(&[store.bars()[index].c], o);
                    }
                }
                other => panic!("{other:?}"),
            }
        }
    }
    for (o, col) in outs.iter().zip(&mut streamed) {
        col.push(o[0]);
    }
    let bits = |v: &[Bar]| -> Vec<[u64; 6]> {
        v.iter()
            .map(|b| {
                [
                    b.t as u64,
                    b.o.to_bits(),
                    b.h.to_bits(),
                    b.l.to_bits(),
                    b.c.to_bits(),
                    b.v.to_bits(),
                ]
            })
            .collect()
    };
    assert_eq!(
        bits(store.bars()),
        bits(&reference),
        "joined series = live from start"
    );

    let seam = history.len() - 1;
    for (i, n) in ["ema", "rsi"].iter().enumerate() {
        let mut batch_study = create(n, &BTreeMap::new(), Field::Close).unwrap();
        let batch = run(batch_study.as_mut(), &[&Field::Close.column(&reference)]).unwrap();
        let want: Vec<Option<u64>> = batch[0][seam..]
            .iter()
            .map(|x| x.map(f64::to_bits))
            .collect();
        let got: Vec<Option<u64>> = streamed[i].iter().map(|x| x.map(f64::to_bits)).collect();
        assert_eq!(got, want, "{n} across the seam");
    }
}

#[test]
fn seed_rules() {
    let mut feed = LiveFeed::new(cfg(&["1m"]), xist());
    let misaligned = Bar {
        t: at(10, 4, 30),
        o: 1.0,
        h: 1.0,
        l: 1.0,
        c: 1.0,
        v: 0.0,
    };
    assert!(feed.seed(SYM, "1m", Some(misaligned), 5).is_err());
    assert!(feed.seed(SYM, "nope", None, 5).is_err());
    feed.seed(SYM, "1m", None, 5).unwrap();
    feed.push(&msg(at(10, 0, 0), &snapshot(at(9, 59, 0), 50.0, 0.0)));
    let out = feed.push(&msg(at(10, 1, 0), &trade(at(10, 1, 0), 51.0, 100.0)));
    assert_eq!(out.events[0].seq, 5);
    assert!(feed.seed(SYM, "1m", None, 9).is_err());
}

/// An initial whose `updateDate` is the symbol's old last trade (hours before the clock)
/// still becomes the day's volume base: the next trade's bar carries only its increment.
#[test]
fn stale_initial_still_sets_the_volume_base() {
    let mut feed = LiveFeed::new(cfg(&["1m"]), xist());
    let now = at(14, 0, 0);
    let other = SymbolMessage {
        symbol_id: 8,
        update_date: Some(ud(now)),
        last: Some(5.0),
        daily_volume: Some(10.0),
        ..SymbolMessage::default()
    };
    feed.push(&RawRecord::message(
        (now + DELAY_MS) * 1000,
        "mx/symbol/TEST2@lvl2",
        other.encode_to_vec(),
    ));
    assert_eq!(feed.clock_ms(), Some(now));
    feed.push(&msg(
        now + DELAY_MS + 10,
        &snapshot(at(10, 35, 0), 50.0, 5000.0),
    ));
    let t = at(14, 0, 30);
    let out = feed.push(&msg(t + DELAY_MS, &trade(t, 50.5, 5100.0)));
    let bar = out
        .events
        .iter()
        .find_map(|e| match &e.change {
            DataChange::Bar { bar } if e.feed.symbol == SYM => Some(bar.clone()),
            _ => None,
        })
        .expect("a TEST1 bar");
    assert_eq!(bar.t, at(14, 0, 0));
    assert_eq!(bar.v.to_bits(), 100.0f64.to_bits());
    assert_eq!(feed.stats().late, 0);
    assert_eq!(feed.stats().behind_clock, 0);
}
