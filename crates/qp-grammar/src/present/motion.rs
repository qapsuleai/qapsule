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

mod face;
mod frame;

pub use face::{change_duration, change_progress, face_change, FaceChange, MAX_SPOTS, PLATE};
pub use frame::change_frame;

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
    use super::face::{text_spots, texts};
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
