//! The five R2 views (`rounds/R2/tasks/VIEWS.md` V1–V5), every mark, colour modes, log
//! scale, gaps, points beyond the last bar, emphasis, multi-panel layout, themes and
//! determinism.

mod common;

use qp_grammar::scale::{slot_x, value_at, value_y};
use qp_grammar::{compile, CompileOptions, MemProvider};
use qp_schema::grammar::{Emphasis, EmphasisKind, Mark, Origin, ScaleKind};
use qp_schema::protocol::Target;
use qp_schema::scene::{Prim, SceneDesc, SceneLayer};
use qp_schema::{Author, Fact, Finding, Point, Provenance};

fn opts() -> CompileOptions {
    CompileOptions::new(1200, 600)
}

fn layer<'a>(s: &'a SceneDesc, id: &str) -> &'a SceneLayer {
    s.panels
        .iter()
        .flat_map(|p| p.layers.iter())
        .find(|l| l.id == id)
        .unwrap_or_else(|| panic!("layer {id}"))
}

fn prov() -> Provenance {
    Provenance {
        author: Author::Engine {
            name: "test".into(),
            version: "0".into(),
        },
        at: 0,
        based_on: Vec::new(),
        approved_by: None,
    }
}

/// The sample provider plus a fact (RSI crossing 70) and a finding on it.
fn with_facts() -> (MemProvider, u64) {
    let bars = common::sample_bars();
    let rsi = common::study("rsi", &[("period", 14.0)], &bars)["rsi"].clone();
    let bar = (1..rsi.len())
        .find(|i| matches!((rsi[i - 1], rsi[*i]), (Some(a), Some(b)) if a < 70.0 && b >= 70.0))
        .unwrap_or(150) as u64;
    let fact = Fact {
        id: "fact-rsi-cross70".into(),
        kind: "cross_above".into(),
        series: "rsi14".into(),
        bar,
        t: bars[bar as usize].t,
        text: "RSI(14) crossed 70".into(),
        evidence: [("level".to_owned(), 70.0)].into_iter().collect(),
        evidence_ref: None,
        meaning: None,
        provenance: prov(),
    };
    let finding = Finding {
        id: "fnd-overbought".into(),
        claim: "Overbought".into(),
        facts: vec![fact.id.clone()],
        confidence: 0.7,
        invalidation: qp_schema::Condition::Text { text: "x".into() },
        provenance: prov(),
    };
    (
        common::provider().with_fact(fact).with_finding(finding),
        bar,
    )
}

const V1: &str = r#"{"id":"three","x":{"kind":"time"},
  "panels":[{"id":0,"name":"price","y":{"kind":"linear"}},
            {"id":1,"name":"rsi","y":{"kind":"linear","domain":[0,100]}},
            {"id":2,"name":"macd","y":{"kind":"linear"}}],
  "layers":[{"id":"candles","mark":"candle","series":"px","panel":0},
    {"id":"rsi","mark":"line","series":"rsi14","panel":1},
    {"id":"ob","mark":"rule","panel":1,"encoding":{"y":70},
     "origin":{"/encoding/y":{"kind":"threshold","name":"rsi.overbought"}}},
    {"id":"os","mark":"rule","panel":1,"encoding":{"y":30},
     "origin":{"/encoding/y":{"kind":"threshold","name":"rsi.oversold"}}},
    {"id":"hist","mark":"bar","series":"macd","panel":2,
     "encoding":{"y":"histogram",
       "color":{"when":{"field":"histogram","op":"ge","than":0},"then":"up","else":"down"}},
     "origin":{"/encoding/color/when/than":{"kind":"threshold","name":"macd.zero"}}}]}"#;

