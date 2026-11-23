//! M10 alternative `tutamir` (R4): the sentence stays, the mark points.
//!
//! The reader's sentence is the thread through the transition: from the feed it moves, at
//! full strength, from the face into the opened capsule (`headline` is kept); from the
//! capsule to the chart nothing is drawn over the layer's sentence line (the sentence the
//! shell shows above every opened depth), so the same sentence stays put, now as the
//! chart's title line, while the capsule's own copy of it fades with the card. The
//! capsule's mark travels to the place on the chart its
//! sentence is about: the fact's bar × the fact's level, where it lands as a small pin
//! and hands over to the chart's own highlighted evidence, which appears **before** the
//! rest of the chart. Everything else of the old scene fades out calmly; the price panel
//! slides into the chart's price panel while it fades, so the eye keeps "same thing".
//!
//! Elements (`Opening.shared`), on top of `r3`'s (the anchors both scenes have):
//! - `pin` (own id, replaces `glyph` when a capsule opens to the chart): the capsule's mark,
//!   before = its box in the capsule scene, after = a box just above the fact's point on
//!   the chart (bar column centre × level; the level's box or the panel middle);
//! - anchors the new scene has and the old one lacks (`bar:<i>`, `fact:<id>`,
//!   `evidence:<fact>`): after-only, drawn from the new scene early;
//! - `backdrop` (own id): the whole old scene, before-only, fading out.
//!
//! Before-only and after-only elements have one box; they never count as shared in the
//! measures (which need both boxes). Determinism: IEEE basic operations only.

use qp_schema::protocol::{OpenDepth, SharedElement};
use qp_schema::scene::PixelRect;

use super::{anchors, r3_shared, Frame, FrameElement, Side, Source};

/// The capsule's mark as a pin on the chart (scene px): the chart's own mark size.
#[allow(clippy::cast_possible_truncation)]
const PIN_PX: i32 = qp_grammar::depth::CHART_GLYPH_PX as i32;
/// Gap between the pin and the point it marks (scene px).
const PIN_GAP: i32 = 6;

fn is_focus(id: &str) -> bool {
    id.starts_with("bar:") || id.starts_with("fact:") || id.starts_with("evidence:")
}

fn cx(r: &PixelRect) -> f64 {
    f64::from(r.left + r.right) / 2.0
}

fn cy(r: &PixelRect) -> f64 {
    f64::from(r.top + r.bottom) / 2.0
}

/// The point on the after scene the capsule is about: the marked bar's column centre ×
/// the level of the evidence under it (or any fact layer; else the price panel middle).
fn focus_point(a: &std::collections::BTreeMap<String, PixelRect>) -> Option<(f64, f64)> {
    let panel = a.get("price")?;
    let bar = a
        .iter()
        .find(|(k, _)| k.starts_with("bar:"))
        .map(|(_, r)| *r);
    let level = |pre: &str| -> Option<PixelRect> {
        let mut hits = a
            .iter()
            .filter(|(k, _)| k.starts_with(pre))
            .map(|(_, r)| *r);
        let first = hits.next()?;
        let under = bar.and_then(|b| {
            std::iter::once(first)
                .chain(hits)
                .find(|r| f64::from(r.left) <= cx(&b) && cx(&b) <= f64::from(r.right))
        });
        Some(under.unwrap_or(first))
    };
    let lv = level("evidence:").or_else(|| level("fact:"));
    let x = bar.or(lv).map_or(cx(panel), |r| cx(&r));
    let y = lv.map_or(cy(panel), |r| cy(&r));
    let top = f64::from(panel.top) + 14.0;
    let bottom = f64::from(panel.bottom) - 14.0;
    Some((x, y.clamp(top.min(bottom), bottom.max(top))))
}

/// The pin's box over `(x, y)`: above the point, or below it near the panel's top.
#[allow(clippy::cast_possible_truncation)]
fn pin_box(x: f64, y: f64, panel: &PixelRect) -> PixelRect {
    let l = qp_look::detmath::px(x) - PIN_PX / 2;
    let yi = qp_look::detmath::px(y);
    let top = if yi - PIN_GAP - PIN_PX >= panel.top {
        yi - PIN_GAP - PIN_PX
    } else {
        yi + PIN_GAP
    };
    PixelRect {
        left: l,
        top,
        right: l + PIN_PX,
        bottom: top + PIN_PX,
    }
}

