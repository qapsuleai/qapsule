//! Connection behaviour against the loopback mock broker and token service.
//! Every address is 127.0.0.1; nothing leaves the machine.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::cognitive_complexity
)]

use std::sync::Arc;
use std::time::Duration;

use qp_live::auth::{AuthClient, AuthPolicy};
use qp_live::backoff::BackoffPolicy;
use qp_live::config::{Endpoint, LiveConfig, MqttProtocol};
use qp_live::conn::{
    is_multiconnect_halted, reset_multiconnect_halt, ConnState, ConnectOptions, DisconnectReason,
    LiveConnection, LiveEvent, TokenRenewal,
};
use qp_live::error::{HaltReason, LiveError};
use qp_live::halt::{halt_log, halt_status, reset_halt, ManualClock, ResetError, ResetOutcome};
use qp_live::lock::LockError;
use qp_live::mock::{
    mock_config, test_classifier, BrokerScript, MockBroker, MockTokenService, RecordingSink,
    MULTICONNECT_TOPIC,
};
use qp_live::mqtt::SUBACK_FAILURE;

const WAIT: Duration = Duration::from_secs(10);

fn fast_auth() -> AuthPolicy {
    AuthPolicy {
        refresh_before: Duration::from_secs(1),
        min_interval: Duration::from_millis(100),
        max_interval: Duration::from_secs(2),
        max_failures: 3,
        default_lifetime: Duration::from_secs(3600),
        http_timeout: Duration::from_secs(2),
    }
}

fn fast_backoff() -> BackoffPolicy {
    BackoffPolicy {
        initial: Duration::from_millis(50),
        factor: 2,
        max: Duration::from_millis(400),
        max_failures: 4,
        stable_after: Duration::from_secs(30),
    }
}

struct Rig {
    tokens: MockTokenService,
    broker: MockBroker,
    cfg: LiveConfig,
    auth: Arc<AuthClient>,
    sink: Arc<RecordingSink>,
    dir: tempfile::TempDir,
}

impl Rig {
    fn new(path: &str, script: BrokerScript) -> Self {
        Self::with(path, script, 3600, fast_auth())
    }

    fn with(path: &str, script: BrokerScript, lifetime: i64, policy: AuthPolicy) -> Self {
        let tokens = MockTokenService::start_default(lifetime);
        let broker = MockBroker::start(script);
        let cfg = mock_config(&tokens, &broker.url(path)).unwrap();
        let auth = Arc::new(AuthClient::new(&cfg, policy));
        Self {
            tokens,
            broker,
            cfg,
            auth,
            sink: RecordingSink::new(),
            dir: tempfile::tempdir().unwrap(),
        }
    }

    fn endpoint(&self) -> &Endpoint {
        self.cfg.market_endpoint()
    }

    fn opts(&self, topics: &[&str]) -> ConnectOptions {
        let mut o = ConnectOptions::new(topics.iter().map(|t| t.to_string()).collect());
        o.lock_dir = self.dir.path().to_path_buf();
        o.backoff = fast_backoff();
        o.classifier = Some(test_classifier());
        o.poll_interval = Duration::from_millis(20);
        o.close_timeout = Duration::from_secs(1);
        o
    }

    fn open(&self, opts: ConnectOptions) -> Result<LiveConnection, LiveError> {
        LiveConnection::open(
            &self.cfg,
            self.endpoint(),
            self.auth.clone(),
            opts,
            self.sink.clone(),
        )
    }
}

fn connected(s: &ConnState) -> bool {
    matches!(s, ConnState::Connected { .. })
}

fn session(n: u32) -> impl Fn(&ConnState) -> bool {
    move |s| matches!(s, ConnState::Connected { session } if *session >= n)
}

fn halted(s: &ConnState) -> bool {
    matches!(s, ConnState::Halted(_))
}

