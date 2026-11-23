//! R5/M21 motion design **`nabiz`**: little motion, and only where it means something.
//!
//! Direction ("az ve anlamlı hareket: nabız"): transitions are short and plain — a
//! cross-fade with a slight deepening, nothing flies across the screen. The motion budget
//! goes to the moments that carry meaning:
//!
//! - **pulse** ("nabız"): the glyph of a *live* capsule (valid, story open) breathes —
//!   a faint ring around it swells and fades, slowly. A capsule that is not live is still;
//! - **beat** ("vuruş"): a state change the device sees is announced by one distinct beat
//!   on the glyph — born: one ring outwards; strengthened (or confirmed): two quick rings
//!   outwards; weakened: one faint ring inwards; invalid (invalidated, no longer holds): one
//!   strong ring inwards; closed (realized, expired, retracted): a disc that settles;
//! - **trail** ("iz"): the changed element keeps a fading highlight for a few seconds.
//!
//! Under **reduce motion** the same facts are given by still, formal marks, which are drawn
//! in both modes (so nothing is lost when motion is off): the **live dot** next to the glyph
//! of a live capsule, and the **change label** (the stage word: "doğdu", "güçlendi",
//! "geçersiz"…) while the change is new to the reader (since the reader's last look; without
//! one, while it is on the newest bar). Pulse, beat and trail are only added emphasis; they
//! are painted by the shell as overlay boxes the core describes per frame ([`overlay`]), so
//! the face itself is never redrawn for them (change blindness: the signal is local, the
//! card stays).
//!
//! Everything here is a pure function of its inputs (the frame of a time `t` is the same
//! on every run; no trigonometry: the pulse curve is a polynomial). Colours, sizes and the
//! duration scale come from the look ([`Look`]); the design's own durations and amplitudes
//! are the constants below (a look scales the durations with `motion.duration_scale`;
//! listed in `rounds/R5/experiments/M21/nabiz/DESIGN.md` as values for the registry).

use qp_schema::capsule::Direction;
use qp_schema::capsule::{Stage, Validity};
use qp_schema::protocol::{Language, SharedElement};
use qp_schema::scene::{Align, PixelRect, Prim, Rgba, SceneDesc};

use super::{r3_shared, Frame, FrameElement, Side, Source};
use qp_look::look::Look;
use qp_look::theme::Theme;

mod overlay;

pub use overlay::{
    beat_running, label_width, overlay, Beat, FaceMotion, OverlayBox, BEAT_GAP_MS, BEAT_MS,
    PULSE_ALPHA, PULSE_GROW, PULSE_MS, TRAIL_ALPHA, TRAIL_MS,
};

// --- transition --------------------------------------------------------------------

/// The whole old scene, deeper next (before box only).
const WHOLE_IN: &str = "scene:before";
/// The whole old scene, shallower next (closing; before box only).
const WHOLE_OUT: &str = "scene:before:out";
/// The whole new scene (after box only).
const WHOLE_AFTER: &str = "scene:after";
/// How much the old scene grows (opening) or shrinks (closing) while it fades.
const DEEPEN: f64 = 0.05;
/// The new scene settles from this much smaller (opening) or larger (closing).
const SETTLE: f64 = 0.03;

fn whole(sc: &SceneDesc) -> PixelRect {
    PixelRect {
        left: 0,
        top: 0,
        right: i32::try_from(sc.width).unwrap_or(i32::MAX),
        bottom: i32::try_from(sc.height).unwrap_or(i32::MAX),
    }
}

/// Shared elements: `r3`'s anchors (both boxes; the measured ones) plus the two whole
/// scenes (one box each, so never counted as shared). The old scene's id says the
/// direction (deeper or back).
#[must_use]
pub fn shared(before: &Side<'_>, after: &Side<'_>) -> Vec<SharedElement> {
    let mut out = r3_shared(before, after);
    if let Some(sc) = before.scene {
        let id = if after.depth >= before.depth {
            WHOLE_IN
        } else {
            WHOLE_OUT
        };
        out.push(SharedElement {
            id: id.to_owned(),
            before: Some(whole(sc)),
            after: None,
        });
    }
    if let Some(sc) = after.scene {
        out.push(SharedElement {
            id: WHOLE_AFTER.to_owned(),
            before: None,
            after: Some(whole(sc)),
        });
    }
    out
}