#[test]
fn v1_three_panels() {
    let p = common::provider();
    let s = compile(&common::view(V1), &p, &opts()).unwrap();
    assert_eq!(s.panels.len(), 3);
    // Weights 3:1:1 over 542 px minus two 8 px gaps.
    let h: Vec<i32> = s
        .panels
        .iter()
        .map(|p| p.rect.bottom - p.rect.top)
        .collect();
    assert_eq!(h, vec![315, 105, 106]);
    assert_eq!(s.panels[0].rect.top, 34);
    assert_eq!(s.panels[1].rect.top, 34 + 315 + 8);
    assert_eq!(s.panels[2].rect.bottom, 576);
    assert_eq!(s.panels[1].y.min, 0.0);
    assert_eq!(s.panels[1].y.max, 100.0);
    let ob = layer(&s, "ob");
    assert_eq!(ob.constants["/encoding/y"].value, 70.0);
    assert!(
        matches!(&ob.constants["/encoding/y"].origin, Origin::Threshold { name } if name == "rsi.overbought")
    );
    assert_eq!(ob.items.len(), 1);
    assert_eq!(ob.items[0].bar, None);
    let ry = value_y(&s.panels[1].y, 70.0).unwrap().round() as i32;
    assert_eq!(
        ob.prims[0],
        Prim::Rect {
            x: 8,
            y: ry,
            w: 1120,
            h: 1,
            color: [130, 136, 150, 255]
        }
    );
    // Histogram: up where >= 0, down elsewhere; base 0; warmup skipped.
    let hist = layer(&s, "hist");
    assert_eq!(hist.mark, Mark::Bar);
    assert!(hist.skipped > 0);
    assert_eq!(hist.items.len() as u64 + hist.skipped, 200);
    let zero = value_y(&s.panels[2].y, 0.0).unwrap().round() as i32;
    for (it, pr) in hist.items.iter().zip(&hist.prims) {
        let v = it.values["histogram"];
        let Prim::Rect { y, h, color, .. } = pr else {
            panic!()
        };
        assert_eq!(
            *color,
            if v >= 0.0 {
                [38, 166, 154, 255]
            } else {
                [239, 83, 80, 255]
            }
        );
        if v >= 0.0 {
            assert_eq!(*y + *h as i32, zero.max(*y + 1));
        } else {
            assert_eq!(*y, zero);
        }
    }
    assert!(hist.constants.contains_key("/encoding/color/when/than"));
    // Month labels only below the bottom panel; vertical grid in every panel.
    let labels: Vec<i32> = s
        .under
        .iter()
        .filter_map(|p| match p {
            Prim::Text { text, baseline, .. } if text.starts_with("2025-") => Some(*baseline),
            _ => None,
        })
        .collect();
    assert!(!labels.is_empty());
    assert!(labels.iter().all(|b| *b == 576 + 16));
    // Sub-panel legend inside the panel's top-left corner.
    assert!(s
        .over
        .iter()
        .any(|p| matches!(p, Prim::Text { text, x, baseline, .. }
        if text.starts_with("RSI14 ") && *x == 12 && *baseline == s.panels[1].rect.top + 12)));
    // A last-value tag per panel (candle close, RSI, histogram).
    let tags = s
        .over
        .iter()
        .filter(|p| matches!(p, Prim::Rect { x: 1129, h: 16, .. }))
        .count();
    assert_eq!(tags, 3);
}

const V2: &str = r#"{"id":"boll","x":{"kind":"time"},
  "panels":[{"id":0,"y":{"kind":"linear"}}],
  "layers":[
    {"id":"band","mark":"band","series":"bb","panel":0,
     "encoding":{"y":"upper","y2":"lower","color":"accent","opacity":0.18}},
    {"id":"mid","mark":"line","series":"bb","panel":0,"encoding":{"y":"middle","color":"accent"}},
    {"id":"candles","mark":"candle","series":"px","panel":0}]}"#;

#[test]
fn v2_bollinger_band() {
    let p = common::provider();
    let s = compile(&common::view(V2), &p, &opts()).unwrap();
    let band = layer(&s, "band");
    assert_eq!(band.fields, vec!["upper", "lower"]);
    assert_eq!(band.skipped, 19);
    assert_eq!(band.items.len(), 181);
    assert_eq!(band.items[0].bar, Some(19));
    // One fill (no gap after warmup), alpha 230 (accent) * 0.18 = 41; before the candles.
    let fills: Vec<&Prim> = band.prims.iter().collect();
    assert_eq!(fills.len(), 1);
    let Prim::Fill { points, color } = fills[0] else {
        panic!()
    };
    assert_eq!(color[3], 41);
    assert_eq!(points.len(), 2 * 181);
    let order: Vec<&str> = s.panels[0].layers.iter().map(|l| l.id.as_str()).collect();
    assert_eq!(order, vec!["band", "mid", "candles"]);
    // Legends: band (label only) and the middle line, right-aligned, in layer order.
    let legends: Vec<(String, i32)> = s
        .over
        .iter()
        .filter_map(|p| match p {
            Prim::Text {
                text,
                x,
                baseline: 23,
                px,
                ..
            } if *px == 11.0 => Some((text.clone(), *x)),
            _ => None,
        })
        .collect();
    assert_eq!(legends.len(), 2);
    assert_eq!(legends[0].0, "BB(20,2)");
    assert!(legends[1].0.starts_with("BB(20,2) "));
    assert_eq!(legends[1].1, 1192);
    assert!(legends[0].1 < legends[1].1);
    // Auto domain covers the bands.
    let upper_max = band
        .items
        .iter()
        .map(|i| i.values["upper"])
        .fold(f64::MIN, f64::max);
    assert!(s.panels[0].y.max > upper_max);
}

