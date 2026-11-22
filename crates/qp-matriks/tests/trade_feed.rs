//! (RP4 T3) `LiveFeed` with the trade stream (rule 9): no initial state (the day so far is
//! a gap filled from history), stale while a gap is open, repair after a lost trade
//! connection with de-duplication by trade number, bars bit-equal to bars built from the
//! full trade list, the price card kept for phases only, special trades optional.
//! Hand-built trades with made-up symbols and prices; no network.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::cognitive_complexity
)]

use std::cell::RefCell;

use prost::Message as _;
use qp_calendar::{builtin, parse_date, ExchangeCalendar};
use qp_matriks::messages::{SymbolMessage, TradeMessage};
use qp_matriks::{
    ConnChange, FeedOut, FeedSource, LiveFeed, LiveFeedConfig, RawRecord, RecordKind, Trade,
    TradeFeedConfig, TradeGap, TradeHistory,
};
use qp_schema::data::{DataEvent, FeedKey};
use qp_schema::Bar;
use qp_source::{BarMethod, FeedStore, LiveBars, LiveConfig, SpecialTrades};

const DAY: &str = "2026-11-18";
const SYM: &str = "TEST1";

fn xist() -> ExchangeCalendar {
    ExchangeCalendar::new(&builtin("XIST").expect("XIST")).expect("calendar")
}

fn at(hh: i64, mm: i64, ss: i64) -> i64 {
    parse_date(DAY).expect("date") * 86_400_000 + ((hh * 60 + mm) * 60 + ss) * 1000 - 3 * 3_600_000
}

fn local_midnight() -> i64 {
    at(0, 0, 0)
}

/// The n-th trade of the day: 10:00:00 + 7 s · n, price walks, trade number 1000 + n.
fn nth(n: i64) -> Trade {
    let t = at(10, 0, 0) + 7_000 * n + 3;
    Trade {
        symbol: SYM.into(),
        // As REST text would give it (and as the live f32 widens to).
        price: qp_matriks::trade::widen_price((100.0 + ((n * 37) % 23) as f64 * 0.05) as f32),
        quantity: Some(1 + (n * 13) % 50),
        side: None,
        timestamp_ns: t * 1_000_000 + 123,
        order_no: Some(format!("{}", 1000 + n)),
        is_trade_ex: false,
    }
}

/// Wire message of a trade (default unit: ns since local midnight), with buyer / seller
/// set to prove they never surface.
fn wire(t: &Trade) -> Vec<u8> {
    TradeMessage {
        symbol: t.symbol.clone(),
        order_no: t.order_no.clone(),
        price: Some(t.price as f32),
        quantity: t.quantity,
        active_bid_or_ask: Some("A".into()),
        timestamp: Some(t.timestamp_ns - local_midnight() * 1_000_000),
        buyer: Some("BRKBUY".into()),
        seller: Some("BRKSELL".into()),
        is_trade_ex: Some(t.is_trade_ex),
    }
    .encode_to_vec()
}

fn msg(t: &Trade, arrival_ms: i64) -> RawRecord {
    let topic = if t.is_trade_ex {
        format!("mx/tradeex/{SYM}")
    } else {
        format!("mx/trade/{SYM}")
    };
    RawRecord::message(arrival_ms * 1000, topic, wire(t))
}

fn conn(kind: RecordKind, arrival_ms: i64) -> RawRecord {
    RawRecord {
        t_us: arrival_ms * 1000,
        kind,
        topic: "trade".into(),
        payload: Vec::new(),
    }
}

fn feed() -> LiveFeed {
    LiveFeed::with_trades(
        LiveFeedConfig {
            intervals: vec!["1m".into(), "5m".into()],
            ..LiveFeedConfig::default()
        },
        xist(),
        TradeFeedConfig::default(),
    )
}

/// Fake REST history over a full trade list, with a call log.
struct FakeHistory {
    all: Vec<Trade>,
    calls: RefCell<Vec<(String, i64, i64)>>,
}

