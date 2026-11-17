//! (RP3/M12b) The `canvas` paint path's command stream: the [`SceneDesc`]'s primitives
//! in paint order, scaled to the device scale with the reference renderer's own rule
//! (`qp_render::cpu::scale_prim`), encoded as a flat `f32` command buffer that the web
//! shell's `paint/canvas.js` maps one to one onto Canvas 2D calls
//! (`shells/web/paint/README.md`, "canvas komut akışı"). Every number, colour, font and
//! text the painter passes on is prepared here; the painter does no arithmetic, no layout
//! and no text building.
//!
//! Beyond the paint list v1 (`super::paint`) the encoder
//! - emits a style (`fillStyle`, `strokeStyle`, `lineWidth`, `font`, `textAlign`) only when
//!   it changes;
//! - batches consecutive strokes (fills, circles) of one opaque colour (and, for fills, one
//!   winding direction) into one path with one `stroke()` (`fill()`);
//! - draws text with the reference renderer's own glyph pixels (fontdue coverage in the
//!   text's colour) from a glyph atlas the browser keeps on a canvas, each glyph at the
//!   reference pen position (rounded to whole pixels, as `qp_render::cpu` does); variants
//!   let the browser draw the glyphs itself (`fillText`, glyph by glyph or as one run);
//! - repaints only the region that changed since the previous list of the same canvas (a
//!   clip rectangle; every primitive whose bounds meet it is painted again, in order, over
//!   the background), and the whole canvas when that region is large.

use std::collections::BTreeMap;

use qp_schema::scene::{Align, Prim, Rgba, SceneDesc};

use super::ShellCore;

/// Command codes of the stream (`f32`; arguments follow the code).
pub mod op {
    /// `fillStyle = S[a]`.
    pub const FILL_STYLE: f32 = 1.0;
    /// `strokeStyle = S[a]`.
    pub const STROKE_STYLE: f32 = 2.0;
    /// `lineWidth = a`.
    pub const LINE_WIDTH: f32 = 3.0;
    /// `font = S[a]`.
    pub const FONT: f32 = 4.0;
    /// `textAlign = S[a]`.
    pub const ALIGN: f32 = 5.0;
    /// `fillRect(x, y, w, h)`.
    pub const RECT: f32 = 6.0;
    /// `beginPath()`.
    pub const BEGIN: f32 = 7.0;
    /// `moveTo(x, y)`.
    pub const MOVE: f32 = 8.0;
    /// `lineTo(x, y)`.
    pub const LINE: f32 = 9.0;
    /// `closePath()`.
    pub const CLOSE: f32 = 10.0;
    /// `arc(x, y, r, 0, a1)`.
    pub const ARC: f32 = 11.0;
    /// `stroke()`.
    pub const STROKE: f32 = 12.0;
    /// `fill()` (non-zero).
    pub const FILL: f32 = 13.0;
    /// `fillText(S[a], x, y)`.
    pub const TEXT: f32 = 14.0;
    /// `save(); beginPath(); rect(x, y, w, h); clip()`.
    pub const CLIP: f32 = 15.0;
    /// `restore()`.
    pub const RESTORE: f32 = 16.0;
    /// `lineCap = S[a]; lineJoin = S[b]; miterLimit = c; textBaseline = S[d]`
    /// (and `imageSmoothingEnabled = false`).
    pub const INIT: f32 = 17.0;
    /// `drawImage(atlas, sx, sy, w, h, dx, dy, w, h)`: one glyph from the glyph atlas.
    pub const GLYPH: f32 = 18.0;
    /// The glyph atlas is `w × h`; rows `y0 .. y0 + rows` are replaced by
    /// `Shell::canvas_atlas_px()` (`putImageData` at `(0, y0)`).
    pub const ATLAS: f32 = 19.0;
}

/// The number of arguments that follow command `code` (`None`: not a command).
#[must_use]
pub fn arity(code: f32) -> Option<usize> {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let n = match code as u32 {
        7 | 10 | 12 | 13 | 16 => 0,
        1..=5 => 1,
        8 | 9 => 2,
        14 => 3,
        6 | 11 | 15 | 17 | 19 => 4,
        18 => 6,
        _ => return None,
    };
    Some(n)
}

