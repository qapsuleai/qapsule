//! (RP3/M12c) The GPU paint path's core half: a scene turned into **triangles** ready for
//! WebGPU (`shells/web/paint/gpu.js` uploads them and issues draw calls; it computes
//! nothing).
//!
//! The same primitives as the reference renderer, in the same paint order
//! (`SceneDesc::prims`, scaled with `qp_render::cpu::scale_prim`), become:
//!
//! - **vertices** ([`VERTEX_BYTES`] each: clip-space `x, y` as `f32`, premultiplied
//!   colour as 4 × `u8`, atlas texel `u, v` as `f32`). Every vertex samples the glyph
//!   atlas; shapes point at a reserved fully covered texel, so one shader draws shapes and
//!   text in one batch;
//! - **draws**: `(kind, first vertex, vertex count)`. [`DRAW_PLAIN`] is a run of
//!   triangles drawn as they are (opaque or non-overlapping geometry, glyph quads);
//!   [`DRAW_STENCIL_FILL`] writes a non-zero winding count into the stencil buffer,
//!   [`DRAW_STENCIL_UNION`] marks covered samples (a translucent stroke whose pieces
//!   overlap at joins), and [`DRAW_COVER`] then paints the primitive's bounding box where
//!   the stencil is set (and clears it): "stencil, then cover", so a translucent shape is
//!   blended exactly once per sample, as in the reference renderer;
//! - a **glyph atlas** (`R8`, [`ATLAS_W`] × [`ATLAS_H`]): fontdue coverage of each glyph at
//!   each size, the same bitmaps the reference renderer composites, placed with the same
//!   rule (pen rounded to whole pixels), so text quads sample them 1:1.
//!
//! Anti-aliasing of lines, polygons and circles comes from the GPU's 4× multisampling
//! (K12: MSAA only); pixel-aligned rectangles and glyph quads cover whole pixels and
//! need none. Output is compared with the reference renderer under a tolerance, never
//! byte for byte (K12).

use std::collections::BTreeMap;

use qp_schema::scene::{Align, Prim, Rgba, SceneDesc};

use super::ShellCore;

/// Bytes per vertex: `x, y` (`f32` clip space), colour (`unorm8x4`, premultiplied),
/// `u, v` (`f32` atlas texels). `gpu.js` declares the same layout.
pub const VERTEX_BYTES: usize = 20;
/// A run of triangles drawn as they are.
pub const DRAW_PLAIN: u32 = 0;
/// Triangles that add their winding (front +1, back −1) to the stencil (non-zero fill).
pub const DRAW_STENCIL_FILL: u32 = 1;
/// Triangles that mark every sample they cover in the stencil (union; strokes).
pub const DRAW_STENCIL_UNION: u32 = 2;
/// A box painted where the stencil is non-zero; it clears the stencil it paints.
pub const DRAW_COVER: u32 = 3;
/// Glyph atlas width (texels; a multiple of 256 so a row is a valid WebGPU copy pitch).
pub const ATLAS_W: u32 = 2048;
/// Glyph atlas height (texels).
pub const ATLAS_H: u32 = 1024;
/// The reserved fully covered atlas texel shapes sample (top-left 2 × 2 block).
const SOLID_UV: [f32; 2] = [0.5, 0.5];
/// Gap between glyphs in the atlas (texels).
const GAP: u32 = 1;
/// The reference renderer's miter limit (tiny-skia's default).
const MITER_LIMIT: f32 = 4.0;
/// Largest distance (device px) between a circle and its polygon.
const CIRCLE_TOLERANCE: f32 = 0.05;

/// A glyph placed in the atlas: fontdue metrics and its texel origin.
#[derive(Clone, Copy)]
struct Glyph {
    xmin: i32,
    ymin: i32,
    w: u32,
    h: u32,
    advance: f32,
    ax: u32,
    ay: u32,
}

/// The glyph atlas: a shelf packer over an `R8` texture kept in memory, with the rows
/// changed since the last hand-out ([`GpuPainter::atlas_dirty`]).
struct Atlas {
    pixels: Vec<u8>,
    glyphs: BTreeMap<(usize, char, u32), Glyph>,
    shelf_x: u32,
    shelf_y: u32,
    shelf_h: u32,
    dirty: Option<(u32, u32)>,
    generation: u32,
}

