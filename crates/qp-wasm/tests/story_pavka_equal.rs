//! (R5/m18-pavka-perf) The story chart `pavka`'s performance change draws exactly what it
//! drew before, end to end on the device: the viewer core of the demo (`story: pavka`,
//! opened to a capsule's chart) in both languages, the default and both named themes,
//! zoomed from the opening range out to every bar (so from a few lanes to thinned lanes
//! and clusters), with the closed stories shown and hidden and with the focus moved to
//! another story. Every scene's digest (`scene_digest`: FNV-1a over the scene's JSON
//! bytes) equals the one recorded with the code before the change (`r5/m18-pavka`
//! d62207a; `tests/data/pavka_scene_digests.txt`). A dense world (history 1000, days 1200:
//! ~200 stories in view and clusters) is checked the same way.
//!
//! `QP_PAVKA_RECORD=1` writes the file instead of comparing (only ever with the old code).
#![cfg(feature = "replica")]

use qp_session::ReplayHost;
use qp_wasm::drive::{parity_host, pump_viewer};
use qp_wasm::shell::{scene_digest, ShellCore};
use serde_json::{json, Value};

struct Run {
    core: ShellCore,
    host: ReplayHost,
    served: Vec<qp_schema::data::FeedKey>,
}

impl Run {
    fn pump(&mut self) {
        pump_viewer(&mut self.core, &mut self.host, &mut self.served);
    }
    fn tap(&mut self, x: f64, y: f64) {
        self.core.pointer("scene", "down", 1, x, y);
        self.core.pointer("scene", "up", 1, x, y);
        self.pump();
    }
    fn look(&mut self, v: &Value) {
        self.core.set_look(v);
        self.pump();
    }
}

fn open(host: ReplayHost, lang: &str, step: u32, look: &Value) -> Run {
    let mut r = Run {
        core: ShellCore::new_viewer("arkin", lang, &[]).expect("core"),
        host,
        served: Vec::new(),
    };
    r.host.set_record_events(true);
    r.pump();
    r.core.seek(&step.to_string());
    r.pump();
    r.look(look);
    let ids: Vec<String> = r.core.feed().iter().map(|i| i.id.clone()).collect();
    assert!(!ids.is_empty(), "feed");
    r.core.open_capsule(&ids[0]);
    r.pump();
    r.tap(195.0, 150.0);
    assert_eq!(r.core.open_json()["depth"], json!("chart"));
    r
}

/// The story lanes of the scene other than the selected one: (centre x, centre y).
fn other_lane(r: &mut Run) -> Option<(f64, f64)> {
    let s = r.core.scene()?;
    let open = r.core.open_json()["capsule"].as_str()?.to_owned();
    s.panels
        .iter()
        .filter(|p| p.name.as_deref() == Some("stories"))
        .flat_map(|p| &p.layers)
        .filter(|l| l.id.starts_with("story:") && l.id != format!("story:{open}"))
        .map(|l| l.items[0].hit)
        .next()
        .map(|h| {
            (
                f64::from(h.left + h.right) / 2.0,
                f64::from(h.top + h.bottom) / 2.0,
            )
        })
}

/// The story part's (lanes, clusters, visible range) of a scene: part of each state's
/// name, so the record shows what was drawn.
fn shape(r: &mut Run, s: &qp_schema::scene::SceneDesc) -> String {
    let layers = || {
        s.panels
            .iter()
            .filter(|p| p.name.as_deref() == Some("stories"))
            .flat_map(|p| &p.layers)
    };
    let lanes = layers().filter(|l| l.id.starts_with("story:")).count();
    let clusters = layers()
        .filter(|l| l.id.starts_with("story-cluster:"))
        .count();
    let range = r
        .core
        .look()
        .range
        .as_ref()
        .map(|v| format!("{}-{}", v.from_bar, v.to_bar))
        .unwrap_or_default();
    format!("lanes {lanes} clusters {clusters} range {range}")
}

/// Every checked state's (name, digest) in a world: per (language, theme) opening, the
/// wheel moved by each of `wheels` in turn (negative: in, positive: out), the closed
/// stories shown and hidden at each, then the focus moved to another story.
fn states(
    host: fn() -> ReplayHost,
    step: u32,
    openings: &[(&str, &str)],
    wheels: &[i32],
    tag: &str,
) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for &(lang, theme) in openings {
        // (R5/S) Pinned to the designs the record was taken with (before R5's presentation
        // defaults: look, face, voice, open capsule).
        let mut look =
            json!({"story": "pavka", "look": "r4", "face": "vadrel", "voice": "r4", "open": "r4"});
        if !theme.is_empty() {
            look["theme"] = json!(theme);
        }
        let th = if theme.is_empty() { "default" } else { theme };
        let mut r = open(host(), lang, step, &look);
        for (i, &z) in wheels.iter().enumerate() {
            for _ in 0..z.unsigned_abs() {
                r.core.wheel(f64::from(z.signum()), 150.0);
                r.pump();
            }
            for hide in [false, true] {
                r.look(&json!({ "hide_closed": hide }));
                let s = r.core.scene().expect("scene");
                let name = format!(
                    "{tag} {lang} {th} wheel{i} {} {}",
                    if hide { "hidden" } else { "shown" },
                    shape(&mut r, &s)
                );
                out.push((name, scene_digest(&s)));
            }
            r.look(&json!({"hide_closed": false}));
        }
        // The focus moved to another story (at the last zoom).
        if let Some((x, y)) = other_lane(&mut r) {
            r.tap(x, y);
            let s = r.core.scene().expect("scene");
            let name = format!("{tag} {lang} {th} focus moved {}", shape(&mut r, &s));
            out.push((name, scene_digest(&s)));
        }
    }
    out
}

fn dense_host() -> ReplayHost {
    let init = qp_wasm::shell::demo_session_init(&qp_wasm::shell::DemoOptions {
        history: 1000,
        days: 1200,
        ..qp_wasm::shell::DemoOptions::default()
    });
    let none = std::env::temp_dir().join("qp-demo-parity-no-local-data");
    ReplayHost::with_local_dir(init, std::path::Path::new("."), &none).expect("host")
}

#[test]
fn pavka_scenes_equal_the_recorded_ones() {
    let all = [
        ("tr", ""),
        ("tr", "light"),
        ("tr", "dark"),
        ("en", ""),
        ("en", "light"),
        ("en", "dark"),
    ];
    let mut got = states(parity_host, 160, &all, &[0, -4, -4, 30], "demo");
    got.extend(states(
        dense_host,
        200,
        &[("tr", ""), ("en", "light")],
        &[0, -6, 3, 6, 40],
        "dense",
    ));
    let text: String = got.iter().map(|(n, d)| format!("{d} {n}\n")).collect();
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/pavka_scene_digests.txt");
    if std::env::var("QP_PAVKA_RECORD").is_ok_and(|v| v == "1") {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &text).unwrap();
        eprintln!("recorded {} scenes", got.len());
        return;
    }
    let want = std::fs::read_to_string(&path).expect("recorded digests");
    let distinct: std::collections::BTreeSet<&str> = got.iter().map(|(_, d)| d.as_str()).collect();
    eprintln!("{} scenes, {} distinct", got.len(), distinct.len());
    assert_eq!(text, want);
}