/// The reference renderer's miter limit (tiny-skia's default).
const MITER: f32 = super::paint::PAINT_MITER_LIMIT;
/// A partial repaint is used only while the changed region covers at most this share of
/// the canvas (above it a full repaint costs about the same and is simpler).
const PARTIAL_MAX_SHARE: f64 = 0.5;

/// How text is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Text {
    /// One `fillText` per text, the browser places the glyphs (`textAlign`).
    Run,
    /// `fillText` glyph by glyph at the reference renderer's pen positions.
    Glyphs,
    /// The reference renderer's own glyph pixels (fontdue coverage in the text's colour)
    /// from a glyph atlas, `drawImage` at the reference positions.
    Atlas,
}

/// How the stream is made (`Shell::canvas_config`; for the experiment's variants).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    /// How text is drawn.
    pub text: Text,
    /// Batch consecutive same-style paths.
    pub batch: bool,
    /// Repaint only the changed region.
    pub partial: bool,
    /// Leave text out (diagnostic: the visual-difference test finds the text pixels as
    /// the difference between a stream with and without text).
    pub no_text: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            text: Text::Atlas,
            batch: true,
            partial: true,
            no_text: false,
        }
    }
}

impl Config {
    /// From the page's `?canvas=` value: comma-separated `glyphs` (the browser draws text
    /// glyph by glyph), `run` (the browser draws each text as one run), `atlas` (the
    /// default: text from the glyph atlas), `nobatch`, `full` (no partial repaint), `notext`
    /// (diagnostic); anything else is ignored.
    #[must_use]
    pub fn parse(s: &str) -> Self {
        let mut c = Self::default();
        for w in s.split(',') {
            match w.trim() {
                "run" => c.text = Text::Run,
                "glyphs" => c.text = Text::Glyphs,
                "atlas" => c.text = Text::Atlas,
                "nobatch" => c.batch = false,
                "full" => c.partial = false,
                "notext" => c.no_text = true,
                _ => {}
            }
        }
        c
    }
}

/// A command stream and its string table.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Stream {
    /// The commands.
    pub ops: Vec<f32>,
    /// The strings the commands index (colours, fonts, texts, alignment names).
    pub strings: Vec<String>,
    /// The stream repaints only a region (it starts with [`op::CLIP`]).
    pub partial: bool,
}

fn css(c: Rgba) -> String {
    format!("#{:02x}{:02x}{:02x}{:02x}", c[0], c[1], c[2], c[3])
}

#[derive(Clone, Copy, PartialEq)]
enum Batch {
    None,
    Stroke,
    Fill,
}

/// Bounds `[x0, y0, x1, y1]` in device px.
type Bounds = [f32; 4];

struct Enc<'a> {
    cfg: Config,
    atlas: &'a mut Atlas,
    ops: Vec<f32>,
    strings: Vec<String>,
    index: BTreeMap<String, f32>,
    fill: Option<f32>,
    stroke: Option<f32>,
    lw: Option<f32>,
    font: Option<f32>,
    align: Option<f32>,
    batch: Batch,
    key: (Rgba, f32, i8),
}

#[allow(clippy::cast_precision_loss)]
impl<'a> Enc<'a> {
    fn new(cfg: Config, atlas: &'a mut Atlas) -> Self {
        Self {
            cfg,
            atlas,
            ops: Vec::new(),
            strings: Vec::new(),
            index: BTreeMap::new(),
            fill: None,
            stroke: None,
            lw: None,
            font: None,
            align: None,
            batch: Batch::None,
            key: ([0; 4], 0.0, 0),
        }
    }

    fn s(&mut self, v: &str) -> f32 {
        if let Some(&i) = self.index.get(v) {
            return i;
        }
        let i = self.strings.len() as f32;
        self.strings.push(v.to_owned());
        self.index.insert(v.to_owned(), i);
        i
    }

    fn push(&mut self, v: &[f32]) {
        self.ops.extend_from_slice(v);
    }

    fn init(&mut self) {
        let (cap, join, base) = (self.s("butt"), self.s("miter"), self.s("alphabetic"));
        self.push(&[op::INIT, cap, join, MITER, base]);
    }

