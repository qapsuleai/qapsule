//! RP3/M12a: where the time of a software paint goes (native; `--ignored`, prints).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::cognitive_complexity
)]
#![cfg(feature = "replica")]

use std::time::Instant;

use qp_render::cpu::{rasterize_scaled, scale_prim};
use qp_schema::scene::Prim;
use qp_session::ReplayHost;
use qp_wasm::drive::parity_host;
use qp_wasm::shell::{scene_digest, ShellCore};
use serde_json::json;

fn pump(core: &mut ShellCore, host: &mut ReplayHost) {
    for _ in 0..500 {
        let out = core.take_outbox();
        if out.is_empty() {
            break;
        }
        for m in out {
            for l in host.handle_raw("w", &m.to_string()) {
                if l.conn == "w" {
                    let msg = serde_json::to_value(&l.msg).expect("msg");
                    core.on_frame(&json!({ "msg": msg }).to_string())
                        .expect("frame");
                }
            }
        }
    }
}

fn seek(core: &mut ShellCore, host: &mut ReplayHost, n: u64) {
    let before = host.recorded_len();
    core.seek(&n.to_string());
    pump(core, host);
    let evs: Vec<_> = host.recorded_since(before);
    for ev in &evs {
        core.data_event(ev);
    }
    pump(core, host);
}

fn kind(p: &Prim) -> &'static str {
    match p {
        Prim::Rect { .. } => "rect",
        Prim::Polyline { .. } => "line",
        Prim::Fill { .. } => "fill",
        Prim::Circle { .. } => "circle",
        Prim::Text { .. } => "text",
    }
}

#[test]
#[ignore = "profile; run with --ignored --nocapture"]
fn profile() {
    let mut host = parity_host();
    host.set_record_events(true);
    let data = host.init().data.clone().expect("data");
    let evs = host.recorded_events();
    let mut core = ShellCore::new("arkin", "tr", &data, &evs).expect("core");
    core.set_dpr(3.0);
    pump(&mut core, &mut host);
    seek(&mut core, &mut host, 120);
    let id = core.feed()[0].id.clone();
    let face = core.face(&id).expect("face");
    core.open_capsule(&id);
    pump(&mut core, &mut host);
    core.open_to("chart");
    pump(&mut core, &mut host);
    let sc = core.scene().expect("scene");
    for (name, s) in [("face", &face), ("scene", &sc)] {
        let prims: Vec<Prim> = s.prims().map(|p| scale_prim(p, 3)).collect();
        let mut counts = std::collections::BTreeMap::new();
        for p in &prims {
            *counts.entry(kind(p)).or_insert(0) += 1;
        }
        println!("{name}: {}x{} css, prims {:?}", s.width, s.height, counts);
        let n = 20;
        let t = Instant::now();
        for _ in 0..n {
            let _ = core.scene();
        }
        println!(
            "  compile scene() {:.3} ms",
            t.elapsed().as_secs_f64() * 1e3 / f64::from(n)
        );
        let t = Instant::now();
        for _ in 0..n {
            let _ = scene_digest(s);
        }
        println!(
            "  digest {:.3} ms",
            t.elapsed().as_secs_f64() * 1e3 / f64::from(n)
        );
        let t = Instant::now();
        for _ in 0..n {
            let _ = rasterize_scaled(s, 3).unwrap();
        }
        println!(
            "  rasterize {:.3} ms",
            t.elapsed().as_secs_f64() * 1e3 / f64::from(n)
        );
        let pm = rasterize_scaled(s, 3).unwrap();
        let t = Instant::now();
        for _ in 0..n {
            let _ = qp_render::cpu::demultiply(pm.data());
        }
        println!(
            "  demultiply {:.3} ms",
            t.elapsed().as_secs_f64() * 1e3 / f64::from(n)
        );
        // Per kind: rasterize only that kind.
        for k in ["rect", "line", "fill", "circle", "text"] {
            let only: Vec<Prim> = s.prims().filter(|p| kind(p) == k).cloned().collect();
            let mut s2 = s.clone();
            s2.under = only;
            s2.over.clear();
            for pnl in &mut s2.panels {
                for l in &mut pnl.layers {
                    l.prims.clear();
                }
            }
            let t = Instant::now();
            for _ in 0..n {
                let _ = rasterize_scaled(&s2, 3).unwrap();
            }
            println!(
                "  only {k}: {:.3} ms",
                t.elapsed().as_secs_f64() * 1e3 / f64::from(n)
            );
        }
    }
    // How prims change from step to step (chart open, following the last bar).
    let mut last: Vec<Prim> = sc.prims().cloned().collect();
    for step in 121..126 {
        seek(&mut core, &mut host, step);
        let s = core.scene().expect("scene");
        let now: Vec<Prim> = s.prims().cloned().collect();
        let first = now
            .iter()
            .zip(&last)
            .position(|(a, b)| a != b)
            .unwrap_or(now.len().min(last.len()));
        let same = now.iter().filter(|p| last.contains(p)).count();
        println!(
            "step {step}: {} prims (was {}), first diff at {first}, {same} unchanged anywhere",
            now.len(),
            last.len()
        );
        last = now;
    }
}
