//! Spike: draw the same `Scene` with vello_cpu (sparse-strip CPU renderer) and measure.
//!
//! ```sh
//! cargo run --release -p qp-render --example spike_vello_cpu --features spike-vello-cpu-mt
//! cargo run --release -p qp-render --example spike_vello_cpu --features spike-vello-cpu-mt \
//!     --target x86_64-apple-darwin          # cross-arch determinism (Rosetta)
//! ```
//! Prints one JSON line per configuration; writes PNGs to `target/spikes/`.

#[path = "common/mod.rs"]
mod common;

use std::time::Instant;

use common::{diff, fnv1a, golden_rgba, median, ms, write_png};
#[path = "../tests/r0/mod.rs"]
mod r0;

use qp_render::scene::stress_scene;
use qp_render::spikes::vello_cpu::{rasterize, Config};

fn main() {
    let t_first = Instant::now();
    let json = std::fs::read_to_string(common::root().join("fixtures/bars/sample.json")).unwrap();
    let bars = qp_render::parse_bars(&json).unwrap();
    let (w, h) = (qp_render::DEFAULT_WIDTH, qp_render::DEFAULT_HEIGHT);
    let sample = r0::scene(&bars, w, h);
    let base = Config::default();
    let _first = rasterize(&sample, base).unwrap();
    let first_frame_ms = ms(t_first);

    let golden = golden_rgba("sample.png");
    let stress_golden = golden_rgba("stress.png");
    let stress = stress_scene(600, 300);
    // 100k bars without LOD, like the R0 measurement.
    let big = r0::scene_full(&qp_render::synthetic_bars(100_000, 7), w, h);
    let mt = if cfg!(feature = "spike-vello-cpu-mt") {
        std::thread::available_parallelism().map_or(1, |n| n.get().min(8) as u16)
    } else {
        0
    };

    println!("first_frame_ms (font + parse + scene + raster, speed/1T) = {first_frame_ms:.2}");
    let mut cfgs = vec![
        ("speed-u8/1T", base),
        (
            "quality-f32/1T",
            Config {
                quality: true,
                ..base
            },
        ),
        (
            "speed-u8/baseline-simd/1T",
            Config {
                baseline_simd: true,
                ..base
            },
        ),
    ];
    if mt > 0 {
        cfgs.push((
            "speed-u8/MT",
            Config {
                threads: mt,
                ..base
            },
        ));
        cfgs.push((
            "quality-f32/MT",
            Config {
                quality: true,
                threads: mt,
                ..base
            },
        ));
    }
    for (name, cfg) in cfgs {
        let a = rasterize(&sample, cfg).unwrap();
        let b = rasterize(&sample, cfg).unwrap();
        let times: Vec<f64> = (0..15)
            .map(|_| {
                let t = Instant::now();
                let _ = rasterize(&sample, cfg);
                ms(t)
            })
            .collect();
        let big_times: Vec<f64> = (0..3)
            .map(|_| {
                let t = Instant::now();
                let _ = rasterize(&big, cfg);
                ms(t)
            })
            .collect();
        let big_a = fnv1a(&rasterize(&big, cfg).unwrap());
        let big_b = fnv1a(&rasterize(&big, cfg).unwrap());
        let s = rasterize(&stress, cfg).unwrap();
        let (dn, dm, mean) = diff(&a, &golden);
        let (sn, sm, smean) = diff(&s, &stress_golden);
        println!(
            "{{\"stack\":\"vello_cpu 0.4.0\",\"cfg\":\"{name}\",\"arch\":\"{}\",\
             \"sample_median_ms\":{:.2},\"bars100k_median_ms\":{:.2},\
             \"repeat_identical\":{},\"bars100k_repeat_identical\":{},\
             \"sample_fnv\":\"{:016x}\",\"stress_fnv\":\"{:016x}\",\"bars100k_fnv\":\"{big_a:016x}\",\
             \"vs_reference_sample\":{{\"px\":{dn},\"max\":{dm},\"mean\":{mean:.4}}},\
             \"vs_reference_stress\":{{\"px\":{sn},\"max\":{sm},\"mean\":{smean:.4}}}}}",
            std::env::consts::ARCH,
            median(times),
            median(big_times),
            a == b,
            big_a == big_b,
            fnv1a(&a),
            fnv1a(&s),
        );
        let tag = format!("{}_{}", std::env::consts::ARCH, name.replace('/', "_"));
        write_png(&format!("vello_cpu_{tag}_sample.png"), &a, w, h);
        write_png(&format!("vello_cpu_{tag}_stress.png"), &s, 600, 300);
    }
}
