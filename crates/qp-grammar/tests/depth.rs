//! Depth rules (`docs/GRAMMAR.md` §13) and the RD1 transcript 16 shape of the chart view.

mod common;

use qp_grammar::depth::{capsule_view, chart_view, effective_depth, present, sentence};
use qp_grammar::{compile, CompileOptions, MemProvider};
use qp_schema::grammar::{EmphasisKind, Mark, Origin};
use qp_schema::protocol::Profile;
use qp_schema::{Author, Depth, Fact, Finding, Provenance};
use serde_json::{json, Value};

fn prov() -> Provenance {
    Provenance {
        author: Author::Engine {
            name: "t".into(),
            version: "0".into(),
        },
        at: 0,
        based_on: Vec::new(),
        approved_by: None,
    }
}

fn doc(bar: u64, headline: bool) -> MemProvider {
    let bars = common::sample_bars();
    let fact = Fact {
        id: "fact-rsi-cross70".into(),
        kind: "cross_above".into(),
        series: "rsi14".into(),
        bar,
        t: bars[bar as usize].t,
        text: "RSI(14) crossed 70".into(),
        evidence: [("level".to_owned(), 70.0)].into_iter().collect(),
        evidence_ref: None,
        provenance: prov(),
    };
    let finding = Finding {
        id: "fnd-overbought".into(),
        claim: "THYAO looks overbought".into(),
        facts: vec![fact.id.clone()],
        confidence: 0.62,
        invalidation: qp_schema::Condition::Text { text: "-".into() },
        provenance: prov(),
    };
    let p = common::provider().with_fact(fact).with_finding(finding);
    if headline {
        p.with_headline("fnd-overbought", "Overbought: RSI above 70")
    } else {
        p
    }
}

fn profile(depth: Depth, series_data: bool, max_points: u32) -> Profile {
    Profile {
        depth,
        series_data,
        max_points,
        max_glyphs: 6,
        speech: false,
    }
}

/// Subset match: every key of `want` is in `got` with a matching value; arrays match
/// element-wise with equal length (the conformance runner's rule).
fn subset(want: &Value, got: &Value) -> bool {
    match (want, got) {
        (Value::Object(w), Value::Object(g)) => w
            .iter()
            .all(|(k, v)| g.get(k).is_some_and(|gv| subset(v, gv))),
        (Value::Array(w), Value::Array(g)) => {
            w.len() == g.len() && w.iter().zip(g).all(|(a, b)| subset(a, b))
        }
        _ => want == got,
    }
}

#[test]
fn chart_view_keeps_the_transcript_16_shape() {
    let p = doc(170, true);
    let v = chart_view("fnd-overbought", &p, "v-overbought").unwrap();
    let got = serde_json::to_value(&v).unwrap();
    let want = json!({
        "id": "v-overbought",
        "x": {"kind": "time"},
        "layers": [{"mark":"candle","series":"px","panel":0},
                   {"mark":"line","series":"rsi14","panel":1},
                   {"mark":"rule","panel":1,"encoding":{"y":"70"}}]
    });
    assert!(subset(&want, &got), "{got}");
    // x.domain = [t(bar - 30), t(last)].
    let bars = common::sample_bars();
    assert_eq!(v.x.domain, Some([bars[140].t as f64, bars[199].t as f64]));
    assert_eq!(
        v.layers[2].origin["/encoding/y"],
        Origin::Fact {
            fact: "fact-rsi-cross70".into(),
            field: "level".into()
        }
    );
    assert_eq!(v.emphasis.len(), 1);
    assert_eq!(v.emphasis[0].kind, EmphasisKind::Highlight);
    assert_eq!(
        v.emphasis[0].target.item.as_deref(),
        Some("fact-rsi-cross70")
    );
    // It compiles: highlight strip at the fact bar, 60 visible bars.
    let s = compile(&v, &p, &CompileOptions::new(1200, 600)).unwrap();
    assert_eq!((s.x.from_bar, s.x.to_bar), (140, 199));
    assert_eq!(s.emphasis[0].from_bar, Some(170));
    assert_eq!(s.panels[1].layers[1].constants["/encoding/y"].value, 70.0);
    // Near the start the look-back is clamped at bar 0.
    let v = chart_view("fnd-overbought", &doc(10, true), "x").unwrap();
    assert_eq!(v.x.domain.unwrap()[0], bars[0].t as f64);
}

