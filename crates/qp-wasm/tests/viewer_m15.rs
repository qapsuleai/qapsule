//! RP3/M15 (K48, `docs/PROTOCOL.md` §22): the device as a viewer.
//!
//! - The viewer's faces (the authority's ready scenes) and its open scene (compiled over
//!   the device's own data of the open instrument only) equal the full replica's, digest
//!   for digest, on the parity script; the viewer core (the browser's) equals the
//!   protocol-only viewer driver.
//! - On the feed screen a viewer subscribes to no data; with a capsule open, only to that
//!   instrument's feed.
//! - The device's work and traffic per step do not depend on the size of the universe
//!   (30 vs 300 instruments, natively; 3,000 in the browser measurement).
#![cfg(feature = "replica")]

use qp_wasm::drive::{parity_host, parity_host_n, shell_less, viewer_less, with_viewer_core};
use serde_json::{json, Value};

fn script() -> Vec<Value> {
    script_with(&json!({}))
}

/// The parity script after a first `style` step `first` (empty: none).
fn script_with(first: &Value) -> Vec<Value> {
    let mut s = script_steps();
    if first.as_object().is_some_and(|o| !o.is_empty()) {
        s.insert(0, json!({ "style": first }));
    }
    s
}

fn script_steps() -> Vec<Value> {
    serde_json::from_value(json!([
        {"seek_step": 120},
        {"digest": "feed"},
        {"tap_face": {"index": 0, "x": 120, "y": 60}},
        {"digest": "capsule"},
        {"tap_scene": {"x": 195, "y": 150}},
        {"digest": "chart"},
        {"pinch2x": {}},
        {"digest": "pinched"},
        {"drag": {"dx": 60}},
        {"digest": "dragged"},
        {"seek_step": 126},
        {"digest": "later"},
        {"style": {"theme": "light", "alphabet": "rozet"}},
        {"digest": "styled"},
        {"back": {}},
        {"back": {}},
        {"digest": "back"}
    ]))
    .unwrap()
}

const STEPS: [&str; 8] = [
    "feed", "capsule", "chart", "pinched", "dragged", "later", "styled", "back",
];

#[test]
fn viewer_faces_and_open_scene_equal_the_full_replica() {
    // (R5/S) Under the presented open capsule design (`tesir`) the capsule depth says what
    // the device's own check found (K48), which the full replica does not have: the two
    // are compared under today's card (`r4`); the viewer core and driver below are
    // compared under the presentation's defaults. The story chart likewise: the full
    // replica has no host coin-flip rule (`pavka` DESIGN, "Sınırlar").
    let pin = json!({"open": "r4", "story": "r4"});
    let a = shell_less(&mut parity_host(), "arkin", "tr", &script_with(&pin));
    let v = viewer_less(&mut parity_host(), "arkin", "tr", &script_with(&pin));
    for k in STEPS {
        assert_eq!(a[k]["faces"], v[k]["faces"], "faces at {k}");
        assert_eq!(a[k]["open"], v[k]["open"], "open at {k}");
        assert_eq!(a[k]["look"], v[k]["look"], "look at {k}");
    }
    assert!(a["chart"]["open"]["digest"].is_string());
    assert!(a["later"]["faces"]
        .as_array()
        .is_some_and(|f| f.iter().all(|x| x["digest"].is_string())));
    // Data subscriptions: none on the feed screen; one feed while a capsule is open.
    assert_eq!(v["feed"]["data"], json!([]));
    assert_eq!(v["back"]["data"], json!([]));
    let open = v["chart"]["data"].as_array().expect("data");
    assert_eq!(open.len(), 1, "{}", v["chart"]["data"]);
    assert!(v["chart"]["document"].is_string());
    assert!(v["feed"]["document"].is_null());
    // The open capsule is checked on the device's own data.
    assert_eq!(v["chart"]["trust"], "device", "{}", v["chart"]);
}

#[test]
fn viewer_core_equals_viewer_driver() {
    let v = viewer_less(&mut parity_host(), "arkin", "tr", &script());
    let c = with_viewer_core(&mut parity_host(), "arkin", "tr", &script());
    for k in STEPS {
        for f in ["document", "faces", "open", "look", "data"] {
            assert_eq!(v[k][f], c[k][f], "{f} at {k}");
        }
    }
}

