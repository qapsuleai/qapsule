//! Backend-independent scene: chart layout in `f64`, emitted as a flat list of primitives.
//!
//! Every backend (the tiny-skia reference and the spike backends) draws the *same* `Scene`,
//! so pixel differences between backends come from rasterization only, never from layout.
//!
//! Determinism rules used here (see `SPIKE.md`):
//! - only IEEE basic operations (`+ - * /`) plus `floor`/`round`/`min`/`max`; no
//!   transcendental functions (`log10`, `powf`, ...), whose libm implementations differ
//!   between native targets and wasm32;
//! - geometry that does not need anti-aliasing is snapped to whole pixels;
//! - number formatting uses Rust's own `core::fmt`, which is target-independent.
//!
//! This module is public only so the spike examples can reuse it; it is not a stable API.

use crate::RenderError;
use qp_schema::Series;

/// Straight (non-premultiplied) RGBA8 colour.
pub type Rgba = [u8; 4];

/// Horizontal text alignment relative to the anchor `x`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Right,
}

/// One drawing primitive.
#[derive(Debug, Clone, PartialEq)]
pub enum Prim {
    /// Pixel-aligned filled rectangle, drawn without anti-aliasing.
    Rect {
        x: i32,
        y: i32,
        w: u32,
        h: u32,
        color: Rgba,
    },
    /// Anti-aliased polyline stroke in pixel coordinates.
    Polyline {
        points: Vec<(f32, f32)>,
        width: f32,
        color: Rgba,
    },
    /// A single line of text. `x` is the anchor, `baseline` the baseline y, `px` the font size.
    Text {
        x: i32,
        baseline: i32,
        px: f32,
        text: String,
        color: Rgba,
        align: Align,
    },
}

/// A complete frame description.
#[derive(Debug, Clone, PartialEq)]
pub struct Scene {
    pub width: u32,
    pub height: u32,
    pub background: Rgba,
    pub prims: Vec<Prim>,
}

const BG: Rgba = [16, 18, 24, 255];
const GRID: Rgba = [34, 38, 48, 255];
const AXIS: Rgba = [84, 90, 104, 255];
const TEXT: Rgba = [200, 205, 215, 255];
const MUTED: Rgba = [130, 136, 150, 255];
const UP: Rgba = [38, 166, 154, 255];
const DOWN: Rgba = [239, 83, 80, 255];
const OVERLAY: Rgba = [255, 183, 77, 230];

const MIN_W: u32 = 200;
const MIN_H: u32 = 120;
const MAX_DIM: u32 = 16_384;
/// The overlay study (computed by `qp-compute`) and its title label.
const OVERLAY_STUDY: &str = "sma";
const OVERLAY_PERIOD: f64 = 20.0;
const OVERLAY_LABEL: &str = "SMA20";

const TITLE_PX: f32 = 16.0;
const LABEL_PX: f32 = 11.0;

/// Plot rectangle in pixels (`right`/`bottom` exclusive).
struct Plot {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

impl Plot {
    fn w(&self) -> f64 {
        f64::from(self.right - self.left)
    }
    fn h(&self) -> f64 {
        f64::from(self.bottom - self.top)
    }
}

/// Picks a 1/2/5 x 10^k step `>= raw` without `log10`/`powf`.
fn nice_step(raw: f64) -> f64 {
    let mut mag = 1.0_f64;
    while mag * 10.0 <= raw {
        mag *= 10.0;
    }
    while mag > raw {
        mag /= 10.0;
    }
    let r = raw / mag;
    let m = if r <= 1.0 {
        1.0
    } else if r <= 2.0 {
        2.0
    } else if r <= 5.0 {
        5.0
    } else {
        10.0
    };
    m * mag
}

/// Days since 1970-01-01 to (year, month). Howard Hinnant's `civil_from_days`, integer-only.
fn year_month(days: i64) -> (i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m)
}

