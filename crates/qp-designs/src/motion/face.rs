//! What a change on a face covers ([`FaceChange`], [`face_change`]) and how long it runs.

use super::*;

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

pub(super) fn texts(sc: &SceneDesc) -> Vec<(TextKey, PixelRect)> {
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
pub(super) fn text_spots(before: &SceneDesc, after: &SceneDesc) -> Vec<PixelRect> {
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
        qp_grammar::depth::transition_anchors(sc, None, None)
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
