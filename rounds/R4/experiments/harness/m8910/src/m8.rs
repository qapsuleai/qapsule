//! M8 — the capsule's closed face (SPEC §2).
//!
//! - [`faces`]: every found `S8` state's face scene by a face design, from the web shell's
//!   own function (`qp_wasm::shell::face_scene`) over the shell core's replica at the
//!   state's replay moment; TR and EN sentence, light and dark theme;
//! - [`computed`]: SPEC §2(b) measures from the scenes (`SceneDesc`) and the hard
//!   thresholds;
//! - [`sheets`]: the two persona pages (6 faces each, labels A–F as layout elements), their
//!   screenshots (light, dark, grey), hidden keys and persona prompts;
//! - [`critics`]: the critics' directories (every design's pages, per critic different
//!   random letters and order).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use qp_schema::protocol::{CapsuleItem, ViewState};
use qp_schema::scene::DataSource;
use qp_schema::scene::{Prim, SceneDesc};
use qp_schema::Mark;

use crate::blind;
use crate::color::contrast;
use crate::io;
use crate::prompts;
use crate::rng::Rng;
use crate::s8::{self, S8};
use crate::shots;
use crate::world::{Core, Kind, World};

/// Faces per page.
pub const PER_PAGE: usize = 6;
/// Seed of the page order (fixed: every design shows the same states on the same page in
/// the same place).
pub const ORDER_SEED: u64 = 1;
/// Page labels, top to bottom.
pub const LABELS: [&str; PER_PAGE] = ["A", "B", "C", "D", "E", "F"];
/// Face width (the phone).
pub const FACE_W: u32 = 390;
/// Themes measured.
pub const THEMES: [&str; 2] = ["light", "dark"];
/// Languages measured.
pub const LANGS: [&str; 2] = ["tr", "en"];

/// Share of the glyph box a primitive must cover to count as the glyph's identifying part.
pub const GLYPH_PART: f64 = 0.02;

/// Hard thresholds (SPEC §2(b)).
pub mod limits {
    /// Text contrast (normal text).
    pub const TEXT: f64 = 4.5;
    /// Text contrast at 18 px and above.
    pub const TEXT_LARGE: f64 = 3.0;
    /// Large text from this size (CSS px).
    pub const LARGE_PX: f64 = 18.0;
    /// Glyph (non-text) contrast.
    pub const GLYPH: f64 = 3.0;
    /// Smallest text (CSS px).
    pub const MIN_PX: f64 = 11.0;
    /// Longest headline (characters, TR and EN).
    pub const HEADLINE_CHARS: usize = 80;
    /// Most additional information units.
    pub const EXTRA: usize = 2;
}

/// One rendered face.
#[derive(Debug, Clone)]
pub struct Face {
    /// S8 state.
    pub state: usize,
    /// Language.
    pub lang: &'static str,
    /// Theme.
    pub theme: &'static str,
    /// The scene (`None`: the reader gets no face scene — the shell shows the sentence).
    pub scene: Option<SceneDesc>,
    /// The sentence the shell shows without a scene.
    pub text: String,
}

fn look(face: &str, theme: &str, alphabet: Option<&str>) -> ViewState {
    ViewState {
        face: (face != qp_grammar::faces::DEFAULT_FACE).then(|| face.to_owned()),
        theme: Some(theme.to_owned()),
        alphabet: alphabet.map(str::to_owned),
        ..ViewState::default()
    }
}