fn fan_provider() -> MemProvider {
    let bars = common::sample_bars();
    let last = bars.last().unwrap().clone();
    let mut p = common::provider();
    for (id, z) in [
        ("q05", -1.6),
        ("q20", -0.8),
        ("q35", -0.4),
        ("q65", 0.4),
        ("q80", 0.8),
        ("q95", 1.6),
    ] {
        let pts: Vec<Point> = (0..=20)
            .map(|k| Point {
                t: last.t + k * 86_400_000,
                v: Some(last.c * (1.0 + z * 0.004 * k as f64)),
            })
            .collect();
        p = p.with_points(id, pts);
    }
    p
}

const V3: &str = r#"{"id":"fan","x":{"kind":"time"},
  "panels":[{"id":0,"y":{"kind":"linear"}}],
  "layers":[
    {"id":"candles","mark":"candle","series":"px","panel":0},
    {"id":"b90","mark":"band","series":"q05","panel":0,
     "encoding":{"y":"v","y2":{"series":"q95","field":"v"},"color":"accent","opacity":0.15}},
    {"id":"b60","mark":"band","series":"q20","panel":0,
     "encoding":{"y":"v","y2":{"series":"q80","field":"v"},"color":"accent","opacity":0.22}},
    {"id":"b30","mark":"band","series":"q35","panel":0,
     "encoding":{"y":"v","y2":{"series":"q65","field":"v"},"color":"accent","opacity":0.30}}]}"#;

#[test]
fn v3_fan_beyond_last_bar() {
    let p = fan_provider();
    let s = compile(&common::view(V3), &p, &opts()).unwrap();
    assert_eq!(s.bars, 200);
    assert_eq!(s.x.from_bar, 0);
    assert_eq!(s.x.to_bar, 219, "range extends over the 20 future slots");
    let b90 = layer(&s, "b90");
    assert_eq!(b90.fields, vec!["v", "q95.v"]);
    assert_eq!(b90.items.len(), 21);
    assert_eq!(b90.items[0].bar, Some(199));
    assert_eq!(b90.items[20].bar, Some(219));
    let last_t = common::sample_bars().last().unwrap().t;
    assert_eq!(b90.items[20].t, Some(last_t + 20 * 86_400_000));
    // Increasing opacity layers.
    let alpha = |id: &str| match &layer(&s, id).prims[0] {
        Prim::Fill { color, .. } => color[3],
        _ => panic!(),
    };
    assert!(alpha("b90") < alpha("b60") && alpha("b60") < alpha("b30"));
    // Future slots lie right of the last candle and inside the plot.
    let x219 = slot_x(&s.x, 219);
    assert!(x219 > slot_x(&s.x, 199) && x219 < 1128.0);
    // With a transient viewport the range is not extended.
    let mut o = opts();
    o.range = Some((100, 199));
    let s2 = compile(&common::view(V3), &p, &o).unwrap();
    assert_eq!((s2.x.from_bar, s2.x.to_bar), (100, 199));
    assert_eq!(layer(&s2, "b90").items.len(), 1);
}

