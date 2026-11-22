//! `qp explain` and `qp scene`: headless semantic summary and scene description.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::cognitive_complexity
)]

use std::path::{Path, PathBuf};

use assert_cmd::Command;
use qp_schema::scene::SceneDesc;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn qp() -> Command {
    Command::cargo_bin("qp").expect("qp binary is built")
}

fn run(args: &[&str]) -> (bool, String, String) {
    let out = qp().current_dir(root()).args(args).output().unwrap();
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

const INIT: &str = "fixtures/protocol/init/thyao.json";

#[test]
fn explain_main_view() {
    let (ok, text, err) = run(&["explain", "--init", INIT, "--view", "main"]);
    assert!(ok, "{err}");
    for line in [
        "view main \"THYAO 1G\" (1200x600)",
        "x: slots 0..199 of 200 bar(s), 2025-01-02 .. 2025-10-08, 5.6 px per slot",
        "panel 1: y1 linear [0, 100], 2 layer(s)",
        "  l0 candle px \"THYAO 1d\" fields o h l c: 200 item(s); last c=124.69 h=126.61 l=124.47 o=125.26 (bar 199, 2025-10-08)",
        "  l3 rule constant: 1 item(s); last y=70",
        "    /encoding/y = 70 from rsi.overbought",
    ] {
        assert!(text.lines().any(|l| l == line), "missing `{line}` in:\n{text}");
    }
    // Deterministic.
    let (_, again, _) = run(&["explain", "--init", INIT, "--view", "main"]);
    assert_eq!(text, again);
}

#[test]
fn explain_after_a_transcript_and_frame_size() {
    let (ok, text, err) = run(&[
        "explain",
        "--transcript",
        "fixtures/protocol/conformance/16-screen-describe-presenter.json",
        "--view",
        "v-overbought",
        "--width",
        "800",
        "--height",
        "400",
    ]);
    assert!(ok, "{err}");
    assert!(text.starts_with("view v-overbought (800x400)\n"), "{text}");
    assert!(text.contains("x: slots 120..199 of 200 bar(s)"), "{text}");
    assert!(
        text.contains("/encoding/y = 70 from fact-rsi-cross70.level"),
        "{text}"
    );
    assert!(text.contains("emphasis: highlight bars 150..150"), "{text}");
}

#[test]
fn scene_prints_the_scene_description() {
    let (ok, text, err) = run(&[
        "scene", "--init", INIT, "--view", "main", "--width", "600", "--height", "300",
    ]);
    assert!(ok, "{err}");
    let s: SceneDesc = serde_json::from_str(text.trim()).unwrap();
    assert_eq!((s.view.as_str(), s.width, s.height), ("main", 600, 300));
    assert_eq!(s.panels.len(), 3);
    assert_eq!(s.panels[0].layers[0].items.len(), 200);
}

#[test]
fn unknown_view_and_bad_size_fail() {
    let (ok, _, err) = run(&["explain", "--init", INIT, "--view", "mian"]);
    assert!(!ok);
    assert!(err.contains("\"code\":\"not_found\""), "{err}");
    assert!(err.contains("\"did_you_mean\":[\"main\"]"), "{err}");
    let (ok, _, err) = run(&["scene", "--init", INIT, "--view", "main", "--width", "10"]);
    assert!(!ok);
    assert!(err.contains("\"code\":\"invalid\""), "{err}");
    let (ok, _, _) = run(&["explain", "--view", "main"]);
    assert!(!ok, "needs --init or --transcript");
}
