//! Unresolvable views give errors that name the layer, the place and the reason.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::cognitive_complexity
)]

mod support;

use qp_grammar::{compile, CompileOptions, ErrorKind, GrammarError};

fn err(json: &str) -> GrammarError {
    let p = support::provider();
    compile(&support::view(json), &p, &CompileOptions::new(1200, 600)).unwrap_err()
}

fn one(layer: &str) -> String {
    format!(
        r#"{{"id":"v","x":{{"kind":"time"}},"panels":[{{"id":0,"y":{{"kind":"linear"}}}}],
            "layers":[{{"id":"candles","mark":"candle","series":"px","panel":0}},{layer}]}}"#
    )
}

#[test]
fn unknown_series() {
    let e = err(&one(
        r#"{"id":"x","mark":"line","series":"nope","panel":0}"#,
    ));
    assert_eq!(e.kind, ErrorKind::NotFound);
    assert_eq!(e.layer.as_deref(), Some("x"));
    assert_eq!(e.path, "/series");
    assert!(
        e.to_string().contains("series `nope` does not exist"),
        "{e}"
    );
}

#[test]
fn unknown_field_lists_fields() {
    let e = err(&one(
        r#"{"id":"x","mark":"line","series":"bb","panel":0,"encoding":{"y":"uper"}}"#,
    ));
    assert_eq!(e.kind, ErrorKind::NotFound);
    assert_eq!(e.path, "/encoding/y");
    let s = e.to_string();
    assert!(s.starts_with("layer `x` /encoding/y:"), "{s}");
    assert!(s.contains("upper, middle, lower, bandwidth"), "{s}");
}

#[test]
fn unknown_field_of_other_series() {
    let e = err(&one(
        r#"{"id":"x","mark":"band","series":"bb","panel":0,"encoding":{"y":"upper","y2":{"series":"sma50","field":"smaa"}}}"#,
    ));
    assert_eq!(e.path, "/encoding/y2");
    assert!(e.reason.contains("sma50"));
}

#[test]
fn candle_needs_bars() {
    let e = err(&one(
        r#"{"id":"x","mark":"candle","series":"rsi14","panel":0}"#,
    ));
    assert_eq!(e.kind, ErrorKind::Invalid);
    assert_eq!(e.layer.as_deref(), Some("x"));
}

#[test]
fn constant_without_origin() {
    let e = err(&one(
        r#"{"id":"r","mark":"rule","panel":0,"encoding":{"y":123.45}}"#,
    ));
    assert_eq!(e.kind, ErrorKind::NoOrigin);
    assert_eq!(e.path, "/encoding/y");
    assert!(e.to_string().contains("123.45 has no origin"), "{e}");
    // A condition threshold too.
    let e = err(&one(
        r#"{"id":"h","mark":"bar","series":"macd","panel":0,"encoding":{"y":"histogram","color":{"when":{"field":"histogram","op":"ge","than":0},"then":"up","else":"down"}}}"#,
    ));
    assert_eq!(e.kind, ErrorKind::NoOrigin);
    assert_eq!(e.path, "/encoding/color/when/than");
}

#[test]
fn origin_must_match_constant() {
    let e = err(&one(
        r#"{"id":"r","mark":"rule","panel":0,"encoding":{"y":71},"origin":{"/encoding/y":{"kind":"threshold","name":"rsi.overbought"}}}"#,
    ));
    assert_eq!(e.kind, ErrorKind::NoOrigin);
    assert!(e.reason.contains("does not match"));
}

#[test]
fn origin_must_resolve() {
    let e = err(&one(
        r#"{"id":"r","mark":"rule","panel":0,"origin":{"/encoding/y":{"kind":"threshold","name":"rsi.overbougth"}}}"#,
    ));
    assert_eq!(e.kind, ErrorKind::NotFound);
    assert!(e.reason.contains("rsi.overbougth"));
    let e = err(&one(
        r#"{"id":"r","mark":"rule","panel":0,"origin":{"/encoding/y":{"kind":"fact","fact":"nope","field":"level"}}}"#,
    ));
    assert_eq!(e.kind, ErrorKind::NotFound);
    let e = err(&one(
        r#"{"id":"r","mark":"rule","panel":0,"origin":{"/encoding/width":{"kind":"threshold","name":"rsi.mid"}}}"#,
    ));
    assert_eq!(e.kind, ErrorKind::Invalid);
    assert!(e.path.starts_with("/origin/"));
}

