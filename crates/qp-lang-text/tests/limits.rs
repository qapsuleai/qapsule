//! Hostile input: deep nesting, huge sources and tokens give coded errors (no stack
//! overflow, no panic); names that would inject text are refused by the printer.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use qp_lang_text::{lower, lower_with, print, print_reject, TextLimits, DEFAULT_MAX_DEPTH};
use qp_schema::program::{Program, ProgramReject};
use qp_schema::protocol::{ErrorCode, Language};

fn too_large(src: &str) {
    let e = lower(src, Language::En).unwrap_err();
    assert!(
        e.iter()
            .any(|e| e.reason == "E_TOO_LARGE" && e.code == Some(ErrorCode::Limit)),
        "{e:#?}"
    );
}

/// Runs on a 2 MiB stack (the default for spawned threads), so unbounded recursion would crash the test.
fn small_stack(f: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(f)
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn deep_nesting_is_a_coded_error() {
    small_stack(|| {
        let n = 30_000;
        too_large(&format!(
            "series s = {}close{}\n",
            "(".repeat(n),
            ")".repeat(n)
        ));
        too_large(&format!("series s = {}close\n", "-".repeat(n)));
        too_large(&format!("event e = {}true\n", "not ".repeat(n / 4)));
        too_large(&format!(
            "series s = {}close{}\n",
            "abs(".repeat(n / 4),
            ")".repeat(n / 4)
        ));
        too_large(&format!(
            "series s = {}\n",
            vec!["close"; 20_000].join(" + ")
        ));
        too_large(&format!("series s = close{}\n", "[1]".repeat(20_000)));
        too_large(&format!(
            "series s = bucket(close, {}1{})\n",
            "[".repeat(n),
            "]".repeat(n)
        ));
    });
    // At the limit it still lowers.
    let ok = format!(
        "series s = {}close{}\n",
        "(".repeat(DEFAULT_MAX_DEPTH - 1),
        ")".repeat(DEFAULT_MAX_DEPTH - 1)
    );
    assert!(lower(&ok, Language::En).is_ok());
}

/// The sizes an audit measured aborting (release, 2 MiB thread): each must now give a
/// coded error.
#[test]
fn audited_sizes_are_coded() {
    small_stack(|| {
        too_large(&format!(
            "series s = {}close{}\n",
            "(".repeat(1000),
            ")".repeat(1000)
        ));
        too_large(&format!(
            "series s = {}close{}\n",
            "abs(".repeat(1000),
            ")".repeat(1000)
        ));
        too_large(&format!("series s = close{}\n", " + 1".repeat(3000)));
        too_large(&format!("series s = {}close\n", "-".repeat(10_000)));
    });
}

#[test]
fn left_out_constructs_anywhere() {
    for src in [
        "x = if close > 1 then 1 else 0\n",
        "series s = close + request.security(\"X\", close)\n",
        "series s = strategy.position\n",
        "if = close\n",
    ] {
        let e = lower(src, Language::En).unwrap_err();
        assert!(
            e.iter().any(|e| e.reason == "E_UNSUPPORTED")
                || e[0].reason == "E_PARSE" && src.starts_with("if ="),
            "{src}: {e:#?}"
        );
    }
    let e = lower("series s = close * 1e400\n", Language::En).unwrap_err();
    assert_eq!(e[0].reason, "E_PARSE");
    assert!(e[0].message.contains("exponent"), "{}", e[0].message);
    assert_eq!(e[0].span.unwrap().len, 5);
}

#[test]
fn source_and_token_size_are_capped() {
    let limits = TextLimits {
        max_source_chars: 100,
        ..TextLimits::default()
    };
    let e = lower_with(&"#".repeat(101), Language::En, &limits).unwrap_err();
    assert_eq!((e.len(), e[0].reason.as_str()), (1, "E_TOO_LARGE"));
    too_large(&format!("series s = {}\n", "x".repeat(5000)));
    too_large(&format!(
        "param p int = 1 in 1..2 \"{}\"\n",
        "d".repeat(5000)
    ));
    too_large(&format!("series s = close + {}\n", "9".repeat(5000)));
}

#[test]
fn deep_ir_prints_with_bounded_inlining() {
    small_stack(|| {
        let mut nodes = vec![serde_json::json!({"id": "_0", "op": "input", "field": "close"})];
        for i in 1..20_000 {
            nodes.push(
                serde_json::json!({"id": format!("_{i}"), "op": "neg", "x": format!("_{}", i - 1)}),
            );
        }
        let p: Program = serde_json::from_value(serde_json::json!({
            "version": "qp-ir/0", "nodes": nodes,
            "outputs": [{"name": "s", "kind": "series", "node": "_19999"}]
        }))
        .unwrap();
        let text = print(&p).unwrap();
        assert!(text.lines().count() > 100);
        // A node that reads itself (bad IR) is not inlined into itself.
        let p: Program = serde_json::from_value(serde_json::json!({
            "version": "qp-ir/0",
            "nodes": [{"id": "_1", "op": "neg", "x": "_1"}],
            "outputs": [{"name": "s", "kind": "series", "node": "x"}]
        }))
        .unwrap();
        assert_eq!(print(&p).unwrap(), "v1 = -v1\nseries s = x\n");
    });
}

#[test]
fn injecting_names_are_refused() {
    let base = |out: &str, key: &str, tf: &str, sel: &str| {
        serde_json::from_value::<Program>(serde_json::json!({
            "version": "qp-ir/0",
            "params": [{"key": key, "kind": "int", "min": 1, "max": 5, "default": 2}],
            "nodes": [{"id": "m", "op": "macd", "x": "c", "fast": 1, "slow": 2, "signal": 3},
                      {"id": "c", "op": "input", "field": "close", "tf": tf}],
            "outputs": [{"name": out, "kind": "series", "node": format!("m.{sel}")}]
        }))
        .unwrap()
    };
    assert!(print(&base("ok", "k", "1d", "signal")).is_ok());
    for (p, at) in [
        (
            base("x = 1\nseries y", "k", "1d", "signal"),
            "/outputs/0/name",
        ),
        (
            base("ok", "k\nseries z = close", "1d", "signal"),
            "/params/0/key",
        ),
        (base("ok", "close", "1d", "signal"), "/params/0/key"),
        (
            base("ok", "k", "1d\nseries z = open", "signal"),
            "/nodes/1/tf",
        ),
        (base("ok", "k", "1d", "signal\nevent e"), "/outputs/0/node"),
    ] {
        assert_eq!(print(&p).unwrap_err().at, at);
    }
    let r = ProgramReject {
        code: "E_X \"a\"\nseries s".into(),
        reason: "r\n\"".into(),
    };
    assert_eq!(print_reject(&r).unwrap_err().at, "/reject/code");
    let r = ProgramReject {
        code: "E_LOOKAHEAD".into(),
        reason: "line\nbreak \"q\"".into(),
    };
    assert_eq!(
        print_reject(&r).unwrap(),
        "reject E_LOOKAHEAD \"line\\nbreak \\\"q\\\"\"\n"
    );
}

/// A chain of `height` nodes: `_0` = close, then each node wraps the previous one.
fn chain(kind: &str, height: usize, underscore: bool) -> Program {
    let id = |i: usize| {
        if underscore {
            format!("_{i}")
        } else {
            format!("n{i}")
        }
    };
    let shared = if underscore { "_c" } else { "cc" };
    let mut nodes = vec![serde_json::json!({"id": id(0), "op": "input", "field": "close"})];
    for i in 1..height {
        let prev = id(i - 1);
        let mut n = match kind {
            "neg" => serde_json::json!({"op": "neg", "x": prev}),
            "not" => serde_json::json!({"op": "not", "x": prev}),
            "sub_left" => serde_json::json!({"op": "sub", "a": prev, "b": shared}),
            "sub_right" => serde_json::json!({"op": "sub", "a": shared, "b": prev}),
            "abs" => serde_json::json!({"op": "abs", "x": prev}),
            "lag" => serde_json::json!({"op": "lag", "x": prev, "n": 1}),
            "sel" => {
                serde_json::json!({"op": "macd", "x": format!("{prev}.signal"), "fast": 1, "slow": 2, "signal": 3})
            }
            _ => serde_json::json!({"op": "bucket", "x": prev, "edges": [1.0]}),
        };
        n["id"] = serde_json::json!(id(i));
        nodes.push(n);
    }
    // `_c` is a named-by-text node read many times (never inlined).
    nodes.insert(
        0,
        serde_json::json!({"id": shared, "op": "input", "field": "open"}),
    );
    let root = id(height - 1);
    let node = if kind == "sel" {
        format!("{root}.signal")
    } else {
        root
    };
    serde_json::from_value(serde_json::json!({
        "version": "qp-ir/0", "nodes": nodes,
        "outputs": [{"name": "s", "kind": "series", "node": node}]
    }))
    .unwrap()
}

fn lowered(text: &str) -> Program {
    match lower(text, Language::En)
        .unwrap_or_else(|e| panic!("{text}\n{e:#?}"))
        .doc()
        .unwrap()
    {
        qp_schema::program::ProgramDoc::Program(p) => p,
        qp_schema::program::ProgramDoc::Reject(_) => panic!("reject"),
    }
}

/// Near-limit IR always prints to text that lowers: without `_` ids byte-equal; with
/// `_` ids up to renaming (same canonical text), then byte-equal on the second pass.
#[test]
fn near_limit_ir_prints_to_text_that_lowers() {
    small_stack(|| {
        let heights = [30, 31, 32, 33, 50, 60, 62, 63, 64, 65, 66, 100, 128, 200];
        for kind in [
            "neg",
            "not",
            "sub_left",
            "sub_right",
            "abs",
            "lag",
            "sel",
            "bucket",
        ] {
            for h in heights {
                let p = chain(kind, h, false);
                let q = lowered(&print(&p).unwrap());
                assert_eq!(
                    q.canonical_json().unwrap(),
                    p.canonical_json().unwrap(),
                    "{kind} {h}"
                );
                let p = chain(kind, h, true);
                let text = print(&p).unwrap();
                let q = lowered(&text);
                assert_eq!(q.nodes.len(), p.nodes.len(), "{kind} {h}");
                assert_eq!(print(&q).unwrap(), text, "{kind} {h}");
                let q2 = lowered(&print(&q).unwrap());
                assert_eq!(
                    q2.canonical_json().unwrap(),
                    q.canonical_json().unwrap(),
                    "{kind} {h}"
                );
            }
        }
    });
}

/// Text within the limit is never cut by the printer: `lower(print(lower(t))) == lower(t)`
/// byte for byte, up to the parser's limit.
#[test]
fn near_limit_text_round_trips() {
    small_stack(|| {
        let max = DEFAULT_MAX_DEPTH;
        let mut texts = vec![];
        for k in [30, 33, 50, 62, 63] {
            texts.push(format!("series s = {}close\n", "- ".repeat(k)));
            texts.push(format!("series s = {}close\n", "-".repeat(k)));
            texts.push(format!("event e = {}true\n", "not ".repeat(k)));
            texts.push(format!(
                "series s = {}close{}\n",
                "abs(".repeat(k),
                ")".repeat(k)
            ));
            texts.push(format!("series s = close{}\n", "[1]".repeat(k)));
            texts.push(format!("series s = close{}\n", " - open".repeat(k)));
            texts.push(format!(
                "series s = {}close{}\n",
                "(open - ".repeat(k / 2),
                ")".repeat(k / 2)
            ));
        }
        texts.push(format!(
            "series s = {}close{}\n",
            "(".repeat(max - 1),
            ")".repeat(max - 1)
        ));
        let mut lowered_count = 0;
        let total = texts.len();
        for t in texts {
            let Ok(l) = lower(&t, Language::En) else {
                continue;
            };
            lowered_count += 1;
            let qp_schema::program::ProgramDoc::Program(p) = l.doc().unwrap() else {
                panic!()
            };
            let printed = print(&p).unwrap();
            let q = lowered(&printed);
            assert_eq!(
                q.canonical_json().unwrap(),
                p.canonical_json().unwrap(),
                "{t}"
            );
            assert!(!printed.contains("v1 ="), "cut: {t}");
        }
        assert!(
            lowered_count + 4 >= total,
            "{lowered_count} of {total} lowered"
        );
    });
}
