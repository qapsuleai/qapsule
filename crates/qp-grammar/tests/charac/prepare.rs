//! `prepare` (one layer into value rows) through `compile`: every layer mark × data source
//! (series, fact, finding, constant with an origin, none) × look, the glyph styles, the
//! compile options around it, and the error paths (kind, layer, path, message).

use qp_grammar::{compile, CompileOptions, Cursor};
use qp_schema::grammar::View;
use qp_schema::protocol::Language;
use qp_schema::scene::{Prim, SceneDesc};

use crate::fixture::{self, compile_in, looks, shape};
use crate::snap::{err_line, scene_digest, Book};
use crate::support;

const W: u32 = 900;
const H: u32 = 520;

/// Fact origin (`"f-brk"` unless `fact` names another).
fn o(fact: &str, field: &str) -> String {
    format!(r#"{{"kind":"fact","fact":"{fact}","field":"{field}"}}"#)
}

/// The layer cases: `(name, layer json without id)`.
#[allow(clippy::too_many_lines)]
fn cases() -> Vec<(String, String)> {
    let th = |n: &str| format!(r#"{{"kind":"threshold","name":"{n}"}}"#);
    let last = r#"{"kind":"series","series":"px","field":"c","at":"last"}"#;
    let mut c: Vec<(&str, String)> = vec![
        // candle
        ("candle.series", r#""mark":"candle","series":"px","panel":0"#.into()),
        ("candle.series.values", r#""mark":"candle","series":"rsi14","panel":1"#.into()),
        ("candle.points", r#""mark":"candle","series":"q","panel":0"#.into()),
        ("candle.fact", r#""mark":"candle","fact":"f-brk","panel":0"#.into()),
        ("candle.finding", r#""mark":"candle","finding":"fnd-brk","panel":0"#.into()),
        ("candle.none", r#""mark":"candle","panel":0"#.into()),
        // bar
        ("bar.series.default", r#""mark":"bar","series":"macd","panel":2"#.into()),
        (
            "bar.series.when",
            format!(
                r#""mark":"bar","series":"macd","panel":2,"encoding":{{"y":"histogram","color":{{"when":{{"field":"histogram","op":"ge","than":0}},"then":"up","else":"down"}}}},"origin":{{"/encoding/color/when/than":{}}}"#,
                th("macd.zero")
            ),
        ),
        (
            "bar.series.y2",
            format!(
                r#""mark":"bar","series":"rsi14","panel":1,"encoding":{{"y":"rsi","y2":50}},"origin":{{"/encoding/y2":{}}}"#,
                th("rsi.mid")
            ),
        ),
        ("bar.series.volume", r#""mark":"bar","series":"px","panel":2,"encoding":{"y":"v"}"#.into()),
        ("bar.fact", r#""mark":"bar","fact":"f-rsi","panel":1,"encoding":{"y":"level"}"#.into()),
        ("bar.finding", r#""mark":"bar","finding":"fnd-rsi","panel":1,"encoding":{"y":"level"}"#.into()),
        ("bar.const", format!(r#""mark":"bar","panel":1,"encoding":{{"y":70}},"origin":{{"/encoding/y":{}}}"#, th("rsi.overbought"))),
        // line
        ("line.series.main", r#""mark":"line","series":"sma20","panel":0"#.into()),
        ("line.series.multi", r#""mark":"line","series":"bb","panel":0"#.into()),
        ("line.series.field", r#""mark":"line","series":"bb","panel":0,"encoding":{"y":"upper","color":"s3","width":2,"opacity":0.6}"#.into()),
        ("line.series.bars", r#""mark":"line","series":"px","panel":0,"encoding":{"y":"h"}"#.into()),
        (
            "line.series.steps",
            format!(
                r#""mark":"line","series":"rsi14","panel":1,"encoding":{{"color":{{"field":"rsi","thresholds":[30,70],"colors":["down","muted","up"]}}}},"origin":{{"/encoding/color/thresholds/0":{},"/encoding/color/thresholds/1":{}}}"#,
                th("rsi.oversold"),
                th("rsi.overbought")
            ),
        ),
        ("line.points", r#""mark":"line","series":"q","panel":0"#.into()),
        ("line.fact", r#""mark":"line","fact":"f-rsi","panel":1,"encoding":{"y":"level"}"#.into()),
        ("line.finding", r#""mark":"line","finding":"fnd-two","panel":0,"encoding":{"y":"level"}"#.into()),
        ("line.const", format!(r#""mark":"line","panel":0,"encoding":{{"y":120}},"origin":{{"/encoding/y":{last}}}"#)),
        // area
        ("area.series", r#""mark":"area","series":"sma50","panel":0,"encoding":{"opacity":0.3}"#.into()),
        (
            "area.series.y2.when",
            r#""mark":"area","series":"px","panel":0,"encoding":{"y":"c","y2":{"series":"sma50","field":"sma"},"color":{"when":{"field":"c","op":"ge","than":{"series":"sma50","field":"sma"}},"then":"up","else":"down"}}"#.into(),
        ),
        (
            "area.extent",
            format!(
                r#""mark":"area","series":"sma20","panel":0,"origin":{{"/encoding/x":{},"/encoding/x2":{}}}"#,
                o("f-brk", "geometry.0.from_bar"),
                o("f-brk", "geometry.0.to_bar")
            ),
        ),
        ("area.fact", r#""mark":"area","fact":"f-brk","panel":0,"encoding":{"y":"level"}"#.into()),
        ("area.finding", r#""mark":"area","finding":"fnd-brk","panel":0,"encoding":{"y":"level"}"#.into()),
        ("area.const", format!(r#""mark":"area","panel":0,"encoding":{{"y":120}},"origin":{{"/encoding/y":{last}}}"#)),
        // band
        ("band.series", r#""mark":"band","series":"bb","panel":0,"encoding":{"y":"upper","y2":"lower","color":"accent","opacity":0.2}"#.into()),
        ("band.series.ref", r#""mark":"band","series":"sma20","panel":0,"encoding":{"y":"sma","y2":{"series":"sma50","field":"sma"}}"#.into()),
        ("band.points", r#""mark":"band","series":"q","panel":0,"encoding":{"y":"v","y2":{"series":"q","field":"v"}}"#.into()),
        ("band.no_y2", r#""mark":"band","series":"bb","panel":0,"encoding":{"y":"upper"}"#.into()),
        ("band.no_y", r#""mark":"band","series":"bb","panel":0"#.into()),
        (
            "band.extent",
            format!(
                r#""mark":"band","panel":0,"origin":{{"/encoding/y":{},"/encoding/y2":{},"/encoding/x":{},"/encoding/x2":{}}}"#,
                o("f-sq", "u"),
                o("f-sq", "l"),
                o("f-sq", "geometry.0.from_bar"),
                o("f-sq", "geometry.0.to_bar")
            ),
        ),
        ("band.fact", r#""mark":"band","fact":"f-sq","panel":0,"encoding":{"y":"u","y2":"l"}"#.into()),
        ("band.finding", r#""mark":"band","finding":"fnd-sq","panel":0,"encoding":{"y":"u","y2":"l"}"#.into()),
        // rule
        ("rule.threshold", format!(r#""mark":"rule","panel":1,"encoding":{{"y":70,"color":"warn"}},"origin":{{"/encoding/y":{}}}"#, th("rsi.overbought"))),
        ("rule.threshold.string", format!(r#""mark":"rule","panel":1,"encoding":{{"y":"30"}},"origin":{{"/encoding/y":{}}}"#, th("rsi.oversold"))),
        ("rule.no_origin", r#""mark":"rule","panel":0,"encoding":{"y":123.45}"#.into()),
        ("rule.series_last", format!(r#""mark":"rule","panel":0,"origin":{{"/encoding/y":{last}}}"#)),
        ("rule.fact_origin", format!(r#""mark":"rule","panel":0,"origin":{{"/encoding/y":{}}}"#, o("f-brk", "level"))),
        ("rule.vertical", format!(r#""mark":"rule","panel":0,"origin":{{"/encoding/x":{}}}"#, o("f-brk", "geometry.0.to_bar"))),
        (
            "rule.segment",
            format!(
                r#""mark":"rule","panel":0,"origin":{{"/encoding/y":{},"/encoding/y2":{},"/encoding/x":{},"/encoding/x2":{}}}"#,
                o("f-div", "a"),
                o("f-div", "b"),
                o("f-div", "geometry.0.points.0.bar"),
                o("f-div", "geometry.0.points.1.bar")
            ),
        ),
        ("rule.fact", r#""mark":"rule","fact":"f-rsi","panel":1,"encoding":{"y":"level"}"#.into()),
        ("rule.finding", r#""mark":"rule","finding":"fnd-rsi","panel":1,"encoding":{"y":"level"}"#.into()),
        ("rule.series", r#""mark":"rule","series":"sma20","panel":0"#.into()),
        // point
        ("point.series", r#""mark":"point","series":"rsi14","panel":1"#.into()),
        ("point.series.size", r#""mark":"point","series":"px","panel":2,"encoding":{"y":"v","size":{"field":"v","range":[2,9]}}"#.into()),
        ("point.fact", r#""mark":"point","fact":"f-brk","panel":0,"encoding":{"y":"value"}"#.into()),
        ("point.fact.level", r#""mark":"point","fact":"f-rsi","panel":1,"encoding":{"y":"level","size":8,"color":"accent"}"#.into()),
        ("point.finding", r#""mark":"point","finding":"fnd-two","panel":0,"encoding":{"y":"value"}"#.into()),
        (
            "point.const",
            format!(
                r#""mark":"point","panel":0,"origin":{{"/encoding/x":{},"/encoding/y":{}}}"#,
                o("f-div", "geometry.0.points.1.bar"),
                o("f-div", "b")
            ),
        ),
        ("point.none", r#""mark":"point","panel":0"#.into()),
        // text
        ("text.series", r#""mark":"text","series":"px","panel":0,"encoding":{"text":"c"}"#.into()),
        ("text.fact", r#""mark":"text","fact":"f-rsi","panel":1,"encoding":{"y":"level","text":"text"}"#.into()),
        ("text.fact.unknown", r#""mark":"text","fact":"f-rsi","panel":1,"encoding":{"y":"level","text":"nope"}"#.into()),
        ("text.finding.claim", r#""mark":"text","finding":"fnd-rsi","panel":1,"encoding":{"y":"level","text":"claim"}"#.into()),
        ("text.none", r#""mark":"text","panel":0,"encoding":{"text":"claim"}"#.into()),
    ];
    // Panel-positioned capsule texts: every text field a capsule scene reads.
    let pos = r#""x":{"panel":0,"px":8},"y":{"panel":0.1,"px":4}"#;
    let wrap = r#""x":{"panel":0,"px":8},"x2":{"panel":0.5},"y":{"panel":0.2}"#;
    let mut owned: Vec<(String, String)> = c.drain(..).map(|(a, b)| (a.to_owned(), b)).collect();
    for (f, field) in [
        ("fnd-brk", "headline"),
        ("fnd-brk", "ledger"),
        ("fnd-brk", "detail"),
        ("fnd-brk", "claim"),
        ("fnd-parts", "headline"),
        ("fnd-parts", "ledger"),
        ("fnd-lock", "headline"),
        ("fnd-rsi", "headline"),
        ("fnd-brk", "open.basis"),
        ("fnd-brk", "open.why"),
        ("fnd-lock", "open.basis"),
        ("fnd-brk", "open.trust"),
        ("fnd-brk", "open.h.ledger"),
    ] {
        owned.push((
            format!("text.panel.{f}.{field}"),
            format!(
                r#""mark":"text","finding":"{f}","panel":0,"encoding":{{{pos},"text":"{field}"}}"#
            ),
        ));
    }
    for k in ask_fields()
        .into_iter()
        .filter(|k| k.contains("/a/") || k.contains("/q/who"))
    {
        owned.push((
            format!("text.panel.fnd-brk.{k}"),
            format!(
                r#""mark":"text","finding":"fnd-brk","panel":0,"encoding":{{{pos},"text":"{k}"}}"#
            ),
        ));
    }
    owned.push((
        "text.panel.wrap".into(),
        format!(r#""mark":"text","finding":"fnd-brk","panel":0,"encoding":{{{wrap},"text":"headline","size":14}}"#),
    ));
    owned.push((
        "text.panel.no_data".into(),
        format!(r#""mark":"text","panel":0,"encoding":{{{pos},"text":"headline"}}"#),
    ));
    // glyph
    for (n, l) in [
        (
            "glyph.finding",
            format!(
                r#""mark":"glyph","finding":"fnd-brk","panel":0,"encoding":{{{pos},"size":40}}"#
            ),
        ),
        (
            "glyph.finding.bare",
            r#""mark":"glyph","finding":"fnd-div","panel":0"#.into(),
        ),
        (
            "glyph.no_capsule",
            r#""mark":"glyph","finding":"fnd-rsi","panel":0"#.into(),
        ),
        (
            "glyph.fact",
            r#""mark":"glyph","fact":"f-brk","panel":0"#.into(),
        ),
        (
            "glyph.series",
            r#""mark":"glyph","series":"px","panel":0"#.into(),
        ),
        ("glyph.none", r#""mark":"glyph","panel":0"#.into()),
    ] {
        owned.push((n.into(), l));
    }
    // invalid combinations
    for (n, l) in [
        ("err.two_sources", r#""mark":"line","series":"sma20","fact":"f-brk","panel":0"#.to_owned()),
        ("err.three_sources", r#""mark":"point","series":"sma20","fact":"f-brk","finding":"fnd-brk","panel":0"#.to_owned()),
        ("err.unknown_series", r#""mark":"line","series":"nope","panel":0"#.to_owned()),
        ("err.unknown_field", r#""mark":"line","series":"bb","panel":0,"encoding":{"y":"uper"}"#.to_owned()),
        ("err.unknown_ref_field", r#""mark":"band","series":"bb","panel":0,"encoding":{"y":"upper","y2":{"series":"sma50","field":"smaa"}}"#.to_owned()),
        ("err.unknown_fact", r#""mark":"point","fact":"nope","panel":0"#.to_owned()),
        ("err.unknown_finding", r#""mark":"point","finding":"nope","panel":0"#.to_owned()),
        ("err.finding_missing_fact", r#""mark":"point","finding":"fnd-missing","panel":0"#.to_owned()),
        ("err.opacity", r#""mark":"line","series":"sma20","panel":0,"encoding":{"opacity":1.5}"#.to_owned()),
        ("err.width", r#""mark":"line","series":"sma20","panel":0,"encoding":{"width":0}"#.to_owned()),
        ("err.x_on_line", format!(r#""mark":"line","series":"sma20","panel":0,"origin":{{"/encoding/x":{}}}"#, o("f-brk", "bar"))),
        ("err.x2_on_point", format!(r#""mark":"point","fact":"f-brk","panel":0,"encoding":{{"y":"value"}},"origin":{{"/encoding/x2":{}}}"#, o("f-brk", "bar"))),
        ("err.x2_without_x", format!(r#""mark":"band","series":"bb","panel":0,"encoding":{{"y":"upper","y2":"lower"}},"origin":{{"/encoding/x2":{}}}"#, o("f-brk", "bar"))),
        ("err.x_not_a_bar", r#""mark":"rule","panel":0,"encoding":{"x":"c"}"#.to_owned()),
        ("err.bad_color", r#""mark":"line","series":"sma20","panel":0,"encoding":{"color":"chartreuse-ish"}"#.to_owned()),
        ("err.bad_size", r#""mark":"point","series":"rsi14","panel":1,"encoding":{"size":-1}"#.to_owned()),
        ("err.threshold_no_origin", r#""mark":"line","series":"rsi14","panel":1,"encoding":{"color":{"field":"rsi","thresholds":[50],"colors":["down","up"]}}"#.to_owned()),
        ("err.panel", r#""mark":"line","series":"sma20","panel":9"#.to_owned()),
    ] {
        owned.push((n.into(), l));
    }
    owned
}

/// The `ask/…` text fields the `nobra` open design reads (from its own view).
pub fn ask_fields() -> Vec<String> {
    let p = fixture::provider(Language::Tr);
    let ctx = qp_grammar::open::OpenCtx {
        ask: Some("who"),
        lang: Language::Tr,
        check: None,
        frame: [390, 640],
    };
    let v = qp_grammar::open::capsule_view(
        Some("nobra"),
        &qp_grammar::look::R4,
        "fnd-brk",
        120,
        &p,
        "v",
        &ctx,
    )
    .expect("nobra view");
    v.layers
        .iter()
        .filter_map(|l| l.encoding.text.clone())
        .filter(|t| t.starts_with("ask/"))
        .collect()
}

fn view_with(layer: &str) -> View {
    support::view(&format!(
        r#"{{"id":"m","x":{{"kind":"time"}},
        "panels":[{{"id":0,"name":"price","y":{{"kind":"linear"}}}},
                  {{"id":1,"name":"rsi","y":{{"kind":"linear","domain":[0,100]}}}},
                  {{"id":2,"name":"macd","y":{{"kind":"linear"}}}}],
        "layers":[{{"id":"candles","mark":"candle","series":"px","panel":0}},{layer}]}}"#
    ))
}

/// The case layer `t` in a scene: data, fields, prims, items, first text.
fn layer_summary(s: &SceneDesc) -> String {
    let Some(l) = s
        .panels
        .iter()
        .flat_map(|p| &p.layers)
        .find(|l| l.id == "t")
    else {
        return format!("no layer t; {}", shape(s));
    };
    let text = l.prims.iter().find_map(|q| match q {
        Prim::Text { text, .. } => Some(text.as_str()),
        _ => None,
    });
    format!(
        "{:?} {} fields={} consts={} prims={} items={} text={}",
        l.mark,
        serde_json::to_string(&l.data).unwrap(),
        l.fields.join(","),
        l.constants.keys().cloned().collect::<Vec<_>>().join(","),
        l.prims.len(),
        l.items.len(),
        text.unwrap_or("-")
    )
}

#[test]
fn charac_prepare_layer_matrix() {
    let p = fixture::provider(Language::Tr);
    let mut ok = Book::new("prepare-matrix");
    let mut errs = Book::new("prepare-errors");
    let looks = looks();
    for (name, json) in cases() {
        let v = view_with(&format!(r#"{{"id":"t",{json}}}"#));
        let mut digests = Vec::new();
        let mut first: Option<String> = None;
        let mut err_lines: Vec<String> = Vec::new();
        for look in &looks {
            match compile_in(&v, &p, look, W, H, None) {
                Ok(s) => {
                    digests.push(format!("{look}={}", &scene_digest(&s)[..10]));
                    first.get_or_insert_with(|| layer_summary(&s));
                }
                Err(e) => {
                    let l = err_line(&e);
                    if !err_lines.contains(&l) {
                        err_lines.push(l);
                    }
                }
            }
        }
        if !digests.is_empty() {
            // The light theme of the default look, too.
            let mut o = CompileOptions::new(W, H);
            o.theme = Some("light".into());
            if let Ok(s) = compile(&v, &p, &o) {
                digests.push(format!("light={}", &scene_digest(&s)[..10]));
            }
            ok.put(&name, &digests.join(","), first.as_deref().unwrap_or("-"));
        }
        if !err_lines.is_empty() {
            errs.note(&name, &err_lines.join(" || "));
        }
    }
    ok.check();
    errs.check();
}

#[test]
fn charac_glyph_styles() {
    let mut b = Book::new("glyph-styles");
    let p = fixture::provider(Language::Tr);
    let v = view_with(
        r#"{"id":"t","mark":"glyph","finding":"FND","panel":0,"encoding":{"x":{"panel":0.1},"y":{"panel":0.1},"size":48}}"#,
    );
    for f in fixture::capsule_findings() {
        let json = serde_json::to_string(&v).unwrap().replace("FND", f);
        let v: View = serde_json::from_str(&json).unwrap();
        for alphabet in qp_grammar::glyph::ALPHABETS {
            for outcome in qp_grammar::glyph::outcome::names() {
                for look in ["r4", "tavek"] {
                    let mut o = CompileOptions::new(W, H);
                    o.look = Some(look.into());
                    o.alphabet = Some(alphabet.into());
                    o.outcome = Some(outcome.into());
                    let case = format!("{f}.{alphabet}.{outcome}.{look}");
                    match compile(&v, &p, &o) {
                        Ok(s) => b.put(&case, &scene_digest(&s), &layer_summary(&s)),
                        Err(e) => b.note(&case, &err_line(&e)),
                    }
                }
            }
        }
    }
    for (case, alphabet, outcome) in [
        ("bad.alphabet", Some("nope"), None),
        ("bad.outcome", None, Some("nope")),
    ] {
        let mut o = CompileOptions::new(W, H);
        o.alphabet = alphabet.map(str::to_owned);
        o.outcome = outcome.map(str::to_owned);
        let v = view_with(r#"{"id":"t","mark":"glyph","finding":"fnd-brk","panel":0}"#);
        b.note(
            case,
            &compile(&v, &p, &o).map_or_else(|e| err_line(&e), |s| shape(&s)),
        );
    }
    b.check();
}

const FULL: &str = r#"{"id":"full","title":"Full view","x":{"kind":"time"},
  "panels":[{"id":0,"name":"price","y":{"kind":"linear"}},
            {"id":1,"name":"rsi","y":{"kind":"linear","domain":[0,100]}},
            {"id":2,"name":"macd","y":{"kind":"linear"}}],
  "layers":[{"id":"candles","mark":"candle","series":"px","panel":0},
    {"id":"bb","mark":"band","series":"bb","panel":0,"encoding":{"y":"upper","y2":"lower","opacity":0.15}},
    {"id":"sma20","mark":"line","series":"sma20","panel":0},
    {"id":"fact","mark":"point","fact":"f-brk","panel":0,"encoding":{"y":"value"}},
    {"id":"rsi","mark":"line","series":"rsi14","panel":1},
    {"id":"ob","mark":"rule","panel":1,"encoding":{"y":70},"origin":{"/encoding/y":{"kind":"threshold","name":"rsi.overbought"}}},
    {"id":"hist","mark":"bar","series":"macd","panel":2,"encoding":{"y":"histogram"}}],
  "emphasis":[{"target":{"layer":"sma20"},"kind":"highlight"}]}"#;

#[test]
fn charac_compile_options() {
    let mut b = Book::new("compile-options");
    let p = fixture::provider(Language::Tr);
    let v = support::view(FULL);
    let base = || CompileOptions::new(W, H);
    let mut cases: Vec<(String, CompileOptions)> = vec![("default".into(), base())];
    let mut o = base();
    o.range = Some((120, 170));
    cases.push(("range".into(), o));
    let mut o = base();
    o.range = Some((150, 400));
    cases.push(("range.clamped".into(), o));
    let mut o = base();
    o.max_points = Some(60);
    cases.push(("max_points".into(), o));
    let mut o = CompileOptions::new(200, 120);
    o.lod = false;
    cases.push(("small.no_lod".into(), o));
    let mut o = CompileOptions::new(200, 120);
    o.lod = true;
    cases.push(("small.lod".into(), o));
    let mut o = base();
    o.roles = true;
    cases.push(("roles".into(), o));
    let mut o = base();
    o.theme = Some("light".into());
    cases.push(("light".into(), o));
    let mut o = base();
    o.emphasis = serde_json::from_str(r#"[{"target":{"layer":"rsi"},"kind":"focus"},{"target":{"from_bar":150,"to_bar":160},"kind":"dim"},{"target":{"item":"f-brk"},"kind":"highlight"}]"#).unwrap();
    cases.push(("emphasis".into(), o));
    for look in looks() {
        for (n, cur) in [
            (
                "cursor",
                Some(Cursor {
                    bar: 150,
                    price: None,
                }),
            ),
            (
                "cursor.price",
                Some(Cursor {
                    bar: 199,
                    price: Some(118.0),
                }),
            ),
            ("nocursor", None),
        ] {
            let mut o = base();
            o.look = Some(look.into());
            o.cursor = cur;
            cases.push((format!("{look}.{n}"), o));
        }
    }
    for (bad, f) in [
        ("bad.look", "look"),
        ("bad.face", "face"),
        ("bad.theme", "theme"),
    ] {
        let mut o = base();
        match f {
            "look" => o.look = Some("nope".into()),
            "face" => o.face = Some("nope".into()),
            _ => o.theme = Some("nope".into()),
        }
        cases.push((bad.into(), o));
    }
    let mut o = CompileOptions::new(100, 50);
    o.look = None;
    cases.push(("bad.size".into(), o));
    for (n, o) in cases {
        match compile(&v, &p, &o) {
            Ok(s) => b.put(&n, &scene_digest(&s), &shape(&s)),
            Err(e) => b.note(&n, &err_line(&e)),
        }
    }
    b.check();
}
