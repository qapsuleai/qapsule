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

// RG1: `super::paint::…` paths of the moved items resolve through this re-declaration.
use super::paint;

// RG1: the atlas and the repaint region moved to `paint_canvas/`.
mod atlas;
mod region;

use atlas::Atlas;
use region::{bounds, changed, meets};

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
                font,
                ..
            } => {
                if self.cfg.no_text {
                    return;
                }
                let Some(tf) = super::paint::text_font(font.as_deref()) else {
                    return;
                };
                self.flush();
                let y = *baseline as f32;
                if self.cfg.text == Text::Atlas {
                    for (ch, gx) in glyph_pens(tf.1, *x, *px, text, *align) {
                        match self.atlas.slot((tf.0, tf.1), ch, *px, *color) {
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
                            None => self.glyph_text(tf.2, ch, gx, y, *px, *color),
                        }
                    }
                    return;
                }
                self.fill_style(*color);
                let font = format!("{px}px {}", tf.2);
                let f = self.s(&font);
                Self::set(&mut self.font, f, op::FONT, &mut self.ops);
                if self.cfg.text == Text::Glyphs {
                    for (ch, gx) in glyph_pens(tf.1, *x, *px, text, *align) {
                        self.glyph_text(tf.2, ch, gx, y, *px, *color);
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
    fn glyph_text(&mut self, family: &str, ch: char, gx: f32, y: f32, px: f32, color: Rgba) {
        self.fill_style(color);
        let font = format!("{px}px {family}");
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
fn glyph_pens(font: &fontdue::Font, x: i32, px: f32, text: &str, align: Align) -> Vec<(char, f32)> {
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
mod tests;
