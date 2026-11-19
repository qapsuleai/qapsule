//! Raw message log round trip and damage detection (hand-built records only).

use prost::Message;
use qp_matriks::messages::SymbolMessage;
use qp_matriks::rawlog::{HEADER_LEN, MAGIC};
use qp_matriks::{RawError, RawReader, RawRecord, RawWriter, RecordKind, SymbolBook};

fn sample() -> Vec<RawRecord> {
    let card = SymbolMessage {
        symbol_id: 9001,
        last: Some(10.15),
        ..Default::default()
    }
    .encode_to_vec();
    vec![
        RawRecord {
            t_us: 1,
            kind: RecordKind::ConnectionOpened,
            topic: "market".into(),
            payload: vec![],
        },
        RawRecord {
            t_us: 2,
            kind: RecordKind::Subscribed,
            topic: "mx/symbol/TEST1@lvl2".into(),
            payload: vec![],
        },
        RawRecord::message(3, "mx/symbol/TEST1@lvl2", card),
        RawRecord {
            t_us: 4,
            kind: RecordKind::SubscribeRejected,
            topic: "mx/symbol/TEST2@lvl2".into(),
            payload: vec![0x80],
        },
        RawRecord {
            t_us: -5,
            kind: RecordKind::Note,
            topic: String::new(),
            payload: "run start".into(),
        },
        RawRecord {
            t_us: i64::MAX,
            kind: RecordKind::ConnectionLost,
            topic: "market".into(),
            payload: "eof".into(),
        },
    ]
}

fn write_all(recs: &[RawRecord]) -> Vec<u8> {
    let mut w = RawWriter::new(Vec::new()).unwrap();
    for r in recs {
        w.write(r).unwrap();
    }
    w.into_inner()
}

#[test]
fn round_trip() {
    let bytes = write_all(&sample());
    assert_eq!(&bytes[..8], &MAGIC);
    let (recs, err) = RawReader::new(bytes.as_slice()).unwrap().read_all();
    assert!(err.is_none());
    assert_eq!(recs, sample());
    // Deterministic: same records, same bytes.
    assert_eq!(write_all(&sample()), bytes);
}

#[test]
fn empty_log_and_header_errors() {
    let bytes = write_all(&[]);
    assert_eq!(bytes.len(), HEADER_LEN);
    assert_eq!(
        RawReader::new(bytes.as_slice()).unwrap().read_all().0,
        vec![]
    );
    assert!(matches!(
        RawReader::new(&b"NOTALOG!...."[..]),
        Err(RawError::BadMagic)
    ));
    assert!(matches!(
        RawReader::new(&bytes[..10]),
        Err(RawError::Truncated { offset: 0 })
    ));
    let mut v2 = bytes.clone();
    v2[8] = 2;
    assert!(matches!(
        RawReader::new(v2.as_slice()),
        Err(RawError::UnsupportedVersion(2))
    ));
}

#[test]
fn every_truncation_point_is_detected() {
    let recs = sample();
    let bytes = write_all(&recs);
    // Record boundaries.
    let mut bounds = vec![HEADER_LEN];
    for i in 1..=recs.len() {
        bounds.push(write_all(&recs[..i]).len());
    }
    for cut in HEADER_LEN..bytes.len() {
        let (got, err) = RawReader::new(&bytes[..cut]).unwrap().read_all();
        let complete = bounds.iter().filter(|&&b| b <= cut).count() - 1;
        assert_eq!(got, recs[..complete], "cut {cut}");
        if bounds.contains(&cut) {
            assert!(err.is_none(), "cut {cut} at boundary is a clean end");
        } else {
            match err {
                Some(RawError::Truncated { offset }) => {
                    assert_eq!(offset as usize, bounds[complete])
                }
                other => panic!("cut {cut}: expected Truncated, got {other:?}"),
            }
        }
    }
}

#[test]
fn flipped_byte_is_corrupt_and_offset_allows_repair() {
    let recs = sample();
    let mut bytes = write_all(&recs);
    let second = write_all(&recs[..1]).len();
    bytes[second + 12] ^= 0xFF; // inside record 2's body
    let mut r = RawReader::new(bytes.as_slice()).unwrap();
    assert_eq!(r.next_record().unwrap().unwrap(), recs[0]);
    assert!(matches!(r.next_record(), Err(RawError::Corrupt { .. })));
    assert_eq!(
        r.offset() as usize,
        second,
        "offset = end of last good record"
    );
    assert!(
        r.next_record().unwrap().is_none(),
        "reader stops after an error"
    );
}

#[test]
fn resume_appends_without_header() {
    let recs = sample();
    let mut bytes = write_all(&recs[..2]);
    let mut w = RawWriter::resume(&mut bytes);
    for r in &recs[2..] {
        w.write(r).unwrap();
    }
    assert_eq!(bytes, write_all(&recs));
}

#[test]
fn writer_rejects_oversized_topic() {
    let mut w = RawWriter::new(Vec::new()).unwrap();
    let topic = "x".repeat(70_000);
    assert!(matches!(
        w.write_parts(0, RecordKind::Message, &topic, &[]),
        Err(RawError::TooLarge)
    ));
}

/// Replay drives the same decoder: two replays of one log give identical state.
#[test]
fn replay_is_deterministic() {
    let mut recs = sample();
    let upd = SymbolMessage {
        symbol_id: 9001,
        bid: Some(10.1),
        ..Default::default()
    }
    .encode_to_vec();
    recs.push(RawRecord::message(7, "mx/symbol/TEST1@lvl2", upd));
    let bytes = write_all(&recs);
    let run = || {
        let mut book = SymbolBook::new();
        let mut events = Vec::new();
        for r in RawReader::new(bytes.as_slice()).unwrap() {
            let r = r.unwrap();
            if r.kind != RecordKind::Message {
                continue;
            }
            if let qp_matriks::Topic::Symbol { symbol, .. } = qp_matriks::parse_topic(&r.topic) {
                events.push(book.apply(&symbol, &r.payload).unwrap());
            }
        }
        (events, book.state("TEST1").cloned())
    };
    let a = run();
    assert_eq!(a, run());
    assert_eq!(a.0.len(), 2);
    let s = a.1.unwrap();
    assert_eq!((s.last, s.bid), (Some(10.15), Some(10.1)));
}
