//! Runs the crate's own transcripts and, when present, the official conformance
//! transcripts in-process (the `qp serve` transport is tested in `qp-cli`).

mod support;

use qp_session::conform::{collect_files, run_file, subset, TransportKind};
use serde_json::json;

fn run_dir(rel: &str) -> usize {
    let root = support::root();
    let dir = root.join(rel);
    if !dir.is_dir() {
        return 0;
    }
    let files = collect_files(&[dir]).unwrap();
    for f in &files {
        let o = run_file(f, &root, &TransportKind::InProc);
        assert!(o.failure.is_none(), "{}: {}", o.name, o.failure.unwrap());
    }
    files.len()
}

#[test]
fn own_transcripts_pass_inproc() {
    assert_eq!(run_dir("crates/qp-session/tests/transcripts"), 4);
}

#[test]
fn official_transcripts_pass_inproc() {
    run_dir("fixtures/protocol/conformance");
}

#[test]
fn subset_matching_rules() {
    assert!(subset(&json!({"a":1}), &json!({"a":1.0,"b":2}), "").is_ok());
    assert!(subset(&json!({"a":"$any"}), &json!({"a":[1]}), "").is_ok());
    let e = subset(&json!({"a":{"b":[1,2]}}), &json!({"a":{"b":[1]}}), "").unwrap_err();
    assert_eq!(e.0, "/a/b");
    let e = subset(&json!({"a":{"c":1}}), &json!({"a":{}}), "").unwrap_err();
    assert_eq!(e.0, "/a/c");
    assert!(subset(&json!([1, 2]), &json!([1, 3]), "").is_err());
}

#[test]
fn a_wrong_transcript_fails_with_a_precise_message() {
    let root = support::root();
    let dir = tempfile::tempdir().unwrap();
    let t = json!({"name":"bad","description":"d","init":"fixtures/protocol/init/thyao.json","steps":[
        {"send":{"conn":"a","msg":{"type":"hello","protocol":"qp/0","principal":"arkin","surface":{"kind":"desktop"}}},
         "expect":[{"conn":"a","msg":{"type":"welcome","role":"viewer"}}]}]});
    let f = dir.path().join("bad.json");
    std::fs::write(&f, t.to_string()).unwrap();
    let o = run_file(&f, &root, &TransportKind::InProc);
    let msg = o.failure.unwrap();
    assert!(msg.starts_with("step 0 message 0 at `/role`"), "{msg}");
}