    fn set(slot: &mut Option<f32>, v: f32, code: f32, ops: &mut Vec<f32>) {
        if *slot != Some(v) {
            *slot = Some(v);
            ops.extend_from_slice(&[code, v]);
        }
    }

    fn fill_style(&mut self, c: Rgba) {
        let i = self.s(&css(c));
        Self::set(&mut self.fill, i, op::FILL_STYLE, &mut self.ops);
    }

    fn stroke_style(&mut self, c: Rgba, lw: f32) {
        let i = self.s(&css(c));
        Self::set(&mut self.stroke, i, op::STROKE_STYLE, &mut self.ops);
        Self::set(&mut self.lw, lw, op::LINE_WIDTH, &mut self.ops);
    }

    fn flush(&mut self) {
        match self.batch {
            Batch::None => {}
            Batch::Stroke => self.ops.push(op::STROKE),
            Batch::Fill => self.ops.push(op::FILL),
        }
        self.batch = Batch::None;
    }

    /// Opens (or continues) a path batch of kind `b` and style `key`.
    fn path(&mut self, b: Batch, key: (Rgba, f32, i8)) {
        let joins = self.cfg.batch && self.batch == b && self.key == key && key.0[3] == 255;
        if joins {
            return;
        }
        self.flush();
        match b {
            Batch::Stroke => self.stroke_style(key.0, key.1),
            _ => self.fill_style(key.0),
        }
        self.ops.push(op::BEGIN);
        self.batch = b;
        self.key = key;
    }

    fn points(&mut self, pts: &[[f32; 2]]) {
        for (i, &[x, y]) in pts.iter().enumerate() {
            let code = if i == 0 { op::MOVE } else { op::LINE };
            self.push(&[code, x, y]);
        }
    }

    fn prim(&mut self, p: &Prim) {
        match p {
            Prim::Rect { x, y, w, h, color } => {
                if *w == 0 || *h == 0 {
                    return;
                }
                self.flush();
                self.fill_style(*color);
                self.push(&[op::RECT, *x as f32, *y as f32, *w as f32, *h as f32]);
            }
            Prim::Polyline {
                points,
                width,
                color,
            } => {
                if !(width.is_finite() && *width >= 0.0) || points.len() < 2 {
                    return;
                }
                // tiny-skia strokes width 0 as a hairline; the canvas ignores 0.
                let lw = if *width == 0.0 { 1.0 } else { *width };
                self.path(Batch::Stroke, (*color, lw, 0));
                self.points(points);
            }
            Prim::Fill { points, color } => {
                if points.len() < 2 {
                    return;
                }
                self.path(Batch::Fill, (*color, 0.0, winding(points)));
                self.points(points);
                self.ops.push(op::CLOSE);
            }
            Prim::Circle { cx, cy, r, color } => {
                if !(r.is_finite() && *r > 0.0) {
                    return;
                }
                self.path(Batch::Fill, (*color, 0.0, 1));
                // A sub-path of its own: the arc would otherwise join the previous point.
                self.push(&[
                    op::MOVE,
                    cx + r,
                    *cy,
                    op::ARC,
                    *cx,
                    *cy,
                    *r,
                    std::f32::consts::TAU,
                ]);
            }
            Prim::Text {
                x,
                baseline,
                px,
                text,
                color,
                align,
            } => {
                if self.cfg.no_text {
                    return;
                }
                self.flush();
                let y = *baseline as f32;
                if self.cfg.text == Text::Atlas {
                    for (ch, gx) in glyph_pens(*x, *px, text, *align) {
                        match self.atlas.slot(ch, *px, *color) {
                            Some(g) if g.w == 0 => {}
                            #[allow(clippy::cast_precision_loss)]
                            Some(g) => self.push(&[
                                op::GLYPH,
                                g.sx as f32,
                                g.sy as f32,
                                g.w as f32,
                                g.h as f32,
                                gx + g.dx as f32,
                                *baseline as f32 - g.h as f32 - g.dy as f32,
                            ]),
                            // the atlas is full: the browser draws this glyph
                            None => self.glyph_text(ch, gx, y, *px, *color),
                        }
                    }
                    return;
                }
                self.fill_style(*color);
                let font = format!("{px}px {}", super::paint::PAINT_FONT_FAMILY);
                let f = self.s(&font);
                Self::set(&mut self.font, f, op::FONT, &mut self.ops);
                if self.cfg.text == Text::Glyphs {
                    for (ch, gx) in glyph_pens(*x, *px, text, *align) {
                        self.glyph_text(ch, gx, y, *px, *color);
                    }
                } else {
                    let a = self.s(match align {
                        Align::Left => "left",
                        Align::Right => "right",
                    });
                    Self::set(&mut self.align, a, op::ALIGN, &mut self.ops);
                    let t = self.s(text);
                    self.push(&[op::TEXT, t, *x as f32, y]);
                }
            }
        }
    }

