//! R5/M18 `korda` (lifelines) through the shell core and the authority: the story chart
//! at chart depth over the device's story list (`docs/PROTOCOL.md` §23.5), focus moving to
//! a tapped lane, a cluster zooming, the closed stories hidden and shown (K54), and the
//! device's scene telling the same as the authority's point query: every tap below is
//! aimed with the device's own `hit` reading of the scene it painted and the authority's
//! answer is checked against it.
#![cfg(feature = "replica")]

use qp_schema::protocol::OpenDepth;
use qp_schema::scene::{DataSource, PixelRect, SceneDesc, ScenePanel};
use qp_session::{hit_scene, ReplayHost};
use qp_wasm::drive::{parity_host, pump_viewer};
use qp_wasm::shell::ShellCore;
use serde_json::{json, Value};

struct Dev {
    host: ReplayHost,
    core: ShellCore,
    served: Vec<qp_schema::data::FeedKey>,
}

impl Dev {
    fn new(step: u64, lang: &str) -> Self {
        let mut host = parity_host();
        host.set_record_events(true);
        let core = ShellCore::new_viewer("arkin", lang, &[]).expect("core");
        let mut d = Dev {
            host,
            core,
            served: Vec::new(),
        };
        d.pump();
        d.core.seek(&step.to_string());
        d.pump();
        d
    }
    fn pump(&mut self) {
        pump_viewer(&mut self.core, &mut self.host, &mut self.served);
    }
    fn tap(&mut self, x: f64, y: f64) {
        self.core.pointer("scene", "down", 1, x, y);
        self.core.pointer("scene", "up", 1, x, y);
        self.pump();
    }
    fn style(&mut self, v: &Value) {
        self.core.set_look(v);
        self.pump();
    }
    /// Opens the first feed capsule down to its chart.
    fn open_chart(&mut self) {
        let ids: Vec<String> = self.core.feed().iter().map(|r| r.id.clone()).collect();
        for id in &ids {
            self.core.visible(id, true);
        }
        self.pump();
        let target = format!("face:{}", ids[0]);
        self.core.pointer(&target, "down", 1, 120.0, 60.0);
        self.core.pointer(&target, "up", 1, 120.0, 60.0);
        self.pump();
        self.tap(195.0, 150.0);
        assert_eq!(self.core.look().depth, OpenDepth::Chart, "chart open");
    }
    fn pinch(&mut self) {
        self.core.pointer("scene", "down", 1, 150.0, 100.0);
        self.core.pointer("scene", "down", 2, 250.0, 100.0);
        self.core.pointer("scene", "move", 2, 350.0, 100.0);
        self.core.pointer("scene", "up", 2, 350.0, 100.0);
        self.core.pointer("scene", "up", 1, 150.0, 100.0);
        self.pump();
    }
    fn scene(&mut self) -> SceneDesc {
        self.core.scene().expect("open scene")
    }
    fn open_capsule(&mut self) -> String {
        self.core.open_json()["capsule"]
            .as_str()
            .unwrap_or_default()
            .to_owned()
    }
}

fn stories_panel(s: &SceneDesc) -> &ScenePanel {
    s.panels
        .iter()
        .find(|p| p.name.as_deref() == Some("stories"))
        .expect("stories panel")
}

fn center(r: &PixelRect) -> (f64, f64) {
    (
        f64::from(r.left + r.right - 1) / 2.0,
        f64::from(r.top + r.bottom - 1) / 2.0,
    )
}

/// Lane layers `(layer id, finding, hit box)` of the story panel.
fn lanes(s: &SceneDesc) -> Vec<(String, String, PixelRect)> {
    stories_panel(s)
        .layers
        .iter()
        .filter(|l| l.id.starts_with("story:"))
        .map(|l| match &l.data {
            DataSource::Finding { finding } => (l.id.clone(), finding.clone(), l.items[0].hit),
            _ => panic!("a lane draws a finding"),
        })
        .collect()
}

/// The device's own reading of what is under (`x`, `y`).
fn device_hit(s: &SceneDesc, x: f64, y: f64) -> Option<String> {
    hit_scene(s, x, y, &[]).item.map(|i| i.layer)
}

fn dump(s: &SceneDesc, name: &str) {
    if let Ok(dir) = std::env::var("QP_KORDA_PNG") {
        let png = qp_render::render_scene_png(s).expect("png");
        std::fs::write(format!("{dir}/{name}.png"), png).expect("write");
    }
}

