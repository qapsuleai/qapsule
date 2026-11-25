//! Program parameters (PR §2.1; GOAL acceptance 9): definition, binding, the two-level
//! range check, two instances of one program.
#![allow(clippy::panic)]

mod support;

use qp_lang::{Op, WindowKind};
use qp_schema::program::ProgramParamValue;
use serde_json::json;
use support::*;

fn sma_len() -> serde_json::Value {
    prog_p(
        json!([int_param("len", 2, 500, 20)]),
        sma(json!({"param": "len"}))["nodes"].clone(),
        json!([{"name": "s", "kind": "series", "node": "s"}]),
    )
}

fn period_of(r: &qp_lang::CheckReport) -> Option<usize> {
    match r.checked.as_ref()?.nodes[1].op {
        Op::Window {
            kind: WindowKind::Sma,
            period,
            ..
        } => Some(period),
        _ => None,
    }
}

#[test]
fn definition_errors() {
    let with = |def: serde_json::Value| {
        prog_p(
            json!([def]),
            close_series()["nodes"].clone(),
            close_series()["outputs"].clone(),
        )
    };
    only(
        &chk(&with(
            json!({"key": "len", "kind": "int", "max": 5, "default": 3}),
        )),
        "E_MISSING_ARG",
        "/params/0",
    );
    let r = chk(&with(json!({"key": "len", "kind": "int", "default": 3})));
    assert_eq!(
        r.errors.len(),
        2,
        "min and max both missing: {:#?}",
        r.errors
    );
    only(
        &chk(&with(
            json!({"key": "on", "kind": "bool", "min": 0, "default": true}),
        )),
        "E_BAD_PARAM",
        "/params/0/min",
    );
    only(
        &chk(&with(
            json!({"key": "len", "kind": "int", "min": 9, "max": 3, "default": 3}),
        )),
        "E_PARAM_RANGE",
        "/params/0/min",
    );
    only(
        &chk(&with(
            json!({"key": "Len", "kind": "int", "min": 1, "max": 3, "default": 3}),
        )),
        "E_BAD_PARAM",
        "/params/0/key",
    );
    only(
        &chk(&with(json!({"key": "t", "kind": "text", "default": "a"}))),
        "E_TYPE",
        "/params/0/kind",
    );
    only(
        &chk(&with(json!({"key": "on", "kind": "bool", "default": 1}))),
        "E_TYPE",
        "/params/0/default",
    );
    only(
        &chk(&with(
            json!({"key": "on", "kind": "bool", "default": true, "dflt": 1}),
        )),
        "E_UNKNOWN_FIELD",
        "/params/0/dflt",
    );
    let ok = chk(&with(
        json!({"key": "on", "kind": "bool", "default": true, "unit": "x", "description": "d"}),
    ));
    assert!(ok.ok(), "{:#?}", ok.errors);
    assert_eq!(ok.params.len(), 1);
}

#[test]
fn binding_fills_defaults_and_resolves_values() {
    let r = chk(&sma_len());
    assert_eq!(period_of(&r), Some(20));
    let params = r
        .checked
        .as_ref()
        .map(|c| c.params.clone())
        .unwrap_or_default();
    assert_eq!(params.get("len"), Some(&ProgramParamValue::Int(20)));
    let r = chk_params(&sma_len(), &json!({"len": 30}));
    assert_eq!(period_of(&r), Some(30));
}

#[test]
fn binding_errors() {
    only(&chk_params(&sma_len(), &json!({"len": 30.0})), "E_TYPE", "");
    only(
        &chk_params(&sma_len(), &json!({"len": 1})),
        "E_PARAM_RANGE",
        "",
    );
    only(
        &chk_params(&sma_len(), &json!({"len": 501})),
        "E_PARAM_RANGE",
        "",
    );
    only(&chk_params(&sma_len(), &json!([1])), "E_TYPE", "");
    // A float parameter takes an integer-written value as a float.
    let p = prog_p(
        json!([{"key": "k", "kind": "float", "min": 0.5, "max": 3, "default": 2}]),
        json!([{"id": "c", "op": "input", "field": "close"},
               {"id": "b", "op": "bollinger", "x": "c", "period": 20, "k": {"param": "k"}}]),
        json!([{"name": "m", "kind": "series", "node": "b"}]),
    );
    let r = chk_params(&p, &json!({"k": 3}));
    assert!(r.ok(), "{:#?}", r.errors);
    let c = r.checked.unwrap_or_else(|| panic!("checked"));
    assert_eq!(c.params.get("k"), Some(&ProgramParamValue::Float(3.0)));
    assert!(matches!(c.nodes[1].op, Op::Bollinger { k, .. } if k == 3.0));
}

