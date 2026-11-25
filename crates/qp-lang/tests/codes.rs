//! One test per error sub-code and warning of PR §11.2–§11.3 (RL1 VERIFY 8): each
//! program has exactly the named finding, at its JSON pointer, with its closed code.

mod support;

use qp_lang::{
    check, check_live_budget, CheckCtx, Lang, Limit, Limits, LiveUsage, Purpose, Reason,
};
use qp_schema::protocol::ErrorCode;
use serde_json::json;
use support::*;

fn code(reason: &str) -> Option<ErrorCode> {
    Reason::parse(reason).and_then(Reason::code)
}

#[test]
fn e_parse_is_the_text_languages_and_maps_to_schema() {
    // A JSON value is already parsed; E_PARSE comes from the text language (PR §12.4).
    assert_eq!(code("E_PARSE"), Some(ErrorCode::Schema));
}

#[test]
fn e_version() {
    let mut p = close_series();
    p["version"] = json!("rx4-ir/0");
    let e = only(&chk(&p), "E_VERSION", "/version");
    assert_eq!(e.code, Some(ErrorCode::Schema));
    assert_eq!(e.expected, vec!["qp-ir/0"]);
}

#[test]
fn e_unknown_op_suggests_nearest_and_turkish() {
    let p = prog(
        json!([{"id": "c", "op": "input", "field": "close"},
               {"id": "r", "op": "rsii", "x": "c", "period": 14}]),
        json!([{"name": "r", "kind": "series", "node": "r"}]),
    );
    let e = only(&chk(&p), "E_UNKNOWN_OP", "/nodes/1/op");
    assert_eq!(e.did_you_mean[0], "rsi");
    assert!(e.expected.contains(&"percentrank".to_owned()));
    let p = sma(json!(3));
    let mut p2 = p.clone();
    p2["nodes"][1]["op"] = json!("ussel_ortalama");
    let e = only(&chk(&p2), "E_UNKNOWN_OP", "/nodes/1/op");
    assert_eq!(e.did_you_mean[0], "ema");
}

#[test]
fn e_unknown_field_and_bar_field() {
    let mut p = sma(json!(3));
    p["nodes"][1]["perod"] = json!(3);
    let e = only(&chk(&p), "E_UNKNOWN_FIELD", "/nodes/1/perod");
    assert_eq!(e.did_you_mean, vec!["period"]);
    let mut p = close_series();
    p["nodes"][0]["field"] = json!("clse");
    let e = only(&chk(&p), "E_UNKNOWN_FIELD", "/nodes/0/field");
    assert_eq!(e.did_you_mean[0], "close");
    let mut p = close_series();
    p["extra"] = json!(1);
    only(&chk(&p), "E_UNKNOWN_FIELD", "/extra");
}

#[test]
fn e_type() {
    // A num where `and` wants a bool.
    let p = prog(
        json!([{"id": "c", "op": "input", "field": "close"},
               {"id": "t", "op": "const", "value": true},
               {"id": "a", "op": "and", "a": "c", "b": "t"}]),
        json!([{"name": "e", "kind": "event", "node": "a"}]),
    );
    let e = only(&chk(&p), "E_TYPE", "/nodes/2/a");
    assert_eq!(e.code, Some(ErrorCode::Schema));
    // A float parameter in an integer field.
    let p = prog_p(
        json!([{"key": "len", "kind": "float", "min": 1, "max": 50, "default": 20}]),
        sma(json!({"param": "len"}))["nodes"].clone(),
        json!([{"name": "s", "kind": "series", "node": "s"}]),
    );
    only(&chk(&p), "E_TYPE", "/nodes/1/period");
    // `20.0` for an int parameter's default (PR §2.1).
    let p = prog_p(
        json!([{"key": "len", "kind": "int", "min": 1, "max": 50, "default": 20.0}]),
        close_series()["nodes"].clone(),
        close_series()["outputs"].clone(),
    );
    only(&chk(&p), "E_TYPE", "/params/0/default");
    // A series output on a bool node.
    let p = prog(
        json!([{"id": "t", "op": "const", "value": true}]),
        json!([{"name": "s", "kind": "series", "node": "t"}]),
    );
    only(&chk(&p), "E_TYPE", "/outputs/0/node");
}

