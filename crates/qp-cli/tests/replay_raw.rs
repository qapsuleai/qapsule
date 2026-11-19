//! `qp replay --raw` (R6/T5, VERIFY 8): a hand-built raw log (made-up symbol and prices,
//! written in a temporary directory) replays to byte-equal output twice.

use assert_cmd::Command;
use prost::Message as _;
use qp_matriks::messages::SymbolMessage;
use qp_matriks::{RawRecord, RawWriter, RecordKind};

fn records() -> Vec<RawRecord> {
    let base = qp_calendar::days_from_civil(2026, 11, 18) * 86_400_000 - 3 * 3_600_000;
    let topic = "mx/symbol/TEST1@lvl2";
    let mut out = vec![RawRecord {
        t_us: (base + 10 * 3_600_000) * 1000,
        kind: RecordKind::ConnectionOpened,
        topic: "market".into(),
        payload: Vec::new(),
    }];
    let mut cum = 0.0;
    for i in 0..30i64 {
        let local = 10 * 3_600_000 + 5_000 + i * 23_000;
        let t = base + local;
        cum += 50.0;
        let m = SymbolMessage {
            symbol_id: 1,
            update_date: Some(format!(
                "2026-11-18T{:02}:{:02}:{:02}.{:03}+03",
                local / 3_600_000,
                local / 60_000 % 60,
                local / 1000 % 60,
                local % 1000
            )),
            last: Some(20.0 + (i % 5) as f64 * 0.1),
            daily_volume: Some(cum),
            stock_status: (i == 0).then_some(2),
            ..SymbolMessage::default()
        };
        out.push(RawRecord::message(
            (t + 900_000) * 1000,
            topic,
            m.encode_to_vec(),
        ));
    }
    out
}

#[test]
fn replay_raw_twice_is_byte_equal() {
    let dir = tempfile::tempdir().unwrap();
    let raw = dir.path().join("fake.qpraw");
    let mut w = RawWriter::new(std::fs::File::create(&raw).unwrap()).unwrap();
    for r in records() {
        w.write(&r).unwrap();
    }
    w.flush().unwrap();
    drop(w);
    let run = || {
        Command::cargo_bin("qp")
            .unwrap()
            .args(["replay", "--raw"])
            .arg(&raw)
            .args(["--intervals", "1m,5m,1d"])
            .output()
            .unwrap()
    };
    let a = run();
    let b = run();
    assert!(a.status.success(), "{}", String::from_utf8_lossy(&a.stderr));
    assert_eq!(a.stdout, b.stdout);
    let text = String::from_utf8(a.stdout).unwrap();
    assert!(text.lines().count() > 30);
    assert!(text
        .lines()
        .all(|l| serde_json::from_str::<serde_json::Value>(l).is_ok()));
    assert!(String::from_utf8_lossy(&a.stderr).contains("not under fixtures/local/"));
}
