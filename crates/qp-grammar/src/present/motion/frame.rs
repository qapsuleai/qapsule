//! A change's frame at progress `p` ([`change_frame`]).

use super::*;

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