#[test]
fn subscribes_one_by_one_and_delivers_messages() {
    let topics = ["mx/symbol/AAA@lvl2", "mx/symbol/BBB@lvl2", "mx/timestamp"];
    let mut script = BrokerScript::default();
    script
        .initial
        .insert(topics[0].to_string(), vec![b"init-a".to_vec()]);
    let rig = Rig::new("t-basic", script);
    let conn = rig.open(rig.opts(&topics)).unwrap();
    assert!(conn.wait_for(WAIT, connected).is_some());
    assert!(rig.sink.wait_for(
        WAIT,
        |e| matches!(e, LiveEvent::Subscribed { topic } if topic == "mx/timestamp")
    ));
    rig.broker.publish("mx/symbol/BBB@lvl2", b"update-b");
    assert!(rig.sink.wait_messages(WAIT, 2));

    let msgs = rig.sink.messages();
    assert_eq!(msgs[0].topic, topics[0]);
    assert_eq!(msgs[0].payload, b"init-a");
    assert_eq!(msgs[1].topic, topics[1]);
    assert_eq!((msgs[0].seq, msgs[1].seq), (1, 2));
    assert!(msgs.iter().all(|m| m.session == 1));
    let now_us = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_micros() as i64;
    assert!((now_us - msgs[1].received_at_us).abs() < 60_000_000);

    let st = rig.broker.stats();
    let c = &st.connects[0];
    assert_eq!(c.protocol_name, "MQIsdp");
    assert_eq!(c.protocol_level, 3);
    assert_eq!(c.ws_subprotocol.as_deref(), Some("mqttv3.1"));
    assert_eq!(c.username.as_deref(), Some("JWT"));
    assert_eq!(c.password.as_deref(), Some(rig.tokens.issued()[0].as_str()));
    assert!(c.clean_session);
    assert_eq!(c.keepalive, 30);
    assert!(c.client_id.starts_with("qp-") && c.client_id.len() <= 23);
    // One topic per SUBSCRIBE, in deterministic (sorted) order.
    let mut sorted = topics.to_vec();
    sorted.sort();
    let packets: Vec<Vec<String>> = sorted.iter().map(|t| vec![t.to_string()]).collect();
    assert_eq!(st.subscribe_packets, packets);
    assert_eq!(rig.tokens.requests(), 1);

    conn.close().unwrap();
    let st = rig.broker.stats();
    assert_eq!(st.disconnects, 1, "client must send MQTT DISCONNECT");
    assert!(rig.broker.wait_until(WAIT, |s| s.current == 0));
    assert!(rig.sink.count(|e| *e == LiveEvent::Closed) == 1);
}

#[test]
fn protocol_flavour_is_configurable() {
    let mut rig = Rig::new("t-v311", BrokerScript::default());
    rig.cfg.mqtt.protocol = MqttProtocol::V311;
    rig.cfg.mqtt.ws_subprotocol = "mqtt".to_string();
    rig.cfg.mqtt.keepalive_secs = 15;
    let conn = rig.open(rig.opts(&["mx/timestamp"])).unwrap();
    assert!(conn.wait_for(WAIT, connected).is_some());
    let c = rig.broker.stats().connects[0].clone();
    assert_eq!((c.protocol_name.as_str(), c.protocol_level), ("MQTT", 4));
    assert_eq!(c.ws_subprotocol.as_deref(), Some("mqtt"));
    assert_eq!(c.keepalive, 15);
    conn.close().unwrap();
}

#[test]
fn keepalive_sends_pingreq() {
    let mut rig = Rig::new("t-ping", BrokerScript::default());
    rig.cfg.mqtt.keepalive_secs = 1;
    let conn = rig.open(rig.opts(&["mx/timestamp"])).unwrap();
    assert!(conn.wait_for(WAIT, connected).is_some());
    assert!(rig.broker.wait_until(WAIT, |s| s.pings >= 2));
    assert_eq!(conn.state(), ConnState::Connected { session: 1 });
    conn.close().unwrap();
}