/// The faces of every found state by design `face` (module docs).
///
/// # Errors
/// Unknown face, unreadable `S8`, a world that does not start.
pub fn faces(face: &str, alphabet: Option<&str>, s8: &S8) -> Result<Vec<Face>, String> {
    let fd = qp_grammar::faces::by_name(Some(face)).map_err(|e| e.reason.clone())?;
    let mut out = Vec::new();
    for kind in [Kind::Demo, Kind::Hourly] {
        let mut sts: Vec<&s8::State> = s8
            .states
            .iter()
            .filter(|s| s.found && Kind::from_name(&s.world) == Some(kind))
            .collect();
        if sts.is_empty() {
            continue;
        }
        sts.sort_by_key(|s| (s.step, s.state));
        let mut w = World::new(kind)?;
        let mut c = Core::connect(&mut w, "m8", crate::READER, "tr")?;
        for st in sts {
            if w.step() < st.step {
                c.seek(&mut w, st.step);
            }
            let item: CapsuleItem = serde_json::from_value(st.item.clone())
                .map_err(|e| format!("S8 #{}: {e}", st.state))?;
            for lang in LANGS {
                let mut it = item.clone();
                it.sentence.text = if lang == "tr" {
                    item.sentence.tr.clone()
                } else {
                    item.sentence.en.clone()
                };
                for theme in THEMES {
                    let scene = if st.principal == crate::VISITOR {
                        // The unentitled reader's replica has no bars: the shell shows the
                        // sentence as text (`feed_json` `scene: false`).
                        None
                    } else {
                        let mp = c.core.max_points();
                        let lk = look(fd.name, theme, alphabet);
                        c.core.replica_mut().session_mut().and_then(|s| {
                            qp_wasm::shell::face_scene(s, &it, &lk, mp, FACE_W, fd.height)
                        })
                    };
                    out.push(Face {
                        state: st.state,
                        lang,
                        theme,
                        scene,
                        text: it.sentence.text.clone(),
                    });
                }
            }
        }
    }
    out.sort_by_key(|f| (f.state, f.lang, f.theme));
    Ok(out)
}

// ---------------------------------------------------------------------------
// (b) computed measures
// ---------------------------------------------------------------------------

/// Measures of one face (one state, language, theme).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Measures {
    /// S8 state.
    pub state: usize,
    /// Language.
    pub lang: String,
    /// Theme.
    pub theme: String,
    /// Whether the face has a scene (else the shell shows the sentence as text).
    pub scene: bool,
    /// The headline as drawn (text items of the finding's `headline` layer, joined).
    pub headline: String,
    /// Headline characters.
    pub headline_chars: usize,
    /// Headline words.
    pub headline_words: usize,
    /// Text items in the face.
    pub text_items: usize,
    /// Text box area / face area.
    pub text_area_share: f64,
    /// Additional information units (README: text layers besides the headline + panels
    /// with non-text, non-glyph marks).
    pub extra_units: usize,
    /// Glyph box area (px²).
    pub glyph_area: f64,
    /// Glyph area / face area.
    pub glyph_share: f64,
    /// Lowest text contrast (WCAG) and its required minimum.
    pub text_contrast_min: f64,
    /// Whether every text item meets its minimum (4.5, or 3 at ≥ 18 px).
    pub text_contrast_ok: bool,
    /// Contrast of the glyph's identifying part (highest-contrast primitive covering at
    /// least [`GLYPH_PART`] of the glyph box).
    pub glyph_contrast: f64,
    /// Lowest contrast of any glyph primitive (information).
    pub glyph_contrast_min: f64,
    /// Smallest text size (CSS px).
    pub min_text_px: f64,
    /// Face height (CSS px).
    pub height: u32,
}

fn prim_box(p: &Prim) -> Option<[f64; 4]> {
    let pts = |v: &[[f32; 2]]| {
        let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
        for [x, y] in v {
            x0 = x0.min(f64::from(*x));
            y0 = y0.min(f64::from(*y));
            x1 = x1.max(f64::from(*x));
            y1 = y1.max(f64::from(*y));
        }
        (x1 >= x0).then_some([x0, y0, x1, y1])
    };
    match p {
        Prim::Rect { x, y, w, h, .. } => Some([
            f64::from(*x),
            f64::from(*y),
            f64::from(*x) + f64::from(*w),
            f64::from(*y) + f64::from(*h),
        ]),
        Prim::Polyline { points, .. } | Prim::Fill { points, .. } => pts(points),
        Prim::Circle { cx, cy, r, .. } => Some([
            f64::from(cx - r),
            f64::from(cy - r),
            f64::from(cx + r),
            f64::from(cy + r),
        ]),
        Prim::Text { .. } => None,
    }
}

