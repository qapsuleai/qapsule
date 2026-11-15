//! RP2/T4 measurement (criterion 2): the snapshot of a 10,000-bar **bound** document and
//! the device-ready time.
//!
//! Document: `fixtures/protocol/docs/thyao-bound.json` with `px` bound to
//! `THYAO 1d {"last": BARS}` (+ rsi14, macd, bb20, view `main`), plus a fact, a finding and
//! a live capsule (as in RD1/T6d and RP2/T1). The authority resolves `px` from a
//! synthetic 10,000-bar feed (`MemoryProvider`) and anchors it.
//!
//! - Snapshot: the `result` message of `{"kind":"document"}` (bytes as sent).
//! - Device ready = parse the snapshot + `Session::with_provider` (resolve the binding
//!   from the anchor via a `MemoryProvider` holding the bars; the provider itself is
//!   built beforehand: on a device it is the local data source) + `hello` of a reader
//!   (`arkin`, phone) and of `feed` — the RP2/T1 definition, with resolution added.
//!
//! Usage: `bound_snapshot [BARS=10000] [REPS=15] [SNAPSHOT_OUT]` (writes the snapshot
//! to `SNAPSHOT_OUT` for `gzip -6`). Runs natively and on wasm32-wasip1 (Node WASI,
//! `rounds/RP2/experiments/tick-bench/run-wasi.mjs`).

use std::path::Path;
use std::time::Instant;

use qp_schema::data::FeedKey;
use qp_schema::protocol::{DocumentRef, SessionInit};
use qp_schema::Document;
use qp_session::Session;
use qp_source::MemoryProvider;
use serde_json::{json, Value};

const DOC: &str = include_str!("../../../fixtures/protocol/docs/thyao-bound.json");

fn principals() -> Value {
    json!([
        {"id":"arkin","author":{"kind":"human","name":"arkin"},"role":"owner"},
        {"id":"scout","author":{"kind":"agent","name":"scout"},"role":"proposer"},
        {"id":"feed","author":{"kind":"engine","name":"feed","version":"0"},"role":"feed"},
        {"id":"host","author":{"kind":"engine","name":"host","version":"0"},"role":"host"}
    ])
}

fn init(doc: Document) -> SessionInit {
    serde_json::from_value(json!({
        "document": "placeholder",
        "clock": 1_760_000_000_000_i64,
        "principals": principals(),
    }))
    .map(|mut i: SessionInit| {
        i.document = DocumentRef::Inline(Box::new(doc));
        i
    })
    .expect("init")
}

fn provider(bars: usize) -> MemoryProvider {
    let mut p = MemoryProvider::new();
    p.add_feed(
        FeedKey {
            symbol: "THYAO".into(),
            interval: "1d".into(),
            feed: None,
        },
        qp_source::synthetic_bars(bars, 7),
    )
    .expect("feed");
    p.entitle("host", ["*".to_owned()]).expect("host");
    p.entitle("arkin", ["THYAO".to_owned()]).expect("arkin");
    p
}

fn first_msg(out: &[qp_schema::protocol::OutboundLine]) -> Value {
    serde_json::to_value(&out[0].msg).expect("json")
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
}