#[test]
fn e_missing_arg() {
    let p = prog(
        json!([{"id": "c", "op": "input", "field": "close"},
               {"id": "s", "op": "sma", "x": "c"}]),
        json!([{"name": "s", "kind": "series", "node": "s"}]),
    );
    let e = only(&chk(&p), "E_MISSING_ARG", "/nodes/1");
    assert_eq!(e.expected, vec!["period"]);
}

#[test]
fn e_unknown_tf() {
    let up = |tf: &str| {
        prog(
            json!([{"id": "d", "op": "input", "field": "close", "tf": tf},
                   {"id": "a", "op": "align", "x": "d"}]),
            json!([{"name": "a", "kind": "series", "node": "a"}]),
        )
    };
    assert!(chk_bound(&up("4h"), "1h").ok());
    assert!(chk_bound(&up("1d"), "1h").ok());
    only(&chk_bound(&up("90m"), "1h"), "E_UNKNOWN_TF", "/nodes/0/tf");
    only(&chk_bound(&up("30m"), "1h"), "E_UNKNOWN_TF", "/nodes/0/tf");
    only(&chk_bound(&up("4h"), "1d"), "E_UNKNOWN_TF", "/nodes/0/tf");
    let e = only(&chk_bound(&up("1h"), "1h"), "E_UNKNOWN_TF", "/nodes/0/tf");
    assert!(e.hint.is_some());
    let e = only(&chk(&up("60m")), "E_UNKNOWN_TF", "/nodes/0/tf");
    assert_eq!(e.did_you_mean, vec!["1h"]);
    only(&chk(&up("3x")), "E_UNKNOWN_TF", "/nodes/0/tf");
    // A bound series without an interval and a higher timeframe.
    let ctx = CheckCtx {
        binding: Some(qp_lang::Binding {
            interval: None,
            bars: None,
        }),
        ..CheckCtx::default()
    };
    let r = check(&up("4h"), &ctx);
    only(&r, "E_UNKNOWN_TF", "/nodes/0/tf");
}

#[test]
fn e_clock_mismatch() {
    let p = prog(
        json!([{"id": "c", "op": "input", "field": "close"},
               {"id": "h", "op": "input", "field": "close", "tf": "4h"},
               {"id": "s", "op": "sub", "a": "c", "b": "h"}]),
        json!([{"name": "s", "kind": "series", "node": "s"}]),
    );
    let e = only(&chk_bound(&p, "1h"), "E_CLOCK_MISMATCH", "/nodes/2/b");
    assert_eq!(e.expected, vec!["base"]);
    // `align` of a base input.
    let p = prog(
        json!([{"id": "c", "op": "input", "field": "close"},
               {"id": "a", "op": "align", "x": "c"}]),
        json!([{"name": "a", "kind": "series", "node": "a"}]),
    );
    only(&chk(&p), "E_CLOCK_MISMATCH", "/nodes/1/x");
}

#[test]
fn e_output_clock() {
    let p = prog(
        json!([{"id": "h", "op": "input", "field": "close", "tf": "1d"},
               {"id": "m", "op": "sma", "x": "h", "period": 10}]),
        json!([{"name": "m", "kind": "series", "node": "m"}]),
    );
    only(&chk_bound(&p, "1h"), "E_OUTPUT_CLOCK", "/outputs/0/node");
}

#[test]
fn e_no_output() {
    let mut p = close_series();
    p["outputs"] = json!([]);
    only(&chk(&p), "E_NO_OUTPUT", "/outputs");
    let mut p = close_series();
    p.as_object_mut().map(|o| o.remove("outputs"));
    only(&chk(&p), "E_NO_OUTPUT", "");
}

