//! Golden image test for the reference renderer.
//!
//! `fixtures/golden/sample.png` must equal, byte for byte, `render_scene_png` of the R0
//! view (candles + SMA(20) over `fixtures/bars/sample.json`, 1200x600) as compiled by
//! `qp-grammar` (study values from `qp-compute`, test-only; see `r0/mod.rs`). This is the
//! end-to-end check that the scene description reproduces the R0 chart exactly.
//!
//! Existing goldens are never regenerated (`CLAUDE.md`); `QP_UPDATE_GOLDEN=1` only writes
//! a golden that does not exist yet. On mismatch the actual image is written to
//! `target/golden-actual/` and the pixel-diff statistics are printed.

mod r0;

use std::path::PathBuf;

use qp_render::{
    parse_bars, render_scene_png, render_scene_rgba, scene, DEFAULT_HEIGHT, DEFAULT_WIDTH,
};
use qp_schema::scene::{Align, Prim};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn sample() -> qp_render::Series {
    let json = std::fs::read_to_string(root().join("fixtures/bars/sample.json")).unwrap();
    parse_bars(&json).unwrap()
}

fn decode_png(bytes: &[u8]) -> (u32, u32, Vec<u8>) {
    let dec = png::Decoder::new(std::io::Cursor::new(bytes));
    let mut reader = dec.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut buf).unwrap();
    assert_eq!(info.color_type, png::ColorType::Rgba);
    buf.truncate(info.buffer_size());
    (info.width, info.height, buf)
}

/// (differing pixels, max channel difference)
fn pixel_diff(a: &[u8], b: &[u8]) -> (usize, u8) {
    let mut n = 0;
    let mut max = 0;
    for (pa, pb) in a.chunks_exact(4).zip(b.chunks_exact(4)) {
        let d = pa
            .iter()
            .zip(pb)
            .map(|(x, y)| x.abs_diff(*y))
            .max()
            .unwrap_or(0);
        if d > 0 {
            n += 1;
            max = max.max(d);
        }
    }
    (n, max)
}

fn write_actual(name: &str, bytes: &[u8]) -> PathBuf {
    let dir = root().join("target/golden-actual");
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join(name);
    std::fs::write(&p, bytes).unwrap();
    p
}

#[test]
fn golden_sample_png() {
    let scene = r0::scene(&sample(), DEFAULT_WIDTH, DEFAULT_HEIGHT);
    let actual = render_scene_png(&scene).unwrap();
    check_golden("sample.png", &actual);
}

/// Spike-only stress scene (fractional AA strokes, translucency, many text sizes), 600x300,
/// a hand-written primitive list.
#[test]
fn golden_stress_png() {
    let actual = render_scene_png(&scene::stress_scene(600, 300)).unwrap();
    check_golden("stress.png", &actual);
    let rgba = qp_render::render_stress_rgba(600, 300).unwrap();
    assert_eq!(qp_render::encode_png(&rgba, 600, 300).unwrap(), actual);
}

fn check_golden(name: &str, actual: &[u8]) {
    let golden_path = root().join("fixtures/golden").join(name);
    if !golden_path.exists() && std::env::var_os("QP_UPDATE_GOLDEN").is_some_and(|v| v == "1") {
        std::fs::write(&golden_path, actual).unwrap();
        eprintln!("wrote new golden {}", golden_path.display());
        return;
    }

    let golden = std::fs::read(&golden_path).unwrap_or_else(|e| {
        panic!(
            "cannot read {} ({e}); a new golden can be written with QP_UPDATE_GOLDEN=1",
            golden_path.display()
        )
    });
    if actual != golden.as_slice() {
        let p = write_actual(name, actual);
        let (gw, gh, gpx) = decode_png(&golden);
        let (aw, ah, apx) = decode_png(actual);
        let stats = if (gw, gh) == (aw, ah) {
            let (n, max) = pixel_diff(&gpx, &apx);
            format!(
                "{n} differing pixels of {}, max channel diff {max}",
                gw * gh
            )
        } else {
            format!("size differs: golden {gw}x{gh}, actual {aw}x{ah}")
        };
        panic!(
            "golden mismatch: {stats}. Actual written to {}. \
             Existing goldens are not updated (CLAUDE.md).",
            p.display()
        );
    }
}

#[test]
fn renders_are_identical() {
    let bars = sample();
    let big = r0::scene(&bars, DEFAULT_WIDTH, DEFAULT_HEIGHT);
    assert_eq!(
        render_scene_png(&big).unwrap(),
        render_scene_png(&big).unwrap()
    );
    let small = r0::scene(&bars, 640, 360);
    let ra = render_scene_rgba(&small).unwrap();
    let rb = render_scene_rgba(&small).unwrap();
    assert_eq!(ra, rb);
    assert_eq!(ra.len(), 640 * 360 * 4);
}

