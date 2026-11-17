//! R5/M18 story chart `pavka` end to end (K50, K54, K48): the viewer core of the demo
//! with `story: pavka`, opened to a capsule's chart.
//!
//! - The chart depth draws every story of the open instrument's visible range (the list
//!   the authority sent, §23), the selected one last, in the focus colour.
//! - The device's scene and the authority's agree: a tap on another story's lane (the
//!   device's pixels, resolved by the authority's `tap_at`) moves the focus to exactly that
//!   story, over the same visible range; a tap on the selected lane opens its evidence.
//! - The checkbox hides and shows the closed stories (view state, `hide_closed`).
//! - Without `story` nothing changes (the default chart).
//!
//! `QP_STORY_PNG=<dir>` writes the scenes as PNG files there (a visual check).
#![cfg(feature = "replica")]

use qp_schema::scene::{DataSource, SceneDesc};
use qp_session::ReplayHost;
use qp_wasm::drive::{parity_host, pump_viewer};
use qp_wasm::shell::ShellCore;
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
    fn scene(&mut self) -> SceneDesc {
        self.core.scene().expect("open scene")
    }
    fn open(&mut self) -> Value {
        self.core.open_json()
    }
    fn png(&mut self, name: &str) {
        if let Ok(dir) = std::env::var("QP_STORY_PNG") {
            let s = self.scene();
            let png = qp_render::render_scene_png(&s).expect("png");
            std::fs::write(std::path::Path::new(&dir).join(format!("{name}.png")), png).unwrap();
        }
    }
}

/// The demo (synthetic, paused) at step `step`, the first feed capsule opened to its chart,
/// with `look` set first.
fn chart(look: &Value, lang: &str, step: u32) -> Run {
    let mut host = parity_host();
    host.set_record_events(true);
    let core = ShellCore::new_viewer("arkin", lang, &[]).expect("core");
    let mut r = Run {
        core,
        host,
        served: Vec::new(),
    };
    r.pump();
    r.core.seek(&step.to_string());
    r.pump();
    if look.as_object().is_some_and(|o| !o.is_empty()) {
        r.core.set_look(look);
        r.pump();
    }
    let ids: Vec<String> = r.core.feed().iter().map(|i| i.id.clone()).collect();
    assert!(!ids.is_empty(), "feed");
    r.core.open_capsule(&ids[0]);
    r.pump();
    r.tap(195.0, 150.0);
    assert_eq!(r.open()["depth"], json!("chart"), "{}", r.open());
    r
}

fn story_layers(s: &SceneDesc) -> Vec<(String, String, [i32; 4])> {
    s.panels
        .iter()
        .filter(|p| p.name.as_deref() == Some("stories"))
        .flat_map(|p| &p.layers)
        .filter(|l| l.id.starts_with("story:"))
        .filter_map(|l| match &l.data {
            DataSource::Finding { finding } => {
                let h = l.items[0].hit;
                Some((
                    l.id.clone(),
                    finding.clone(),
                    [h.left, h.top, h.right, h.bottom],
                ))
            }
            _ => None,
        })
        .collect()
}

fn open_finding(r: &mut Run) -> String {
    let cap = r.open()["capsule"].as_str().unwrap().to_owned();
    r.core.stories_json()["stories"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == json!(cap))
        .map(|s| s["finding"].as_str().unwrap().to_owned())
        .expect("open capsule in the story list")
}

#[test]
fn story_chart_end_to_end() {
    let mut r = chart(&json!({"story": "pavka"}), "tr", 160);
    assert_eq!(r.core.look().story.as_deref(), Some("pavka"));
    let st = r.core.stories_json();
    assert_eq!(st["complete"], json!(true));
    let rows = st["stories"].as_array().unwrap().len();
    let s = r.scene();
    r.png("chart");
    let lanes = story_layers(&s);
    eprintln!("stories held {rows}, lanes drawn {}", lanes.len());
    assert!(!lanes.is_empty(), "lanes drawn");
    // The selected story is drawn last (wins the point query on its row).
    let focus = open_finding(&mut r);
    assert_eq!(lanes.last().unwrap().1, focus);

    // A tap on another story moves the focus to it, over the same range.
    let range0 = r.core.look().range.clone().expect("range");
    let other = lanes
        .iter()
        .rev()
        .find(|(_, f, _)| *f != focus)
        .cloned()
        .expect("another story in view");
    let [l, t, rt, b] = other.2;
    r.tap(f64::from(l + rt) / 2.0, f64::from(t + b) / 2.0);
    let cap = other.0.trim_start_matches("story:").to_owned();
    assert_eq!(r.open()["capsule"], json!(cap), "focus moved: {}", r.open());
    assert_eq!(r.open()["depth"], json!("chart"));
    let range1 = r.core.look().range.clone().expect("range");
    assert_eq!(
        (range0.from_bar, range0.to_bar),
        (range1.from_bar, range1.to_bar)
    );
    let s1 = r.scene();
    r.png("focus-moved");
    assert_eq!(
        story_layers(&s1).last().unwrap().1,
        other.1,
        "new focus drawn last"
    );
    // The story list was not asked again (same instrument).
    let asked = r.core.stories_asked();

    // Hide the closed stories: the checkbox in the summary line's right margin.
    let tg = s1
        .panels
        .iter()
        .flat_map(|p| &p.layers)
        .find(|l| l.id == "story-toggle")
        .expect("checkbox")
        .items[0]
        .hit;
    r.tap(
        f64::from(tg.left + tg.right) / 2.0,
        f64::from(tg.top + tg.bottom) / 2.0,
    );
    assert!(r.core.look().hide_closed, "hidden");
    let s2 = r.scene();
    r.png("closed-hidden");
    let st = r.core.stories_json();
    let closed: Vec<String> = st["stories"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| !s["closed_bar"].is_null())
        .map(|s| s["finding"].as_str().unwrap().to_owned())
        .collect();
    for (_, f, _) in story_layers(&s2) {
        assert!(!closed.contains(&f) || f == other.1, "{f} closed but drawn");
    }
    r.tap(
        f64::from(tg.left + tg.right) / 2.0,
        f64::from(tg.top + tg.bottom) / 2.0,
    );
    assert!(!r.core.look().hide_closed, "shown again");
    assert_eq!(story_layers(&r.scene()), story_layers(&s1));

    // A tap on the selected lane opens its evidence (the opening rule).
    let mine = story_layers(&s1).last().unwrap().2;
    r.tap(
        f64::from(mine[0] + mine[2]) / 2.0,
        f64::from(mine[1] + mine[3]) / 2.0,
    );
    assert_eq!(r.open()["depth"], json!("evidence"), "{}", r.open());
    assert_eq!(r.core.stories_asked(), asked);

    // Zoomed out to every bar: still drawn (thinned or clustered), English words.
    let mut r = chart(&json!({"story": "pavka"}), "en", 160);
    for _ in 0..12 {
        r.core.wheel(1.0, 150.0);
        r.pump();
    }
    let s = r.scene();
    r.png("zoomed-out-en");
    let n = story_layers(&s).len();
    let clusters = s
        .panels
        .iter()
        .flat_map(|p| &p.layers)
        .filter(|l| l.id.starts_with("story-cluster:"))
        .count();
    eprintln!(
        "zoomed out: range {:?}, lanes {n}, clusters {clusters}",
        r.core.look().range
    );
    assert!(n > 0);
}