#[test]
fn v4_log_scale() {
    let bars = common::growth_bars(200);
    let p = common::provider_with(bars.clone(), "GROWTH");
    let v = common::view(
        r#"{"id":"logv","x":{"kind":"time"},"panels":[{"id":0,"y":{"kind":"log"}}],
            "layers":[{"id":"candles","mark":"candle","series":"px","panel":0}]}"#,
    );
    let s = compile(&v, &p, &opts()).unwrap();
    let y = s.panels[0].y;
    assert_eq!(y.kind, ScaleKind::Log);
    assert!(y.min < bars[0].l && y.max > bars[199].h);
    // Equal ratios take equal pixels: 10→20 and 100→200.
    let d1 = value_y(&y, 10.0).unwrap() - value_y(&y, 20.0).unwrap();
    let d2 = value_y(&y, 100.0).unwrap() - value_y(&y, 200.0).unwrap();
    assert!((d1 - d2).abs() < 1e-9, "{d1} vs {d2}");
    // Round trip through the deterministic log/exp.
    for v in [12.5, 55.0, 333.0] {
        let back = value_at(&y, value_y(&y, v).unwrap());
        assert!((back - v).abs() / v < 1e-12);
    }
    // 1-2-5 grid labels.
    let labels: Vec<String> = s
        .under
        .iter()
        .filter_map(|p| match p {
            Prim::Text { text, x: 1134, .. } => Some(text.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(labels, vec!["10.00", "20.00", "50.00", "100.00", "200.00"]);
    // Linear on the same data differs (convexity), and a log compile is stable.
    assert_eq!(compile(&v, &p, &opts()).unwrap(), s);
}

#[test]
fn log_scale_skips_non_positive_values() {
    let bars = common::sample_bars();
    let vals: Vec<Option<f64>> = (0..200).map(|i| Some(f64::from(i) - 50.0)).collect();
    let p =
        common::provider_with(bars, "S").with_values("lin", "px", vec![("v", vals)], "v", &["v"]);
    let v = common::view(
        r#"{"id":"l","x":{"kind":"time"},"panels":[{"id":0,"y":{"kind":"log"}}],
            "layers":[{"id":"ln","mark":"line","series":"lin","panel":0}]}"#,
    );
    let s = compile(&v, &p, &opts()).unwrap();
    let l = layer(&s, "ln");
    assert_eq!(l.skipped, 51, "values -50..=0 are not drawn on a log scale");
    assert_eq!(l.items.len(), 149);
    assert!(s.panels[0].y.min > 0.0);
}

const V5: &str = r#"{"id":"cond","x":{"kind":"time"},
  "panels":[{"id":0,"y":{"kind":"linear"}},{"id":1,"y":{"kind":"linear","domain":[0,100]}}],
  "layers":[
    {"id":"fill","mark":"area","series":"px","panel":0,
     "encoding":{"y":"c","y2":{"series":"sma50","field":"sma"},"opacity":0.3,
       "color":{"when":{"field":"c","op":"ge","than":{"series":"sma50","field":"sma"}},
                "then":"up","else":"down"}}},
    {"id":"close","mark":"line","series":"px","panel":0,"encoding":{"y":"c","color":"text"}},
    {"id":"rsi","mark":"line","series":"rsi14","panel":1,
     "encoding":{"color":{"field":"rsi","thresholds":[30,70],"colors":["up","muted","down"]}},
     "origin":{"/encoding/color/thresholds/0":{"kind":"threshold","name":"rsi.oversold"},
               "/encoding/color/thresholds/1":{"kind":"threshold","name":"rsi.overbought"}}}]}"#;

#[test]
fn v5_conditional_and_stepped_colour() {
    let p = common::provider();
    let s = compile(&common::view(V5), &p, &opts()).unwrap();
    let fill = layer(&s, "fill");
    let up = [38, 166, 154, 77];
    let down = [239, 83, 80, 77];
    let colors: Vec<[u8; 4]> = fill
        .prims
        .iter()
        .map(|p| match p {
            Prim::Fill { color, .. } => *color,
            _ => panic!(),
        })
        .collect();
    assert!(colors.len() > 2, "the area changes colour");
    assert!(colors.iter().all(|c| *c == up || *c == down));
    assert!(
        colors.windows(2).all(|w| w[0] != w[1]),
        "segments alternate"
    );
    // Split points are at the crossing: the first segment ends where close == sma50, i.e.
    // its last top point equals its last bottom point (the two edges meet).
    let Prim::Fill { points, .. } = &fill.prims[0] else {
        panic!()
    };
    let n = points.len() / 2;
    let (top_end, bot_end) = (points[n - 1], points[n]);
    assert!((top_end[0] - bot_end[0]).abs() < 1e-3);
    assert!(
        (top_end[1] - bot_end[1]).abs() < 1e-2,
        "{top_end:?} {bot_end:?}"
    );
    // ...and not at a bar centre (an interpolated x).
    let centres: Vec<f32> = (0..200).map(|i| slot_x(&s.x, i) as f32 + 0.5).collect();
    assert!(!centres.contains(&top_end[0]));

    // Stepped colours: up below 30, muted between, down at or above 70.
    let rsi = layer(&s, "rsi");
    let palette = [
        [38, 166, 154, 255],
        [130, 136, 150, 255],
        [239, 83, 80, 255],
    ];
    let mut seen = std::collections::BTreeSet::new();
    for pr in &rsi.prims {
        let Prim::Polyline { color, points, .. } = pr else {
            panic!()
        };
        assert!(palette.contains(color));
        seen.insert(*color);
        // Each segment boundary that is a split lies on a threshold line.
        for q in [points[0], points[points.len() - 1]] {
            let v = value_at(&s.panels[1].y, f64::from(q[1]));
            let on_threshold = (v - 30.0).abs() < 0.01 || (v - 70.0).abs() < 0.01;
            let on_bar = centres.contains(&q[0]);
            assert!(on_threshold || on_bar, "segment end {q:?} = {v}");
        }
    }
    assert!(seen.len() >= 2);
    assert_eq!(rsi.constants.len(), 2);
}

