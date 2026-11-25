//! Text errors (PR §11.1, §12.4): all at once, in position order, with span, closed
//! code, sub-code, suggestions and messages in two languages; the source map.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use qp_lang_text::lower;
use qp_schema::protocol::{ErrorCode, Language, ProgramError, Span};

fn errors(src: &str) -> Vec<ProgramError> {
    match lower(src, Language::En) {
        Ok(l) => panic!("expected errors, got {}", l.ir),
        Err(e) => e,
    }
}

fn one(src: &str) -> ProgramError {
    let e = errors(src);
    assert_eq!(e.len(), 1, "{e:#?}");
    e.into_iter().next().unwrap()
}

fn span(line: u32, col: u32, len: u32) -> Option<Span> {
    Some(Span { line, col, len })
}

#[test]
fn every_line_is_reported_and_ordered() {
    let e = errors(
        "a = sma(close, 14\nb = close +\nseries x = a\nc = foo(close)\nseries y = c > 1 > 2\n",
    );
    let got: Vec<(&str, Option<Span>)> = e.iter().map(|e| (e.reason.as_str(), e.span)).collect();
    assert_eq!(
        got,
        vec![
            ("E_PARSE", span(1, 18, 1)),
            ("E_PARSE", span(2, 12, 1)),
            ("E_UNKNOWN_OP", span(4, 5, 3)),
            ("E_PARSE", span(5, 18, 1)),
        ]
    );
    assert!(e.iter().all(|e| e.code == Some(ErrorCode::Schema)));
    assert_eq!(e[0].message, "expected `)`, found the end of the line");
    assert_eq!(
        e[3].message,
        "comparisons do not chain; join them with `and`"
    );
}

#[test]
fn unknown_function_suggests() {
    let e = one("b = rsii(close, 14)\nseries b2 = b\n");
    assert_eq!(e.reason, "E_UNKNOWN_OP");
    assert_eq!(e.span, span(1, 5, 4));
    assert_eq!(e.did_you_mean, vec!["rsi"]);
    assert!(e.expected.contains(&"percentrank".to_owned()));
    assert_eq!(e.message, "unknown function `rsii`");

    let e = one("x = ussel_ortalama(close, 5)\nseries s = x\n");
    assert_eq!(e.did_you_mean.first().map(String::as_str), Some("ema"));
    assert!(e.hint.unwrap().contains("Turkish"));
    let e = one("series s = ortalama(close, 5)\n");
    assert_eq!(e.did_you_mean.first().map(String::as_str), Some("sma"));
}

#[test]
fn unknown_and_later_names() {
    let e = one("series x = clse\n");
    assert_eq!(
        (e.reason.as_str(), e.code),
        ("E_UNKNOWN_REF", Some(ErrorCode::NotFound))
    );
    assert_eq!(e.did_you_mean.first().map(String::as_str), Some("close"));
    assert_eq!(e.span, span(1, 12, 4));

    let e = one("a = b + 1\nb = close\nseries s = a\n");
    assert_eq!(e.reason, "E_UNKNOWN_REF");
    assert!(e.hint.unwrap().contains("line 2"));

    let e = one("series s = kapanış\n");
    assert_eq!(e.did_you_mean.first().map(String::as_str), Some("close"));
    assert_eq!(e.span, span(1, 12, 7));
}

#[test]
fn number_fields() {
    let e = one("series s = sma(close, close)\n");
    assert_eq!((e.reason.as_str(), e.span), ("E_TYPE", span(1, 23, 5)));
    let e = one("series s = sma(close, n)\n");
    assert_eq!(
        (e.reason.as_str(), e.code),
        ("E_UNKNOWN_PARAM", Some(ErrorCode::NotFound))
    );
    let e = one("k = 3\nseries s = close[k]\n");
    assert_eq!(e.reason, "E_TYPE");
    assert!(e.hint.unwrap().contains("param"));
    let e = one("series s = sma(close)\n");
    assert_eq!(e.reason, "E_MISSING_ARG");
    assert!(e.message.contains("`period`"));
    let e = one("series s = abs(close, 2)\n");
    assert_eq!(e.reason, "E_PARSE");
}

#[test]
fn representable_values_are_left_to_the_checker() {
    let l = lower(
        "series s = sma(close, 2.5)\nseries t = close[-1]\n",
        Language::En,
    )
    .unwrap();
    assert_eq!(l.ir["nodes"][1]["period"], serde_json::json!(2.5));
    assert_eq!(l.ir["nodes"][3]["n"], serde_json::json!(-1));
    let l = lower(
        "signal s = close > 1 side tut source \"x\" \"1\"\n",
        Language::En,
    )
    .unwrap();
    assert_eq!(l.ir["outputs"][0]["side"], "tut");
    assert!(l.ir["outputs"][0].get("rule").is_none());
}

