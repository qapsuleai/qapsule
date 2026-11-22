//! Checks measured from the compiled design fixture (`fixtures/design/desk.json`): P2 and
//! P4–P8. The fixture is a protocol transcript (it passes `qp conform`); this module reads
//! its document, its `add_study` operations and its `put_view` view, computes the studies
//! with `qp-compute` (as the authority does) and compiles the view with the grammar.

use std::collections::BTreeMap;

use qp_compute::Field;
use qp_grammar::look::{font, Look, Profile, DEFAULT_FONT, FONTS};
use qp_grammar::{compile, CompileOptions, Cursor, MemProvider};
use qp_schema::grammar::{Mark, View};
use qp_schema::protocol::Language;
use qp_schema::scene::{Align, PixelRect, Prim, SceneDesc};
use serde_json::Value;

use crate::color::{chroma_ab, de, Vision};
use crate::thresholds as th;
use crate::Row;

/// The fixture: its view and a provider with its data.
pub struct Fixture {
    view: View,
    provider: MemProvider,
}

fn read(rel: &str) -> Value {
    let p = format!("{}/../../{rel}", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{p}: {e}"));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{p}: {e}"))
}

/// Loads `fixtures/design/desk.json`.
pub fn fixture() -> Fixture {
    let t = read("fixtures/design/desk.json");
    let doc = read(t["init"]["document"].as_str().expect("init.document"));
    let s = &doc["series"][0];
    let series = qp_schema::parse_bar_series(&s.to_string()).expect("bar series");
    let bars = series.bars().expect("bars").to_vec();
    let id = s["id"].as_str().expect("series id");
    let mut p = MemProvider::new().with_bars(
        id,
        bars.clone(),
        s["instrument"]["symbol"].as_str(),
        s["interval"].as_str(),
    );
    let mut view = None;
    let ops = t["steps"].as_array().expect("steps").iter();
    for op in ops
        .filter_map(|s| s["send"]["msg"]["ops"].as_array())
        .flatten()
    {
        match op["op"].as_str() {
            Some("add_study") => {
                let name = op["study"].as_str().expect("study");
                let params: BTreeMap<String, f64> = op["params"]
                    .as_object()
                    .map(|m| m.iter().filter_map(|(k, v)| Some((k.clone(), v.as_f64()?))))
                    .into_iter()
                    .flatten()
                    .collect();
                let out = qp_compute::compute_bars(name, &params, Field::Close, &bars)
                    .expect("study")
                    .outputs;
                let fields: Vec<(&str, Vec<Option<f64>>)> =
                    out.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();
                let sid = op["id"].as_str().expect("study id");
                let label = format!(
                    "{}{}",
                    name.to_uppercase(),
                    params.get("period").copied().unwrap_or_default()
                );
                p = p
                    .with_values(sid, id, fields, name, &[name])
                    .with_label(sid, &label);
            }
            Some("put_view") => {
                view = Some(serde_json::from_value(op["view"].clone()).expect("view"))
            }
            _ => {}
        }
    }
    Fixture {
        view: view.expect("a put_view"),
        provider: p,
    }
}

impl Fixture {
    /// The view compiled in `look` and `scheme` at `w` × `h`, every text with its role,
    /// the pointer on bar [`th::CURSOR_BAR`] (RD2/C2a: the crosshair is measured).
    pub fn scene(&self, look: &'static Look, scheme: &str, w: u32, h: u32) -> SceneDesc {
        self.scene_of(&self.view, look, scheme, w, h)
    }

    fn scene_of(
        &self,
        view: &View,
        look: &'static Look,
        scheme: &str,
        w: u32,
        h: u32,
    ) -> SceneDesc {
        let mut o = CompileOptions::new(w, h);
        o.look_record = Some(look);
        o.theme = Some(scheme.to_owned());
        o.roles = true;
        o.cursor = Some(Cursor {
            bar: th::CURSOR_BAR,
            price: None,
        });
        compile(view, &self.provider, &o).expect("the design fixture compiles")
    }

    /// The view in `look` (dark) for an English reader.
    pub fn scene_en(&self, look: &'static Look, w: u32, h: u32) -> SceneDesc {
        let en = Fixture {
            view: self.view.clone(),
            provider: self.provider.clone().with_lang(Language::En),
        };
        en.scene(look, "dark", w, h)
    }

