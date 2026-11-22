//! The partial repaint region: primitive bounds and the changed area between two scenes
//! (RG1: moved out of `paint_canvas.rs`).

use super::*;

/// Conservative device-pixel bounds of everything `p` may paint (anti-aliasing, miter
/// joins and the browser's own glyph shapes included).
pub(super) fn bounds(p: &Prim) -> Option<Bounds> {
    #[allow(clippy::cast_precision_loss)]
    let pts_box = |pts: &[[f32; 2]], m: f32| -> Option<Bounds> {
        let mut b = [
            f32::INFINITY,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NEG_INFINITY,
        ];
        for &[x, y] in pts {
            if !(x.is_finite() && y.is_finite()) {
                continue;
            }
            b = [b[0].min(x), b[1].min(y), b[2].max(x), b[3].max(y)];
        }
        b[0].is_finite()
            .then_some([b[0] - m, b[1] - m, b[2] + m, b[3] + m])
    };
    #[allow(clippy::cast_precision_loss)]
    match p {
        Prim::Rect { x, y, w, h, .. } => Some([
            *x as f32,
            *y as f32,
            *x as f32 + *w as f32,
            *y as f32 + *h as f32,
        ]),
        Prim::Polyline { points, width, .. } => {
            let w = if width.is_finite() {
                width.max(1.0)
            } else {
                1.0
            };
            pts_box(points, w * MITER * 0.5 + 2.0)
        }
        Prim::Fill { points, .. } => pts_box(points, 2.0),
        Prim::Circle { cx, cy, r, .. } => {
            Some([cx - r - 2.0, cy - r - 2.0, cx + r + 2.0, cy + r + 2.0])
        }
        Prim::Text {
            x,
            baseline,
            px,
            text,
            align,
            font,
            ..
        } => {
            let font = super::paint::text_font(font.as_deref())?.1;
            let adv: f32 = text
                .chars()
                .map(|ch| font.metrics(ch, *px).advance_width)
                .sum();
            let (x, y) = (*x as f32, *baseline as f32);
            let (x0, x1) = match align {
                Align::Left => (x, x + adv),
                Align::Right => (x - adv, x),
            };
            let m = px * 0.5 + 2.0;
            Some([x0 - m, y - px * 1.25 - 2.0, x1 + m, y + px * 0.5 + 2.0])
        }
    }
}

pub(super) fn meets(a: &Bounds, b: &Bounds) -> bool {
    a[0] < b[2] && b[0] < a[2] && a[1] < b[3] && b[1] < a[3]
}

/// The region (whole device pixels, inside the canvas) where `prev` and `next` differ,
/// compared position by position; `None` when nothing differs.
pub(super) fn changed(prev: &[Prim], next: &[Prim], w: f32, h: f32) -> Option<Bounds> {
    let mut d = [
        f32::INFINITY,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NEG_INFINITY,
    ];
    let n = prev.len().max(next.len());
    for i in 0..n {
        let (a, b) = (prev.get(i), next.get(i));
        if a == b {
            continue;
        }
        for p in [a, b].into_iter().flatten() {
            // A primitive without usable bounds may paint anywhere.
            let r = bounds(p).unwrap_or([0.0, 0.0, w, h]);
            d = [
                d[0].min(r[0]),
                d[1].min(r[1]),
                d[2].max(r[2]),
                d[3].max(r[3]),
            ];
        }
    }
    if !d[0].is_finite() {
        return None;
    }
    let r = [
        d[0].floor().max(0.0),
        d[1].floor().max(0.0),
        d[2].ceil().min(w),
        d[3].ceil().min(h),
    ];
    Some(r)
}
