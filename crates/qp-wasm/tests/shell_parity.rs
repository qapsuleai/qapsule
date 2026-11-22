//! R4/T5 (GOAL criterion 4): the shell core driven with raw pointer events gives the same
//! document and scene digests as the same session driven with protocol messages only.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::cognitive_complexity
)]
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

#[test]
fn style_rejects_unknown_names_and_changes_nothing() {
    let mut h = parity_host();
    let hello =
        r#"{"type":"hello","protocol":"qp/0","principal":"arkin","surface":{"kind":"phone"}}"#;
    h.handle_raw("a", hello);
    for (field, bad) in [("face", "tile"), ("alphabet", "nope"), ("theme", "neon")] {
        let mut g = json!({"kind": "style", "theme": "light"});
        g[field] = json!(bad);
        let m = json!({"type": "input", "id": "s", "gesture": g});
        let out = h.handle_raw("a", &m.to_string());
        let v = serde_json::to_value(&out[0].msg).unwrap();
        assert_eq!(v["error"]["code"], "invalid", "{v}");
        assert_eq!(v["error"]["path"], format!("/gesture/{field}"), "{v}");
    }
    let m = json!({"type":"input","id":"s","gesture":{"kind":"style","alphabet":"rozet"}});
    let v = serde_json::to_value(&h.handle_raw("a", &m.to_string())[0].msg).unwrap();
    let st = &v["data"]["interaction"]["state"];
    assert_eq!(st["alphabet"], "rozet", "{v}");
    assert!(
        st.get("theme").is_none(),
        "a rejected style changed nothing: {v}"
    );
}

/// R4/H-M8910: the `style` gesture's feed layout, transition, outcome style and last look;
/// unknown names are `invalid` at their own path; defaults are not written back.
#[test]
fn style_names_the_m8_m9_m10_m11_designs() {
    let mut h = parity_host();
    let hello =
        r#"{"type":"hello","protocol":"qp/0","principal":"arkin","surface":{"kind":"phone"}}"#;
    h.handle_raw("a", hello);
    for field in ["feed", "transition", "outcome"] {
        let mut g = json!({"kind": "style"});
        g[field] = json!("nope");
        let m = json!({"type": "input", "id": "s", "gesture": g});
        let v = serde_json::to_value(&h.handle_raw("a", &m.to_string())[0].msg).unwrap();
        assert_eq!(v["error"]["code"], "invalid", "{v}");
        assert_eq!(v["error"]["path"], format!("/gesture/{field}"), "{v}");
    }
    let m = json!({"type":"input","id":"s","gesture":{"kind":"style","face":"card",
        "feed":"r3","transition":"r3","outcome":"r3","seen":1234}});
    let v = serde_json::to_value(&h.handle_raw("a", &m.to_string())[0].msg).unwrap();
    let st = &v["data"]["interaction"]["state"];
    for k in ["face", "feed", "transition", "outcome"] {
        assert!(st.get(k).is_none(), "default `{k}` is not written: {v}");
    }
    assert_eq!(st["seen"], 1234, "{v}");
    let m = json!({"type":"input","id":"s","gesture":{"kind":"style","seen":0}});
    let v = serde_json::to_value(&h.handle_raw("a", &m.to_string())[0].msg).unwrap();
    assert!(
        v["data"]["interaction"]["state"].get("seen").is_none(),
        "{v}"
    );
}

/// R4/H-M8910: the shell takes sizes from the core — the face design's height, the device
/// scale (backing pixels), the scene's CSS lengths — and the transition's frames.
#[test]
fn sizes_scale_and_frames_come_from_the_core() {
    use qp_wasm::shell::ShellCore;
    let mut h = parity_host();
    h.set_record_events(true);
    let data = h.init().data.clone().unwrap();
    let mut core = ShellCore::new("arkin", "tr", &data, &[]).unwrap();
    let fs = core.face_size_json();
    assert_eq!(fs["css"], json!([390, qp_grammar::faces::height(None)]));
    assert_eq!(fs["px"], fs["css"]);
    assert_eq!(fs["style"]["height"], "240px");
    assert!(core.set_dpr(2.0));
    assert!(!core.set_dpr(2.2));
    assert_eq!(core.face_size_json()["px"], json!([780, 480]));
    assert!(core.set_dpr(f64::NAN));
    assert_eq!(core.scale(), 1);
    let ch = ShellCore::choices_json();
    for k in ["face", "alphabet", "feed", "transition", "outcome"] {
        assert!(ch[k][0].as_str().is_some(), "{ch}");
    }
    assert_eq!(ch["face"][0], "r3");
    // No transition yet: an empty frame, done at p = 1.
    let f = core.transition_frame(1.0, 0.0, 0.0, 0.0, 0.0);
    assert_eq!(f["done"], true);
    assert_eq!(f["elements"], json!([]));
    assert!((core.transition_progress(100.0) - 0.0).abs() < 1e-12);
    assert!((core.transition_progress(230.0) - 0.5).abs() < 1e-12);
}