impl Atlas {
    fn new(generation: u32) -> Self {
        let mut pixels = vec![0u8; (ATLAS_W * ATLAS_H) as usize];
        for y in 0..2 {
            for x in 0..2 {
                pixels[(y * ATLAS_W + x) as usize] = 255;
            }
        }
        Self {
            pixels,
            glyphs: BTreeMap::new(),
            shelf_x: 2 + GAP,
            shelf_y: 0,
            shelf_h: 2,
            dirty: Some((0, ATLAS_H)),
            generation,
        }
    }

    fn mark(&mut self, y0: u32, y1: u32) {
        self.dirty = Some(match self.dirty {
            Some((a, b)) => (a.min(y0), b.max(y1)),
            None => (y0, y1),
        });
    }

    /// The glyph of `ch` at `size` px, rasterised and placed on first use; `None` when the
    /// atlas is full (the caller starts a new atlas and builds the frame again).
    fn glyph(&mut self, (fi, font): (usize, &fontdue::Font), ch: char, size: f32) -> Option<Glyph> {
        if let Some(g) = self.glyphs.get(&(fi, ch, size.to_bits())) {
            return Some(*g);
        }
        let (m, cov) = font.rasterize(ch, size);
        let (w, h) = (m.width as u32, m.height as u32);
        if w + GAP > ATLAS_W || h + GAP > ATLAS_H {
            return None;
        }
        if self.shelf_x + w + GAP > ATLAS_W {
            self.shelf_y += self.shelf_h + GAP;
            self.shelf_x = 0;
            self.shelf_h = 0;
        }
        if self.shelf_y + h > ATLAS_H {
            return None;
        }
        let (ax, ay) = (self.shelf_x, self.shelf_y);
        for row in 0..h {
            let dst = ((ay + row) * ATLAS_W + ax) as usize;
            let src = (row * w) as usize;
            self.pixels[dst..dst + w as usize].copy_from_slice(&cov[src..src + w as usize]);
        }
        self.mark(ay, ay + h);
        self.shelf_x += w + GAP;
        self.shelf_h = self.shelf_h.max(h);
        let g = Glyph {
            xmin: m.xmin,
            ymin: m.ymin,
            w,
            h,
            advance: m.advance_width,
            ax,
            ay,
        };
        self.glyphs.insert((fi, ch, size.to_bits()), g);
        Some(g)
    }
}

/// One built frame: vertex bytes and draws (`kind, first, count` triples).
#[derive(Default)]
struct Frame {
    verts: Vec<u8>,
    n: u32,
    draws: Vec<u32>,
    bg: [f64; 4],
    // Clip-space mapping: x' = x·sx − 1, y' = 1 − y·sy.
    sx: f32,
    sy: f32,
}

/// The atlas ran out of room while building a frame.
struct AtlasFull;

impl Frame {
    fn new(w: u32, h: u32, bg: Rgba) -> Self {
        let f = |c: u8| f64::from(c) / 255.0;
        let a = f(bg[3]);
        Self {
            // Premultiplied clear colour.
            bg: [f(bg[0]) * a, f(bg[1]) * a, f(bg[2]) * a, a],
            sx: 2.0 / w.max(1) as f32,
            sy: 2.0 / h.max(1) as f32,
            ..Self::default()
        }
    }

    fn vertex(&mut self, [x, y]: [f32; 2], c: [u8; 4], [u, v]: [f32; 2]) {
        let cx = x * self.sx - 1.0;
        let cy = 1.0 - y * self.sy;
        self.verts.extend_from_slice(&cx.to_le_bytes());
        self.verts.extend_from_slice(&cy.to_le_bytes());
        self.verts.extend_from_slice(&c);
        self.verts.extend_from_slice(&u.to_le_bytes());
        self.verts.extend_from_slice(&v.to_le_bytes());
        self.n += 1;
    }

    fn tri(&mut self, a: [f32; 2], b: [f32; 2], c: [f32; 2], col: [u8; 4]) {
        self.vertex(a, col, SOLID_UV);
        self.vertex(b, col, SOLID_UV);
        self.vertex(c, col, SOLID_UV);
    }

    /// Adds the vertices from `first` to now as one draw of `kind`; consecutive plain
    /// runs merge into one draw.
    fn draw(&mut self, kind: u32, first: u32) {
        let count = self.n - first;
        if count == 0 {
            return;
        }
        let k = self.draws.len();
        if kind == DRAW_PLAIN && k >= 3 && self.draws[k - 3] == DRAW_PLAIN {
            self.draws[k - 1] += count;
            return;
        }
        self.draws.extend_from_slice(&[kind, first, count]);
    }

