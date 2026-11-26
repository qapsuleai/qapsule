//! `add_study` equivalence (PR §14, VERIFY 11): each of the five studies, written as a
//! program, gives bit-equal values to `qp_compute::compute_bars` on the same bars, in batch,
//! streaming and "revise the last bar" modes.

use std::collections::BTreeMap;

use qp_compute::{compute_bars, create, Field};
use qp_lang::{ExecOptions, Executor};
use qp_schema::Bar;
use serde_json::{json, Value};

use crate::support::*;

fn int(key: &str, default: i64) -> Value {
    json!({"key": key, "kind": "int", "min": 1, "max": 2000, "default": default})
}

/// The PR §14 program of `study` (`_1` = close, `_2` the study node).
fn program(study: &str) -> Value {
    let close = json!({"id": "_1", "op": "input", "field": "close"});
    let series = |name: &str, node: &str| json!({"name": name, "kind": "series", "node": node});
    match study {
        "sma" | "ema" | "rsi" => prog_p(
            json!([int("period", if study == "rsi" { 14 } else { 20 })]),
            json!([close, {"id": "_2", "op": study, "x": "_1", "period": {"param": "period"}}]),
            json!([series(study, "_2")]),
        ),
        "macd" => prog_p(
            json!([int("fast", 12), int("slow", 26), int("signal", 9)]),
            json!([close, {"id": "m", "op": "macd", "x": "_1", "fast": {"param": "fast"},
                           "slow": {"param": "slow"}, "signal": {"param": "signal"}}]),
            json!([
                series("macd", "m.macd"),
                series("signal", "m.signal"),
                series("histogram", "m.histogram")
            ]),
        ),
        _ => prog_p(
            json!([int("period", 20),
                   {"key": "k", "kind": "float", "min": 0.0001, "max": 100.0, "default": 2.0}]),
            json!([close, {"id": "b", "op": "bollinger", "x": "_1", "period": {"param": "period"},
                           "k": {"param": "k"}}]),
            json!([
                series("upper", "b.upper"),
                series("middle", "b.middle"),
                series("lower", "b.lower"),
                series("bandwidth", "b.bandwidth")
            ]),
        ),
    }
}

/// `add_study`'s columns in streaming mode with revisions of every bar.
fn study_revised(
    name: &str,
    params: &BTreeMap<String, f64>,
    bars: &[Bar],
) -> Vec<Vec<Option<f64>>> {
    let mut s = create(name, params, Field::Close).unwrap();
    let n = s.outputs().len();
    let mut cols = vec![Vec::new(); n];
    let mut out = vec![None; n];
    for b in bars {
        s.push(&[b.c * 1.01], &mut out);
        s.update_last(&[b.c], &mut out);
        for (c, v) in cols.iter_mut().zip(&out) {
            c.push(*v);
        }
    }
    cols
}

fn check_study(name: &str, bound: &Value, params: BTreeMap<String, f64>) {
    let b = bars(700, 21);
    let reference = compute_bars(name, &params, Field::Close, &b).unwrap();
    let revised = study_revised(name, &params, &b);
    let c = checked_with(
        &program(name),
        "1h",
        Some(bound),
        qp_lang::Limits::default(),
    );
    let batch = run(&c, &b);
    let mut live = Executor::new(&c, ExecOptions::default()).unwrap();
    for bar in &b {
        live.push(Bar {
            c: bar.c * 0.99,
            ..bar.clone()
        })
        .unwrap();
        live.update_last(bar.clone()).unwrap();
    }
    live.close_live();
    let names: Vec<&str> = create(name, &params, Field::Close)
        .unwrap()
        .outputs()
        .to_vec();
    for (k, out) in names.iter().enumerate() {
        let want = bits(&reference.outputs[*out]);
        assert_eq!(bits(&values(&batch, out)), want, "{name}.{out} batch");
        assert_eq!(bits(&values(&live, out)), want, "{name}.{out} live");
        assert_eq!(bits(&revised[k]), want, "{name}.{out} add_study revised");
        assert!(
            want.iter().flatten().count() > 600,
            "{name}.{out} has values"
        );
    }
}

fn p(kv: &[(&str, f64)]) -> BTreeMap<String, f64> {
    kv.iter().map(|(k, v)| ((*k).to_owned(), *v)).collect()
}

#[test]
fn sma_is_bit_equal() {
    check_study("sma", &json!({}), p(&[("period", 20.0)]));
    check_study("sma", &json!({"period": 1}), p(&[("period", 1.0)]));
}

#[test]
fn ema_is_bit_equal() {
    check_study("ema", &json!({}), p(&[("period", 20.0)]));
    check_study("ema", &json!({"period": 50}), p(&[("period", 50.0)]));
}

#[test]
fn rsi_is_bit_equal() {
    check_study("rsi", &json!({}), p(&[("period", 14.0)]));
    check_study("rsi", &json!({"period": 3}), p(&[("period", 3.0)]));
}

#[test]
fn macd_is_bit_equal() {
    check_study(
        "macd",
        &json!({}),
        p(&[("fast", 12.0), ("slow", 26.0), ("signal", 9.0)]),
    );
    check_study(
        "macd",
        &json!({"fast": 3, "slow": 10, "signal": 4}),
        p(&[("fast", 3.0), ("slow", 10.0), ("signal", 4.0)]),
    );
}

#[test]
fn bollinger_is_bit_equal() {
    check_study("bollinger", &json!({}), p(&[("period", 20.0), ("k", 2.0)]));
    check_study(
        "bollinger",
        &json!({"period": 7, "k": 1.5}),
        p(&[("period", 7.0), ("k", 1.5)]),
    );
}