/// Saturating `f64 -> i32` after rounding (Rust `as` saturates; NaN maps to 0).
fn px(v: f64) -> i32 {
    v.round() as i32
}

fn rect(x: i32, y: i32, w: i32, h: i32, color: Rgba) -> Prim {
    Prim::Rect {
        x,
        y,
        w: w.max(0).unsigned_abs(),
        h: h.max(0).unsigned_abs(),
        color,
    }
}

/// Overlay values (one per bar, `None` during warmup) from the compute core.
fn overlay_values(data: &[qp_schema::Bar]) -> Result<Vec<Option<f64>>, RenderError> {
    let params = std::collections::BTreeMap::from([("period".to_owned(), OVERLAY_PERIOD)]);
    let out = qp_compute::compute_bars(OVERLAY_STUDY, &params, qp_compute::Field::Close, data)
        .map_err(|e| RenderError::new(e.to_string()))?;
    Ok(out.outputs.into_values().next().unwrap_or_default())
}

/// Lays out the chart for `bars` at `width` x `height`.
///
/// # Errors
/// Returns an error if the size is outside `200x120 ..= 16384x16384` or there are no bars.
#[allow(clippy::too_many_lines)]
pub fn build_scene(bars: &Series, width: u32, height: u32) -> Result<Scene, RenderError> {
    if !(MIN_W..=MAX_DIM).contains(&width) || !(MIN_H..=MAX_DIM).contains(&height) {
        return Err(RenderError::new(format!(
            "size {width}x{height} out of range ({MIN_W}x{MIN_H} ..= {MAX_DIM}x{MAX_DIM})"
        )));
    }
    let data = crate::bars::bars_of(bars)?;
    let Some(last) = data.last() else {
        return Err(RenderError::new("bars file has no bars"));
    };
    let (wi, hi) = (width.cast_signed(), height.cast_signed());
    let plot = Plot {
        left: 8,
        top: 34,
        right: wi - 72,
        bottom: hi - 24,
    };
    let mut prims = Vec::with_capacity(data.len() * 2 + 64);

    // Price range with 5% padding.
    let mut lo = f64::INFINITY;
    let mut hi_p = f64::NEG_INFINITY;
    for b in data {
        lo = lo.min(b.l);
        hi_p = hi_p.max(b.h);
    }
    let span = (hi_p - lo).max(1e-9);
    let (pmin, pmax) = (lo - span * 0.05, hi_p + span * 0.05);
    let y_of = |p: f64| f64::from(plot.top) + (pmax - p) / (pmax - pmin) * plot.h();

    // Horizontal grid + price labels.
    let step = nice_step((pmax - pmin) / 6.0);
    let mut k = (pmin / step).ceil();
    while k * step <= pmax {
        let v = k * step;
        let y = px(y_of(v));
        if y > plot.top && y < plot.bottom {
            prims.push(rect(plot.left, y, plot.right - plot.left, 1, GRID));
            prims.push(Prim::Text {
                x: plot.right + 6,
                baseline: y + 4,
                px: LABEL_PX,
                text: format!("{v:.2}"),
                color: MUTED,
                align: Align::Left,
            });
        }
        k += 1.0;
    }

    // Bar slots.
    let n = data.len();
    let slot = plot.w() / n as f64;
    let cx_of = |i: usize| (f64::from(plot.left) + (i as f64 + 0.5) * slot).floor();

    // Vertical grid + time labels at month changes.
    let mut prev_month: Option<(i64, i64)> = None;
    let mut last_label_x = i32::MIN;
    for (i, b) in data.iter().enumerate() {
        let ym = year_month(b.t.div_euclid(86_400_000));
        if prev_month.is_some_and(|p| p != ym) {
            let x = cx_of(i) as i32;
            if x.saturating_sub(last_label_x) >= 72 {
                prims.push(rect(x, plot.top, 1, plot.bottom - plot.top, GRID));
                prims.push(Prim::Text {
                    x: x - 21,
                    baseline: plot.bottom + 16,
                    px: LABEL_PX,
                    text: format!("{:04}-{:02}", ym.0, ym.1),
                    color: MUTED,
                    align: Align::Left,
                });
                last_label_x = x;
            }
        }
        prev_month = Some(ym);
    }

    // Axes.
    prims.push(rect(
        plot.right,
        plot.top,
        1,
        plot.bottom - plot.top + 1,
        AXIS,
    ));
    prims.push(rect(
        plot.left,
        plot.bottom,
        plot.right - plot.left,
        1,
        AXIS,
    ));

    // Candles.
    let mut body_w = ((slot * 0.7).floor() as i32).max(1);
    if body_w > 2 && body_w % 2 == 0 {
        body_w -= 1;
    }
    for (i, b) in data.iter().enumerate() {
        let color = if b.c >= b.o { UP } else { DOWN };
        let cx = cx_of(i) as i32;
        let (yh, yl) = (px(y_of(b.h)), px(y_of(b.l)));
        prims.push(rect(cx, yh, 1, (yl - yh).max(1), color));
        let (yo, yc) = (px(y_of(b.o)), px(y_of(b.c)));
        let top = yo.min(yc);
        prims.push(rect(
            cx - body_w / 2,
            top,
            body_w,
            (yo - yc).abs().max(1),
            color,
        ));
    }

    // Overlay line: SMA(20) of close, computed by the compute core (`qp-compute`). One
    // value per bar; warmup bars (None) get no point.
    let overlay = overlay_values(data)?;
    let mut overlay_last = None;
    let points: Vec<(f32, f32)> = overlay
        .iter()
        .enumerate()
        .filter_map(|(i, v)| v.map(|v| (i, v)))
        .map(|(i, v)| {
            overlay_last = Some(v);
            ((cx_of(i) + 0.5) as f32, y_of(v) as f32)
        })
        .collect();
    if !points.is_empty() {
        prims.push(Prim::Polyline {
            points,
            width: 1.5,
            color: OVERLAY,
        });
    }

    // Last-price tag on the price axis.
    let last_color = if last.c >= last.o { UP } else { DOWN };
    let ly = px(y_of(last.c));
    prims.push(rect(
        plot.right + 1,
        ly - 8,
        wi - plot.right - 1,
        16,
        last_color,
    ));
    prims.push(Prim::Text {
        x: plot.right + 6,
        baseline: ly + 4,
        px: LABEL_PX,
        text: format!("{:.2}", last.c),
        color: BG,
        align: Align::Left,
    });

    // Title: symbol, interval, last close, SMA value.
    prims.push(Prim::Text {
        x: plot.left + 4,
        baseline: 23,
        px: TITLE_PX,
        text: format!(
            "{} {}",
            bars.symbol().unwrap_or(""),
            bars.interval.as_deref().unwrap_or("")
        ),
        color: TEXT,
        align: Align::Left,
    });
    prims.push(Prim::Text {
        x: plot.left + 140,
        baseline: 23,
        px: TITLE_PX,
        text: format!("C {:.2}", last.c),
        color: last_color,
        align: Align::Left,
    });
    if let Some(s) = overlay_last {
        prims.push(Prim::Text {
            x: wi - 8,
            baseline: 23,
            px: LABEL_PX,
            text: format!("{OVERLAY_LABEL} {s:.2}"),
            color: OVERLAY,
            align: Align::Right,
        });
    }

    Ok(Scene {
        width,
        height,
        background: BG,
        prims,
    })
}

