//! Token request rate limiting and secret masking (R6 VERIFY 5). Loopback only.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::cognitive_complexity
)]

use std::net::TcpListener;
use std::sync::Arc;
use std::time::Duration;

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use qp_live::auth::{AuthClient, AuthPolicy};
use qp_live::backoff::BackoffPolicy;
use qp_live::conn::{ConnState, ConnectOptions, LiveConnection};
use qp_live::error::AuthError;
use qp_live::mock::{
    mock_config, mock_config_urls, test_classifier, BrokerScript, MockBroker, MockTokenService,
    RecordingSink, FAKE_HARDWARE_ID, FAKE_PASSWORD, FAKE_USERNAME,
};

const WAIT: Duration = Duration::from_secs(10);

fn policy(min_interval_ms: u64, max_failures: u32) -> AuthPolicy {
    AuthPolicy {
        refresh_before: Duration::from_secs(1),
        min_interval: Duration::from_millis(min_interval_ms),
        max_interval: Duration::from_secs(5),
        max_failures,
        default_lifetime: Duration::from_secs(3600),
        http_timeout: Duration::from_secs(2),
    }
}

/// A loopback port nobody listens on.
fn closed_port_url(scheme: &str, path: &str) -> String {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = l.local_addr().unwrap();
    drop(l);
    format!("{scheme}://{addr}/{path}")
}

#[test]
fn valid_token_is_reused_and_requests_are_rate_limited() {
    let tokens = MockTokenService::start_default(3600);
    let cfg = mock_config(&tokens, "ws://127.0.0.1:9/t-rate").unwrap();
    // refresh_before > lifetime: every call wants a new token, so only the limiter holds it.
    let mut p = policy(300, 5);
    p.refresh_before = Duration::from_secs(7200);
    let auth = AuthClient::new(&cfg, p);
    let a = auth.token().unwrap();
    let b = auth.token().unwrap();
    assert_eq!(
        tokens.requests(),
        1,
        "second call within min_interval must not request"
    );
    assert_eq!(
        format!("{:?}", a.expires_at()),
        format!("{:?}", b.expires_at())
    );
    std::thread::sleep(Duration::from_millis(350));
    auth.token().unwrap();
    assert_eq!(tokens.requests(), 2);

    // A sane policy keeps using the cached token without any request.
    let auth = AuthClient::new(&cfg, policy(0, 5));
    for _ in 0..20 {
        auth.token().unwrap();
    }
    assert_eq!(tokens.requests(), 3);
}

#[test]
fn failures_back_off_exponentially_and_stop() {
    let tokens = MockTokenService::start_default(3600);
    tokens.fail_next(100);
    let cfg = mock_config(&tokens, "ws://127.0.0.1:9/t-authfail").unwrap();
    let auth = AuthClient::new(&cfg, policy(40, 3));
    assert_eq!(auth.token().unwrap_err(), AuthError::Http { status: 500 });
    // Gap after one failure is 80 ms.
    match auth.token().unwrap_err() {
        AuthError::RateLimited { wait } => assert!(wait <= Duration::from_millis(80)),
        other => panic!("{other:?}"),
    }
    std::thread::sleep(Duration::from_millis(90));
    assert_eq!(auth.token().unwrap_err(), AuthError::Http { status: 500 });
    std::thread::sleep(Duration::from_millis(170));
    assert_eq!(auth.token().unwrap_err(), AuthError::Http { status: 500 });
    // Three consecutive failures: stopped, no more requests however long we wait.
    std::thread::sleep(Duration::from_millis(400));
    assert_eq!(auth.token().unwrap_err(), AuthError::Halted { failures: 3 });
    assert_eq!(tokens.requests(), 3);
    auth.reset_after_halt();
    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(auth.token().unwrap_err(), AuthError::Http { status: 500 });
    assert_eq!(tokens.requests(), 4);
}

