//! Spike: draw the same `Scene` with vello (GPU compute, wgpu) headless and measure.
//!
//! ```sh
//! cargo run --release -p qp-render --example spike_vello_gpu --features spike-vello-gpu
//! ```
//! Needs a GPU (Metal/Vulkan/DX12). Prints one JSON line per antialiasing mode; writes PNGs
//! to `target/spikes/`. Timings include GPU->CPU readback (the headless use case).

#[path = "common/mod.rs"]
mod common;

use std::time::Instant;

use common::{diff, fnv1a, golden_rgba, median, ms, write_png};
#[path = "../tests/r0/mod.rs"]
mod r0;

use qp_render::scene::stress_scene;
use qp_render::spikes::vello_gpu::GpuRenderer;
use vello::AaConfig;

fn main() {
    let t_init = Instant::now();
    let mut gpu = match GpuRenderer::new() {
        Ok(g) => g,
        Err(e) => {
            println!("{{\"stack\":\"vello 0.12.0\",\"error\":\"{e}\"}}");
            return;
        }
    };
    let init_ms = ms(t_init);
    let json = std::fs::read_to_string(common::root().join("fixtures/bars/sample.json")).unwrap();
    let bars = qp_render::parse_bars(&json).unwrap();
    let (w, h) = (qp_render::DEFAULT_WIDTH, qp_render::DEFAULT_HEIGHT);
    let sample = r0::scene(&bars, w, h);
    let t = Instant::now();
    let _ = gpu.render(&sample, AaConfig::Area).unwrap();
    let first_frame_ms = ms(t);
    println!(
        "adapter = {}; init_ms (device + pipeline compile) = {init_ms:.1}; \
         first_frame_ms (after init, Area) = {first_frame_ms:.2}",
        gpu.adapter_info
    );

    let golden = golden_rgba("sample.png");
    let stress_golden = golden_rgba("stress.png");
    let stress = stress_scene(600, 300);
    // 100k bars without LOD, like the R0 measurement.
    let big = r0::scene_full(&qp_render::synthetic_bars(100_000, 7), w, h);
    for (name, aa) in [
        ("area", AaConfig::Area),
        ("msaa8", AaConfig::Msaa8),
        ("msaa16", AaConfig::Msaa16),
    ] {
        let a = gpu.render(&sample, aa).unwrap();
        // Run-to-run determinism: 30 renders, count distinct outputs and the worst diff.
        let mut uniq = std::collections::BTreeSet::new();
        let mut worst = (0usize, 0u8);
        uniq.insert(fnv1a(&a));
        for _ in 0..30 {
            let r = gpu.render(&sample, aa).unwrap();
            if uniq.insert(fnv1a(&r)) {
                let (n, m, _) = diff(&r, &a);
                worst = worst.max((n, m));
            }
        }
        let identical = uniq.len() == 1;
        println!(
            "  [{name}] sample x31: {} distinct outputs, worst diff vs first: {} px, max channel {}",
            uniq.len(),
            worst.0,
            worst.1
        );
        let times: Vec<f64> = (0..15)
            .map(|_| {
                let t = Instant::now();
                let _ = gpu.render(&sample, aa);
                ms(t)
            })
            .collect();
        let big_times: Vec<f64> = (0..5)
            .map(|_| {
                let t = Instant::now();
                let _ = gpu.render(&big, aa);
                ms(t)
            })
            .collect();
        let big_a = fnv1a(&gpu.render(&big, aa).unwrap());
        let big_b = fnv1a(&gpu.render(&big, aa).unwrap());
        let s = gpu.render(&stress, aa).unwrap();
        let s2 = gpu.render(&stress, aa).unwrap();
        let (dn, dm, mean) = diff(&a, &golden);
        let (sn, sm, smean) = diff(&s, &stress_golden);
        println!(
            "{{\"stack\":\"vello 0.12.0 (wgpu 30)\",\"aa\":\"{name}\",\
             \"sample_median_ms\":{:.2},\"bars100k_median_ms\":{:.2},\
             \"repeat_identical_x10\":{identical},\"bars100k_repeat_identical\":{},\
             \"stress_repeat_identical\":{},\
             \"sample_fnv\":\"{:016x}\",\"stress_fnv\":\"{:016x}\",\"bars100k_fnv\":\"{big_a:016x}\",\
             \"vs_reference_sample\":{{\"px\":{dn},\"max\":{dm},\"mean\":{mean:.4}}},\
             \"vs_reference_stress\":{{\"px\":{sn},\"max\":{sm},\"mean\":{smean:.4}}}}}",
            median(times),
            median(big_times),
            big_a == big_b,
            s == s2,
            fnv1a(&a),
            fnv1a(&s),
        );
        write_png(&format!("vello_gpu_{name}_sample.png"), &a, w, h);
        write_png(&format!("vello_gpu_{name}_stress.png"), &s, 600, 300);
    }
}