impl TradeHistory for FakeHistory {
    fn trades(&self, symbol: &str, start_ms: i64, end_ms: i64) -> Result<Vec<Trade>, String> {
        self.calls
            .borrow_mut()
            .push((symbol.to_owned(), start_ms, end_ms));
        // Newest first, to prove the feed sorts.
        Ok(self
            .all
            .iter()
            .rev()
            .filter(|t| t.symbol == symbol && (start_ms..=end_ms).contains(&t.t_ms()))
            .cloned()
            .collect())
    }
}

fn key(iv: &str) -> FeedKey {
    FeedKey {
        symbol: SYM.into(),
        interval: iv.into(),
        feed: None,
    }
}

fn bars(evs: &[DataEvent], iv: &str) -> Vec<Bar> {
    let mut s = FeedStore::new(key(iv));
    for e in evs.iter().filter(|e| e.feed.interval == iv) {
        s.apply(e).expect("event accepted");
    }
    s.bars().to_vec()
}

/// Bars built directly from the full trade list (the reference).
fn reference(all: &[Trade], iv: &str) -> Vec<Bar> {
    let mut lb = LiveBars::new(
        key(iv),
        xist(),
        LiveConfig {
            method: BarMethod::Trades,
            ..LiveConfig::default()
        },
    )
    .unwrap();
    let mut evs = Vec::new();
    for t in all {
        evs.extend(lb.push_trade(&t.tick()));
    }
    bars(&evs, iv)
}

fn bit_equal(a: &[Bar], b: &[Bar]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(x, y)| {
            x.t == y.t
                && [x.o, x.h, x.l, x.c, x.v]
                    .iter()
                    .zip([y.o, y.h, y.l, y.c, y.v])
                    .all(|(p, q)| p.to_bits() == q.to_bits())
        })
}

fn collect(evs: &mut Vec<DataEvent>, out: FeedOut) {
    evs.extend(out.events);
}

#[test]
fn no_initial_state_day_so_far_comes_from_history() {
    let all: Vec<Trade> = (0..60).map(nth).collect();
    let hist = FakeHistory {
        all: all.clone(),
        calls: RefCell::default(),
    };
    let mut f = feed();
    f.watch_trades(SYM, at(9, 40, 0));
    let mut evs = Vec::new();
    // Before the trade connection: stale, nothing to repair yet.
    assert_eq!(f.trade_stale_symbols(), vec![SYM.to_owned()]);
    assert!(f.pending_trade_repairs().is_empty());
    let out = f.push_from(
        FeedSource::Trade,
        &conn(RecordKind::ConnectionOpened, at(10, 3, 30)),
    );
    assert_eq!(
        out.trade_connection,
        Some(ConnChange::Opened { session: 1 })
    );
    assert!(f.trade_connected());
    // First live trades arrive before the repair: buffered, no events, still stale.
    for n in 30..33 {
        let out = f.push(&msg(&all[n as usize], all[n as usize].t_ms() + 40));
        assert!(out.events.is_empty());
    }
    assert_eq!(f.trade_stale_symbols(), vec![SYM.to_owned()]);
    let gaps = f.pending_trade_repairs();
    assert_eq!(
        gaps,
        vec![TradeGap {
            symbol: SYM.into(),
            from_ms: at(9, 40, 0),
            to_ms: all[32].t_ms(),
        }]
    );
    let (out, errs) = f.repair_trades(&hist);
    assert!(errs.is_empty());
    collect(&mut evs, out);
    assert!(f.trade_stale_symbols().is_empty());
    assert!(f.pending_trade_repairs().is_empty());
    assert_eq!(hist.calls.borrow().len(), 1);
    // Live continues.
    for t in &all[33..] {
        collect(&mut evs, f.push(&msg(t, t.t_ms() + 40)));
    }
    for iv in ["1m", "5m"] {
        assert!(bit_equal(&bars(&evs, iv), &reference(&all, iv)), "{iv}");
    }
    let st = f.stats().trades;
    assert_eq!(st.buffered, 3);
    assert_eq!(
        st.duplicates, 3,
        "the buffered trades were in the history too"
    );
    assert_eq!(st.unit_ns_from_midnight, 30);
    assert_eq!(st.far_from_arrival, 0);
    // Buyer and seller are nowhere in the feed.
    let dump = format!("{f:?}{evs:?}");
    assert!(!dump.contains("BRKBUY") && !dump.contains("BRKSELL"));
}

