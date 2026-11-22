//! `nabiz`'s face overlay over time: the pulse, the beat and the trail.

use super::*;

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