/// The open capsule's finding is drawn as the selected lane: first under the header.
fn selected_on_top(s: &SceneDesc, cap: &str) {
    let p = stories_panel(s);
    let l = p
        .layers
        .iter()
        .find(|l| l.id == format!("story:{cap}"))
        .expect("the open story has a lane");
    assert_eq!(
        l.items[0].hit.top,
        p.rect.top + qp_grammar::stories::korda::HEADER_PX
    );
}

#[test]
fn default_chart_has_no_story_panel() {
    let mut d = Dev::new(700, "tr");
    d.open_chart();
    let s = d.scene();
    assert!(s
        .panels
        .iter()
        .all(|p| p.name.as_deref() != Some("stories")));
    // The list is still asked for (K48), only not drawn.
    assert!(d.core.stories_asked() >= 1);
}

#[test]
fn story_chart_draws_every_story_in_view_and_moves_the_focus() {
    let mut d = Dev::new(2000, "tr");
    d.style(&json!({"story":"korda"}));
    assert_eq!(d.core.look().story.as_deref(), Some("korda"));
    d.open_chart();
    let cap0 = d.open_capsule();
    let s = d.scene();
    dump(&s, "focus-0");
    selected_on_top(&s, &cap0);
    // Every story of the list whose life meets the visible range has a lane.
    let st = d.core.stories_json();
    let rows = st["stories"].as_array().expect("rows").clone();
    let now = s.bars - 1;
    let in_view = rows
        .iter()
        .filter(|r| {
            let born = r["born_bar"].as_u64().unwrap();
            let end = r["closed_bar"]
                .as_u64()
                .unwrap_or_else(|| r["last_bar"].as_u64().unwrap().max(now));
            born <= s.x.to_bar && end >= s.x.from_bar
        })
        .count();
    assert!(in_view >= 20, "a crowded chart: {in_view}");
    assert_eq!(lanes(&s).len(), in_view);
    // Zoom in, then move the focus three times; each tap is aimed with the device's hit.
    d.pinch();
    for k in 0..3 {
        let s = d.scene();
        let before = (s.x.from_bar, s.x.to_bar);
        let cur = d.open_capsule();
        let (id, _, hit) = lanes(&s)
            .into_iter()
            .filter(|(id, _, _)| *id != format!("story:{cur}"))
            .nth(k)
            .expect("another story in view");
        let (x, y) = center(&hit);
        assert_eq!(device_hit(&s, x, y).as_deref(), Some(id.as_str()));
        d.tap(x, y);
        let now_cap = d.open_capsule();
        assert_eq!(
            format!("story:{now_cap}"),
            id,
            "the authority moved the focus to it"
        );
        assert_eq!(d.core.look().depth, OpenDepth::Chart);
        let s2 = d.scene();
        dump(&s2, &format!("focus-{}", k + 1));
        assert_eq!(
            (s2.x.from_bar, s2.x.to_bar),
            before,
            "the bars in view stay"
        );
        selected_on_top(&s2, &now_cap);
    }
    // A tap on the open story's own lane opens its evidence (the opening rule).
    let s = d.scene();
    let cur = d.open_capsule();
    let (_, _, hit) = lanes(&s)
        .into_iter()
        .find(|(id, _, _)| *id == format!("story:{cur}"))
        .expect("own lane");
    let (x, y) = center(&hit);
    d.tap(x, y);
    assert_eq!(d.core.look().depth, OpenDepth::Evidence);
    assert_eq!(d.open_capsule(), cur);
}

#[test]
fn closed_stories_hide_and_show() {
    let mut d = Dev::new(2000, "tr");
    d.style(&json!({"story":"korda"}));
    d.open_chart();
    d.pinch();
    let s = d.scene();
    let shown = lanes(&s).len();
    let p = stories_panel(&s);
    let hdr = p
        .layers
        .iter()
        .find(|l| l.id == qp_grammar::stories::TOGGLE_LAYER)
        .expect("header");
    assert_eq!(hdr.items[0].values["shown"], 1.0);
    let closed = hdr.items[0].values["closed"];
    assert!(closed >= 1.0);
    let (x, y) = center(&hdr.items[0].hit);
    assert_eq!(
        device_hit(&s, x, y).as_deref(),
        Some(qp_grammar::stories::TOGGLE_LAYER)
    );
    d.tap(x, y);
    assert_eq!(d.core.look().closed, Some(false));
    let s = d.scene();
    dump(&s, "hidden");
    let cur = d.open_capsule();
    let left = lanes(&s);
    assert!(left.len() < shown);
    let rows = d.core.stories_json()["stories"].clone();
    for (id, _, _) in &left {
        let cap = id.trim_start_matches("story:");
        let row = rows
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == cap)
            .expect("row");
        assert!(
            row.get("closed_bar").is_none() || cap == cur,
            "only open stories and the selected one stay: {cap}"
        );
    }
    let hdr = stories_panel(&s)
        .layers
        .iter()
        .find(|l| l.id == qp_grammar::stories::TOGGLE_LAYER)
        .unwrap();
    assert_eq!(hdr.items[0].values["shown"], 0.0);
    assert_eq!(hdr.items[0].values["closed"], closed);
    let (x, y) = center(&hdr.items[0].hit);
    d.tap(x, y);
    assert_eq!(d.core.look().closed, None);
    assert_eq!(lanes(&d.scene()).len(), shown);
}

