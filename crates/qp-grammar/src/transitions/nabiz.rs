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
use crate::look::Look;
use crate::theme::Theme;

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
    crate::depth::transition_anchors(scene, None, None)
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
    if let Some(text) = m.label {
        #[allow(clippy::cast_possible_truncation)]
        scene.over.push(Prim::Text {
            x: pl.label_at.0,
            baseline: pl.label_at.1,
            px: look.types.label.px as f32,
            text: text.to_owned(),
            color: change_color(m.change, theme),
            align: Align::Left,
            font: look.scene_font("label"),
            role: None,
        });
    }
}

// --- overlay: pulse, beat, trail --------------------------------------------------

/// Pulse period (ms, before the look's duration scale).
pub const PULSE_MS: f64 = 2600.0;
/// Pulse ring: how much it grows over the glyph box, and its peak opacity.
pub const PULSE_GROW: f64 = 0.14;
/// Peak opacity of the pulse ring (faint).
pub const PULSE_ALPHA: f64 = 0.38;
/// Beat length (ms, before the duration scale).
pub const BEAT_MS: f64 = 900.0;
/// Delay of the second ring of a strengthening beat (ms).
pub const BEAT_GAP_MS: f64 = 260.0;
/// Trail length after the beat (ms, before the duration scale).
pub const TRAIL_MS: f64 = 4000.0;
/// Peak opacity of the trail's highlight.
pub const TRAIL_ALPHA: f64 = 0.28;
/// Pad of a ring around the glyph box (px).
const RING_PAD: f64 = 4.0;

/// One overlay box of a frame, in face px (= CSS px of the face).
#[derive(Debug, Clone, PartialEq)]
pub struct OverlayBox {
    /// Stable key within the face (`pulse`, `beat`, `beat2`, `trail`).
    pub key: &'static str,
    /// `[left, top, width, height]`.
    pub rect: [f64; 4],
    /// Opacity, 0..=1.
    pub opacity: f64,
    /// Scale about the box's centre.
    pub scale: f64,
    /// A ring of this colour and width (px), round.
    pub ring: Option<(Rgba, f64)>,
    /// A fill of this colour (with the box's corner radius).
    pub fill: Option<(Rgba, f64)>,
}

/// One beat the device saw on a face: its change and how long ago it started (ms).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Beat {
    /// The change.
    pub change: Change,
    /// Time since it started, ms (≥ 0).
    pub age: f64,
}

/// What one face's overlay knows at a frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FaceMotion {
    /// The face's mark places.
    pub places: Places,
    /// Live (pulses).
    pub live: bool,
    /// Its direction (the pulse's colour).
    pub direction: Direction,
    /// The latest beat, if one is still running (beat or trail).
    pub beat: Option<Beat>,
    /// The label's width (px; the trail's width), 0 without a label.
    pub label_w: i32,
    /// Clock, ms (the pulse's phase).
    pub t: f64,
}

/// Whether a beat of this age still shows (beat or trail), in `look`.
#[must_use]
pub fn beat_running(age: f64, look: &Look) -> bool {
    age < (BEAT_MS + TRAIL_MS) * look.motion.duration_scale
}

fn ring_box(g: &PixelRect) -> [f64; 4] {
    let (cx, cy) = centre(g);
    let side = f64::from((g.right - g.left).max(g.bottom - g.top)) + 2.0 * RING_PAD;
    [cx - side / 2.0, cy - side / 2.0, side, side]
}

