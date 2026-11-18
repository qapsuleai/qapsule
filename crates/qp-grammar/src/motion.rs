//! (R5/M21) Motion designs: what moves when something changes, and what stands in for
//! the motion when the reader asked for less of it ("hareketi azalt").
//!
//! A motion design bundles three answers (`rounds/R5/GOAL.md` M21):
//!
//! 1. **The opening** (feed → capsule → chart and back): a depth transition design
//!    ([`crate::transitions`]), which the design names ([`MotionDesign::transition`]).
//! 2. **A live change of a face in the feed** (a value the sentence states changed): what
//!    the face shows while it changes ([`change_frame`], kind [`ChangeKind::Value`]).
//! 3. **A state change** (born, strengthened, weakened, no longer holds, failed, came true,
//!    expired, withdrawn): the glyph itself changes form ([`change_frame`]).
//!
//! Under **reduced motion** all three happen at once and every change leaves a small,
//! lasting status label on the face instead ([`stamp`], "az önce güçlendi"): the same
//! fact through a still channel (information loss 0; [`ChangeKind`] is the fact both
//! channels carry, [`label`] its words).
//!
//! The reference **`r4`** is today's motion: the presented transition, no change marks
//! (a face repaints in place). `surek` is "continuity: the same object changes place"
//! (`rounds/R5/experiments/M21/surek/DESIGN.md`).
//!
//! Frames are pure functions of the two face scenes' boxes and the progress `p`, so any
//! in-between frame can be reproduced (frozen) from the scene descriptions alone. They use
//! the transition frame vocabulary ([`crate::transitions::Frame`]): pixel copies from the
//! face before (the shell's snapshot of its canvas) or after the change, each at a page box
//! and an opacity; `scene_opacity` is the opacity of the repainted face itself. Durations
//! and the easing curve come from the look (`Look.motion.change_ms`, `duration_scale`,
//! `ease`).
//!
//! **Adding a design:** one line in [`MOTIONS`] (and, if it opens differently, a
//! transition design in `transitions/`).

use std::collections::BTreeSet;

use qp_schema::capsule::{Stage, Validity};
use qp_schema::protocol::{CapsuleItem, Language};
use qp_schema::scene::{Align, PixelRect, Prim, SceneDesc};

use crate::look::{self, Look};
use crate::theme::Theme;
use crate::transitions::{self, Frame, FrameElement, Source};

/// One motion design.
#[derive(Debug, Clone, Copy)]
pub struct MotionDesign {
    /// Setting name (`[a-z][a-z0-9_-]*`).
    pub name: &'static str,
    /// The depth transition design it opens with (`transitions::TRANSITIONS`).
    pub transition: &'static str,
    /// Whether it marks a face's own changes (live value, state) with motion — and, under
    /// reduced motion, with a lasting label.
    pub marks: bool,
    /// (R5/S, `nabiz`) Its live marks: still marks drawn into each face (live dot, change
    /// label) and the overlay boxes of a display frame (pulse, beat, trail); `None`: none.
    pub live: Option<transitions::LiveMarks>,
}

impl PartialEq for MotionDesign {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

/// The default design: today's motion.
pub const DEFAULT_MOTION: &str = "r4";

/// Motion designs a reader may choose; the first is the default.
pub static MOTIONS: &[MotionDesign] = &[
    MotionDesign {
        name: "r4",
        transition: transitions::PRESENTED_TRANSITION,
        marks: false,
        live: None,
    },
    // --- M21 alternatives: one line each
    MotionDesign {
        name: "surek",
        transition: "surek",
        marks: true,
        live: None,
    },
    // (R5/S) M21's second: `nabiz` (a short deepening cross-fade; a live capsule pulses,
    // a state change beats once and leaves a trail; still marks under reduced motion).
    MotionDesign {
        name: "nabiz",
        transition: "nabiz",
        marks: false,
        live: Some(transitions::LiveMarks {
            draw_face: transitions::nabiz::draw_face,
            places: transitions::nabiz::places,
            overlay: transitions::nabiz::overlay,
        }),
    },
];

/// (R5/S) The motion design the presentation shell starts with (M21: no winner was
/// measured; `surek` by the pre-written rule, `rounds/R5/experiments/M21/RESULTS.md`).
/// The protocol has no motion setting (a device presentation setting).
pub const PRESENTED_MOTION: &str = "surek";

/// Registered names.
#[must_use]
pub fn names() -> Vec<&'static str> {
    MOTIONS.iter().map(|m| m.name).collect()
}