#[test]
fn origin_without_written_constant_supplies_it() {
    let p = support::provider();
    let v = support::view(&one(
        r#"{"id":"r","mark":"rule","panel":0,"origin":{"/encoding/y":{"kind":"series","series":"px","field":"c","at":"last"}}}"#,
    ));
    let s = compile(&v, &p, &CompileOptions::new(1200, 600)).unwrap();
    let r = &s.panels[0].layers[1];
    let last = support::sample_bars().last().unwrap().c;
    assert_eq!(r.constants["/encoding/y"].value, last);
    assert_eq!(r.items[0].values["y"], last);
}

#[test]
fn bad_colour_and_opacity() {
    let e = err(&one(
        r#"{"id":"x","mark":"line","series":"sma20","panel":0,"encoding":{"color":"reddish"}}"#,
    ));
    assert_eq!(e.kind, ErrorKind::Invalid);
    assert_eq!(e.path, "/encoding/color");
    assert!(e.reason.contains("accent"));
    let e = err(&one(
        r#"{"id":"x","mark":"line","series":"sma20","panel":0,"encoding":{"opacity":1.5}}"#,
    ));
    assert_eq!(e.path, "/encoding/opacity");
}

#[test]
fn stepped_colour_shape() {
    let e = err(&one(
        r#"{"id":"x","mark":"line","series":"rsi14","panel":0,"encoding":{"color":{"field":"rsi","thresholds":[30,70],"colors":["up","down"]}},
            "origin":{"/encoding/color/thresholds/0":{"kind":"threshold","name":"rsi.oversold"},"/encoding/color/thresholds/1":{"kind":"threshold","name":"rsi.overbought"}}}"#,
    ));
    assert_eq!(e.path, "/encoding/color/colors");
}

#[test]
fn unknown_panel_lists_panels() {
    let e = err(
        r#"{"id":"v","x":{"kind":"time"},"panels":[{"id":0,"name":"price","y":{"kind":"linear"}}],
            "layers":[{"id":"c","mark":"candle","series":"px","panel":3}]}"#,
    );
    assert_eq!(e.kind, ErrorKind::NotFound);
    assert_eq!(e.path, "/panel");
    assert!(e.reason.contains("0 (price)"));
}

#[test]
fn band_needs_y2() {
    let e = err(&one(
        r#"{"id":"b","mark":"band","series":"bb","panel":0,"encoding":{"y":"upper"}}"#,
    ));
    assert_eq!(e.path, "/encoding/y2");
}

#[test]
fn view_level_errors() {
    let e = err(
        r#"{"id":"v","x":{"kind":"time"},"panels":[{"id":0,"y":{"kind":"log","domain":[0,10]}}],
            "layers":[{"id":"c","mark":"candle","series":"px","panel":0}]}"#,
    );
    assert_eq!(e.path, "/panels/0/y/domain");
    let e = err(
        r#"{"id":"v","x":{"kind":"time","domain":[0,1]},"panels":[{"id":0,"y":{"kind":"linear"}}],
            "layers":[{"id":"c","mark":"candle","series":"px","panel":0}]}"#,
    );
    assert_eq!(e.path, "/x/domain");
    let e = err(
        r#"{"id":"v","x":{"kind":"time"},"panels":[{"id":0,"y":{"kind":"linear"}}],
            "layers":[{"id":"r","mark":"rule","panel":0,"origin":{"/encoding/y":{"kind":"threshold","name":"rsi.mid"}}}]}"#,
    );
    assert!(e.reason.contains("no bar series"));
    let e = err(
        r#"{"id":"v","theme":"neon","x":{"kind":"time"},"panels":[{"id":0,"y":{"kind":"linear"}}],
            "layers":[{"id":"c","mark":"candle","series":"px","panel":0}]}"#,
    );
    assert_eq!(e.path, "/theme");
    let p = support::provider();
    let v = support::view(&one(
        r#"{"id":"x","mark":"line","series":"sma20","panel":0}"#,
    ));
    assert!(compile(&v, &p, &CompileOptions::new(100, 600)).is_err());
}

#[test]
fn emphasis_target_must_exist() {
    let p = support::provider();
    let mut o = CompileOptions::new(1200, 600);
    o.emphasis =
        vec![serde_json::from_str(r#"{"target":{"layer":"nope"},"kind":"focus"}"#).unwrap()];
    let v = support::view(&one(
        r#"{"id":"x","mark":"line","series":"sma20","panel":0}"#,
    ));
    let e = compile(&v, &p, &o).unwrap_err();
    assert_eq!(e.kind, ErrorKind::NotFound);
    assert!(e.path.ends_with("/target/layer"));
}

#[test]
fn x_channel_only_on_rules() {
    let e = err(&one(
        r#"{"id":"x","mark":"line","series":"sma20","panel":0,"encoding":{"x":"t"}}"#,
    ));
    assert_eq!(e.path, "/encoding/x");
}