#[test]
fn without_a_story_design_the_chart_is_unchanged() {
    let mut a = chart(&json!({}), "tr", 160);
    let mut b = chart(&json!({"story": "r4"}), "tr", 160);
    assert_eq!(b.core.look().story, None, "r4 is the default (not stored)");
    let (sa, sb) = (a.scene(), b.scene());
    assert_eq!(sa, sb);
    assert!(story_layers(&sa).is_empty());
    assert!(!sa
        .panels
        .iter()
        .any(|p| p.name.as_deref() == Some("stories")));
}

#[test]
fn an_unknown_story_design_is_refused() {
    let mut r = chart(&json!({}), "tr", 160);
    r.core.set_look(&json!({"story": "nope"}));
    r.pump();
    let e = r.core.take_error().unwrap_or_default();
    assert!(e.contains("unknown story"), "{e}");
    assert_eq!(r.core.look().story, None);
}

/// Story counts of a longer synthetic history (for the K52 measurement's setting).
#[test]
#[ignore = "measurement helper: cargo test -- --ignored story_density"]
fn story_density() {
    for (history, days) in [(120, 260), (800, 1000), (1000, 1200)] {
        let init = qp_wasm::shell::demo_session_init(&qp_wasm::shell::DemoOptions {
            history,
            days,
            ..qp_wasm::shell::DemoOptions::default()
        });
        let none = std::env::temp_dir().join("qp-demo-parity-no-local-data");
        let t0 = std::time::Instant::now();
        let host =
            ReplayHost::with_local_dir(init, std::path::Path::new("."), &none).expect("host");
        let mut r = Run {
            core: ShellCore::new_viewer("arkin", "tr", &[]).expect("core"),
            host,
            served: Vec::new(),
        };
        r.host.set_record_events(true);
        r.pump();
        r.core.seek("200");
        r.pump();
        r.core.set_look(&json!({"story": "pavka"}));
        r.pump();
        let ids: Vec<String> = r.core.feed().iter().map(|i| i.id.clone()).collect();
        r.core.open_capsule(&ids[0]);
        r.pump();
        r.tap(195.0, 150.0);
        for _ in 0..30 {
            r.core.wheel(1.0, 150.0);
            r.pump();
        }
        let s = r.scene();
        let n = r.core.stories_json()["stories"]
            .as_array()
            .map_or(0, Vec::len);
        eprintln!(
            "history {history} days {days}: {n} stories, range {:?}, lanes {}, host {:?}",
            r.core.look().range.as_ref().map(|v| (v.from_bar, v.to_bar)),
            story_layers(&s).len(),
            t0.elapsed()
        );
        if history == 1000 {
            r.png("dense");
            // Primitives by kind: the story part vs the whole scene.
            let kind = |p: &qp_schema::scene::Prim| match p {
                qp_schema::scene::Prim::Rect { .. } => "rect",
                qp_schema::scene::Prim::Fill { .. } => "fill",
                qp_schema::scene::Prim::Circle { .. } => "circle",
                qp_schema::scene::Prim::Polyline { .. } => "line",
                qp_schema::scene::Prim::Text { .. } => "text",
            };
            let mut all = std::collections::BTreeMap::new();
            for p in s.prims() {
                *all.entry(kind(p)).or_insert(0) += 1;
            }
            let mut story = std::collections::BTreeMap::new();
            for p in s
                .panels
                .iter()
                .filter(|p| p.name.as_deref() == Some("stories"))
                .flat_map(|p| &p.layers)
                .flat_map(|l| &l.prims)
            {
                *story.entry(kind(p)).or_insert(0) += 1;
            }
            eprintln!("prims: scene {all:?}; story panel {story:?}");
        }
    }
}