/// `tutamir` shared elements (module docs).
#[must_use]
pub fn shared(before: &Side<'_>, after: &Side<'_>) -> Vec<SharedElement> {
    let (Some(b), Some(a), Some(bs)) = (anchors(before), anchors(after), before.scene) else {
        return r3_shared(before, after);
    };
    let to_chart = after.depth >= OpenDepth::Chart && before.depth < OpenDepth::Chart;
    let pin = if to_chart {
        b.get("glyph")
            .copied()
            .zip(focus_point(&a).and_then(|(x, y)| a.get("price").map(|p| pin_box(x, y, p))))
    } else {
        None
    };
    let mut out = vec![SharedElement {
        id: "backdrop".to_owned(),
        before: Some(PixelRect {
            left: 0,
            top: 0,
            right: i32::try_from(bs.width).unwrap_or(i32::MAX),
            bottom: i32::try_from(bs.height).unwrap_or(i32::MAX),
        }),
        after: None,
    }];
    for s in r3_shared(before, after) {
        if pin.is_some() && s.id == "glyph" {
            continue;
        }
        out.push(s);
    }
    for (k, r) in &a {
        if is_focus(k) && !b.contains_key(k) {
            out.push(SharedElement {
                id: k.clone(),
                before: None,
                after: Some(*r),
            });
        }
    }
    if let Some((pb, pa)) = pin {
        out.push(SharedElement {
            id: "pin".to_owned(),
            before: Some(pb),
            after: Some(pa),
        });
    }
    out
}

fn lerp(a: f64, b: f64, p: f64) -> f64 {
    a + (b - a) * p
}

/// `t` over `[a, b]`, clamped to 0..=1 and eased (smoothstep).
fn ramp(t: f64, a: f64, b: f64) -> f64 {
    let u = ((t - a) / (b - a)).clamp(0.0, 1.0);
    u * u * (3.0 - 2.0 * u)
}

fn page(r: &PixelRect, o: (f64, f64)) -> [f64; 4] {
    [
        f64::from(r.left) + o.0,
        f64::from(r.top) + o.1,
        f64::from((r.right - r.left).max(1)),
        f64::from((r.bottom - r.top).max(1)),
    ]
}

fn mix(a: [f64; 4], b: [f64; 4], p: f64) -> [f64; 4] {
    [
        lerp(a[0], b[0], p),
        lerp(a[1], b[1], p),
        lerp(a[2], b[2], p),
        lerp(a[3], b[3], p),
    ]
}

/// Paint rank: backdrop under everything, the mark and the sentence on top.
fn rank(id: &str) -> u8 {
    match id {
        "backdrop" => 0,
        "price" => 1,
        "pin" => 5,
        k if is_focus(k) => 3,
        _ => 2,
    }
}