    /// One glyph drawn by the browser (`fillText`) at pen `gx`, baseline `y`.
    fn glyph_text(&mut self, ch: char, gx: f32, y: f32, px: f32, color: Rgba) {
        self.fill_style(color);
        let font = format!("{px}px {}", super::paint::PAINT_FONT_FAMILY);
        let f = self.s(&font);
        Self::set(&mut self.font, f, op::FONT, &mut self.ops);
        let a = self.s("left");
        Self::set(&mut self.align, a, op::ALIGN, &mut self.ops);
        let mut b = [0u8; 4];
        let g = self.s(ch.encode_utf8(&mut b));
        self.push(&[op::TEXT, g, gx, y]);
    }

    fn finish(mut self, partial: bool) -> Stream {
        self.flush();
        // The atlas rows this stream's glyphs need go up before anything is drawn: right
        // after INIT (and the CLIP of a partial repaint).
        if let Some((w, h, y0, rows)) = self.atlas.upload() {
            let at = if partial { 10 } else { 5 };
            #[allow(clippy::cast_precision_loss)]
            let a = [op::ATLAS, w as f32, h as f32, y0 as f32, rows as f32];
            self.ops.splice(at..at, a);
        }
        Stream {
            ops: self.ops,
            strings: self.strings,
            partial,
        }
    }
}