/// Ink area of a primitive (px²): rect and circle areas, polygon area (shoelace), stroke
/// length × width.
fn ink(p: &Prim) -> f64 {
    match p {
        Prim::Rect { w, h, .. } => f64::from(*w) * f64::from(*h),
        Prim::Circle { r, .. } => std::f64::consts::PI * f64::from(*r) * f64::from(*r),
        Prim::Fill { points, .. } => {
            let n = points.len();
            let mut a = 0.0;
            for i in 0..n {
                let (p, q) = (points[i], points[(i + 1) % n]);
                a += f64::from(p[0]) * f64::from(q[1]) - f64::from(q[0]) * f64::from(p[1]);
            }
            (a / 2.0).abs()
        }
        Prim::Polyline { points, width, .. } => {
            let len: f64 = points
                .windows(2)
                .map(|w| {
                    let (dx, dy) = (f64::from(w[1][0] - w[0][0]), f64::from(w[1][1] - w[0][1]));
                    (dx * dx + dy * dy).sqrt()
                })
                .sum();
            len * f64::from(*width)
        }
        Prim::Text { .. } => 0.0,
    }
}

fn color_of(p: &Prim) -> [u8; 4] {
    match p {
        Prim::Rect { color, .. }
        | Prim::Polyline { color, .. }
        | Prim::Fill { color, .. }
        | Prim::Circle { color, .. }
        | Prim::Text { color, .. } => *color,
    }
}

/// Advance width of `text` at `px` in the renderer's embedded font.
fn text_width(text: &str, px: f32) -> f64 {
    qp_render::cpu::font().map_or(0.0, |f| {
        text.chars()
            .map(|c| f64::from(f.metrics(c, px).advance_width))
            .sum()
    })
}

fn text_box(p: &Prim) -> Option<[f64; 4]> {
    if let Prim::Text {
        x,
        baseline,
        px,
        text,
        align,
        ..
    } = p
    {
        let w = text_width(text, *px);
        let x0 = match align {
            qp_schema::scene::Align::Left => f64::from(*x),
            qp_schema::scene::Align::Right => f64::from(*x) - w,
        };
        let top = f64::from(*baseline) - f64::from(*px) * 0.8;
        Some([x0, top, x0 + w, top + f64::from(*px)])
    } else {
        None
    }
}

/// The opaque colour under the centre of `b` among the primitives painted before index
/// `upto` (a filled rect/shape whose box contains the point), else the scene background.
fn under(prims: &[&Prim], upto: usize, b: [f64; 4], bg: [u8; 4]) -> [u8; 4] {
    let (cx, cy) = ((b[0] + b[2]) / 2.0, (b[1] + b[3]) / 2.0);
    let mut c = bg;
    for p in &prims[..upto] {
        if matches!(
            p,
            Prim::Rect { .. } | Prim::Fill { .. } | Prim::Circle { .. }
        ) {
            if let Some(r) = prim_box(p) {
                if cx >= r[0] && cx <= r[2] && cy >= r[1] && cy <= r[3] {
                    c = crate::color::over(color_of(p), c);
                }
            }
        }
    }
    c
}