/// `tutamir` frame (module docs). Timeline over `p`: the rest of the old scene fades out
/// (0–0.3); the price panel slides into place (0–0.6) while fading (0.1–0.55); the new
/// scene's evidence appears first (0.1–0.4), the rest of the new scene after it
/// (0.3–0.95); the mark flies to the fact's point (0–0.6), rests there and fades as the
/// chart takes over (0.8–1); other kept elements move (0–0.6) and fade late (0.75–1).
#[must_use]
pub fn frame(shared: &[SharedElement], from: (f64, f64), to: (f64, f64), p: f64) -> Frame {
    let p = p.clamp(0.0, 1.0);
    let mv = ramp(p, 0.0, 0.6);
    let mut order: Vec<&SharedElement> = shared.iter().collect();
    order.sort_by_key(|s| rank(&s.id));
    let mut elements = Vec::with_capacity(order.len());
    for s in order {
        let (src, source, rect, opacity) = match (s.before.as_ref(), s.after.as_ref()) {
            (Some(b), None) => (*b, Source::Before, page(b, from), 1.0 - ramp(p, 0.0, 0.3)),
            (None, Some(a)) => (*a, Source::After, page(a, to), ramp(p, 0.1, 0.4)),
            (Some(b), Some(a)) if is_focus(&s.id) => (
                *a,
                Source::After,
                mix(page(b, from), page(a, to), ramp(p, 0.0, 0.6)),
                ramp(p, 0.1, 0.4),
            ),
            (Some(b), Some(a)) if s.id == "price" => (
                *b,
                Source::Before,
                mix(page(b, from), page(a, to), mv),
                1.0 - ramp(p, 0.1, 0.55),
            ),
            (Some(b), Some(a)) if s.id == "pin" => (
                *b,
                Source::Before,
                mix(page(b, from), page(a, to), mv),
                1.0 - ramp(p, 0.8, 1.0),
            ),
            (Some(b), Some(a)) => (
                *b,
                Source::Before,
                mix(page(b, from), page(a, to), mv),
                1.0 - ramp(p, 0.75, 1.0),
            ),
            (None, None) => continue,
        };
        elements.push(FrameElement {
            id: s.id.clone(),
            source,
            src,
            rect,
            opacity,
        });
    }
    Frame {
        elements,
        scene_opacity: if p >= 1.0 { 1.0 } else { ramp(p, 0.3, 0.95) },
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

    fn sh(id: &str, b: Option<PixelRect>, a: Option<PixelRect>) -> SharedElement {
        SharedElement {
            id: id.into(),
            before: b,
            after: a,
        }
    }

    #[test]
    fn frame_rule_and_ends() {
        let list = vec![
            sh("pin", Some(r(10, 10, 50, 50)), Some(r(200, 100, 230, 130))),
            sh("backdrop", Some(r(0, 0, 390, 240)), None),
            sh("price", Some(r(0, 60, 390, 240)), Some(r(0, 0, 390, 300))),
            sh("bar:7", None, Some(r(195, 0, 205, 300))),
            sh(
                "headline",
                Some(r(12, 10, 300, 40)),
                Some(r(12, 40, 300, 70)),
            ),
        ];
        let frames: Vec<Frame> = [0.0, 0.33, 0.67, 1.0]
            .iter()
            .map(|&p| frame(&list, (0.0, 100.0), (0.0, 100.0), p))
            .collect();
        // The frame rule: same elements (count, order, source, src) at every p.
        for f in &frames {
            let a: Vec<_> = f
                .elements
                .iter()
                .map(|e| (&e.id, e.source, e.src))
                .collect();
            let b: Vec<_> = frames[0]
                .elements
                .iter()
                .map(|e| (&e.id, e.source, e.src))
                .collect();
            assert_eq!(a, b);
        }
        let ids: Vec<&str> = frames[0].elements.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, ["backdrop", "price", "headline", "bar:7", "pin"]);
        let f0 = &frames[0];
        assert_eq!(f0.elements[4].rect, [10.0, 110.0, 40.0, 40.0]);
        assert!((f0.elements[0].opacity - 1.0).abs() < 1e-12);
        assert!(f0.elements[3].opacity.abs() < 1e-12);
        assert!(f0.scene_opacity.abs() < 1e-12);
        let f1 = &frames[3];
        assert_eq!(f1.elements[4].rect, [200.0, 200.0, 30.0, 30.0]);
        assert!(f1.elements[4].opacity.abs() < 1e-12);
        assert!(f1.elements[2].opacity.abs() < 1e-12);
        assert_eq!(f1.elements[2].rect, [12.0, 140.0, 288.0, 30.0]);
        assert!((f1.elements[3].opacity - 1.0).abs() < 1e-12);
        assert!((f1.scene_opacity - 1.0).abs() < 1e-12);
    }

    #[test]
    fn pin_sits_above_the_point() {
        let panel = r(0, 0, 390, 300);
        assert_eq!(pin_box(100.0, 150.0, &panel), r(84, 112, 116, 144));
        assert_eq!(pin_box(100.0, 20.0, &panel), r(84, 26, 116, 58));
    }
}
