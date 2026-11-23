//! (RF1/Ö, RD2 audit K1) P8's drawn half: the meaning glyph on the plot, measured on the
//! separate design fixture `fixtures/design/desk-marks.json` (the desk fixture draws no
//! glyph, and it and its goldens stay as they are). The fixture is a protocol transcript
//! (it passes `qp conform`); this module reads its document (bars, facts, findings,
//! capsules with their stories) and its `put_view` view `marks`, and compiles the view.
//! The glyph's spec comes from the capsule's story, as in the authority's provider.

use qp_grammar::{compile, CompileOptions, MemProvider};
use qp_look::look::Look;
use qp_schema::grammar::{Mark, View};
use qp_schema::scene::{Prim, SceneDesc};

use crate::scene::read;
use crate::thresholds as th;

/// The marks fixture: its view and a provider with its bars, findings and capsules.
pub struct Marks {
    view: View,
    provider: MemProvider,
}

/// Loads `fixtures/design/desk-marks.json`.
pub fn fixture() -> Marks {
    let t = read("fixtures/design/desk-marks.json");
    let doc = read(t["init"]["document"].as_str().expect("init.document"));
    let s = &doc["series"][0];
    let series = qp_schema::parse_bar_series(&s.to_string()).expect("bar series");
    let mut p = MemProvider::new().with_bars(
        s["id"].as_str().expect("series id"),
        series.bars().expect("bars").to_vec(),
        s["instrument"]["symbol"].as_str(),
        s["interval"].as_str(),
    );
    for f in doc["facts"].as_array().into_iter().flatten() {
        p = p.with_fact(serde_json::from_value(f.clone()).expect("fact"));
    }
    for f in doc["findings"].as_array().into_iter().flatten() {
        p = p.with_finding(serde_json::from_value(f.clone()).expect("finding"));
    }
    for c in doc["capsules"].as_array().into_iter().flatten() {
        p = p.with_capsule(serde_json::from_value(c.clone()).expect("capsule"));
    }
    let view = t["steps"]
        .as_array()
        .expect("steps")
        .iter()
        .filter_map(|s| s["send"]["msg"]["ops"].as_array())
        .flatten()
        .filter(|op| op["op"] == "put_view" && op["view"]["id"] == "marks")
        .map(|op| serde_json::from_value::<View>(op["view"].clone()).expect("view"))
        .next_back()
        .expect("view marks");
    Marks { view, provider: p }
}

impl Marks {
    /// The view compiled in `look` and `scheme` at the desktop chart size.
    pub fn scene(&self, look: &'static Look, scheme: &str) -> SceneDesc {
        let (w, h) = th::DESK;
        let mut o = CompileOptions::without_designs(w, h);
        o.look_record = Some(look);
        o.theme = Some(scheme.to_owned());
        compile(&self.view, &self.provider, &o).expect("the marks fixture compiles")
    }
}

/// The glyphs drawn on the plot: (count, greatest height in px). The height is the
/// vertical extent of each glyph layer's drawn primitives (and of its items' hit boxes).
pub fn drawn(s: &SceneDesc) -> (usize, f64) {
    let mut n = 0;
    let mut tallest: f64 = 0.0;
    for l in s
        .panels
        .iter()
        .flat_map(|p| &p.layers)
        .filter(|l| l.mark == Mark::Glyph)
    {
        n += l.items.len();
        for i in &l.items {
            tallest = tallest.max(f64::from(i.hit.bottom - i.hit.top));
        }
        let ys = l.prims.iter().flat_map(prim_ys);
        let (lo, hi) = ys.fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), y| {
            (lo.min(y), hi.max(y))
        });
        if hi > lo {
            tallest = tallest.max(hi - lo);
        }
    }
    (n, tallest)
}

fn prim_ys(p: &Prim) -> Vec<f64> {
    match p {
        Prim::Rect { y, h, .. } => vec![f64::from(*y), f64::from(*y) + f64::from(*h)],
        Prim::Polyline { points, .. } | Prim::Fill { points, .. } => {
            points.iter().map(|q| f64::from(q[1])).collect()
        }
        Prim::Circle { cy, r, .. } => vec![f64::from(cy - r), f64::from(cy + r)],
        // A glyph's text (if any) sits inside its box; the box bounds it.
        Prim::Text { .. } => Vec::new(),
    }
}
