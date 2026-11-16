//! M10 alternative `zumlak`: the opening as a camera move.
//!
//! The small price line of the capsule and the chart's price panel draw the same
//! coordinates (bar × price). The design finds the part of that coordinate plane both
//! scenes show (`span`: the same window, one box in each scene) and turns the two boxes into
//! one camera: at `p = 0` the old scene is where it was and the new scene is scaled and
//! placed so that its copy of the window lies exactly on the old one; at `p = 1` the new
//! scene is where it belongs and the old one is scaled with it. In between the scale moves
//! geometrically (a zoom feels even that way) and the capsule's fact point moves on a
//! straight line from its old page position to its new one, so the marked moment stays
//! under the eye while the rest of the chart grows around it. Only the old price panel
//! moves with the camera (the capsule's words stay where they were and fade, so no text is
//! magnified); the new scene fades in; the chart's column of the marked bar is drawn at full opacity from the first
//! frame (the moment the chart grows out of), and the glyph flies from its place in the
//! capsule to its place over the bar. Closing is the same move backwards (a zoom out).
//!
//! Shared elements (the measured ones, both boxes): the anchors both scenes have except
//! `price` (the panels differ outside the common window; `span` is the part that is the
//! same data), plus `span`. Two one-sided entries carry the whole scenes to the frame
//! function (`scene:before`, `pane:before` = the old price panel, `scene:after`; one box each, so they are not counted as
//! shared). Without a common window (no price panel on a side, no scene) the frame is the
//! reference `r3` frame.

use qp_schema::protocol::SharedElement;
use qp_schema::scene::{PixelRect, SceneDesc, ScenePanel};
use qp_schema::{scene::DataSource, Mark};

use super::{anchors, r3_frame, Frame, FrameElement, Side, Source};
use crate::{detmath, scale};

/// The common window id.
const SPAN: &str = "span";
/// The whole old scene (before box only).
const WHOLE_BEFORE: &str = "scene:before";
/// The old scene's price panel (before box only).
const PANE_BEFORE: &str = "pane:before";
/// The whole new scene (after box only).
const WHOLE_AFTER: &str = "scene:after";

/// The panel drawing `series` as candles or a line (the anchors' `price` rule).
fn price_panel<'a>(sc: &'a SceneDesc, series: Option<&str>) -> Option<&'a ScenePanel> {
    let series = series?;
    sc.panels.iter().find(|p| {
        p.layers.iter().any(|l| {
            matches!(l.mark, Mark::Candle | Mark::Line)
                && matches!(&l.data, DataSource::Series { series: s } if s == series)
        })
    })
}

/// Bar coordinate (continuous; slot `b` spans `[b, b + 1)`) at pixel x `px`.
#[allow(clippy::cast_precision_loss)]
fn bar_at(sc: &SceneDesc, px: f64) -> f64 {
    sc.x.from_bar as f64 + (px - sc.x.left) / sc.x.slot
}

/// Pixel x of bar coordinate `u`.
#[allow(clippy::cast_precision_loss)]
fn bar_x(sc: &SceneDesc, u: f64) -> f64 {
    sc.x.left + (u - sc.x.from_bar as f64) * sc.x.slot
}

#[allow(clippy::cast_possible_truncation)]
fn px(v: f64) -> i32 {
    v.round() as i32
}