#[test]
fn line_breaks_at_gaps() {
    let bars = common::sample_bars();
    let mut vals: Vec<Option<f64>> = bars.iter().map(|b| Some(b.c)).collect();
    for v in &mut vals[80..90] {
        *v = None;
    }
    vals[150] = None;
    let p = common::provider_with(bars, "S").with_values("g", "px", vec![("v", vals)], "v", &["v"]);
    let v = common::view(
        r#"{"id":"g","x":{"kind":"time"},"panels":[{"id":0,"y":{"kind":"linear"}}],
            "layers":[{"id":"ln","mark":"line","series":"g","panel":0}]}"#,
    );
    let s = compile(&v, &p, &opts()).unwrap();
    let l = layer(&s, "ln");
    assert_eq!(l.prims.len(), 3);
    assert_eq!(l.skipped, 11);
    assert_eq!(l.items.len(), 189);
    let lens: Vec<usize> = l
        .prims
        .iter()
        .map(|p| match p {
            Prim::Polyline { points, .. } => points.len(),
            _ => 0,
        })
        .collect();
    assert_eq!(lens, vec![80, 60, 49]);
}

#[test]
fn point_text_and_vertical_rule_from_facts() {
    let (p, bar) = with_facts();
    let v = common::view(
        r#"{"id":"f","x":{"kind":"time"},
          "panels":[{"id":0,"y":{"kind":"linear"}},{"id":1,"y":{"kind":"linear","domain":[0,100]}}],
          "layers":[{"id":"candles","mark":"candle","series":"px","panel":0},
            {"id":"rsi","mark":"line","series":"rsi14","panel":1},
            {"id":"lvl","mark":"rule","panel":1,"encoding":{"y":"70"},
             "origin":{"/encoding/y":{"kind":"fact","fact":"fact-rsi-cross70","field":"level"}}},
            {"id":"dot","mark":"point","fact":"fact-rsi-cross70","panel":1,"encoding":{"y":"value","size":8}},
            {"id":"txt","mark":"text","fact":"fact-rsi-cross70","panel":1},
            {"id":"vr","mark":"rule","finding":"fnd-overbought","panel":0},
            {"id":"area","mark":"area","series":"px","panel":0,"encoding":{"y":"c"}}]}"#,
    );
    let s = compile(&v, &p, &opts()).unwrap();
    let dot = layer(&s, "dot");
    assert_eq!(dot.items.len(), 1);
    assert_eq!(dot.items[0].bar, Some(bar));
    let Prim::Circle { r, cx, .. } = dot.prims[0] else {
        panic!()
    };
    assert_eq!(r, 4.0);
    assert_eq!(cx, slot_x(&s.x, bar) as f32 + 0.5);
    let txt = layer(&s, "txt");
    assert_eq!(txt.fields, vec!["level", "text"]);
    let Prim::Text { text, px, .. } = &txt.prims[0] else {
        panic!()
    };
    assert_eq!(text, "RSI(14) crossed 70");
    assert_eq!(*px, 11.0);
    // Text default y is the fact's level.
    assert_eq!(txt.items[0].values["level"], 70.0);
    let vr = layer(&s, "vr");
    assert_eq!(vr.items[0].bar, Some(bar));
    let Prim::Rect { x, h, .. } = vr.prims[0] else {
        panic!()
    };
    assert_eq!(x, slot_x(&s.x, bar) as i32);
    assert_eq!(h as i32, s.panels[0].rect.bottom - s.panels[0].rect.top);
    assert_eq!(
        layer(&s, "lvl").constants["/encoding/y"].origin,
        Origin::Fact {
            fact: "fact-rsi-cross70".into(),
            field: "level".into()
        }
    );
    // Area without y2 fills to the panel bottom, default opacity 0.25 and palette s1.
    let area = layer(&s, "area");
    let Prim::Fill { color, points } = &area.prims[0] else {
        panic!()
    };
    assert_eq!(*color, [255, 183, 77, 58]);
    assert_eq!(points.last().unwrap()[1], s.panels[0].rect.bottom as f32);
}

