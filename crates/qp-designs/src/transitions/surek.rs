//! (R5/M21) Transition `surek`: continuity — the same object changes place.
//!
//! Opening a capsule from the feed, its glyph and its headline leave their place in the
//! face and travel to their place in the open capsule; the rest of the face dissolves where
//! it was (so the travelling pieces are visibly *taken out of* that card), and the capsule's
//! scene comes up around them only once they are on their way. Going down from the capsule
//! to the chart the glyph flies to its own bar (the chart's glyph anchor over the capsule's
//! marked bar), the fact geometry with it. Closing is the same move backwards (the scenes
//! swap roles; the design is symmetric).
//!
//! Shared elements: the anchors both scenes have (`depth::transition_anchors`) except
//! `price` (two different panels: stretching one into the other is not "the same object")
//! and `evidence:*` (the evidence ink is the chart's own); one one-sided entry
//! `scene:before` (the whole old scene, before box only) carries the card that dissolves.
//!
//! Frames: a travelling element follows a smooth step over `p ∈ [0, TRAVEL]` (position and
//! size together, so a glyph grows from the face's size to the card's); the old scene fades
//! out in place over `[0, DISSOLVE]`; the new scene fades in over `[SCENE_IN.0,
//! SCENE_IN.1]`. The glyph is painted last (on top). The stage fractions are the design's
//! own ratios, not durations: the duration is [`DURATION_MS`] times the look's
//! `motion.duration_scale`, through the look's easing curve (`transitions::progress_in`).
//! Every in-between frame is a pure function of the shared boxes and `p`, so the harness
//! can freeze any `p` (`window.qp.freeze`).

use qp_schema::protocol::SharedElement;
use qp_schema::scene::PixelRect;

use super::{anchors, Frame, FrameElement, Side, Source};

/// (R5/M21) The design's own duration, ms (≤ 300 ms, `RESEARCH.md` F14); the look scales
/// it. Kayda alınmalı: a look field for the opening's duration (today: the design's).
pub const DURATION_MS: f64 = 300.0;

/// The whole old scene (before box only).
pub const WHOLE_BEFORE: &str = "scene:before";

/// Share of the progress over which the travelling pieces move.
const TRAVEL: f64 = 0.8;
/// Share of the progress over which the old scene dissolves in place.
const DISSOLVE: f64 = 0.3;
/// The new scene fades in over this part of the progress.
const SCENE_IN: (f64, f64) = (0.3, 0.85);

/// Whether anchor `id` travels.
fn travels(id: &str) -> bool {
    id != "price" && !id.starts_with("evidence:")
}

/// `surek` shared elements (module docs).
#[must_use]
pub fn shared(before: &Side<'_>, after: &Side<'_>) -> Vec<SharedElement> {
    let (ab, aa) = (anchors(before), anchors(after));
    let mut out: Vec<SharedElement> = Vec::new();
    if let (Some(ab), Some(aa)) = (&ab, &aa) {
        for (k, rb) in ab {
            if !travels(k) {
                continue;
            }
            if let Some(ra) = aa.get(k) {
                out.push(SharedElement {
                    id: k.clone(),
                    before: Some(*rb),
                    after: Some(*ra),
                });
            }
        }
    }
    if out.is_empty() {
        // Nothing travels: the reference (the anchors both have, a plain cross-fade).
        return super::r3_shared(before, after);
    }
    out.push(SharedElement {
        id: WHOLE_BEFORE.to_owned(),
        before: before.scene.map(|sc| PixelRect {
            left: 0,
            top: 0,
            right: i32::try_from(sc.width).unwrap_or(i32::MAX),
            bottom: i32::try_from(sc.height).unwrap_or(i32::MAX),
        }),
        after: None,
    });
    out
}