#[test]
fn reconnect_storm_does_not_hammer_the_token_service() {
    // Broker refuses every handshake; the connection retries with back-off and halts.
    let tokens = MockTokenService::start_default(3600);
    let broker = MockBroker::start(BrokerScript {
        refuse_handshakes: 1000,
        ..Default::default()
    });
    let cfg = mock_config(&tokens, &broker.url("t-storm")).unwrap();
    let auth = Arc::new(AuthClient::new(&cfg, policy(100, 3)));
    let dir = tempfile::tempdir().unwrap();
    let mut o = ConnectOptions::new(vec!["mx/timestamp".into()]);
    o.lock_dir = dir.path().into();
    o.backoff = BackoffPolicy {
        initial: Duration::from_millis(10),
        factor: 1,
        max: Duration::from_millis(10),
        max_failures: 30,
        stable_after: Duration::from_secs(30),
    };
    let conn =
        LiveConnection::open(&cfg, cfg.market_endpoint(), auth, o, RecordingSink::new()).unwrap();
    assert!(conn
        .wait_for(WAIT, |s| matches!(s, ConnState::Halted(_)))
        .is_some());
    assert_eq!(broker.stats().refused_handshakes, 30);
    assert_eq!(tokens.requests(), 1, "reconnects must not request tokens");
    let _ = conn.close();
}

/// Every secret value the tests below must never see in any text.
fn secrets(tokens: &[String], urls: &[String]) -> Vec<String> {
    let mut s = vec![
        FAKE_USERNAME.to_string(),
        FAKE_PASSWORD.to_string(),
        FAKE_HARDWARE_ID.to_string(),
        STANDARD.encode(format!("{FAKE_USERNAME}:{FAKE_PASSWORD}")),
        "eyJ".to_string(),
        "127.0.0.1".to_string(),
    ];
    s.extend(tokens.iter().cloned());
    for u in urls {
        s.push(u.clone());
        let rest = u.split_once("://").map(|(_, r)| r).unwrap_or(u);
        s.push(rest.split('/').next().unwrap().to_string()); // host:port
    }
    s
}

fn assert_clean(texts: &[String], secrets: &[String]) {
    assert!(!texts.is_empty());
    for t in texts {
        let lower = t.to_ascii_lowercase();
        for s in secrets {
            assert!(
                !lower.contains(&s.to_ascii_lowercase()),
                "secret leaked into text: {t}"
            );
        }
    }
}