/// The design named `name` (`None` or unknown: the default).
#[must_use]
pub fn by_name(name: Option<&str>) -> &'static MotionDesign {
    let n = name.unwrap_or(DEFAULT_MOTION);
    MOTIONS.iter().find(|m| m.name == n).unwrap_or(&MOTIONS[0])
}

/// Whether `name` is a known design.
#[must_use]
pub fn is_motion(name: &str) -> bool {
    MOTIONS.iter().any(|m| m.name == name)
}

/// What changed on a face: the one fact a change animation and its still stand-in carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ChangeKind {
    /// Doğdu: a new capsule.
    Born,
    /// Güçlendi (also "teyit edildi": a further evidence family).
    Strengthened,
    /// Zayıfladı.
    Weakened,
    /// Artık geçerli değil: the state condition broke (the story is still open).
    NoLonger,
    /// Tutmadı (`invalidated`).
    Invalidated,
    /// Gerçekleşti.
    Realized,
    /// Süresi doldu.
    Expired,
    /// Geri çekildi.
    Retracted,
    /// The capsule's own values changed within the same stage (a new fact of the same
    /// family, another evidence family); not the age clock.
    Value,
}

impl ChangeKind {
    /// Every kind, in order.
    pub const ALL: [ChangeKind; 9] = [
        ChangeKind::Born,
        ChangeKind::Strengthened,
        ChangeKind::Weakened,
        ChangeKind::NoLonger,
        ChangeKind::Invalidated,
        ChangeKind::Realized,
        ChangeKind::Expired,
        ChangeKind::Retracted,
        ChangeKind::Value,
    ];

    /// Stable name (records, tests, the page API).
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            ChangeKind::Born => "born",
            ChangeKind::Strengthened => "strengthened",
            ChangeKind::Weakened => "weakened",
            ChangeKind::NoLonger => "no_longer",
            ChangeKind::Invalidated => "invalidated",
            ChangeKind::Realized => "realized",
            ChangeKind::Expired => "expired",
            ChangeKind::Retracted => "retracted",
            ChangeKind::Value => "value",
        }
    }
}

/// The lasting label of a change under reduced motion (no digits: it is the same for a
/// reader without a data entitlement, K33).
#[must_use]
pub fn label(kind: ChangeKind, lang: Language) -> &'static str {
    match (lang, kind) {
        (Language::Tr, ChangeKind::Born) => "az önce doğdu",
        (Language::Tr, ChangeKind::Strengthened) => "az önce güçlendi",
        (Language::Tr, ChangeKind::Weakened) => "az önce zayıfladı",
        (Language::Tr, ChangeKind::NoLonger) => "az önce geçerliliğini yitirdi",
        (Language::Tr, ChangeKind::Invalidated) => "az önce tutmadı",
        (Language::Tr, ChangeKind::Realized) => "az önce gerçekleşti",
        (Language::Tr, ChangeKind::Expired) => "az önce süresi doldu",
        (Language::Tr, ChangeKind::Retracted) => "az önce geri çekildi",
        (Language::Tr, ChangeKind::Value) => "az önce güncellendi",
        (Language::En, ChangeKind::Born) => "just appeared",
        (Language::En, ChangeKind::Strengthened) => "just strengthened",
        (Language::En, ChangeKind::Weakened) => "just weakened",
        (Language::En, ChangeKind::NoLonger) => "just stopped holding",
        (Language::En, ChangeKind::Invalidated) => "just failed",
        (Language::En, ChangeKind::Realized) => "just came true",
        (Language::En, ChangeKind::Expired) => "just expired",
        (Language::En, ChangeKind::Retracted) => "just withdrawn",
        (Language::En, ChangeKind::Value) => "just updated",
    }
}

