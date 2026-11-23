//! Every conformance transcript, run in process, with its **complete** output locked per
//! step (the runner itself checks only subset patterns): every message, in order, on every
//! connection — commands, notifications, data events, clock, subscriptions, inputs,
//! viewer updates, both worlds' readers — and the final document, log and replay.

use qp_schema::protocol::SessionInit;
use serde_json::Value;

use super::rig::{root, Rig};
use super::snap::Book;

fn init(v: &Value) -> SessionInit {
    match v {
        Value::String(p) => qp_session::read_json(&root().join(p)).unwrap(),
        o => serde_json::from_value(o.clone()).unwrap(),
    }
}

fn run(book: &mut Book, file: &std::path::Path) {
    let t: Value = serde_json::from_str(&std::fs::read_to_string(file).unwrap()).unwrap();
    let name = t["name"].as_str().unwrap().to_owned();
    let mut rig = Rig::with(init(&t["init"]));
    for (i, step) in t["steps"].as_array().unwrap().iter().enumerate() {
        let out = if let Some(s) = step.get("send") {
            rig.send(s["conn"].as_str().unwrap(), &s["msg"])
        } else if let Some(r) = step.get("send_raw") {
            rig.raw(r["conn"].as_str().unwrap(), r["line"].as_str().unwrap())
        } else {
            rig.data(&step["data"])
        };
        book.lines(&format!("{name}.{i:02}"), &out);
    }
    let (d, s) = rig.fin();
    book.put(&format!("{name}.final"), &d, &s);
}

fn files(from: &str, to: &str) -> Vec<std::path::PathBuf> {
    let mut v: Vec<_> = std::fs::read_dir(root().join("fixtures/protocol/conformance"))
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            let n = p.file_name().unwrap().to_string_lossy().into_owned();
            p.extension().is_some_and(|e| e == "json") && n.as_str() >= from && n.as_str() < to
        })
        .collect();
    v.sort();
    v
}

#[test]
fn charac_session_transcripts_core() {
    let mut book = Book::new("transcripts-core");
    let f = files("01", "30");
    assert_eq!(f.len(), 29);
    for p in f {
        run(&mut book, &p);
    }
    book.check();
}

#[test]
fn charac_session_transcripts_data() {
    let mut book = Book::new("transcripts-data");
    let f = files("30", "49");
    assert_eq!(f.len(), 19);
    for p in f {
        run(&mut book, &p);
    }
    book.check();
}

#[test]
fn charac_session_transcripts_surface() {
    let mut book = Book::new("transcripts-surface");
    let f = files("49", "99");
    assert_eq!(f.len(), 22);
    for p in f {
        run(&mut book, &p);
    }
    book.check();
}
