//! RP4 L-prep: `qp live record --trades` against the loopback mock broker (no network):
//! one connection to the trade endpoint only, level-1 trade topics (`@lvl2` refused before
//! anything connects), a `-trade.qpraw` raw log without buyer / seller bytes, and `qp live
//! inspect` printing only aggregate trade statistics.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::cognitive_complexity
)]

use std::process::{Command, Stdio};
use std::time::Duration;

use prost::Message as _;
use qp_live::mock::{BrokerScript, MockBroker, MockTokenService};

fn qp(root: &std::path::Path, tokens: &MockTokenService, broker: &MockBroker) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_qp"));
    c.current_dir(root)
        .env_clear()
        .env("PATH", std::env::var("PATH").unwrap_or_default())
        .env("MATRIKS_TOKEN_SERVICE_URL", tokens.url())
        .env("MATRIKS_API_USERNAME", qp_live::mock::FAKE_USERNAME)
        .env("MATRIKS_API_PASSWORD", qp_live::mock::FAKE_PASSWORD)
        .env("MATRIKS_HARDWARE_ID", "qapsule-test")
        .env("MATRIKS_SERVICE_TYPE_CODE", "H")
        .env("MATRIKS_MQTT_MODE", "delayed")
        .env("MATRIKS_MQTT_DL_HOST", broker.url("market"));
    c
}

#[test]
fn record_trades_then_inspect() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("fixtures/local")).unwrap();
    std::fs::write(root.join("Cargo.toml"), "").unwrap();
    let tokens = MockTokenService::start_default(3600);
    let broker = MockBroker::start(BrokerScript::default());

    // `@lvl2` is refused before any token or connection.
    let out = qp(root, &tokens, &broker)
        .args([
            "live",
            "record",
            "--trades",
            "--symbols",
            "AAA",
            "--level",
            "@lvl2",
        ])
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("level 2"));
    assert_eq!(broker.stats().connections_total, 0);
    assert_eq!(tokens.requests(), 0);

    let child = qp(root, &tokens, &broker)
        .args([
            "live",
            "record",
            "--trades",
            "--tradeex",
            "--symbols",
            "AAA,BBB",
        ])
        .args(["--seconds", "4", "--every", "1"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    assert!(broker.wait_until(Duration::from_secs(10), |s| s
        .path("trade")
        .subscribed_topics
        .len()
        >= 4));
    let now_ms = i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    )
    .unwrap();
    // Since-midnight nanoseconds (the documented unit) of "now − 2 s", Istanbul.
    let ns = ((now_ms - 2_000 + 3 * 3_600_000).rem_euclid(86_400_000)) * 1_000_000;
    for (i, (sym, side)) in [("AAA", "B"), ("AAA", "S"), ("BBB", "B")]
        .iter()
        .enumerate()
    {
        let m = qp_matriks::messages::TradeMessage {
            symbol: (*sym).to_owned(),
            order_no: Some(format!("{}", 987_650 + i)),
            price: Some(12.5),
            quantity: Some(3),
            active_bid_or_ask: Some((*side).to_owned()),
            timestamp: Some(ns),
            buyer: Some("BUYERX".to_owned()),
            seller: Some("SELLERY".to_owned()),
            is_trade_ex: Some(false),
        };
        broker.publish(&format!("mx/trade/{sym}"), &m.encode_to_vec());
    }
    let out = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "{stdout}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(stdout.contains("endpoint delayed-trade"), "{stdout}");
    assert!(stdout.contains("trades: 3 messages"), "{stdout}");
    assert!(stdout.contains("buyer 3, seller 3"), "{stdout}");
    let st = broker.stats();
    assert_eq!(st.path("trade").connections_total, 1);
    assert_eq!(
        st.path("market").connections_total,
        0,
        "trade endpoint only"
    );
    let topics = st.path("trade").subscribed_topics;
    assert!(topics.iter().all(|t| !t.contains('@')), "{topics:?}");
    assert!(topics.contains(&"mx/tradeex/BBB".to_owned()));

    let raw_dir = root.join("fixtures/local/live/raw");
    let files: Vec<_> = std::fs::read_dir(&raw_dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(files.len(), 1);
    assert!(files[0].to_string_lossy().ends_with("-trade.qpraw"));
    let bytes = std::fs::read(&files[0]).unwrap();
    assert!(!bytes.windows(6).any(|w| w == b"BUYERX"));
    assert!(!bytes.windows(7).any(|w| w == b"SELLERY"));

    let out = qp(root, &tokens, &broker)
        .args(["live", "inspect"])
        .arg(&files[0])
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{text}");
    assert!(
        text.contains("trades: 3 messages (3 mx/trade, 0 mx/tradeex), 2 symbols"),
        "{text}"
    );
    assert!(text.contains("  buyer: 0"), "{text}");
    assert!(text.contains("  seller: 0"), "{text}");
    assert!(text.contains("  timestamp: 3"), "{text}");
    assert!(text.contains("buyer_present=3 seller_present=3"), "{text}");
    assert!(
        text.contains("valid as ns since local midnight: 3 of 3"),
        "{text}"
    );
    assert!(text.contains("\"ns_from_midnight\": 3"), "{text}");
    assert!(
        text.contains("\"B\": 2") && text.contains("\"S\": 1"),
        "{text}"
    );
    // No value: no price, no trade number.
    assert!(!text.contains("12.5") && !text.contains("98765"), "{text}");
}
