//! (RP3/M12a) The retained software painter: the reference renderer's own drawing code
//! ([`crate::cpu::draw_prim`]), without the needless work of painting every frame from
//! nothing.
//!
//! A [`Retained`] keeps the pixels of the last frame it painted and that frame's primitive
//! list. A new list is matched against the old one; only the region the changed
//! primitives touch (the *dirty rectangle*) is painted again, and with
//! [`RetainedOpts::layer`] the image of the list's unchanged *prefix* (background, grid,
//! history bars: whatever comes before the first change in paint order) is kept as a
//! second pixmap, so that the dirty region starts from it instead of from the background.
//!
//! **Exactness.** The result is byte-identical to [`crate::cpu::rasterize`] of the same
//! list (tested: `tests/retained.rs`, and in the shell over a replay). Why it holds:
//! - every primitive is drawn by the same function on a pixmap of the same size, with the
//!   same (identity) transform, so a primitive's coverage of a pixel and its blend with
//!   the pixel below do not depend on what else is drawn;
//! - a pixel outside the dirty rectangle is touched by the same primitives in the same
//!   order in the old and the new list (the changed ones' boxes, [`crate::cpu::prim_bounds`],
//!   are conservative and the matching keeps order), so it keeps its old value;
//! - a pixel inside it is recomputed from the background (or from the prefix image, which
//!   is exactly the state after the prefix) by every primitive whose box meets the
//!   rectangle, in order. Primitives are drawn whole on a full-size scratch pixmap (no
//!   translation or clipping, which could move a coverage value), and only the rectangle
//!   is copied back.

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use qp_schema::scene::{Align, Prim, Rgba};
use tiny_skia::{Color, Pixmap};

use crate::cpu::{draw_prim, font, prim_bounds, GlyphCache, MAX_DIM};
use crate::RenderError;

/// A prefix image is kept only for at least this many unchanged leading primitives (a
/// shorter prefix is cheaper to draw again than to copy).
pub const LAYER_MIN_PRIMS: usize = 16;
/// A dirty rectangle covering more than this share (per mille) of the surface is painted
/// as a whole frame (the copy from the scratch pixmap would cost more than it saves).
pub const FULL_SHARE_PERMILLE: u64 = 600;

/// What [`Retained::paint`] may do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetainedOpts {
    /// Paint only the dirty rectangle (off: every change repaints the whole frame).
    pub dirty: bool,
    /// Keep the unchanged prefix's image.
    pub layer: bool,
}

impl Default for RetainedOpts {
    fn default() -> Self {
        Self {
            dirty: true,
            layer: true,
        }
    }
}

/// What the last [`Retained::paint`] did (for the measurements).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PaintStats {
    /// The whole frame was painted.
    pub full: bool,
    /// Primitives drawn.
    pub drawn: usize,
    /// Pixels of the painted region.
    pub area: u64,
    /// The prefix image was (re)built in this paint.
    pub layer_built: bool,
    /// The paint started from the prefix image (primitives before it were not drawn).
    pub layer_used: bool,
}

/// Work memory shared by every [`Retained`] of one kind of surface: the scratch pixmap the
/// dirty region is painted on, and the glyph bitmaps (they depend only on the character
/// and the size, so they live across frames).
#[derive(Debug, Default)]
pub struct Scratch {
    pm: Option<Pixmap>,
    glyphs: GlyphCache,
}

impl Scratch {
    /// Bytes held (pixmap and glyph bitmaps).
    #[must_use]
    pub fn bytes(&self) -> usize {
        self.pm.as_ref().map_or(0, |p| p.data().len())
            + self.glyphs.values().map(|(_, v)| v.len()).sum::<usize>()
    }
}

/// One surface's last frame: its pixels and its primitives.
#[derive(Debug, Default)]
pub struct Retained {
    w: u32,
    h: u32,
    bg: Rgba,
    prims: Vec<Prim>,
    keys: Vec<u64>,
    boxes: Vec<Option<[i64; 4]>>,
    pm: Option<Pixmap>,
    /// The prefix image: the frame after the first `n` primitives.
    layer: Option<(usize, Pixmap)>,
    /// An invalidated prefix image's allocation, for the next one.
    spare: Option<Pixmap>,
    stats: PaintStats,
}

