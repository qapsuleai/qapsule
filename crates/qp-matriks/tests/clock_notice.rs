//! `TimeMessage` / `MarketClock` and `MessengerMessage` classification,
//! with hand-built messages (no real data).

use prost::Message;
use qp_matriks::messages::{MessengerMessage, SymbolMessage, TimeMessage};
use qp_matriks::{classify_notice, decode_time, BrokerNotice, ClockEvent, MarketClock};

fn time(us: i64, source: &str, open: bool, holiday: bool) -> Vec<u8> {
    TimeMessage {
        timestamp: us,
        source: source.into(),
        is_bist_pp_holiday: holiday,
        is_bist_pp_open: open,
    }
    .encode_to_vec()
}

// 2001-09-09T01:46:40.123456Z
const T0: i64 = 1_000_000_000_123_456;

#[test]
fn decodes_time_message_units() {
    let t = decode_time(&time(T0, "MX", true, false)).unwrap();
    assert_eq!(t.micros, T0);
    assert_eq!(t.millis(), 1_000_000_000_123);
    assert!(t.is_mx() && t.is_bist_pp_open && !t.is_bist_pp_holiday);
    // Negative values floor (not truncate) to ms.
    let n = decode_time(&time(-1, "MX", false, false)).unwrap();
    assert_eq!(n.millis(), -1);
}

/// proto3: `false` / `0` / `""` are not on the wire; absent and default look the same.
#[test]
fn proto3_defaults_are_not_on_the_wire() {
    let t = decode_time(&time(T0, "MX", false, false)).unwrap();
    assert_eq!(t.fields.names(), vec!["timestamp", "source"]);
    assert!(!t.fields.contains_name("isBistPPOpen"));
    let empty = decode_time(&[]).unwrap();
    assert_eq!(
        (empty.micros, empty.source.as_str(), empty.is_bist_pp_open),
        (0, "", false)
    );
    assert!(empty.fields.is_empty());
}

#[test]
fn market_clock_uses_only_mx() {
    let mut c = MarketClock::new();
    assert_eq!(c.is_open(), None);

    // Non-MX tick before any MX: openness stays unknown.
    let ev = c.apply(&decode_time(&time(T0, "BISTPP", true, false)).unwrap());
    assert!(ev.is_empty());
    assert_eq!(c.is_open(), None);
    assert_eq!(c.last_any_micros(), Some(T0));
    assert_eq!(c.last_mx_micros(), None);

    let ev = c.apply(&decode_time(&time(T0 + 1, "MX", false, false)).unwrap());
    assert_eq!(
        ev,
        vec![ClockEvent::Known {
            at_micros: T0 + 1,
            open: false,
            holiday: false
        }]
    );

    // Same state: no event.
    assert!(c
        .apply(&decode_time(&time(T0 + 2, "MX", false, false)).unwrap())
        .is_empty());

    let ev = c.apply(&decode_time(&time(T0 + 3, "MX", true, false)).unwrap());
    assert_eq!(ev, vec![ClockEvent::Opened { at_micros: T0 + 3 }]);
    assert_eq!(c.is_open(), Some(true));

    // A contradicting BISTPP flag does not close the market.
    assert!(c
        .apply(&decode_time(&time(T0 + 4, "BISTPP", false, false)).unwrap())
        .is_empty());
    assert_eq!(c.is_open(), Some(true));
    assert_eq!(c.last_mx_micros(), Some(T0 + 3));
    assert_eq!(c.last_any_micros(), Some(T0 + 4));

    let ev = c.apply(&decode_time(&time(T0 + 5, "MX", false, true)).unwrap());
    assert_eq!(
        ev,
        vec![
            ClockEvent::Closed { at_micros: T0 + 5 },
            ClockEvent::HolidayChanged {
                at_micros: T0 + 5,
                holiday: true
            }
        ]
    );
    assert_eq!(c.is_holiday(), Some(true));

    c.reset();
    assert_eq!(c.is_open(), None);
}

fn messenger(code: &str, subject: Option<&str>) -> Vec<u8> {
    MessengerMessage {
        code: code.into(),
        from: Some("test".into()),
        timestamp: Some(42),
        content_type: None,
        subject: subject.map(Into::into),
        content: Some("made-up notice".into()),
    }
    .encode_to_vec()
}

#[test]
fn multi_connect_in_both_shapes() {
    let a = classify_notice(
        "mx/messenger",
        &messenger("DISCONNECT", Some("MULTICONNECT")),
    )
    .unwrap();
    assert!(a.is_multi_connect());
    let b = classify_notice("mx/messenger", &messenger("MULTICONNECT", None)).unwrap();
    assert!(b.is_multi_connect());
    let c = classify_notice("mx/user/u1/messenger", &messenger(" multiconnect ", None)).unwrap();
    assert!(c.is_multi_connect());
    let d = classify_notice(
        "mx/user/u1/mx/messenger",
        &messenger("disconnect", Some("multiconnect")),
    )
    .unwrap();
    assert!(d.is_multi_connect());
    if let BrokerNotice::MultiConnect(info) = a {
        assert_eq!(info.subject.as_deref(), Some("MULTICONNECT"));
        assert_eq!(info.timestamp, Some(42));
        assert_eq!(info.content_type, None, "proto default is not filled in");
    }
}

#[test]
fn other_codes() {
    let n = |code, subj| classify_notice("mx/messenger", &messenger(code, subj)).unwrap();
    assert!(matches!(n("DISCONNECT", None), BrokerNotice::Disconnect(_)));
    assert!(matches!(
        n("DISCONNECT", Some("OTHER")),
        BrokerNotice::Disconnect(_)
    ));
    assert!(matches!(n("ALERT", None), BrokerNotice::Alert(_)));
    assert!(matches!(n("METAUPDATE", None), BrokerNotice::MetaUpdate(_)));
    assert!(matches!(
        n("SOMETHING", Some("MULTICONNECT")),
        BrokerNotice::Other(_)
    ));
}

#[test]
fn non_messenger_topics_and_bad_payloads() {
    let card = SymbolMessage {
        symbol_id: 1,
        ..Default::default()
    }
    .encode_to_vec();
    assert_eq!(classify_notice("mx/symbol/TEST1@lvl2", &card), None);
    assert_eq!(
        classify_notice("mx/timestamp", &messenger("MULTICONNECT", None)),
        None
    );
    assert!(matches!(
        classify_notice("mx/messenger", &[0x0a, 0x09, b'x']),
        Some(BrokerNotice::Malformed { .. })
    ));
}