/// Bytes of the viewer connection's messages and data plane per step, on the feed
/// screen and with a capsule open, for a universe of `n` instruments.
fn traffic(n: u32) -> (usize, usize, usize, usize) {
    use qp_session::ReplayHost;
    let mut host: ReplayHost = parity_host_n(n);
    host.set_record_events(true);
    let conn = "m";
    let send = |h: &mut ReplayHost, m: Value| h.handle_raw(conn, &m.to_string());
    let mut served = Vec::new();
    let _ = send(
        &mut host,
        json!({"type":"hello","protocol":"qp/0","principal":"arkin","surface":{"kind":"phone"}}),
    );
    let _ = send(
        &mut host,
        json!({"type":"input","id":"s","gesture":qp_wasm::shell::ShellCore::presented_look()}),
    );
    let _ = send(
        &mut host,
        json!({"type":"replay","id":"r0","control":{"kind":"seek","step":120}}),
    );
    let first = send(
        &mut host,
        json!({"type":"subscribe","id":"v","topic":{"kind":"viewer","limit":40}}),
    );
    let feed: Vec<String> = first
        .iter()
        .find_map(|l| match &l.msg {
            qp_schema::protocol::ServerMsg::Notify {
                event: qp_schema::protocol::Event::Viewer { viewer, .. },
                ..
            } => viewer
                .feed
                .as_ref()
                .map(|f| f.iter().map(|r| r.capsule.id.clone()).collect()),
            _ => None,
        })
        .expect("feed");
    let vis: Vec<&String> = feed.iter().take(3).collect();
    let _ = send(
        &mut host,
        json!({"type":"visible","id":"v1","sub":"v","visible":vis}),
    );
    let step = |h: &mut ReplayHost, served: &mut Vec<_>| -> (usize, usize) {
        let before = h.recorded_len();
        let lines = send(
            h,
            json!({"type":"replay","id":"r","control":{"kind":"step","n":1}}),
        );
        for l in lines.iter().filter(|l| l.conn == conn) {
            if let qp_schema::protocol::ServerMsg::Notify {
                event: qp_schema::protocol::Event::Viewer { viewer, .. },
                ..
            } = &l.msg
            {
                assert!(viewer.feed.as_ref().is_none_or(|f| f.len() <= 40));
                assert!(viewer.faces.len() <= 3, "only the visible faces");
            }
        }
        let msg: usize = lines
            .iter()
            .filter(|l| l.conn == conn)
            .map(|l| serde_json::to_string(&l.msg).unwrap().len())
            .sum();
        let data: usize = qp_wasm::drive::viewer_frames(h, conn, served, before)
            .iter()
            .map(String::len)
            .sum();
        (msg, data)
    };
    let (mut fm, mut fd) = (0, 0);
    for _ in 0..9 {
        let (m, d) = step(&mut host, &mut served);
        fm += m;
        fd += d;
    }
    let _ = send(
        &mut host,
        json!({"type":"input","id":"o","gesture":{"kind":"open","capsule":feed[0],"to":"capsule","width":390,"height":300}}),
    );
    let _ = qp_wasm::drive::viewer_frames(&host, conn, &mut served, host.recorded_len());
    assert_eq!(served.len(), 1, "one feed while open: {served:?}");
    let (mut om, mut od) = (0, 0);
    for _ in 0..9 {
        let (m, d) = step(&mut host, &mut served);
        om += m;
        od += d;
    }
    (fm, fd, om, od)
}

#[test]
fn viewer_traffic_does_not_grow_with_the_universe() {
    let (fm30, fd30, om30, od30) = traffic(30);
    let (fm300, fd300, om300, od300) = traffic(300);
    eprintln!(
        "per 9 steps, feed screen msg/data: 30 {fm30}/{fd30}, 300 {fm300}/{fd300}; open: 30 {om30}/{od30}, 300 {om300}/{od300}"
    );
    let near = |a: usize, b: usize| {
        #[allow(clippy::cast_precision_loss)]
        let (a, b) = (a as f64, b as f64);
        (a - b).abs() <= 0.2 * a.max(b) + 4096.0
    };
    // The data plane of a viewer carries only the replay state on the feed screen and
    // one instrument's events with a capsule open.
    assert!(near(fd30, fd300), "feed data {fd30} vs {fd300}");
    assert!(near(od30, od300), "open data {od30} vs {od300}");
    // The faces of three visible capsules.
    assert!(near(fm30, fm300), "feed msgs {fm30} vs {fm300}");
    // With a capsule open the slice comes again when that instrument's items change (once
    // per closed bar at most): bounded, but which capsule is open differs between the two
    // universes, so only a bound is checked here (the measurement reports the bytes).
    assert!(
        om30 < 400_000 && om300 < 400_000,
        "open msgs {om30}, {om300}"
    );
}