/// The change from `old` to `new` of one capsule of the feed, if any. `old = None`: the
/// capsule was not in the last feed; it counts as born only when it is in its `born`
/// stage and newer than everything the last feed held (`newest_before`, `None`: there
/// was no last feed — the first feed marks nothing).
#[must_use]
pub fn classify(
    old: Option<&CapsuleItem>,
    new: &CapsuleItem,
    newest_before: Option<i64>,
) -> Option<ChangeKind> {
    let Some(old) = old else {
        return (new.stage == Stage::Born && newest_before.is_some_and(|t| new.t > t))
            .then_some(ChangeKind::Born);
    };
    if old.stage != new.stage {
        return match new.stage {
            Stage::Candidate | Stage::Born => None,
            Stage::Strengthened | Stage::Confirmed => Some(ChangeKind::Strengthened),
            Stage::Weakened => Some(ChangeKind::Weakened),
            Stage::Realized => Some(ChangeKind::Realized),
            Stage::Invalidated => Some(ChangeKind::Invalidated),
            Stage::Expired => Some(ChangeKind::Expired),
            Stage::Retracted => Some(ChangeKind::Retracted),
        };
    }
    if old.validity == Validity::Valid && new.validity == Validity::NoLonger {
        return Some(ChangeKind::NoLonger);
    }
    // A new transition within the same stage (another fact of the same family) or a
    // change in the evidence families: the capsule's own values changed. The age clock
    // ("12 işlem günü önce") and the face's price line move with every bar and are not a
    // change of the capsule (marking them would mark every face on every bar).
    (old.last_bar != new.last_bar || old.confirmations != new.confirmations)
        .then_some(ChangeKind::Value)
}

/// The geometry of one face change (face scene px): where the glyph was and is, and the
/// boxes of the texts that differ.
#[derive(Debug, Clone, PartialEq)]
pub struct FaceChange {
    /// What changed.
    pub kind: ChangeKind,
    /// The face's size.
    pub size: (u32, u32),
    /// The glyph's box before the change (the after box when the before scene is unknown).
    pub glyph_before: Option<PixelRect>,
    /// The glyph's box after the change.
    pub glyph_after: Option<PixelRect>,
    /// Boxes of the texts that differ (at most [`MAX_SPOTS`]; each covers the text before
    /// and after).
    pub spots: Vec<PixelRect>,
}

/// The pixel a text plate copies: the face's top left corner, its background in every
/// face design (the glyph and the texts are inset from it).
pub const PLATE: PixelRect = PixelRect {
    left: 0,
    top: 0,
    right: 1,
    bottom: 1,
};

/// At most this many text boxes take part in a change frame.
pub const MAX_SPOTS: usize = 4;

fn union(a: PixelRect, b: PixelRect) -> PixelRect {
    PixelRect {
        left: a.left.min(b.left),
        top: a.top.min(b.top),
        right: a.right.max(b.right),
        bottom: a.bottom.max(b.bottom),
    }
}

/// Box of a text prim (the compiler's measure, `FontAsset::width`).
#[allow(clippy::cast_possible_truncation)]
fn text_box(
    x: i32,
    baseline: i32,
    px: f32,
    text: &str,
    font: Option<&str>,
    align: Align,
) -> PixelRect {
    let f = font.and_then(look::font).unwrap_or(&look::FONTS[0]);
    let size = f64::from(px);
    let w = f.width(text, size);
    let left = match align {
        Align::Left => x,
        Align::Right => x - w,
    };
    PixelRect {
        left,
        top: baseline - (size * f.ascent).ceil() as i32,
        right: left + w,
        bottom: baseline + (size * 0.25).ceil() as i32,
    }
}

type TextKey = (i32, i32, u32, String);

fn texts(sc: &SceneDesc) -> Vec<(TextKey, PixelRect)> {
    let mut out = Vec::new();
    let mut take = |p: &Prim| {
        if let Prim::Text {
            x,
            baseline,
            px,
            text,
            font,
            align,
            ..
        } = p
        {
            out.push((
                (*x, *baseline, px.to_bits(), text.clone()),
                text_box(*x, *baseline, *px, text, font.as_deref(), *align),
            ));
        }
    };
    sc.under.iter().for_each(&mut take);
    for p in &sc.panels {
        for l in &p.layers {
            l.prims.iter().for_each(&mut take);
        }
    }
    sc.over.iter().for_each(&mut take);
    out
}

