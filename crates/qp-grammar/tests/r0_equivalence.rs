//! Criterion 3: the R0 scene (candles + SMA(20) over the sample, 1200x600) compiled from a
//! grammar view equals, primitive for primitive and value for value, what
//! `qp_render::scene::build_scene` produces.

mod common;

use qp_grammar::{compile, CompileOptions};
use qp_render::scene as r0;
use qp_schema::scene::{Align, Prim};

/// A comparable form of both primitive types.
#[derive(Debug, PartialEq)]
enum P {
    Rect(i32, i32, u32, u32, [u8; 4]),
    Poly(Vec<(u32, u32)>, u32, [u8; 4]),
    Text(i32, i32, u32, String, [u8; 4], bool),
    Other(String),
}

fn ours(p: &Prim) -> P {
    match p {
        Prim::Rect { x, y, w, h, color } => P::Rect(*x, *y, *w, *h, *color),
        Prim::Polyline {
            points,
            width,
            color,
        } => P::Poly(
            points
                .iter()
                .map(|q| (q[0].to_bits(), q[1].to_bits()))
                .collect(),
            width.to_bits(),
            *color,
        ),
        Prim::Text {
            x,
            baseline,
            px,
            text,
            color,
            align,
        } => P::Text(
            *x,
            *baseline,
            px.to_bits(),
            text.clone(),
            *color,
            *align == Align::Left,
        ),
        other => P::Other(format!("{other:?}")),
    }
}

fn oracle(p: &r0::Prim) -> P {
    match p {
        r0::Prim::Rect { x, y, w, h, color } => P::Rect(*x, *y, *w, *h, *color),
        r0::Prim::Polyline {
            points,
            width,
            color,
        } => P::Poly(
            points
                .iter()
                .map(|q| (q.0.to_bits(), q.1.to_bits()))
                .collect(),
            width.to_bits(),
            *color,
        ),
        r0::Prim::Text {
            x,
            baseline,
            px,
            text,
            color,
            align,
        } => P::Text(
            *x,
            *baseline,
            px.to_bits(),
            text.clone(),
            *color,
            *align == r0::Align::Left,
        ),
    }
}

/// The V0 view as built by protocol messages (`rounds/R2/tasks/VIEWS.md` V0).
const R0_VIEW: &str = r#"{"id":"r0","x":{"kind":"time"},
  "panels":[{"id":0,"y":{"kind":"linear"}}],
  "layers":[{"id":"candles","mark":"candle","series":"px","panel":0},
            {"id":"sma","mark":"line","series":"sma20","panel":0}]}"#;

#[test]
fn r0_scene_is_reproduced_exactly() {
    let series = common::sample_series();
    let expected = r0::build_scene(&series, 1200, 600).expect("oracle");
    let p = common::provider();
    let scene =
        compile(&common::view(R0_VIEW), &p, &CompileOptions::new(1200, 600)).expect("compile");
    assert_eq!(scene.width, expected.width);
    assert_eq!(scene.height, expected.height);
    assert_eq!(scene.background, expected.background);
    let a: Vec<P> = scene.prims().map(ours).collect();
    let b: Vec<P> = expected.prims.iter().map(oracle).collect();
    for (i, (x, y)) in a.iter().zip(&b).enumerate() {
        assert_eq!(x, y, "primitive {i} differs");
    }
    assert_eq!(a.len(), b.len(), "primitive count");
    assert_eq!(scene.lod, None);
    assert_eq!(scene.bars, 200);
    // The line layer skips the 19 warmup bars.
    assert_eq!(scene.panels[0].layers[1].skipped, 19);
}