/// (RP3/M15 measurement, `rounds/RP3/experiments/M15.md`) The authority's work per viewer
/// device per step: one extra `viewer_flush` after each step (it computes everything a
/// flush computes — feed, visible faces, slice — and sends nothing new), on the feed screen
/// and with a capsule open; and the step itself. `QP_M15_N` instruments (default 30). Run
/// `cargo test --release -p qp-wasm --features replica --test viewer_m15 -- --ignored --nocapture`.
#[test]
#[ignore = "measurement"]
fn server_work_per_device() {
    use std::time::Instant;
    let n: u32 = std::env::var("QP_M15_N")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30);
    let t = Instant::now();
    let mut host = parity_host_n(n);
    let build = t.elapsed().as_secs_f64() * 1e3;
    let conn = "m";
    let send = |h: &mut qp_session::ReplayHost, m: Value| h.handle_raw(conn, &m.to_string());
    let _ = send(
        &mut host,
        json!({"type":"hello","protocol":"qp/0","principal":"arkin","surface":{"kind":"phone"}}),
    );
    let _ = send(
        &mut host,
        json!({"type":"input","id":"s","gesture":qp_wasm::shell::ShellCore::presented_look()}),
    );
    let first = send(
        &mut host,
        json!({"type":"subscribe","id":"v","topic":{"kind":"viewer","limit":40}}),
    );
    let feed: Vec<String> = first
        .iter()
        .find_map(|l| match &l.msg {
            qp_schema::protocol::ServerMsg::Notify {
                event: qp_schema::protocol::Event::Viewer { viewer, .. },
                ..
            } => viewer
                .feed
                .as_ref()
                .map(|f| f.iter().map(|r| r.capsule.id.clone()).collect()),
            _ => None,
        })
        .expect("feed");
    let vis: Vec<&String> = feed.iter().take(4).collect();
    let _ = send(
        &mut host,
        json!({"type":"visible","id":"v1","sub":"v","visible":vis}),
    );
    let phase = |h: &mut qp_session::ReplayHost| -> (Vec<f64>, Vec<f64>) {
        let (mut steps, mut flush) = (Vec::new(), Vec::new());
        for _ in 0..9 {
            let t = Instant::now();
            let _ = send(
                h,
                json!({"type":"replay","id":"r","control":{"kind":"step","n":1}}),
            );
            steps.push(t.elapsed().as_secs_f64() * 1e3);
            let t = Instant::now();
            let _ = h.session_mut().viewer_flush();
            flush.push(t.elapsed().as_secs_f64() * 1e3);
        }
        (steps, flush)
    };
    let (fs, ff) = phase(&mut host);
    let _ = send(
        &mut host,
        json!({"type":"input","id":"o","gesture":{"kind":"open","capsule":feed[0],"to":"capsule","width":390,"height":300}}),
    );
    let (os, of) = phase(&mut host);
    let med = |v: &[f64]| {
        let mut s = v.to_vec();
        s.sort_by(f64::total_cmp);
        s[s.len() / 2]
    };
    let mean = |v: &[f64]| v.iter().sum::<f64>() / 9.0;
    println!(
        "{}",
        json!({"instruments": n, "build_ms": build,
            "feed": {"step_ms_median": med(&fs), "step_ms_mean": mean(&fs),
                     "flush_ms_median": med(&ff), "flush_ms_mean": mean(&ff)},
            "open": {"step_ms_median": med(&os), "step_ms_mean": mean(&os),
                     "flush_ms_median": med(&of), "flush_ms_mean": mean(&of)}})
    );
}
