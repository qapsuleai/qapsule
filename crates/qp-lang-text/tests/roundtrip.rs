//! Round-trip properties of PR §12.3, on randomly generated programs (fixed seeds):
//! (a) `lower(print(lower(t))) == lower(t)` byte for byte; (b) `lower(print(p))` equals
//! `p` byte for byte when `p` has no `_` ids, and up to `_` renaming otherwise (same
//! canonical text, same node count); (c) `print` is idempotent.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod gen;

use gen::{ir_program, Rng, TextGen};
use qp_lang_text::{lower, print};
use qp_schema::program::{Program, ProgramDoc};
use qp_schema::protocol::Language;

const CASES: usize = 300;

fn program(src: &str) -> (serde_json::Value, Program) {
    let l = lower(src, Language::En).unwrap_or_else(|e| panic!("{src}\n{e:#?}"));
    match l.doc().unwrap_or_else(|e| panic!("{src}\n{e}\n{}", l.ir)) {
        ProgramDoc::Program(p) => (l.ir, p),
        ProgramDoc::Reject(_) => panic!("reject"),
    }
}

#[test]
fn text_lower_print_lower_is_stable() {
    let mut r = Rng::new(0x5EED_0001);
    for case in 0..CASES {
        let src = TextGen::new(&mut r).program();
        let (ir1, p1) = program(&src);
        let text1 = print(&p1);
        let (ir2, p2) = program(&text1);
        assert_eq!(
            ir1, ir2,
            "case {case}\n--- source\n{src}--- printed\n{text1}"
        );
        assert_eq!(
            p1.canonical_json().unwrap(),
            p2.canonical_json().unwrap(),
            "case {case}"
        );
        assert_eq!(print(&p2), text1, "case {case}: print not idempotent");
    }
}

#[test]
fn ir_without_underscore_ids_is_identity() {
    let mut r = Rng::new(0x5EED_0002);
    for case in 0..CASES {
        let v = ir_program(&mut r, false);
        let p: Program = serde_json::from_value(v.clone()).unwrap_or_else(|e| panic!("{e}\n{v}"));
        let text = print(&p);
        let (_, q) = program(&text);
        assert_eq!(
            q.canonical_json().unwrap(),
            p.canonical_json().unwrap(),
            "case {case}\n{text}"
        );
    }
}

#[test]
fn ir_with_underscore_ids_is_identity_up_to_renaming() {
    let mut r = Rng::new(0x5EED_0003);
    for case in 0..CASES {
        let v = ir_program(&mut r, true);
        let p: Program = serde_json::from_value(v.clone()).unwrap_or_else(|e| panic!("{e}\n{v}"));
        let text = print(&p);
        let (_, q) = program(&text);
        assert_eq!(q.nodes.len(), p.nodes.len(), "case {case}\n{text}");
        assert_eq!(print(&q), text, "case {case}");
        let (_, q2) = program(&print(&q));
        assert_eq!(
            q2.canonical_json().unwrap(),
            q.canonical_json().unwrap(),
            "case {case}"
        );
    }
}

#[test]
fn printer_names_unprintable_ids() {
    // A `_` node used twice gets the smallest unused `v<k>`; one used once is inlined.
    let p: Program = serde_json::from_value(serde_json::json!({
        "version": "qp-ir/0",
        "params": [{"key": "v1", "kind": "int", "min": 1, "max": 9, "default": 2}],
        "nodes": [
            {"id": "_7", "op": "input", "field": "close"},
            {"id": "_8", "op": "sma", "x": "_7", "period": {"param": "v1"}},
            {"id": "v2", "op": "sub", "a": "_8", "b": "_8"},
            {"id": "_9", "op": "neg", "x": "v2"},
            {"id": "_10", "op": "const", "value": 2.0},
            {"id": "_11", "op": "neg", "x": "_10"}
        ],
        "outputs": [{"name": "s", "kind": "series", "node": "_9"},
                    {"name": "t", "kind": "series", "node": "_11"}]
    }))
    .unwrap();
    assert_eq!(
        print(&p),
        "param v1 int = 2 in 1..9\nv3 = sma(close, v1)\nv2 = v3 - v3\nseries s = -v2\nseries t = -(2)\n"
    );
}

#[test]
fn printer_keeps_tree_shape() {
    for (src, canonical) in [
        ("series s = close - (open - low)\n", None),
        (
            "series s = (close - open) - low\n",
            Some("series s = close - open - low\n"),
        ),
        ("series s = close / (open * low)\n", None),
        ("event e = (close > open) == (low > high)\n", None),
        ("event e = not close > open\n", None),
        ("event e = (not close) > open\n", None),
        (
            "event e = close > open and (low > high or low < open)\n",
            None,
        ),
        (
            "event e = (close > open and low > high) or low < open\n",
            Some("event e = close > open and low > high or low < open\n"),
        ),
        (
            "series s = (-2)[1] + -(close)[2] - -2 * (-close)\n",
            Some("series s = (-2)[1] + -close[2] - -2 * -close\n"),
        ),
        ("series s = (close + open).x\n", None),
        ("series s = -(-close)\n", None),
        ("series s = -(2) + -2\n", None),
        ("series s = 0.0000001 + 1000000000000000000000 + -0\n", None),
        ("series s = 2.50 + 007\n", Some("series s = 2.5 + 7\n")),
    ] {
        let (ir, p) = program(src);
        let text = print(&p);
        assert_eq!(text, canonical.unwrap_or(src));
        assert_eq!(program(&text).0, ir, "{src}");
    }
}