/// A rectangle `[x0, y0, x1, y1)` in surface pixels.
type Region = [u32; 4];

fn key(p: &Prim) -> u64 {
    let mut h = DefaultHasher::new();
    let pts = |h: &mut DefaultHasher, v: &[[f32; 2]]| {
        v.len().hash(h);
        for [x, y] in v {
            x.to_bits().hash(h);
            y.to_bits().hash(h);
        }
    };
    match p {
        Prim::Rect {
            x,
            y,
            w,
            h: hh,
            color,
        } => {
            (0u8, x, y, w, hh, color).hash(&mut h);
        }
        Prim::Polyline {
            points,
            width,
            color,
        } => {
            (1u8, width.to_bits(), color).hash(&mut h);
            pts(&mut h, points);
        }
        Prim::Fill { points, color } => {
            (2u8, color).hash(&mut h);
            pts(&mut h, points);
        }
        Prim::Circle { cx, cy, r, color } => {
            (3u8, cx.to_bits(), cy.to_bits(), r.to_bits(), color).hash(&mut h);
        }
        Prim::Text {
            x,
            baseline,
            px,
            text,
            color,
            align,
            font,
            ..
        } => {
            let a = matches!(align, Align::Right);
            (4u8, x, baseline, px.to_bits(), text, color, a).hash(&mut h);
            // (R5/T3) The font asset; absent (the default) hashes as before.
            if let Some(f) = font {
                f.hash(&mut h);
            }
        }
    }
    h.finish()
}

/// A 64-bit fingerprint of a whole frame (size, background, every primitive in order):
/// equal fingerprints mean the same pixels, up to a hash collision (SipHash with fixed
/// keys, deterministic). Lets a caller know that a surface it no longer retains still
/// shows the frame it would paint.
#[must_use]
pub fn fingerprint(w: u32, h: u32, bg: Rgba, prims: &[Prim]) -> u64 {
    let mut s = DefaultHasher::new();
    (w, h, bg, prims.len()).hash(&mut s);
    for p in prims {
        key(p).hash(&mut s);
    }
    s.finish()
}

/// (RP3/S) A 64-bit fingerprint of everything in `scene` that its pixels depend on: size,
/// background and every primitive of [`SceneDesc::prims`] in paint order, each field
/// bit for bit (the same per-primitive key as the retained painter; SipHash with fixed
/// keys, deterministic). No serialisation: the shell's "did this face/scene change since
/// it was painted" check. Equal fingerprints mean the same pixels up to a 64-bit hash
/// collision; it is not a digest for parity (that stays `scene_digest`).
#[must_use]
pub fn scene_fingerprint(scene: &qp_schema::scene::SceneDesc) -> u64 {
    let mut s = DefaultHasher::new();
    (scene.width, scene.height, scene.background).hash(&mut s);
    let mut n = 0usize;
    for p in scene.prims() {
        key(p).hash(&mut s);
        n += 1;
    }
    n.hash(&mut s);
    s.finish()
}

fn copy_region(dst: &mut Pixmap, src: &Pixmap, r: Region) {
    let stride = dst.width() as usize * 4;
    let (a, b) = (r[0] as usize * 4, r[2] as usize * 4);
    let s = src.data();
    let d = dst.data_mut();
    for y in r[1] as usize..r[3] as usize {
        let o = y * stride;
        d[o + a..o + b].copy_from_slice(&s[o + a..o + b]);
    }
}

fn fill_region(dst: &mut Pixmap, bg: Rgba, r: Region) {
    let c = Color::from_rgba8(bg[0], bg[1], bg[2], bg[3])
        .premultiply()
        .to_color_u8();
    let px = [c.red(), c.green(), c.blue(), c.alpha()];
    let stride = dst.width() as usize * 4;
    let d = dst.data_mut();
    for y in r[1] as usize..r[3] as usize {
        let o = y * stride;
        for p in d[o + r[0] as usize * 4..o + r[2] as usize * 4].chunks_exact_mut(4) {
            p.copy_from_slice(&px);
        }
    }
}

fn meets(b: Option<[i64; 4]>, r: Region) -> bool {
    b.is_none_or(|b| {
        b[0] < i64::from(r[2])
            && b[2] > i64::from(r[0])
            && b[1] < i64::from(r[3])
            && b[3] > i64::from(r[1])
    })
}

