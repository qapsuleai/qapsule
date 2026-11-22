//! RP2/T9 (R2 leftover): a command with selectors no longer copies the document per op.
//! The result must be the same as the same command written with ids; the timing test
//! measures the cost on a 10,000-bar document (`cargo test --release -p qp-session
//! --test selector_copies -- --ignored --nocapture`).

mod support;

use std::time::Instant;

use qp_schema::protocol::DocumentRef;
use qp_schema::{Bar, Document, SeriesData};
use serde_json::{json, Value};
use support::{init, root, H};

/// The `thyao` document with its bar series extended to `n` bars (deterministic).
fn big_init(n: usize) -> qp_schema::protocol::SessionInit {
    let mut init = init();
    let mut doc: Document = match &init.document {
        DocumentRef::Path(p) => qp_session::read_json(&root().join(p)).unwrap(),
        DocumentRef::Inline(d) => (**d).clone(),
    };
    let px = doc.series.iter_mut().find(|s| s.id == "px").unwrap();
    let Some(SeriesData::Bars { bars }) = &mut px.data else {
        panic!("px has no bars")
    };
    let first = bars[0].t;
    let have = bars.len();
    let mut pre: Vec<Bar> = (0..n - have)
        .map(|i| {
            #[allow(clippy::cast_precision_loss)]
            let c = 100.0 + ((i * 7919) % 200) as f64 / 10.0;
            Bar {
                t: first - (n - have - i) as i64 * 86_400_000,
                o: c,
                h: c + 1.0,
                l: c - 1.0,
                c,
                v: 1000.0,
            }
        })
        .collect();
    pre.append(bars);
    *bars = pre;
    // Facts keep their meaning: shift their bars.
    let shift = (n - have) as u64;
    for f in &mut doc.facts {
        f.bar += shift;
    }
    init.document = DocumentRef::Inline(Box::new(doc));
    init
}

fn session(init: &qp_schema::protocol::SessionInit) -> H {
    let mut h = H::with(init.clone());
    h.hello("a", "arkin", "headless");
    h
}

/// Three ops; the second and third name layers by selector (the first changes the
/// document, so they resolve against the document as it leaves it).
fn three_ops(sel: bool) -> Value {
    let candle = if sel {
        json!({"mark":"candle"})
    } else {
        json!("l0")
    };
    let bb = if sel {
        json!({"shows":"bb20"})
    } else {
        json!("l1")
    };
    json!({"type":"command","id":"c1","ops":[
        {"op":"set_encoding","view":"main","layer":"l0","encoding":{"color":"up"}},
        {"op":"set_encoding","view":"main","layer":candle,"encoding":{"color":"down"}},
        {"op":"set_encoding","view":"main","layer":bb,"encoding":{"color":"up"}}]})
}

fn one_op(sel: bool) -> Value {
    let candle = if sel {
        json!({"mark":"candle"})
    } else {
        json!("l0")
    };
    json!({"type":"command","id":"c1","ops":[
        {"op":"set_encoding","view":"main","layer":candle,"encoding":{"color":"down"}}]})
}

fn doc_bytes(h: &H) -> Vec<u8> {
    serde_json::to_vec(h.s.document()).unwrap()
}

#[test]
fn selector_commands_equal_their_id_form() {
    let init = big_init(2_000);
    for make in [three_ops as fn(bool) -> Value, one_op] {
        let mut a = session(&init);
        let mut b = session(&init);
        let ra = a.one("a", make(true));
        let rb = b.one("a", make(false));
        assert_eq!(ra["status"], "applied", "{ra}");
        assert_eq!(rb["status"], "applied", "{rb}");
        assert_eq!(doc_bytes(&a), doc_bytes(&b));
        // The log holds ids only (§17.4): same bytes.
        assert_eq!(
            serde_json::to_vec(a.s.log()).unwrap(),
            serde_json::to_vec(b.s.log()).unwrap()
        );
    }
}

#[test]
fn a_failing_earlier_op_changes_nothing_and_is_reported() {
    let init = big_init(500);
    let mut h = session(&init);
    let before = doc_bytes(&h);
    let r = h.one(
        "a",
        json!({"type":"command","id":"c1","ops":[
            {"op":"set_encoding","view":"main","layer":"nope","encoding":{"color":"up"}},
            {"op":"set_encoding","view":"main","layer":{"mark":"candle"},"encoding":{"color":"down"}}]}),
    );
    assert_eq!(r["type"], "error", "{r}");
    assert_eq!(r["error"]["path"], "/ops/0/layer", "{r}");
    assert_eq!(doc_bytes(&h), before);
    // A later selector still sees what an earlier op inserted (scratch copy).
    let r = h.one(
        "a",
        json!({"type":"command","id":"c2","ops":[
            {"op":"insert_layer","view":"main","layer":{"id":"x1","mark":"line","series":"bb20","encoding":{"y":"upper"}}},
            {"op":"set_encoding","view":"main","layer":{"output":"upper"},"encoding":{"color":"down"}}]}),
    );
    assert_eq!(r["status"], "applied", "{r}");
    assert_eq!(r["resolved"]["/ops/1/layer"], "x1", "{r}");
}

fn median(mut v: Vec<u128>) -> u128 {
    v.sort_unstable();
    v[v.len() / 2]
}

#[test]
#[ignore = "timing; run in release with --ignored --nocapture"]
fn selector_command_timing_10k_bars() {
    let n: usize = std::env::var("QP_BARS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(10_000);
    let init = big_init(n);
    println!("| command ({n} bars) | n | median µs |\n|---|--:|--:|");
    for (name, msg) in [
        ("1 op, id", one_op(false)),
        ("1 op, selector", one_op(true)),
        ("3 ops, ids", three_ops(false)),
        ("3 ops, selectors in ops 2 and 3", three_ops(true)),
    ] {
        let line = msg.to_string();
        let mut t = Vec::new();
        for i in 0..230 {
            let mut h = session(&init);
            let st = Instant::now();
            std::hint::black_box(h.s.handle_raw("a", &line));
            if i >= 30 {
                t.push(st.elapsed().as_nanos());
            }
        }
        println!("| {name} | 200 | {:.1} |", median(t) as f64 / 1000.0);
    }
}