/// 0 below `a`, 1 above `b`, smooth in between.
pub(crate) fn ramp(p: f64, a: f64, b: f64) -> f64 {
    let t = ((p - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn rect_at(r: &PixelRect, o: (f64, f64)) -> [f64; 4] {
    [
        f64::from(r.left) + o.0,
        f64::from(r.top) + o.1,
        f64::from((r.right - r.left).max(1)),
        f64::from((r.bottom - r.top).max(1)),
    ]
}

fn lerp4(a: [f64; 4], b: [f64; 4], t: f64) -> [f64; 4] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
        a[3] + (b[3] - a[3]) * t,
    ]
}

/// `surek` frame (module docs). The element list is the same for every `p`: the old
/// scene first, then the travelling pieces in id order with the glyph last.
#[must_use]
pub fn frame(shared: &[SharedElement], from: (f64, f64), to: (f64, f64), p: f64) -> Frame {
    let p = p.clamp(0.0, 1.0);
    let mut elements = Vec::new();
    if let Some(s) = shared.iter().find(|s| s.id == WHOLE_BEFORE) {
        if let Some(b) = &s.before {
            elements.push(FrameElement {
                id: s.id.clone(),
                source: Source::Before,
                src: *b,
                rect: rect_at(b, from),
                opacity: 1.0 - ramp(p, 0.0, DISSOLVE),
            });
        }
    }
    let t = ramp(p, 0.0, TRAVEL);
    let travelling = |glyph: bool| {
        shared
            .iter()
            .filter(move |s| (s.id == "glyph") == glyph && s.id != WHOLE_BEFORE)
            .filter_map(move |s| {
                let (b, a) = (s.before.as_ref()?, s.after.as_ref()?);
                Some(FrameElement {
                    id: s.id.clone(),
                    source: Source::Before,
                    src: *b,
                    rect: lerp4(rect_at(b, from), rect_at(a, to), t),
                    opacity: 1.0,
                })
            })
    };
    elements.extend(travelling(false));
    elements.extend(travelling(true));
    Frame {
        elements,
        scene_opacity: ramp(p, SCENE_IN.0, SCENE_IN.1),
    }
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

    fn sample() -> Vec<SharedElement> {
        vec![
            SharedElement {
                id: "glyph".into(),
                before: Some(r(8, 8, 40, 40)),
                after: Some(r(20, 30, 60, 70)),
            },
            SharedElement {
                id: "headline".into(),
                before: Some(r(48, 8, 300, 30)),
                after: Some(r(70, 30, 380, 56)),
            },
            SharedElement {
                id: WHOLE_BEFORE.into(),
                before: Some(r(0, 0, 390, 120)),
                after: None,
            },
        ]
    }

    #[test]
    fn same_list_every_frame_and_ends_where_the_new_scene_has_them() {
        let sh = sample();
        let (from, to) = ((0.0, 200.0), (0.0, 60.0));
        let ids = |f: &Frame| f.elements.iter().map(|e| e.id.clone()).collect::<Vec<_>>();
        let f0 = frame(&sh, from, to, 0.0);
        assert_eq!(ids(&f0), ["scene:before", "headline", "glyph"]);
        for p in [0.1, 0.25, 0.5, 0.75, 1.0] {
            let f = frame(&sh, from, to, p);
            assert_eq!(ids(&f), ids(&f0));
            for (a, b) in f.elements.iter().zip(&f0.elements) {
                assert_eq!(a.src, b.src);
            }
        }
        // p = 0: where the face has them; the new scene is not seen yet.
        assert_eq!(f0.elements[2].rect, [8.0, 208.0, 32.0, 32.0]);
        assert!(f0.scene_opacity.abs() < 1e-12);
        assert!((f0.elements[0].opacity - 1.0).abs() < 1e-12);
        // p = 1: where the open capsule has them; the old card is gone.
        let f1 = frame(&sh, from, to, 1.0);
        assert_eq!(f1.elements[2].rect, [20.0, 90.0, 40.0, 40.0]);
        assert!((f1.scene_opacity - 1.0).abs() < 1e-12);
        assert!(f1.elements[0].opacity.abs() < 1e-12);
    }

    #[test]
    fn a_quarter_in_the_pieces_are_on_their_way_and_the_card_is_going() {
        let sh = sample();
        let f = frame(&sh, (0.0, 200.0), (0.0, 60.0), 0.25);
        let g = &f.elements[2];
        // Strictly between the two places (not at either end).
        assert!(g.rect[1] < 208.0 && g.rect[1] > 90.0, "{:?}", g.rect);
        // The card the pieces came out of is almost gone (no ghost of them left behind).
        assert!(f.elements[0].opacity < 0.2);
        let early = frame(&sh, (0.0, 200.0), (0.0, 60.0), 0.1);
        assert!(early.elements[0].opacity > 0.5);
        assert!(f.scene_opacity.abs() < 1e-12);
    }

    #[test]
    fn price_and_evidence_do_not_travel() {
        assert!(!travels("price"));
        assert!(!travels("evidence:f1"));
        assert!(travels("glyph") && travels("bar:12") && travels("fact:f1"));
    }
}
