//! (RD2/C2a) The chart's familiar alphabet from the look's [`crate::look::ChartLook`]
//! (VISUAL §5 P4–P8; ADOPTED OV-5…OV-10, OV-12; `docs/GRAMMAR.md` §9a). A legacy look
//! ([`Look::familiar`] false) takes R0's paths, moved here unchanged (goldens byte-equal).
//! Texts are built here in the reader's language; collision avoidance is deterministic: a
//! label that would meet a tag, a pill, another label or the frame edge is left out.

use qp_schema::protocol::Language;

use super::*;
use crate::look::Look;
use crate::theme::Theme;

mod format;
mod legend;
mod marks;
mod price;
mod time;
mod volume;

pub(super) use legend::title_figures;
pub(super) use marks::{crosshair, tags};
pub(super) use volume::{soften, volume};

/// Height a tag or pill adds to the figure size (px).
const PILL_PAD_Y: f64 = 6.0;
/// Baseline offset that centres a figure on a pixel row, in em (R0: 4 at 11 px).
const MID: f64 = 0.36;
/// Time-label baseline below the plot beyond the label size (px; R0: 16 at 11 px).
const BASELINE_PAD: f64 = 5.0;
/// Inset of a tag's, a pill's or a price label's text (px; R0's tag inset).
const PILL_PAD_X: i32 = 6;

/// (RD2/C2a-fix) The fixed alphabet measures (not in the look record; a direction cannot
/// change them): `docs/design/CONTRACT.md` "Sabit alfabe ölçüleri" lists the same values.
#[allow(clippy::cast_lossless)]
pub const ALPHABET_FIXED: [(&str, &[f64]); 10] = [
    ("VOLUME_ALPHA", &[volume::VOLUME_ALPHA]),
    ("DASH", &[marks::DASH.0 as f64, marks::DASH.1 as f64]),
    ("DOT", &[marks::DOT.0 as f64, marks::DOT.1 as f64]),
    ("GAPS", &[legend::GAPS.0 as f64, legend::GAPS.1 as f64]),
    ("PILL_PAD_X", &[PILL_PAD_X as f64]),
    ("PILL_PAD_Y", &[PILL_PAD_Y]),
    ("MID", &[MID]),
    ("BASELINE_PAD", &[BASELINE_PAD]),
    ("LABEL_GAP", &[time::LABEL_GAP as f64]),
    ("PLACE_GAP", &[time::PLACE_GAP as f64]),
];

/// The cursor resolved to the frame.
#[derive(Debug, Clone, Copy)]
pub(super) struct At {
    slot: u64,
    x: i32,
    /// Value on the first panel's scale and its pixel row (inside that panel).
    price: Option<(f64, i32)>,
}

/// What the alphabet reads from one compile.
pub(super) struct Chart<'a> {
    pub fr: &'a Frame<'a>,
    pub look: &'static Look,
    pub x: &'a XMap,
    pub rects: &'a [PixelRect],
    pub ymaps: &'a [YMap],
    /// Card panels (no value scale).
    pub card: &'a [bool],
    /// Last-value tags per panel: pixel row, value, colour.
    pub tags: &'a [Option<(i32, f64, Rgba)>],
    pub panel0: u32,
    pub w: i32,
    pub at: Option<At>,
    /// The view's display zone (K14; `None`: UTC).
    pub tz: Option<qp_calendar::Zone>,
}

impl Chart<'_> {
    /// This chart with the cursor `c` resolved (a legacy look, or one without crosshair
    /// pills, ignores it).
    pub(super) fn with_cursor(mut self, c: Option<Cursor>) -> Self {
        let on = self.look.familiar() && self.look.chart.crosshair_pills;
        self.at = c.filter(|_| on).map(|c| {
            let fr = self.fr;
            let slot = c.bar.clamp(fr.from, fr.to);
            let close = usize::try_from(slot)
                .ok()
                .and_then(|i| fr.bars.get(i))
                .map(|b| b.c);
            let rc = self.rects[0];
            let price = c.price.or(close).and_then(|v| {
                let y = px(value_y(&self.ymaps[0], v)?);
                (!self.card[0] && y > rc.top && y < rc.bottom).then_some((v, y))
            });
            #[allow(clippy::cast_possible_truncation)]
            let x = crate::scale::slot_x(self.x, slot) as i32;
            At { slot, x, price }
        });
        self
    }

    fn theme(&self) -> &Theme {
        &self.fr.theme
    }

    /// Bar time `t` (ms) as wall-clock ms in the display zone (labels read local time).
    fn local(&self, t: i64) -> i64 {
        t + self
            .tz
            .map_or(0, |z| i64::from(z.offset_at(t.div_euclid(1000))) * 1000)
    }

    fn lang(&self) -> Language {
        self.fr.p.reader_lang().unwrap_or_default()
    }

    /// The size of type role `role`.
    #[allow(clippy::cast_possible_truncation)]
    fn size(&self, role: &str) -> f32 {
        self.look.types.role(role).map_or(11.0, |r| r.px as f32)
    }

    fn width(&self, text: &str, role: &str) -> i32 {
        est_width(text, f64::from(self.size(role)), self.look.role_font(role))
    }

    /// Top and bottom of a text in `role` on `baseline` (the font's ascent; descent 0.2 em).
    fn vbox(&self, baseline: i32, role: &str) -> (f64, f64) {
        let s = f64::from(self.size(role));
        let b = f64::from(baseline);
        (b - self.look.role_font(role).ascent * s, b + 0.2 * s)
    }

    /// Height of a tag or pill.
    fn pill_h(&self) -> i32 {
        px(f64::from(self.size("figure")) + PILL_PAD_Y)
    }

    /// Baseline offset that centres a figure on a pixel row.
    fn mid(&self) -> i32 {
        px(f64::from(self.size("figure")) * MID)
    }

    /// The last panel with a value scale (time labels go under it; R5/M20).
    fn last_rect(&self) -> PixelRect {
        let n = self.rects.len() - 1;
        if !self.card[n] {
            return self.rects[n];
        }
        self.rects
            .iter()
            .zip(self.card)
            .rev()
            .find(|(_, c)| !**c)
            .map_or(self.rects[n], |(r, _)| *r)
    }

    /// Baseline of the time labels and the date pill.
    fn time_baseline(&self) -> i32 {
        self.last_rect().bottom + px(f64::from(self.size("label")) + BASELINE_PAD)
    }

    /// A legend value in the title row: R0's two decimals, or the reader's language.
    pub(super) fn num(&self, v: f64) -> String {
        if self.look.familiar() {
            format::NumFmt::new(v.abs(), None, self.lang()).fmt(v)
        } else {
            format!("{v:.2}")
        }
    }
}

/// The price axes of the panels, then the time axis.
pub(super) fn axes(ch: &Chart<'_>, under: &mut Vec<Prim>) {
    price::price_axis(ch, under);
    time::time_axis(ch, under);
}

/// A compiler text in `role`.
fn text(x: i32, baseline: i32, px: f32, text: String, color: Rgba, role: &str) -> Prim {
    Prim::Text {
        x,
        baseline,
        px,
        text,
        color,
        align: Align::Left,
        font: None,
        role: Some(role.to_owned()),
    }
}

/// The plot margins: the look's, with the right one its axis gutter under chart
/// conventions.
pub(super) fn margins(look: &Look) -> [i32; 4] {
    let mut m = look.space.plot;
    if look.familiar() {
        m[1] = px(look.chart.axis_gutter_px);
    }
    m
}