#[test]
fn e_unknown_ref() {
    let mut p = sma(json!(3));
    p["nodes"][1]["x"] = json!("cc");
    let e = only(&chk(&p), "E_UNKNOWN_REF", "/nodes/1/x");
    assert_eq!(e.code, Some(ErrorCode::NotFound));
    assert_eq!(e.did_you_mean, vec!["c"]);
    // A later node.
    let p = prog(
        json!([{"id": "s", "op": "sma", "x": "c", "period": 3},
               {"id": "c", "op": "input", "field": "close"}]),
        json!([{"name": "s", "kind": "series", "node": "s"}]),
    );
    only(&chk(&p), "E_UNKNOWN_REF", "/nodes/0/x");
    // A missing output of a multi-output node.
    let p = prog(
        json!([{"id": "c", "op": "input", "field": "close"},
               {"id": "m", "op": "macd", "x": "c", "fast": 12, "slow": 26, "signal": 9}]),
        json!([{"name": "s", "kind": "series", "node": "m.signl"}]),
    );
    let e = only(&chk(&p), "E_UNKNOWN_REF", "/outputs/0/node");
    assert_eq!(e.did_you_mean, vec!["signal"]);
}

#[test]
fn e_unknown_param() {
    let p = prog_p(
        json!([int_param("len", 1, 50, 20)]),
        sma(json!({"param": "lenn"}))["nodes"].clone(),
        json!([{"name": "s", "kind": "series", "node": "s"}]),
    );
    let e = only(&chk(&p), "E_UNKNOWN_PARAM", "/nodes/1/period");
    assert_eq!(e.code, Some(ErrorCode::NotFound));
    assert_eq!(e.did_you_mean, vec!["len"]);
    // A bound value for an undefined key: no pointer (outside the program).
    let p = prog_p(
        json!([int_param("len", 1, 50, 20)]),
        sma(json!({"param": "len"}))["nodes"].clone(),
        json!([{"name": "s", "kind": "series", "node": "s"}]),
    );
    let r = chk_params(&p, &json!({"lne": 3}));
    assert_eq!(
        errs(&r),
        vec![("E_UNKNOWN_PARAM".to_owned(), String::new())]
    );
    assert_eq!(r.errors[0].at, None);
    assert_eq!(r.errors[0].did_you_mean, vec!["len"]);
}

#[test]
fn e_duplicate_name() {
    let p = prog(
        json!([{"id": "c", "op": "input", "field": "close"},
               {"id": "c", "op": "input", "field": "open"}]),
        json!([{"name": "c", "kind": "series", "node": "c"},
               {"name": "c", "kind": "series", "node": "c"}]),
    );
    let r = chk(&p);
    assert_eq!(
        errs(&r),
        vec![
            ("E_DUPLICATE_NAME".to_owned(), "/nodes/1/id".to_owned()),
            ("E_DUPLICATE_NAME".to_owned(), "/outputs/1/name".to_owned()),
        ]
    );
    assert_eq!(r.errors[0].code, Some(ErrorCode::Duplicate));
    let p = prog_p(
        json!([int_param("len", 1, 50, 20), int_param("len", 1, 50, 20)]),
        close_series()["nodes"].clone(),
        close_series()["outputs"].clone(),
    );
    only(&chk(&p), "E_DUPLICATE_NAME", "/params/1/key");
}