/// Width of the glyph atlas in pixels.
const ATLAS_W: u32 = 1024;
/// First and largest height of the glyph atlas (it doubles when full; past the largest
/// height new glyphs are drawn by the browser, and the atlas starts over at the next
/// stream).
const ATLAS_H: (u32, u32) = (128, 2048);

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
    px: Vec<u8>,
    slots: BTreeMap<(char, u32, Rgba), Slot>,
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
    fn begin(&mut self) {
        if self.y + self.row > ATLAS_H.1 / 4 * 3 {
            self.slots.clear();
            (self.x, self.y, self.row) = (0, 0, 0);
            self.px.fill(0);
        }
    }

    /// The slot of `ch` at size `px` in colour `c`, rasterised and packed on first use;
    /// `None` when the atlas is full.
    fn slot(&mut self, ch: char, px: f32, c: Rgba) -> Option<Slot> {
        let key = (ch, px.to_bits(), c);
        if let Some(s) = self.slots.get(&key) {
            return Some(*s);
        }
        let font = qp_render::cpu::font().ok()?;
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
    fn upload(&mut self) -> Option<(u32, u32, u32, u32)> {
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

/// The sign of a polygon's shoelace area (the direction it winds; 0 when degenerate).
fn winding(pts: &[[f32; 2]]) -> i8 {
    let mut a = 0.0f64;
    for (i, p) in pts.iter().enumerate() {
        let q = pts[(i + 1) % pts.len()];
        a += f64::from(p[0]) * f64::from(q[1]) - f64::from(q[0]) * f64::from(p[1]);
    }
    if a > 0.0 {
        1
    } else if a < 0.0 {
        -1
    } else {
        0
    }
}

/// The reference renderer's pen positions (`qp_render::cpu`): advances from fontdue at
/// size `px`, a right-aligned text ends at `x`, every glyph at its pen rounded to a whole
/// pixel. White space draws nothing and is left out.
fn glyph_pens(x: i32, px: f32, text: &str, align: Align) -> Vec<(char, f32)> {
    let Ok(font) = qp_render::cpu::font() else {
        return Vec::new();
    };
    #[allow(clippy::cast_precision_loss)]
    let x = x as f32;
    let advance: f32 = text
        .chars()
        .map(|ch| font.metrics(ch, px).advance_width)
        .sum();
    let mut pen = match align {
        Align::Left => x,
        Align::Right => x - advance,
    };
    let mut out = Vec::with_capacity(text.len());
    for ch in text.chars() {
        if !ch.is_whitespace() {
            out.push((ch, pen.round()));
        }
        pen += font.metrics(ch, px).advance_width;
    }
    out
}

/// Conservative device-pixel bounds of everything `p` may paint (anti-aliasing, miter
/// joins and the browser's own glyph shapes included).
fn bounds(p: &Prim) -> Option<Bounds> {
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
            ..
        } => {
            let font = qp_render::cpu::font().ok()?;
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

fn meets(a: &Bounds, b: &Bounds) -> bool {
    a[0] < b[2] && b[0] < a[2] && a[1] < b[3] && b[1] < a[3]
}

/// The region (whole device pixels, inside the canvas) where `prev` and `next` differ,
/// compared position by position; `None` when nothing differs.
fn changed(prev: &[Prim], next: &[Prim], w: f32, h: f32) -> Option<Bounds> {
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

/// The stream that paints `next` (scaled primitives) on a `w × h` canvas with background
/// `bg`: the whole canvas, or (with `prev`, the list the canvas shows now) only the region
/// that changed. `None` when the reference renderer would paint nothing (a text size
/// outside its range) or when nothing changed.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn stream(
    cfg: Config,
    atlas: &mut Atlas,
    prev: Option<&[Prim]>,
    next: &[Prim],
    size: (u32, u32),
    bg: Rgba,
) -> Option<Stream> {
    for p in next {
        if let Prim::Text { px, .. } = p {
            if !(px.is_finite() && *px > 0.0 && *px <= qp_render::cpu::MAX_TEXT_PX) {
                return None;
            }
        }
    }
    let (w, h) = (size.0 as f32, size.1 as f32);
    atlas.begin();
    let mut e = Enc::new(cfg, atlas);
    e.init();
    let region = match prev {
        Some(prev) if cfg.partial => match changed(prev, next, w, h) {
            None => return None,
            Some(r) => {
                let share = f64::from((r[2] - r[0]) * (r[3] - r[1])) / f64::from(w * h);
                (share <= PARTIAL_MAX_SHARE).then_some(r)
            }
        },
        _ => None,
    };
    let Some(r) = region else {
        e.fill_style(bg);
        e.push(&[op::RECT, 0.0, 0.0, w, h]);
        for p in next {
            e.prim(p);
        }
        return Some(e.finish(false));
    };
    let (cw, ch) = (r[2] - r[0], r[3] - r[1]);
    debug_assert_eq!(e.ops.len(), 5);
    e.push(&[op::CLIP, r[0], r[1], cw, ch]);
    e.fill_style(bg);
    e.push(&[op::RECT, r[0], r[1], cw, ch]);
    for p in next {
        if bounds(p).is_none_or(|b| meets(&b, &r)) {
            e.prim(p);
        }
    }
    e.flush();
    e.ops.push(op::RESTORE);
    Some(e.finish(true))
}

/// The scaled primitives of `scene` at device scale `s` and its device size.
#[must_use]
pub fn scaled(scene: &SceneDesc, s: u32) -> (Vec<Prim>, (u32, u32)) {
    let prims = scene
        .prims()
        .map(|p| qp_render::cpu::scale_prim(p, s))
        .collect();
    (
        prims,
        (
            scene.width.saturating_mul(s),
            scene.height.saturating_mul(s),
        ),
    )
}

/// The `canvas` path's state: what each canvas shows now (for partial repaints), the
/// configuration and the last stream's string table.
#[derive(Debug, Default)]
pub struct CanvasPainter {
    cfg: Config,
    faces: BTreeMap<String, (Vec<Prim>, (u32, u32))>,
    scene: Option<(Vec<Prim>, (u32, u32))>,
    strings: Vec<String>,
    atlas: Atlas,
}

impl CanvasPainter {
    /// Sets the configuration (`Config::parse`) and forgets what the canvases show.
    pub fn configure(&mut self, cfg: Config) {
        self.cfg = cfg;
        self.faces.clear();
        self.scene = None;
    }

    /// The string table of the last stream handed out (JSON array).
    #[must_use]
    pub fn strings_json(&self) -> String {
        serde_json::to_string(&self.strings).unwrap_or_else(|_| "[]".to_owned())
    }

    /// The glyph atlas rows the last stream's [`op::ATLAS`] command replaces.
    pub fn atlas_px(&mut self) -> Vec<u8> {
        self.atlas.take_pending()
    }

    fn hand(&mut self, s: Option<Stream>) -> Vec<f32> {
        match s {
            Some(s) => {
                self.strings = s.strings;
                s.ops
            }
            None => Vec::new(),
        }
    }

    /// The stream for capsule `id`'s face (empty when unchanged or none; the same
    /// short-circuit and record as [`ShellCore::face_rgba`]). `force`: the canvas is new
    /// or was cleared, paint all of it.
    pub fn face(&mut self, core: &mut ShellCore, id: &str, force: bool) -> Vec<f32> {
        let Some(scene) = core.face(id) else {
            return Vec::new();
        };
        if !core.face_changed(id, &scene, force) {
            return Vec::new();
        }
        let (prims, size) = scaled(&scene, core.scale);
        let prev = self
            .faces
            .get(id)
            .filter(|(_, s)| !force && *s == size)
            .map(|(p, _)| p.as_slice());
        let s = stream(
            self.cfg,
            &mut self.atlas,
            prev,
            &prims,
            size,
            scene.background,
        );
        self.faces.insert(id.to_owned(), (prims, size));
        self.hand(s)
    }

    /// The stream for the open capsule's scene (empty when unchanged or none; as
    /// [`ShellCore::scene_rgba`]).
    pub fn scene(&mut self, core: &mut ShellCore, force: bool) -> Vec<f32> {
        let Some(scene) = core.scene() else {
            core.scene_shown = None;
            self.scene = None;
            return Vec::new();
        };
        if !core.scene_changed(&scene, force) {
            return Vec::new();
        }
        let (prims, size) = scaled(&scene, core.scale);
        let prev = self
            .scene
            .as_ref()
            .filter(|(_, s)| !force && *s == size)
            .map(|(p, _)| p.as_slice());
        let s = stream(
            self.cfg,
            &mut self.atlas,
            prev,
            &prims,
            size,
            scene.background,
        );
        self.scene = Some((prims, size));
        self.hand(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The command codes of a stream, arguments skipped.
    fn codes(ops: &[f32]) -> Vec<f32> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < ops.len() {
            out.push(ops[i]);
            i += 1 + arity(ops[i]).expect("a command");
        }
        assert_eq!(i, ops.len());
        out
    }

    fn count(ops: &[f32], code: f32) -> usize {
        codes(ops)
            .iter()
            .filter(|&&v| (v - code).abs() < f32::EPSILON)
            .count()
    }

    fn text(x: i32, t: &str, align: Align) -> Prim {
        Prim::Text {
            x,
            baseline: 30,
            px: 33.0,
            text: t.to_owned(),
            color: [255, 255, 255, 255],
            align,
        }
    }

    fn line(y: f32) -> Prim {
        Prim::Polyline {
            points: vec![[0.0, y], [100.0, y]],
            width: 3.0,
            color: [1, 2, 3, 255],
        }
    }

    #[test]
    fn styles_are_emitted_once_and_paths_batched() {
        let s = stream(
            Config::default(),
            &mut Atlas::default(),
            None,
            &[line(10.0), line(20.0)],
            (200, 100),
            [0; 4],
        )
        .unwrap();
        let strokes = count(&s.ops, op::STROKE);
        assert_eq!(strokes, 1, "{:?}", s.ops);
        let nb = Config {
            batch: false,
            ..Config::default()
        };
        let s = stream(
            nb,
            &mut Atlas::default(),
            None,
            &[line(10.0), line(20.0)],
            (200, 100),
            [0; 4],
        )
        .unwrap();
        assert_eq!(count(&s.ops, op::STROKE), 2);
        // the stroke style appears once
        assert_eq!(count(&s.ops, op::STROKE_STYLE), 1);
    }

    #[test]
    fn glyphs_sit_at_the_reference_pens() {
        let font = qp_render::cpu::font().unwrap();
        let adv = font.metrics('0', 33.0).advance_width;
        let g = glyph_pens(10, 33.0, "0 0", Align::Left);
        assert_eq!(g.len(), 2);
        assert!((g[0].1 - 10.0).abs() < f32::EPSILON);
        assert!((g[1].1 - (10.0 + 2.0 * adv).round()).abs() < f32::EPSILON);
        let r = glyph_pens(100, 33.0, "00", Align::Right);
        assert!((r[0].1 - (100.0 - 2.0 * adv).round()).abs() < f32::EPSILON);
    }

    #[test]
    fn a_small_change_repaints_a_clipped_region_only() {
        let a = vec![line(10.0), text(10, "123", Align::Left), line(90.0)];
        let mut b = a.clone();
        b[1] = text(10, "124", Align::Left);
        let s = stream(
            Config::default(),
            &mut Atlas::default(),
            Some(&a),
            &b,
            (1000, 1000),
            [0; 4],
        )
        .unwrap();
        assert!(s.partial);
        assert!(
            (codes(&s.ops)[1] - op::CLIP).abs() < f32::EPSILON,
            "{:?}",
            s.ops
        );
        // the far line (y 90 ± 8) is outside the text's region (y ≈ -13 … 49) and is left out
        assert_eq!(count(&s.ops, op::STROKE), 1);
        // unchanged → nothing
        assert!(stream(
            Config::default(),
            &mut Atlas::default(),
            Some(&b),
            &b,
            (1000, 1000),
            [0; 4]
        )
        .is_none());
        // a change covering most of the canvas → full repaint
        let s = stream(
            Config::default(),
            &mut Atlas::default(),
            Some(&a),
            &b,
            (60, 60),
            [0; 4],
        )
        .unwrap();
        assert!(!s.partial);
    }

    #[test]
    fn text_size_outside_the_reference_range_paints_nothing() {
        let bad = Prim::Text {
            x: 0,
            baseline: 0,
            px: 0.0,
            text: "x".into(),
            color: [0; 4],
            align: Align::Left,
        };
        assert!(stream(
            Config::default(),
            &mut Atlas::default(),
            None,
            &[bad],
            (10, 10),
            [0; 4]
        )
        .is_none());
    }

    #[test]
    fn atlas_glyphs_are_the_reference_coverage_and_uploaded_first() {
        let cfg = Config {
            text: Text::Atlas,
            ..Config::default()
        };
        let mut atlas = Atlas::default();
        let t = text(10, "SYN 12", Align::Left);
        let s = stream(
            cfg,
            &mut atlas,
            None,
            std::slice::from_ref(&t),
            (300, 60),
            [0; 4],
        )
        .unwrap();
        let c = codes(&s.ops);
        assert_eq!(c[0], op::INIT);
        assert_eq!(c[1], op::ATLAS, "{c:?}");
        assert_eq!(count(&s.ops, op::GLYPH), 5); // the space has no pixels
        assert_eq!(count(&s.ops, op::TEXT), 0);
        let px = atlas.take_pending();
        assert_eq!(px.len(), (s.ops[6] * s.ops[9] * 4.0) as usize);
        // the 'S' glyph: fontdue's coverage times the colour's alpha
        let (m, cov) = qp_render::cpu::font().unwrap().rasterize('S', 33.0);
        let slot = atlas.slot('S', 33.0, [255, 255, 255, 255]).unwrap();
        assert_eq!((slot.w as usize, slot.h as usize), (m.width, m.height));
        let k = cov.iter().position(|&v| v > 0).unwrap();
        let (r, col) = (k / m.width, k % m.width);
        let i = ((slot.sy as usize + r) * ATLAS_W as usize + slot.sx as usize + col) * 4;
        assert_eq!(atlas.px[i + 3], cov[k]);
        // the same glyphs again: nothing to upload
        let s = stream(cfg, &mut atlas, None, &[t], (300, 60), [0; 4]).unwrap();
        assert_eq!(count(&s.ops, op::ATLAS), 0);
    }

    #[test]
    fn config_parses() {
        let c = Config::parse("run,full");
        assert!(c.text == Text::Run && !c.partial && c.batch);
        assert_eq!(Config::parse("glyphs").text, Text::Glyphs);
        assert_eq!(Config::default().text, Text::Atlas);
        assert_eq!(Config::parse(""), Config::default());
    }
}