#[test]
fn reject_form() {
    let e = one("reject E_LOOKHEAD \"no\"\n");
    assert_eq!(e.reason, "E_PARSE");
    assert_eq!(
        e.did_you_mean.first().map(String::as_str),
        Some("E_LOOKAHEAD")
    );
    assert_eq!(e.span, span(1, 8, 10));
    let e = one("x = close\nreject E_UNSUPPORTED \"no\"\n");
    assert_eq!((e.reason.as_str(), e.span.unwrap().line), ("E_PARSE", 2));
    let e = one("reject E_UNSUPPORTED\n");
    assert_eq!(e.reason, "E_PARSE");
}

#[test]
fn left_out_constructs_are_coded() {
    for (src, line) in [
        ("for i = 1\n", 1),
        ("var x = 1\n", 1),
        ("if close > 1\n", 1),
        ("x = close > 1 ? 1 : 0\n", 1),
        ("x = security(\"AAPL\", close)\n", 1),
        ("series s = close\nstrategy.entry(\"long\")\n", 2),
    ] {
        let e = one(src);
        assert_eq!(
            (e.reason.as_str(), e.code, e.span.unwrap().line),
            ("E_UNSUPPORTED", Some(ErrorCode::Unsupported), line),
            "{src}"
        );
    }
}

#[test]
fn names_and_order() {
    let e = errors(
        "param n int = 1 in 1..5\nn = close\nx = close\nx = open\nparam m int = 1 in 1..5\n",
    );
    let got: Vec<(&str, u32)> = e
        .iter()
        .map(|e| (e.reason.as_str(), e.span.unwrap().line))
        .collect();
    assert_eq!(
        got,
        vec![
            ("E_DUPLICATE_NAME", 2),
            ("E_DUPLICATE_NAME", 4),
            ("E_PARSE", 5)
        ]
    );
    assert_eq!(e[0].code, Some(ErrorCode::Duplicate));
    let e = one("close = 1\n");
    assert_eq!(e.reason, "E_PARSE");
    let e = one("_x = close\n");
    assert_eq!(e.reason, "E_PARSE");
}

#[test]
fn hints_for_other_notations() {
    let e = one("series s = !defined(close)\n");
    assert_eq!(e.span, span(1, 12, 1));
    assert!(e.hint.unwrap().contains("`not`"));
    let e = one("seri s = close\n");
    assert_eq!(e.did_you_mean, vec!["series"]);
    let e = one("signal s = close > 1 side al foo 3\n");
    assert!(e.expected.contains(&"max_bars".to_owned()));
}

#[test]
fn bilingual() {
    let src = "b = rsii(close, 14)\nseries s = b\n";
    let en = lower(src, Language::En).unwrap_err();
    let tr = lower(src, Language::Tr).unwrap_err();
    assert_eq!(en[0].message, "unknown function `rsii`");
    assert_eq!(tr[0].message, "bilinmeyen işlev `rsii`");
    assert_eq!(
        (en[0].code, &en[0].reason, en[0].span),
        (tr[0].code, &tr[0].reason, tr[0].span)
    );
    let tr = lower("a = sma(close, 14\n", Language::Tr).unwrap_err();
    assert_eq!(tr[0].message, "`)` bekleniyordu, satır sonu bulundu");
    let tr = lower("x = ortalama(close, 3)\n", Language::Tr).unwrap_err();
    assert!(tr[0].hint.as_deref().unwrap().contains("Türkçe"));
}

#[test]
fn spans_count_characters() {
    let e = one("signal s = close > 1 side al source \"şğüİ\" \"1\" ?\n");
    assert_eq!(e.span, span(1, 48, 1));
    let e = one("# açıklama: çok güzel\nseries s = clöse\n");
    assert_eq!(e.span, span(2, 12, 5));
}

#[test]
fn source_map_points_checker_errors_at_text() {
    let src = "param n int = 3 in 1..5\nm = sma(close, n)\nseries s = m * 2\n";
    let l = lower(src, Language::En).unwrap();
    assert_eq!(l.map.span_at("/nodes/1/period"), span(2, 16, 1));
    assert_eq!(l.map.span_at("/nodes/1"), span(2, 5, 13));
    assert_eq!(l.map.span_at("/nodes/1/x/zz"), span(2, 9, 5));
    assert_eq!(l.map.span_at("/params/0/max"), span(1, 23, 1));
    assert_eq!(l.map.span_at("/outputs/0/node"), span(3, 12, 5));
    let mut errs = vec![ProgramError {
        code: Some(ErrorCode::Limit),
        reason: "E_WINDOW_LIMIT".into(),
        message: "x".into(),
        at: Some("/nodes/1/period".into()),
        span: None,
        expected: vec![],
        did_you_mean: vec![],
        hint: None,
    }];
    l.map.attach(&mut errs);
    assert_eq!(errs[0].span, span(2, 16, 1));
}