#[test]
fn e_bad_param() {
    only(&chk(&sma(json!(0))), "E_BAD_PARAM", "/nodes/1/period");
    // The int field given 20.0 (PR §4.2: fractional or 0 is E_BAD_PARAM).
    let e = only(&chk(&sma(json!(20.0))), "E_BAD_PARAM", "/nodes/1/period");
    only(&chk(&sma(json!(2.5))), "E_BAD_PARAM", "/nodes/1/period");
    assert_eq!(e.code, Some(ErrorCode::Invalid));
    let bucket = |edges: serde_json::Value| {
        prog(
            json!([{"id": "c", "op": "input", "field": "close"},
                   {"id": "b", "op": "bucket", "x": "c", "edges": edges}]),
            json!([{"name": "st", "kind": "state", "dims": [{"name": "lvl", "node": "b"}]}]),
        )
    };
    assert!(chk(&bucket(json!([1, 2, 3]))).ok());
    only(
        &chk(&bucket(json!([2, 1]))),
        "E_BAD_PARAM",
        "/nodes/1/edges",
    );
    only(&chk(&bucket(json!([]))), "E_BAD_PARAM", "/nodes/1/edges");
    let line = prog(
        json!([{"id": "l", "op": "line", "a": {"t": 2000, "price": 10}, "b": {"t": 1000, "price": 11}}]),
        json!([{"name": "l", "kind": "series", "node": "l"}]),
    );
    only(&chk(&line), "E_BAD_PARAM", "/nodes/0/b");
    let mut p = close_series();
    p["nodes"][0]["id"] = json!("9c");
    p["outputs"][0]["node"] = json!("9c");
    only(&chk(&p), "E_BAD_PARAM", "/nodes/0/id");
}

#[test]
fn e_param_range() {
    // The definition's range exceeds the field's (`period` 1 … max_window).
    let p = prog_p(
        json!([int_param("len", 1, 5000, 20)]),
        sma(json!({"param": "len"}))["nodes"].clone(),
        json!([{"name": "s", "kind": "series", "node": "s"}]),
    );
    let e = only(&chk(&p), "E_PARAM_RANGE", "/nodes/1/period");
    assert_eq!(e.code, Some(ErrorCode::Invalid));
    // A bound value outside the definition.
    let p = prog_p(
        json!([int_param("len", 1, 50, 20)]),
        sma(json!({"param": "len"}))["nodes"].clone(),
        json!([{"name": "s", "kind": "series", "node": "s"}]),
    );
    let r = chk_params(&p, &json!({"len": 51}));
    assert_eq!(errs(&r), vec![("E_PARAM_RANGE".to_owned(), String::new())]);
    // A default outside the definition.
    let p = prog_p(
        json!([int_param("len", 1, 50, 60)]),
        close_series()["nodes"].clone(),
        close_series()["outputs"].clone(),
    );
    only(&chk(&p), "E_PARAM_RANGE", "/params/0/default");
}

fn signal(rule: serde_json::Value, side: &str) -> serde_json::Value {
    let mut out = json!({"name": "g", "kind": "signal", "node": "x", "side": side,
                         "source": {"strategy": "s", "version": "1"}});
    if !rule.is_null() {
        out["rule"] = rule;
    }
    prog(
        json!([{"id": "c", "op": "input", "field": "close"},
               {"id": "k", "op": "const", "value": 100},
               {"id": "x", "op": "cross_above", "a": "c", "b": "k"}]),
        json!([out]),
    )
}

#[test]
fn e_no_outcome_rule() {
    let e = only(
        &chk(&signal(json!(null), "al")),
        "E_NO_OUTCOME_RULE",
        "/outputs/0",
    );
    assert_eq!(e.code, Some(ErrorCode::Invalid));
    only(
        &chk(&signal(json!({"target_pct": 3}), "al")),
        "E_NO_OUTCOME_RULE",
        "/outputs/0/rule",
    );
    assert!(chk(&signal(json!({"max_bars": 24}), "sat")).ok());
}

#[test]
fn e_bad_rule() {
    only(
        &chk(&signal(json!({"target_pct": -1, "max_bars": 24}), "al")),
        "E_BAD_RULE",
        "/outputs/0/rule/target_pct",
    );
    let e = only(
        &chk(&signal(json!({"max_bars": 1000}), "al")),
        "E_BAD_RULE",
        "/outputs/0/rule/max_bars",
    );
    assert_eq!(e.expected, vec!["1..=500"]);
}