/// The overlay boxes of one face at its frame (empty: nothing to paint). Pure.
#[must_use]
pub fn overlay(f: &FaceMotion, look: &Look, theme: &Theme) -> Vec<OverlayBox> {
    let k = look.motion.duration_scale.max(0.01);
    let ease = look.motion.ease;
    let ring = ring_box(&f.places.glyph);
    let line = look.surface.line.max(1.0);
    let mut out = Vec::new();
    // A running beat takes the glyph (the pulse waits until it ends).
    let beating = f.beat.is_some_and(|b| b.age < BEAT_MS * k);
    if f.live && !beating {
        let period = PULSE_MS * k;
        let u = (f.t / period).rem_euclid(1.0);
        let w = 4.0 * u * (1.0 - u);
        let w = w * w;
        out.push(OverlayBox {
            key: "pulse",
            rect: ring,
            opacity: PULSE_ALPHA * w,
            scale: 1.0 + PULSE_GROW * w,
            ring: Some((pulse_color(f.direction, theme), 1.5 * line)),
            fill: None,
        });
    }
    if let Some(b) = f.beat {
        let c = change_color(Some(b.change), theme);
        let d = BEAT_MS * k;
        let p = ease.apply(b.age / d);
        let fade_out = 1.0 - p;
        if b.age < d {
            match b.change {
                Change::Born => out.push(OverlayBox {
                    key: "beat",
                    rect: ring,
                    opacity: 0.9 * fade_out,
                    scale: 1.0 + 1.0 * p,
                    ring: Some((pulse_color(f.direction, theme), 2.0 * line)),
                    fill: None,
                }),
                Change::Strengthened => {
                    let col = pulse_color(f.direction, theme);
                    let q =
                        ease.apply(((b.age - BEAT_GAP_MS * k) / (d - BEAT_GAP_MS * k)).max(0.0));
                    let first = ease.apply((b.age / (d - BEAT_GAP_MS * k)).min(1.0));
                    out.push(OverlayBox {
                        key: "beat",
                        rect: ring,
                        opacity: 0.85 * (1.0 - first),
                        scale: 1.0 + 0.6 * first,
                        ring: Some((col, 2.0 * line)),
                        fill: None,
                    });
                    out.push(OverlayBox {
                        key: "beat2",
                        rect: ring,
                        opacity: if b.age < BEAT_GAP_MS * k {
                            0.0
                        } else {
                            0.85 * (1.0 - q)
                        },
                        scale: 1.0 + 0.6 * q,
                        ring: Some((col, 2.0 * line)),
                        fill: None,
                    });
                }
                Change::Weakened | Change::Invalid => {
                    let strong = b.change == Change::Invalid;
                    // Inwards: from wide and faint to the glyph's size, then gone.
                    let a = if strong { 0.95 } else { 0.5 };
                    let rise = (p / 0.6).min(1.0);
                    let fall = ((p - 0.85) / 0.15).clamp(0.0, 1.0);
                    out.push(OverlayBox {
                        key: "beat",
                        rect: ring,
                        opacity: a * rise * (1.0 - fall),
                        scale: if strong { 2.0 } else { 1.6 }
                            - (if strong { 1.1 } else { 0.6 }) * p,
                        ring: Some((c, if strong { 3.0 } else { 1.5 } * line)),
                        fill: None,
                    });
                }
                Change::Closed => out.push(OverlayBox {
                    key: "beat",
                    rect: ring,
                    opacity: 0.45 * fade_out,
                    scale: 1.0,
                    ring: None,
                    fill: Some((theme.closed, ring[2] / 2.0)),
                }),
            }
        }
        if f.label_w > 0 {
            // The trail: a highlight over the label from the beat's start, fading out
            // over the trail's length after the beat.
            let t_end = (BEAT_MS + TRAIL_MS) * k;
            let fade = ((b.age - d) / (t_end - d)).clamp(0.0, 1.0);
            let pad = f64::from(look.space.scale[0]);
            let (top, bottom) = f.places.row;
            let x = f64::from(f.places.label_at.0) - pad;
            out.push(OverlayBox {
                key: "trail",
                rect: [
                    x,
                    f64::from(top) - pad / 2.0,
                    f64::from(f.label_w) + 2.0 * pad,
                    f64::from(bottom - top) + pad,
                ],
                opacity: TRAIL_ALPHA * (1.0 - ease.apply(fade)),
                scale: 1.0,
                ring: None,
                fill: Some((
                    match b.change {
                        Change::Invalid | Change::Weakened => theme.invalid,
                        _ => theme.accent,
                    },
                    look.surface.pill_radius,
                )),
            });
        }
    }
    out
}