fn smooth(p: f64, a: f64, b: f64) -> f64 {
    let t = ((p - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn centre(r: &PixelRect) -> (f64, f64) {
    (
        f64::from(r.left + r.right) / 2.0,
        f64::from(r.top + r.bottom) / 2.0,
    )
}

/// `r` (scene px) scaled by `k` about the scene point `f`, placed at page offset `o`.
fn scaled(r: &PixelRect, f: (f64, f64), k: f64, o: (f64, f64)) -> [f64; 4] {
    let (l, t) = (f64::from(r.left), f64::from(r.top));
    let (w, h) = (
        f64::from((r.right - r.left).max(1)),
        f64::from((r.bottom - r.top).max(1)),
    );
    [
        f.0 + (l - f.0) * k + o.0,
        f.1 + (t - f.1) * k + o.1,
        w * k,
        h * k,
    ]
}

/// A frame: the old scene grows a little about its glyph (deeper; shrinks when closing)
/// while it fades out over the first 70 %; the new scene, at its own place, settles from
/// slightly smaller (larger when closing) to its size while it fades in. Nothing moves
/// from one place to another. Without scenes: the new scene fades in.
#[must_use]
pub fn frame(shared: &[SharedElement], from: (f64, f64), to: (f64, f64), p: f64) -> Frame {
    let p = p.clamp(0.0, 1.0);
    let glyph = shared.iter().find(|s| s.id == "glyph");
    let mut elements = Vec::new();
    let mut deeper = true;
    if let Some(s) = shared
        .iter()
        .find(|s| s.id == WHOLE_IN || s.id == WHOLE_OUT)
    {
        if let Some(r) = s.before {
            deeper = s.id == WHOLE_IN;
            let f = glyph
                .and_then(|g| g.before)
                .map_or(centre(&r), |g| centre(&g));
            let k = if deeper {
                1.0 + DEEPEN * p
            } else {
                1.0 - DEEPEN * p
            };
            elements.push(FrameElement {
                id: s.id.clone(),
                source: Source::Before,
                src: r,
                rect: scaled(&r, f, k, from),
                opacity: 1.0 - smooth(p, 0.0, 0.7),
            });
        }
    }
    let mut scene_opacity = smooth(p, 0.15, 1.0);
    if let Some(r) = shared
        .iter()
        .find(|s| s.id == WHOLE_AFTER)
        .and_then(|s| s.after)
    {
        let f = glyph
            .and_then(|g| g.after)
            .map_or(centre(&r), |g| centre(&g));
        let k0 = if deeper { 1.0 - SETTLE } else { 1.0 + SETTLE };
        let e = smooth(p, 0.0, 1.0);
        elements.push(FrameElement {
            id: WHOLE_AFTER.to_owned(),
            source: Source::After,
            src: r,
            rect: scaled(&r, f, k0 + (1.0 - k0) * e, to),
            opacity: scene_opacity,
        });
        // The real scene shows at the end only (the copy carries the fade).
        scene_opacity = if p >= 1.0 { 1.0 } else { 0.0 };
    }
    Frame {
        elements,
        scene_opacity,
    }
}

// --- live marks: what changed, from the capsule's row --------------------------------

/// A state change of a capsule, as the marks tell it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    /// Doğdu.
    Born,
    /// Güçlendi / teyit edildi.
    Strengthened,
    /// Zayıfladı.
    Weakened,
    /// Geçersiz: invalidated, or the state condition no longer holds.
    Invalid,
    /// Kapandı: realized, expired, retracted.
    Closed,
}

impl Change {
    /// The change a capsule's latest transition is (`None`: a candidate).
    #[must_use]
    pub fn of(stage: Stage, validity: Validity) -> Option<Self> {
        Some(match stage {
            Stage::Candidate => return None,
            Stage::Invalidated => Self::Invalid,
            Stage::Realized | Stage::Expired | Stage::Retracted => Self::Closed,
            _ if validity == Validity::NoLonger => Self::Invalid,
            Stage::Born => Self::Born,
            Stage::Strengthened | Stage::Confirmed => Self::Strengthened,
            Stage::Weakened => Self::Weakened,
        })
    }

    /// The change from what the device saw before (`None`: first seen) to now; `None`
    /// when nothing it tells changed.
    #[must_use]
    pub fn between(before: Option<(Stage, Validity)>, now: (Stage, Validity)) -> Option<Self> {
        match before {
            None => (now.0 == Stage::Born && now.1 == Validity::Valid).then_some(Self::Born),
            Some(b) if b == now => None,
            Some(_) => Self::of(now.0, now.1),
        }
    }
}

/// Whether a capsule is live (its glyph pulses; the live dot is drawn): valid, story open.
#[must_use]
pub fn is_live(stage: Stage, validity: Validity) -> bool {
    validity == Validity::Valid && !stage.is_closed() && stage != Stage::Candidate
}

/// The change label: the stage word in the reader's language.
#[must_use]
pub fn label(stage: Stage, validity: Validity, lang: Language) -> Option<&'static str> {
    let tr = lang == Language::Tr;
    let w = match stage {
        Stage::Candidate => return None,
        _ if validity == Validity::NoLonger && !stage.is_closed() => {
            if tr {
                "artık tutmuyor"
            } else {
                "no longer holds"
            }
        }
        Stage::Born => {
            if tr {
                "doğdu"
            } else {
                "born"
            }
        }
        Stage::Strengthened => {
            if tr {
                "güçlendi"
            } else {
                "strengthened"
            }
        }
        Stage::Confirmed => {
            if tr {
                "teyit edildi"
            } else {
                "confirmed"
            }
        }
        Stage::Weakened => {
            if tr {
                "zayıfladı"
            } else {
                "weakened"
            }
        }
        Stage::Invalidated => {
            if tr {
                "geçersiz"
            } else {
                "invalidated"
            }
        }
        Stage::Realized => {
            if tr {
                "gerçekleşti"
            } else {
                "played out"
            }
        }
        Stage::Expired => {
            if tr {
                "süresi doldu"
            } else {
                "expired"
            }
        }
        Stage::Retracted => {
            if tr {
                "geri çekildi"
            } else {
                "retracted"
            }
        }
    };
    Some(w)
}

/// What the still marks of one face show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FaceMarks<'a> {
    /// The capsule is live (the live dot).
    pub live: bool,
    /// The change label, while the change is new to the reader.
    pub label: Option<&'a str>,
    /// Its kind (the label's colour).
    pub change: Option<Change>,
    /// The capsule's direction (the live dot's colour: its glyph's).
    pub direction: Direction,
    /// (R5/S) Under reduced motion the live dot also says it in words ([`live_word`]):
    /// the pulse that tells "live" in motion is gone, so the fact stays as text.
    pub live_word: Option<&'a str>,
}