#[test]
fn session_level_sub_codes_map_to_their_codes() {
    // Raised by `qp-program` (PS §1.1, §1.2, §3.1), not by the IR check.
    assert_eq!(code("E_NOT_BAR_SERIES"), Some(ErrorCode::Invalid));
    assert_eq!(code("E_PROGRAM_SERIES"), Some(ErrorCode::Invalid));
    assert_eq!(code("E_EVENT_OPERAND"), Some(ErrorCode::Unsupported));
    let e = Reason::EventOperand.error(Some("/ops/0".to_owned()), "m".to_owned());
    assert_eq!(
        (e.reason.as_str(), e.code),
        ("E_EVENT_OPERAND", Some(ErrorCode::Unsupported))
    );
}

#[test]
fn e_try_only() {
    let ctx = CheckCtx {
        purpose: Purpose::Document,
        ..CheckCtx::default()
    };
    let r = check(&signal(json!({"max_bars": 24}), "al"), &ctx);
    let e = only(&r, "E_TRY_ONLY", "/outputs/0/kind");
    assert_eq!(e.code, Some(ErrorCode::Unsupported));
    assert!(chk(&signal(json!({"max_bars": 24}), "al")).ok());
}

#[test]
fn e_window_limit() {
    let e = only(&chk(&sma(json!(5000))), "E_WINDOW_LIMIT", "/nodes/1/period");
    assert_eq!(
        (e.code, e.expected),
        (Some(ErrorCode::Limit), vec!["2000".to_owned()])
    );
}

#[test]
fn e_too_many_nodes() {
    let l = Limits::default().with(Limit::MaxNodes, 1);
    let e = only(&chk_limits(&sma(json!(3)), l), "E_TOO_MANY_NODES", "/nodes");
    assert_eq!(e.expected, vec!["1"]);
}

#[test]
fn e_too_large() {
    let l = Limits::default().with(Limit::MaxOutputs, 1);
    let mut p = close_series();
    p["outputs"] = json!([{"name": "a", "kind": "series", "node": "c"},
                          {"name": "b", "kind": "series", "node": "c"}]);
    only(&chk_limits(&p, l), "E_TOO_LARGE", "/outputs");
    let l = Limits::default().with(Limit::MaxParams, 0);
    let p = prog_p(
        json!([int_param("len", 1, 50, 20)]),
        close_series()["nodes"].clone(),
        close_series()["outputs"].clone(),
    );
    let r = chk_limits(&p, l);
    assert_eq!(
        errs(&r)[0],
        ("E_TOO_LARGE".to_owned(), "/params".to_owned())
    );
    let l = Limits::default().with(Limit::MaxIrBytes, 64);
    only(&chk_limits(&sma(json!(3)), l), "E_TOO_LARGE", "");
}

#[test]
fn e_cost_limit() {
    let mut p = sma(json!(3));
    p["nodes"][1]["op"] = json!("stdev");
    p["nodes"][1]["period"] = json!(700);
    let r = chk(&p);
    let e = only(&r, "E_COST_LIMIT", "");
    assert_eq!(
        (e.code, e.expected),
        (Some(ErrorCode::Limit), vec!["666".to_owned()])
    );
    assert!(e.hint.is_some());
    assert_eq!(r.cost.map(|c| c.per_bar), Some(701));
}

#[test]
fn e_budget() {
    let limits = Limits::default();
    let fine = LiveUsage {
        instances: 32,
        cost_per_bar: 2000,
        log_bytes: 10,
    };
    assert!(check_live_budget(&fine, &limits, Lang::En).is_empty());
    let over = LiveUsage {
        instances: 33,
        cost_per_bar: 2001,
        log_bytes: 10,
    };
    let e = check_live_budget(&over, &limits, Lang::Tr);
    assert_eq!(e.len(), 2);
    assert!(e
        .iter()
        .all(|e| e.reason == "E_BUDGET" && e.code == Some(ErrorCode::Limit)));
    assert_eq!(e[0].expected, vec!["32"]);
}

