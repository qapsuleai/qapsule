//! Every error at once (PR §11.1; GOAL acceptance 4): positions, order, close names,
//! silence downstream of a broken node, truncation, languages, odd JSON.

mod support;

use qp_lang::{check, CheckCtx, Lang, Limit, Limits};
use serde_json::json;
use support::*;

fn many_errors() -> serde_json::Value {
    json!({
        "version": "qp-ir/0",
        "params": [{"key": "len", "kind": "int", "min": 1, "max": 50, "default": 20}],
        "nodes": [
            {"id": "c", "op": "input", "field": "clsoe"},
            {"id": "r", "op": "rsii", "x": "c", "period": 14},
            {"id": "s", "op": "sma", "x": "c", "period": {"param": "lne"}},
            {"id": "d", "op": "sub", "a": "r", "b": "s"},
            {"id": "l", "op": "lag", "x": "q", "n": -2},
            {"id": "e", "op": "ema", "x": "c", "perod": 3}
        ],
        "outputs": [
            {"name": "out", "kind": "series", "node": "d"},
            {"name": "out", "kind": "fingerprint", "node": "d"},
            {"name": "g", "kind": "signal", "node": "x", "side": "tut",
             "source": {"strategy": "s", "version": "1"}}
        ]
    })
}

#[test]
fn every_error_with_position_and_suggestion() {
    let r = chk(&many_errors());
    assert!(!r.ok());
    let got = errs(&r);
    let want: Vec<(String, String)> = [
        ("E_UNKNOWN_FIELD", "/nodes/0/field"),
        ("E_UNKNOWN_OP", "/nodes/1/op"),
        ("E_UNKNOWN_PARAM", "/nodes/2/period"),
        ("E_UNKNOWN_REF", "/nodes/4/x"),
        ("E_LOOKAHEAD", "/nodes/4/n"),
        ("E_MISSING_ARG", "/nodes/5"),
        ("E_UNKNOWN_FIELD", "/nodes/5/perod"),
        ("E_DUPLICATE_NAME", "/outputs/1/name"),
        ("E_UNKNOWN_OP", "/outputs/1/kind"),
        ("E_UNKNOWN_REF", "/outputs/2/node"),
        ("E_UNSUPPORTED", "/outputs/2/side"),
        ("E_NO_OUTCOME_RULE", "/outputs/2"),
    ]
    .iter()
    .map(|(a, b)| ((*a).to_owned(), (*b).to_owned()))
    .collect();
    let mut sorted_want = want.clone();
    sorted_want.sort_by(|a, b| a.1.cmp(&b.1).then(a.0.cmp(&b.0)));
    let mut sorted_got = got.clone();
    sorted_got.sort_by(|a, b| a.1.cmp(&b.1).then(a.0.cmp(&b.0)));
    assert_eq!(sorted_got, sorted_want, "{:#?}", r.errors);
    // Node `d` reads broken nodes and adds nothing (no noise); positions are in order.
    assert!(!got.iter().any(|(_, at)| at.starts_with("/nodes/3")));
    let pos: Vec<&str> = got.iter().map(|(_, at)| at.as_str()).collect();
    let ord = |a: &str| if a.starts_with("/nodes") { 0 } else { 1 };
    assert!(pos.windows(2).all(|w| ord(w[0]) <= ord(w[1])), "{pos:?}");
    let dym = |at: &str| {
        r.errors
            .iter()
            .find(|e| e.at.as_deref() == Some(at))
            .map(|e| e.did_you_mean.clone())
    };
    assert_eq!(dym("/nodes/0/field").unwrap_or_default()[0], "close");
    assert_eq!(dym("/nodes/1/op").unwrap_or_default()[0], "rsi");
    assert_eq!(dym("/nodes/2/period"), Some(vec!["len".to_owned()]));
    assert_eq!(dym("/nodes/5/perod"), Some(vec!["period".to_owned()]));
    assert_eq!(dym("/outputs/1/kind"), Some(vec!["state".to_owned()]));
    // Within one node, errors keep pointer order then reason.
    let n5: Vec<&str> = got
        .iter()
        .filter(|(_, at)| at.starts_with("/nodes/5"))
        .map(|(r, _)| r.as_str())
        .collect();
    assert_eq!(n5, vec!["E_MISSING_ARG", "E_UNKNOWN_FIELD"]);
    assert!(r.cost.is_none() && r.checked.is_none() && r.instance_id.is_none());
}

#[test]
fn errors_are_cut_at_the_limit() {
    let l = Limits::default().with(Limit::CheckMaxErrors, 3);
    let r = chk_limits(&many_errors(), l);
    assert_eq!(r.errors.len(), 3);
    assert!(r.truncated && r.to_answer().truncated);
    assert!(!chk(&many_errors()).truncated);
}

#[test]
fn messages_in_two_languages_same_codes() {
    let en = chk(&many_errors());
    let tr = check(
        &many_errors(),
        &CheckCtx {
            lang: Lang::Tr,
            ..CheckCtx::default()
        },
    );
    assert_eq!(errs(&en), errs(&tr));
    assert!(en
        .errors
        .iter()
        .zip(&tr.errors)
        .all(|(a, b)| a.message != b.message));
    assert!(tr.errors[1].message.contains("bilinmeyen işlev"));
}

#[test]
fn answer_carries_errors_limits_and_outputs() {
    let a = chk(&many_errors()).to_answer();
    assert!(!a.ok);
    assert_eq!(a.limits.len(), 20);
    let v = serde_json::to_value(&a).unwrap_or_default();
    assert_eq!(v["errors"][0]["code"], "schema");
    assert_eq!(v["limits"]["program.max_cost_per_bar"], 666);
}

#[test]
fn odd_json_never_panics_and_reports() {
    let odd = [
        json!(null),
        json!(3),
        json!([]),
        json!({}),
        json!({"version": 1, "nodes": {}, "outputs": "x", "params": 5}),
        json!({"version": "qp-ir/0", "nodes": [null, 1, [], {"id": 3, "op": []}], "outputs": [null]}),
        json!({"version": "qp-ir/0", "nodes": [{"id": "a", "op": "line", "a": 1, "b": {"t": "x"}}],
               "outputs": [{"name": "a", "kind": "state", "dims": [1, {"name": 2}]}]}),
        json!({"version": "qp-ir/0", "nodes": [{"id": "a", "op": "const", "value": {"param": 1, "x": 2}}],
               "outputs": [{"name": "a", "kind": "signal", "rule": 1, "source": [], "side": 2}]}),
        json!({"reject": 1}),
        json!({"reject": {"code": "E_NOPE"}, "extra": 1}),
    ];
    for v in odd {
        let r = chk(&v);
        assert!(!r.ok(), "{v}");
        assert!(r
            .errors
            .iter()
            .all(|e| e.code.is_some() && e.reason.starts_with("E_")));
    }
}

#[test]
fn a_refusal_is_ok() {
    let r = chk(&json!({"reject": {"code": "E_LOOKAHEAD", "reason": "needs the next close"}}));
    assert!(r.ok());
    assert_eq!(
        r.reject.as_ref().map(|j| j.code.as_str()),
        Some("E_LOOKAHEAD")
    );
    assert!(r.checked.is_none() && r.program_id.is_none());
    let r = chk(&json!({"reject": {"code": "E_LOKAHEAD", "reason": "x"}}));
    let e = only(&r, "E_BAD_PARAM", "/reject/code");
    assert_eq!(e.did_you_mean[0], "E_LOOKAHEAD");
}