#[test]
fn multi_output_line_and_palette() {
    let p = common::provider();
    let v = common::view(
        r#"{"id":"m","x":{"kind":"time"},
          "panels":[{"id":0,"y":{"kind":"linear"}},{"id":2,"name":"macd","y":{"kind":"linear"}}],
          "layers":[{"id":"candles","mark":"candle","series":"px","panel":0},
            {"id":"bb","mark":"line","series":"bb","panel":0},
            {"id":"m","mark":"line","series":"macd","panel":2}]}"#,
    );
    let s = compile(&v, &p, &opts()).unwrap();
    let bb = layer(&s, "bb");
    assert_eq!(bb.fields, vec!["upper", "middle", "lower"]);
    let colors: Vec<[u8; 4]> = bb
        .prims
        .iter()
        .map(|p| match p {
            Prim::Polyline { color, .. } => *color,
            _ => panic!(),
        })
        .collect();
    assert_eq!(
        colors,
        vec![
            [255, 183, 77, 230],
            [0x42, 0xa5, 0xf5, 255],
            [0xab, 0x47, 0xbc, 255]
        ]
    );
    // One item per (slot, output), slot-major then output order, each with its own box
    // (GRAMMAR §16: a line point is ±4 px), so a pixel between two bands hits nothing.
    assert_eq!(bb.items.len() % 3, 0);
    for (i, key) in ["upper", "middle", "lower"].iter().enumerate() {
        let it = &bb.items[i];
        assert_eq!(it.bar, bb.items[0].bar);
        assert_eq!(it.values.keys().collect::<Vec<_>>(), vec![*key]);
    }
    let last = bb.items.len() - 3;
    let (up, mid) = (&bb.items[last].hit, &bb.items[last + 1].hit);
    assert!(up.bottom - up.top <= 10 && mid.bottom - mid.top <= 10);
    let gap_y = (up.bottom + mid.top) / 2;
    assert!(up.bottom < mid.top, "bands are apart at the last bar");
    let x = (up.left + up.right) / 2;
    assert!(!bb.items.iter().any(|it| {
        let h = &it.hit;
        h.left <= x && x < h.right && h.top <= gap_y && gap_y < h.bottom
    }));
    let m = layer(&s, "m");
    assert_eq!(m.fields, vec!["macd", "signal"]);
    // Palette restarts per panel.
    assert!(matches!(
        m.prims[0],
        Prim::Polyline {
            color: [255, 183, 77, 230],
            ..
        }
    ));
}

