//! Field presence and protobuf merge semantics for `SymbolMessage`.
//! All messages are built by hand with made-up symbols (`TEST1`) and prices.

use prost::Message;
use qp_matriks::messages::symbol_message::PublishReason;
use qp_matriks::messages::SymbolMessage;
use qp_matriks::{InitialPolicy, SymbolBook, UpdateKind};

fn enc(m: &SymbolMessage) -> Vec<u8> {
    m.encode_to_vec()
}

fn full_card() -> SymbolMessage {
    SymbolMessage {
        symbol_id: 9001,
        symbol_code: Some("TEST1".into()),
        bid: Some(10.10),
        ask: Some(10.20),
        last: Some(10.15),
        daily_volume: Some(123_456.5),
        daily_quantity: Some(12_000.0),
        high: Some(10.40),
        low: Some(9.90),
        day_close: Some(10.00),
        stock_status: Some(2),
        session_is_open: Some(true),
        limit_up: Some(11.00),
        ..Default::default()
    }
}

/// Library behaviour check (not assumed): prost generates proto2 `optional`
/// scalars as `Option<T>`, an absent field decodes to `None`, and an explicit
/// zero is `Some(0.0)` — distinguishable from absent.
#[test]
fn prost_proto2_optional_absent_is_none() {
    // Hand-written wire bytes, independent of prost's encoder:
    // field 1 (symbolId, sint32 zigzag 7 -> 14), field 5 (bid, fixed64 1.5).
    let mut wire = vec![0x08, 14, 0x29];
    wire.extend_from_slice(&1.5f64.to_le_bytes());
    let m = SymbolMessage::decode(wire.as_slice()).unwrap();
    assert_eq!(m.symbol_id, 7);
    assert_eq!(m.bid, Some(1.5));
    assert_eq!(m.last, None);
    assert_eq!(m.daily_volume, None);
    assert_eq!(m.stock_status, None);
    assert_eq!(m.publish_reason, None);
    // Generated getters fall back to the proto default; the field stays None.
    assert_eq!(m.incremental_quantity, None);
    assert_eq!(m.incremental_quantity(), 0.0);
    assert_eq!(m.publish_reason(), PublishReason::Update);

    // Explicit zero survives the round trip as Some(0.0).
    let z = SymbolMessage {
        symbol_id: 7,
        last: Some(0.0),
        ..Default::default()
    };
    let back = SymbolMessage::decode(enc(&z).as_slice()).unwrap();
    assert_eq!(back.last, Some(0.0));
    assert_eq!(back.bid, None);
    // Encoder writes nothing for None fields: only symbolId + last on the wire.
    assert_eq!(
        qp_matriks::wire::field_numbers(&enc(&z)).unwrap(),
        vec![1, 9]
    );
}

/// Observed prost behaviour: a missing proto2 `required` field does not fail
/// decoding; `symbolId` becomes 0. The book keys by topic symbol, not symbolId.
#[test]
fn prost_missing_required_field_decodes() {
    let only_bid = SymbolMessage {
        bid: Some(1.0),
        ..Default::default()
    };
    // symbol_id = 0 is the proto3-like default and *is* written by prost for
    // required fields; build the wire by hand to really omit it.
    let mut wire = vec![0x29];
    wire.extend_from_slice(&1.0f64.to_le_bytes());
    let m = SymbolMessage::decode(wire.as_slice()).unwrap();
    assert_eq!(m.symbol_id, 0);
    assert_eq!(m.bid, only_bid.bid);
}

/// VERIFY 7: an update carrying only `bid` leaves `last`, `dailyVolume`, … unchanged.
#[test]
fn update_with_only_bid_changes_only_bid() {
    let mut book = SymbolBook::new();
    let first = book.apply("TEST1", &enc(&full_card())).unwrap();
    assert_eq!(first.kind, UpdateKind::Initial);

    let upd = SymbolMessage {
        symbol_id: 9001,
        bid: Some(10.12),
        ..Default::default()
    };
    let ev = book.apply("TEST1", &enc(&upd)).unwrap();
    assert_eq!(ev.kind, UpdateKind::Update);
    assert_eq!(ev.fields.names(), vec!["symbolId", "bid"]);
    assert!(ev.fields.contains_name("bid"));
    assert!(!ev.fields.contains_name("last"));
    assert_eq!(ev.publish_reason, None);
    assert_eq!(ev.stock_status, None);

    let s = book.state("TEST1").unwrap();
    let mut expected = full_card();
    expected.bid = Some(10.12);
    assert_eq!(s, &expected);
    assert_eq!(s.last, Some(10.15));
    assert_eq!(s.daily_volume, Some(123_456.5));
    assert_eq!(s.stock_status, Some(2));
}

#[test]
fn explicit_zero_overwrites_absent_does_not() {
    let mut book = SymbolBook::new();
    book.apply("TEST1", &enc(&full_card())).unwrap();
    let upd = SymbolMessage {
        symbol_id: 9001,
        daily_volume: Some(0.0),
        ..Default::default()
    };
    book.apply("TEST1", &enc(&upd)).unwrap();
    let s = book.state("TEST1").unwrap();
    assert_eq!(s.daily_volume, Some(0.0));
    assert_eq!(s.last, Some(10.15));
}