/// The window of the bar × price plane both price panels show, as a box in each scene.
fn common_window(b: &Side<'_>, a: &Side<'_>) -> Option<(PixelRect, PixelRect)> {
    let (sb, sa) = (b.scene?, a.scene?);
    let (pb, pa) = (
        price_panel(sb, b.price_series)?,
        price_panel(sa, a.price_series)?,
    );
    if !(sb.x.slot > 0.0 && sa.x.slot > 0.0) {
        return None;
    }
    let seen = |sc: &SceneDesc, p: &ScenePanel| {
        let (u0, u1) = (
            bar_at(sc, f64::from(p.rect.left)),
            bar_at(sc, f64::from(p.rect.right)),
        );
        let (v0, v1) = (
            scale::value_at(&p.y, f64::from(p.rect.bottom)),
            scale::value_at(&p.y, f64::from(p.rect.top)),
        );
        (u0, u1, v0.min(v1), v0.max(v1))
    };
    let (bu0, bu1, bv0, bv1) = seen(sb, pb);
    let (au0, au1, av0, av1) = seen(sa, pa);
    let (u0, u1) = (bu0.max(au0), bu1.min(au1));
    let (v0, v1) = (bv0.max(av0), bv1.min(av1));
    if !(u1 > u0 && v1 > v0) {
        return None;
    }
    let rect = |sc: &SceneDesc, p: &ScenePanel| -> Option<PixelRect> {
        let (top, bottom) = (scale::value_y(&p.y, v1)?, scale::value_y(&p.y, v0)?);
        let r = PixelRect {
            left: px(bar_x(sc, u0)),
            top: px(top),
            right: px(bar_x(sc, u1)),
            bottom: px(bottom),
        };
        (r.right - r.left >= 4 && r.bottom - r.top >= 4).then_some(r)
    };
    Some((rect(sb, pb)?, rect(sa, pa)?))
}

fn whole(sc: &SceneDesc) -> PixelRect {
    PixelRect {
        left: 0,
        top: 0,
        right: i32::try_from(sc.width).unwrap_or(i32::MAX),
        bottom: i32::try_from(sc.height).unwrap_or(i32::MAX),
    }
}