#[allow(clippy::cast_precision_loss)]
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let bars: usize = args.get(1).and_then(|a| a.parse().ok()).unwrap_or(10_000);
    let reps: usize = args.get(2).and_then(|a| a.parse().ok()).unwrap_or(15);
    let out = args.get(3);

    // --- Authority: load, resolve, anchor, add fact/finding/capsule. ---
    let mut doc: Document = serde_json::from_str(DOC).expect("doc");
    if let qp_schema::SeriesSource::Bind { window, .. } = &mut doc.series[0].source {
        window.last = Some(u32::try_from(bars).expect("bars"));
    }
    let mut auth = Session::with_provider(init(doc), Path::new("."), Box::new(provider(bars)))
        .expect("authority");
    auth.handle_raw(
        "a",
        r#"{"type":"hello","protocol":"qp/0","principal":"arkin","surface":{"kind":"headless"}}"#,
    );
    let last = bars - 1;
    let cmd = json!({"type":"command","id":"c1","ops":[
        {"op":"add_fact","id":"fact-1","kind":"cross_above","series":"rsi14","bar":last,"text":"RSI(14) bar.","evidence":{"level":70.0}},
        {"op":"add_finding","id":"fnd-1","claim":"THYAO günlükte RSI(14) aşırı alım bölgesine girdi.","facts":["fact-1"],"confidence":0.65,
         "invalidation":{"kind":"threshold","series":"rsi14","op":"cross_below","value":70.0},"capsule":"THYAO aşırı alımda."}]});
    let ack = first_msg(&auth.handle_raw("a", &cmd.to_string()));
    assert_eq!(ack["status"], "applied", "{ack}");

    // --- Snapshot. ---
    let t0 = Instant::now();
    let snap_lines = auth.handle_raw(
        "a",
        r#"{"type":"query","id":"snap","query":{"kind":"document"}}"#,
    );
    let snapshot = serde_json::to_string(&snap_lines[0].msg).expect("snapshot");
    let serialize_ms = t0.elapsed().as_secs_f64() * 1e3;
    assert!(
        !snapshot.contains("\"bars\":["),
        "the snapshot must hold no bars"
    );
    if let Some(path) = out {
        std::fs::write(path, &snapshot).expect("write snapshot");
    }
    println!("bars = {bars}");
    println!("snapshot_bytes = {}", snapshot.len());
    println!("snapshot_query_serialize_ms = {serialize_ms:.3}");

    // --- Device ready. ---
    let providers: Vec<MemoryProvider> = (0..reps * 4).map(|_| provider(bars)).collect();
    let mut providers = providers.into_iter();
    let mut total = Vec::new();
    let (mut parse, mut new, mut hello_r, mut hello_f) = (vec![], vec![], vec![], vec![]);
    for _ in 0..reps {
        let p = providers.next().expect("provider");
        let t = Instant::now();
        let v: Value = serde_json::from_str(&snapshot).expect("parse");
        let d: Document = serde_json::from_value(v["data"]["document"].clone()).expect("doc");
        let mut s = Session::with_provider(init(d), Path::new("."), Box::new(p)).expect("device");
        s.handle_raw(
            "r",
            r#"{"type":"hello","protocol":"qp/0","principal":"arkin","surface":{"kind":"phone"}}"#,
        );
        s.handle_raw(
            "f",
            r#"{"type":"hello","protocol":"qp/0","principal":"feed","surface":{"kind":"headless"}}"#,
        );
        total.push(t.elapsed().as_secs_f64() * 1e3);
        assert!(
            s.log().is_empty(),
            "the anchored snapshot resolves without a new entry"
        );
        assert_eq!(
            s.binding_state("px"),
            Some(qp_schema::data::BindingState::Ready)
        );
    }
    for _ in 0..reps {
        let p = providers.next().expect("provider");
        let t = Instant::now();
        let v: Value = serde_json::from_str(&snapshot).expect("parse");
        let d: Document = serde_json::from_value(v["data"]["document"].clone()).expect("doc");
        parse.push(t.elapsed().as_secs_f64() * 1e3);
        let t = Instant::now();
        let mut s = Session::with_provider(init(d), Path::new("."), Box::new(p)).expect("device");
        new.push(t.elapsed().as_secs_f64() * 1e3);
        let t = Instant::now();
        s.handle_raw(
            "r",
            r#"{"type":"hello","protocol":"qp/0","principal":"arkin","surface":{"kind":"phone"}}"#,
        );
        hello_r.push(t.elapsed().as_secs_f64() * 1e3);
        let t = Instant::now();
        s.handle_raw(
            "f",
            r#"{"type":"hello","protocol":"qp/0","principal":"feed","surface":{"kind":"headless"}}"#,
        );
        hello_f.push(t.elapsed().as_secs_f64() * 1e3);
    }
    println!("ready_total_ms_median = {:.3}", median(total));
    println!("ready_parse_ms_median = {:.3}", median(parse));
    println!("ready_session_new_resolve_ms_median = {:.3}", median(new));
    println!("ready_hello_reader_ms_median = {:.3}", median(hello_r));
    println!("ready_hello_feed_ms_median = {:.3}", median(hello_f));
}