/// Glyph layout identical to the reference renderer's (fontdue advances on the embedded
/// font, pen rounded to whole pixels). Returns `(glyph id, x)`; y is the baseline.
/// Used by the spike backends so their text differs from the reference only by rasterizer.
///
/// # Errors
/// Only if the embedded font is unusable.
pub fn layout_glyphs(
    text: &str,
    px: f32,
    x: i32,
    align: Align,
) -> Result<Vec<(u32, f32)>, RenderError> {
    let font = crate::cpu::font()?;
    let advance: f32 = text
        .chars()
        .map(|c| font.metrics(c, px).advance_width)
        .sum();
    let mut pen = match align {
        Align::Left => x as f32,
        Align::Right => x as f32 - advance,
    };
    let mut out = Vec::with_capacity(text.len());
    for c in text.chars() {
        out.push((u32::from(font.lookup_glyph_index(c)), pen.round()));
        pen += font.metrics(c, px).advance_width;
    }
    Ok(out)
}

/// Determinism stress scene (spike only): anti-aliased strokes at fractional coordinates and
/// widths, translucent colours, overlapping fills and text at many fractional sizes. Used by
/// the `stress` golden and the browser comparison to exercise paths the chart barely touches.
#[must_use]
pub fn stress_scene(width: u32, height: u32) -> Scene {
    let mut rng = crate::bars::Lcg(0x5EED);
    let (wf, hf) = (f64::from(width), f64::from(height));
    let mut prims = Vec::new();
    let col = |rng: &mut crate::bars::Lcg, a: u8| -> Rgba {
        let v = rng.next_u32().to_le_bytes();
        [v[0], v[1], v[2], a]
    };
    for i in 0..40 {
        let a = col(&mut rng, 255);
        let x = (rng.unit() + 1.0) * 0.5 * wf;
        let y = (rng.unit() + 1.0) * 0.5 * hf;
        let side = (8.0 + 30.0 * (rng.unit() + 1.0)).floor() as i32;
        prims.push(rect(
            x as i32,
            y as i32,
            side,
            side,
            if i % 2 == 0 {
                a
            } else {
                [a[0], a[1], a[2], 128]
            },
        ));
    }
    for _ in 0..30 {
        let n = 2 + (rng.next_u32() % 12) as usize;
        let points = (0..n)
            .map(|_| {
                (
                    ((rng.unit() + 1.0) * 0.5 * wf) as f32,
                    ((rng.unit() + 1.0) * 0.5 * hf) as f32,
                )
            })
            .collect();
        let width = (0.25 + 4.0 * (rng.unit() + 1.0)) as f32;
        let alpha = (64 + rng.next_u32() % 192) as u8;
        prims.push(Prim::Polyline {
            points,
            width,
            color: col(&mut rng, alpha),
        });
    }
    let mut y = 14;
    let mut size = 7.0_f32;
    while y < height.cast_signed() && size < 40.0 {
        let alpha = (128 + rng.next_u32() % 128) as u8;
        prims.push(Prim::Text {
            x: 4 + (rng.next_u32() % 40).cast_signed(),
            baseline: y,
            px: size,
            text: format!("Qapsule {size:.2}px 0123456789 +-.,:%$ AaBbGgQqWw"),
            color: col(&mut rng, alpha),
            align: Align::Left,
        });
        y += size as i32 + 4;
        size += 1.37;
    }
    Scene {
        width,
        height,
        background: [250, 250, 248, 255],
        prims,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nice_steps() {
        assert!((nice_step(0.7) - 1.0).abs() < 1e-12);
        assert!((nice_step(3.0) - 5.0).abs() < 1e-12);
        assert!((nice_step(13.0) - 20.0).abs() < 1e-12);
        assert!((nice_step(0.031) - 0.05).abs() < 1e-12);
    }

    #[test]
    fn civil_dates() {
        assert_eq!(year_month(0), (1970, 1));
        assert_eq!(year_month(20_090), (2025, 1)); // 2025-01-02
        assert_eq!(year_month(19_782), (2024, 2)); // 2024-02-29
    }

    #[test]
    fn size_limits() {
        let b = crate::bars::synthetic_bars(10, 1);
        assert!(build_scene(&b, 10, 10).is_err());
        assert!(build_scene(&b, 400, 200).is_ok());
    }
}
