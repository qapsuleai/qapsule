//! RP3/M14 (GOAL criterion 6): changes accumulate in the core for one paint pass per
//! display frame; a frame without data events raises nothing to paint; the paint pass
//! paints only the faces on screen and none behind the open layer; the page's mode
//! (secure context, WebGPU, JIT probe) is in the summary with its warnings.
#![cfg(feature = "replica")]

use qp_session::ReplayHost;
use qp_wasm::drive::parity_host;
use qp_wasm::shell::{core_from_start, ShellCore};
use serde_json::{json, Value};

/// Passes the core's messages to `host` and the answers back (through the timed frame
/// path, as the page does) until it is idle.
fn pump(core: &mut ShellCore, host: &mut ReplayHost, t: f64) {
    for _ in 0..200 {
        let out = core.take_outbox();
        if out.is_empty() {
            break;
        }
        for m in out {
            for l in host.handle_raw("w", &m.to_string()) {
                if l.conn == "w" {
                    let msg = serde_json::to_value(&l.msg).expect("msg");
                    core.on_frame_at(&json!({ "msg": msg }).to_string(), Some(t))
                        .expect("frame");
                }
            }
        }
    }
}

/// A page's core on the demo host after `n` steps (full replica), welcomed, with its feed.
fn page(n: usize) -> (ShellCore, ReplayHost) {
    let mut h = parity_host();
    h.set_record_events(true);
    for _ in 0..n {
        let _ = h.take_step();
    }
    let mut data = h.init().data.clone().expect("data");
    data.entitlements.retain(|p, _| p == "arkin");
    let start = json!({"conn":"c1","token":"t","principal":"arkin","lang":"tr",
        "data":data,"events":h.recorded_events(),"info":{}})
    .to_string();
    let mut core = core_from_start(&start).expect("core");
    pump(&mut core, &mut h, 5.0);
    (core, h)
}

/// The next step of `h` as one data frame.
fn data_frame(h: &mut ReplayHost) -> String {
    let from = h.recorded_len();
    let _ = h.take_step();
    let evs: Vec<Value> = h.recorded_events()[from..]
        .iter()
        .map(|e| serde_json::to_value(e).expect("event"))
        .collect();
    json!({"data": evs, "replay": h.state()}).to_string()
}

#[test]
fn frames_within_one_display_frame_cost_one_paint_pass() {
    let (mut core, mut h) = page(120);
    // The opening's changes were taken by nobody yet: one pass takes them all.
    assert!(core.has_changes());
    let first = core.take_changes().expect("opening changes");
    assert_eq!(first["feed"], json!(true));
    assert!(core.take_changes().is_none());
    core.metric_reset();
    // Two data frames before the next animation frame: one pass, the union of changes.
    let a = data_frame(&mut h);
    let b = data_frame(&mut h);
    core.on_frame_at(&a, Some(100.0)).expect("a");
    core.on_frame_at(&b, Some(104.0)).expect("b");
    let ch = core.take_changes().expect("changes");
    assert_eq!(ch, json!({"feed": true, "scene": true, "status": true}));
    assert!(core.take_changes().is_none(), "taken once");
    core.metric_pass(110.0, 113.0);
    let m = core.metrics_json();
    assert_eq!(m["counts"]["data_frames"], json!(2));
    assert_eq!(m["counts"]["paint_passes"], json!(1));
    assert_eq!(m["counts"]["data_passes"], json!(1));
    // (R5/T0) Two data frames are too few for a rate: "insufficient samples", not 0.5.
    assert_eq!(m["counts"]["passes_per_data_frame"], Value::Null);
    assert_eq!(m["counts"]["tick_plus_pass_ms"], Value::Null);
    // From the first frame's arrival (100) to the end of the pass (113); its wait (110).
    assert_eq!(m["span_ms"]["frame_to_paint"]["max"], json!(13.0));
    assert_eq!(m["span_ms"]["pass_wait"]["max"], json!(10.0));
    assert_eq!(m["span_ms"]["paint_pass"]["max"], json!(3.0));
}