#[test]
fn suback_refusal_is_reported_per_topic_and_not_resubscribed() {
    let mut script = BrokerScript::default();
    script
        .suback_codes
        .insert("mx/symbol/BAD".into(), SUBACK_FAILURE);
    let rig = Rig::new("t-suback", script);
    let mut opts = rig.opts(&["mx/symbol/GOOD", "mx/symbol/BAD"]);
    opts.subscribe_batch = 2;
    let conn = rig.open(opts).unwrap();
    assert!(rig
        .sink
        .wait_for(WAIT, |e| matches!(e, LiveEvent::SubscribeRejected { .. })));
    let events = rig.sink.events();
    assert!(events.contains(&LiveEvent::SubscribeRejected {
        topic: "mx/symbol/BAD".into(),
        code: SUBACK_FAILURE
    }));
    assert!(events.contains(&LiveEvent::Subscribed {
        topic: "mx/symbol/GOOD".into()
    }));
    // Batch of two in one packet.
    assert_eq!(
        rig.broker.stats().subscribe_packets,
        vec![vec![
            "mx/symbol/BAD".to_string(),
            "mx/symbol/GOOD".to_string()
        ]]
    );
    rig.broker.drop_connections();
    assert!(conn.wait_for(WAIT, session(2)).is_some());
    assert!(rig
        .broker
        .wait_until(WAIT, |s| s.subscribe_packets.len() == 2));
    assert_eq!(
        rig.broker.stats().subscribe_packets[1],
        vec!["mx/symbol/GOOD".to_string()]
    );
    conn.close().unwrap();
}

#[test]
fn drop_backs_off_reconnects_resubscribes_without_new_token() {
    let topics = ["mx/symbol/AAA@lvl2", "mx/timestamp"];
    let rig = Rig::new("t-drop", BrokerScript::default());
    let conn = rig.open(rig.opts(&topics)).unwrap();
    assert!(conn.wait_for(WAIT, connected).is_some());
    assert!(rig
        .broker
        .wait_until(WAIT, |s| s.subscribe_packets.len() == 2));
    rig.broker.publish("mx/timestamp", b"t1");
    assert!(rig.sink.wait_messages(WAIT, 1));

    rig.broker.drop_connections();
    assert!(conn.wait_for(WAIT, session(2)).is_some());
    assert!(rig
        .broker
        .wait_until(WAIT, |s| s.subscribe_packets.len() == 4));
    rig.broker.publish("mx/timestamp", b"t2");
    assert!(rig.sink.wait_messages(WAIT, 2));

    let events = rig.sink.events();
    assert!(
        events.contains(&LiveEvent::Disconnected {
            reason: DisconnectReason::ServerClosed
        }),
        "{events:?}"
    );
    assert!(events.contains(&LiveEvent::BackingOff {
        failures: 1,
        delay: Duration::from_millis(50)
    }));
    // Same token on reconnect: no new token request.
    assert_eq!(rig.tokens.requests(), 1);
    let st = rig.broker.stats();
    assert_eq!(st.connects.len(), 2);
    assert_eq!(st.connects[0].password, st.connects[1].password);
    assert_eq!(st.max_concurrent, 1);
    let msgs = rig.sink.messages();
    assert_eq!((msgs[1].seq, msgs[1].session), (2, 2));
    conn.close().unwrap();
}

