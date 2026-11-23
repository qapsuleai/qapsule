//! (RP3/T2) The paint list: a scene handed to a paint path that does not take pixels
//! from the core (`shells/web/paint/README.md`). It is the [`SceneDesc`]'s primitives in
//! paint order (`SceneDesc::prims`), scaled to the device scale with the reference
//! renderer's own rule (`qp_render::cpu::scale_prim`), every number a painter needs
//! given (colours as CSS `#rrggbbaa`, the font as a CSS font string, the full turn of a
//! circle), so that the painter only calls drawing functions: no arithmetic, no layout,
//! no text building, no sorting.

use qp_schema::scene::{Align, Prim, Rgba, SceneDesc};
use serde_json::{json, Value};

use super::ShellCore;

/// Version of the paint list format.
pub const PAINT_LIST_VERSION: u32 = 1;
/// The CSS font family a paint path registers the core's default font under
/// (`Shell::font`). (R5/T3) Every embedded asset has its own (`look::FontAsset::css_family`,
/// [`text_font`]).
pub const PAINT_FONT_FAMILY: &str = "QpMono";

/// (R5/T3) The font a text prim names (`Prim::Text::font`; absent or unknown: the default):
/// its index in `qp_render::fonts::FILES`, the parsed file and the CSS family the canvas
/// path registered it under.
#[must_use]
pub fn text_font(id: Option<&str>) -> Option<(usize, &'static fontdue::Font, &'static str)> {
    let i = qp_render::fonts::index(id);
    let font = qp_render::fonts::by_index(i).ok()?;
    let family = qp_look::look::font(qp_render::fonts::FILES[i].0)
        .map_or(PAINT_FONT_FAMILY, |f| f.css_family);
    Some((i, font, family))
}
/// The miter limit of the reference renderer's strokes (tiny-skia's default; the
/// canvas default is 10).
pub const PAINT_MITER_LIMIT: f32 = 4.0;

fn css(c: Rgba) -> String {
    format!("#{:02x}{:02x}{:02x}{:02x}", c[0], c[1], c[2], c[3])
}

fn prim_json(p: &Prim) -> Value {
    match p {
        Prim::Rect { x, y, w, h, color } => {
            json!({"k": "rect", "x": x, "y": y, "w": w, "h": h, "c": css(*color)})
        }
        Prim::Polyline {
            points,
            width,
            color,
        } => json!({"k": "line", "pts": points, "lw": width, "c": css(*color)}),
        Prim::Fill { points, color } => json!({"k": "fill", "pts": points, "c": css(*color)}),
        Prim::Circle { cx, cy, r, color } => json!({"k": "circle", "x": cx, "y": cy, "r": r,
            "a1": std::f64::consts::TAU, "c": css(*color)}),
        Prim::Text {
            x,
            baseline,
            px,
            text,
            color,
            align,
            font,
            ..
        } => json!({"k": "text", "x": x, "y": baseline, "t": text,
            "font": format!("{px}px {}", text_font(font.as_deref()).map_or(PAINT_FONT_FAMILY, |f| f.2)),
            "align": match align { Align::Left => "left", Align::Right => "right" },
            "c": css(*color)}),
    }
}

/// The paint list of `scene` at the integer device scale `s` (JSON, format v1:
/// `{v, w, h, scale, bg, miter, prims}`; `shells/web/paint/README.md`).
#[must_use]
pub fn paint_list(scene: &SceneDesc, s: u32) -> Value {
    let prims: Vec<Value> = scene
        .prims()
        .map(|p| prim_json(&qp_render::cpu::scale_prim(p, s)))
        .collect();
    json!({
        "v": PAINT_LIST_VERSION,
        "w": scene.width.saturating_mul(s),
        "h": scene.height.saturating_mul(s),
        "scale": s,
        "bg": css(scene.background),
        "miter": PAINT_MITER_LIMIT,
        "prims": prims,
    })
}

impl ShellCore {
    /// (RP3/T2) The paint list of capsule `id`'s face at the device scale, or `None`
    /// when it has not changed since the last hand-out (same short-circuit and same
    /// record as [`ShellCore::face_rgba`]) or has no scene.
    pub fn face_paint(&mut self, id: &str, force: bool) -> Option<Value> {
        let scene = self.face(id)?;
        if !self.face_changed(id, &scene, force) {
            return None;
        }
        Some(paint_list(&scene, self.scale))
    }

    /// (RP3/T2) The paint list of the open capsule's scene (`None` when unchanged or
    /// none; as [`ShellCore::scene_rgba`]).
    pub fn scene_paint(&mut self, force: bool) -> Option<Value> {
        let Some(scene) = self.scene() else {
            self.scene_shown = None;
            return None;
        };
        if !self.scene_changed(&scene, force) {
            return None;
        }
        Some(paint_list(&scene, self.scale))
    }
}
