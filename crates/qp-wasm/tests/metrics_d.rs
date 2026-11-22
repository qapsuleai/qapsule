//! R4/D: the metrics page's additions (GOAL 9): p99 and long-frame shares, the zoom
//! mode, device facts, memory samples, the look in the summary and the reset.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::cognitive_complexity
)]
#![cfg(feature = "replica")]

use qp_wasm::drive::parity_host;
use qp_wasm::shell::ShellCore;

fn core() -> ShellCore {
    let host = parity_host();
    let data = host.init().data.clone().expect("data");
    let evs = host.recorded_events();
    ShellCore::new("arkin", "tr", &data, &evs).expect("core")
}

#[test]
fn frame_summaries_carry_p99_and_long_frame_shares() {
    let mut c = core();
    // 98 frames of 10 ms, one of 20 ms, one of 40 ms (100 intervals) in `zoom`.
    let mut t = 0.0;
    c.metric_frame("zoom", t);
    for k in 0..100 {
        t += match k {
            50 => 20.0,
            70 => 40.0,
            _ => 10.0,
        };
        c.metric_frame("zoom", t);
    }
    let m = c.metrics_json();
    let z = &m["frame_ms"]["zoom"];
    assert_eq!(z["n"], 100, "{z}");
    assert_eq!(z["p50"], 10.0);
    assert_eq!(z["p99"], 20.0, "nearest rank: the 99th of 100");
    assert_eq!(z["max"], 40.0);
    assert_eq!(
        z["long"], 0.02,
        "RP3/T2: 10 ms cadence → 90 Hz class, long > 16.7 ms"
    );
    assert_eq!(z["very_long"], 0.01);
    assert!(z.get("gaps_1s").is_none());
    // A 300 ms frame counts; a 2 s gap is only counted.
    c.metric_frame("zoom", t + 300.0);
    c.metric_frame("zoom", t + 2300.0);
    let z = &c.metrics_json()["frame_ms"]["zoom"];
    assert_eq!(z["n"], 101, "{z}");
    assert_eq!(z["max"], 300.0);
    assert_eq!(z["gaps_1s"], 1);
    // Spans keep the plain summary (with p99, without long-frame shares).
    c.metric_span("face", 0.0, 2.0);
    let m = c.metrics_json();
    assert_eq!(m["span_ms"]["face"]["p99"], 2.0);
    assert!(m["span_ms"]["face"].get("long").is_none());
    let rows = c.metrics_rows().to_string();
    assert!(rows.contains("frame.zoom.long"), "{rows}");
}

#[test]
fn device_memory_and_reset() {
    let mut c = core();
    c.metric_device("cores", 8.0);
    c.metric_device("bad", f64::NAN);
    c.metric_memory(3_000_000.0);
    c.metric_memory(2_000_000.0);
    c.metric_memory(0.0); // unknown: ignored
    c.metric_mark("first_capsule", 120.0);
    c.metric_frame("idle", 0.0);
    c.metric_frame("idle", 8.0);
    let m = c.metrics_json();
    assert_eq!(m["device"]["cores"], 8.0);
    assert!(m["device"].get("bad").is_none());
    assert_eq!(m["device"]["scale"], 1);
    assert_eq!(m["memory_bytes"]["js_heap"]["last"], 2_000_000.0);
    assert_eq!(m["memory_bytes"]["js_heap"]["max"], 3_000_000.0);
    // Off the web there is no WASM memory to report.
    assert!(m["memory_bytes"].get("wasm").is_none());
    assert!(m["look"].is_object());
    c.metric_reset();
    let m = c.metrics_json();
    assert_eq!(m["frame_ms"].as_object().map(serde_json::Map::len), Some(0));
    assert_eq!(m["open_ms"]["first_capsule"], 120.0, "marks stay");
    assert_eq!(m["device"]["cores"], 8.0, "device facts stay");
    // The first frame after a reset only starts the series.
    c.metric_frame("idle", 100.0);
    assert_eq!(
        c.metrics_json()["frame_ms"]
            .as_object()
            .map(serde_json::Map::len),
        Some(0)
    );
}