/// The measures of one face.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn measure(f: &Face, height: u32) -> Measures {
    let Some(sc) = &f.scene else {
        let chars = f.text.chars().count();
        return Measures {
            state: f.state,
            lang: f.lang.to_owned(),
            theme: f.theme.to_owned(),
            scene: false,
            headline: f.text.clone(),
            headline_chars: chars,
            headline_words: f.text.split_whitespace().count(),
            text_items: 1,
            text_area_share: 0.0,
            extra_units: 0,
            glyph_area: 0.0,
            glyph_share: 0.0,
            text_contrast_min: 0.0,
            text_contrast_ok: true,
            glyph_contrast: 0.0,
            glyph_contrast_min: 0.0,
            min_text_px: 0.0,
            height,
        };
    };
    let area = f64::from(sc.width) * f64::from(sc.height);
    let prims: Vec<&Prim> = sc.prims().collect();
    // Headline layer(s): the finding's text layers drawing `headline`.
    let mut headline: Vec<String> = Vec::new();
    let mut extra = 0usize;
    let mut glyph_box: Option<[f64; 4]> = None;
    let mut glyph_prims: Vec<&Prim> = Vec::new();
    let mut headline_layers = 0usize;
    for p in &sc.panels {
        let mut panel_graphic = false;
        for l in &p.layers {
            let is_head = matches!(l.data, DataSource::Finding { .. })
                && l.mark == Mark::Text
                && l.fields.iter().any(|x| x == "headline");
            if l.mark == Mark::Glyph {
                for it in &l.items {
                    let r = [
                        f64::from(it.hit.left),
                        f64::from(it.hit.top),
                        f64::from(it.hit.right),
                        f64::from(it.hit.bottom),
                    ];
                    glyph_box = Some(glyph_box.map_or(r, |g| {
                        [
                            g[0].min(r[0]),
                            g[1].min(r[1]),
                            g[2].max(r[2]),
                            g[3].max(r[3]),
                        ]
                    }));
                }
                glyph_prims.extend(l.prims.iter());
                continue;
            }
            if l.prims.is_empty() {
                continue;
            }
            if is_head {
                headline_layers += 1;
                for pr in &l.prims {
                    if let Prim::Text { text, .. } = pr {
                        headline.push(text.clone());
                    }
                }
            } else if l.mark == Mark::Text {
                extra += 1;
            } else {
                panel_graphic = true;
            }
        }
        if panel_graphic {
            extra += 1;
        }
    }
    let _ = headline_layers;
    let headline = headline.join(" ");
    let mut text_items = 0usize;
    let mut text_area = 0.0;
    let mut min_px = f64::MAX;
    let mut tc_min = f64::MAX;
    let mut tc_ok = true;
    for (i, p) in prims.iter().enumerate() {
        if let Prim::Text { px, .. } = p {
            text_items += 1;
            let b = text_box(p).unwrap_or([0.0; 4]);
            text_area += (b[2] - b[0]).max(0.0) * (b[3] - b[1]).max(0.0);
            let pxf = f64::from(*px);
            min_px = min_px.min(pxf);
            let c = contrast(color_of(p), under(&prims, i, b, sc.background));
            tc_min = tc_min.min(c);
            let need = if pxf >= limits::LARGE_PX {
                limits::TEXT_LARGE
            } else {
                limits::TEXT
            };
            if c < need {
                tc_ok = false;
            }
        }
    }
    let gb = glyph_box.unwrap_or([0.0; 4]);
    let glyph_area = (gb[2] - gb[0]).max(0.0) * (gb[3] - gb[1]).max(0.0);
    // The glyph's identifying part: the highest-contrast primitive whose ink covers at
    // least `GLYPH_PART` of the glyph box (a speck does not count), each against what is
    // under its centre (an earlier primitive of the glyph, e.g. a plate, else the face's
    // background).
    let mut dom: Option<(f64, f64)> = None; // (ink, contrast)
    let mut gmin = f64::MAX;
    for (i, p) in glyph_prims.iter().enumerate() {
        let col = color_of(p);
        if col[3] == 0 {
            continue;
        }
        let bx = prim_box(p).unwrap_or([0.0; 4]);
        let c = contrast(col, under(&glyph_prims, i, bx, sc.background));
        gmin = gmin.min(c);
        let a = ink(p);
        if a >= GLYPH_PART * glyph_area && dom.is_none_or(|(_, dc)| c > dc) {
            dom = Some((a, c));
        }
    }
    Measures {
        state: f.state,
        lang: f.lang.to_owned(),
        theme: f.theme.to_owned(),
        scene: true,
        headline_chars: headline.chars().count(),
        headline_words: headline.split_whitespace().count(),
        headline,
        text_items,
        text_area_share: io::round(text_area / area, 4),
        extra_units: extra,
        glyph_area: io::round(glyph_area, 1),
        glyph_share: io::round(glyph_area / area, 4),
        text_contrast_min: io::round(if tc_min == f64::MAX { 0.0 } else { tc_min }, 2),
        text_contrast_ok: tc_ok,
        glyph_contrast: io::round(dom.map_or(0.0, |d| d.1), 2),
        glyph_contrast_min: io::round(if gmin == f64::MAX { 0.0 } else { gmin }, 2),
        min_text_px: if min_px == f64::MAX { 0.0 } else { min_px },
        height,
    }
}