#[test]
fn multiconnect_notice_halts_without_retry_until_reset() {
    let rig = Rig::new("t-multi", BrokerScript::default());
    let conn = rig.open(rig.opts(&["mx/timestamp"])).unwrap();
    assert!(conn.wait_for(WAIT, connected).is_some());
    rig.broker.send_multiconnect();
    let st = conn.wait_for(WAIT, halted).unwrap();
    assert_eq!(st, ConnState::Halted(HaltReason::MultiConnect));
    // Far longer than the whole back-off schedule: still no new connection.
    std::thread::sleep(Duration::from_millis(1200));
    let stats = rig.broker.stats();
    assert_eq!(stats.connections_total, 1, "no retry after multi-connect");
    assert_eq!(stats.disconnects, 1, "closed with MQTT DISCONNECT");
    assert!(rig.broker.wait_until(WAIT, |s| s.current == 0));
    assert_eq!(rig.sink.count(|e| matches!(e, LiveEvent::Halted { .. })), 1);
    assert!(rig
        .sink
        .event_texts()
        .iter()
        .any(|t| t.contains("multi-connect")));

    let key = conn.key().clone();
    match conn.close() {
        Err(LiveError::Halted { reason, .. }) => assert_eq!(reason, HaltReason::MultiConnect),
        other => panic!("expected explicit halt error, got {other:?}"),
    }
    // Reopening is refused before any network activity, also via the marker file.
    assert!(is_multiconnect_halted(rig.dir.path(), &key));
    assert!(key.halt_marker(rig.dir.path()).exists());
    match rig.open(rig.opts(&["mx/timestamp"])) {
        Err(LiveError::Halted { reason, .. }) => assert_eq!(reason, HaltReason::MultiConnect),
        other => panic!("expected refusal, got {other:?}"),
    }
    assert_eq!(rig.broker.stats().connections_total, 1);

    // Only an explicit reset with a note re-enables the endpoint.
    assert!(!reset_multiconnect_halt(rig.dir.path(), &key, ""));
    assert!(reset_multiconnect_halt(
        rig.dir.path(),
        &key,
        "test: operator checked"
    ));
    let conn = rig.open(rig.opts(&["mx/timestamp"])).unwrap();
    assert!(conn.wait_for(WAIT, connected).is_some());
    conn.close().unwrap();
}

const MIN: Duration = Duration::from_secs(60);

fn a52() -> HaltReason {
    HaltReason::UnexplainedServerClose {
        closes: 2,
        window_secs: 600,
    }
}

/// A52 (K64): two unexplained server closes within 10 minutes (sliding; the injected clock
/// moves 5 minutes, nobody waits) halt for good: no retry, a marker file naming the reason
/// (not multi-connect), reopening refused before any network activity; only a reset with
/// a note opens it again.
#[test]
fn two_unexplained_server_closes_within_ten_minutes_halt_until_reset() {
    let rig = Rig::new("t-a52", BrokerScript::default());
    let clock = ManualClock::new();
    let mut o = rig.opts(&["mx/timestamp"]);
    o.clock = clock.clock();
    let conn = rig.open(o).unwrap();
    assert!(conn.wait_for(WAIT, connected).is_some());
    rig.broker.drop_connections();
    // The first one: back off and reconnect, as before.
    assert!(conn.wait_for(WAIT, session(2)).is_some());
    clock.advance(5 * MIN);
    rig.broker.drop_connections();
    let st = conn.wait_for(WAIT, halted).unwrap();
    assert_eq!(st, ConnState::Halted(a52()));
    std::thread::sleep(Duration::from_millis(1200));
    assert_eq!(
        rig.broker.stats().connections_total,
        2,
        "no retry after the halt"
    );
    assert!(rig.broker.wait_until(WAIT, |s| s.current == 0));
    let texts = rig.sink.event_texts();
    assert!(
        texts.iter().any(|t| t.contains("unexplained server close")),
        "{texts:?}"
    );
    assert!(
        !texts.iter().any(|t| t.contains("multi-connect")),
        "{texts:?}"
    );
    let key = conn.key().clone();
    match conn.close() {
        Err(LiveError::Halted { reason, .. }) => assert_eq!(reason, a52()),
        other => panic!("expected the halt, got {other:?}"),
    }
    // The marker says why (told apart from multi-connect) and is logged.
    let marker = std::fs::read_to_string(key.halt_marker(rig.dir.path())).unwrap();
    assert!(
        marker.contains("halted=unexplained-server-close"),
        "{marker}"
    );
    assert!(marker.contains("closes=2") && marker.contains("window_secs=600"));
    let log = std::fs::read_to_string(halt_log(rig.dir.path(), &key)).unwrap();
    assert!(log.contains("HALT unexplained-server-close"), "{log}");
    match rig.open(rig.opts(&["mx/timestamp"])) {
        Err(LiveError::Halted { reason, .. }) => assert_eq!(reason, a52()),
        other => panic!("expected refusal, got {other:?}"),
    }
    assert_eq!(rig.broker.stats().connections_total, 2);
    assert_eq!(rig.tokens.requests(), 1);

    assert_eq!(
        reset_halt(rig.dir.path(), &key, "   "),
        Err(ResetError::EmptyNote)
    );
    assert!(rig.open(rig.opts(&["mx/timestamp"])).is_err());
    assert_eq!(
        reset_halt(rig.dir.path(), &key, "test: a person checked"),
        Ok(ResetOutcome::Reset { previous: a52() })
    );
    assert_eq!(halt_status(rig.dir.path(), &key), None);
    let log = std::fs::read_to_string(halt_log(rig.dir.path(), &key)).unwrap();
    assert!(log.contains("note=\"test: a person checked\""), "{log}");
    let conn = rig.open(rig.opts(&["mx/timestamp"])).unwrap();
    assert!(conn.wait_for(WAIT, connected).is_some());
    conn.close().unwrap();
}

