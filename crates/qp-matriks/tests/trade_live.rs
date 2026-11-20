//! (RP4 T3) The trade connector end to end against the loopback mock broker (`qp-live`
//! `mock` feature): the market and the trade endpoint open at once on one identity, each
//! with one connection; no initial state on the trade stream; trade messages → live bars;
//! a server drop → stale → repair from a fake REST history → live again, bars bit-equal to
//! the bars built from the full trade list; `@lvl2` never subscribed. Every address is
//! 127.0.0.1; no real data.

use std::collections::BTreeMap;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use prost::Message as _;
use qp_calendar::{builtin, parse_date, ExchangeCalendar};
use qp_live::auth::{AuthClient, AuthPolicy};
use qp_live::backoff::BackoffPolicy;
use qp_live::conn::{ConnState, ConnectOptions, LiveConnection, LiveEvent, LiveMessage, LiveSink};
use qp_live::lock::EndpointKey;
use qp_live::mock::{mock_config, BrokerScript, MockBroker, MockTokenService};
use qp_matriks::messages::TradeMessage;
use qp_matriks::trade::widen_price;
use qp_matriks::{
    trade_topic_at, trade_topics, FeedSource, LiveFeed, LiveFeedConfig, RawRecord, RecordKind,
    TopicError, Trade, TradeDecodeConfig, TradeFeedConfig, TradeHistory, TradeStream,
    TradeTimeUnit,
};
use qp_schema::data::{DataEvent, FeedKey};
use qp_schema::Bar;
use qp_source::{BarMethod, FeedStore, LiveBars, LiveConfig};

const WAIT: Duration = Duration::from_secs(10);
const DAY: &str = "2026-11-18";
const SYM: &str = "TEST1";

fn xist() -> ExchangeCalendar {
    ExchangeCalendar::new(&builtin("XIST").expect("XIST")).expect("calendar")
}

fn at(hh: i64, mm: i64, ss: i64) -> i64 {
    parse_date(DAY).expect("date") * 86_400_000 + ((hh * 60 + mm) * 60 + ss) * 1000 - 3 * 3_600_000
}

fn nth(n: i64) -> Trade {
    let t = at(11, 0, 0) + 9_000 * n;
    Trade {
        symbol: SYM.into(),
        price: widen_price((50.0 + ((n * 31) % 17) as f64 * 0.02) as f32),
        quantity: Some(10 + (n * 7) % 90),
        side: None,
        timestamp_ns: t * 1_000_000,
        order_no: Some(format!("{}", 5000 + n)),
        is_trade_ex: false,
    }
}

/// The test publishes epoch-ms timestamps (the arrival is the wall clock, so a
/// since-midnight value would land on today's date); the unit is a setting.
fn wire(t: &Trade) -> Vec<u8> {
    TradeMessage {
        symbol: t.symbol.clone(),
        order_no: t.order_no.clone(),
        price: Some(t.price as f32),
        quantity: t.quantity,
        active_bid_or_ask: Some("B".into()),
        timestamp: Some(t.t_ms()),
        buyer: None,
        seller: None,
        is_trade_ex: Some(false),
    }
    .encode_to_vec()
}

/// Turns connection callbacks into raw records on one channel (both endpoints).
struct ChanSink {
    source: FeedSource,
    tx: Mutex<Sender<(FeedSource, RawRecord)>>,
}

impl LiveSink for ChanSink {
    fn on_message(&self, m: &LiveMessage<'_>) {
        let rec = RawRecord::message(m.received_at_us, m.topic, m.payload.to_vec());
        let _ = self.tx.lock().unwrap().send((self.source, rec));
    }

    fn on_event(&self, _endpoint: &EndpointKey, e: &LiveEvent) {
        let (kind, topic) = match e {
            LiveEvent::Connected { .. } => (RecordKind::ConnectionOpened, String::new()),
            LiveEvent::Disconnected { .. } => (RecordKind::ConnectionLost, String::new()),
            LiveEvent::Subscribed { topic } => (RecordKind::Subscribed, topic.clone()),
            LiveEvent::SubscribeRejected { topic, .. } => {
                (RecordKind::SubscribeRejected, topic.clone())
            }
            _ => return,
        };
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros() as i64;
        let rec = RawRecord {
            t_us: now,
            kind,
            topic,
            payload: Vec::new(),
        };
        let _ = self.tx.lock().unwrap().send((self.source, rec));
    }
}