/// `zumlak` shared elements (module docs).
#[must_use]
pub fn shared(before: &Side<'_>, after: &Side<'_>) -> Vec<SharedElement> {
    let Some((wb, wa)) = common_window(before, after) else {
        return super::r3_shared(before, after);
    };
    let (ab, aa) = (anchors(before), anchors(after));
    let mut out: Vec<SharedElement> = Vec::new();
    if let (Some(ab), Some(aa)) = (&ab, &aa) {
        for (k, rb) in ab {
            if k == "price" {
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
    out.push(SharedElement {
        id: SPAN.to_owned(),
        before: Some(wb),
        after: Some(wa),
    });
    out.push(SharedElement {
        id: WHOLE_BEFORE.to_owned(),
        before: before.scene.map(whole),
        after: None,
    });
    out.push(SharedElement {
        id: PANE_BEFORE.to_owned(),
        before: before
            .scene
            .and_then(|sc| price_panel(sc, before.price_series))
            .map(|p| p.rect),
        after: None,
    });
    out.push(SharedElement {
        id: WHOLE_AFTER.to_owned(),
        before: None,
        after: after.scene.map(whole),
    });
    out
}

fn w(r: &PixelRect) -> f64 {
    f64::from((r.right - r.left).max(1))
}

fn h(r: &PixelRect) -> f64 {
    f64::from((r.bottom - r.top).max(1))
}

fn centre(r: &PixelRect) -> (f64, f64) {
    (
        (f64::from(r.left) + f64::from(r.right)) / 2.0,
        (f64::from(r.top) + f64::from(r.bottom)) / 2.0,
    )
}

/// 0 below `a`, 1 above `b`, smooth in between.
fn ramp(p: f64, a: f64, b: f64) -> f64 {
    let t = ((p - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// `k^e` for `k > 0` (deterministic).
fn pow(k: f64, e: f64) -> f64 {
    detmath::log10(k).map_or(1.0, |l| detmath::pow10(e * l))
}

/// The camera at one progress: old-scene point → page point.
struct Camera {
    /// Old → new scene scale per axis (`span` after / before).
    k: (f64, f64),
    /// Old → new scene offset: `new = off + old · k`.
    off: (f64, f64),
    /// Focus in old-scene px.
    f: (f64, f64),
    /// Focus on the page now.
    q: (f64, f64),
    /// Scale now (old-scene px → page px).
    s: (f64, f64),
}

impl Camera {
    fn new(
        span: (&PixelRect, &PixelRect),
        f: (f64, f64),
        from: (f64, f64),
        to: (f64, f64),
        e: f64,
    ) -> Self {
        let (b, a) = span;
        let k = (w(a) / w(b), h(a) / h(b));
        let off = (
            f64::from(a.left) - f64::from(b.left) * k.0,
            f64::from(a.top) - f64::from(b.top) * k.1,
        );
        let f_new = (off.0 + f.0 * k.0, off.1 + f.1 * k.1);
        let q = (
            (from.0 + f.0) + ((to.0 + f_new.0) - (from.0 + f.0)) * e,
            (from.1 + f.1) + ((to.1 + f_new.1) - (from.1 + f.1)) * e,
        );
        Self {
            k,
            off,
            f,
            q,
            s: (pow(k.0, e), pow(k.1, e)),
        }
    }

    /// Page box of an old-scene box.
    fn old(&self, r: &PixelRect) -> [f64; 4] {
        [
            self.q.0 + (f64::from(r.left) - self.f.0) * self.s.0,
            self.q.1 + (f64::from(r.top) - self.f.1) * self.s.1,
            w(r) * self.s.0,
            h(r) * self.s.1,
        ]
    }

    /// Page box of a new-scene box.
    fn new_box(&self, r: &PixelRect) -> [f64; 4] {
        let x = (f64::from(r.left) - self.off.0) / self.k.0;
        let y = (f64::from(r.top) - self.off.1) / self.k.1;
        [
            self.q.0 + (x - self.f.0) * self.s.0,
            self.q.1 + (y - self.f.1) * self.s.1,
            w(r) / self.k.0 * self.s.0,
            h(r) / self.k.1 * self.s.1,
        ]
    }
}

fn find<'a>(shared: &'a [SharedElement], id: &str) -> Option<&'a SharedElement> {
    shared.iter().find(|s| s.id == id)
}

fn both(s: &SharedElement) -> Option<(&PixelRect, &PixelRect)> {
    Some((s.before.as_ref()?, s.after.as_ref()?))
}

/// `zumlak` frame (module docs). Paint order: old scene (in place), old price panel
/// (camera), new scene (camera), the marked bar's column
/// of the new scene, the glyph.
#[must_use]
pub fn frame(shared: &[SharedElement], from: (f64, f64), to: (f64, f64), p: f64) -> Frame {
    let p = p.clamp(0.0, 1.0);
    let Some(span) = find(shared, SPAN).and_then(both) else {
        return r3_frame(shared, from, to, p);
    };
    let e = ramp(p, 0.0, 1.0);
    // Focus: the fact point of the old scene, else the marked bar's column at the
    // window's middle height, else the window's centre.
    let fact = shared
        .iter()
        .find(|s| s.id.starts_with("fact:"))
        .and_then(both);
    let bar = shared
        .iter()
        .find(|s| s.id.starts_with("bar:"))
        .and_then(both);
    let f = match (fact, bar) {
        (Some((b, _)), _) => centre(b),
        (None, Some((b, _))) => (centre(b).0, centre(span.0).1),
        _ => centre(span.0),
    };
    let cam = Camera::new(span, f, from, to, e);
    let mut elements = Vec::new();
    if let Some(r) = find(shared, WHOLE_BEFORE).and_then(|s| s.before) {
        elements.push(FrameElement {
            id: WHOLE_BEFORE.to_owned(),
            source: Source::Before,
            src: r,
            rect: [
                f64::from(r.left) + from.0,
                f64::from(r.top) + from.1,
                w(&r),
                h(&r),
            ],
            opacity: 1.0 - ramp(p, 0.15, 0.6),
        });
    }
    if let Some(r) = find(shared, PANE_BEFORE).and_then(|s| s.before) {
        elements.push(FrameElement {
            id: PANE_BEFORE.to_owned(),
            source: Source::Before,
            src: r,
            rect: cam.old(&r),
            opacity: 1.0 - ramp(p, 0.35, 0.8),
        });
    }
    if let Some(r) = find(shared, WHOLE_AFTER).and_then(|s| s.after) {
        elements.push(FrameElement {
            id: WHOLE_AFTER.to_owned(),
            source: Source::After,
            src: r,
            rect: cam.new_box(&r),
            opacity: ramp(p, 0.05, 0.55),
        });
    }
    if let Some((s, (_, a))) = shared
        .iter()
        .find(|s| s.id.starts_with("bar:"))
        .and_then(|s| both(s).map(|x| (s, x)))
    {
        elements.push(FrameElement {
            id: s.id.clone(),
            source: Source::After,
            src: *a,
            rect: cam.new_box(a),
            opacity: 1.0,
        });
    }
    if let Some((b, a)) = find(shared, "glyph").and_then(both) {
        let (rb, ra) = (
            [
                f64::from(b.left) + from.0,
                f64::from(b.top) + from.1,
                w(b),
                h(b),
            ],
            [
                f64::from(a.left) + to.0,
                f64::from(a.top) + to.1,
                w(a),
                h(a),
            ],
        );
        let mut rect = [0.0; 4];
        for (i, v) in rect.iter_mut().enumerate() {
            *v = rb[i] + (ra[i] - rb[i]) * e;
        }
        elements.push(FrameElement {
            id: "glyph".to_owned(),
            source: Source::Before,
            src: *b,
            rect,
            opacity: 1.0,
        });
    }
    Frame {
        elements,
        scene_opacity: ramp(p, 0.85, 1.0),
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

    fn el(id: &str, b: Option<PixelRect>, a: Option<PixelRect>) -> SharedElement {
        SharedElement {
            id: id.into(),
            before: b,
            after: a,
        }
    }

    fn close(x: [f64; 4], y: [f64; 4]) -> bool {
        x.iter().zip(y).all(|(a, b)| (a - b).abs() < 1e-6)
    }

    #[test]
    fn camera_ends_and_frame_rule() {
        let sh = vec![
            el(
                "bar:7",
                Some(r(100, 50, 104, 90)),
                Some(r(200, 0, 216, 300)),
            ),
            el("glyph", Some(r(0, 0, 20, 20)), Some(r(190, 10, 222, 42))),
            el(SPAN, Some(r(50, 50, 150, 90)), Some(r(0, 0, 400, 300))),
            el(WHOLE_BEFORE, Some(r(0, 0, 390, 240)), None),
            el(PANE_BEFORE, Some(r(40, 40, 160, 100)), None),
            el(WHOLE_AFTER, None, Some(r(0, 0, 390, 600))),
        ];
        let (from, to) = ((0.0, 100.0), (0.0, 60.0));
        let f0 = frame(&sh, from, to, 0.0);
        let f1 = frame(&sh, from, to, 1.0);
        let fm = frame(&sh, from, to, 0.4);
        // The same elements, sources and boxes at every p.
        let ids = |f: &Frame| {
            f.elements
                .iter()
                .map(|e| (e.id.clone(), e.source, e.src))
                .collect::<Vec<_>>()
        };
        assert_eq!(ids(&f0), ids(&f1));
        assert_eq!(ids(&f0), ids(&fm));
        assert_eq!(f0.elements.len(), 5);
        // p = 0: the old scene where it was; the new scene's window on the old window.
        assert!(close(f0.elements[0].rect, [0.0, 100.0, 390.0, 240.0]));
        assert!(close(f0.elements[1].rect, [40.0, 140.0, 120.0, 60.0]));
        let new0 = f0.elements[2].rect; // new scene 390×600, k = (4, 7.5)
        assert!(close(new0, [50.0, 150.0, 97.5, 80.0]));
        // p = 1: the new scene where it belongs; the old window on the new window.
        assert!(close(f1.elements[2].rect, [0.0, 60.0, 390.0, 600.0]));
        // The old scene stays in place; its price panel lands on the new window's scale.
        assert!(close(f1.elements[0].rect, [0.0, 100.0, 390.0, 240.0]));
        assert!(close(f1.elements[1].rect, [-40.0, -15.0, 480.0, 450.0]));
        assert!((f1.scene_opacity - 1.0).abs() < 1e-12);
        assert!(f0.scene_opacity.abs() < 1e-12);
        // The glyph lands on its chart box.
        assert!(close(f1.elements[4].rect, [190.0, 70.0, 32.0, 32.0]));
    }

    #[test]
    fn without_window_is_r3() {
        let sh = vec![el("glyph", Some(r(0, 0, 10, 10)), Some(r(5, 5, 15, 15)))];
        assert_eq!(
            frame(&sh, (0.0, 0.0), (0.0, 0.0), 0.5),
            r3_frame(&sh, (0.0, 0.0), (0.0, 0.0), 0.5)
        );
    }
}
