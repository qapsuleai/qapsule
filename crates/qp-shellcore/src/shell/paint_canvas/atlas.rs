//! The Canvas 2D encoder's glyph atlas (RG1: moved out of `paint_canvas.rs`).

use super::*;

/// Width of the glyph atlas in pixels.
pub(super) const ATLAS_W: u32 = 1024;

/// First and largest height of the glyph atlas (it doubles when full; past the largest
/// height new glyphs are drawn by the browser, and the atlas starts over at the next
/// stream).
pub(super) const ATLAS_H: (u32, u32) = (128, 2048);

/// A glyph's place in the atlas and its offset from the pen (`dx`) and baseline (`dy`),
/// as the reference renderer places its bitmap (`qp_render::cpu`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Slot {
    /// Left in the atlas.
    pub sx: u32,
    /// Top in the atlas.
    pub sy: u32,
    /// Width (0: the glyph has no pixels).
    pub w: u32,
    /// Height.
    pub h: u32,
    /// fontdue `xmin`.
    pub dx: i32,
    /// fontdue `ymin`.
    pub dy: i32,
}

/// The glyph atlas of the `atlas` text mode: fontdue coverage (exactly the reference
/// renderer's: `Font::rasterize` at the same size) in the text's colour, straight RGBA,
/// shelf-packed; the browser keeps a copy on a canvas, updated row band by row band.
#[derive(Debug, Default)]
pub struct Atlas {
    h: u32,
    pub(super) px: Vec<u8>,
    slots: BTreeMap<(usize, char, u32, Rgba), Slot>,
    x: u32,
    y: u32,
    row: u32,
    dirty: Option<(u32, u32)>,
    pending: Vec<u8>,
}

/// Exact `round(x / 255)` for `x <= 255 * 255` (as the reference renderer's).
fn div255(x: u32) -> u32 {
    let x = x + 128;
    (x + (x >> 8)) >> 8
}

impl Atlas {
    fn mark(&mut self, y0: u32, y1: u32) {
        self.dirty = Some(match self.dirty {
            None => (y0, y1),
            Some((a, b)) => (a.min(y0), b.max(y1)),
        });
    }

    /// Starts over when more than three quarters of the largest atlas are used.
    pub(super) fn begin(&mut self) {
        if self.y + self.row > ATLAS_H.1 / 4 * 3 {
            self.slots.clear();
            (self.x, self.y, self.row) = (0, 0, 0);
            self.px.fill(0);
        }
    }

    /// The slot of `ch` at size `px` in colour `c`, rasterised and packed on first use;
    /// `None` when the atlas is full.
    pub(super) fn slot(
        &mut self,
        (fi, font): (usize, &fontdue::Font),
        ch: char,
        px: f32,
        c: Rgba,
    ) -> Option<Slot> {
        let key = (fi, ch, px.to_bits(), c);
        if let Some(s) = self.slots.get(&key) {
            return Some(*s);
        }
        let (m, cov) = font.rasterize(ch, px);
        #[allow(clippy::cast_possible_truncation)]
        let (gw, gh) = (m.width as u32, m.height as u32);
        if gw == 0 || gh == 0 {
            let s = Slot {
                sx: 0,
                sy: 0,
                w: 0,
                h: 0,
                dx: 0,
                dy: 0,
            };
            self.slots.insert(key, s);
            return Some(s);
        }
        // one empty pixel between glyphs
        if self.x + gw + 1 > ATLAS_W {
            (self.x, self.y, self.row) = (0, self.y + self.row, 0);
        }
        if gw + 1 > ATLAS_W {
            return None;
        }
        if self.h == 0 {
            self.h = ATLAS_H.0;
        }
        let used = self.y + gh + 1;
        if used > self.h {
            let mut h = self.h;
            while h < used && h < ATLAS_H.1 {
                h *= 2;
            }
            if used > h {
                return None;
            }
            // the browser's atlas canvas is cleared when it grows: send all used rows again
            self.h = h;
            self.mark(0, self.y + self.row);
        }
        self.px.resize((ATLAS_W * self.h * 4) as usize, 0);
        let (sx, sy) = (self.x, self.y);
        let stride = (ATLAS_W * 4) as usize;
        for r in 0..m.height {
            for col in 0..m.width {
                let a = div255(u32::from(c[3]) * u32::from(cov[r * m.width + col]));
                let i = (sy as usize + r) * stride + (sx as usize + col) * 4;
                #[allow(clippy::cast_possible_truncation)]
                self.px[i..i + 4].copy_from_slice(&[c[0], c[1], c[2], a as u8]);
            }
        }
        self.x += gw + 1;
        self.row = self.row.max(gh + 1);
        self.mark(sy, sy + gh);
        let s = Slot {
            sx,
            sy,
            w: gw,
            h: gh,
            dx: m.xmin,
            dy: m.ymin,
        };
        self.slots.insert(key, s);
        Some(s)
    }

    /// The rows to send to the browser since the last call (`(w, h, y0, rows)`; their
    /// pixels are then [`Atlas::take_pending`]).
    pub(super) fn upload(&mut self) -> Option<(u32, u32, u32, u32)> {
        let (y0, y1) = self.dirty.take()?;
        let stride = (ATLAS_W * 4) as usize;
        self.pending = self.px[y0 as usize * stride..y1 as usize * stride].to_vec();
        Some((ATLAS_W, self.h, y0, y1 - y0))
    }

    /// The pixels of the last [`Atlas::upload`] (straight RGBA rows).
    pub fn take_pending(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.pending)
    }
}