#[test]
fn e_lookahead() {
    let p = prog(
        json!([{"id": "c", "op": "input", "field": "close"},
               {"id": "l", "op": "lag", "x": "c", "n": -1}]),
        json!([{"name": "l", "kind": "series", "node": "l"}]),
    );
    let e = only(&chk(&p), "E_LOOKAHEAD", "/nodes/1/n");
    assert_eq!(e.code, Some(ErrorCode::Unsupported));
    let mut p = close_series();
    p["nodes"][0]["partial"] = json!(true);
    only(&chk(&p), "E_LOOKAHEAD", "/nodes/0/partial");
}

#[test]
fn e_unsupported() {
    let e = only(
        &chk(&signal(json!({"max_bars": 24}), "tut")),
        "E_UNSUPPORTED",
        "/outputs/0/side",
    );
    assert_eq!(e.code, Some(ErrorCode::Unsupported));
    assert_eq!(e.expected, vec!["al", "sat"]);
    only(
        &chk(&signal(json!({"kind": "breakout", "max_bars": 24}), "al")),
        "E_UNSUPPORTED",
        "/outputs/0/rule/kind",
    );
}

#[test]
fn w_consecutive_events_and_edge_shapes() {
    let ev = |node: serde_json::Value| {
        prog(
            json!([{"id": "c", "op": "input", "field": "close"},
                   {"id": "k", "op": "const", "value": 100},
                   {"id": "g", "op": "gt", "a": "c", "b": "k"},
                   {"id": "x", "op": "cross_above", "a": "c", "b": "k"},
                   {"id": "y", "op": "and", "a": "g", "b": "x"},
                   {"id": "z", "op": "cooldown", "trigger": "g", "bars": 0}]),
            json!([{"name": "e", "kind": "event", "node": node},
                   {"name": "u", "kind": "event", "node": "z"}]),
        )
    };
    let r = chk(&ev(json!("g")));
    assert!(r.ok());
    let w: Vec<_> = r
        .warnings
        .iter()
        .map(|w| (w.reason.as_str(), w.at.as_deref().unwrap_or("")))
        .collect();
    assert!(
        w.contains(&("W_CONSECUTIVE_EVENTS", "/outputs/0/node")),
        "{w:?}"
    );
    assert!(
        w.contains(&("W_CONSECUTIVE_EVENTS", "/outputs/1/node")),
        "{w:?}"
    );
    assert!(r.warnings.iter().all(|w| w.code.is_none()));
    let r = chk(&ev(json!("y")));
    assert!(!r
        .warnings
        .iter()
        .any(|w| w.at.as_deref() == Some("/outputs/0/node")));
}

#[test]
fn w_unused_node() {
    let mut p = sma(json!(3));
    p["outputs"][0]["node"] = json!("c");
    let r = chk(&p);
    assert!(r.ok());
    let w: Vec<_> = r
        .warnings
        .iter()
        .map(|w| (w.reason.as_str(), w.at.as_deref().unwrap_or("")))
        .collect();
    assert_eq!(w, vec![("W_UNUSED_NODE", "/nodes/1")]);
    // Its cost still counts.
    assert_eq!(r.cost.map(|c| (c.per_bar, c.live_per_bar)), Some((3, 1)));
}

#[test]
fn every_reason_has_a_code_or_is_a_warning() {
    for r in Reason::ALL {
        assert_eq!(
            r.code().is_none(),
            r.as_str().starts_with("W_"),
            "{}",
            r.as_str()
        );
        assert_eq!(Reason::parse(r.as_str()), Some(r));
    }
}
