//! R4/H-M8910: depth transitions (M10; the `style` gesture's `transition`).
//!
//! A transition design decides, from the scenes before and after a depth change (sentence →
//! capsule → chart → evidence and back), **which elements are shared** (the `Opening.shared`
//! of the answer, `docs/PROTOCOL.md` §4.6.1) and **what each in-between frame shows**: per
//! element a box and an opacity, and the new scene's opacity. The shell paints a frame by
//! copying each element's pixels from the old (or new) scene into the given box at the given
//! opacity; it computes nothing (the progress `p` comes from the core too).
//!
//! The reference `r3` is the T4/T5 transition: the shared elements are the anchors both
//! scenes have ([`depth::transition_anchors`], [`depth::shared_elements`]); a frame moves each
//! element linearly from its before box to its after box at full opacity while the new scene
//! fades in.
//!
//! **Adding an alternative (M10):** in this file only — write the `shared` and `frame`
//! functions below the `M10 alternatives` marker (or `mod <name>;` with
//! `src/transitions/<name>.rs`) and add one line to [`TRANSITIONS`]. A design may reuse
//! [`r3_shared`] / [`r3_frame`] and change only one of them. Element ids are free, but an id
//! that also names a [`depth::transition_anchors`] anchor must mean that anchor.

use std::collections::BTreeMap;

use qp_schema::protocol::{OpenDepth, SharedElement};
use qp_schema::scene::{PixelRect, SceneDesc};

use crate::depth;

/// One side of a transition.
#[derive(Debug, Clone, Copy)]
pub struct Side<'a> {
    /// The depth.
    pub depth: OpenDepth,
    /// Its scene (`None`: the sentence depth, or the reader gets no scene there).
    pub scene: Option<&'a SceneDesc>,
    /// The bar series of the scene's price panel, if any.
    pub price_series: Option<&'a str>,
    /// The capsule's marked bar (its first fact's bar), if any.
    pub marked_bar: Option<u64>,
}

/// Which scene an element's pixels are copied from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// The scene before the transition.
    Before,
    /// The scene after it.
    After,
}

/// One element of an in-between frame, in page CSS px.
#[derive(Debug, Clone, PartialEq)]
pub struct FrameElement {
    /// Shared element id.
    pub id: String,
    /// Where its pixels come from.
    pub source: Source,
    /// Its box inside the source scene (scene px).
    pub src: PixelRect,
    /// Where it is drawn: `[left, top, width, height]` in page CSS px.
    pub rect: [f64; 4],
    /// Opacity, 0..=1.
    pub opacity: f64,
}

/// An in-between frame at progress `p`.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    /// Elements, in paint order.
    pub elements: Vec<FrameElement>,
    /// Opacity of the new scene, 0..=1.
    pub scene_opacity: f64,
}

/// The shared elements of a transition.
pub type SharedFn = fn(before: &Side<'_>, after: &Side<'_>) -> Vec<SharedElement>;

/// A frame at progress `p` (0..=1): `from` is the page position (left, top) of the old
/// scene, `to` of the new one. The element list (count, order, source, `src`) must be the
/// same for every `p`; only `rect` and `opacity` change with `p` (the shell builds the
/// copies once). An element not to be seen gets opacity 0.
pub type FrameFn = fn(shared: &[SharedElement], from: (f64, f64), to: (f64, f64), p: f64) -> Frame;

/// One transition design.
#[derive(Debug, Clone, Copy)]
pub struct TransitionDesign {
    /// Setting name (`[a-z][a-z0-9_-]*`, not an ordinary word).
    pub name: &'static str,
    /// Duration in ms (the core turns the shell's timestamps into `p`).
    pub duration_ms: f64,
    /// The shared elements.
    pub shared: SharedFn,
    /// A frame.
    pub frame: FrameFn,
}

/// The default design.
pub const DEFAULT_TRANSITION: &str = "r3";

/// The anchors of one side (`depth::transition_anchors`), `None` without a scene.
#[must_use]
pub fn anchors(s: &Side<'_>) -> Option<BTreeMap<String, PixelRect>> {
    s.scene
        .map(|sc| depth::transition_anchors(sc, s.price_series, s.marked_bar))
}

/// `r3` shared elements: the anchors both scenes have (T4).
#[must_use]
pub fn r3_shared(before: &Side<'_>, after: &Side<'_>) -> Vec<SharedElement> {
    depth::shared_elements(anchors(before).as_ref(), anchors(after).as_ref())
}