#[test]
fn png_round_trips_to_rgba() {
    let s = r0::scene(&sample(), DEFAULT_WIDTH, DEFAULT_HEIGHT);
    let rgba = render_scene_rgba(&s).unwrap();
    let png = render_scene_png(&s).unwrap();
    let (w, h, px) = decode_png(&png);
    assert_eq!((w, h), (DEFAULT_WIDTH, DEFAULT_HEIGHT));
    assert_eq!(px, rgba);
}

/// The renderer reads the primitives and nothing else: the same primitives moved into a
/// bare scene (no panels, layers or items) give the same pixels.
#[test]
fn only_primitives_are_drawn() {
    let s = r0::scene(&sample(), DEFAULT_WIDTH, DEFAULT_HEIGHT);
    let bare = scene::scene_of(
        s.width,
        s.height,
        s.background,
        s.prims().cloned().collect(),
    );
    assert_eq!(
        render_scene_rgba(&s).unwrap(),
        render_scene_rgba(&bare).unwrap()
    );
}

fn px_at(rgba: &[u8], w: u32, x: u32, y: u32) -> [u8; 4] {
    let i = ((y * w + x) * 4) as usize;
    [rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]]
}

#[test]
fn fill_and_circle_are_drawn() {
    let red = [255, 0, 0, 255];
    let blue = [0, 0, 255, 255];
    let bg = [0, 0, 0, 255];
    let prims = vec![
        Prim::Fill {
            points: vec![[10.0, 10.0], [40.0, 10.0], [40.0, 40.0], [10.0, 40.0]],
            color: red,
        },
        Prim::Circle {
            cx: 80.0,
            cy: 25.0,
            r: 10.0,
            color: blue,
        },
        // Degenerate and non-finite primitives draw nothing (and do not fail).
        Prim::Fill {
            points: vec![[1.0, 1.0]],
            color: red,
        },
        Prim::Circle {
            cx: f32::NAN,
            cy: 5.0,
            r: 3.0,
            color: red,
        },
        Prim::Circle {
            cx: 5.0,
            cy: 5.0,
            r: -3.0,
            color: red,
        },
        Prim::Polyline {
            points: vec![[0.0, 0.0], [5.0, 5.0]],
            width: f32::INFINITY,
            color: red,
        },
    ];
    let (w, h) = (120, 60);
    let rgba = render_scene_rgba(&scene::scene_of(w, h, bg, prims)).unwrap();
    assert_eq!(px_at(&rgba, w, 25, 25), red);
    assert_eq!(px_at(&rgba, w, 80, 25), blue);
    assert_eq!(px_at(&rgba, w, 5, 5), bg);
    assert_eq!(px_at(&rgba, w, 60, 25), bg);
    assert_eq!(px_at(&rgba, w, 80, 45), bg);
    // Anti-aliased rim: partially covered pixels along the circle's row.
    let rim = (0..w)
        .map(|x| px_at(&rgba, w, x, 25)[2])
        .filter(|&b| b > 0 && b < 255)
        .count();
    assert!(rim > 0, "circle edge is anti-aliased");
}

/// Non-zero winding: a pentagram's centre is filled (even-odd would leave it empty).
#[test]
fn fill_uses_non_zero_winding() {
    let star = vec![
        [50.0, 5.0],
        [79.0, 95.0],
        [2.0, 40.0],
        [98.0, 40.0],
        [21.0, 95.0],
    ];
    let rgba = render_scene_rgba(&scene::scene_of(
        100,
        100,
        [0, 0, 0, 255],
        vec![Prim::Fill {
            points: star,
            color: [255, 255, 255, 255],
        }],
    ))
    .unwrap();
    assert_eq!(px_at(&rgba, 100, 50, 55), [255, 255, 255, 255]);
}

#[test]
fn rejects_bad_sizes_and_text() {
    let ok = scene::scene_of(10, 10, [0, 0, 0, 255], Vec::new());
    assert!(render_scene_rgba(&ok).is_ok());
    for (w, h) in [(0, 10), (10, 0), (16_385, 10)] {
        assert!(render_scene_rgba(&scene::scene_of(w, h, [0; 4], Vec::new())).is_err());
    }
    for px in [f32::NAN, 0.0, -1.0, 1e6] {
        let t = Prim::Text {
            x: 0,
            baseline: 5,
            px,
            text: "x".to_owned(),
            color: [255; 4],
            align: Align::Left,
        };
        assert!(render_scene_rgba(&scene::scene_of(10, 10, [0; 4], vec![t])).is_err());
    }
}

#[test]
fn sample_fixture_is_well_formed() {
    let bars = sample();
    assert_eq!(bars.symbol(), Some("SAMPLE"));
    assert_eq!(bars.bars().unwrap().len(), 200);
    // Matches the deterministic generator (examples/gen_sample.rs).
    assert_eq!(bars, qp_render::synthetic_bars(200, 20_251_114));
}
