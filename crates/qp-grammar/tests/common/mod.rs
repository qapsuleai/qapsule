//! Shared test fixtures: the R0 sample bars and a provider whose study values come from
//! `qp-compute` (tests only; the crate itself never computes).
#![allow(dead_code)]

use std::collections::BTreeMap;

use qp_compute::Field;
use qp_grammar::MemProvider;
use qp_schema::grammar::View;
use qp_schema::{Bar, Series};

pub fn sample_series() -> Series {
    let json = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/bars/sample.json"
    ))
    .expect("sample bars");
    qp_schema::parse_bar_series(&json).expect("valid sample")
}

pub fn sample_bars() -> Vec<Bar> {
    sample_series().bars().expect("bars").to_vec()
}

/// Study outputs from the compute core, in output order.
pub fn study(
    name: &str,
    params: &[(&str, f64)],
    bars: &[Bar],
) -> BTreeMap<String, Vec<Option<f64>>> {
    let params: BTreeMap<String, f64> = params.iter().map(|(k, v)| ((*k).to_owned(), *v)).collect();
    qp_compute::compute_bars(name, &params, Field::Close, bars)
        .expect("study")
        .outputs
}

#[allow(clippy::too_many_arguments)]
fn add(
    p: MemProvider,
    id: &str,
    input: &str,
    out: BTreeMap<String, Vec<Option<f64>>>,
    order: &[&str],
    main: &str,
    plot: &[&str],
    label: &str,
) -> MemProvider {
    let fields: Vec<(&str, Vec<Option<f64>>)> = order
        .iter()
        .map(|f| {
            (
                *f,
                out.get(*f).cloned().unwrap_or_else(|| panic!("output {f}")),
            )
        })
        .collect();
    p.with_values(id, input, fields, main, plot)
        .with_label(id, label)
}

/// `px` (sample bars) with `sma20`, `sma50`, `rsi14`, `macd`, `bb`.
pub fn provider_with(bars: Vec<Bar>, symbol: &str) -> MemProvider {
    // The v0 named thresholds (docs/GRAMMAR.md §10); the authority supplies the real list.
    let mut p = MemProvider::new()
        .with_threshold("rsi.overbought", 70.0)
        .with_threshold("rsi.oversold", 30.0)
        .with_threshold("rsi.mid", 50.0)
        .with_threshold("macd.zero", 0.0)
        .with_bars("px", bars.clone(), Some(symbol), Some("1d"));
    p = add(
        p,
        "sma20",
        "px",
        study("sma", &[("period", 20.0)], &bars),
        &["sma"],
        "sma",
        &["sma"],
        "SMA20",
    );
    p = add(
        p,
        "sma50",
        "px",
        study("sma", &[("period", 50.0)], &bars),
        &["sma"],
        "sma",
        &["sma"],
        "SMA50",
    );
    p = add(
        p,
        "rsi14",
        "px",
        study("rsi", &[("period", 14.0)], &bars),
        &["rsi"],
        "rsi",
        &["rsi"],
        "RSI14",
    );
    p = add(
        p,
        "macd",
        "px",
        study(
            "macd",
            &[("fast", 12.0), ("slow", 26.0), ("signal", 9.0)],
            &bars,
        ),
        &["macd", "signal", "histogram"],
        "macd",
        &["macd", "signal"],
        "MACD(12,26,9)",
    );
    p = add(
        p,
        "bb",
        "px",
        study("bollinger", &[("period", 20.0), ("k", 2.0)], &bars),
        &["upper", "middle", "lower", "bandwidth"],
        "middle",
        &["upper", "middle", "lower"],
        "BB(20,2)",
    );
    p
}

pub fn provider() -> MemProvider {
    provider_with(sample_bars(), "SAMPLE")
}

pub fn view(json: &str) -> View {
    serde_json::from_str(json).expect("view json")
}

/// Synthetic bars growing exponentially from 10 to 400 (the `growth` document, V4).
pub fn growth_bars(n: usize) -> Vec<Bar> {
    let t0 = 1_735_776_000_000_i64;
    let mut v = 10.0_f64;
    // 40^(1/(n-1)) by repeated multiplication of a fixed ratio found by bisection.
    let (mut lo, mut hi) = (1.0_f64, 2.0_f64);
    for _ in 0..80 {
        let mid = (lo + hi) / 2.0;
        let mut x = 1.0;
        for _ in 1..n {
            x *= mid;
        }
        if x < 40.0 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    (0..n)
        .map(|i| {
            let o = v;
            let c = v * lo;
            let b = Bar {
                t: t0 + i as i64 * 86_400_000,
                o,
                h: c * 1.01,
                l: o * 0.99,
                c,
                v: 1000.0,
            };
            v = c;
            b
        })
        .collect()
}
