//! RP3/M12a: where the time of a software paint goes (native; `--ignored`, prints).
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
    let before = host.recorded_events().len();
    core.seek(&n.to_string());
    pump(core, host);
    let evs: Vec<_> = host.recorded_events()[before..].to_vec();
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
    let evs = host.recorded_events().to_vec();
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

/// The `cpu` path's work over a replay (native, per technique set): time of the core
/// calls, paints, partial paints, prefix-image use and painted area share.
#[test]
#[ignore = "profile; run with --ignored --nocapture"]
fn cpu_path_work() {
    for opts in ["", "nolayer", "nodirty", "off"] {
        let mut host = parity_host();
        host.set_record_events(true);
        let data = host.init().data.clone().expect("data");
        let evs = host.recorded_events().to_vec();
        let mut core = ShellCore::new("arkin", "tr", &data, &evs).expect("core");
        core.set_dpr(3.0);
        core.cpu_options(opts);
        pump(&mut core, &mut host);
        seek(&mut core, &mut host, 120);
        let ids: Vec<String> = core.feed().iter().take(5).map(|i| i.id.clone()).collect();
        for id in &ids {
            core.cpu_face(id, true, false);
        }
        let mut f = Work::default();
        let mut ref_ms = 0.0;
        for id in &ids {
            core.face_rgba(id, true);
        }
        for step in 121..161 {
            seek(&mut core, &mut host, step);
            for id in &ids {
                let t = Instant::now();
                let a = core.cpu_face(id, false, true);
                f.add(&core, Some(id), &a, t);
                // The reference path's call on the same state, for comparison.
                let t = Instant::now();
                let _ = core.face_rgba(id, false);
                ref_ms += t.elapsed().as_secs_f64() * 1e3;
            }
        }
        println!("  ref face_rgba, same 200 calls: {ref_ms:.2} ms");
        let id = ids[0].clone();
        core.open_capsule(&id);
        pump(&mut core, &mut host);
        core.open_to("chart");
        pump(&mut core, &mut host);
        core.cpu_scene(true, false);
        let mut s = Work::default();
        for step in 161..201 {
            seek(&mut core, &mut host, step);
            let t = Instant::now();
            let a = core.cpu_scene(false, true);
            s.add(&core, None, &a, t);
        }
        println!(
            "opts {opts:>8}: faces {} (of 200 calls) | scene {} (of 40) | bytes {}",
            f.line(),
            s.line(),
            core.cpu_bytes()
        );
    }
}

#[derive(Default)]
struct Work {
    ms: f64,
    paints: u32,
    partial: u32,
    layer: u32,
    area: f64,
}

impl Work {
    fn add(&mut self, core: &ShellCore, id: Option<&str>, a: &[f64], t: Instant) {
        self.ms += t.elapsed().as_secs_f64() * 1e3;
        if a.is_empty() {
            return;
        }
        let st = core.cpu_stats(id).expect("stats");
        self.paints += 1;
        self.partial += u32::from(!st.full);
        self.layer += u32::from(st.layer_used);
        #[allow(clippy::cast_precision_loss)]
        let share = st.area as f64 / (a[2] * a[3]);
        self.area += share;
    }

    fn line(&self) -> String {
        format!(
            "{} paints, {} partial, {} from prefix image, mean area {:.3}, {:.2} ms",
            self.paints,
            self.partial,
            self.layer,
            self.area / f64::from(self.paints.max(1)),
            self.ms
        )
    }
}
