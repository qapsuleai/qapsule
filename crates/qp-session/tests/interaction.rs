//! Interaction in the core (R4, `docs/PROTOCOL.md` §4.6, §7): pixel gestures resolve to
//! the bar-space gestures, the view state is presence (1000 inputs leave the log and the
//! document untouched) and an agent reads it with `screen`.

use std::path::{Path, PathBuf};

use qp_schema::protocol::{OutboundLine, SessionInit};
use qp_session::{document_digest, Session};
use serde_json::{json, Value};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The init of the R4 input transcripts (bound THYAO, capsule engine, view `main`).
fn session() -> Session {
    let t: Value = serde_json::from_str(
        &std::fs::read_to_string(
            root().join("fixtures/protocol/conformance/r4-input-tap-hit.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let init: SessionInit = serde_json::from_value(t["init"].clone()).unwrap();
    Session::new(init, &root()).unwrap()
}

const CAP: &str = "cap.THYAO.breakout.down.124";

fn send(s: &mut Session, conn: &str, msg: &Value) -> Vec<OutboundLine> {
    s.handle_raw(conn, &msg.to_string())
}

fn hello(s: &mut Session, conn: &str, who: &str) {
    let w = send(
        s,
        conn,
        &json!({"type":"hello","protocol":"qp/0","principal":who,"surface":{"kind":"phone"}}),
    );
    assert_eq!(serde_json::to_value(&w[0].msg).unwrap()["type"], "welcome");
}

/// The `data` of the first reply (panics on an error).
fn data(s: &mut Session, conn: &str, g: Value) -> Value {
    let out = send(s, conn, &json!({"type":"input","id":"i","gesture":g}));
    let m = serde_json::to_value(&out[0].msg).unwrap();
    assert_eq!(m["type"], "result", "{m}");
    m["data"].clone()
}

fn vp(d: &Value) -> (u64, u64) {
    let v = if d["kind"] == "interaction" {
        &d["interaction"]["viewport"]
    } else {
        &d["viewport"]
    };
    (
        v["from_bar"].as_u64().unwrap(),
        v["to_bar"].as_u64().unwrap(),
    )
}

#[test]
fn drag_and_pinch_equal_pan_and_zoom() {
    // Two connections at the same start; one drags/pinches in pixels, the other sends the
    // bar-space gesture computed from the same scene's slot width.
    let mut s = session();
    hello(&mut s, "px", "arkin");
    hello(&mut s, "bar", "ana");
    let opts = qp_session::CompileOptions::new(390, 300);
    for (i, dx) in [37.0, -120.0, 5.0, 400.0, -3.0, -1000.0]
        .into_iter()
        .enumerate()
    {
        let scene = s.scene_for("px", Some("main"), &opts).unwrap();
        let bars = qp_grammar::gesture::drag_bars(&scene.x, dx);
        let a = vp(&data(
            &mut s,
            "px",
            json!({"kind":"drag","target":{"view":"main"},"dx":dx}),
        ));
        let b = vp(&data(
            &mut s,
            "bar",
            json!({"kind":"pan","view":"main","bars":bars}),
        ));
        assert_eq!(a, b, "drag {i}");
        let f = 1.0 + (i as f64) * 0.3;
        let scene = s.scene_for("px", Some("main"), &opts).unwrap();
        let anchor = qp_grammar::gesture::pinch_anchor(&scene.x, 150.0);
        let a = vp(&data(
            &mut s,
            "px",
            json!({"kind":"pinch","target":{"view":"main"},"factor":f,"x":150.0}),
        ));
        let b = vp(&data(
            &mut s,
            "bar",
            json!({"kind":"zoom","view":"main","factor":f,"anchor_bar":anchor}),
        ));
        assert_eq!(a, b, "pinch {i}");
    }
}

#[test]
fn pure_range_rules_match_the_bar_space_gestures() {
    let mut s = session();
    hello(&mut s, "p", "arkin");
    let last_slot = 128;
    let mut cur = vp(&data(
        &mut s,
        "p",
        json!({"kind":"look","view":"main","from_bar":60,"to_bar":119}),
    ));
    for (i, bars) in [7_i64, -30, 200, -500, 3].into_iter().enumerate() {
        let want = qp_grammar::gesture::pan_range(cur.0, cur.1, bars, last_slot);
        cur = vp(&data(
            &mut s,
            "p",
            json!({"kind":"pan","view":"main","bars":bars}),
        ));
        assert_eq!(cur, want, "pan {i}");
        let f = [1.7, 0.4, 3.0, 0.9, 1.1][i];
        let anchor = cur.0 + (cur.1 - cur.0) / 3;
        let want = qp_grammar::gesture::zoom_range(cur.0, cur.1, f, anchor, last_slot + 1);
        cur = vp(&data(
            &mut s,
            "p",
            json!({"kind":"zoom","view":"main","factor":f,"anchor_bar":anchor}),
        ));
        assert_eq!(cur, want, "zoom {i}");
    }
}

#[test]
fn a_thousand_inputs_leave_log_and_document_untouched() {
    let mut s = session();
    hello(&mut s, "p", "arkin");
    hello(&mut s, "a", "ana");
    let log0 = s.log().len();
    let doc0 = document_digest(s.document());
    let sub = send(
        &mut s,
        "a",
        &json!({"type":"subscribe","id":"ch","topic":{"kind":"changes"}}),
    );
    assert_eq!(sub.len(), 1);
    for i in 0..1000 {
        let g = match i % 8 {
            0 => json!({"kind":"drag","target":{"view":"main"},"dx": (i % 37) as f64 - 18.0}),
            1 => json!({"kind":"pinch","target":{"view":"main"},"factor":1.3,"x":200.0}),
            2 => json!({"kind":"pinch","target":{"view":"main"},"factor":0.77}),
            3 => json!({"kind":"tap_at","target":{"view":"main"},"x":(i % 380) as f64,"y":150.0}),
            4 => json!({"kind":"open","capsule":CAP}),
            5 => json!({"kind":"drag","dx":-12.0}),
            6 => json!({"kind":"tap_at","x":100.0,"y":100.0}),
            _ => json!({"kind":"close","to":"sentence"}),
        };
        let out = send(
            &mut s,
            "p",
            &json!({"type":"input","id":format!("g{i}"),"gesture":g}),
        );
        for l in &out {
            assert_ne!(l.conn, "a", "a gesture reached another connection: {l:?}");
        }
    }
    assert_eq!(s.log().len(), log0);
    assert_eq!(document_digest(s.document()), doc0);
    // The agent reads the view state with `screen`.
    data(
        &mut s,
        "p",
        json!({"kind":"open","capsule":CAP,"to":"chart"}),
    );
    let out = send(
        &mut s,
        "a",
        &json!({"type":"query","id":"s","query":{"kind":"screen","principal":"arkin"}}),
    );
    let m = serde_json::to_value(&out[0].msg).unwrap();
    let st = &m["data"]["screens"][0]["view_state"];
    assert_eq!(st["depth"], "chart", "{m}");
    assert_eq!(st["capsule"], CAP);
    assert!(st["range"]["from_bar"].is_u64());
    assert_eq!(s.log().len(), log0);
}

#[test]
fn opening_rules_and_shared_elements() {
    let mut s = session();
    hello(&mut s, "p", "arkin");
    let d = data(&mut s, "p", json!({"kind":"open","capsule":CAP}));
    let o = &d["interaction"]["opening"];
    assert_eq!(
        (o["from"].as_str(), o["to"].as_str()),
        (Some("sentence"), Some("capsule"))
    );
    let ids: Vec<&str> = o["shared"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x["id"].as_str().unwrap())
        .collect();
    for want in ["glyph", "headline", "price", "bar:124"] {
        assert!(ids.contains(&want), "{ids:?}");
    }
    // A tap on the opened capsule opens its chart; the marked bar stays shared.
    let d = data(&mut s, "p", json!({"kind":"tap_at","x":50.0,"y":200.0}));
    let o = &d["interaction"]["opening"];
    assert_eq!(o["to"], "chart");
    assert!(o["shared"]
        .as_array()
        .unwrap()
        .iter()
        .any(|x| x["id"] == "bar:124"));
    assert!(o["presentation"]["view"].is_object());
    // A surface that asked for at most capsule depth cannot open the chart.
    let w = send(
        &mut s,
        "w",
        &json!({"type":"hello","protocol":"qp/0","principal":"arkin","surface":{"kind":"phone","max_depth":"capsule"}}),
    );
    assert_eq!(w.len(), 1);
    let out = send(
        &mut s,
        "w",
        &json!({"type":"input","id":"x","gesture":{"kind":"open","capsule":CAP,"to":"chart"}}),
    );
    let m = serde_json::to_value(&out[0].msg).unwrap();
    assert_eq!(m["error"]["code"], "invalid", "{m}");
    assert_eq!(m["error"]["path"], "/gesture/to");
}

#[test]
fn frame_size_is_validated_and_remembered() {
    let mut s = session();
    hello(&mut s, "p", "arkin");
    let out = send(
        &mut s,
        "p",
        &json!({"type":"input","id":"x","gesture":{"kind":"tap_at","target":{"view":"main"},"x":1.0,"y":1.0,"width":50}}),
    );
    let m = serde_json::to_value(&out[0].msg).unwrap();
    assert_eq!(m["error"]["path"], "/gesture/width", "{m}");
    let d = data(
        &mut s,
        "p",
        json!({"kind":"tap_at","target":{"view":"main"},"x":1.0,"y":1.0,"width":800,"height":500}),
    );
    assert_eq!(
        d["interaction"]["state"]["frames"]["view:main"],
        json!([800, 500])
    );
}