/// The boxes of the texts one scene has and the other has not, merged by line (boxes that
/// overlap vertically become one), at most [`MAX_SPOTS`], top to bottom.
fn text_spots(before: &SceneDesc, after: &SceneDesc) -> Vec<PixelRect> {
    let (tb, ta) = (texts(before), texts(after));
    let kb: BTreeSet<&TextKey> = tb.iter().map(|(k, _)| k).collect();
    let ka: BTreeSet<&TextKey> = ta.iter().map(|(k, _)| k).collect();
    let mut boxes: Vec<PixelRect> = tb
        .iter()
        .filter(|(k, _)| !ka.contains(k))
        .chain(ta.iter().filter(|(k, _)| !kb.contains(k)))
        .map(|(_, r)| *r)
        .collect();
    boxes.sort_by_key(|r| (r.top, r.left));
    let mut out: Vec<PixelRect> = Vec::new();
    for r in boxes {
        match out.last_mut() {
            Some(l) if r.top < l.bottom && r.bottom > l.top => *l = union(*l, r),
            _ => out.push(r),
        }
    }
    out.truncate(MAX_SPOTS);
    out
}

/// The geometry of a change of kind `kind` from face scene `before` (`None`: unknown, or
/// the capsule is new) to `after`.
#[must_use]
pub fn face_change(kind: ChangeKind, before: Option<&SceneDesc>, after: &SceneDesc) -> FaceChange {
    let glyph = |sc: &SceneDesc| {
        crate::depth::transition_anchors(sc, None, None)
            .get("glyph")
            .copied()
    };
    let glyph_after = glyph(after);
    let glyph_before = before.and_then(glyph).or(glyph_after);
    let spots = match (kind, before) {
        (ChangeKind::Born, _) | (_, None) => Vec::new(),
        (_, Some(b)) => text_spots(b, after),
    };
    FaceChange {
        kind,
        size: (after.width, after.height),
        glyph_before,
        glyph_after,
        spots,
    }
}

/// The progress (0..=1) of a change that started at `t0` ms, at `t` ms, in `look`.
#[must_use]
pub fn change_progress(look: &Look, t0: f64, t: f64) -> f64 {
    let d = change_duration(look);
    if !(t.is_finite() && t0.is_finite()) || d <= 0.0 {
        return 1.0;
    }
    look.motion.ease.apply(((t - t0) / d).clamp(0.0, 1.0))
}

/// A change's duration in `look`, ms.
#[must_use]
pub fn change_duration(look: &Look) -> f64 {
    look.motion.change_ms * look.motion.duration_scale
}

use transitions::surek::ramp;

/// `4p(1 − p)`: 0 at both ends, 1 in the middle.
fn bump(p: f64) -> f64 {
    4.0 * p * (1.0 - p)
}

fn at(r: &PixelRect, o: (f64, f64)) -> [f64; 4] {
    [
        f64::from(r.left) + o.0,
        f64::from(r.top) + o.1,
        f64::from((r.right - r.left).max(1)),
        f64::from((r.bottom - r.top).max(1)),
    ]
}

/// `r` scaled by (`sx`, `sy`) about its centre and moved by (`dx`, `dy`).
fn about(r: [f64; 4], sx: f64, sy: f64, dx: f64, dy: f64) -> [f64; 4] {
    let (cx, cy) = (r[0] + r[2] / 2.0, r[1] + r[3] / 2.0);
    let (w, h) = (r[2] * sx, r[3] * sy);
    [cx - w / 2.0 + dx, cy - h / 2.0 + dy, w, h]
}

fn el(id: &str, source: Source, src: PixelRect, rect: [f64; 4], opacity: f64) -> FrameElement {
    FrameElement {
        id: id.to_owned(),
        source,
        src,
        rect,
        opacity: opacity.clamp(0.0, 1.0),
    }
}

/// Halves of a box (left, right).
fn halves(r: &PixelRect) -> (PixelRect, PixelRect) {
    let mid = r.left + ((r.right - r.left) / 2).max(1);
    (
        PixelRect { right: mid, ..*r },
        PixelRect { left: mid, ..*r },
    )
}

