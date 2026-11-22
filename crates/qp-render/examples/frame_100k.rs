//! 100,000-bar frame time (R2 criterion 10): the R0 view (candles + SMA(20)) over 100k
//! deterministic synthetic bars, compiled by `qp-grammar` and rasterized by the reference
//! renderer at 1200x600, with level of detail on and off. Study values are computed once,
//! outside the timings (the authority supplies them; the frame does not compute).
//!
//! ```sh
//! cargo run --release -p qp-render --example frame_100k [-- RUNS]   # default 31
//! ```
//! Prints the median (and min/max) over RUNS runs of compile, rasterize (straight RGBA)
//! and total (compile + rasterize in one timed span), and the number of pixels that
//! differ between the LOD and the full frame (decoded RGBA compare).

#[path = "../tests/r0/mod.rs"]
mod r0;

use std::time::Instant;

use qp_grammar::{compile, CompileOptions};

const N: usize = 100_000;
const W: u32 = 1200;
const H: u32 = 600;

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1e3
}

fn stats(mut v: Vec<f64>) -> (f64, f64, f64) {
    v.sort_by(f64::total_cmp);
    (v[v.len() / 2], v[0], v[v.len() - 1])
}

fn main() {
    let runs: usize = std::env::args()
        .nth(1)
        .and_then(|a| a.parse().ok())
        .unwrap_or(31)
        .max(1);
    let series = qp_source::synthetic_series(N, 7);
    let provider = r0::provider(&series);
    let view = r0::view();

    let mut frames = Vec::new();
    println!(
        "100k bars, {W}x{H}, {runs} runs, arch {}",
        std::env::consts::ARCH
    );
    println!(
        "| mode | compile ms (min-max) | rasterize ms (min-max) | total ms (min-max) | prims | lod |"
    );
    println!("|---|---|---|---|---|---|");
    for (name, lod) in [("LOD", true), ("full", false)] {
        let mut o = CompileOptions::new(W, H);
        o.lod = lod;
        // Warm-up (font parse, glyph cache, allocator).
        let scene = compile(&view, &provider, &o).expect("compile");
        let rgba = qp_render::render_scene_rgba(&scene).expect("render");
        let (mut c, mut r, mut t) = (Vec::new(), Vec::new(), Vec::new());
        for _ in 0..runs {
            let t0 = Instant::now();
            let s = compile(&view, &provider, &o).expect("compile");
            let c_ms = ms(t0);
            let t1 = Instant::now();
            let px = qp_render::render_scene_rgba(&s).expect("render");
            let r_ms = ms(t1);
            let total = ms(t0);
            assert_eq!(px, rgba, "frame not deterministic");
            c.push(c_ms);
            r.push(r_ms);
            t.push(total);
        }
        let (cm, cmin, cmax) = stats(c);
        let (rm, rmin, rmax) = stats(r);
        let (tm, tmin, tmax) = stats(t);
        println!(
            "| {name} | {cm:.3} ({cmin:.3}-{cmax:.3}) | {rm:.3} ({rmin:.3}-{rmax:.3}) | \
             {tm:.3} ({tmin:.3}-{tmax:.3}) | {} | {:?} |",
            scene.prims().count(),
            scene.lod
        );
        frames.push(rgba);
    }
    let differing = frames[0]
        .chunks_exact(4)
        .zip(frames[1].chunks_exact(4))
        .filter(|(a, b)| a != b)
        .count();
    println!(
        "LOD vs full: {differing} of {} pixels differ ({:.3}%)",
        W * H,
        100.0 * differing as f64 / f64::from(W * H)
    );
}
