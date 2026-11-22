//! R5/M18 (K48, K50; `docs/PROTOCOL.md` §23.5): the viewer asks for the open
//! instrument's story list only when the chart opens, keeps it while the chart is open
//! (pinch, drag, replay steps, style) and drops it when the chart closes. Nothing is
//! asked on the feed screen or at capsule depth.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::cognitive_complexity
)]
#![cfg(feature = "replica")]

use qp_wasm::drive::{parity_host, with_viewer_core};
use serde_json::{json, Value};

fn script() -> Vec<Value> {
    serde_json::from_value(json!([
        {"seek_step": 120},
        {"digest": "feed"},
        {"tap_face": {"index": 0, "x": 120, "y": 60}},
        {"digest": "capsule"},
        {"tap_scene": {"x": 195, "y": 150}},
        {"digest": "chart"},
        {"pinch2x": {}},
        {"drag": {"dx": 60}},
        {"digest": "dragged"},
        {"seek_step": 140},
        {"digest": "later"},
        {"back": {}},
        {"digest": "capsule2"},
        {"back": {}},
        {"digest": "back"}
    ]))
    .unwrap()
}

#[test]
fn story_list_is_asked_only_at_chart_depth() {
    let c = with_viewer_core(&mut parity_host(), "arkin", "tr", &script());
    let st = |k: &str| c[k]["stories"].clone();
    // Feed screen and capsule depth: nothing asked, nothing held.
    for k in ["feed", "capsule"] {
        assert_eq!(st(k)["asked"], json!(0), "{k}: {}", st(k));
        assert!(st(k)["rows"].is_null(), "{k}");
    }
    // The chart opened: the open instrument's list, complete.
    let chart = st("chart");
    assert!(chart["asked"].as_u64().is_some_and(|n| n >= 1), "{chart}");
    assert_eq!(chart["complete"], json!(true), "{chart}");
    let rows = chart["rows"].as_u64().expect("rows");
    assert!(rows > 0, "{chart}");
    assert_eq!(chart["total"], json!(rows));
    // Panning and pinching ask nothing more; replay steps update through the topic.
    assert_eq!(st("dragged")["asked"], chart["asked"]);
    let later = st("later");
    assert_eq!(later["asked"], chart["asked"], "{later}");
    assert!(later["rows"].as_u64().is_some_and(|n| n >= rows), "{later}");
    // Back to capsule depth and to the feed: the list is dropped, not asked again.
    for k in ["capsule2", "back"] {
        assert!(st(k)["rows"].is_null(), "{k}: {}", st(k));
        assert_eq!(st(k)["asked"], chart["asked"], "{k}");
    }
}
