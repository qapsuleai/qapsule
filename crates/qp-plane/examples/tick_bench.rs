//! Tick cost of program series in the values cache (PS §1.3, acceptance 17): ns per tick
//! (`Cache::apply_delta`) for a live revision of the last bar and for a new bar, on a
//! 500-bar series with 1, 8 and 32 series, program vs the equivalent `add_study` series.
//! Release build:
//!
//! ```sh
//! cargo run --release -p qp-plane --example tick_bench
//! ```

#![allow(
    clippy::unwrap_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::print_stdout,
    clippy::panic
)]

use std::time::Instant;

use qp_plane::values::{values, Cache, Delta};
use qp_schema::{Bar, Document, SeriesData};
use serde_json::{json, Value};

const T0: i64 = 1_704_067_200_000;
const HOUR: i64 = 3_600_000;
const SMA: &str = r#"{"version":"qp-ir/0","params":[{"key":"period","kind":"int","min":1,"max":2000,"default":20}],"nodes":[{"id":"_1","op":"input","field":"close"},{"id":"_2","op":"sma","x":"_1","period":{"param":"period"}}],"outputs":[{"name":"sma","kind":"series","node":"_2"}]}"#;
const BREAKOUT: &str = r#"{"version":"qp-ir/0","params":[{"key":"len","kind":"int","min":2,"max":500,"default":20}],"nodes":[{"id":"c","op":"input","field":"close"},{"id":"h","op":"input","field":"high"},{"id":"hi","op":"highest","x":"h","period":{"param":"len"}},{"id":"top","op":"lag","x":"hi","n":1},{"id":"up","op":"gt","a":"c","b":"top"},{"id":"ok","op":"defined","x":"top"},{"id":"k","op":"and","a":"up","b":"ok"}],"outputs":[{"name":"kirilim","kind":"event","node":"k"}]}"#;

fn bars(n: usize) -> Vec<Bar> {
    let mut s: u64 = 0x2545_F491_4F6C_DD1D;
    let mut c = 100.0;
    (0..n)
        .map(|i| {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            let u = (s >> 11) as f64 / (1u64 << 53) as f64;
            let o = c;
            c *= 1.0 + (u - 0.5) * 0.04;
            Bar {
                t: T0 + i as i64 * HOUR,
                o,
                h: o.max(c) * 1.004,
                l: o.min(c) * 0.996,
                c,
                v: 100.0,
            }
        })
        .collect()
}

fn program(id: &str, ir: &str, output: &str, params: &Value) -> Value {
    json!({"id": id, "interval": "1h",
        "source": {"kind": "study", "study": "program", "inputs": ["px"], "params": {},
            "output": output,
            "program": {"instance": id, "program_id": "sha256:0", "instance_id": "sha256:0",
                "scope": "document", "owner": "o", "since_t": 0, "params": params,
                "ir": serde_json::from_str::<Value>(ir).unwrap()}}})
}

/// `k` series of `kind` on a 500-bar `px`.
fn doc(b: &[Bar], kind: &str, k: usize) -> (Document, Vec<String>) {
    let mut series = vec![
        json!({"id": "px", "interval": "1h", "source": {"kind": "inline"},
        "data": {"kind": "bars", "bars": b}}),
    ];
    let mut ids = Vec::new();
    for i in 0..k {
        let id = format!("s{i}");
        let p = 10 + i as u64;
        series.push(match kind {
            "study sma" => json!({"id": id, "source": {"kind": "study", "study": "sma",
                "inputs": ["px"], "params": {"period": p}}}),
            "program sma" => program(&id, SMA, "sma", &json!({"period": p})),
            _ => program(&id, BREAKOUT, "kirilim", &json!({"len": p})),
        });
        ids.push(id);
    }
    let d = serde_json::from_value(json!({"schema_version": "0", "id": "d", "series": series}));
    (d.unwrap(), ids)
}

fn bars_mut(d: &mut Document) -> &mut Vec<Bar> {
    match &mut d.series[0].data {
        Some(SeriesData::Bars { bars }) => bars,
        _ => panic!("px has bars"),
    }
}

/// Median ns per tick of `ticks` revisions and of `ticks` new bars.
fn run(kind: &str, k: usize, ticks: usize) -> (f64, f64) {
    let (mut d, ids) = doc(&bars(500), kind, k);
    let mut cache = Cache::default();
    for id in &ids {
        values(&d, &mut cache, id).unwrap();
    }
    cache.apply_delta(&d, "px", Delta::Revised { index: 499 });
    let median = |mut v: Vec<f64>| {
        v.sort_by(f64::total_cmp);
        v[v.len() / 2]
    };
    let mut rev = Vec::with_capacity(ticks);
    for i in 0..ticks {
        let b = bars_mut(&mut d);
        let n = b.len();
        b[n - 1].c *= if i % 2 == 0 { 1.001 } else { 0.999 };
        cache.begin_tick();
        let t = Instant::now();
        cache.apply_delta(&d, "px", Delta::Revised { index: n - 1 });
        rev.push(t.elapsed().as_nanos() as f64);
    }
    let mut add = Vec::with_capacity(ticks);
    for _ in 0..ticks {
        let b = bars_mut(&mut d);
        let last = b[b.len() - 1].clone();
        b.push(Bar {
            t: last.t + HOUR,
            o: last.c,
            ..last
        });
        let from = b.len() - 1;
        cache.begin_tick();
        let t = Instant::now();
        cache.apply_delta(&d, "px", Delta::Appended { from });
        add.push(t.elapsed().as_nanos() as f64);
    }
    std::hint::black_box(cache.get("s0"));
    (median(rev), median(add))
}

fn main() {
    println!("| series | count | revision ns/tick | new bar ns/tick |");
    println!("|---|---:|---:|---:|");
    for kind in ["study sma", "program sma", "program breakout (event)"] {
        for k in [1, 8, 32] {
            let (r, a) = run(kind, k, 2000);
            println!("| {kind} | {k} | {r:.0} | {a:.0} |");
        }
    }
}
