//! RP3/M12a: the `cpu` paint path's core side gives the reference renderer's pixels byte
//! for byte over a replay (faces and the open chart, ticks, zoom, drag, style), paints
//! less than the whole frame when little changed, and paints at scale 1 only while a
//! chart gesture is in progress.
#![cfg(feature = "replica")]

use qp_render::cpu::rasterize_scaled;
use qp_session::ReplayHost;
use qp_wasm::drive::parity_host;
use qp_wasm::shell::ShellCore;
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

fn setup(opts: &str) -> (ShellCore, ReplayHost) {
    let mut host = parity_host();
    host.set_record_events(true);
    let data = host.init().data.clone().expect("data");
    let evs = host.recorded_events().to_vec();
    let mut core = ShellCore::new("arkin", "tr", &data, &evs).expect("core");
    core.set_dpr(3.0);
    core.cpu_options(opts);
    pump(&mut core, &mut host);
    seek(&mut core, &mut host, 120);
    (core, host)
}

/// Paints every face (and the scene) through the path and checks each answer's frame
/// against the reference. Returns (paints, partial paints).
fn check_all(core: &mut ShellCore, force: bool, what: &str) -> (usize, usize) {
    let ids: Vec<String> = core.feed().iter().map(|i| i.id.clone()).collect();
    let (mut n, mut partial) = (0, 0);
    for id in ids.iter().take(8) {
        let a = core.cpu_face(id, force);
        if a.is_empty() {
            continue;
        }
        let want = rasterize_scaled(&core.face(id).unwrap(), core.scale()).unwrap();
        assert_eq!(a[2] as u32, want.width());
        assert_eq!(
            core.cpu_frame().unwrap(),
            want.data(),
            "{what}: face {id} differs ({:?})",
            core.cpu_stats(Some(id))
        );
        n += 1;
        if !core.cpu_stats(Some(id)).unwrap().full {
            partial += 1;
        }
    }
    let a = core.cpu_scene(force);
    if !a.is_empty() {
        let s = if a[8] == 1.0 { 1 } else { core.scale() };
        let want = rasterize_scaled(&core.scene().unwrap(), s).unwrap();
        assert_eq!(a[2] as u32, want.width(), "{what}: scene width");
        assert_eq!(
            core.cpu_frame().unwrap(),
            want.data(),
            "{what}: scene differs ({:?})",
            core.cpu_stats(None)
        );
        n += 1;
        if a[6] as u32 != want.width() || a[7] as u32 != want.height() {
            partial += 1;
        }
    }
    (n, partial)
}

#[test]
fn the_path_paints_the_reference_pixels_over_a_replay() {
    for opts in ["", "nolayer", "nodirty", "off"] {
        let (mut core, mut host) = setup(opts);
        let (n, _) = check_all(&mut core, true, "first");
        assert!(n >= 8, "{opts}: {n} paints");
        let mut partial = 0;
        for step in 121..131 {
            seek(&mut core, &mut host, step);
            partial += check_all(&mut core, false, &format!("{opts} step {step}")).1;
        }
        let id = core.feed()[0].id.clone();
        core.open_capsule(&id);
        pump(&mut core, &mut host);
        check_all(&mut core, false, "capsule");
        core.open_to("chart");
        pump(&mut core, &mut host);
        check_all(&mut core, false, "chart");
        for step in 131..141 {
            seek(&mut core, &mut host, step);
            partial += check_all(&mut core, false, &format!("{opts} chart step {step}")).1;
        }
        core.wheel(-1.0, 250.0);
        pump(&mut core, &mut host);
        check_all(&mut core, false, "wheel");
        // A drag: painted at scale 1 while it moves, at the device scale once it ends.
        core.pointer("scene", "down", 1, 195.0, 150.0);
        for k in 1..6 {
            core.pointer("scene", "move", 1, 195.0 + 12.0 * f64::from(k), 150.0);
            pump(&mut core, &mut host);
            assert!(core.cpu_moving());
            let a = core.cpu_scene(false);
            if !a.is_empty() {
                assert_eq!(
                    a[8] == 1.0,
                    !opts.contains("off"),
                    "{opts}: reduced while moving"
                );
                let s = if a[8] == 1.0 { 1 } else { 3 };
                let want = rasterize_scaled(&core.scene().unwrap(), s).unwrap();
                assert_eq!(core.cpu_frame().unwrap(), want.data(), "{opts}: drag {k}");
            }
        }
        core.pointer("scene", "up", 1, 255.0, 150.0);
        pump(&mut core, &mut host);
        assert!(!core.cpu_moving());
        let a = core.cpu_scene(false);
        if !opts.contains("off") {
            assert!(!a.is_empty(), "{opts}: the settle repaint is owed");
            assert_eq!(a[8], 0.0);
            assert_eq!((a[2], a[6]), (1170.0, 1170.0), "{opts}: whole, full scale");
        }
        let want = rasterize_scaled(&core.scene().unwrap(), 3).unwrap();
        assert_eq!(core.cpu_frame().unwrap(), want.data(), "{opts}: settled");
        assert!(core.cpu_scene(false).is_empty(), "{opts}: nothing more");
        core.set_look(&json!({"theme": "light", "alphabet": "rozet"}));
        pump(&mut core, &mut host);
        check_all(&mut core, false, "styled");
        core.back();
        pump(&mut core, &mut host);
        check_all(&mut core, false, "back");
        if opts.is_empty() || opts == "nolayer" {
            assert!(partial > 5, "{opts}: only {partial} partial paints");
        } else {
            assert_eq!(partial, 0, "{opts}");
        }
    }
}

#[test]
fn faces_beyond_the_keep_limit_are_dropped_and_painted_whole_again() {
    let (mut core, _host) = setup("keep=2");
    let ids: Vec<String> = core.feed().iter().map(|i| i.id.clone()).collect();
    for id in ids.iter().take(3) {
        assert!(!core.cpu_face(id, false).is_empty());
    }
    // The first one was dropped: painted again (whole) although nothing changed.
    let a = core.cpu_face(&ids[0], false);
    assert!(!a.is_empty());
    assert!(core.cpu_stats(Some(&ids[0])).unwrap().full);
    // The third one is kept: nothing to paint.
    assert!(core.cpu_face(&ids[2], false).is_empty());
    // Forced: the kept frame is handed over whole, without painting.
    let a = core.cpu_face(&ids[2], true);
    assert_eq!((a[4], a[5], a[6], a[7]), (0.0, 0.0, a[2], a[3]));
    assert!(core.cpu_bytes() > 0);
}
