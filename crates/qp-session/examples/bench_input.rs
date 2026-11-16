//! Local benchmark (R4, GOAL criterion 5): the core's cost of gesture → new scene on a
//! phone. A phone connection looks at 60 bars of a bound daily series with the capsule
//! engine running; each sample is one pixel gesture handled by the authority (`drag` /
//! `pinch` on the document view, then on an open capsule's chart) plus compiling the
//! scene the shell would paint next at 390x300. Prints median and p95 in microseconds.
//!
//! ```text
//! cargo run --release -p qp-session --example bench_input
//! ```

use std::path::{Path, PathBuf};
use std::time::Instant;

use qp_schema::protocol::SessionInit;
use qp_session::{CompileOptions, Session};
use serde_json::{json, Value};

const SAMPLES: usize = 2000;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn session() -> Session {
    let t: Value = serde_json::from_str(
        &std::fs::read_to_string(
            root().join("fixtures/protocol/conformance/51-input-tap-hit.json"),
        )
        .expect("run make_input_transcripts first"),
    )
    .expect("json");
    let init: SessionInit = serde_json::from_value(t["init"].clone()).expect("init");
    Session::new(init, &root()).expect("session")
}

fn stats(name: &str, mut v: Vec<f64>) {
    v.sort_by(f64::total_cmp);
    let med = v[v.len() / 2];
    let p95 = v[v.len() * 95 / 100];
    println!(
        "{name:<44} median {med:>8.1} µs   p95 {p95:>8.1} µs   (n = {})",
        v.len()
    );
}

fn run(
    s: &mut Session,
    gesture: impl Fn(usize) -> Value,
    view: Option<&str>,
) -> (Vec<f64>, Vec<f64>) {
    let opts = CompileOptions::new(390, 300);
    let (mut only, mut full) = (Vec::new(), Vec::new());
    for i in 0..SAMPLES {
        let msg = json!({"type":"input","id":"g","gesture":gesture(i)}).to_string();
        let t0 = Instant::now();
        let out = s.handle_raw("p", &msg);
        let t1 = Instant::now();
        assert!(!out.is_empty());
        if let Some(v) = view {
            let scene = s.scene_for("p", Some(v), &opts).expect("scene");
            std::hint::black_box(scene);
        }
        let t2 = Instant::now();
        only.push((t1 - t0).as_secs_f64() * 1e6);
        full.push((t2 - t0).as_secs_f64() * 1e6);
    }
    (only, full)
}

fn main() {
    let mut s = session();
    s.handle_raw(
        "p",
        &json!({"type":"hello","protocol":"qp/0","principal":"arkin","surface":{"kind":"phone"}})
            .to_string(),
    );
    s.handle_raw(
        "p",
        &json!({"type":"input","id":"l","gesture":{"kind":"look","view":"main","from_bar":60,"to_bar":119}}).to_string(),
    );
    let drag = |i: usize| json!({"kind":"drag","target":{"view":"main"},"dx": if i.is_multiple_of(2) { 12.0 } else { -12.0 }});
    let (a, b) = run(&mut s, drag, Some("main"));
    stats("drag, document view (gesture)", a);
    stats("drag, document view (gesture + scene)", b);
    let pinch = |i: usize| json!({"kind":"pinch","target":{"view":"main"},"factor": if i.is_multiple_of(2) { 1.25 } else { 0.8 },"x":200.0});
    let (a, b) = run(&mut s, pinch, Some("main"));
    stats("pinch, document view (gesture)", a);
    stats("pinch, document view (gesture + scene)", b);
    let pan = |i: usize| json!({"kind":"pan","view":"main","bars": if i.is_multiple_of(2) { 2 } else { -2 }});
    let (a, _) = run(&mut s, pan, None);
    stats("pan (bar space, for comparison)", a);
    s.handle_raw(
        "p",
        &json!({"type":"input","id":"o","gesture":{"kind":"open","capsule":"cap.THYAO.breakout.down.124","to":"chart"}}).to_string(),
    );
    // The capsule chart's scene is compiled inside the gesture (pixels → bars need it);
    // the shell compiles the same view once more to paint.
    let cdrag =
        |i: usize| json!({"kind":"drag","dx": if i.is_multiple_of(2) { 12.0 } else { -12.0 }});
    let (a, _) = run(&mut s, cdrag, None);
    stats("drag, open capsule chart (gesture)", a);
    let cpinch = |i: usize| json!({"kind":"pinch","factor": if i.is_multiple_of(2) { 1.25 } else { 0.8 },"x":200.0});
    let (a, _) = run(&mut s, cpinch, None);
    stats("pinch, open capsule chart (gesture)", a);
    let open = |i: usize| {
        if i.is_multiple_of(2) {
            json!({"kind":"close","to":"capsule"})
        } else {
            json!({"kind":"open","capsule":"cap.THYAO.breakout.down.124","to":"chart"})
        }
    };
    let (a, _) = run(&mut s, open, None);
    stats("open/close capsule ↔ chart (with shared)", a);
    println!("log entries after all gestures: {}", s.log().len());
}