/// A pixmap of `w` × `h`, reusing `old`'s allocation when it has that size.
fn pixmap(old: Option<Pixmap>, w: u32, h: u32) -> Result<Pixmap, RenderError> {
    match old {
        Some(p) if p.width() == w && p.height() == h => Ok(p),
        _ => Pixmap::new(w, h).ok_or_else(|| RenderError::new("cannot allocate pixmap")),
    }
}

impl Retained {
    /// The last painted frame (premultiplied RGBA8; with an opaque background every alpha
    /// is 255 and this is also straight RGBA).
    #[must_use]
    pub fn pixmap(&self) -> Option<&Pixmap> {
        self.pm.as_ref()
    }

    /// The last frame's background.
    #[must_use]
    pub fn background(&self) -> Rgba {
        self.bg
    }

    /// What the last paint did.
    #[must_use]
    pub fn stats(&self) -> PaintStats {
        self.stats
    }

    /// Bytes held (frame, prefix image, primitive list not counted).
    #[must_use]
    pub fn bytes(&self) -> usize {
        self.pm.as_ref().map_or(0, |p| p.data().len())
            + self.layer.as_ref().map_or(0, |(_, p)| p.data().len())
            + self.spare.as_ref().map_or(0, |p| p.data().len())
    }

    #[allow(clippy::too_many_lines)]
    /// Paints the frame `w` × `h` on `bg` with `prims` (device pixels, paint order) over
    /// the last one. Returns the region that changed (`[x, y, w, h]`), `None` when no
    /// pixel changed. The pixels are then [`Retained::pixmap`], byte-identical to
    /// [`crate::cpu::rasterize`] of the same list.
    ///
    /// # Errors
    /// As [`crate::cpu::rasterize`] (size, text size, allocation, font).
    pub fn paint(
        &mut self,
        w: u32,
        h: u32,
        bg: Rgba,
        prims: Vec<Prim>,
        scratch: &mut Scratch,
        opts: RetainedOpts,
    ) -> Result<Option<[u32; 4]>, RenderError> {
        if !(1..=MAX_DIM).contains(&w) || !(1..=MAX_DIM).contains(&h) {
            return Err(RenderError::new(format!(
                "size {w}x{h} out of range (1x1 ..= {MAX_DIM}x{MAX_DIM})"
            )));
        }
        let font = font()?;
        self.stats = PaintStats::default();
        let keys: Vec<u64> = prims.iter().map(key).collect();
        let boxes: Vec<Option<[i64; 4]>> = prims.iter().map(|p| prim_bounds(font, p)).collect();
        let same_surface = self.pm.is_some() && self.w == w && self.h == h && self.bg == bg;
        let whole: Region = [0, 0, w, h];
        // The first change in paint order, and the dirty rectangle.
        let (first, dirty) = if same_surface {
            let (first, d) = self.diff(&prims, &keys, &boxes, w, h);
            match d {
                None => {
                    self.prims = prims;
                    self.keys = keys;
                    self.boxes = boxes;
                    if self.layer.as_ref().is_some_and(|(n, _)| *n > first) {
                        self.spare = self.layer.take().map(|(_, p)| p);
                    }
                    return Ok(None);
                }
                Some(d) => (first, d),
            }
        } else {
            self.spare = None;
            self.layer = None;
            (0, whole)
        };
        let area = u64::from(dirty[2] - dirty[0]) * u64::from(dirty[3] - dirty[1]);
        let full = !opts.dirty
            || !same_surface
            || area * 1000 > u64::from(w) * u64::from(h) * FULL_SHARE_PERMILLE;
        let region = if full { whole } else { dirty };
        // The prefix image: still valid when the prefix did not change; built anew when
        // the prefix is long enough to be worth it.
        if self.layer.as_ref().is_some_and(|(n, _)| *n > first) || !opts.layer {
            self.spare = self.layer.take().map(|(_, p)| p);
        }
        if opts.layer && self.layer.is_none() && same_surface && first >= LAYER_MIN_PRIMS {
            let mut lp = pixmap(self.spare.take(), w, h)?;
            fill_region(&mut lp, bg, whole);
            for p in &prims[..first] {
                draw_prim(&mut lp, font, &mut scratch.glyphs, p)?;
            }
            self.layer = Some((first, lp));
            self.stats.layer_built = true;
        }
        let pm = self.pm.take();
        let mut target = if full {
            pixmap(pm, w, h)?
        } else {
            self.pm = pm;
            pixmap(scratch.pm.take(), w, h)?
        };
        let start = match &self.layer {
            Some((n, lp)) => {
                copy_region(&mut target, lp, region);
                self.stats.layer_used = true;
                *n
            }
            None => {
                fill_region(&mut target, bg, region);
                0
            }
        };
        let mut drawn = 0;
        for (p, b) in prims[start..].iter().zip(&boxes[start..]) {
            if full || meets(*b, region) {
                draw_prim(&mut target, font, &mut scratch.glyphs, p)?;
                drawn += 1;
            }
        }
        if full {
            self.pm = Some(target);
        } else {
            if let Some(pm) = self.pm.as_mut() {
                copy_region(pm, &target, region);
            }
            scratch.pm = Some(target);
        }
        self.w = w;
        self.h = h;
        self.bg = bg;
        self.prims = prims;
        self.keys = keys;
        self.boxes = boxes;
        self.stats.full = full;
        self.stats.drawn = drawn;
        self.stats.area = u64::from(region[2] - region[0]) * u64::from(region[3] - region[1]);
        Ok(Some([
            region[0],
            region[1],
            region[2] - region[0],
            region[3] - region[1],
        ]))
    }