/// A frame of face change `ch` at progress `p`, the face at page (`left`, `top`). The
/// element list (count, order, source, `src`) is the same for every `p`.
///
/// - every kind but `born`: each changed text changes in place, in turn (the text before
///   goes out onto a plate of the face's background, the text after comes in, swells once
///   and settles — the only local emphasis; nothing else of the card moves);
/// - `born`: the face comes up and its glyph grows from a point at its own place;
/// - `strengthened`: the glyph swells once into its new form;
/// - `weakened`: the old glyph shrinks into the new;
/// - `no_longer`: the old glyph sinks and shrinks into the new;
/// - `invalidated`: the old glyph **breaks**: its two halves split apart and fall away
///   while the new (failed) form appears between them;
/// - `realized`: the new form is stamped down onto the old;
/// - `expired`, `retracted`: the old glyph folds flat into the new.
#[must_use]
pub fn change_frame(ch: &FaceChange, left: f64, top: f64, p: f64) -> Frame {
    let p = p.clamp(0.0, 1.0);
    let o = (left, top);
    let mut els: Vec<FrameElement> = Vec::new();
    if ch.kind != ChangeKind::Born {
        for (i, s) in ch.spots.iter().enumerate() {
            let r = at(s, o);
            // In turn, never both at once: the text before goes out on a plate of the
            // face's own background (one background pixel stretched over the box), then
            // the text after comes in, swells once and settles while the plate goes.
            let sw = 1.0 + 0.12 * bump(ramp(p, 0.4, 1.0));
            els.push(el(
                &format!("text:{i}:plate"),
                Source::After,
                PLATE,
                r,
                1.0 - ramp(p, 0.4, 0.75),
            ));
            els.push(el(
                &format!("text:{i}:after"),
                Source::After,
                *s,
                about(r, sw, sw, 0.0, 0.0),
                ramp(p, 0.35, 0.65),
            ));
            els.push(el(
                &format!("text:{i}:before"),
                Source::Before,
                *s,
                r,
                1.0 - ramp(p, 0.0, 0.35),
            ));
        }
    }
    let (gb, ga) = (ch.glyph_before, ch.glyph_after);
    let base = if ch.kind == ChangeKind::Born {
        ramp(p, 0.35, 0.9)
    } else {
        1.0
    };
    match ch.kind {
        ChangeKind::Born => {
            if let Some(a) = ga {
                let s = 0.2 + 0.8 * ramp(p, 0.0, 0.6);
                els.push(el(
                    "glyph:after",
                    Source::After,
                    a,
                    about(at(&a, o), s, s, 0.0, 0.0),
                    1.0,
                ));
            }
        }
        ChangeKind::Strengthened | ChangeKind::Realized => {
            if let (Some(b), Some(a)) = (gb, ga) {
                els.push(el(
                    "glyph:before",
                    Source::Before,
                    b,
                    at(&b, o),
                    1.0 - ramp(p, 0.0, 0.4),
                ));
                let s = if ch.kind == ChangeKind::Realized {
                    1.4 - 0.4 * ramp(p, 0.0, 0.6)
                } else {
                    1.0 + 0.3 * bump(p)
                };
                els.push(el(
                    "glyph:after",
                    Source::After,
                    a,
                    about(at(&a, o), s, s, 0.0, 0.0),
                    ramp(p, 0.0, 0.35),
                ));
            }
        }
        ChangeKind::Weakened | ChangeKind::NoLonger => {
            if let Some(b) = gb {
                let t = ramp(p, 0.0, 0.7);
                let r = at(&b, o);
                let (s, dy) = if ch.kind == ChangeKind::NoLonger {
                    (1.0 - 0.15 * t, 0.25 * r[3] * t)
                } else {
                    (1.0 - 0.25 * t, 0.0)
                };
                els.push(el(
                    "glyph:before",
                    Source::Before,
                    b,
                    about(r, s, s, 0.0, dy),
                    1.0 - ramp(p, 0.2, 0.75),
                ));
            }
        }
        ChangeKind::Invalidated => {
            if let Some(b) = gb {
                let t = ramp(p, 0.0, 0.6);
                let (l, r) = halves(&b);
                let (rl, rr) = (at(&l, o), at(&r, o));
                let (w, h) = (at(&b, o)[2], at(&b, o)[3]);
                let fade = 1.0 - ramp(p, 0.35, 0.85);
                els.push(el(
                    "glyph:before:left",
                    Source::Before,
                    l,
                    about(rl, 1.0, 1.0, -0.12 * w * t, 0.12 * h * t),
                    fade,
                ));
                els.push(el(
                    "glyph:before:right",
                    Source::Before,
                    r,
                    about(rr, 1.0, 1.0, 0.12 * w * t, -0.12 * h * t),
                    fade,
                ));
            }
        }
        ChangeKind::Expired | ChangeKind::Retracted => {
            if let Some(b) = gb {
                let t = ramp(p, 0.0, 0.7);
                els.push(el(
                    "glyph:before",
                    Source::Before,
                    b,
                    about(at(&b, o), 1.0, 1.0 - 0.8 * t, 0.0, 0.0),
                    1.0 - ramp(p, 0.3, 0.8),
                ));
            }
        }
        ChangeKind::Value => {}
    }
    Frame {
        elements: els,
        scene_opacity: base,
    }
}