/// Summary of a design's computed measures and its hard thresholds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Computed {
    /// Harness version.
    pub harness: String,
    /// Face design.
    pub face: String,
    /// Face height (CSS px).
    pub height: u32,
    /// Per face.
    pub faces: Vec<Measures>,
    /// Headline characters: max and mean, per language.
    pub headline_chars: BTreeMap<String, [f64; 2]>,
    /// Headline words: max and mean, per language.
    pub headline_words: BTreeMap<String, [f64; 2]>,
    /// Hard threshold results (name → pass) and the worst values.
    pub thresholds: BTreeMap<String, Threshold>,
    /// All hard thresholds pass.
    pub pass: bool,
}

/// One hard threshold.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Threshold {
    /// The limit.
    pub limit: f64,
    /// The worst value over the faces.
    pub worst: f64,
    /// Pass.
    pub pass: bool,
}

/// The computed measures of design `face` over `faces`.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn computed(face: &str, faces: &[Face]) -> Computed {
    let height = qp_grammar::faces::height(Some(face));
    let ms: Vec<Measures> = faces.iter().map(|f| measure(f, height)).collect();
    let mut hc = BTreeMap::new();
    let mut hw = BTreeMap::new();
    for lang in LANGS {
        let v: Vec<&Measures> = ms
            .iter()
            .filter(|m| m.lang == lang && m.theme == "light")
            .collect();
        let n = v.len().max(1) as f64;
        let mc = v.iter().map(|m| m.headline_chars).max().unwrap_or(0) as f64;
        let ac = v.iter().map(|m| m.headline_chars as f64).sum::<f64>() / n;
        let mw = v.iter().map(|m| m.headline_words).max().unwrap_or(0) as f64;
        let aw = v.iter().map(|m| m.headline_words as f64).sum::<f64>() / n;
        hc.insert(lang.to_owned(), [mc, io::round(ac, 2)]);
        hw.insert(lang.to_owned(), [mw, io::round(aw, 2)]);
    }
    let sc: Vec<&Measures> = ms.iter().filter(|m| m.scene).collect();
    let mut th = BTreeMap::new();
    let worst_min = |f: &dyn Fn(&Measures) -> f64| sc.iter().map(|m| f(m)).fold(f64::MAX, f64::min);
    let tc = worst_min(&|m| m.text_contrast_min);
    th.insert(
        "text_contrast".to_owned(),
        Threshold {
            limit: limits::TEXT,
            worst: io::round(if tc == f64::MAX { 0.0 } else { tc }, 2),
            pass: sc.iter().all(|m| m.text_contrast_ok),
        },
    );
    let gc = worst_min(&|m| m.glyph_contrast);
    th.insert(
        "glyph_contrast".to_owned(),
        Threshold {
            limit: limits::GLYPH,
            worst: io::round(if gc == f64::MAX { 0.0 } else { gc }, 2),
            pass: sc.iter().all(|m| m.glyph_contrast >= limits::GLYPH),
        },
    );
    let mp = worst_min(&|m| m.min_text_px);
    th.insert(
        "min_text_px".to_owned(),
        Threshold {
            limit: limits::MIN_PX,
            worst: if mp == f64::MAX { 0.0 } else { mp },
            pass: sc.iter().all(|m| m.min_text_px >= limits::MIN_PX),
        },
    );
    let hmax = ms.iter().map(|m| m.headline_chars).max().unwrap_or(0);
    th.insert(
        "headline_chars".to_owned(),
        Threshold {
            limit: limits::HEADLINE_CHARS as f64,
            worst: hmax as f64,
            pass: hmax <= limits::HEADLINE_CHARS,
        },
    );
    let emax = ms.iter().map(|m| m.extra_units).max().unwrap_or(0);
    th.insert(
        "extra_units".to_owned(),
        Threshold {
            limit: limits::EXTRA as f64,
            worst: emax as f64,
            pass: emax <= limits::EXTRA,
        },
    );
    let pass = th.values().all(|t| t.pass);
    Computed {
        harness: crate::HARNESS_VERSION.to_owned(),
        face: face.to_owned(),
        height,
        faces: ms,
        headline_chars: hc,
        headline_words: hw,
        thresholds: th,
        pass,
    }
}

// ---------------------------------------------------------------------------
// sheets
// ---------------------------------------------------------------------------