fn lerp(a: f64, b: f64, p: f64) -> f64 {
    a + (b - a) * p
}

fn rect_at(r: &PixelRect, o: (f64, f64)) -> [f64; 4] {
    [
        f64::from(r.left) + o.0,
        f64::from(r.top) + o.1,
        f64::from((r.right - r.left).max(1)),
        f64::from((r.bottom - r.top).max(1)),
    ]
}

/// `r3` frame: every element with both boxes moves linearly from before to after at full
/// opacity (pixels from the old scene); the new scene fades in with `p`.
#[must_use]
pub fn r3_frame(shared: &[SharedElement], from: (f64, f64), to: (f64, f64), p: f64) -> Frame {
    let p = p.clamp(0.0, 1.0);
    let elements = shared
        .iter()
        .filter_map(|s| {
            let (b, a) = (s.before.as_ref()?, s.after.as_ref()?);
            let (rb, ra) = (rect_at(b, from), rect_at(a, to));
            Some(FrameElement {
                id: s.id.clone(),
                source: Source::Before,
                src: *b,
                rect: [
                    lerp(rb[0], ra[0], p),
                    lerp(rb[1], ra[1], p),
                    lerp(rb[2], ra[2], p),
                    lerp(rb[3], ra[3], p),
                ],
                opacity: 1.0,
            })
        })
        .collect();
    Frame {
        elements,
        scene_opacity: p,
    }
}

// --- M10 alternatives: shared / frame functions (or `mod <name>;`) go below this line --

mod tutamir;
mod zumlak;

/// Transition designs a connection may choose; the first is the default.
pub static TRANSITIONS: &[TransitionDesign] = &[
    TransitionDesign {
        name: "r3",
        duration_ms: 260.0,
        shared: r3_shared,
        frame: r3_frame,
    },
    // --- M10 alternatives: one line each
    TransitionDesign {
        name: "tutamir",
        duration_ms: 420.0,
        shared: tutamir::shared,
        frame: tutamir::frame,
    },
    TransitionDesign {
        name: "zumlak",
        duration_ms: 480.0,
        shared: zumlak::shared,
        frame: zumlak::frame,
    },
];

/// Registered names.
#[must_use]
pub fn names() -> Vec<&'static str> {
    TRANSITIONS.iter().map(|t| t.name).collect()
}

/// Whether `name` is a known design.
#[must_use]
pub fn is_transition(name: &str) -> bool {
    TRANSITIONS.iter().any(|t| t.name == name)
}

/// The design named `name` (`None` or unknown: the default).
#[must_use]
pub fn by_name(name: Option<&str>) -> &'static TransitionDesign {
    let n = name.unwrap_or(DEFAULT_TRANSITION);
    TRANSITIONS
        .iter()
        .find(|t| t.name == n)
        .unwrap_or(&TRANSITIONS[0])
}

/// Progress of a transition that started at `t0` ms, at `t` ms (0..=1).
#[must_use]
pub fn progress(name: Option<&str>, t0: f64, t: f64) -> f64 {
    let d = by_name(name).duration_ms;
    if !(t.is_finite() && t0.is_finite()) || d <= 0.0 {
        return 1.0;
    }
    ((t - t0) / d).clamp(0.0, 1.0)
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

    #[test]
    fn r3_frame_interpolates() {
        let sh = vec![
            SharedElement {
                id: "glyph".into(),
                before: Some(r(0, 0, 10, 10)),
                after: Some(r(100, 50, 120, 70)),
            },
            SharedElement {
                id: "headline".into(),
                before: None,
                after: Some(r(0, 0, 5, 5)),
            },
        ];
        let f0 = r3_frame(&sh, (0.0, 0.0), (0.0, 100.0), 0.0);
        assert_eq!(f0.elements.len(), 1);
        assert_eq!(f0.elements[0].rect, [0.0, 0.0, 10.0, 10.0]);
        let f1 = r3_frame(&sh, (0.0, 0.0), (0.0, 100.0), 1.0);
        assert_eq!(f1.elements[0].rect, [100.0, 150.0, 20.0, 20.0]);
        assert!((f1.scene_opacity - 1.0).abs() < 1e-12);
        let fh = r3_frame(&sh, (0.0, 0.0), (0.0, 100.0), 0.5);
        assert_eq!(fh.elements[0].rect, [50.0, 75.0, 15.0, 15.0]);
        assert!((progress(None, 0.0, 130.0) - 0.5).abs() < 1e-12);
        assert_eq!(TRANSITIONS[0].name, DEFAULT_TRANSITION);
    }
}
