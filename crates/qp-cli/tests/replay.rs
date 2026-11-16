//! `qp replay` (docs/PROTOCOL.md §21.3): a backward `seek` is a new authority
//! (`reset: true`, epoch + 1) that closes every connection; the tool connects its
//! watching connection again (`hello`, `capsules` + `changes`) and the stream goes on.

use assert_cmd::Command;
use serde_json::Value;

fn run(dir: &std::path::Path, name: &str) -> Vec<u8> {
    let controls = dir.join("controls.json");
    std::fs::write(
        &controls,
        r#"[{"at":5,"control":{"kind":"step","n":3}},
            {"at":40,"control":{"kind":"seek","step":12}},
            {"at":60,"control":{"kind":"status"}},
            {"at":70,"control":{"kind":"seek","step":20}}]"#,
    )
    .unwrap();
    let out = dir.join(name);
    Command::cargo_bin("qp")
        .unwrap()
        .args([
            "replay",
            "--source",
            "synthetic",
            "--seed",
            "7",
            "--instruments",
            "6",
            "--days",
            "200",
            "--steps",
            "160",
            "--controls",
        ])
        .arg(&controls)
        .arg("--out")
        .arg(&out)
        .env_remove("QAPSULE_LOCAL_BARS")
        .assert()
        .success();
    std::fs::read(out).unwrap()
}

#[test]
fn backward_seek_reconnects_and_the_stream_goes_on() {
    let tmp = tempfile::tempdir().unwrap();
    let a = run(tmp.path(), "a.jsonl");
    let b = run(tmp.path(), "b.jsonl");
    assert_eq!(a, b, "two runs differ");

    let msgs: Vec<Value> = String::from_utf8(a)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str::<Value>(l).unwrap()["msg"].clone())
        .collect();
    // No message is an error (before the fix: `handshake` after the first backward seek).
    for m in &msgs {
        assert_ne!(m["type"], "error", "error line: {m}");
    }
    let resets: Vec<usize> = msgs
        .iter()
        .enumerate()
        .filter(|(_, m)| m["data"]["replay"]["reset"] == true)
        .map(|(i, _)| i)
        .collect();
    assert_eq!(resets.len(), 2, "two backward seeks");
    let last = *resets.last().unwrap();
    let epoch = msgs[last]["data"]["replay"]["epoch"].as_u64().unwrap();
    assert_eq!(epoch, 2);
    let after = &msgs[last + 1..];
    // The new connection: welcome, then both subscriptions acknowledged.
    assert_eq!(after[0]["type"], "welcome", "{}", after[0]);
    assert_eq!(after[1]["id"], "caps");
    assert_eq!(after[2]["id"], "ch");
    // And the stream goes on: notifications after the last backward seek.
    let notes = after.iter().filter(|m| m["type"] == "notify").count();
    assert!(notes > 0, "no notification after the last backward seek");
    // The status between the seeks answered on the reconnected connection.
    assert!(msgs
        .iter()
        .any(|m| m["id"] == "ctl2" && m["type"] == "result"));
}
