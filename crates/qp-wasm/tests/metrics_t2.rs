//! RP3/T2: the metrics page's own bugs (GOAL 9) — long frames against the detected
//! refresh rate, the opening timeline across a new core, the zoom warning — plus the paint
//! list handed to non-pixel paint paths and the automatic comparison's driver (GOAL 8).
#![cfg(feature = "replica")]

use qp_session::ReplayHost;
use qp_wasm::drive::parity_host;
use qp_wasm::shell::bench::{BENCH_SEEK_STEP, PHASES};
use qp_wasm::shell::{Refresh, ShellCore};
use serde_json::{json, Value};

fn core_on(host: &ReplayHost) -> ShellCore {
    let data = host.init().data.clone().expect("data");
    let evs = host.recorded_events().to_vec();
    ShellCore::new("arkin", "tr", &data, &evs).expect("core")
}

fn core() -> ShellCore {
    core_on(&parity_host())
}

fn pump(core: &mut ShellCore, host: &mut ReplayHost) {
    for _ in 0..200 {
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

/// Feeds `series` (ms intervals) as `mode` frames.
fn frames(c: &mut ShellCore, mode: &str, series: &[f64]) {
    let mut t = 1000.0;
    c.metric_frame(mode, t);
    for dt in series {
        t += dt;
        c.metric_frame(mode, t);
    }
}

/// iOS reports rAF intervals as integer milliseconds: a perfect 60 Hz display gives
/// 16, 17, 17, 16, 17, 17, … (R4 counted every 17 as "long").
fn ios_60hz(n: usize) -> Vec<f64> {
    (0..n)
        .map(|k| if k % 3 == 0 { 16.0 } else { 17.0 })
        .collect()
}

#[test]
fn a_60hz_integer_rounded_series_has_no_long_frames() {
    let mut c = core();
    frames(&mut c, "idle", &ios_60hz(300));
    let m = c.metrics_json();
    assert_eq!(m["refresh"]["hz"], 60.0, "{}", m["refresh"]);
    assert_eq!(m["refresh"]["source"], "idle");
    let idle = &m["frame_ms"]["idle"];
    assert_eq!(idle["long"], 0.0, "{idle}");
    assert_eq!(idle["very_long"], 0.0, "{idle}");
    assert!(idle.get("long_16_7").is_none(), "the R4 field is gone");
    let rows = c.metrics_rows().to_string();
    assert!(rows.contains("60 Hz"), "{rows}");
}

#[test]
fn genuine_dropped_frames_are_counted() {
    let mut c = core();
    // 200 frames at 60 Hz (integer ms), of which 20 missed one vsync (33/34 ms) and 4
    // missed two or more (50, 67 ms).
    let mut s = ios_60hz(200);
    for k in 0..20 {
        s[k * 10 + 5] = if k % 2 == 0 { 33.0 } else { 34.0 };
    }
    s[1] = 50.0;
    s[2] = 67.0;
    s[3] = 50.0;
    s[4] = 51.0;
    frames(&mut c, "idle", &s);
    let m = c.metrics_json();
    assert_eq!(m["refresh"]["hz"], 60.0);
    let idle = &m["frame_ms"]["idle"];
    assert_eq!(idle["long"], 0.12, "24 of 200: {idle}");
    assert_eq!(idle["very_long"], 0.02, "4 of 200: {idle}");
    // A scroll series is judged against the same (idle) refresh.
    frames(&mut c, "scroll", &[16.0, 17.0, 33.0, 17.0]);
    assert_eq!(c.metrics_json()["frame_ms"]["scroll"]["long"], 0.25);
}

#[test]
fn a_120hz_display_is_detected_and_its_budget_used() {
    let r = Refresh::estimate(&[8.0, 9.0, 8.0, 8.0, 9.0, 8.0, 8.0, 9.0, 8.0, 8.0].repeat(3));
    assert_eq!(r.hz, 120.0);
    let mut c = core();
    let mut s: Vec<f64> = [8.3, 8.4].repeat(50);
    s[10] = 16.7; // one missed vsync at 120 Hz (would not be long at 60 Hz)
    frames(&mut c, "idle", &s);
    let m = c.metrics_json();
    assert_eq!(m["refresh"]["hz"], 120.0);
    assert_eq!(m["frame_ms"]["idle"]["long"], 0.01);
    // Too few frames: 60 Hz is assumed and said so.
    let r = Refresh::estimate(&[8.0; 5]);
    assert_eq!((r.hz, r.median_ms), (60.0, None));
}

#[test]
fn the_opening_timeline_survives_a_later_start_frame() {
    // The R4 desktop summary: wasm_ready 79 ms, start_frame 836,034 ms — a later start
    // frame (reconnect / backward seek) built a new core whose marks were taken minutes
    // after the opening. Now the new core takes the old one's metrics over.
    let mut first = core();
    first.metric_mark("start_frame", 2596.0);
    first.metric_mark("wasm_ready", 1794.0);
    first.metric_mark("connected", 2687.0);
    first.metric_mark("feed", 3788.0);
    first.metric_mark("first_capsule", 3884.0);
    first.metric_span("tick", 0.0, 3.0);
    let mut next = core();
    next.metric_inherit(&mut first);
    next.metric_mark("start_frame", 836_034.0);
    next.metric_mark("wasm_ready", 1794.0);
    next.metric_mark("connected", 836_048.4);
    let m = next.metrics_json();
    assert_eq!(m["open_ms"]["start_frame"], 2596.0, "{}", m["open_ms"]);
    assert_eq!(m["open_ms"]["connected"], 2687.0);
    assert_eq!(m["open_ms"]["first_capsule"], 3884.0);
    assert_eq!(m["open_repeats"]["start_frame"], 1);
    assert_eq!(m["open_repeats"]["connected"], 1);
    assert_eq!(m["open_flags"], json!([]));
    assert_eq!(m["span_ms"]["tick"]["n"], 1, "the series carry on");
    // A page whose only marks are implausible says so.
    let mut c = core();
    c.metric_mark("wasm_ready", 79.1);
    c.metric_mark("start_frame", 836_034.0);
    c.metric_mark("connected", 836_000.0);
    let flags = c.metrics_json()["open_flags"].to_string();
    assert!(flags.contains("start_frame 836034 ms > 60000"), "{flags}");
    assert!(
        flags.contains("start_frame 836034 ms after connected"),
        "{flags}"
    );
}

#[test]
fn zoom_and_scale_warnings() {
    let codes = |c: &ShellCore| -> Vec<String> {
        c.warnings()
            .iter()
            .map(|w| w["code"].as_str().unwrap().to_owned())
            .collect()
    };
    // The R4 desktop run: DPR 0.8 (zoomed out), painted at scale 1.
    let mut c = core();
    c.metric_device("dpr", 0.800_000_011_920_929);
    c.set_dpr(0.800_000_011_920_929);
    assert_eq!(
        codes(&c),
        ["dpr_below_1", "dpr_fraction", "scale_differs"],
        "{:?}",
        c.warnings()
    );
    assert!(c.metrics_rows().to_string().contains("uyarı"));
    assert_eq!(
        c.metrics_json()["warnings"].as_array().map(Vec::len),
        Some(3)
    );
    // The phone: DPR 3, scale 3.
    let mut c = core();
    c.metric_device("dpr", 3.0);
    c.set_dpr(3.0);
    assert!(codes(&c).is_empty());
    // A fractional DPR above 1 (zoom 125 % on a DPR 2 screen).
    let mut c = core();
    c.metric_device("dpr", 2.5);
    c.set_dpr(2.5);
    assert_eq!(codes(&c), ["dpr_fraction", "scale_differs"]);
}

#[test]
fn the_paint_path_is_in_the_summary() {
    let mut c = core();
    assert_eq!(c.metrics_json()["paint"], Value::Null);
    c.set_paint("webgpu", "ref", "unsupported");
    let p = &c.metrics_json()["paint"];
    assert_eq!(
        *p,
        json!({"requested":"webgpu","used":"ref","fallback":"unsupported"})
    );
    c.set_paint("ref", "ref", "");
    assert_eq!(c.metrics_json()["paint"]["fallback"], Value::Null);
}

#[test]
fn a_removed_paint_path_is_reported() {
    // (R5/T0, K51) `?paint=cpu` opens with `canvas`; the summary and the metrics page say
    // so (a warning first, then the paint row).
    let mut c = core();
    c.metric_device("dpr", 3.0);
    c.set_dpr(3.0);
    c.set_paint("cpu", "canvas", "removed");
    assert_eq!(
        c.metrics_json()["paint"],
        json!({"requested":"cpu","used":"canvas","fallback":"removed"})
    );
    let w = c.warnings();
    assert_eq!(w.len(), 1, "{w:?}");
    assert_eq!(w[0]["code"], json!("paint_removed"));
    let text = w[0]["text"].as_str().unwrap();
    assert!(
        text.contains("`cpu`") && text.contains("`canvas`"),
        "{text}"
    );
    let rows = c.metrics_rows().to_string();
    assert!(rows.contains("canvas (istenen cpu: removed)"), "{rows}");
    assert!(rows.contains("was removed (K51)"), "{rows}");
    // Other fallbacks are not a removal.
    c.set_paint("gpu", "ref", "unsupported");
    assert!(c.warnings().is_empty());
}

#[test]
fn paint_lists_follow_the_scene_and_its_short_circuit() {
    let mut host = parity_host();
    let mut c = core_on(&host);
    pump(&mut c, &mut host);
    let ids: Vec<String> = c.feed().iter().map(|i| i.id.clone()).collect();
    let id = ids
        .into_iter()
        .find(|id| c.face(id).is_some())
        .expect("a face with a scene");
    let scene = c.face(&id).unwrap();
    let list = c.face_paint(&id, false).expect("first hand-out");
    assert_eq!(list["v"], 1);
    assert_eq!(list["scale"], 1);
    assert_eq!(list["w"], scene.width);
    assert_eq!(list["h"], scene.height);
    let n = scene.prims().count();
    assert_eq!(list["prims"].as_array().map(Vec::len), Some(n));
    assert!(list["bg"].as_str().unwrap().starts_with('#'));
    // Unchanged → nothing; forced → again; the pixel path shares the record.
    assert!(c.face_paint(&id, false).is_none());
    assert!(c.face_rgba(&id, false).is_empty());
    assert!(c.face_paint(&id, true).is_some());
    // At scale 3 every coordinate is the reference renderer's scaled one.
    c.set_dpr(3.0);
    let l3 = c.face_paint(&id, false).expect("scale change repaints");
    assert_eq!(l3["w"], scene.width * 3);
    let first_text = |l: &Value| {
        l["prims"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["k"] == "text")
            .cloned()
    };
    if let (Some(a), Some(b)) = (first_text(&list), first_text(&l3)) {
        assert_eq!(b["x"].as_i64().unwrap(), a["x"].as_i64().unwrap() * 3);
        assert!(b["font"].as_str().unwrap().ends_with("px QpMono"), "{b}");
        assert!(b["c"].as_str().unwrap().len() == 9, "#rrggbbaa: {b}");
    }
}

#[test]
fn the_comparison_runs_every_path_and_ends_with_one_summary() {
    // Without a host the core never reaches the seek step: each prepare waits its limit
    // and the opening finds an empty feed; the driver must still finish on its own with
    // one summary that lists every path and says what went wrong.
    let mut c = core();
    c.bench_start(
        &json!({"paths": ["ref", "fixture"], "current": "ref", "phase_ms": 1000}),
        0.0,
    );
    let mut t = 0.0;
    let mut acts = Vec::new();
    let mut scrolls = Vec::new();
    for _ in 0..20_000 {
        t += 16.0;
        let a = c.bench_tick(t, 5000.0, 700.0);
        if a.is_null() {
            break;
        }
        if let Some(s) = a.get("scroll").and_then(Value::as_f64) {
            scrolls.push(s);
        }
        acts.push(a);
    }
    let last = acts.last().expect("actions");
    assert_eq!(last["done"], true, "{last}");
    assert_eq!(last["paint"], "ref", "the page's own path is restored");
    assert_eq!(last["tab"], "metrics");
    let switches: Vec<&Value> = acts.iter().filter_map(|a| a.get("paint")).collect();
    assert_eq!(switches, [&json!("ref"), &json!("fixture"), &json!("ref")]);
    assert!(scrolls.iter().any(|s| *s > 1900.0), "the feed is scrolled");
    assert!(scrolls.iter().all(|s| (0.0..=2000.0).contains(s)));
    let s = c.bench_summary();
    assert_eq!(s["kind"], "qapsule-paint-compare");
    assert_eq!(s["config"]["seek_step"], BENCH_SEEK_STEP);
    let paths = s["paths"].as_array().unwrap();
    assert_eq!(paths.len(), 2);
    assert_eq!(paths[0]["name"], "ref");
    assert_eq!(paths[1]["name"], "fixture");
    for p in paths {
        assert!(
            p["phases"][PHASES[0]]["frame_ms"]["n"].as_u64().unwrap() > 30,
            "{p}"
        );
        assert!(p["phases"][PHASES[1]]["frame_ms"]["n"].as_u64().unwrap() > 30);
        let errs = p["errors"].to_string();
        assert!(errs.contains("prepare") && errs.contains("open"), "{errs}");
    }
    assert!(s["total_ms"].as_f64().unwrap() < 180_000.0);
    let rows = c.bench_rows();
    assert_eq!(rows["head"], json!(["", "ref", "fixture"]));
    assert!(rows["rows"].as_array().unwrap().len() > 10);
    // Finished: no more actions.
    assert!(c.bench_tick(t + 16.0, 0.0, 0.0).is_null());
}

#[test]
fn the_comparison_survives_a_new_core() {
    let mut a = core();
    a.bench_start(&json!({"paths": ["ref"]}), 0.0);
    let _ = a.bench_tick(16.0, 0.0, 0.0);
    let mut b = core();
    b.metric_inherit(&mut a);
    assert!(
        a.bench_tick(32.0, 0.0, 0.0).is_null(),
        "the old core gave it away"
    );
    assert!(!b.bench_tick(32.0, 0.0, 0.0).is_null() || b.bench_running());
}

#[test]
fn the_comparison_selects_and_reverses_paths() {
    // (RP3/S) `only` keeps the named supported paths (registry order); `order: reverse`
    // runs them reversed; an empty selection falls back to `ref`.
    let order = |cfg: Value| {
        let mut c = core();
        c.bench_start(&cfg, 0.0);
        let mut t = 0.0;
        let mut first = None;
        for _ in 0..20_000 {
            t += 16.0;
            let a = c.bench_tick(t, 5000.0, 700.0);
            if a.is_null() {
                break;
            }
            if first.is_none() {
                first = a.get("paint").cloned();
            }
        }
        let names: Vec<Value> = c.bench_summary()["paths"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p["name"].clone())
            .collect();
        (first, names)
    };
    let all = json!(["ref", "canvas", "fixture", "gpu"]);
    let (first, names) = order(json!({"paths": all, "phase_ms": 1000, "only": "gpu, ref"}));
    assert_eq!(names, [json!("ref"), json!("gpu")]);
    assert_eq!(first, Some(json!("ref")));
    let (first, names) = order(json!({"paths": all, "phase_ms": 1000, "order": "reverse"}));
    assert_eq!(
        names,
        [
            json!("gpu"),
            json!("fixture"),
            json!("canvas"),
            json!("ref")
        ]
    );
    assert_eq!(first, Some(json!("gpu")));
    let (_, names) = order(json!({"paths": all, "phase_ms": 1000, "only": "nope"}));
    assert_eq!(names, [json!("ref")]);
}