#[test]
fn static_range_of_the_definition_against_the_field() {
    // `lag.n` accepts 0, `period` does not.
    let lag = prog_p(
        json!([int_param("n", 0, 10, 1)]),
        json!([{"id": "c", "op": "input", "field": "close"},
               {"id": "l", "op": "lag", "x": "c", "n": {"param": "n"}}]),
        json!([{"name": "l", "kind": "series", "node": "l"}]),
    );
    assert!(chk(&lag).ok());
    let per = prog_p(
        json!([int_param("n", 0, 10, 1)]),
        sma(json!({"param": "n"}))["nodes"].clone(),
        json!([{"name": "s", "kind": "series", "node": "s"}]),
    );
    only(&chk(&per), "E_PARAM_RANGE", "/nodes/1/period");
    // `k` must stay > 0 over the whole range.
    let k = prog_p(
        json!([{"key": "k", "kind": "float", "min": 0, "max": 3, "default": 2}]),
        json!([{"id": "c", "op": "input", "field": "close"},
               {"id": "b", "op": "bollinger", "x": "c", "period": 20, "k": {"param": "k"}}]),
        json!([{"name": "m", "kind": "series", "node": "b"}]),
    );
    only(&chk(&k), "E_PARAM_RANGE", "/nodes/1/k");
}

#[test]
fn bound_values_are_checked_where_order_matters() {
    let macd = prog_p(
        json!([
            int_param("fast", 1, 2000, 12),
            int_param("slow", 1, 2000, 26)
        ]),
        json!([{"id": "c", "op": "input", "field": "close"},
               {"id": "m", "op": "macd", "x": "c", "fast": {"param": "fast"},
                "slow": {"param": "slow"}, "signal": 9}]),
        json!([{"name": "macd", "kind": "series", "node": "m"}]),
    );
    assert!(chk(&macd).ok());
    only(
        &chk_params(&macd, &json!({"fast": 30})),
        "E_BAD_PARAM",
        "/nodes/1/slow",
    );
    let bucket = prog_p(
        json!([{"key": "lo", "kind": "float", "min": 0, "max": 100, "default": 30},
               {"key": "hi", "kind": "float", "min": 0, "max": 100, "default": 70}]),
        json!([{"id": "c", "op": "input", "field": "close"},
               {"id": "b", "op": "bucket", "x": "c", "edges": [{"param": "lo"}, {"param": "hi"}]}]),
        json!([{"name": "st", "kind": "state", "dims": [{"name": "zone", "node": "b"}]}]),
    );
    assert!(chk(&bucket).ok());
    only(
        &chk_params(&bucket, &json!({"lo": 80})),
        "E_BAD_PARAM",
        "/nodes/1/edges",
    );
}

#[test]
fn two_instances_of_one_program() {
    let a = chk_params(&sma_len(), &json!({"len": 10}));
    let b = chk_params(&sma_len(), &json!({"len": 30}));
    assert!(a.ok() && b.ok());
    assert_eq!(a.program_id, b.program_id);
    assert_ne!(a.instance_id, b.instance_id);
    // Defaults fill the set: no values and the default value are one instance.
    let c = chk(&sma_len());
    let d = chk_params(&sma_len(), &json!({"len": 20}));
    assert_eq!(c.instance_id, d.instance_id);
}

#[test]
fn constants_and_rules_take_parameters() {
    let p = prog_p(
        json!([{"key": "lvl", "kind": "float", "min": 0, "max": 1000, "default": 100},
               {"key": "on", "kind": "bool", "default": true},
               {"key": "tp", "kind": "float", "min": 0.5, "max": 10, "default": 3},
               int_param("hz", 1, 50, 24)]),
        json!([{"id": "c", "op": "input", "field": "close"},
               {"id": "k", "op": "const", "value": {"param": "lvl"}},
               {"id": "o", "op": "const", "value": {"param": "on"}},
               {"id": "x", "op": "cross_above", "a": "c", "b": "k"},
               {"id": "y", "op": "and", "a": "x", "b": "o"}]),
        json!([{"name": "g", "kind": "signal", "node": "y", "side": "al",
                "rule": {"target_pct": {"param": "tp"}, "max_bars": {"param": "hz"}},
                "source": {"strategy": "s", "version": "1"}}]),
    );
    let r = chk(&p);
    assert!(r.ok(), "{:#?}", r.errors);
    let c = r.checked.unwrap_or_else(|| panic!("checked"));
    assert!(matches!(
        c.nodes[2].op,
        Op::Const(qp_lang::Literal::Bool(true))
    ));
    match &c.outputs[0].spec {
        qp_lang::OutputSpec::Signal { rule, .. } => {
            assert_eq!(
                (rule.target_pct, rule.stop_pct, rule.max_bars),
                (Some(3.0), None, 24)
            );
        }
        other => panic!("{other:?}"),
    }
}