    /// The price panel alone, compiled so that its plot is `plot_h` high.
    pub fn price_only(&self, look: &'static Look, scheme: &str, w: u32, plot_h: u32) -> SceneDesc {
        let mut v = self.view.clone();
        let first = v.panels[0].id.unwrap_or_default();
        v.panels.truncate(1);
        v.layers.retain(|l| l.panel == first);
        let pad = look.space.plot[2] + look.space.plot[3];
        self.scene_of(&v, look, scheme, w, plot_h + pad.unsigned_abs())
    }
}

type Box4 = [f64; 4];

fn prims(s: &SceneDesc) -> impl Iterator<Item = &Prim> {
    s.under.iter().chain(&s.over).chain(
        s.panels
            .iter()
            .flat_map(|p| p.layers.iter().flat_map(|l| &l.prims)),
    )
}

fn text_box(p: &Prim) -> Option<Box4> {
    let Prim::Text {
        x,
        baseline,
        px,
        text,
        align,
        font: f,
        ..
    } = p
    else {
        return None;
    };
    let f = font(f.as_deref().unwrap_or(DEFAULT_FONT)).unwrap_or(&FONTS[0]);
    let px = f64::from(*px);
    let w = f64::from(f.width(text, px));
    let x = f64::from(*x);
    let l = if *align == Align::Right { x - w } else { x };
    let b = f64::from(*baseline);
    Some([l, b - f.ascent * px, l + w, b + 0.2 * px])
}

fn rect_box(r: &PixelRect) -> Box4 {
    [r.left, r.top, r.right, r.bottom].map(f64::from)
}

fn overlaps(a: &Box4, b: &Box4) -> bool {
    a[0] < b[2] && b[0] < a[2] && a[1] < b[3] && b[1] < a[3]
}

fn role_is(p: &Prim, want: &str) -> bool {
    matches!(p, Prim::Text { role: Some(r), .. } if r == want)
}

/// Price-axis labels (figures right of a panel) and time labels (below the last panel).
fn axis_labels(s: &SceneDesc) -> Vec<&Prim> {
    let bottom = s.panels.iter().map(|p| p.rect.bottom).max().unwrap_or(0);
    s.under
        .iter()
        .filter(|p| match p {
            Prim::Text { x, baseline, .. } => {
                (role_is(p, "figure") && s.panels.iter().any(|q| *x > q.rect.right))
                    || (role_is(p, "label") && *baseline > bottom)
            }
            _ => false,
        })
        .collect()
}

fn price_labels(s: &SceneDesc) -> Vec<i32> {
    let r = s.panels[0].rect;
    let mut v: Vec<i32> = s
        .under
        .iter()
        .filter_map(|p| match p {
            Prim::Text { x, baseline, .. }
                if role_is(p, "figure") && *x > r.right && *baseline <= r.bottom + 8 =>
            {
                Some(*baseline)
            }
            _ => None,
        })
        .collect();
    v.sort_unstable();
    v
}

/// Time labels: their count and their distinct colours (two levels: two colour roles).
fn time_labels(s: &SceneDesc) -> (usize, usize) {
    let bottom = s.panels.iter().map(|p| p.rect.bottom).max().unwrap_or(0);
    let mut colors = Vec::new();
    let mut n = 0;
    for p in s.under.iter().filter(|p| role_is(p, "label")) {
        if let Prim::Text {
            baseline, color, ..
        } = p
        {
            if *baseline > bottom {
                n += 1;
                if !colors.contains(color) {
                    colors.push(*color);
                }
            }
        }
    }
    (n, colors.len())
}