#[test]
fn lost_trade_connection_is_stale_then_repaired_bit_equal() {
    let all: Vec<Trade> = (0..120).map(nth).collect();
    let hist = FakeHistory {
        all: all.clone(),
        calls: RefCell::default(),
    };
    let mut f = feed();
    f.watch_trades(SYM, local_midnight());
    let mut evs = Vec::new();
    f.push_from(
        FeedSource::Trade,
        &conn(RecordKind::ConnectionOpened, at(10, 0, 0)),
    );
    collect(&mut evs, f.repair_trades(&hist).0); // nothing yet in history before 10:00
    for t in &all[0..50] {
        collect(&mut evs, f.push(&msg(t, t.t_ms() + 40)));
    }
    let lost_at = all[49].t_ms() + 500;
    let out = f.push_from(
        FeedSource::Trade,
        &conn(RecordKind::ConnectionLost, lost_at),
    );
    assert_eq!(out.trade_connection, Some(ConnChange::Lost));
    assert_eq!(f.trade_stale_symbols(), vec![SYM.to_owned()]);
    assert!(f.pending_trade_repairs().is_empty(), "no repair while down");
    // 50..80 happen while disconnected (never delivered). The market clock keeps moving
    // far ahead; the trade bars are not finalized during the gap.
    let before = f.stats().finalized;
    collect(&mut evs, f.advance_clock(all[85].t_ms() + 600_000));
    assert_eq!(f.stats().finalized, before, "no finalization inside a gap");
    f.push_from(
        FeedSource::Trade,
        &conn(RecordKind::ConnectionOpened, all[80].t_ms()),
    );
    for t in &all[80..90] {
        collect(&mut evs, f.push(&msg(t, t.t_ms() + 40)));
    }
    let gaps = f.pending_trade_repairs();
    assert_eq!(gaps.len(), 1);
    assert_eq!(
        gaps[0].from_ms,
        all[49].t_ms(),
        "from: trades known complete"
    );
    let (out, errs) = f.repair_trades(&hist);
    assert!(errs.is_empty());
    assert!(
        !out.finalized.is_empty(),
        "held bars finalize after the repair"
    );
    collect(&mut evs, out);
    assert!(f.trade_stale_symbols().is_empty());
    for t in &all[90..] {
        collect(&mut evs, f.push(&msg(t, t.t_ms() + 40)));
    }
    for iv in ["1m", "5m"] {
        assert!(bit_equal(&bars(&evs, iv), &reference(&all, iv)), "{iv}");
    }
    let st = f.stats().trades;
    assert_eq!(st.connections_lost, 1);
    assert_eq!(st.connections_opened, 2);
    assert_eq!(st.buffered, 10);
    assert!(st.duplicates >= 10, "{st:?}");
    // Same records again → byte-equal events.
    let mut g = feed();
    g.watch_trades(SYM, local_midnight());
    let mut evs2 = Vec::new();
    g.push_from(
        FeedSource::Trade,
        &conn(RecordKind::ConnectionOpened, at(10, 0, 0)),
    );
    collect(&mut evs2, g.repair_trades(&hist).0);
    for t in &all[0..50] {
        collect(&mut evs2, g.push(&msg(t, t.t_ms() + 40)));
    }
    g.push_from(
        FeedSource::Trade,
        &conn(RecordKind::ConnectionLost, lost_at),
    );
    collect(&mut evs2, g.advance_clock(all[85].t_ms() + 600_000));
    g.push_from(
        FeedSource::Trade,
        &conn(RecordKind::ConnectionOpened, all[80].t_ms()),
    );
    for t in &all[80..90] {
        collect(&mut evs2, g.push(&msg(t, t.t_ms() + 40)));
    }
    collect(&mut evs2, g.repair_trades(&hist).0);
    for t in &all[90..] {
        collect(&mut evs2, g.push(&msg(t, t.t_ms() + 40)));
    }
    assert_eq!(format!("{evs:?}"), format!("{evs2:?}"));
}