    /// Paints `col` over the bounding box `[x0, y0, x1, y1]` where the stencil is set.
    fn cover(&mut self, b: [f32; 4], col: [u8; 4]) {
        let first = self.n;
        let [x0, y0, x1, y1] = [b[0] - 1.0, b[1] - 1.0, b[2] + 1.0, b[3] + 1.0];
        self.tri([x0, y0], [x1, y0], [x1, y1], col);
        self.tri([x0, y0], [x1, y1], [x0, y1], col);
        self.draw(DRAW_COVER, first);
    }
}

/// Exact `round(x / 255)` for `x <= 255 * 255` (as the reference compositor).
fn div255(x: u32) -> u32 {
    let x = x + 128;
    (x + (x >> 8)) >> 8
}

/// Straight `#rrggbbaa` → premultiplied bytes, with the alpha scaled by `coverage`
/// (`0..=1`, tiny-skia's thin-stroke modulation).
fn premul(c: Rgba, coverage: f32) -> [u8; 4] {
    let a = if coverage >= 1.0 {
        u32::from(c[3])
    } else {
        (f32::from(c[3]) * coverage.max(0.0) + 0.5) as u32
    };
    let p = |v: u8| div255(u32::from(v) * a) as u8;
    [p(c[0]), p(c[1]), p(c[2]), a as u8]
}

fn sub(a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
    [a[0] - b[0], a[1] - b[1]]
}

fn cross(a: [f32; 2], b: [f32; 2]) -> f32 {
    a[0] * b[1] - a[1] * b[0]
}

/// `points` without consecutive duplicates and non-finite points.
fn clean(points: &[[f32; 2]], closed: bool) -> Vec<[f32; 2]> {
    let mut out: Vec<[f32; 2]> = Vec::with_capacity(points.len());
    for &p in points {
        if !(p[0].is_finite() && p[1].is_finite()) {
            return Vec::new();
        }
        if out.last() != Some(&p) {
            out.push(p);
        }
    }
    if closed && out.len() > 1 && out.first() == out.last() {
        out.pop();
    }
    out
}

/// Convex and simple: every turn has the same sign and each axis changes direction at
/// most twice (a star polygon also turns one way, but its x direction flips more).
fn convex(p: &[[f32; 2]]) -> bool {
    let n = p.len();
    if n < 3 {
        return false;
    }
    let (mut sign, mut fx, mut fy) = (0.0f32, 0, 0);
    let (mut lx, mut ly) = (0.0f32, 0.0f32);
    for i in 0..n {
        let a = p[i];
        let b = p[(i + 1) % n];
        let c = p[(i + 2) % n];
        let e = sub(b, a);
        let z = cross(e, sub(c, b));
        if z != 0.0 {
            if sign != 0.0 && z.signum() != sign {
                return false;
            }
            sign = z.signum();
        }
        if e[0] != 0.0 {
            if lx != 0.0 && e[0].signum() != lx {
                fx += 1;
            }
            lx = e[0].signum();
        }
        if e[1] != 0.0 {
            if ly != 0.0 && e[1].signum() != ly {
                fy += 1;
            }
            ly = e[1].signum();
        }
    }
    sign != 0.0 && fx <= 2 && fy <= 2
}

fn bounds(p: &[[f32; 2]], pad: f32) -> [f32; 4] {
    let mut b = [f32::MAX, f32::MAX, f32::MIN, f32::MIN];
    for q in p {
        b[0] = b[0].min(q[0]);
        b[1] = b[1].min(q[1]);
        b[2] = b[2].max(q[0]);
        b[3] = b[3].max(q[1]);
    }
    [b[0] - pad, b[1] - pad, b[2] + pad, b[3] + pad]
}

/// A polygon (non-zero winding): a fan when convex (drawn as it is), otherwise stencil
/// then cover.
fn fill(f: &mut Frame, pts: &[[f32; 2]], col: [u8; 4]) {
    let p = clean(pts, true);
    if p.len() < 3 {
        return;
    }
    let first = f.n;
    for i in 1..p.len() - 1 {
        f.tri(p[0], p[i], p[i + 1], col);
    }
    if convex(&p) {
        f.draw(DRAW_PLAIN, first);
    } else {
        f.draw(DRAW_STENCIL_FILL, first);
        f.cover(bounds(&p, 0.0), col);
    }
}