/// P2: distinct tones of the drawn colours (alpha ignored: a tone is a colour role).
pub fn p2(s: &SceneDesc, scheme: &'static str) -> Row {
    let bg = s.background;
    let mut neutral = false;
    let mut tones: Vec<[u8; 4]> = Vec::new();
    for p in prims(s) {
        let c = match p {
            Prim::Rect { color, .. }
            | Prim::Polyline { color, .. }
            | Prim::Fill { color, .. }
            | Prim::Circle { color, .. }
            | Prim::Text { color, .. } => *color,
        };
        // A fully transparent primitive draws nothing; otherwise the tone is the colour
        // role, whatever its alpha.
        if c[3] == 0 || c[..3] == bg[..3] {
            continue;
        }
        let c = [c[0], c[1], c[2], 255];
        if chroma_ab(c) < th::P2_NEUTRAL_C {
            neutral = true;
        } else if !tones
            .iter()
            .any(|t| de(*t, c, Vision::Normal) <= th::P2_TONE_DE)
        {
            tones.push(c);
        }
    }
    let n = tones.len() + usize::from(neutral);
    Row::check(
        "P2",
        scheme,
        "distinct tones in the chart scene",
        format!("{n} ({} hues + neutral)", tones.len()),
        format!("<= {}", th::P2_TONES_MAX),
        n <= th::P2_TONES_MAX,
    )
}

/// P4: price-axis label spacing (desk scene) and count in a 300 px-high plot.
pub fn p4(desk: &SceneDesc, plot300: &SceneDesc, scheme: &'static str) -> Row {
    let b = price_labels(desk);
    let mut gaps: Vec<i32> = b.windows(2).map(|w| w[1] - w[0]).collect();
    gaps.sort_unstable();
    let spacing = gaps.get(gaps.len() / 2).map_or(0.0, |g| f64::from(*g));
    let n = price_labels(plot300).len();
    let (lo, hi) = th::P4_SPACING;
    Row::check(
        "P4",
        scheme,
        "price label spacing px / labels in 300 px plot",
        format!("{spacing} / {n}"),
        format!("{lo}-{hi} / >= {}", th::P4_MIN_LABELS),
        (lo..=hi).contains(&spacing) && n >= th::P4_MIN_LABELS,
    )
}

/// P5: a two-level time axis (minor and major labels in two colour roles, drawn) with
/// enough labels at phone width.
pub fn p5(phone: &SceneDesc, look: &Look, scheme: &'static str) -> Row {
    let (n, levels) = time_labels(phone);
    let two = look.chart.two_level_time && levels >= 2;
    Row::check(
        "P5",
        scheme,
        "two-level time axis / labels at 390 px",
        format!("{} / {n}", if two { "yes" } else { "no" }),
        format!("yes / >= {}", th::P5_MIN_LABELS),
        two && n >= th::P5_MIN_LABELS,
    )
}

/// Axis-label overlaps in one scene: with a plot, with another text, with a thin
/// vertical line (a selection or crosshair line; the axis lines themselves excluded), or
/// past the frame's right edge.
fn overlap_count(s: &SceneDesc) -> usize {
    let labels: Vec<Box4> = axis_labels(s).into_iter().filter_map(text_box).collect();
    let texts: Vec<Box4> = prims(s).filter_map(text_box).collect();
    let lines: Vec<Box4> = prims(s)
        .filter_map(|p| match p {
            Prim::Rect { x, y, w, h, .. }
                if *w <= 2 && *h >= 8 && !s.panels.iter().any(|q| q.rect.right == *x) =>
            {
                Some([
                    f64::from(*x),
                    f64::from(*y),
                    f64::from(*x) + f64::from(*w),
                    f64::from(*y) + f64::from(*h),
                ])
            }
            _ => None,
        })
        .collect();
    let mut n = 0;
    for l in &labels {
        n += s
            .panels
            .iter()
            .filter(|p| overlaps(l, &rect_box(&p.rect)))
            .count();
        // A label meets its own box once among the texts.
        n += texts
            .iter()
            .filter(|t| overlaps(l, t))
            .count()
            .saturating_sub(1);
        n += lines.iter().filter(|v| overlaps(l, v)).count();
        // A label past the frame's right edge has left its gutter.
        n += usize::from(l[2] > f64::from(s.width));
    }
    n
}

/// P6: axis-label overlaps in the desk and the phone scene.
#[allow(clippy::absurd_extreme_comparisons)] // the threshold is 0 today; it may change once
pub fn p6(desk: &SceneDesc, phone: &SceneDesc, scheme: &'static str) -> Row {
    let (a, b) = (overlap_count(desk), overlap_count(phone));
    Row::check(
        "P6",
        scheme,
        "axis-label overlaps + overflow (desk + phone)",
        format!("{} ({a} + {b})", a + b),
        format!("= {}", th::P6_OVERLAPS_MAX),
        a + b <= th::P6_OVERLAPS_MAX,
    )
}