#[test]
fn capsule_view_rule() {
    let p = doc(170, true);
    let v = capsule_view("fnd-overbought", 60, &p, "cap").unwrap();
    assert_eq!(v.panels.len(), 1);
    let marks: Vec<Mark> = v.layers.iter().map(|l| l.mark).collect();
    assert_eq!(marks, vec![Mark::Line, Mark::Rule, Mark::Point]);
    assert_eq!(v.layers[2].fact.as_deref(), Some("fact-rsi-cross70"));
    assert_eq!(v.title.as_deref(), Some("Overbought: RSI above 70"));
    let bars = common::sample_bars();
    assert_eq!(v.x.domain, Some([bars[140].t as f64, bars[199].t as f64]));
    let s = compile(&v, &p, &CompileOptions::new(400, 200)).unwrap();
    assert_eq!(s.x.to_bar - s.x.from_bar + 1, 60);
    assert_eq!(s.panels[0].layers[2].items.len(), 1);
    // Fact bar outside the last max_points bars: no point.
    let v = capsule_view("fnd-overbought", 20, &p, "cap").unwrap();
    assert_eq!(v.layers.len(), 2);
}

#[test]
fn present_by_profile() {
    let p = doc(170, false);
    // Watch: sentence only (the claim, no headline).
    let watch = present(
        "fnd-overbought",
        Depth::Chart,
        &profile(Depth::Sentence, false, 0),
        &p,
    )
    .unwrap();
    assert_eq!(watch.depth, Depth::Sentence);
    assert_eq!(watch.text, "THYAO looks overbought");
    assert_eq!(watch.view, None);
    assert_eq!(watch.confidence, None);
    // Phone: capsule over 60 bars.
    let phone = present(
        "fnd-overbought",
        Depth::Chart,
        &profile(Depth::Capsule, true, 60),
        &p,
    )
    .unwrap();
    assert_eq!(phone.depth, Depth::Capsule);
    assert_eq!(phone.confidence, Some(0.62));
    let v = phone.view.unwrap();
    assert_eq!(v.id, "capsule-fnd-overbought");
    // No series data: text only even at capsule depth.
    let lean = present(
        "fnd-overbought",
        Depth::Capsule,
        &profile(Depth::Capsule, false, 0),
        &p,
    )
    .unwrap();
    assert_eq!(lean.view, None);
    // Desktop: chart.
    let desk = present(
        "fnd-overbought",
        Depth::Workspace,
        &profile(Depth::Chart, true, 500),
        &p,
    )
    .unwrap();
    assert_eq!(desk.depth, Depth::Chart);
    assert_eq!(desk.view.unwrap().id, "present-fnd-overbought");
    assert_eq!(
        sentence("fnd-overbought", &doc(170, true)).unwrap(),
        "Overbought: RSI above 70"
    );
    assert_eq!(
        effective_depth(Depth::Workspace, &profile(Depth::Workspace, true, 1)),
        Depth::Chart
    );
    // Deterministic.
    let a = present(
        "fnd-overbought",
        Depth::Chart,
        &profile(Depth::Chart, true, 500),
        &p,
    )
    .unwrap();
    let b = present(
        "fnd-overbought",
        Depth::Chart,
        &profile(Depth::Chart, true, 500),
        &p,
    )
    .unwrap();
    assert_eq!(a, b);
    assert!(present("nope", Depth::Chart, &profile(Depth::Chart, true, 500), &p).is_err());
}

#[test]
fn chart_view_for_a_fact_on_bars() {
    let bars = common::sample_bars();
    let fact = Fact {
        id: "f-brk".into(),
        kind: "breakout".into(),
        series: "px".into(),
        bar: 185,
        t: bars[185].t,
        text: "breakout".into(),
        evidence: [("level".to_owned(), 121.3)].into_iter().collect(),
        evidence_ref: None,
        provenance: prov(),
    };
    let finding = Finding {
        id: "fnd-brk".into(),
        claim: "Breakout".into(),
        facts: vec!["f-brk".into()],
        confidence: 0.5,
        invalidation: qp_schema::Condition::Text { text: "-".into() },
        provenance: prov(),
    };
    let p = common::provider().with_fact(fact).with_finding(finding);
    let v = chart_view("fnd-brk", &p, "v").unwrap();
    assert_eq!(v.layers.len(), 1);
    assert_eq!(v.panels.len(), 1);
    let c = capsule_view("fnd-brk", 60, &p, "c").unwrap();
    assert_eq!(
        c.layers[0].encoding.y,
        Some(qp_schema::grammar::Position::Name("c".into()))
    );
    compile(&c, &p, &CompileOptions::new(400, 200)).unwrap();
}