struct FakeRest {
    all: Vec<Trade>,
    calls: Mutex<Vec<(i64, i64)>>,
}

impl TradeHistory for FakeRest {
    fn trades(&self, symbol: &str, start_ms: i64, end_ms: i64) -> Result<Vec<Trade>, String> {
        self.calls.lock().unwrap().push((start_ms, end_ms));
        Ok(self
            .all
            .iter()
            .filter(|t| t.symbol == symbol && (start_ms..=end_ms).contains(&t.t_ms()))
            .cloned()
            .collect())
    }
}

struct Pump {
    rx: Receiver<(FeedSource, RawRecord)>,
    feed: LiveFeed,
    events: Vec<DataEvent>,
    trade_msgs: usize,
    kinds: BTreeMap<(bool, u8), usize>,
}

impl Pump {
    /// Feeds records until `pred` holds (or panics after `WAIT`).
    fn until(&mut self, what: &str, pred: impl Fn(&Pump) -> bool) {
        let deadline = Instant::now() + WAIT;
        while !pred(self) {
            let left = deadline.saturating_duration_since(Instant::now());
            assert!(!left.is_zero(), "timed out waiting for {what}");
            if let Ok((src, rec)) = self.rx.recv_timeout(left.min(Duration::from_millis(50))) {
                if rec.kind == RecordKind::Message && rec.topic.starts_with("mx/trade") {
                    self.trade_msgs += 1;
                }
                *self
                    .kinds
                    .entry((src == FeedSource::Trade, rec.kind.code()))
                    .or_default() += 1;
                let out = self.feed.push_from(src, &rec);
                self.events.extend(out.events);
            }
        }
    }

