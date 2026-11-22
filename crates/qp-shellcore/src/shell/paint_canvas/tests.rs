use super::*;

use super::atlas::ATLAS_W;

/// The command codes of a stream, arguments skipped.
fn codes(ops: &[f32]) -> Vec<f32> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < ops.len() {
        out.push(ops[i]);
        i += 1 + arity(ops[i]).expect("a command");
    }
    assert_eq!(i, ops.len());
    out
}

fn count(ops: &[f32], code: f32) -> usize {
    codes(ops)
        .iter()
        .filter(|&&v| (v - code).abs() < f32::EPSILON)
        .count()
}

fn text(x: i32, t: &str, align: Align) -> Prim {
    Prim::Text {
        x,
        baseline: 30,
        px: 33.0,
        text: t.to_owned(),
        color: [255, 255, 255, 255],
        align,
        font: None,
        role: None,
    }
}

fn line(y: f32) -> Prim {
    Prim::Polyline {
        points: vec![[0.0, y], [100.0, y]],
        width: 3.0,
        color: [1, 2, 3, 255],
    }
}

#[test]
fn styles_are_emitted_once_and_paths_batched() {
    let s = stream(
        Config::default(),
        &mut Atlas::default(),
        None,
        &[line(10.0), line(20.0)],
        (200, 100),
        [0; 4],
    )
    .unwrap();
    let strokes = count(&s.ops, op::STROKE);
    assert_eq!(strokes, 1, "{:?}", s.ops);
    let nb = Config {
        batch: false,
        ..Config::default()
    };
    let s = stream(
        nb,
        &mut Atlas::default(),
        None,
        &[line(10.0), line(20.0)],
        (200, 100),
        [0; 4],
    )
    .unwrap();
    assert_eq!(count(&s.ops, op::STROKE), 2);
    // the stroke style appears once
    assert_eq!(count(&s.ops, op::STROKE_STYLE), 1);
}

#[test]
fn glyphs_sit_at_the_reference_pens() {
    let font = qp_render::cpu::font().unwrap();
    let adv = font.metrics('0', 33.0).advance_width;
    let g = glyph_pens(
        qp_render::cpu::font().unwrap(),
        10,
        33.0,
        "0 0",
        Align::Left,
    );
    assert_eq!(g.len(), 2);
    assert!((g[0].1 - 10.0).abs() < f32::EPSILON);
    assert!((g[1].1 - (10.0 + 2.0 * adv).round()).abs() < f32::EPSILON);
    let r = glyph_pens(
        qp_render::cpu::font().unwrap(),
        100,
        33.0,
        "00",
        Align::Right,
    );
    assert!((r[0].1 - (100.0 - 2.0 * adv).round()).abs() < f32::EPSILON);
}

#[test]
fn a_small_change_repaints_a_clipped_region_only() {
    let a = vec![line(10.0), text(10, "123", Align::Left), line(90.0)];
    let mut b = a.clone();
    b[1] = text(10, "124", Align::Left);
    let s = stream(
        Config::default(),
        &mut Atlas::default(),
        Some(&a),
        &b,
        (1000, 1000),
        [0; 4],
    )
    .unwrap();
    assert!(s.partial);
    assert!(
        (codes(&s.ops)[1] - op::CLIP).abs() < f32::EPSILON,
        "{:?}",
        s.ops
    );
    // the far line (y 90 ± 8) is outside the text's region (y ≈ -13 … 49) and is left out
    assert_eq!(count(&s.ops, op::STROKE), 1);
    // unchanged → nothing
    assert!(stream(
        Config::default(),
        &mut Atlas::default(),
        Some(&b),
        &b,
        (1000, 1000),
        [0; 4]
    )
    .is_none());
    // a change covering most of the canvas → full repaint
    let s = stream(
        Config::default(),
        &mut Atlas::default(),
        Some(&a),
        &b,
        (60, 60),
        [0; 4],
    )
    .unwrap();
    assert!(!s.partial);
}

#[test]
fn text_size_outside_the_reference_range_paints_nothing() {
    let bad = Prim::Text {
        x: 0,
        baseline: 0,
        px: 0.0,
        text: "x".into(),
        color: [0; 4],
        align: Align::Left,
        font: None,
        role: None,
    };
    assert!(stream(
        Config::default(),
        &mut Atlas::default(),
        None,
        &[bad],
        (10, 10),
        [0; 4]
    )
    .is_none());
}

#[test]
fn atlas_glyphs_are_the_reference_coverage_and_uploaded_first() {
    let cfg = Config {
        text: Text::Atlas,
        ..Config::default()
    };
    let mut atlas = Atlas::default();
    let t = text(10, "SYN 12", Align::Left);
    let s = stream(
        cfg,
        &mut atlas,
        None,
        std::slice::from_ref(&t),
        (300, 60),
        [0; 4],
    )
    .unwrap();
    let c = codes(&s.ops);
    assert_eq!(c[0], op::INIT);
    assert_eq!(c[1], op::ATLAS, "{c:?}");
    assert_eq!(count(&s.ops, op::GLYPH), 5); // the space has no pixels
    assert_eq!(count(&s.ops, op::TEXT), 0);
    let px = atlas.take_pending();
    assert_eq!(px.len(), (s.ops[6] * s.ops[9] * 4.0) as usize);
    // the 'S' glyph: fontdue's coverage times the colour's alpha
    let (m, cov) = qp_render::cpu::font().unwrap().rasterize('S', 33.0);
    let slot = atlas
        .slot(
            (0, qp_render::cpu::font().unwrap()),
            'S',
            33.0,
            [255, 255, 255, 255],
        )
        .unwrap();
    assert_eq!((slot.w as usize, slot.h as usize), (m.width, m.height));
    let k = cov.iter().position(|&v| v > 0).unwrap();
    let (r, col) = (k / m.width, k % m.width);
    let i = ((slot.sy as usize + r) * ATLAS_W as usize + slot.sx as usize + col) * 4;
    assert_eq!(atlas.px[i + 3], cov[k]);
    // the same glyphs again: nothing to upload
    let s = stream(cfg, &mut atlas, None, &[t], (300, 60), [0; 4]).unwrap();
    assert_eq!(count(&s.ops, op::ATLAS), 0);
}

#[test]
fn config_parses() {
    let c = Config::parse("run,full");
    assert!(c.text == Text::Run && !c.partial && c.batch);
    assert_eq!(Config::parse("glyphs").text, Text::Glyphs);
    assert_eq!(Config::default().text, Text::Atlas);
    assert_eq!(Config::parse(""), Config::default());
}
