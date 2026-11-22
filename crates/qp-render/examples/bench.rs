//! Reference-stack (tiny-skia + fontdue) measurements. Reproduce with:
//!
//! ```sh
//! cargo run --release -p qp-render --example bench
//! cargo run --release -p qp-render --example bench --features simd   # SIMD pipelines
//! cargo run --release -p qp-render --example bench --target x86_64-apple-darwin  # Rosetta
//! ```
//! Prints one JSON object: timings (ms), FNV-1a hashes of the RGBA output (compare across
//! machines/targets), and the pixel diff of the sample render against the golden PNG.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::cognitive_complexity
)]

use std::path::PathBuf;
use std::time::Instant;

#[path = "../tests/r0/mod.rs"]
mod r0;

use qp_render::{cpu, DEFAULT_HEIGHT, DEFAULT_WIDTH};

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
}

fn decode_png(bytes: &[u8]) -> Vec<u8> {
    let mut r = png::Decoder::new(std::io::Cursor::new(bytes))
        .read_info()
        .unwrap();
    let mut buf = vec![0; r.output_buffer_size().unwrap()];
    let info = r.next_frame(&mut buf).unwrap();
    buf.truncate(info.buffer_size());
    buf
}

fn diff(a: &[u8], b: &[u8]) -> (usize, u8) {
    let mut n = 0;
    let mut m = 0;
    for (x, y) in a.chunks_exact(4).zip(b.chunks_exact(4)) {
        let d = x
            .iter()
            .zip(y)
            .map(|(p, q)| p.abs_diff(*q))
            .max()
            .unwrap_or(0);
        if d > 0 {
            n += 1;
            m = m.max(d);
        }
    }
    (n, m)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let (w, h) = (DEFAULT_WIDTH, DEFAULT_HEIGHT);

    // First frame: cold font parse + parse JSON + render + PNG encode.
    let t = Instant::now();
    let json = std::fs::read_to_string(root.join("fixtures/bars/sample.json"))?;
    let bars = qp_render::parse_bars(&json)?;
    let png = qp_render::render_scene_png(&r0::scene(&bars, w, h))?;
    let first_frame = ms(t);

    let golden = std::fs::read(root.join("fixtures/golden/sample.png"))?;
    let rgba = qp_render::render_scene_rgba(&r0::scene(&bars, w, h))?;
    let (gd_px, gd_max) = diff(&rgba, &decode_png(&golden));

    let sample_rgba: Vec<f64> = (0..30)
        .map(|_| {
            let t = Instant::now();
            let _ = qp_render::render_scene_rgba(&r0::scene(&bars, w, h));
            ms(t)
        })
        .collect();
    let png_enc: Vec<f64> = (0..10)
        .map(|_| {
            let t = Instant::now();
            let _ = qp_render::encode_png(&rgba, w, h);
            ms(t)
        })
        .collect();

    let big = qp_source::synthetic_series(100_000, 7);
    let mut scene_ms = Vec::new();
    let mut raster_ms = Vec::new();
    let mut big_hash = 0;
    for _ in 0..5 {
        let t = Instant::now();
        let sc = r0::scene(&big, w, h);
        scene_ms.push(ms(t));
        let t = Instant::now();
        let pm = cpu::rasterize(&sc)?;
        raster_ms.push(ms(t));
        let hsh = fnv1a(pm.data());
        assert!(
            big_hash == 0 || big_hash == hsh,
            "100k render not deterministic"
        );
        big_hash = hsh;
    }

    let arch = std::env::consts::ARCH;
    let simd = cfg!(feature = "simd");
    println!(
        "{{\"stack\":\"tiny-skia+fontdue\",\"arch\":\"{arch}\",\"simd\":{simd},\
         \"first_frame_ms\":{first_frame:.2},\"sample_render_rgba_median_ms\":{:.2},\
         \"png_encode_median_ms\":{:.2},\"bars100k_scene_median_ms\":{:.2},\
         \"bars100k_raster_median_ms\":{:.2},\"sample_png_bytes\":{},\
         \"sample_png_equals_golden\":{},\"sample_rgba_vs_golden_diff_pixels\":{gd_px},\
         \"sample_rgba_vs_golden_max_channel\":{gd_max},\"sample_rgba_fnv\":\"{:016x}\",\
         \"bars100k_rgba_fnv\":\"{big_hash:016x}\"}}",
        median(sample_rgba),
        median(png_enc),
        median(scene_ms),
        median(raster_ms),
        png.len(),
        png == golden,
        fnv1a(&rgba),
    );
    Ok(())
}
