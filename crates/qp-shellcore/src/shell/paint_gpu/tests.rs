use super::*;

fn frame() -> Frame {
    Frame::new(100, 50, [0, 0, 0, 255])
}

fn xy(f: &Frame, k: usize) -> [f32; 2] {
    let b = &f.verts[k * VERTEX_BYTES..];
    let r = |o: usize| f32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]);
    [r(0), r(4)]
}

#[test]
fn clip_space_maps_the_corners() {
    let mut f = frame();
    rect(&mut f, 0, 0, 100, 50, [255; 4]);
    assert_eq!(f.n, 6);
    assert_eq!(f.verts.len(), 6 * VERTEX_BYTES);
    assert_eq!(xy(&f, 0), [-1.0, 1.0]);
    assert_eq!(xy(&f, 2), [1.0, -1.0]);
    assert_eq!(f.draws, [DRAW_PLAIN, 0, 6]);
}

#[test]
fn plain_runs_merge_and_stencil_breaks_them() {
    let mut f = frame();
    rect(&mut f, 0, 0, 10, 10, [255; 4]);
    rect(&mut f, 10, 0, 10, 10, [255; 4]);
    assert_eq!(f.draws, [DRAW_PLAIN, 0, 12]);
    // A concave (L-shaped) polygon needs the stencil.
    let l = [
        [0.0, 0.0],
        [20.0, 0.0],
        [20.0, 5.0],
        [5.0, 5.0],
        [5.0, 20.0],
        [0.0, 20.0],
    ];
    fill(&mut f, &l, [10, 10, 10, 128]);
    assert_eq!(&f.draws[3..6], &[DRAW_STENCIL_FILL, 12, 12]);
    assert_eq!(&f.draws[6..9], &[DRAW_COVER, 24, 6]);
    rect(&mut f, 0, 0, 1, 1, [255; 4]);
    assert_eq!(&f.draws[9..], &[DRAW_PLAIN, 30, 6]);
}

#[test]
fn convexity() {
    assert!(convex(&[
        [0.0, 0.0],
        [10.0, 0.0],
        [10.0, 10.0],
        [0.0, 10.0]
    ]));
    assert!(convex(&[[0.0, 0.0], [10.0, 0.0], [5.0, 8.0]]));
    let star = [
        [0.0, -10.0],
        [6.0, 8.0],
        [-9.5, -3.0],
        [9.5, -3.0],
        [-6.0, 8.0],
    ];
    assert!(!convex(&star));
    let l = [
        [0.0, 0.0],
        [20.0, 0.0],
        [20.0, 5.0],
        [5.0, 5.0],
        [5.0, 20.0],
        [0.0, 20.0],
    ];
    assert!(!convex(&l));
    assert!(!convex(&[[0.0, 0.0], [1.0, 1.0], [2.0, 2.0]]));
}

#[test]
fn translucent_strokes_are_covered_once() {
    let mut f = frame();
    let pts = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0]];
    stroke(&mut f, &pts, 3.0, [255, 0, 0, 255]);
    assert_eq!(f.draws[0], DRAW_PLAIN);
    let mut g = frame();
    stroke(&mut g, &pts, 3.0, [255, 0, 0, 100]);
    assert_eq!(g.draws[0], DRAW_STENCIL_UNION);
    assert_eq!(g.draws[3], DRAW_COVER);
    let mut h = frame();
    stroke(&mut h, &[[0.0, 0.0], [10.0, 0.0]], 3.0, [255, 0, 0, 100]);
    assert_eq!(h.draws, [DRAW_PLAIN, 0, 6]);
}

#[test]
fn miter_and_bevel_follow_the_limit() {
    // A right angle: miter (1/sin 45° ≈ 1.41 ≤ 4): two segments + two join triangles.
    let mut f = frame();
    stroke(
        &mut f,
        &[[0.0, 0.0], [10.0, 0.0], [10.0, 10.0]],
        2.0,
        [0, 0, 0, 255],
    );
    assert_eq!(f.n, 12 + 6);
    // A very sharp turn: bevel (one triangle).
    let mut g = frame();
    stroke(
        &mut g,
        &[[0.0, 0.0], [10.0, 0.0], [0.0, 0.5]],
        2.0,
        [0, 0, 0, 255],
    );
    assert_eq!(g.n, 12 + 3);
}

#[test]
fn thin_strokes_scale_the_alpha() {
    assert_eq!(premul([255, 255, 255, 255], 0.5), [128, 128, 128, 128]);
    assert_eq!(premul([200, 100, 0, 255], 1.0), [200, 100, 0, 255]);
    assert_eq!(premul([255, 0, 0, 128], 1.0), [128, 0, 0, 128]);
}

#[test]
fn text_matches_the_reference_placement() {
    let font = qp_render::cpu::font().unwrap();
    let mut atlas = Atlas::new(0);
    let mut f = frame();
    let t = (10, 30, 16.0, "Ab", [255; 4], Align::Left);
    assert!(text(&mut f, &mut atlas, (0, font), t).is_ok());
    let m = font.metrics('A', 16.0);
    let top = (30 - m.height as i32 - m.ymin) as f32;
    let left = (10 + m.xmin) as f32;
    let at = |p: [f32; 2]| [(p[0] + 1.0) / f.sx, (1.0 - p[1]) / f.sy];
    let p = at(xy(&f, 0));
    assert!(
        (p[0] - left).abs() < 1e-3 && (p[1] - top).abs() < 1e-3,
        "{p:?}"
    );
    assert_eq!(atlas.glyphs.len(), 2);
    // The same glyph again is not rasterised twice.
    assert!(text(&mut f, &mut atlas, (0, font), t).is_ok());
    assert_eq!(atlas.glyphs.len(), 2);
}

#[test]
fn a_full_atlas_starts_over() {
    let font = qp_render::cpu::font().unwrap();
    let mut atlas = Atlas::new(0);
    let mut full = false;
    for k in 0..400u32 {
        if atlas.glyph((0, font), 'W', 100.0 + k as f32).is_none() {
            full = true;
            break;
        }
    }
    assert!(full, "the atlas never filled");
    let mut g = GpuPainter {
        atlas,
        frame: Frame::default(),
    };
    let q = Prim::Text {
        x: 0,
        baseline: 8,
        px: 300.0,
        text: "Q".into(),
        color: [255; 4],
        align: Align::Left,
        font: None,
        role: None,
    };
    g.build_prims((10, 10, [0, 0, 0, 255]), &[&q], 1);
    assert_eq!(g.atlas_generation(), 1);
    assert_eq!(g.atlas_glyphs(), 1);
    assert_eq!(g.draws().len(), 3);
}
