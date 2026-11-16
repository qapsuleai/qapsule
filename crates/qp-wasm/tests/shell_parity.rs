//! R4/T5 (GOAL criterion 4): the shell core driven with raw pointer events gives the same
//! document and scene digests as the same session driven with protocol messages only.
#![cfg(feature = "replica")]

use qp_wasm::drive::{parity_host, shell_less, with_core};
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
        {"digest": "pinched"},
        {"drag": {"dx": 60}},
        {"digest": "dragged"},
        {"style": {"theme": "light", "alphabet": "rozet"}},
        {"digest": "styled"},
        {"back": {}},
        {"back": {}},
        {"digest": "back"}
    ]))
    .unwrap()
}

#[test]
fn shell_core_equals_shell_less() {
    let a = shell_less(&mut parity_host(), "arkin", "tr", &script());
    let b = with_core(&mut parity_host(), "arkin", "tr", &script());
    assert!(
        a["feed"]["faces"].as_array().is_some_and(|f| !f.is_empty()),
        "{a}"
    );
    assert_eq!(a["chart"]["open"]["depth"], "chart", "{}", a["chart"]);
    assert!(a["chart"]["open"]["digest"].is_string());
    for k in [
        "feed", "capsule", "chart", "pinched", "dragged", "styled", "back",
    ] {
        assert_eq!(a[k], b[k], "step {k}");
    }
    assert_ne!(a["chart"]["open"]["digest"], a["pinched"]["open"]["digest"]);
    assert_ne!(
        a["pinched"]["open"]["digest"],
        a["dragged"]["open"]["digest"]
    );
    assert_ne!(
        a["dragged"]["open"]["digest"],
        a["styled"]["open"]["digest"]
    );
}
