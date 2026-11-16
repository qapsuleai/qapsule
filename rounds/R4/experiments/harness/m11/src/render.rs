//! Drawing one glyph (copied from the R3 harness, drawing through the outcome style):
//! `OutcomeStyle::draw(alphabet, …)` → scene primitives → the project CPU rasterizer
//! (`qp_render::render_scene_rgba`, the same path as the golden images and the `glyph`
//! mark).

use qp_grammar::glyph::outcome::OutcomeStyle;
use qp_grammar::glyph::{Alphabet, GlyphBox};
use qp_grammar::theme::Theme;
use qp_render::scene::scene_of;
use qp_schema::capsule::GlyphSpec;
use qp_schema::scene::Prim;

/// The primitives of `spec` drawn by `style` with `alphabet` into a `size`×`size` box at
/// the origin.
#[must_use]
pub fn glyph_prims(
    style: &dyn OutcomeStyle,
    alphabet: &dyn Alphabet,
    spec: &GlyphSpec,
    size: u32,
    theme: &Theme,
) -> (Vec<Prim>, GlyphBox) {
    #[allow(clippy::cast_precision_loss)]
    let bx = GlyphBox {
        x: 0.0,
        y: 0.0,
        size: size as f32,
    };
    (style.draw(alphabet, spec, bx, theme), bx)
}

/// Straight RGBA8 pixels of `spec` drawn into a `size`×`size` box on the theme background.
///
/// # Errors
/// Rasterizer failure.
pub fn glyph_rgba(
    style: &dyn OutcomeStyle,
    alphabet: &dyn Alphabet,
    spec: &GlyphSpec,
    size: u32,
    theme: &Theme,
) -> Result<Vec<u8>, String> {
    let (prims, _) = glyph_prims(style, alphabet, spec, size, theme);
    qp_render::render_scene_rgba(&scene_of(size, size, theme.bg, prims)).map_err(|e| e.to_string())
}

/// Number of primitives whose geometry leaves the box (text is not checked: its extent
/// depends on the font). The glyph contract requires 0.
#[must_use]
pub fn out_of_box(prims: &[Prim], bx: GlyphBox) -> usize {
    let (x0, y0, x1, y1) = (bx.x, bx.y, bx.x + bx.size, bx.y + bx.size);
    let tol = 0.01;
    let inside = |x: f32, y: f32| x >= x0 - tol && y >= y0 - tol && x <= x1 + tol && y <= y1 + tol;
    prims
        .iter()
        .filter(|p| match p {
            #[allow(clippy::cast_precision_loss)]
            Prim::Rect { x, y, w, h, .. } => {
                !(inside(*x as f32, *y as f32)
                    && inside((*x as f32) + *w as f32, (*y as f32) + *h as f32))
            }
            Prim::Polyline { points, width, .. } => {
                let hw = width / 2.0;
                points
                    .iter()
                    .any(|[x, y]| !(inside(x - hw, y - hw) && inside(x + hw, y + hw)))
            }
            Prim::Fill { points, .. } => points.iter().any(|[x, y]| !inside(*x, *y)),
            Prim::Circle { cx, cy, r, .. } => !(inside(cx - r, cy - r) && inside(cx + r, cy + r)),
            Prim::Text { .. } => false,
        })
        .count()
}

/// Nearest-neighbour upscale of straight RGBA8 by an integer factor.
#[must_use]
pub fn upscale(rgba: &[u8], w: u32, h: u32, k: u32) -> Vec<u8> {
    let (w, h, k) = (w as usize, h as usize, k as usize);
    let mut out = vec![0u8; w * k * h * k * 4];
    for y in 0..h * k {
        for x in 0..w * k {
            let s = ((y / k) * w + x / k) * 4;
            let d = (y * w * k + x) * 4;
            out[d..d + 4].copy_from_slice(&rgba[s..s + 4]);
        }
    }
    out
}

/// Copies a `w`×`h` RGBA8 tile into a `cw`-wide canvas at `(x, y)`.
pub fn blit(canvas: &mut [u8], cw: u32, tile: &[u8], w: u32, h: u32, x: u32, y: u32) {
    let (cw, w, h, x, y) = (cw as usize, w as usize, h as usize, x as usize, y as usize);
    for row in 0..h {
        let s = row * w * 4;
        let d = ((y + row) * cw + x) * 4;
        canvas[d..d + w * 4].copy_from_slice(&tile[s..s + w * 4]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upscale_nearest() {
        let px = [1, 2, 3, 4, 5, 6, 7, 8];
        let u = upscale(&px, 2, 1, 2);
        assert_eq!(u.len(), 4 * 2 * 4);
        assert_eq!(&u[0..8], &[1, 2, 3, 4, 1, 2, 3, 4]);
        assert_eq!(&u[16..24], &[1, 2, 3, 4, 1, 2, 3, 4]);
    }

    #[test]
    fn box_check() {
        let bx = GlyphBox {
            x: 0.0,
            y: 0.0,
            size: 16.0,
        };
        let c = [0, 0, 0, 255];
        let inside = Prim::Circle {
            cx: 8.0,
            cy: 8.0,
            r: 7.0,
            color: c,
        };
        let outside = Prim::Circle {
            cx: 8.0,
            cy: 8.0,
            r: 9.0,
            color: c,
        };
        assert_eq!(out_of_box(&[inside, outside], bx), 1);
    }
}
