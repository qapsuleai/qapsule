//! Golden image test for the reference renderer.
//!
//! `fixtures/golden/sample.png` must equal `render_png(fixtures/bars/sample.json, 1200, 600)`
//! byte for byte. To regenerate after an intentional visual change:
//!
//! ```sh
//! QP_UPDATE_GOLDEN=1 cargo test -p qp-render --test golden
//! ```
//! On mismatch the actual image is written to `target/golden-actual/sample.png` and the
//! pixel-diff statistics are printed.

use std::path::PathBuf;

use qp_render::{parse_bars, render_png, render_rgba, DEFAULT_HEIGHT, DEFAULT_WIDTH};

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
    let actual = render_png(&sample(), DEFAULT_WIDTH, DEFAULT_HEIGHT).unwrap();
    check_golden("sample.png", &actual);
}

/// Spike-only stress scene (fractional AA strokes, translucency, many text sizes), 600x300.
#[test]
fn golden_stress_png() {
    let rgba = qp_render::render_stress_rgba(600, 300).unwrap();
    let actual = qp_render::encode_png(&rgba, 600, 300).unwrap();
    check_golden("stress.png", &actual);
}

fn check_golden(name: &str, actual: &[u8]) {
    let golden_path = root().join("fixtures/golden").join(name);
    if std::env::var_os("QP_UPDATE_GOLDEN").is_some_and(|v| v == "1") {
        std::fs::write(&golden_path, actual).unwrap();
        eprintln!("updated {}", golden_path.display());
        return;
    }

    let golden = std::fs::read(&golden_path).unwrap_or_else(|e| {
        panic!(
            "cannot read {} ({e}); run with QP_UPDATE_GOLDEN=1",
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
             If the change is intentional: QP_UPDATE_GOLDEN=1 cargo test -p qp-render --test golden",
            p.display()
        );
    }
}

#[test]
fn renders_are_identical() {
    let bars = sample();
    let a = render_png(&bars, DEFAULT_WIDTH, DEFAULT_HEIGHT).unwrap();
    let b = render_png(&bars, DEFAULT_WIDTH, DEFAULT_HEIGHT).unwrap();
    assert_eq!(a, b);
    let ra = render_rgba(&bars, 640, 360).unwrap();
    let rb = render_rgba(&bars, 640, 360).unwrap();
    assert_eq!(ra, rb);
    assert_eq!(ra.len(), 640 * 360 * 4);
}

#[test]
fn png_round_trips_to_rgba() {
    let bars = sample();
    let rgba = render_rgba(&bars, DEFAULT_WIDTH, DEFAULT_HEIGHT).unwrap();
    let png = render_png(&bars, DEFAULT_WIDTH, DEFAULT_HEIGHT).unwrap();
    let (w, h, px) = decode_png(&png);
    assert_eq!((w, h), (DEFAULT_WIDTH, DEFAULT_HEIGHT));
    assert_eq!(px, rgba);
}

#[test]
fn sample_fixture_is_well_formed() {
    let bars = sample();
    assert_eq!(bars.symbol(), Some("SAMPLE"));
    assert_eq!(bars.bars().unwrap().len(), 200);
    // Matches the deterministic generator (examples/gen_sample.rs).
    assert_eq!(bars, qp_render::synthetic_bars(200, 20_251_114));
}
