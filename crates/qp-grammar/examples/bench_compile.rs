//! Micro benchmark (no criterion): `compile` for 100,000 visible bars at 1200x600, with
//! and without level of detail; median of N runs.
//!
//! `cargo run --release -p qp-grammar --example bench_compile [-- N]`

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::cognitive_complexity
)]

use std::collections::BTreeMap;
use std::time::Instant;

use qp_grammar::{compile, CompileOptions, MemProvider};
use qp_schema::grammar::View;
use qp_schema::Bar;

fn bars(n: usize) -> Vec<Bar> {
    // Deterministic LCG random walk.
    let mut s = 0x5eed_u64;
    let mut c = 100.0_f64;
    (0..n)
        .map(|i| {
            s = s
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let u = ((s >> 11) as f64) / ((1_u64 << 53) as f64) - 0.5;
            let o = c;
            c *= 1.0 + u * 0.02;
            Bar {
                t: 1_000_000_000_000 + i as i64 * 60_000,
                o,
                h: o.max(c) * 1.002,
                l: o.min(c) * 0.998,
                c,
                v: 1000.0,
            }
        })
        .collect()
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
}

fn main() {
    let runs: usize = std::env::args()
        .nth(1)
        .and_then(|a| a.parse().ok())
        .unwrap_or(21);
    let data = bars(100_000);
    let params = BTreeMap::from([("period".to_owned(), 20.0)]);
    let ma = qp_compute::compute_bars("sma", &params, qp_compute::Field::Close, &data)
        .expect("values")
        .outputs
        .into_values()
        .next()
        .expect("one output");
    let p = MemProvider::new()
        .with_bars("px", data, Some("BENCH"), Some("1m"))
        .with_values("ma", "px", vec![("v", ma)], "v", &["v"])
        .with_label("ma", "MA20");
    let view: View = serde_json::from_str(
        r#"{"id":"bench","x":{"kind":"time"},"panels":[{"id":0,"y":{"kind":"linear"}}],
            "layers":[{"id":"candles","mark":"candle","series":"px","panel":0},
                      {"id":"ma","mark":"line","series":"ma","panel":0}]}"#,
    )
    .expect("view");
    for (name, lod) in [("LOD", true), ("full (no LOD)", false)] {
        let mut o = CompileOptions::without_designs(1200, 600);
        o.lod = lod;
        let s = compile(&view, &p, &o).expect("compile");
        let prims = s.prims().count();
        let items: usize = s
            .panels
            .iter()
            .flat_map(|p| &p.layers)
            .map(|l| l.items.len())
            .sum();
        let mut times = Vec::with_capacity(runs);
        for _ in 0..runs {
            let t = Instant::now();
            let s = compile(&view, &p, &o).expect("compile");
            times.push(t.elapsed().as_secs_f64() * 1e3);
            std::hint::black_box(s);
        }
        println!(
            "{name:>14}: median {:.3} ms over {runs} runs (min {:.3}, max {:.3}); {prims} prims, {items} items, lod {:?}",
            median(times.clone()),
            times.iter().copied().fold(f64::INFINITY, f64::min),
            times.iter().copied().fold(0.0, f64::max),
            s.lod
        );
    }
}