#[test]
fn emphasis_highlight_focus_dim() {
    let (p, bar) = with_facts();
    let mut v = common::view(V1);
    v.emphasis = vec![Emphasis {
        target: Target {
            item: Some("fnd-overbought".into()),
            ..Target::default()
        },
        kind: EmphasisKind::Highlight,
    }];
    let base = compile(&common::view(V1), &p, &opts()).unwrap();
    let s = compile(&v, &p, &opts()).unwrap();
    assert_eq!(s.emphasis.len(), 1);
    assert_eq!(s.emphasis[0].from_bar, Some(bar));
    assert_eq!(s.emphasis[0].to_bar, Some(bar));
    // One strip per panel (no panel given), painted first, accent with low alpha.
    assert_eq!(s.under.len(), base.under.len() + 3);
    for (i, pr) in s.under[..3].iter().enumerate() {
        let Prim::Rect { x, y, color, .. } = pr else {
            panic!()
        };
        assert_eq!(*y, s.panels[i].rect.top);
        assert_eq!(*x, (8.0 + bar as f64 * s.x.slot).floor() as i32);
        assert_eq!(color[..3], [255, 183, 77]);
        assert!(color[3] < 100);
    }
    // Transient highlight of a layer: its line is 1 px thicker; no strip.
    let mut o = opts();
    o.emphasis = vec![Emphasis {
        target: Target {
            layer: Some("rsi".into()),
            ..Target::default()
        },
        kind: EmphasisKind::Highlight,
    }];
    let s = compile(&common::view(V1), &p, &o).unwrap();
    assert!(matches!(layer(&s, "rsi").prims[0], Prim::Polyline { width, .. } if width == 2.5));
    assert_eq!(s.under.len(), base.under.len());
    // Focus on a layer: veils over all panels, then the layer again on top.
    o.emphasis[0].kind = EmphasisKind::Focus;
    let s = compile(&common::view(V1), &p, &o).unwrap();
    let extra = &s.over[base.over.len()..];
    assert_eq!(extra.len(), 3 + layer(&s, "rsi").prims.len());
    assert!(matches!(
        extra[0],
        Prim::Rect {
            color: [16, 18, 24, 160],
            ..
        }
    ));
    assert_eq!(extra[3], layer(&s, "rsi").prims[0]);
    // Dim a bar range in panel 0: one veil there.
    o.emphasis = vec![Emphasis {
        target: Target {
            panel: Some(0),
            from_bar: Some(180),
            to_bar: Some(199),
            ..Target::default()
        },
        kind: EmphasisKind::Dim,
    }];
    let s = compile(&common::view(V1), &p, &o).unwrap();
    let extra = &s.over[base.over.len()..];
    assert_eq!(extra.len(), 1);
    let Prim::Rect { x, w, y, .. } = extra[0] else {
        panic!()
    };
    assert_eq!(x, (8.0 + 180.0 * s.x.slot).floor() as i32);
    assert_eq!(x + w as i32, 1128);
    assert_eq!(y, 34);
    // Focus on a range: veils left of it in every panel (right edge is the plot end).
    o.emphasis[0].kind = EmphasisKind::Focus;
    o.emphasis[0].target.panel = None;
    let s = compile(&common::view(V1), &p, &o).unwrap();
    assert_eq!(s.over.len() - base.over.len(), 6);
}

#[test]
fn light_theme_and_override() {
    let p = common::provider();
    let mut v = common::view(V2);
    v.theme = Some("light".into());
    let s = compile(&v, &p, &opts()).unwrap();
    assert_eq!(s.background, [255, 255, 255, 255]);
    let mut o = opts();
    o.theme = Some("dark".into());
    let s = compile(&v, &p, &o).unwrap();
    assert_eq!(s.background, [16, 18, 24, 255]);
}

#[test]
fn persistent_range_and_viewport() {
    let p = common::provider();
    let bars = common::sample_bars();
    let mut v = common::view(V2);
    v.x.domain = Some([bars[50].t as f64, bars[99].t as f64]);
    let s = compile(&v, &p, &opts()).unwrap();
    assert_eq!((s.x.from_bar, s.x.to_bar), (50, 99));
    assert_eq!(s.x.slot, 1120.0 / 50.0);
    assert_eq!(layer(&s, "candles").items.len(), 50);
    // The transient viewport wins over the persistent range.
    let mut o = opts();
    o.range = Some((10, 30));
    let s = compile(&v, &p, &o).unwrap();
    assert_eq!((s.x.from_bar, s.x.to_bar), (10, 30));
    assert_eq!(layer(&s, "band").items.len(), 12);
    // Domain fits the visible bars only.
    let vis_hi = bars[10..=30].iter().map(|b| b.h).fold(f64::MIN, f64::max);
    assert!(s.panels[0].y.max < vis_hi + 5.0);
}