/// The fixed page order: found states shuffled with [`ORDER_SEED`], page 1 the first
/// [`PER_PAGE`], page 2 the rest.
#[must_use]
pub fn pages(s8: &S8) -> Vec<Vec<usize>> {
    let mut v: Vec<usize> = s8
        .states
        .iter()
        .filter(|s| s.found)
        .map(|s| s.state)
        .collect();
    Rng::derive(ORDER_SEED, "m8-pages").shuffle(&mut v);
    v.chunks(PER_PAGE).map(<[usize]>::to_vec).collect()
}

/// The code of design `face`'s page `page`.
#[must_use]
pub fn page_code(face: &str, page: usize, salt: u64) -> String {
    blind::code(salt, &format!("m8|{face}|{page}"))
}

/// One page's hidden key.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PageKey {
    /// Experiment.
    pub experiment: String,
    /// Face design.
    pub face: String,
    /// Page (1, 2).
    pub page: usize,
    /// Label → (state, key).
    pub labels: BTreeMap<String, LabelKey>,
}

/// One label's state and key.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LabelKey {
    /// S8 state.
    pub state: usize,
    /// Answer key.
    pub key: s8::Key,
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// The sheet page: the demo's feed chrome (`shells/web/style.css`, the same markup) with
/// the faces stacked and the labels as layout elements.
fn sheet_html(
    theme: &str,
    gray: bool,
    rows: &[(String, Option<PathBuf>, String)],
    height: u32,
) -> String {
    let css = io::repo_root().join("shells/web/style.css");
    let mut s = String::new();
    s.push_str("<!doctype html>\n<html lang=\"tr\"><head><meta charset=\"utf-8\">\n");
    s.push_str(&format!(
        "<link rel=\"stylesheet\" href=\"file://{}\">\n",
        css.canonicalize().unwrap_or(css).display()
    ));
    s.push_str(
        "<style>#phone{height:auto;max-height:none;margin:0;border-radius:0;box-shadow:none}\
         .hl{position:absolute;right:6px;top:6px;z-index:2;min-width:22px;height:22px;padding:0 5px;\
         border-radius:11px;background:#111;color:#fff;font:700 14px/22px system-ui,sans-serif;text-align:center;\
         box-shadow:0 0 0 2px #fff}\
         .face p{padding-right:40px}\
         html.gray{filter:grayscale(1)}</style>\n",
    );
    s.push_str(&format!(
        "</head><body data-theme=\"{theme}\" data-tab=\"feed\"{}>\n",
        ""
    ));
    s.push_str(
        "<main id=\"phone\"><header id=\"top\"><div id=\"title\"><b>Qapsule</b></div></header>\n",
    );
    s.push_str("<section id=\"feed\" class=\"page on\"><div id=\"list\">\n");
    for (label, img, text) in rows {
        s.push_str(&format!(
            "<div class=\"face\"><span class=\"hl\">{label}</span>"
        ));
        match img {
            Some(p) => s.push_str(&format!(
                "<img src=\"file://{}\" style=\"display:block;width:{FACE_W}px;height:{height}px\" alt=\"\">",
                p.display()
            )),
            None => s.push_str(&format!("<p>{}</p>", html_escape(text))),
        }
        s.push_str("</div>\n");
    }
    s.push_str("</div></section></main>\n");
    if gray {
        s.push_str("<script>document.documentElement.classList.add('gray')</script>\n");
    }
    s.push_str("</body></html>\n");
    s
}

/// What [`sheets`] wrote.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SheetOut {
    /// Page → code.
    pub codes: BTreeMap<usize, String>,
}

/// Root of M8's records.
#[must_use]
pub fn root(exp: &Path) -> PathBuf {
    exp.join("M8")
}

/// Reads `S8`.
///
/// # Errors
/// Missing or unreadable file.
pub fn read_s8(path: &Path) -> Result<S8, String> {
    serde_json::from_str(&io::read(path)?).map_err(|e| format!("{}: {e}", path.display()))
}

fn upsert_key(path: &Path, code: &str, v: Value) -> Result<(), String> {
    let mut m: BTreeMap<String, Value> = if path.exists() {
        serde_json::from_str(&io::read(path)?).map_err(|e| e.to_string())?
    } else {
        BTreeMap::new()
    };
    m.insert(code.to_owned(), v);
    io::write_json(path, &m)
}