/// Two unexplained closes more than 10 minutes apart: no halt; back-off and reconnect.
#[test]
fn unexplained_server_closes_further_apart_do_not_halt() {
    let rig = Rig::new("t-a52-apart", BrokerScript::default());
    let clock = ManualClock::new();
    let mut o = rig.opts(&["mx/timestamp"]);
    o.clock = clock.clock();
    let conn = rig.open(o).unwrap();
    assert!(conn.wait_for(WAIT, connected).is_some());
    rig.broker.drop_connections();
    assert!(conn.wait_for(WAIT, session(2)).is_some());
    clock.advance(10 * MIN + Duration::from_secs(1));
    rig.broker.drop_connections();
    assert!(conn.wait_for(WAIT, session(3)).is_some());
    clock.advance(11 * MIN);
    rig.broker.drop_connections();
    assert!(conn.wait_for(WAIT, session(4)).is_some());
    assert!(connected(&conn.state()));
    assert_eq!(rig.sink.count(|e| matches!(e, LiveEvent::Halted { .. })), 0);
    assert!(!conn.key().halt_marker(rig.dir.path()).exists());
    assert_eq!(rig.broker.stats().max_concurrent, 1);
    conn.close().unwrap();
}

/// Local network / path failures do not count: a dead path (no PINGRESP: keepalive
/// timeout, the server closed nothing) and failed handshakes (connect failures), even
/// within the window and next to one real server close.
#[test]
fn local_network_errors_do_not_count_as_server_closes() {
    let mut rig = Rig::new("t-a52-local", BrokerScript::default());
    rig.cfg.mqtt.keepalive_secs = 1;
    let clock = ManualClock::new();
    let mut o = rig.opts(&["mx/timestamp"]);
    o.clock = clock.clock();
    o.backoff.max_failures = 10;
    let conn = rig.open(o).unwrap();
    assert!(conn.wait_for(WAIT, connected).is_some());
    // One real server close (1 of 2).
    rig.broker.drop_connections();
    assert!(conn.wait_for(WAIT, session(2)).is_some());
    // A dead path twice (keepalive timeouts), with two failed handshakes in between.
    let keepalive_timeouts = |sink: &RecordingSink| {
        sink.count(|e| {
            matches!(
                e,
                LiveEvent::Disconnected {
                    reason: DisconnectReason::KeepaliveTimeout
                }
            )
        })
    };
    rig.broker.script(|s| {
        s.ignore_pings = true;
        s.refuse_handshakes = 2;
    });
    let deadline = std::time::Instant::now() + WAIT;
    while keepalive_timeouts(&rig.sink) < 2 {
        assert!(
            std::time::Instant::now() < deadline,
            "two keepalive timeouts"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    rig.broker.script(|s| s.ignore_pings = false);
    assert!(conn.wait_for(WAIT, session(4)).is_some());
    clock.advance(MIN);
    assert_eq!(rig.sink.count(|e| matches!(e, LiveEvent::Halted { .. })), 0);
    assert!(rig.broker.stats().refused_handshakes >= 2);
    assert!(
        rig.sink.count(|e| matches!(
            e,
            LiveEvent::Disconnected {
                reason: DisconnectReason::ConnectFailed(_)
            }
        )) >= 2
    );
    assert!(connected(&conn.state()));
    assert!(!conn.key().halt_marker(rig.dir.path()).exists());
    // The second real server close within the window halts.
    rig.broker.drop_connections();
    assert_eq!(
        conn.wait_for(WAIT, halted).unwrap(),
        ConnState::Halted(a52())
    );
    let _ = conn.close();
}

#[test]
fn other_server_notice_is_reported_and_does_not_halt() {
    let rig = Rig::new("t-notice", BrokerScript::default());
    let conn = rig.open(rig.opts(&["mx/timestamp"])).unwrap();
    assert!(conn.wait_for(WAIT, connected).is_some());
    rig.broker
        .broadcast(MULTICONNECT_TOPIC, b"code=INFO;subject=MAINTENANCE");
    assert!(rig
        .sink
        .wait_for(WAIT, |e| matches!(e, LiveEvent::Notice { .. })));
    assert!(connected(&conn.state()));
    conn.close().unwrap();
}

#[test]
fn consecutive_failures_halt_with_deterministic_backoff() {
    let script = BrokerScript {
        refuse_handshakes: 100,
        ..Default::default()
    };
    let rig = Rig::new("t-fail", script);
    let conn = rig.open(rig.opts(&["mx/timestamp"])).unwrap();
    let st = conn.wait_for(WAIT, halted).unwrap();
    assert_eq!(
        st,
        ConnState::Halted(HaltReason::TooManyFailures { failures: 4 })
    );
    let delays: Vec<Duration> = rig
        .sink
        .events()
        .into_iter()
        .filter_map(|e| match e {
            LiveEvent::BackingOff { delay, .. } => Some(delay),
            _ => None,
        })
        .collect();
    assert_eq!(
        delays,
        vec![
            Duration::from_millis(50),
            Duration::from_millis(100),
            Duration::from_millis(200)
        ]
    );
    std::thread::sleep(Duration::from_millis(500));
    assert_eq!(rig.broker.stats().refused_handshakes, 4);
    // Reconnect attempts reuse the token.
    assert_eq!(rig.tokens.requests(), 1);
    assert!(matches!(conn.close(), Err(LiveError::Halted { .. })));
}

#[test]
fn connack_auth_refusal_refreshes_token_once() {
    let script = BrokerScript {
        connack_codes: [5u8, 0].into(),
        ..Default::default()
    };
    let rig = Rig::new("t-auth1", script);
    let conn = rig.open(rig.opts(&["mx/timestamp"])).unwrap();
    assert!(conn.wait_for(WAIT, connected).is_some());
    assert_eq!(rig.tokens.requests(), 2);
    assert!(rig
        .sink
        .events()
        .contains(&LiveEvent::TokenRefreshForced { code: 5 }));
    let st = rig.broker.stats();
    let issued = rig.tokens.issued();
    assert_eq!(st.connects[0].password.as_deref(), Some(issued[0].as_str()));
    assert_eq!(st.connects[1].password.as_deref(), Some(issued[1].as_str()));
    assert_eq!(st.max_concurrent, 1);
    conn.close().unwrap();
}

#[test]
fn connack_auth_refusal_twice_halts() {
    let script = BrokerScript {
        connack_codes: [4u8, 5].into(),
        ..Default::default()
    };
    let rig = Rig::new("t-auth2", script);
    let conn = rig.open(rig.opts(&["mx/timestamp"])).unwrap();
    let st = conn.wait_for(WAIT, halted).unwrap();
    assert_eq!(st, ConnState::Halted(HaltReason::AuthRejected { code: 5 }));
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(rig.tokens.requests(), 2);
    assert_eq!(rig.broker.stats().connections_total, 2);
    assert!(conn.close().is_err());
}

#[test]
fn connack_protocol_refusal_halts() {
    let script = BrokerScript {
        connack_codes: [1u8].into(),
        ..Default::default()
    };
    let rig = Rig::new("t-proto", script);
    let conn = rig.open(rig.opts(&["mx/timestamp"])).unwrap();
    let st = conn.wait_for(WAIT, halted).unwrap();
    assert_eq!(
        st,
        ConnState::Halted(HaltReason::ProtocolRejected { code: 1 })
    );
    assert_eq!(rig.broker.stats().connections_total, 1);
    drop(conn);
}

#[test]
fn token_renewal_closes_old_connection_before_opening_new() {
    let policy = AuthPolicy {
        refresh_before: Duration::from_secs(2),
        ..fast_auth()
    };
    let rig = Rig::with("t-renew", BrokerScript::default(), 3, policy);
    let mut opts = rig.opts(&["mx/symbol/AAA", "mx/timestamp"]);
    opts.renewal = TokenRenewal::ReconnectBeforeExpiry;
    let conn = rig.open(opts).unwrap();
    assert!(conn.wait_for(Duration::from_secs(15), session(3)).is_some());
    conn.close().unwrap();
    let st = rig.broker.stats();
    assert_eq!(st.max_concurrent, 1, "two connections overlapped");
    assert!(st.disconnects >= 3);
    assert!(rig.tokens.requests() >= 3);
    let passwords: std::collections::BTreeSet<_> =
        st.connects.iter().map(|c| c.password.clone()).collect();
    assert_eq!(
        passwords.len(),
        st.connects.len(),
        "each session a new token"
    );
    assert!(rig.sink.events().contains(&LiveEvent::Disconnected {
        reason: DisconnectReason::TokenRenewal
    }));
}

#[test]
fn default_renewal_keeps_open_connection() {
    let policy = AuthPolicy {
        refresh_before: Duration::from_secs(2),
        ..fast_auth()
    };
    let rig = Rig::with("t-keep", BrokerScript::default(), 3, policy);
    let conn = rig.open(rig.opts(&["mx/timestamp"])).unwrap();
    assert!(conn.wait_for(WAIT, connected).is_some());
    std::thread::sleep(Duration::from_millis(2500));
    assert_eq!(conn.state(), ConnState::Connected { session: 1 });
    assert_eq!(rig.tokens.requests(), 1);
    assert_eq!(rig.broker.stats().connections_total, 1);
    conn.close().unwrap();
}

#[test]
fn second_open_in_same_process_is_refused() {
    let rig = Rig::new("t-inproc", BrokerScript::default());
    let a = rig.open(rig.opts(&["mx/timestamp"])).unwrap();
    assert!(a.wait_for(WAIT, connected).is_some());
    match rig.open(rig.opts(&["mx/timestamp"])) {
        Err(LiveError::Lock(LockError::InProcess { key })) => {
            assert_eq!(key.as_str(), "delayed-t-inproc")
        }
        other => panic!("expected in-process lock error, got {other:?}"),
    }
    assert_eq!(rig.broker.stats().connections_total, 1);
    a.close().unwrap();
    let b = rig.open(rig.opts(&["mx/timestamp"])).unwrap();
    assert!(b.wait_for(WAIT, connected).is_some());
    b.close().unwrap();
    assert_eq!(rig.broker.stats().max_concurrent, 1);
}

#[test]
fn dynamic_subscribe_and_unsubscribe() {
    let rig = Rig::new("t-dyn", BrokerScript::default());
    let conn = rig.open(rig.opts(&["mx/timestamp"])).unwrap();
    assert!(conn.wait_for(WAIT, connected).is_some());
    conn.subscribe(vec!["mx/symbol/NEW".into()]);
    assert!(rig.sink.wait_for(
        WAIT,
        |e| matches!(e, LiveEvent::Subscribed { topic } if topic == "mx/symbol/NEW")
    ));
    conn.unsubscribe(vec!["mx/symbol/NEW".into()]);
    assert!(rig.broker.wait_until(WAIT, |s| s.unsubscribe_packets
        == vec![vec!["mx/symbol/NEW".to_string()]]));
    rig.broker.drop_connections();
    assert!(conn.wait_for(WAIT, session(2)).is_some());
    assert!(rig
        .broker
        .wait_until(WAIT, |s| s.subscribe_packets.len() == 3));
    assert_eq!(
        rig.broker.stats().subscribe_packets[2],
        vec!["mx/timestamp".to_string()]
    );
    conn.close().unwrap();
}