#[test]
fn compile_is_deterministic() {
    let (p, _) = with_facts();
    for json in [V1, V2, V5] {
        let v = common::view(json);
        let a = serde_json::to_string(&compile(&v, &p, &opts()).unwrap()).unwrap();
        let b = serde_json::to_string(&compile(&v, &p, &opts()).unwrap()).unwrap();
        assert_eq!(a, b);
    }
    let p = fan_provider();
    let v = common::view(V3);
    let a = serde_json::to_vec(&compile(&v, &p, &opts()).unwrap()).unwrap();
    let b = serde_json::to_vec(&compile(&v, &p, &opts()).unwrap()).unwrap();
    assert_eq!(a, b);
    // The scene validates against its own generated JSON Schema shape (round-trips).
    let s = compile(&v, &p, &opts()).unwrap();
    let back: SceneDesc = serde_json::from_slice(&a).unwrap();
    assert_eq!(back, s);
}

#[test]
fn deprecated_y_list_is_read_as_panels() {
    let p = common::provider();
    let v = common::view(
        r#"{"id":"old","x":{"kind":"time"},"y":[{"kind":"linear"},{"kind":"linear"}],
            "layers":[{"mark":"candle","series":"px","panel":0},
                      {"mark":"line","series":"rsi14","panel":1}]}"#,
    );
    let s = compile(&v, &p, &opts()).unwrap();
    assert_eq!(s.panels.len(), 2);
    let ids: Vec<&str> = s
        .panels
        .iter()
        .flat_map(|p| p.layers.iter())
        .map(|l| l.id.as_str())
        .collect();
    assert_eq!(ids, vec!["l0", "l1"]);
}

fn texts(s: &SceneDesc) -> Vec<(String, i32, i32)> {
    s.under
        .iter()
        .chain(&s.over)
        .filter_map(|p| match p {
            Prim::Text {
                text, x, baseline, ..
            } => Some((text.clone(), *x, *baseline)),
            _ => None,
        })
        .collect()
}

/// Title row: "C <close>" only when panel 0 draws the bar series; a long title is cut
/// with "…" to the frame; legends that would reach the title are dropped; identical
/// legends in a panel are shown once; a value label under a last-value tag is left out.
#[test]
fn title_row_close_legends_and_tag_labels() {
    let p = common::provider();
    // An RSI-only view with a long title at phone width: no close, title cut, no legend.
    let v = common::view(
        r#"{"id":"cap","title":"A very long capsule headline that cannot fit a phone frame at all",
          "x":{"kind":"time"},"panels":[{"id":0,"y":{"kind":"linear"}}],
          "layers":[{"id":"rsi","mark":"line","series":"rsi14","panel":0}]}"#,
    );
    let s = compile(&v, &p, &CompileOptions::new(390, 300)).unwrap();
    let t = texts(&s);
    assert!(!t.iter().any(|(x, _, _)| x.starts_with("C ")), "{t:?}");
    let title = &t.iter().find(|(_, _, b)| *b == 23).unwrap().0;
    assert!(
        title.ends_with('…') && title.starts_with("A very long"),
        "{title}"
    );
    assert!(title.chars().count() * 96 <= (390 - 8 - 12) * 10, "{title}");
    assert!(
        !t.iter().any(|(x, _, _)| x.starts_with("RSI14")),
        "legend dropped: {t:?}"
    );
    // Two layers on px in panel 0: close shown, one legend for the identical texts.
    let v = common::view(
        r#"{"id":"dup","x":{"kind":"time"},"panels":[{"id":0,"y":{"kind":"linear"}}],
          "layers":[{"id":"a","mark":"line","series":"px","panel":0,"encoding":{"y":"c"}},
                    {"id":"b","mark":"line","series":"px","panel":0,"encoding":{"y":"c"}}]}"#,
    );
    let s = compile(&v, &p, &opts()).unwrap();
    let t = texts(&s);
    assert!(t.iter().any(|(x, _, _)| x == "C 124.69"), "{t:?}");
    assert_eq!(
        t.iter().filter(|(x, _, _)| x == "px 124.69").count(),
        1,
        "{t:?}"
    );
    // RSI 55.10 tag in a 0..100 panel: the 50.00 label under it is left out.
    let s = compile(&common::view(V1), &p, &opts()).unwrap();
    let rsi = &s.panels[1];
    let labels: Vec<String> = texts(&s)
        .into_iter()
        .filter(|(_, x, b)| {
            *x == rsi.rect.right + 6 && *b > rsi.rect.top && *b < rsi.rect.bottom + 8
        })
        .map(|(t, _, _)| t)
        .collect();
    assert!(labels.contains(&"55.10".to_owned()), "{labels:?}");
    assert!(!labels.contains(&"50.00".to_owned()), "{labels:?}");
}