/// Builds design `face`'s pages: face images (scratch), sheet pages, screenshots (light,
/// dark, grey; `raw/screens/<code>[.dark|.gray].png`), hidden keys, persona prompts, and
/// the computed measures (`raw/<face>/computed.json`). `shoot: false` skips Chromium.
///
/// # Errors
/// Any step failing.
pub fn sheets(face: &str, s8: &S8, exp: &Path, salt: u64, shoot: bool) -> Result<SheetOut, String> {
    io::check_name(face)?;
    let height = qp_grammar::faces::height(Some(face));
    let fs = faces(face, None, s8)?;
    let comp = computed(face, &fs);
    let raw = root(exp).join("raw");
    io::write_json(&raw.join(face).join("computed.json"), &comp)?;
    io::write(
        &raw.join(face).join("computed.md"),
        computed_md(&comp).as_bytes(),
    )?;
    let build = io::scratch().join("m8").join(face);
    let mut manifest: Vec<Value> = Vec::new();
    let mut codes = BTreeMap::new();
    for (pi, states) in pages(s8).iter().enumerate() {
        let page = pi + 1;
        let code = page_code(face, page, salt);
        let mut labels = BTreeMap::new();
        for theme in THEMES {
            let mut rows = Vec::new();
            for (li, st) in states.iter().enumerate() {
                let f = fs
                    .iter()
                    .find(|f| f.state == *st && f.lang == "tr" && f.theme == theme)
                    .ok_or(format!("no face for S8 #{st}"))?;
                let img = match &f.scene {
                    Some(sc) => {
                        let rgba = qp_render::render_scene_rgba_scaled(sc, shots::DPR)
                            .map_err(|e| e.to_string())?;
                        let png = qp_render::encode_png(
                            &rgba,
                            sc.width * shots::DPR,
                            sc.height * shots::DPR,
                        )
                        .map_err(|e| e.to_string())?;
                        let p = build.join(format!("p{page}-{theme}-{st}.png"));
                        io::write(&p, &png)?;
                        Some(p.canonicalize().unwrap_or(p))
                    }
                    None => None,
                };
                rows.push((LABELS[li].to_owned(), img, f.text.clone()));
                if theme == "light" {
                    let key = s8
                        .get(*st)
                        .and_then(|s| s.key.clone())
                        .ok_or("missing key")?;
                    labels.insert(LABELS[li].to_owned(), LabelKey { state: *st, key });
                }
            }
            let variants: &[(&str, bool)] = if theme == "light" {
                &[("", false), (".gray", true)]
            } else {
                &[(".dark", false)]
            };
            for (suffix, gray) in variants {
                let html = build.join(format!("p{page}{suffix}.html"));
                io::write(&html, sheet_html(theme, *gray, &rows, height).as_bytes())?;
                let out = raw.join("screens").join(format!("{code}{suffix}.png"));
                if let Some(d) = out.parent() {
                    std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
                }
                manifest.push(
                    json!({"html": html.canonicalize().unwrap_or(html).display().to_string(),
                    "out": out.display().to_string()}),
                );
            }
        }
        let pk = PageKey {
            experiment: "m8".into(),
            face: face.to_owned(),
            page,
            labels,
        };
        io::write_json(&raw.join("keys").join(format!("{code}.json")), &pk)?;
        upsert_key(
            &raw.join("key.json"),
            &code,
            json!({"experiment":"m8","design":face,"page":page,"states":states}),
        )?;
        for persona in prompts::PERSONAS {
            let p = prompts::m8_persona(persona, &format!("{code}.png"), states.len());
            blind::check("m8 prompt", &p)?;
            io::write(
                &raw.join("prompts").join(format!("{persona}_{code}.txt")),
                p.as_bytes(),
            )?;
        }
        codes.insert(page, code);
    }
    if shoot {
        let mf = build.join("shots.json");
        io::write_json(&mf, &manifest)?;
        shots::node("shoot.mjs", &[&mf.display().to_string()])?;
    }
    Ok(SheetOut { codes })
}

