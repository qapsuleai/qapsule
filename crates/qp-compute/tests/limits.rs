//! Period bounds (R1 audit finding 6, RD1 audit D3): every period-like parameter is an
//! integer in `1..=MAX_PERIOD`, in single runs, streaming studies and sweeps, and the
//! error says what is wrong.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::cognitive_complexity
)]

mod support;

use std::collections::BTreeMap;

use qp_compute::sweep::sweep;
use qp_compute::{create, Bollinger, Ema, Field, Macd, Rsi, Sma, MAX_PERIOD};

fn err(name: &str, kv: &[(&str, f64)]) -> String {
    match create(name, &support::params(kv), Field::Close) {
        Ok(_) => panic!("{name} {kv:?} accepted"),
        Err(e) => e.to_string(),
    }
}

/// (study, period key, parameters).
type Case = (&'static str, &'static str, Vec<(&'static str, f64)>);

/// Every (study, period key) pair with the other parameters at valid values.
fn cases(p: f64) -> Vec<Case> {
    vec![
        ("sma", "period", vec![("period", p)]),
        ("ema", "period", vec![("period", p)]),
        ("rsi", "period", vec![("period", p)]),
        ("bollinger", "period", vec![("period", p)]),
        (
            "macd",
            "fast",
            vec![("fast", p), ("slow", 3.0), ("signal", 2.0)],
        ),
        (
            "macd",
            "slow",
            vec![("fast", 2.0), ("slow", p), ("signal", 2.0)],
        ),
        (
            "macd",
            "signal",
            vec![("fast", 2.0), ("slow", 3.0), ("signal", p)],
        ),
    ]
}

#[test]
fn max_period_is_accepted_and_one_more_is_not() {
    let max = MAX_PERIOD as f64;
    for name in ["sma", "ema", "rsi", "bollinger"] {
        assert!(create(name, &support::params(&[("period", max)]), Field::Close).is_ok());
    }
    let macd = [("fast", max - 1.0), ("slow", max), ("signal", max)];
    assert!(create("macd", &support::params(&macd), Field::Close).is_ok());

    for (name, key, kv) in cases(max + 1.0) {
        assert_eq!(
            err(name, &kv),
            format!("parameter `{key}` is 100001, which exceeds the maximum period 100000"),
            "{name}.{key}"
        );
    }
}

#[test]
fn huge_period_message_names_the_bound() {
    // RD1 D3: was "parameter `period` must be an integer >= 1, got 1000000000000000000".
    for (name, key, kv) in cases(1e18) {
        assert_eq!(
            err(name, &kv),
            format!("parameter `{key}` is 1e18, which exceeds the maximum period 100000"),
            "{name}.{key}"
        );
    }
}

#[test]
fn non_integer_and_small_periods_name_the_range() {
    for p in [0.0, -5.0, 1.5, f64::NAN, f64::INFINITY] {
        let msg = err("sma", &[("period", p)]);
        assert!(
            msg.starts_with("parameter `period` must be an integer between 1 and 100000, got "),
            "{msg}"
        );
    }
    assert_eq!(
        err("sma", &[("period", 1.5)]),
        "parameter `period` must be an integer between 1 and 100000, got 1.5"
    );
}

#[test]
fn constructors_enforce_the_bound() {
    let over = MAX_PERIOD + 1;
    assert_eq!(
        Sma::new(over).unwrap_err().to_string(),
        "sma: period 100001 exceeds the maximum period 100000"
    );
    assert!(Ema::new(over).is_err());
    assert!(Rsi::new(over).is_err());
    assert!(Bollinger::new(over, 2.0).is_err());
    assert_eq!(
        Macd::new(2, over, 2).unwrap_err().to_string(),
        "macd: slow 100001 exceeds the maximum period 100000"
    );
    assert!(Macd::new(2, 3, over).is_err());
    assert_eq!(
        Sma::new(0).unwrap_err().to_string(),
        "sma: period must be >= 1"
    );
    assert!(Sma::new(MAX_PERIOD).is_ok());
}

#[test]
fn sweeps_reject_out_of_range_periods_before_work() {
    let x = [1.0, 2.0, 3.0];
    for (name, key, kv) in cases(1e9) {
        let good: BTreeMap<String, f64> = support::params(&cases(5.0)[0].2);
        let bad = support::params(&kv);
        let sets = if name == "macd" {
            vec![bad]
        } else {
            vec![good, bad]
        };
        let e = sweep(name, &sets, &x).unwrap_err().to_string();
        assert_eq!(
            e,
            format!("parameter `{key}` is 1e9, which exceeds the maximum period 100000"),
            "{name}.{key}"
        );
    }
}