/// P7: the familiar chart marks are present.
pub fn p7(s: &SceneDesc, look: &Look, scheme: &'static str) -> Row {
    let r = s.panels[0].rect;
    let tag = s.over.iter().find_map(|p| match p {
        Prim::Text { x, baseline, .. }
            if role_is(p, "figure")
                && *x > r.right
                && (r.top..=r.bottom + 12).contains(baseline) =>
        {
            Some(baseline - 4)
        }
        _ => None,
    });
    let half = (r.right - r.left) / 2;
    let line = tag.is_some_and(|ty| {
        prims(s)
            .filter(|p| match p {
                Prim::Rect { x, y, w, h, .. } => {
                    *h <= 2
                        && i32::try_from(*w).unwrap_or(i32::MAX) < half
                        && *x >= r.left
                        && *x < r.right
                        && (y - ty).abs() <= 1
                }
                Prim::Polyline { points, .. } => {
                    points
                        .iter()
                        .all(|q| (f64::from(q[1]) - f64::from(ty)).abs() <= 1.0)
                        && points.len() >= 2
                }
                _ => false,
            })
            .count()
            >= 3
    });
    let legend: Vec<&str> = s
        .over
        .iter()
        .filter_map(|p| match p {
            Prim::Text { baseline, text, .. } if *baseline < r.top => Some(text.as_str()),
            _ => None,
        })
        .flat_map(str::split_whitespace)
        .collect();
    let has = |k: &str| legend.contains(&k);
    let ohlc = ["O", "H", "L", "C"].iter().all(|k| has(k));
    let legend_ok = ohlc || (look.profile == Profile::Touch && has("C"));
    let cross = crosshair(s);
    let yn = |b: bool| if b { "Y" } else { "N" };
    Row::check(
        "P7",
        scheme,
        "last-value tag / value line / crosshair+pills / OHLC legend",
        format!(
            "{} / {} / {} / {}",
            yn(tag.is_some()),
            yn(line),
            yn(cross),
            if ohlc {
                "OHLC"
            } else if has("C") {
                "C only"
            } else {
                "none"
            }
        ),
        "Y / Y / Y / OHLC (touch: C)".to_owned(),
        tag.is_some() && line && cross && legend_ok,
    )
}

/// The crosshair as drawn: a run of short vertical dashes, a price pill (a second figure
/// in the first panel's gutter beside the tag) and a date pill under the time axis.
fn crosshair(s: &SceneDesc) -> bool {
    let r = s.panels[0].rect;
    let bottom = s.panels.iter().map(|p| p.rect.bottom).max().unwrap_or(0);
    let mut dashes: BTreeMap<i32, usize> = BTreeMap::new();
    for p in &s.over {
        if let Prim::Rect { x, y, w: 1, h, .. } = p {
            if *h <= 4 && *x > r.left && *x < r.right && *y >= r.top {
                *dashes.entry(*x).or_default() += 1;
            }
        }
    }
    let figures = s
        .over
        .iter()
        .filter(|p| matches!(p, Prim::Text { x, baseline, .. } if role_is(p, "figure") && *x > r.right && *baseline > r.top && *baseline <= r.bottom + 12))
        .count();
    let date = s.over.iter().any(
        |p| matches!(p, Prim::Text { baseline, .. } if role_is(p, "label") && *baseline > bottom),
    );
    dashes.values().any(|n| *n >= 3) && figures >= 2 && date
}

/// P8: on-plot semantic marks (the look's cap and any glyph drawn in the scene).
pub fn p8(s: &SceneDesc, look: &Look, scheme: &'static str) -> Row {
    let drawn = s
        .panels
        .iter()
        .flat_map(|p| &p.layers)
        .filter(|l| l.mark == Mark::Glyph)
        .flat_map(|l| &l.items)
        .map(|i| f64::from(i.hit.bottom - i.hit.top))
        .fold(0.0, f64::max);
    let cap = look.chart.mark_max_px;
    Row::check(
        "P8",
        scheme,
        "on-plot mark height px (record / drawn)",
        format!("{cap} / {drawn}"),
        format!("<= {}", th::P8_MARK_MAX),
        cap <= th::P8_MARK_MAX && drawn <= th::P8_MARK_MAX,
    )
}
