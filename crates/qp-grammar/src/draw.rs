//! Pixel work: turns a prepared layer into primitives and items for one panel.

use std::collections::BTreeMap;

use qp_schema::scene::{Item, PixelRect, Prim, Rgba, SceneLayer, XMap, YMap};

use qp_schema::grammar::PanelPos;

use crate::detmath::px;
use crate::glyph::GlyphBox;
use crate::layer::{step_index, Geom, PRow, Prepared, Split};
use crate::scale::value_y;
use crate::theme::Theme;

mod marks;
mod series;

use marks::{anchored_text, glyphs, points, rules, segments};
use series::{areas, bars, candles, lines};

/// Pixel-aligned rectangle primitive (negative sizes clamp to 0), as R0 builds them.
pub(crate) fn rect(x: i32, y: i32, w: i32, h: i32, color: Rgba) -> Prim {
    Prim::Rect {
        x,
        y,
        w: w.max(0).unsigned_abs(),
        h: h.max(0).unsigned_abs(),
        color,
    }
}

/// Advance width of `text` at `size` px in `font`, rounded up (the font's generated
/// advance table, [`crate::look::FontAsset::width`]; a monospaced file: R3's `characters ·
/// size · advance`). Used to wrap and fit text, place legends and size text hit boxes.
pub(crate) fn est_width(text: &str, size: f64, font: &crate::look::FontAsset) -> i32 {
    font.width(text, size)
}

/// Candle body width for a slot (bucket) width: 70 %, at least 1, odd above 2 (R0).
pub(crate) fn body_width(slot: f64) -> i32 {
    #[allow(clippy::cast_possible_truncation)]
    let mut w = ((slot * 0.7).floor() as i32).max(1);
    if w > 2 && w % 2 == 0 {
        w -= 1;
    }
    w
}

/// What `draw` needs about the frame.
pub(crate) struct Geo<'a> {
    pub x: &'a XMap,
    pub y: &'a YMap,
    pub rect: PixelRect,
    /// Bars per bucket.
    pub k: u64,
    /// Highlighted layer: strokes 1 px thicker.
    pub bold: bool,
    /// Theme (glyph colours).
    pub theme: &'a Theme,
    /// (R5/T3) The look (type roles: width estimate, line height, ascent).
    pub look: &'static crate::look::Look,
    /// (R4/M11) Outcome style of glyphs (`CompileOptions::outcome`).
    pub outcome: &'a dyn crate::glyph::outcome::OutcomeStyle,
    /// (R4/T5) Glyph alphabet (`CompileOptions::alphabet`).
    pub alphabet: &'static dyn crate::glyph::Alphabet,
}

impl Geo<'_> {
    /// Pixel column of the centre of `n` slots starting at `slot` (R0's `cx_of` for n = 1).
    fn cx(&self, slot: u64, n: u64) -> f64 {
        #[allow(clippy::cast_precision_loss)]
        let rel = (slot as f64 - self.x.from_bar as f64) + n as f64 / 2.0;
        (self.x.left + rel * self.x.slot).floor()
    }

    fn y(&self, v: Option<f64>) -> Option<f64> {
        let lim = crate::detmath::PX_LIMIT;
        value_y(self.y, v?)
            .filter(|y| !y.is_nan())
            .map(|y| y.clamp(-lim, lim))
    }

    /// Half width of a slot column (bucket of `k`) for hit boxes.
    fn half_slot(&self) -> i32 {
        #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
        let h = (self.x.slot * self.k as f64 / 2.0).floor() as i32;
        h.max(0)
    }

    fn bars(&self, n: u64) -> Option<u64> {
        (self.k > 1).then_some(n)
    }
}

fn hit_x(cx: i32, half: i32) -> (i32, i32) {
    (cx - half, cx + half + 1)
}