    fn count(&self, trade: bool, kind: RecordKind) -> usize {
        self.kinds.get(&(trade, kind.code())).copied().unwrap_or(0)
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

#[test]
fn trade_connector_against_the_mock_broker() {
    // Level 2 cannot even be built.
    assert_eq!(
        trade_topic_at(TradeStream::Regular, SYM, Some("lvl2")),
        Err(TopicError::Level2Refused)
    );

    let all: Vec<Trade> = (0..100).map(nth).collect();
    let rest = FakeRest {
        all: all.clone(),
        calls: Mutex::default(),
    };
    let tokens = MockTokenService::start_default(3600);
    let broker = MockBroker::start(BrokerScript::default());
    let cfg = mock_config(&tokens, &broker.url("market")).unwrap();
    let auth = Arc::new(AuthClient::new(
        &cfg,
        AuthPolicy {
            min_interval: Duration::from_millis(100),
            ..AuthPolicy::default()
        },
    ));
    let dir = tempfile::tempdir().unwrap();
    let opts = |topics: Vec<String>| {
        let mut o = ConnectOptions::new(topics);
        o.lock_dir = dir.path().to_path_buf();
        o.backoff = BackoffPolicy {
            initial: Duration::from_millis(50),
            factor: 2,
            max: Duration::from_millis(400),
            max_failures: 4,
            stable_after: Duration::from_secs(30),
        };
        o.poll_interval = Duration::from_millis(20);
        o
    };
    let (tx, rx) = mpsc::channel();
    let sink = |source| {
        Arc::new(ChanSink {
            source,
            tx: Mutex::new(tx.clone()),
        })
    };
    let market = LiveConnection::open(
        &cfg,
        cfg.market_endpoint(),
        auth.clone(),
        opts(vec![format!("mx/symbol/{SYM}@lvl2")]),
        sink(FeedSource::Market),
    )
    .unwrap();
    let trade = LiveConnection::open(
        &cfg,
        cfg.trade_endpoint(),
        auth.clone(),
        opts(trade_topics(SYM, false).unwrap()),
        sink(FeedSource::Trade),
    )
    .unwrap();
    let mut p = Pump {
        rx,
        feed: LiveFeed::with_trades(
            LiveFeedConfig {
                intervals: vec!["1m".into(), "5m".into()],
                ..LiveFeedConfig::default()
            },
            xist(),
            TradeFeedConfig {
                decode: TradeDecodeConfig {
                    unit: TradeTimeUnit::EpochMillis,
                    ..TradeDecodeConfig::default()
                },
                ..TradeFeedConfig::default()
            },
        ),
        events: Vec::new(),
        trade_msgs: 0,
        kinds: BTreeMap::new(),
    };
    p.feed.watch_trades(SYM, at(10, 0, 0));
    p.until("both connected and subscribed", |p| {
        p.count(true, RecordKind::Subscribed) == 1 && p.count(false, RecordKind::Subscribed) == 1
    });
    assert!(p.feed.trade_connected());

    // No initial state: nothing arrives on the trade stream by itself.
    let quiet = Instant::now() + Duration::from_millis(300);
    p.until("a quiet moment", |_| Instant::now() >= quiet);
    assert_eq!(p.trade_msgs, 0, "the trade stream has no initial state");
    assert_eq!(p.feed.trade_stale_symbols(), vec![SYM.to_owned()]);

    // Day so far from history (trades 0..20), then live 20..50. The host gives the
    // exchange time first (here: just after trade 19), so the gap ends at "now".
    p.events
        .extend(p.feed.advance_clock(all[19].t_ms() + 1_000).events);
    let (out, errs) = p.feed.repair_trades(&rest);
    assert!(errs.is_empty());
    p.events.extend(out.events);
    assert!(p.feed.trade_stale_symbols().is_empty());
    for t in &all[20..50] {
        broker.publish(&format!("mx/trade/{SYM}"), &wire(t));
    }
    p.until("30 live trades", |p| p.trade_msgs == 30);
    assert!(!p.events.is_empty(), "live trades build bars");
    assert!(p.feed.trade_stale_symbols().is_empty());

    // Server drop: both endpoints lost; trades 50..70 happen meanwhile (not delivered).
    broker.drop_connections();
    p.until("trade connection lost", |p| {
        p.count(true, RecordKind::ConnectionLost) >= 1
    });
    // Stale from the loss until the repair.
    assert_eq!(p.feed.trade_stale_symbols(), vec![SYM.to_owned()]);
    p.until("both reconnected", |p| {
        p.count(true, RecordKind::Subscribed) >= 2 && p.count(false, RecordKind::Subscribed) >= 2
    });
    assert!(trade
        .wait_for(WAIT, |s| matches!(s, ConnState::Connected { .. }))
        .is_some());
    // Live again before the repair: buffered, still stale.
    for t in &all[70..80] {
        broker.publish(&format!("mx/trade/{SYM}"), &wire(t));
    }
    p.until("10 more live trades", |p| p.trade_msgs == 40);
    assert_eq!(p.feed.trade_stale_symbols(), vec![SYM.to_owned()]);
    let gap = p.feed.pending_trade_repairs().remove(0);
    assert_eq!(gap.from_ms, all[49].t_ms());
    let (out, errs) = p.feed.repair_trades(&rest);
    assert!(errs.is_empty());
    p.events.extend(out.events);
    assert!(p.feed.trade_stale_symbols().is_empty());
    for t in &all[80..] {
        broker.publish(&format!("mx/trade/{SYM}"), &wire(t));
    }
    p.until("all live trades", |p| p.trade_msgs == 60);

    for iv in ["1m", "5m"] {
        let got = bars(&p.events, iv);
        assert!(!got.is_empty());
        assert!(bit_equal(&got, &reference(&all, iv)), "{iv}: {got:?}");
    }
    assert_eq!(rest.calls.lock().unwrap().len(), 2);
    let st = p.feed.stats().trades;
    assert_eq!(st.unit_epoch_ms, 60);
    assert_eq!(st.buffered, 10);

    trade.close().unwrap();
    market.close().unwrap();
    let stats = broker.stats();
    for path in ["market", "trade"] {
        let ps = stats.path(path);
        assert_eq!(ps.max_concurrent, 1, "{path}: one connection per endpoint");
        assert!(ps.connections_total >= 2, "{path}: reconnected");
    }
    assert_eq!(stats.max_concurrent, 2, "both endpoints open at once");
    // `@lvl2` is never subscribed on the trade endpoint; the market only has the card.
    let trade_topics_seen = stats.path("trade").subscribed_topics;
    assert!(!trade_topics_seen.is_empty());
    assert!(
        trade_topics_seen.iter().all(|t| !t.contains("@lvl2")),
        "{trade_topics_seen:?}"
    );
    assert!(stats
        .subscribe_packets
        .iter()
        .flatten()
        .filter(|t| t.contains("@lvl2"))
        .all(|t| t.starts_with("mx/symbol/")));
}