#[test]
fn the_per_data_frame_rates_count_only_data_passes_and_need_enough_frames() {
    // (R5/T0) The metrics page's "tik + geçiş": RP3's phone scroll phase had one data
    // frame and 42 passes (scrolling, taps), and read 42 passes and 466 ms per data
    // frame. Passes without a data frame's changes no longer count; one data frame is
    // "insufficient samples".
    let (mut core, mut h) = page(120);
    let _ = core.take_changes();
    core.metric_reset();
    let mut t = 1000.0;
    // One data frame among 41 passes of its own (a path's repaint request, scrolling).
    let f = data_frame(&mut h);
    core.on_frame_at(&f, Some(t)).expect("frame");
    core.metric_span("tick", t, t + 2.0);
    assert!(core.take_changes().is_some());
    core.metric_pass(t + 3.0, t + 13.0);
    for _ in 0..41 {
        t += 16.0;
        assert!(core.take_changes().is_none());
        core.metric_pass(t, t + 11.0);
    }
    let c = core.metrics_json()["counts"].clone();
    assert_eq!(c["data_frames"], json!(1));
    assert_eq!(c["paint_passes"], json!(42));
    assert_eq!(c["data_passes"], json!(1));
    assert_eq!(c["passes_per_data_frame"], Value::Null);
    assert_eq!(c["tick_plus_pass_ms"], Value::Null);
    let rows = core.metrics_rows().to_string();
    assert!(
        rows.contains("yetersiz örnek (veri çerçevesi 1, en az 10 gerekir)"),
        "{rows}"
    );
    assert!(!rows.contains("null") && !rows.contains("NaN"), "{rows}");
    // Ten more data frames, each painted by its own pass (tick 2 ms, pass 4 ms), with
    // non-data passes in between: rates over the data passes only.
    core.metric_reset();
    for _ in 0..10 {
        t += 16.0;
        let f = data_frame(&mut h);
        core.on_frame_at(&f, Some(t)).expect("frame");
        core.metric_span("tick", t, t + 2.0);
        assert!(core.take_changes().is_some());
        core.metric_pass(t + 3.0, t + 7.0);
        t += 16.0;
        core.metric_pass(t, t + 50.0);
    }
    let c = core.metrics_json()["counts"].clone();
    assert_eq!(c["data_frames"], json!(10));
    assert_eq!(c["paint_passes"], json!(20));
    assert_eq!(c["data_passes"], json!(10));
    assert_eq!(c["passes_per_data_frame"], json!(1.0));
    assert_eq!(c["tick_plus_pass_ms"], json!(6.0));
    let rows = core.metrics_rows().to_string();
    assert!(
        rows.contains("6.00 ms / veri çerçevesi · 1.00 geçiş / veri çerçevesi"),
        "{rows}"
    );
}

#[test]
fn a_frame_without_data_events_raises_nothing_to_paint() {
    let (mut core, h) = page(120);
    let _ = core.take_changes();
    let only_replay = json!({"data": [], "replay": h.state()}).to_string();
    assert_eq!(
        core.on_frame_at(&only_replay, Some(1.0)).expect("frame"),
        json!({"status": true})
    );
    assert_eq!(core.take_changes(), Some(json!({"status": true})));
    // Nothing at all: no pass.
    core.on_frame_at("{}", Some(2.0)).expect("empty");
    assert!(core.take_changes().is_none());
}

#[test]
fn the_pass_paints_the_faces_on_screen_and_none_behind_the_open_layer() {
    let (mut core, mut h) = page(120);
    let _ = core.take_changes();
    let ids: Vec<String> = core.feed().iter().map(|i| i.id.clone()).collect();
    assert!(ids.len() > 3, "a feed of several capsules");
    core.visible(&ids[0], true);
    core.visible(&ids[1], true);
    core.visible(&ids[2], true);
    core.visible(&ids[2], false);
    core.metric_reset();
    let faces = core.faces_to_paint();
    assert_eq!(faces, vec![ids[0].clone(), ids[1].clone()]);
    let skipped = core.metrics_json()["counts"]["faces_skipped_hidden"]
        .as_u64()
        .expect("count");
    assert!(skipped >= 1, "the off-screen faces are counted");
    // Open the first capsule: the layer covers the feed, no face is painted on ticks.
    core.open_capsule(&ids[0]);
    pump(&mut core, &mut h, 10.0);
    assert_ne!(core.open_json()["depth"], json!("sentence"));
    let f = data_frame(&mut h);
    core.on_frame_at(&f, Some(20.0)).expect("tick");
    assert!(core.take_changes().is_some());
    assert!(core.faces_to_paint().is_empty());
}

#[test]
fn the_summary_states_the_pages_mode() {
    let (mut core, _h) = page(10);
    let m = core.metrics_json();
    assert_eq!(m["mode"]["jit"]["state"], json!("unknown"));
    assert_eq!(m["mode"]["secure_context"], Value::Null);
    core.metric_flag("secure_context", false);
    core.metric_flag("webgpu", false);
    core.metric_span("jit_probe", 0.0, 40.0);
    // The probe survives a reset of the series (it runs once).
    core.metric_reset();
    let m = core.metrics_json();
    assert_eq!(m["mode"]["secure_context"], json!(false));
    assert_eq!(m["mode"]["webgpu"], json!(false));
    assert_eq!(m["mode"]["jit"]["state"], json!("likely_off"));
    assert_eq!(m["mode"]["jit"]["probe_ms"], json!(40.0));
    assert!(m["span_ms"].get("jit_probe").is_none());
    let codes: Vec<&str> = m["warnings"]
        .as_array()
        .expect("warnings")
        .iter()
        .filter_map(|w| w["code"].as_str())
        .collect();
    assert!(codes.contains(&"insecure_context"));
    assert!(codes.contains(&"jit_off"));
    let rows = core.metrics_rows().to_string();
    assert!(rows.contains("secure_context") && rows.contains("jit"));
    core.metric_flag("secure_context", true);
    core.metric_span("jit_probe", 0.0, 2.0);
    let m = core.metrics_json();
    assert_eq!(m["mode"]["jit"]["state"], json!("likely_on"));
    assert!(m["warnings"].as_array().expect("warnings").is_empty());
}

#[test]
fn the_welcome_marks_connected_at_the_frames_arrival() {
    let (core, _h) = page(10);
    assert_eq!(core.metrics_json()["open_ms"]["connected"], json!(5.0));
}