#[test]
fn no_secret_reaches_any_log_or_error_text() {
    let mut texts: Vec<String> = Vec::new();
    let mut urls: Vec<String> = Vec::new();
    let mut issued: Vec<String> = Vec::new();
    let dir = tempfile::tempdir().unwrap();
    let opts = |path: &str| {
        let mut o = ConnectOptions::new(vec!["mx/timestamp".into(), "mx/symbol/AAA".into()]);
        o.lock_dir = dir.path().join(path);
        o.classifier = Some(test_classifier());
        o.poll_interval = Duration::from_millis(20);
        o.backoff = BackoffPolicy {
            initial: Duration::from_millis(20),
            factor: 2,
            max: Duration::from_millis(80),
            max_failures: 3,
            stable_after: Duration::from_secs(30),
        };
        o
    };

    // 1. Token service rejects the credentials (its 401 body echoes the Basic header).
    {
        let tokens = MockTokenService::start("someone-else", "other-pass", 3600);
        let cfg = mock_config(&tokens, "ws://127.0.0.1:9/t-red1").unwrap();
        urls.push(tokens.url());
        texts.push(format!("{cfg:?}"));
        texts.push(format!("{:?}", cfg.market_endpoint()));
        let auth = AuthClient::new(&cfg, policy(0, 5));
        texts.push(format!("{auth:?}"));
        let e = auth.token().unwrap_err();
        assert_eq!(e, AuthError::Http { status: 401 });
        texts.push(e.to_string());
        texts.push(format!("{e:?}"));
    }
    // 2. Token service says success:false with a message echoing the credentials.
    {
        let tokens = MockTokenService::start_default(3600);
        tokens.refuse_next(1);
        let cfg = mock_config(&tokens, "ws://127.0.0.1:9/t-red2").unwrap();
        urls.push(tokens.url());
        let auth = AuthClient::new(&cfg, policy(0, 5));
        let e = auth.token().unwrap_err();
        assert!(matches!(e, AuthError::Refused { .. }));
        texts.push(e.to_string());
        texts.push(format!("{e:?}"));
        let ok = auth.token().unwrap();
        texts.push(format!("{ok:?}"));
        issued.extend(tokens.issued());
    }
    // 3. Token service unreachable.
    {
        let token_url = closed_port_url("http", "issue");
        urls.push(token_url.clone());
        let cfg = mock_config_urls(&token_url, "ws://127.0.0.1:9/t-red3").unwrap();
        let auth = AuthClient::new(&cfg, policy(0, 5));
        let e = auth.token().unwrap_err();
        assert!(matches!(e, AuthError::Transport(_)), "{e:?}");
        texts.push(e.to_string());
        texts.push(format!("{e:?}"));
        let conn = LiveConnection::open(
            &cfg,
            cfg.market_endpoint(),
            Arc::new(auth),
            opts("a"),
            RecordingSink::new(),
        )
        .unwrap();
        conn.wait_for(WAIT, |s| matches!(s, ConnState::Halted(_)));
        if let Err(e) = conn.close() {
            texts.push(e.to_string());
            texts.push(format!("{e:?}"));
        }
    }
    // 4. Broker unreachable, then refused credentials, then multi-connect; collect events.
    {
        let tokens = MockTokenService::start_default(3600);
        urls.push(tokens.url());
        let broker_url = closed_port_url("ws", "t-red4");
        urls.push(broker_url.clone());
        let cfg = mock_config(&tokens, &broker_url).unwrap();
        let sink = RecordingSink::new();
        let auth = Arc::new(AuthClient::new(&cfg, policy(0, 5)));
        let conn = LiveConnection::open(&cfg, cfg.market_endpoint(), auth, opts("b"), sink.clone())
            .unwrap();
        assert!(conn
            .wait_for(WAIT, |s| matches!(s, ConnState::Halted(_)))
            .is_some());
        let e = conn.close().unwrap_err();
        texts.push(e.to_string());
        texts.push(format!("{e:?}"));
        texts.extend(sink.event_texts());
        issued.extend(tokens.issued());
    }
    {
        let tokens = MockTokenService::start_default(3600);
        urls.push(tokens.url());
        let broker = MockBroker::start(BrokerScript {
            connack_codes: [5u8, 5].into(),
            ..Default::default()
        });
        urls.push(broker.url("t-red5"));
        let cfg = mock_config(&tokens, &broker.url("t-red5")).unwrap();
        let sink = RecordingSink::new();
        let auth = Arc::new(AuthClient::new(&cfg, policy(0, 5)));
        let conn = LiveConnection::open(&cfg, cfg.market_endpoint(), auth, opts("c"), sink.clone())
            .unwrap();
        assert!(conn
            .wait_for(WAIT, |s| matches!(s, ConnState::Halted(_)))
            .is_some());
        texts.push(conn.close().unwrap_err().to_string());
        texts.extend(sink.event_texts());
        issued.extend(tokens.issued());
    }
    {
        let tokens = MockTokenService::start_default(3600);
        urls.push(tokens.url());
        let broker = MockBroker::start(BrokerScript::default());
        urls.push(broker.url("t-red6"));
        let cfg = mock_config(&tokens, &broker.url("t-red6")).unwrap();
        let sink = RecordingSink::new();
        let auth = Arc::new(AuthClient::new(&cfg, policy(0, 5)));
        let conn = LiveConnection::open(
            &cfg,
            cfg.market_endpoint(),
            auth.clone(),
            opts("d"),
            sink.clone(),
        )
        .unwrap();
        assert!(conn
            .wait_for(WAIT, |s| matches!(s, ConnState::Connected { .. }))
            .is_some());
        texts.push(format!("{conn:?}"));
        // Second open: lock error text.
        let e = LiveConnection::open(
            &cfg,
            cfg.market_endpoint(),
            auth,
            opts("d"),
            RecordingSink::new(),
        )
        .unwrap_err();
        texts.push(e.to_string());
        // A server notice whose text echoes the token is masked too.
        let token = tokens.issued()[0].clone();
        broker.broadcast(
            qp_live::mock::MULTICONNECT_TOPIC,
            format!("notice for {token} at {}", broker.url("t-red6")).as_bytes(),
        );
        assert!(sink.wait_for(WAIT, |e| matches!(e, qp_live::LiveEvent::Notice { .. })));
        broker.send_multiconnect();
        assert!(conn
            .wait_for(WAIT, |s| matches!(s, ConnState::Halted(_)))
            .is_some());
        texts.push(conn.close().unwrap_err().to_string());
        texts.extend(sink.event_texts());
        issued.extend(tokens.issued());
    }

    texts.retain(|t| !t.is_empty());
    assert!(issued.len() >= 3);
    assert_clean(&texts, &secrets(&issued, &urls));
}