#[test]
fn zoomed_out_stories_cluster_and_a_cluster_zooms_in() {
    let mut d = Dev::new(4000, "tr");
    // A short chart frame (a phone held sideways): little room for lanes.
    d.core.set_frame("scene", 390, 200);
    d.style(&json!({"story":"korda"}));
    d.open_chart();
    // The chart frame is the one the surface reported at chart depth (a tap on the price
    // reports it); back to the capsule and down again at that size.
    d.tap(100.0, 60.0);
    d.core.back();
    d.pump();
    d.tap(195.0, 100.0);
    assert_eq!(d.core.look().depth, OpenDepth::Chart);
    assert_eq!(d.scene().height, 200);
    // Zoom out (wheel down) until the whole history is in view.
    for _ in 0..40 {
        d.core.wheel(100.0, 200.0);
        d.pump();
    }
    let s = d.scene();
    dump(&s, "clusters");
    assert_eq!(s.x.from_bar, 0, "zoomed out to the first bar");
    let p = stories_panel(&s);
    let clusters: Vec<_> = p
        .layers
        .iter()
        .filter(|l| l.id.starts_with(qp_grammar::stories::CLUSTER_LAYER))
        .collect();
    assert!(!clusters.is_empty(), "the whole range is summarized");
    // Every story in view is either a lane or counted in a cluster.
    let counted: f64 = clusters.iter().map(|l| l.items[0].values["count"]).sum();
    let rows = d.core.stories_json()["stories"].as_array().unwrap().len();
    #[allow(clippy::cast_precision_loss)]
    let total = counted + lanes(&s).len() as f64;
    assert!((total - rows as f64).abs() < 0.5, "{total} of {rows}");
    // The largest cluster: tap it, the chart zooms to its bars.
    let big = clusters
        .iter()
        .max_by(|a, b| a.items[0].values["count"].total_cmp(&b.items[0].values["count"]))
        .unwrap();
    let (from, to) = (
        big.items[0].values["from_bar"],
        big.items[0].values["to_bar"],
    );
    let (x, y) = center(&big.items[0].hit);
    assert_eq!(device_hit(&s, x, y).as_deref(), Some(big.id.as_str()));
    d.tap(x, y);
    let s2 = d.scene();
    dump(&s2, "cluster-zoomed");
    #[allow(clippy::cast_precision_loss)]
    let (a, b) = (s2.x.from_bar as f64, s2.x.to_bar as f64);
    assert!(
        a <= from && b >= to.min(s.x.to_bar as f64),
        "{a}..{b} holds {from}..{to}"
    );
    assert!(b - a + 1.0 < (s.x.to_bar - s.x.from_bar + 1) as f64);
    assert_eq!(d.core.look().depth, OpenDepth::Chart);
}

#[test]
fn english_words_and_unknown_name() {
    let mut d = Dev::new(2000, "en");
    d.style(&json!({"story":"korda"}));
    d.open_chart();
    let s = d.scene();
    let texts: Vec<String> = stories_panel(&s)
        .layers
        .iter()
        .flat_map(|l| &l.prims)
        .filter_map(|p| match p {
            qp_schema::scene::Prim::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect();
    assert!(texts.iter().any(|t| t.starts_with("live ")), "{texts:?}");
    assert!(texts.iter().any(|t| t == "hide"), "{texts:?}");
    // An unknown design is refused with the known names; the choice stays.
    d.style(&json!({"story":"nope"}));
    let err = d.core.take_error().unwrap_or_default();
    assert!(err.contains("unknown story"), "{err}");
    assert_eq!(d.core.look().story.as_deref(), Some("korda"));
}