#[test]
fn sequence_of_updates_accumulates() {
    let mut book = SymbolBook::new();
    book.apply("TEST1", &enc(&full_card())).unwrap();
    for (i, px) in [10.16, 10.17, 10.18].iter().enumerate() {
        let upd = SymbolMessage {
            symbol_id: 9001,
            last: Some(*px),
            daily_quantity: Some(12_000.0 + 100.0 * (i as f64 + 1.0)),
            ..Default::default()
        };
        assert_eq!(
            book.apply("TEST1", &enc(&upd)).unwrap().kind,
            UpdateKind::Update
        );
    }
    let s = book.state("TEST1").unwrap();
    assert_eq!(s.last, Some(10.18));
    assert_eq!(s.daily_quantity, Some(12_300.0));
    assert_eq!(s.bid, Some(10.10));
    assert_eq!(s.high, Some(10.40));
}

#[test]
fn symbols_are_independent() {
    let mut book = SymbolBook::new();
    book.apply("TEST1", &enc(&full_card())).unwrap();
    let other = SymbolMessage {
        symbol_id: 9002,
        last: Some(55.5),
        ..Default::default()
    };
    assert_eq!(
        book.apply("TEST2", &enc(&other)).unwrap().kind,
        UpdateKind::Initial
    );
    assert_eq!(book.state("TEST1").unwrap().last, Some(10.15));
    assert_eq!(book.state("TEST2").unwrap().bid, None);
    assert_eq!(book.symbols().collect::<Vec<_>>(), vec!["TEST1", "TEST2"]);
}

#[test]
fn resubscribe_makes_next_message_initial_and_replaces() {
    let mut book = SymbolBook::new();
    book.apply("TEST1", &enc(&full_card())).unwrap();
    book.resubscribed("TEST1");
    assert!(book.awaiting_initial("TEST1"));
    // State stays readable until the initial arrives.
    assert_eq!(book.state("TEST1").unwrap().last, Some(10.15));

    // New full picture without `limitUp` (and with other values).
    let fresh = SymbolMessage {
        symbol_id: 9001,
        last: Some(10.30),
        bid: Some(10.25),
        publish_reason: Some(PublishReason::Refresh as i32),
        ..Default::default()
    };
    let ev = book.apply("TEST1", &enc(&fresh)).unwrap();
    assert_eq!(ev.kind, UpdateKind::Initial);
    assert_eq!(ev.publish_reason_kind(), Some(PublishReason::Refresh));
    let s = book.state("TEST1").unwrap();
    assert_eq!(s, &fresh, "Replace policy: initial rebuilds state");
    assert_eq!(s.limit_up, None, "no stale field from before the initial");

    // Next one is an update again.
    let upd = SymbolMessage {
        symbol_id: 9001,
        ask: Some(10.35),
        ..Default::default()
    };
    assert_eq!(
        book.apply("TEST1", &enc(&upd)).unwrap().kind,
        UpdateKind::Update
    );
    assert_eq!(book.state("TEST1").unwrap().last, Some(10.30));
}

#[test]
fn merge_policy_keeps_old_fields_on_initial() {
    let mut book = SymbolBook::with_policy(InitialPolicy::Merge);
    book.apply("TEST1", &enc(&full_card())).unwrap();
    book.resubscribed_all();
    let fresh = SymbolMessage {
        symbol_id: 9001,
        last: Some(10.30),
        ..Default::default()
    };
    assert_eq!(
        book.apply("TEST1", &enc(&fresh)).unwrap().kind,
        UpdateKind::Initial
    );
    let s = book.state("TEST1").unwrap();
    assert_eq!(s.last, Some(10.30));
    assert_eq!(s.limit_up, Some(11.00));
}

#[test]
fn event_carries_stock_status_and_phase() {
    let mut book = SymbolBook::new();
    book.apply("TEST1", &enc(&full_card())).unwrap();
    let upd = SymbolMessage {
        symbol_id: 9001,
        stock_status: Some(5),
        publish_reason: Some(PublishReason::Update as i32),
        ..Default::default()
    };
    let ev = book.apply("TEST1", &enc(&upd)).unwrap();
    assert_eq!(ev.stock_status, Some(5));
    assert_eq!(ev.session_phase().unwrap().key, "closing_session");
    assert_eq!(ev.publish_reason_kind(), Some(PublishReason::Update));
    assert_eq!(book.state("TEST1").unwrap().stock_status, Some(5));
}

#[test]
fn bad_payload_leaves_state_and_status_unchanged() {
    let mut book = SymbolBook::new();
    book.apply("TEST1", &enc(&full_card())).unwrap();
    let before = book.state("TEST1").unwrap().clone();
    let mut bad = enc(&SymbolMessage {
        symbol_id: 9001,
        bid: Some(1.0),
        last: Some(2.0),
        ..Default::default()
    });
    bad.truncate(bad.len() - 3);
    assert!(book.apply("TEST1", &bad).is_err());
    assert_eq!(book.state("TEST1").unwrap(), &before);
    assert!(!book.awaiting_initial("TEST1"));

    // First message ever being bad: no entry is created, next good one is initial.
    assert!(book.apply("TEST9", &bad).is_err());
    assert!(book.state("TEST9").is_none());
    assert!(book.awaiting_initial("TEST9"));
}

#[test]
fn unknown_field_numbers_are_reported() {
    let mut wire = enc(&SymbolMessage {
        symbol_id: 9001,
        bid: Some(1.0),
        ..Default::default()
    });
    // field 200, varint 1 (not in the proto)
    wire.extend_from_slice(&[0xC0, 0x0C, 0x01]);
    let mut book = SymbolBook::new();
    let ev = book.apply("TEST1", &wire).unwrap();
    assert_eq!(ev.fields.unknown(), vec![200]);
    assert_eq!(ev.fields.names(), vec!["symbolId", "bid"]);
}