/// Markdown summary of [`Computed`].
#[must_use]
pub fn computed_md(c: &Computed) -> String {
    let mut s = format!(
        "# M8 computed — `{}` (height {} px)\n\nAll hard thresholds: **{}**\n\n| threshold | limit | worst | pass |\n|---|---:|---:|---|\n",
        c.face,
        c.height,
        if c.pass { "pass" } else { "FAIL" }
    );
    for (k, t) in &c.thresholds {
        s.push_str(&format!(
            "| {k} | {} | {} | {} |\n",
            t.limit,
            t.worst,
            if t.pass { "yes" } else { "NO" }
        ));
    }
    s.push_str("\n| lang | headline chars max / mean | words max / mean |\n|---|---:|---:|\n");
    for (l, v) in &c.headline_chars {
        let w = c.headline_words.get(l).copied().unwrap_or([0.0, 0.0]);
        s.push_str(&format!(
            "| {l} | {} / {} | {} / {} |\n",
            v[0], v[1], w[0], w[1]
        ));
    }
    s.push_str("\n| state | lang | theme | chars | text items | text area | extra | glyph px² | glyph share | text contrast min | glyph contrast | min px |\n|---:|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|\n");
    for m in &c.faces {
        s.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
            m.state,
            m.lang,
            m.theme,
            m.headline_chars,
            m.text_items,
            m.text_area_share,
            m.extra_units,
            m.glyph_area,
            m.glyph_share,
            m.text_contrast_min,
            m.glyph_contrast,
            m.min_text_px
        ));
    }
    s
}

// ---------------------------------------------------------------------------
// critics
// ---------------------------------------------------------------------------

/// Letters critics see designs as (no hex letter, so they never look like a page code).
pub const CRITIC_LETTERS: [&str; 10] = ["K", "L", "M", "N", "P", "R", "S", "T", "V", "Z"];

/// Sets up critic `k` (seed `k`): copies of every design's pages under per-critic random
/// letters (`raw/critics/c<k>/images/`), the prompt template and its hidden key. Designs:
/// every name with pages in `raw/key.json`.
///
/// # Errors
/// No pages, or I/O.
pub fn critic(exp: &Path, k: u64) -> Result<PathBuf, String> {
    let raw = root(exp).join("raw");
    let key: BTreeMap<String, Value> =
        serde_json::from_str(&io::read(&raw.join("key.json"))?).map_err(|e| e.to_string())?;
    let mut designs: BTreeMap<String, BTreeMap<u64, String>> = BTreeMap::new();
    for (code, v) in &key {
        if v["experiment"] == "m8" {
            let d = v["design"].as_str().unwrap_or_default().to_owned();
            designs
                .entry(d)
                .or_default()
                .insert(v["page"].as_u64().unwrap_or(0), code.clone());
        }
    }
    if designs.len() > CRITIC_LETTERS.len() {
        return Err("too many designs for the critic letters".into());
    }
    let mut names: Vec<String> = designs.keys().cloned().collect();
    let mut rng = Rng::derive(k, "m8-critic");
    rng.shuffle(&mut names);
    let mut letters: Vec<&str> = CRITIC_LETTERS.to_vec();
    rng.shuffle(&mut letters);
    let dir = raw.join("critics").join(format!("c{k}"));
    let mut images: Vec<(String, String)> = Vec::new(); // (source, target name)
    let mut listing: Vec<(String, Vec<String>)> = Vec::new();
    let mut hidden = BTreeMap::new();
    for (i, d) in names.iter().enumerate() {
        let l = letters[i].to_owned();
        hidden.insert(l.clone(), d.clone());
        let mut files = Vec::new();
        for (page, code) in &designs[d] {
            for (suffix, tag) in [("", ""), (".gray", "-gri")] {
                let src = raw.join("screens").join(format!("{code}{suffix}.png"));
                let name = format!("{l}-{page}{tag}.png");
                images.push((src.display().to_string(), name.clone()));
                files.push(name);
            }
        }
        listing.push((l, files));
    }
    let p = prompts::m8_critic(&listing);
    blind::check("critic prompt", &p)?;
    io::write(&dir.join("prompt.template.txt"), p.as_bytes())?;
    io::write_json(&dir.join("images.json"), &images)?;
    io::write_json(&dir.join("key.json"), &hidden)?;
    io::write_json(
        &dir.join("meta.json"),
        &json!({"experiment":"m8-critic","critic":format!("c{k}"),"seed":k,"designs":names.len()}),
    )?;
    Ok(dir)
}
