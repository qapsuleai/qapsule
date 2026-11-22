//! The last-value tag and dotted value line (OV-8); the crosshair with price and date pills
//! (OV-9). Lines are runs of one-pixel rectangles (the scene has no dash).

use super::format::{pill_date, NumFmt};
use super::*;
use crate::look::ValueLine;

/// Crosshair dash and gap (px).
const DASH: (i32, i32) = (4, 3);
/// Value line dot and gap (px).
const DOT: (i32, i32) = (2, 2);

impl Chart<'_> {
    /// The date pill under the cursor: left edge, width, text.
    pub(super) fn date_pill(&self) -> Option<(i32, i32, String)> {
        let at = self.at?;
        let t = self.fr.slot_time(at.slot)?;
        let intraday = self.fr.slot_time(at.slot + 1).or_else(|| {
            at.slot
                .checked_sub(1)
                .and_then(|s| self.fr.slot_time(s))
                .map(|p| 2 * t - p)
        });
        let intraday = intraday.is_some_and(|n| (n - t).abs() < format::DAY_MS);
        let label = pill_date(t, intraday, self.lang());
        let w = self.width(&label, "label") + 2 * PILL_PAD_X;
        Some(((at.x - w / 2).clamp(0, (self.w - w).max(0)), w, label))
    }

    /// A filled tag from `x` to the frame's right edge, centred on row `y`.
    fn pill(&self, out: &mut Vec<Prim>, x: i32, y: i32, label: String, fill: Rgba) {
        let h = self.pill_h();
        out.push(rect(x, y - h / 2, self.w - x, h, fill));
        let ink = self.theme().bg;
        let fig = self.size("figure");
        out.push(text(
            x + PILL_PAD_X - 1,
            y + self.mid(),
            fig,
            label,
            ink,
            "figure",
        ));
    }
}

/// Last-value tags (R0's for a legacy look), and the dotted value line across the first
/// panel when the look asks for it.
pub(in crate::compile) fn tags(ch: &Chart<'_>, under: &mut Vec<Prim>, over: &mut Vec<Prim>) {
    let theme = ch.theme();
    for (pi, (rc, tag)) in ch.rects.iter().zip(ch.tags).enumerate() {
        let Some((ly, v, color)) = *tag else {
            continue;
        };
        let color = [color[0], color[1], color[2], 255];
        if !ch.look.familiar() {
            over.push(rect(rc.right + 1, ly - 8, ch.w - rc.right - 1, 16, color));
            let (fig, label) = (ch.size("figure"), format!("{v:.2}"));
            over.push(text(rc.right + 6, ly + 4, fig, label, theme.bg, "figure"));
            continue;
        }
        if pi == 0 && ch.look.chart.value_line == ValueLine::Dotted {
            let step = usize::try_from(DOT.0 + DOT.1).unwrap_or(1);
            for x in (rc.left..rc.right).step_by(step) {
                under.push(rect(x, ly, DOT.0.min(rc.right - x), 1, color));
            }
        }
        if ch.look.chart.last_value_tag {
            let label = NumFmt::new(v.abs(), None, ch.lang()).fmt(v);
            ch.pill(over, rc.right + 1, ly, label, color);
        }
    }
}

/// The crosshair: dashed lines through the cursor, its price and date pills.
pub(in crate::compile) fn crosshair(ch: &Chart<'_>, over: &mut Vec<Prim>) {
    let Some(at) = ch.at else { return };
    let ink = ch.theme().muted;
    let step = usize::try_from(DASH.0 + DASH.1).unwrap_or(1);
    for (rc, _) in ch.rects.iter().zip(ch.card).filter(|(_, c)| !**c) {
        for y in (rc.top..rc.bottom).step_by(step) {
            over.push(rect(at.x, y, 1, DASH.0.min(rc.bottom - y), ink));
        }
    }
    if let Some((v, y)) = at.price {
        let rc = ch.rects[0];
        for x in (rc.left..rc.right).step_by(step) {
            over.push(rect(x, y, DASH.0.min(rc.right - x), 1, ink));
        }
        let y0 = &ch.ymaps[0];
        let fmt = NumFmt::new(y0.min.abs().max(y0.max.abs()), None, ch.lang());
        ch.pill(over, rc.right + 1, y, fmt.fmt(v), ink);
    }
    if let Some((x, w, label)) = ch.date_pill() {
        let size = ch.size("label");
        let top = ch.last_rect().bottom + 1;
        let baseline = ch.time_baseline();
        over.push(rect(
            x,
            top,
            w,
            baseline + px(f64::from(size) * 0.3) - top,
            ink,
        ));
        let bg = ch.theme().bg;
        over.push(text(x + PILL_PAD_X, baseline, size, label, bg, "label"));
    }
}