/// (R5/S) The word beside the live dot under reduced motion.
#[must_use]
pub fn live_word(lang: Language) -> &'static str {
    if lang == Language::Tr {
        "canlı"
    } else {
        "live"
    }
}

/// Where a face's marks go (scene px): the glyph box, the live dot (centre, radius) and
/// the change label's box and baseline.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Places {
    /// The glyph's box.
    pub glyph: PixelRect,
    /// The live dot: centre x, centre y, radius.
    pub dot: (f32, f32, f32),
    /// The label's anchor x and baseline.
    pub label_at: (i32, i32),
    /// The label row: top and bottom (the trail's box with the label's width).
    pub row: (i32, i32),
}

fn glyph_box(scene: &SceneDesc) -> Option<PixelRect> {
    qp_grammar::depth::transition_anchors(scene, None, None)
        .get("glyph")
        .copied()
}

/// The places of a face's marks (`None`: the face has no glyph). The label goes into the
/// title row above the plot when that row holds no text (under a voice the presented face
/// leaves it empty), else into the bottom margin; it starts at the glyph's left edge with
/// the live dot before it.
#[must_use]
pub fn places(scene: &SceneDesc, look: &Look) -> Option<Places> {
    let g = glyph_box(scene)?;
    let lab = look.types.label;
    let top_row = look.space.plot[2];
    let row_free = !scene
        .over
        .iter()
        .chain(scene.under.iter())
        .any(|p| matches!(p, Prim::Text { baseline, text, .. } if *baseline <= top_row && !text.trim().is_empty()));
    let line = (lab.px * lab.line).round();
    #[allow(clippy::cast_possible_truncation)]
    let line_i = line as i32;
    let h = i32::try_from(scene.height).unwrap_or(i32::MAX);
    let (row_top, row_bottom) = if row_free {
        let mid = top_row / 2;
        (mid - line_i / 2, mid - line_i / 2 + line_i)
    } else {
        let bottom = look.space.plot[3];
        let mid = h - bottom / 2;
        (mid - line_i / 2, mid - line_i / 2 + line_i)
    };
    let r = (lab.px / 3.5).max(2.5);
    let cx = f64::from(g.left) + r;
    let cy = f64::from(row_top + row_bottom) / 2.0;
    // Baseline: the row's middle plus a third of the size (the x-height's middle).
    #[allow(clippy::cast_possible_truncation)]
    let baseline = (cy + lab.px / 3.0).round() as i32;
    #[allow(clippy::cast_possible_truncation)]
    let lx = (cx + r + f64::from(look.space.scale[1])).round() as i32;
    #[allow(clippy::cast_possible_truncation)]
    Some(Places {
        glyph: g,
        dot: (cx as f32, cy as f32, r as f32),
        label_at: (lx, baseline),
        row: (row_top, row_bottom),
    })
}