/// A stroke as tiny-skia draws it: butt caps, miter joins with limit 4 (bevel beyond),
/// widths of at most 1 px as a 1 px line with the alpha scaled by the width.
fn stroke(f: &mut Frame, pts: &[[f32; 2]], width: f32, c: Rgba) {
    if !(width.is_finite() && width >= 0.0) {
        return;
    }
    let (w, coverage) = if width == 0.0 {
        (1.0, 1.0)
    } else if width <= 1.0 {
        (1.0, width)
    } else {
        (width, 1.0)
    };
    let col = premul(c, coverage);
    let p = clean(pts, false);
    if p.len() < 2 {
        return;
    }
    let h = w / 2.0;
    let first = f.n;
    let mut prev: Option<([f32; 2], [f32; 2])> = None; // (unit direction, unit normal)
    let mut outline: Vec<[f32; 2]> = Vec::new();
    for s in 0..p.len() - 1 {
        let (a, b) = (p[s], p[s + 1]);
        let d = sub(b, a);
        let len = (d[0] * d[0] + d[1] * d[1]).sqrt();
        if len == 0.0 || !len.is_finite() {
            continue;
        }
        let u = [d[0] / len, d[1] / len];
        let n = [-u[1] * h, u[0] * h];
        if let Some((u0, n0)) = prev {
            join(f, a, (u0, n0), (u, n), col, &mut outline);
        }
        let (a0, a1) = ([a[0] + n[0], a[1] + n[1]], [a[0] - n[0], a[1] - n[1]]);
        let (b0, b1) = ([b[0] + n[0], b[1] + n[1]], [b[0] - n[0], b[1] - n[1]]);
        f.tri(a0, b0, b1, col);
        f.tri(a0, b1, a1, col);
        outline.extend_from_slice(&[a0, a1, b0, b1]);
        prev = Some((u, n));
    }
    if f.n == first {
        return;
    }
    // Opaque pieces may overlap (each sample gets the same colour); a translucent
    // stroke of one segment cannot overlap itself; otherwise mark, then cover once.
    if col[3] == 255 || f.n - first == 6 {
        f.draw(DRAW_PLAIN, first);
    } else {
        f.draw(DRAW_STENCIL_UNION, first);
        f.cover(bounds(&outline, 0.0), col);
    }
}

/// The outer join at vertex `v` between a segment (direction `u0`, half-width normal
/// `n0`) and the next (`u1`, `n1`).
fn join(
    f: &mut Frame,
    v: [f32; 2],
    (u0, n0): ([f32; 2], [f32; 2]),
    (u1, n1): ([f32; 2], [f32; 2]),
    col: [u8; 4],
    outline: &mut Vec<[f32; 2]>,
) {
    let z = cross(u0, u1);
    let dot = u0[0] * u1[0] + u0[1] * u1[1];
    if z.abs() < 1e-6 {
        // Straight on (nothing to fill) or a full reversal (the miter is infinite and
        // the bevel degenerate: tiny-skia draws nothing either).
        return;
    }
    // The outer side is opposite to the turn.
    let s = if z > 0.0 { -1.0 } else { 1.0 };
    let a = [v[0] + s * n0[0], v[1] + s * n0[1]];
    let b = [v[0] + s * n1[0], v[1] + s * n1[1]];
    // Skia's rule: miter while 1 / sin(half angle) ≤ limit, sin(half) = √((1 + cos θ)/2)
    // with θ the angle between the normals (= between the directions).
    let sin_half = ((1.0 + dot) / 2.0).max(0.0).sqrt();
    if sin_half * MITER_LIMIT >= 1.0 {
        let k = s / (1.0 + dot);
        let m = [v[0] + (n0[0] + n1[0]) * k, v[1] + (n0[1] + n1[1]) * k];
        f.tri(v, a, m, col);
        f.tri(v, m, b, col);
        outline.push(m);
    } else {
        f.tri(v, a, b, col);
    }
    outline.extend_from_slice(&[a, b]);
}

fn circle(f: &mut Frame, cx: f32, cy: f32, r: f32, col: [u8; 4]) {
    if !(cx.is_finite() && cy.is_finite() && r.is_finite() && r > 0.0) {
        return;
    }
    // Segments so that the chord's distance to the arc stays below the tolerance.
    let step = 2.0 * (1.0 - (CIRCLE_TOLERANCE / r).min(1.0)).acos();
    let n = ((std::f32::consts::TAU / step.max(1e-3)).ceil() as u32).clamp(8, 256);
    let first = f.n;
    let at = |k: u32| {
        let t = std::f32::consts::TAU * k as f32 / n as f32;
        [cx + r * t.cos(), cy + r * t.sin()]
    };
    for k in 0..n {
        f.tri([cx, cy], at(k), at(k + 1), col);
    }
    f.draw(DRAW_PLAIN, first);
}