    /// Matches the new list against the old one (order kept: equal prefix, equal suffix,
    /// then a greedy in-order match of the middle). Returns the length of the equal
    /// prefix and the union of the unmatched primitives' boxes, clipped to the surface
    /// (`None`: no pixel can have changed).
    fn diff(
        &self,
        prims: &[Prim],
        keys: &[u64],
        boxes: &[Option<[i64; 4]>],
        w: u32,
        h: u32,
    ) -> (usize, Option<Region>) {
        let (on, nn) = (self.prims.len(), prims.len());
        let eq = |i: usize, j: usize| self.keys[i] == keys[j] && self.prims[i] == prims[j];
        let mut first = 0;
        while first < on.min(nn) && eq(first, first) {
            first += 1;
        }
        let mut tail = 0;
        while tail < on.min(nn) - first && eq(on - 1 - tail, nn - 1 - tail) {
            tail += 1;
        }
        let mut u = [i64::MAX, i64::MAX, i64::MIN, i64::MIN];
        let mut any = false;
        let mut add = |b: Option<[i64; 4]>| {
            any = true;
            let b = b.unwrap_or([0, 0, i64::from(w), i64::from(h)]);
            u = [
                u[0].min(b[0]),
                u[1].min(b[1]),
                u[2].max(b[2]),
                u[3].max(b[3]),
            ];
        };
        // The middle: old[first..on-tail] against new[first..nn-tail].
        let mut by_key: HashMap<u64, Vec<usize>> = HashMap::new();
        for i in (first..on - tail).rev() {
            by_key.entry(self.keys[i]).or_default().push(i);
        }
        let mut matched = vec![false; on];
        let mut cursor = first;
        for j in first..nn - tail {
            let hit = by_key.get_mut(&keys[j]).and_then(|v| {
                while let Some(&i) = v.last() {
                    if i < cursor {
                        v.pop();
                        continue;
                    }
                    if self.prims[i] == prims[j] {
                        v.pop();
                        return Some(i);
                    }
                    return None;
                }
                None
            });
            match hit {
                Some(i) => {
                    matched[i] = true;
                    cursor = i + 1;
                }
                None => add(boxes[j]),
            }
        }
        for (m, b) in matched[first..on - tail]
            .iter()
            .zip(&self.boxes[first..on - tail])
        {
            if !m {
                add(*b);
            }
        }
        if !any {
            return (first, None);
        }
        let clip = |v: i64, hi: u32| v.clamp(0, i64::from(hi)) as u32;
        let r = [clip(u[0], w), clip(u[1], h), clip(u[2], w), clip(u[3], h)];
        if r[0] >= r[2] || r[1] >= r[3] {
            return (first, None);
        }
        (first, Some(r))
    }
}