/// The label's drawn width in `look` (px): the trail's width.
#[must_use]
pub fn label_width(text: &str, look: &Look) -> i32 {
    look.role_font("label").width(text, look.types.label.px)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::look::{ORNEK, R4};
    use qp_schema::scene::XMap;

    fn r(l: i32, t: i32, rr: i32, b: i32) -> PixelRect {
        PixelRect {
            left: l,
            top: t,
            right: rr,
            bottom: b,
        }
    }

    fn sh() -> Vec<SharedElement> {
        vec![
            SharedElement {
                id: "glyph".into(),
                before: Some(r(8, 34, 40, 66)),
                after: Some(r(20, 40, 60, 80)),
            },
            SharedElement {
                id: WHOLE_IN.into(),
                before: Some(r(0, 0, 390, 188)),
                after: None,
            },
            SharedElement {
                id: WHOLE_AFTER.into(),
                before: None,
                after: Some(r(0, 0, 390, 600)),
            },
        ]
    }

    #[test]
    fn frame_is_a_deepening_cross_fade() {
        let s = sh();
        let f0 = frame(&s, (0.0, 100.0), (0.0, 50.0), 0.0);
        assert_eq!(f0.elements.len(), 2);
        // Same list at every p.
        for p in [0.25, 0.5, 0.75, 1.0] {
            let f = frame(&s, (0.0, 100.0), (0.0, 50.0), p);
            assert_eq!(f.elements.len(), 2);
            assert_eq!(f.elements[0].id, f0.elements[0].id);
        }
        // p = 0: old at its place, opaque; new invisible.
        assert_eq!(f0.elements[0].rect, [0.0, 100.0, 390.0, 188.0]);
        assert!((f0.elements[0].opacity - 1.0).abs() < 1e-12);
        assert!(f0.elements[1].opacity.abs() < 1e-12);
        assert!(f0.scene_opacity.abs() < 1e-12);
        // p = 1: new at its place, opaque; real scene shown; old gone.
        let f1 = frame(&s, (0.0, 100.0), (0.0, 50.0), 1.0);
        assert_eq!(f1.elements[1].rect, [0.0, 50.0, 390.0, 600.0]);
        assert!(f1.elements[0].opacity.abs() < 1e-12);
        assert!((f1.scene_opacity - 1.0).abs() < 1e-12);
        // In between: the old scene grew about its glyph (nothing flies: the glyph's
        // centre stays where it was).
        let fm = frame(&s, (0.0, 100.0), (0.0, 50.0), 0.5);
        let e = &fm.elements[0];
        assert!(e.rect[2] > 390.0);
        let gx = 24.0; // glyph centre x in the old scene
        let k = e.rect[2] / 390.0;
        assert!((e.rect[0] + gx * k - gx).abs() < 1e-9);
        // Determinism.
        assert_eq!(fm, frame(&s, (0.0, 100.0), (0.0, 50.0), 0.5));
    }

    #[test]
    fn closing_shrinks() {
        let mut s = sh();
        s[1].id = WHOLE_OUT.into();
        let f = frame(&s, (0.0, 0.0), (0.0, 0.0), 0.5);
        assert!(f.elements[0].rect[2] < 390.0);
        assert!(f.elements[1].rect[2] > 390.0);
    }

    #[test]
    fn changes() {
        use Stage::*;
        assert_eq!(
            Change::between(None, (Born, Validity::Valid)),
            Some(Change::Born)
        );
        assert_eq!(Change::between(None, (Strengthened, Validity::Valid)), None);
        assert_eq!(
            Change::between(Some((Born, Validity::Valid)), (Born, Validity::Valid)),
            None
        );
        assert_eq!(
            Change::between(Some((Born, Validity::Valid)), (Confirmed, Validity::Valid)),
            Some(Change::Strengthened)
        );
        assert_eq!(
            Change::between(Some((Born, Validity::Valid)), (Born, Validity::NoLonger)),
            Some(Change::Invalid)
        );
        assert_eq!(
            Change::between(Some((Born, Validity::Valid)), (Realized, Validity::Closed)),
            Some(Change::Closed)
        );
        assert!(is_live(Born, Validity::Valid));
        assert!(!is_live(Born, Validity::NoLonger));
        assert!(!is_live(Invalidated, Validity::Closed));
        assert_eq!(label(Born, Validity::Valid, Language::Tr), Some("doğdu"));
        assert_eq!(
            label(Weakened, Validity::NoLonger, Language::En),
            Some("no longer holds")
        );
    }

    fn scene() -> SceneDesc {
        SceneDesc {
            view: "v".into(),
            width: 390,
            height: 188,
            background: [0, 0, 0, 255],
            x: XMap {
                from_bar: 0,
                to_bar: 0,
                left: 8.0,
                slot: 1.0,
            },
            bars: 1,
            lod: None,
            panels: vec![],
            emphasis: vec![],
            under: vec![],
            over: vec![],
            terms: vec![],
        }
    }

    fn places_r4() -> Places {
        Places {
            glyph: r(8, 34, 40, 66),
            dot: (11.0, 17.0, 3.0),
            label_at: (20, 21),
            row: (10, 24),
        }
    }

    #[test]
    fn overlay_pulse_beat_trail() {
        let th = R4.dark;
        let mut fm = FaceMotion {
            places: places_r4(),
            live: true,
            direction: Direction::Up,
            beat: None,
            label_w: 40,
            t: 0.0,
        };
        // Pulse: invisible at phase 0, strongest at half the period.
        let a = overlay(&fm, &R4, &th);
        assert_eq!(a.len(), 1);
        assert!(a[0].opacity.abs() < 1e-12);
        fm.t = PULSE_MS / 2.0;
        let b = overlay(&fm, &R4, &th);
        assert!((b[0].opacity - PULSE_ALPHA).abs() < 1e-12);
        // Not live: still.
        fm.live = false;
        assert!(overlay(&fm, &R4, &th).is_empty());
        // Beats have distinct forms.
        let at = |c: Change, age: f64| {
            let mut f = fm;
            f.beat = Some(Beat { change: c, age });
            overlay(&f, &R4, &th)
        };
        let born = at(Change::Born, 450.0);
        let strong = at(Change::Strengthened, 450.0);
        let inv = at(Change::Invalid, 450.0);
        assert!(
            born[0].scale > 1.0 && born.iter().filter(|b| b.key.starts_with("beat")).count() == 1
        );
        assert_eq!(
            strong.iter().filter(|b| b.key.starts_with("beat")).count(),
            2
        );
        assert!(inv[0].scale > 1.0 && at(Change::Invalid, 800.0)[0].scale < inv[0].scale);
        assert!(born[0].scale < at(Change::Born, 800.0)[0].scale);
        // Trail: present after the beat, gone after the trail.
        let tr = at(Change::Born, BEAT_MS + 100.0);
        assert!(tr.iter().any(|b| b.key == "trail" && b.opacity > 0.0));
        assert!(beat_running(BEAT_MS + 100.0, &R4));
        assert!(!beat_running(BEAT_MS + TRAIL_MS, &R4));
        // Determinism.
        assert_eq!(at(Change::Weakened, 300.0), at(Change::Weakened, 300.0));
    }

    fn lum(c: [f64; 3]) -> f64 {
        let ch = |v: f64| {
            let v = v / 255.0;
            if v <= 0.040_45 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * ch(c[0]) + 0.7152 * ch(c[1]) + 0.0722 * ch(c[2])
    }

    fn contrast(fg: Rgba, bg: Rgba) -> f64 {
        let a = f64::from(fg[3]) / 255.0;
        let mix = |i: usize| f64::from(bg[i]) + a * (f64::from(fg[i]) - f64::from(bg[i]));
        let (l1, l2) = (
            lum([mix(0), mix(1), mix(2)]),
            lum([0, 1, 2].map(|i| f64::from(bg[i]))),
        );
        (l1.max(l2) + 0.05) / (l1.min(l2) + 0.05)
    }

    /// The still marks keep the thresholds in the reference and the trial look: the change
    /// label (small text) 4.5:1, the live dot (a non-text mark) 3:1, on the scene's
    /// background, in both schemes.
    #[test]
    fn still_marks_keep_contrast() {
        let mut bad = Vec::new();
        for look in [&R4, &ORNEK] {
            for t in [look.dark, look.light] {
                for c in [
                    None,
                    Some(Change::Born),
                    Some(Change::Strengthened),
                    Some(Change::Weakened),
                    Some(Change::Invalid),
                    Some(Change::Closed),
                ] {
                    let k = contrast(change_color(c, &t), t.bg);
                    if k < 4.5 {
                        bad.push(format!("{} {} label {c:?} {k:.2}", look.name, t.name));
                    }
                }
                for d in [Direction::Up, Direction::Down, Direction::Neutral] {
                    let k = contrast(pulse_color(d, &t), t.bg);
                    if k < 3.0 {
                        bad.push(format!("{} {} dot {d:?} {k:.2}", look.name, t.name));
                    }
                }
            }
        }
        assert!(bad.is_empty(), "{bad:#?}");
    }

    #[test]
    fn still_marks_from_the_look() {
        let mut s = scene();
        let m = FaceMarks {
            live: true,
            label: Some("doğdu"),
            change: Some(Change::Born),
            direction: Direction::Up,
        };
        // No glyph: nothing drawn.
        draw_face(&mut s, &m, &R4, &R4.dark);
        assert!(s.over.is_empty());
        let _ = ORNEK;
    }
}
