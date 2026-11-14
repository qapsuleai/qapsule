//! Same qp-schema types, serde_json without `float_roundtrip` (reference only).

#[path = "../../../../crates/qp-schema/src/protocol.rs"]
#[allow(dead_code)]
mod protocol;
#[path = "../../../../crates/qp-schema/src/types.rs"]
#[allow(dead_code)]
mod types;
mod schema {
    pub use crate::protocol::*;
    pub use crate::types::*;
}
#[path = "../../src/common.rs"]
#[allow(dead_code)]
mod common;

use common::*;
use schema::{Document, ServerMsg};

fn row(case: &str, m: &ServerMsg) {
    let b = serde_json::to_vec(m).unwrap();
    let back: ServerMsg = serde_json::from_slice(&b).unwrap();
    let mm = bit_mismatches(&msg_bits(m), &msg_bits(&back));
    let enc = median_ns(|| {
        std::hint::black_box(serde_json::to_vec(std::hint::black_box(m)).unwrap());
    });
    let dec = median_ns(|| {
        std::hint::black_box(
            serde_json::from_slice::<ServerMsg>(std::hint::black_box(&b)).unwrap(),
        );
    });
    println!(
        "| {case} | json (no float_roundtrip) | {} | {:.2} | {:.2} | bit diffs {mm} |",
        b.len(),
        enc / 1000.0,
        dec / 1000.0
    );
}

fn main() {
    let c = corpus();
    println!(
        "# T6b plain JSON (no float_roundtrip) — platform: {}\n",
        platform()
    );
    println!("| case | codec | bytes | enc µs | dec µs | note |\n|---|---|---:|---:|---:|---|");
    row("warmup", &c.values_rsi);
    row("tick set_last_bar", &c.ticks_set_last[30]);
    row("tick append_bars(1)", &c.ticks_append1[30]);
    row("values RSI 10k", &c.values_rsi);
    row("values MACD(3) 10k", &c.values_macd);
    row("append_bars 10k", &c.append_10k);
    let d = &c.document;
    let b = serde_json::to_vec(d).unwrap();
    let back: Document = serde_json::from_slice(&b).unwrap();
    let mm = bit_mismatches(&doc_bits(d), &doc_bits(&back));
    let enc = median_ns(|| {
        std::hint::black_box(serde_json::to_vec(std::hint::black_box(d)).unwrap());
    });
    let dec = median_ns(|| {
        std::hint::black_box(serde_json::from_slice::<Document>(std::hint::black_box(&b)).unwrap());
    });
    println!("| snapshot Document 10k | json (no float_roundtrip) | {} | {:.2} | {:.2} | bit diffs {mm} |", b.len(), enc / 1000.0, dec / 1000.0);

    let vals = exactness_values(100_000);
    let (a, v) = exactness_msgs(&vals);
    let (ta, tv) = exactness_msgs(&tricky_values());
    let rt = |m: &ServerMsg| {
        let back: ServerMsg = serde_json::from_slice(&serde_json::to_vec(m).unwrap()).unwrap();
        bit_mismatches(&msg_bits(m), &msg_bits(&back))
    };
    println!(
        "\n## f64 exactness without float_roundtrip ({} values)\n",
        vals.len()
    );
    println!(
        "append_bars bit diffs: {} ; values bit diffs: {} ; tricky-only: {}/{}",
        rt(&a),
        rt(&v),
        rt(&ta),
        rt(&tv)
    );
    // Which tricky values fail?
    let mut bad = Vec::new();
    for x in tricky_values() {
        let s = serde_json::to_string(&x).unwrap();
        let y: f64 = serde_json::from_str(&s).unwrap();
        if y.to_bits() != x.to_bits() {
            bad.push(format!("{s} -> {y:e}"));
        }
    }
    println!("tricky failures: {bad:?}");
}