fn rect(f: &mut Frame, x: i32, y: i32, w: u32, h: u32, col: [u8; 4]) {
    if w == 0 || h == 0 {
        return;
    }
    let (x0, y0) = (x as f32, y as f32);
    let (x1, y1) = (x0 + w as f32, y0 + h as f32);
    let first = f.n;
    f.tri([x0, y0], [x1, y0], [x1, y1], col);
    f.tri([x0, y0], [x1, y1], [x0, y1], col);
    f.draw(DRAW_PLAIN, first);
}

/// One text line, placed exactly as the reference renderer's `draw_text`: the advance
/// sum for right alignment, the pen rounded to whole pixels, glyph boxes from fontdue's
/// metrics.
fn text(
    f: &mut Frame,
    atlas: &mut Atlas,
    (fi, font): (usize, &fontdue::Font),
    (x, baseline, size, s, c, align): (i32, i32, f32, &str, Rgba, Align),
) -> Result<(), AtlasFull> {
    if !(size.is_finite() && size > 0.0 && size <= qp_render::cpu::MAX_TEXT_PX) {
        return Ok(());
    }
    let col = premul(c, 1.0);
    let advance: f32 = s
        .chars()
        .map(|ch| font.metrics(ch, size).advance_width)
        .sum();
    let mut pen = match align {
        Align::Left => x as f32,
        Align::Right => x as f32 - advance,
    };
    let first = f.n;
    for ch in s.chars() {
        let g = atlas.glyph((fi, font), ch, size).ok_or(AtlasFull)?;
        if g.w > 0 && g.h > 0 {
            let gx = (pen.round() as i32 + g.xmin) as f32;
            let gy = (baseline - g.h as i32 - g.ymin) as f32;
            let (x1, y1) = (gx + g.w as f32, gy + g.h as f32);
            let (u0, v0) = (g.ax as f32, g.ay as f32);
            let (u1, v1) = (u0 + g.w as f32, v0 + g.h as f32);
            f.vertex([gx, gy], col, [u0, v0]);
            f.vertex([x1, gy], col, [u1, v0]);
            f.vertex([x1, y1], col, [u1, v1]);
            f.vertex([gx, gy], col, [u0, v0]);
            f.vertex([x1, y1], col, [u1, v1]);
            f.vertex([gx, y1], col, [u0, v1]);
        }
        pen += g.advance;
    }
    f.draw(DRAW_PLAIN, first);
    Ok(())
}

/// The GPU path's state in the core: the glyph atlas and the last built frame.
pub struct GpuPainter {
    atlas: Atlas,
    frame: Frame,
}

impl Default for GpuPainter {
    fn default() -> Self {
        Self {
            atlas: Atlas::new(0),
            frame: Frame::default(),
        }
    }
}

impl GpuPainter {
    /// Builds the triangles of `scene` at the integer device scale `s` (the reference
    /// renderer's scaling), replacing the last frame. A full atlas starts a new one (its
    /// generation changes; the shell uploads it whole) and the frame is built again.
    pub fn build(&mut self, scene: &SceneDesc, s: u32) {
        let prims: Vec<&Prim> = scene.prims().collect();
        let (w, h) = (
            scene.width.saturating_mul(s),
            scene.height.saturating_mul(s),
        );
        self.build_prims((w, h, scene.background), &prims, s);
    }

    /// [`GpuPainter::build`] over primitives in paint order, for a device-size frame
    /// `(w, h, background)`.
    pub fn build_prims(&mut self, frame: (u32, u32, Rgba), prims: &[&Prim], s: u32) {
        let Ok(font) = qp_render::cpu::font() else {
            self.frame = Frame::default();
            return;
        };
        for _ in 0..2 {
            if let Ok(f) = self.try_build(frame, prims, s, font) {
                self.frame = f;
                return;
            }
            let g = self.atlas.generation.wrapping_add(1);
            self.atlas = Atlas::new(g);
        }
        self.frame = Frame::default();
    }