/// Colour breaks between two consecutive drawn rows: `(s, colour after)` with `s` in
/// `[0, 1]` along the segment (`docs/GRAMMAR.md` §7).
fn breaks(split: &Split, a: &PRow, b: &PRow) -> Vec<(f64, Rgba)> {
    if a.color == b.color {
        return Vec::new();
    }
    let mid = vec![(0.5, b.color)];
    match (split, a.ckey, b.ckey) {
        (Split::When, Some(ka), Some(kb)) if ka != kb => {
            vec![((ka / (ka - kb)).clamp(0.0, 1.0), b.color)]
        }
        (Split::Steps { thresholds, colors }, Some(ka), Some(kb)) if ka != kb => {
            let mut out = Vec::new();
            if kb > ka {
                for t in thresholds.iter().filter(|t| ka < **t && **t <= kb) {
                    out.push(((t - ka) / (kb - ka), colors[step_index(thresholds, *t)]));
                }
            } else {
                for t in thresholds.iter().rev().filter(|t| kb < **t && **t <= ka) {
                    let i = step_index(thresholds, *t);
                    out.push(((t - ka) / (kb - ka), colors[i.saturating_sub(1)]));
                }
            }
            if out.is_empty() {
                mid
            } else {
                out
            }
        }
        _ => mid,
    }
}

fn lerp(a: [f64; 2], b: [f64; 2], s: f64) -> [f64; 2] {
    [a[0] + (b[0] - a[0]) * s, a[1] + (b[1] - a[1]) * s]
}

#[allow(clippy::cast_possible_truncation)]
fn pt(p: [f64; 2]) -> [f32; 2] {
    [p[0] as f32, p[1] as f32]
}

/// Draws a prepared layer.
pub(crate) fn draw(pl: &Prepared, g: &Geo<'_>) -> SceneLayer {
    let mut out = SceneLayer {
        id: pl.id.clone(),
        mark: pl.mark,
        data: pl.data.clone(),
        fields: pl.fields.clone(),
        constants: pl.constants.clone(),
        label: pl.label.clone(),
        items: Vec::new(),
        prims: Vec::new(),
        skipped: pl.skipped,
    };
    match pl.geom {
        Geom::Candle => candles(pl, g, &mut out),
        Geom::Line => lines(pl, g, &mut out),
        Geom::Area | Geom::Band => areas(pl, g, &mut out),
        Geom::Bar => bars(pl, g, &mut out),
        Geom::RuleH | Geom::RuleV => rules(pl, g, &mut out),
        Geom::Text if pl.anchor.x.is_some() || pl.anchor.y.is_some() => {
            anchored_text(pl, g, &mut out);
        }
        Geom::Point | Geom::Text => points(pl, g, &mut out),
        Geom::Segment => segments(pl, g, &mut out),
        Geom::Glyph => glyphs(pl, g, &mut out),
    }
    out
}

/// (R5/T3) A text mark's metrics at `size` from the look's role of that size
/// (`TypeScale::mark_role`; the default font at line 1.3 when no role has it):
/// `(font asset, line height, ascent em)`. `r4`: `(plex-mono, 1.3, 0.8)`, R3's constants.
pub(crate) fn text_metrics(
    look: &crate::look::Look,
    size: f64,
) -> (&'static crate::look::FontAsset, f64, f64) {
    let role = look.types.mark_role(size);
    let f = role.map_or(&crate::look::FONTS[0], |r| look.role_font(r));
    let line = role
        .and_then(|r| look.types.role(r))
        .map_or(1.3, |r| r.line);
    (f, line, f.ascent)
}

/// Pixel position of a panel-relative coordinate along `[lo, hi)`.
fn panel_px(pp: PanelPos, lo: i32, hi: i32) -> f64 {
    (f64::from(lo) + pp.panel * f64::from(hi - lo) + pp.px).floor()
}

/// Words of `text` wrapped into lines of at most `room` pixels at `size` (estimate as in
/// [`est_width`]); a word longer than a line stands alone.
pub(crate) fn wrap(text: &str, room: i32, size: f64, adv: &crate::look::FontAsset) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut cur = String::new();
    for w in text.split_whitespace() {
        let cand = if cur.is_empty() {
            w.to_owned()
        } else {
            format!("{cur} {w}")
        };
        if cur.is_empty() || est_width(&cand, size, adv) <= room {
            cur = cand;
        } else {
            lines.push(std::mem::take(&mut cur));
            cur = w.to_owned();
        }
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines
}

fn values(keys: &[String], vals: &[f64]) -> BTreeMap<String, f64> {
    keys.iter().cloned().zip(vals.iter().copied()).collect()
}