/// The colour of a change (label, beat, trail).
#[must_use]
pub fn change_color(c: Option<Change>, theme: &Theme) -> Rgba {
    match c {
        Some(Change::Invalid | Change::Weakened) => theme.invalid,
        Some(Change::Closed) => theme.muted,
        _ => theme.text,
    }
}

/// The colour of a glyph's pulse and live dot: its direction's; without a direction the
/// text colour (as the outcome mark's badge: `r4`'s `neutral` is 2.8:1 on the light
/// background, under the 3:1 a non-text mark needs).
#[must_use]
pub fn pulse_color(d: Direction, theme: &Theme) -> Rgba {
    match d {
        Direction::Up => theme.up,
        Direction::Down => theme.down,
        Direction::Neutral => theme.text,
    }
}

/// Draws the still marks into a face scene (after everything else; no hit box changes):
/// the live dot (its direction's colour, as the glyph) and the change label (the look's `label` role). The same
/// in both motion modes.
pub fn draw_face(scene: &mut SceneDesc, m: &FaceMarks<'_>, look: &Look, theme: &Theme) {
    if !m.live && m.label.is_none() {
        return;
    }
    // (R5/S) Reduced motion: "live" in words beside the dot (before a change label).
    let word = m.live_word.filter(|_| m.live);
    let joined;
    let label = match (word, m.label) {
        (Some(w), Some(l)) => {
            joined = format!("{w} · {l}");
            Some(joined.as_str())
        }
        (Some(w), None) => Some(w),
        (None, l) => l,
    };
    let Some(pl) = places(scene, look) else {
        return;
    };
    if m.live {
        scene.over.push(Prim::Circle {
            cx: pl.dot.0,
            cy: pl.dot.1,
            r: pl.dot.2,
            color: pulse_color(m.direction, theme),
        });
    }
    if let Some(text) = label {
        #[allow(clippy::cast_possible_truncation)]
        scene.over.push(Prim::Text {
            x: pl.label_at.0,
            baseline: pl.label_at.1,
            px: look.types.label.px as f32,
            text: text.to_owned(),
            color: if m.label.is_some() {
                change_color(m.change, theme)
            } else {
                theme.text
            },
            align: Align::Left,
            font: look.scene_font("label"),
            role: None,
        });
    }
}

#[cfg(test)]
mod tests;
