//! (RP4 T3) The trade endpoint (`/trade`) beside the market endpoint, against the loopback
//! mock broker: own lock, one connection per endpoint, both open at once on one identity,
//! level-2 trade topics refused locally. Every address is 127.0.0.1.

use std::sync::Arc;
use std::time::Duration;

use qp_live::auth::{AuthClient, AuthPolicy};
use qp_live::backoff::BackoffPolicy;
use qp_live::conn::{ConnState, ConnectOptions, LiveConnection, LiveEvent};
use qp_live::error::LiveError;
use qp_live::lock::LockError;
use qp_live::mock::{mock_config, BrokerScript, MockBroker, MockTokenService, RecordingSink};
use qp_live::mqtt::SUBACK_FAILURE;

const WAIT: Duration = Duration::from_secs(10);

fn opts(dir: &std::path::Path, topics: &[&str]) -> ConnectOptions {
    let mut o = ConnectOptions::new(topics.iter().map(|t| t.to_string()).collect());
    o.lock_dir = dir.to_path_buf();
    o.backoff = BackoffPolicy {
        initial: Duration::from_millis(50),
        factor: 2,
        max: Duration::from_millis(400),
        max_failures: 4,
        stable_after: Duration::from_secs(30),
    };
    o.poll_interval = Duration::from_millis(20);
    o.close_timeout = Duration::from_secs(1);
    o
}

fn connected(s: &ConnState) -> bool {
    matches!(s, ConnState::Connected { .. })
}

#[test]
fn trade_and_market_endpoints_run_together_with_own_locks() {
    let tokens = MockTokenService::start_default(3600);
    let broker = MockBroker::start(BrokerScript::default());
    // Unique market path for this test binary; the trade path is always `/trade`.
    let cfg = mock_config(&tokens, &broker.url("t3-market")).unwrap();
    let auth = Arc::new(AuthClient::new(
        &cfg,
        AuthPolicy {
            min_interval: Duration::from_millis(100),
            ..AuthPolicy::default()
        },
    ));
    let dir = tempfile::tempdir().unwrap();
    let market_sink = RecordingSink::new();
    let trade_sink = RecordingSink::new();

    let market = LiveConnection::open(
        &cfg,
        cfg.market_endpoint(),
        auth.clone(),
        opts(dir.path(), &["mx/symbol/TEST1@lvl2"]),
        market_sink.clone(),
    )
    .unwrap();
    let trade = LiveConnection::open(
        &cfg,
        cfg.trade_endpoint(),
        auth.clone(),
        opts(
            dir.path(),
            &["mx/trade/TEST1", "mx/trade/TEST2@lvl2", "mx/tradeex/TEST1"],
        ),
        trade_sink.clone(),
    )
    .unwrap();
    assert!(market.wait_for(WAIT, connected).is_some());
    assert!(trade.wait_for(WAIT, connected).is_some());

    // Own keys and own lock files.
    assert_eq!(trade.key().as_str(), "delayed-trade");
    assert_ne!(trade.key(), market.key());
    assert!(trade.lock_file().ends_with("live-delayed-trade.lock"));
    assert_ne!(trade.lock_file(), market.lock_file());

    // A second trade connection in the same process is refused before any network use;
    // the market endpoint's lock does not matter to it and vice versa.
    match LiveConnection::open(
        &cfg,
        cfg.trade_endpoint(),
        auth.clone(),
        opts(dir.path(), &["mx/trade/TEST1"]),
        RecordingSink::new(),
    ) {
        Err(LiveError::Lock(LockError::InProcess { key })) => {
            assert_eq!(key.as_str(), "delayed-trade")
        }
        other => panic!("expected an in-process lock error, got {other:?}"),
    }

    // Level 2 refused locally; the other topics subscribed.
    assert!(trade_sink.wait_for(WAIT, |e| matches!(
        e,
        LiveEvent::SubscribeRejected { topic, code }
            if topic == "mx/trade/TEST2@lvl2" && *code == SUBACK_FAILURE
    )));
    assert!(trade_sink.wait_for(
        WAIT,
        |e| matches!(e, LiveEvent::Subscribed { topic } if topic == "mx/tradeex/TEST1")
    ));
    // A later subscription request for level 2 is refused too.
    trade.subscribe(vec![
        "mx/tradeex/TEST3@lvl2".into(),
        "mx/trade/TEST3".into(),
    ]);
    assert!(trade_sink.wait_for(
        WAIT,
        |e| matches!(e, LiveEvent::Subscribed { topic } if topic == "mx/trade/TEST3")
    ));
    assert!(trade_sink.wait_for(WAIT, |e| matches!(
        e,
        LiveEvent::SubscribeRejected { topic, .. } if topic == "mx/tradeex/TEST3@lvl2"
    )));

    broker.publish("mx/trade/TEST1", b"t1");
    broker.publish("mx/symbol/TEST1@lvl2", b"c1");
    assert!(trade_sink.wait_messages(WAIT, 1));
    assert!(market_sink.wait_messages(WAIT, 1));
    assert_eq!(trade_sink.messages()[0].topic, "mx/trade/TEST1");
    assert_eq!(market_sink.messages()[0].topic, "mx/symbol/TEST1@lvl2");

    // Server drops both; each reconnects on its own, never two sockets per endpoint.
    broker.drop_connections();
    assert!(trade
        .wait_for(
            WAIT,
            |s| matches!(s, ConnState::Connected { session } if *session >= 2)
        )
        .is_some());
    assert!(market
        .wait_for(
            WAIT,
            |s| matches!(s, ConnState::Connected { session } if *session >= 2)
        )
        .is_some());

    trade.close().unwrap();
    market.close().unwrap();
    let st = broker.stats();
    let t = st.path("trade");
    let m = st.path("t3-market");
    assert_eq!(t.max_concurrent, 1, "trade: {t:?}");
    assert_eq!(m.max_concurrent, 1, "market: {m:?}");
    assert_eq!(st.max_concurrent, 2, "both endpoints at once");
    assert!(t.connections_total >= 2 && m.connections_total >= 2);
    assert!(
        st.by_path
            .values()
            .flat_map(|p| &p.subscribed_topics)
            .all(|t| !t.contains("@lvl2") || t.starts_with("mx/symbol/")),
        "{st:?}"
    );
    assert!(!t.subscribed_topics.iter().any(|x| x.contains("@lvl2")));
    // One identity: both connections used tokens from the same client.
    assert!(tokens.requests() >= 1);
}