/// The theme a face scene was compiled in (by its background; the look's default
/// otherwise).
fn theme_of(scene: &SceneDesc, look: &Look) -> Theme {
    if scene.background == look.light.bg {
        look.light
    } else if scene.background == look.dark.bg {
        look.dark
    } else {
        look.theme(None).unwrap_or(look.dark)
    }
}

/// Puts the lasting label of change `kind` on face `scene` (reduced motion): the label
/// role's size and font, the theme's text colour on a plate of its background, in the
/// face's lower left corner (inset by the look's `offsets.title_x`; the shell's own marks —
/// trust, coin flip — sit at the right). Pixels only in `scene.over`; the rest of the
/// scene is unchanged.
#[allow(clippy::cast_possible_truncation)]
pub fn stamp(scene: &mut SceneDesc, kind: ChangeKind, lang: Language, look: &Look) {
    let th = theme_of(scene, look);
    let role = look.types.label;
    let font = look.role_font("label");
    let text = label(kind, lang);
    let w = font.width(text, role.px);
    let asc = (role.px * font.ascent).ceil() as i32;
    let desc = (role.px * 0.25).ceil() as i32;
    let inset = look.offsets.title_x.max(2);
    let left = inset;
    let baseline = i32::try_from(scene.height).unwrap_or(i32::MAX) - desc - inset / 2;
    let pad = (role.px / 4.0).ceil() as i32;
    scene.over.push(Prim::Rect {
        x: left - pad,
        y: baseline - asc - pad / 2,
        w: u32::try_from(w + 2 * pad).unwrap_or(0),
        h: u32::try_from(asc + desc + pad).unwrap_or(0),
        color: th.bg,
    });
    scene.over.push(Prim::Text {
        x: left,
        baseline,
        px: role.px as f32,
        text: text.to_owned(),
        color: th.text,
        align: Align::Left,
        font: look.scene_font("label"),
        role: None,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(l: i32, t: i32, rr: i32, b: i32) -> PixelRect {
        PixelRect {
            left: l,
            top: t,
            right: rr,
            bottom: b,
        }
    }

    fn change(kind: ChangeKind) -> FaceChange {
        FaceChange {
            kind,
            size: (390, 120),
            glyph_before: Some(r(8, 8, 40, 40)),
            glyph_after: Some(r(8, 8, 40, 40)),
            spots: vec![r(48, 10, 200, 28)],
        }
    }

    #[test]
    fn registry() {
        assert_eq!(MOTIONS[0].name, DEFAULT_MOTION);
        for m in MOTIONS {
            assert!(transitions::is_transition(m.transition), "{}", m.name);
            assert!(m
                .name
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_'));
        }
        assert!(is_motion("surek") && !is_motion("nope"));
        assert_eq!(by_name(Some("nope")).name, "r4");
        assert!(!by_name(None).marks && by_name(Some("surek")).marks);
    }

    #[test]
    fn labels_have_no_digits_and_differ() {
        for lang in [Language::Tr, Language::En] {
            let all: BTreeSet<&str> = ChangeKind::ALL.iter().map(|k| label(*k, lang)).collect();
            assert_eq!(all.len(), ChangeKind::ALL.len());
            for t in all {
                assert!(!t.chars().any(|c| c.is_ascii_digit()), "{t}");
            }
        }
    }

    #[test]
    fn every_kind_moves_and_keeps_its_element_list() {
        for k in ChangeKind::ALL {
            let ch = change(k);
            let f0 = change_frame(&ch, 0.0, 100.0, 0.0);
            assert!(!f0.elements.is_empty(), "{k:?}");
            let mut moved = false;
            for p in [0.1, 0.25, 0.5, 0.75, 1.0] {
                let f = change_frame(&ch, 0.0, 100.0, p);
                assert_eq!(f.elements.len(), f0.elements.len(), "{k:?}");
                for (a, b) in f.elements.iter().zip(&f0.elements) {
                    assert_eq!((&a.id, a.src, a.source), (&b.id, b.src, b.source));
                    moved |= a.rect != b.rect || (a.opacity - b.opacity).abs() > 1e-9;
                }
            }
            assert!(moved, "{k:?} shows no change");
            // At the end the face itself is fully shown and every copy from before is gone.
            let f1 = change_frame(&ch, 0.0, 100.0, 1.0);
            assert!((f1.scene_opacity - 1.0).abs() < 1e-12);
            for e in &f1.elements {
                if e.source == Source::Before || e.id.ends_with(":plate") {
                    assert!(e.opacity < 1e-9, "{k:?} {}", e.id);
                }
            }
        }
    }

    #[test]
    fn texts_change_in_turn_without_a_double_image() {
        let ch = change(ChangeKind::Value);
        for i in 0..=100 {
            let f = change_frame(&ch, 0.0, 0.0, f64::from(i) / 100.0);
            let op = |id: &str| f.elements.iter().find(|e| e.id == id).unwrap().opacity;
            let (b, a, pl) = (op("text:0:before"), op("text:0:after"), op("text:0:plate"));
            assert!(b.min(a) < 0.15, "p={i}% before {b} after {a}");
            // While the text before is still seen, the plate hides the text after underneath.
            if b > 0.15 {
                assert!(pl > 0.99, "p={i}%");
            }
        }
    }

    #[test]
    fn a_failed_glyph_breaks_in_two() {
        let ch = change(ChangeKind::Invalidated);
        let f = change_frame(&ch, 0.0, 0.0, 0.5);
        let l = f
            .elements
            .iter()
            .find(|e| e.id == "glyph:before:left")
            .unwrap();
        let rr = f
            .elements
            .iter()
            .find(|e| e.id == "glyph:before:right")
            .unwrap();
        // The halves have parted: a gap between them, and they moved opposite ways.
        assert!(rr.rect[0] - (l.rect[0] + l.rect[2]) > 1.0);
        assert!(l.rect[1] > 8.0 && rr.rect[1] < 8.0);
    }

    #[test]
    fn born_has_nothing_from_before() {
        let ch = change(ChangeKind::Born);
        let f = change_frame(&ch, 0.0, 0.0, 0.3);
        assert!(f.elements.iter().all(|e| e.source == Source::After));
        assert!(f.scene_opacity < 1.0);
    }

    #[test]
    fn progress_follows_the_look() {
        let l = &crate::look::R4;
        assert!((change_progress(l, 0.0, change_duration(l) / 2.0) - 0.5).abs() < 1e-12);
        assert!((change_progress(l, 0.0, 1e9) - 1.0).abs() < 1e-12);
    }

    fn blank(bg: [u8; 4]) -> SceneDesc {
        serde_json::from_value(serde_json::json!({
            "view": "f", "width": 390, "height": 120, "background": bg,
            "x": {"from_bar": 0, "to_bar": 0, "left": 0.0, "slot": 1.0}, "bars": 0,
            "panels": [], "emphasis": [], "under": [], "over": []
        }))
        .unwrap()
    }

    #[test]
    fn stamp_puts_the_label_inside_the_face() {
        for l in crate::look::LOOKS {
            for th in [l.dark, l.light] {
                for lang in [Language::Tr, Language::En] {
                    let mut sc = blank(th.bg);
                    stamp(&mut sc, ChangeKind::Strengthened, lang, l);
                    let t = texts(&sc);
                    assert_eq!(t.len(), 1);
                    assert_eq!(t[0].0 .3, label(ChangeKind::Strengthened, lang));
                    let b = t[0].1;
                    assert!(
                        b.left >= 0 && b.top >= 0 && b.right <= 390 && b.bottom <= 120,
                        "{b:?}"
                    );
                    match &sc.over[1] {
                        Prim::Text { color, .. } => assert_eq!(*color, th.text),
                        p => panic!("{p:?}"),
                    }
                }
            }
        }
    }

    #[test]
    fn text_spots_find_the_changed_value_only() {
        let mut a = blank([0, 0, 0, 255]);
        let mut b = blank([0, 0, 0, 255]);
        let t = |s: &str, y: i32| Prim::Text {
            x: 10,
            baseline: y,
            px: 14.0,
            text: s.into(),
            color: [255; 4],
            align: Align::Left,
            font: None,
            role: None,
        };
        a.over = vec![t("same line", 20), t("value 10", 60)];
        b.over = vec![t("same line", 20), t("value 11", 60)];
        let s = text_spots(&a, &b);
        assert_eq!(s.len(), 1);
        assert!(s[0].top > 30, "{s:?}");
    }
}