    fn try_build(
        &mut self,
        (w, h, bg): (u32, u32, Rgba),
        prims: &[&Prim],
        s: u32,
        font: &fontdue::Font,
    ) -> Result<Frame, AtlasFull> {
        let mut f = Frame::new(w, h, bg);
        for p in prims {
            match qp_render::cpu::scale_prim(p, s) {
                Prim::Rect { x, y, w, h, color } => rect(&mut f, x, y, w, h, premul(color, 1.0)),
                Prim::Polyline {
                    points,
                    width,
                    color,
                } => stroke(&mut f, &points, width, color),
                Prim::Fill { points, color } => fill(&mut f, &points, premul(color, 1.0)),
                Prim::Circle { cx, cy, r, color } => {
                    circle(&mut f, cx, cy, r, premul(color, 1.0));
                }
                Prim::Text {
                    x,
                    baseline,
                    px,
                    text: t,
                    color,
                    align,
                    font: face,
                    ..
                } => text(
                    &mut f,
                    &mut self.atlas,
                    match super::paint::text_font(face.as_deref()) {
                        Some((i, ff, _)) if i != 0 => (i, ff),
                        _ => (0, font),
                    },
                    (x, baseline, px, &t, color, align),
                )?,
            }
        }
        Ok(f)
    }

    /// The last frame's vertex bytes ([`VERTEX_BYTES`] per vertex).
    #[must_use]
    pub fn vertices(&self) -> &[u8] {
        &self.frame.verts
    }

    /// The last frame's draws: `kind, first, count` triples.
    #[must_use]
    pub fn draws(&self) -> &[u32] {
        &self.frame.draws
    }

    /// The last frame's draws split into kinds, first vertices and counts.
    #[must_use]
    pub fn draw_column(&self, k: usize) -> Vec<u32> {
        self.frame
            .draws
            .iter()
            .skip(k)
            .step_by(3)
            .copied()
            .collect()
    }

    /// The last frame's clear colour (premultiplied RGBA, 0..1).
    #[must_use]
    pub fn background(&self) -> [f64; 4] {
        self.frame.bg
    }

    /// The atlas generation (changes when the atlas starts anew).
    #[must_use]
    pub fn atlas_generation(&self) -> u32 {
        self.atlas.generation
    }

    /// The atlas rows changed since they were last taken, `[y, width, height]`; empty
    /// when nothing changed.
    #[must_use]
    pub fn atlas_dirty(&self) -> Vec<u32> {
        self.atlas
            .dirty
            .map(|(y0, y1)| vec![y0, ATLAS_W, y1 - y0])
            .unwrap_or_default()
    }

    /// Names every atlas row for upload (the shell made a new texture).
    pub fn mark_atlas(&mut self) {
        self.atlas.mark(0, ATLAS_H);
    }

    /// The texels of the rows [`GpuPainter::atlas_dirty`] names (row pitch [`ATLAS_W`]);
    /// clears the record.
    pub fn take_atlas_rows(&mut self) -> Vec<u8> {
        let Some((y0, y1)) = self.atlas.dirty.take() else {
            return Vec::new();
        };
        self.atlas.pixels[(y0 * ATLAS_W) as usize..(y1 * ATLAS_W) as usize].to_vec()
    }

    /// A vertex buffer size (bytes) that holds the last frame: the next power of two,
    /// at least 64 KiB (the shell grows its buffer only when a frame does not fit).
    #[must_use]
    pub fn capacity(&self) -> u32 {
        let n = u32::try_from(self.frame.verts.len()).unwrap_or(u32::MAX / 2);
        n.max(1 << 16).next_power_of_two()
    }

    /// Glyphs in the atlas (for tests and the metrics).
    #[must_use]
    pub fn atlas_glyphs(&self) -> usize {
        self.atlas.glyphs.len()
    }
}

impl ShellCore {
    /// (RP3/M12c) Builds capsule `id`'s face into `gpu` at the device scale; false when it
    /// has not changed since the last hand-out (the same short-circuit and record as
    /// [`ShellCore::face_rgba`] and the paint list) or has no scene.
    pub fn face_gpu(&mut self, id: &str, force: bool, gpu: &mut GpuPainter) -> bool {
        let Some(scene) = self.face(id) else {
            return false;
        };
        if !self.face_changed(id, &scene, force) {
            return false;
        }
        gpu.build(&scene, self.scale);
        true
    }

    /// (RP3/M12c) Builds the open capsule's scene into `gpu` (false when unchanged or
    /// none; as [`ShellCore::scene_rgba`]).
    pub fn scene_gpu(&mut self, force: bool, gpu: &mut GpuPainter) -> bool {
        let Some(scene) = self.scene() else {
            self.scene_shown = None;
            return false;
        };
        if !self.scene_changed(&scene, force) {
            return false;
        }
        gpu.build(&scene, self.scale);
        true
    }
}

#[cfg(test)]
mod tests;