#[test]
fn failed_history_keeps_the_gap_and_duplicates_are_dropped_live() {
    struct Down;
    impl TradeHistory for Down {
        fn trades(&self, _: &str, _: i64, _: i64) -> Result<Vec<Trade>, String> {
            Err("rest down".into())
        }
    }
    let mut f = feed();
    f.watch_trades(SYM, at(10, 0, 0));
    f.push_from(
        FeedSource::Trade,
        &conn(RecordKind::ConnectionOpened, at(10, 0, 0)),
    );
    let (_, errs) = f.repair_trades(&Down);
    assert_eq!(errs, vec![(SYM.to_owned(), "rest down".to_owned())]);
    assert_eq!(f.trade_stale_symbols(), vec![SYM.to_owned()]);
    assert_eq!(f.stats().trades.repair_errors, 1);
    let gap = f.pending_trade_repairs().remove(0);
    f.apply_trade_history(&gap, Vec::new());
    let t = nth(1);
    let a = f.push(&msg(&t, t.t_ms() + 10));
    let b = f.push(&msg(&t, t.t_ms() + 20));
    assert_eq!(a.events.len(), 2, "1m and 5m bars");
    assert!(b.events.is_empty());
    assert_eq!(f.stats().trades.duplicates, 1);
}

#[test]
fn price_card_gives_phases_but_no_bars_and_special_trades_are_optional() {
    let mut f = feed();
    f.watch_trades(SYM, at(10, 0, 0));
    f.push_from(
        FeedSource::Trade,
        &conn(RecordKind::ConnectionOpened, at(10, 0, 0)),
    );
    let gap = f.pending_trade_repairs().remove(0);
    f.apply_trade_history(&gap, Vec::new());
    let card = SymbolMessage {
        last: Some(500.0),
        daily_volume: Some(1e6),
        stock_status: Some(3),
        update_date: Some("2026-11-18T10:00:30.000+03".into()),
        ..SymbolMessage::default()
    };
    let out = f.push(&RawRecord::message(
        at(10, 0, 31) * 1000,
        format!("mx/symbol/{SYM}@lvl2"),
        card.encode_to_vec(),
    ));
    assert!(out.events.is_empty(), "the card builds no bar");
    assert_eq!(out.phases, vec![(SYM.to_owned(), 3)]);
    let special = Trade {
        is_trade_ex: true,
        ..nth(5)
    };
    let out = f.push(&msg(&special, special.t_ms() + 5));
    assert!(
        out.events.is_empty(),
        "special trades are excluded by default"
    );
    assert_eq!(f.stats().trades.special, 1);

    let mut g = LiveFeed::with_trades(
        LiveFeedConfig {
            intervals: vec!["1m".into()],
            ..LiveFeedConfig::default()
        },
        xist(),
        TradeFeedConfig {
            special: SpecialTrades::Include,
            ..TradeFeedConfig::default()
        },
    );
    g.watch_trades(SYM, at(10, 0, 0));
    g.push_from(
        FeedSource::Trade,
        &conn(RecordKind::ConnectionOpened, at(10, 0, 0)),
    );
    let gap = g.pending_trade_repairs().remove(0);
    g.apply_trade_history(&gap, Vec::new());
    let out = g.push(&msg(&special, special.t_ms() + 5));
    assert_eq!(out.events.len(), 1);
}

#[test]
fn unwatched_symbol_gap_starts_at_local_midnight_and_card_feed_ignores_trades() {
    let mut f = feed();
    f.push_from(
        FeedSource::Trade,
        &conn(RecordKind::ConnectionOpened, at(10, 0, 0)),
    );
    let t = nth(3);
    f.push(&msg(&t, t.t_ms() + 5));
    let gaps = f.pending_trade_repairs();
    assert_eq!(gaps[0].from_ms, local_midnight());
    // A card-only feed counts trade messages as other topics and builds nothing.
    let mut c = LiveFeed::new(LiveFeedConfig::default(), xist());
    let out = c.push(&msg(&t, t.t_ms() + 5));
    assert!(out.events.is_empty());
    assert_eq!(c.stats().other_messages, 1);
    assert!(!c.trades_enabled());
    assert!(c.trade_stale_symbols().is_empty());
}
