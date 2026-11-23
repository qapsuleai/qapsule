//! The title row's figures (OV-7): R0's "C <close>", or the OHLC legend of the cursor's
//! (else the last visible) bar: letters muted, values `text`, change and change % in the
//! direction colour; touch: C and change %; parts drop in a fixed order when narrow.
//! Letters O H L C in both languages: Turkish A Y D K is not in the glossary yet.

use super::format::{signed, NumFmt};
use super::*;
use qp_look::look::Profile;

/// Field letters (open, high, low, close).
const LETTERS: [&str; 4] = ["O", "H", "L", "C"];
/// Room between a letter and its value, and between two parts (px).
pub(super) const GAPS: (i32, i32) = (3, 8);

/// One legend part: letter (if any), value text, value colour.
type Part = (Option<&'static str>, String, Rgba);

/// Draws the figures right of the title (its right edge `title_right`); returns the
/// right edge of what was drawn.
pub(in crate::compile) fn title_figures(
    ch: &Chart<'_>,
    close_x: i32,
    title_right: i32,
    over: &mut Vec<Prim>,
) -> i32 {
    let (fr, theme) = (ch.fr, ch.theme());
    let last_bar = usize::try_from(fr.to.min(fr.n_bars() - 1)).unwrap_or(0);
    if !(ch.look.familiar() && ch.look.chart.ohlc_legend) {
        return close(ch, close_x, last_bar, over);
    }
    let i = ch
        .at
        .and_then(|a| usize::try_from(a.slot).ok())
        .map_or(last_bar, |s| s.min(fr.bars.len() - 1));
    let b = &fr.bars[i];
    let lang = ch.lang();
    let fmt = NumFmt::new(b.h.abs().max(b.l.abs()), None, lang);
    let prev = i.checked_sub(1).map_or(b.o, |j| fr.bars[j].c);
    let chg = b.c - prev;
    let dir = if chg >= 0.0 { theme.up } else { theme.down };
    let c: Part = (Some(LETTERS[3]), fmt.fmt(b.c), theme.text);
    let pct: Part = (None, signed(chg / prev * 100.0, 2, true, lang), dir);
    let pct = Some(pct).filter(|_| prev != 0.0 && prev.is_finite());
    let ohlc: Vec<Part> = [b.o, b.h, b.l, b.c]
        .iter()
        .zip(LETTERS)
        .map(|(v, l)| (Some(l), fmt.fmt(*v), theme.text))
        .collect();
    let mut full = ohlc.clone();
    full.push((None, signed(chg, 2, false, lang), dir));
    full.extend(pct.clone());
    let short: Vec<Part> = std::iter::once(c.clone()).chain(pct).collect();
    let tries = if ch.look.profile == Profile::Touch {
        vec![short, vec![c]]
    } else {
        vec![full, ohlc, short, vec![c]]
    };
    let x0 = title_right + ch.look.offsets.legend_gap;
    let room = ch.w - ch.look.offsets.row_edge - x0;
    let width = |ps: &[Part]| -> i32 {
        ps.iter()
            .map(|(l, v, _)| {
                l.map_or(0, |l| ch.width(l, "figure") + GAPS.0) + ch.width(v, "figure")
            })
            .sum::<i32>()
            + GAPS.1 * (i32::try_from(ps.len()).unwrap_or(1) - 1)
    };
    let Some(parts) = tries.iter().find(|ps| width(ps) <= room) else {
        return title_right;
    };
    let (size, baseline) = (ch.size("figure"), ch.look.offsets.title_baseline);
    let mut x = x0;
    for (l, v, color) in parts {
        if let Some(l) = l {
            over.push(text(
                x,
                baseline,
                size,
                (*l).to_owned(),
                theme.muted,
                "figure",
            ));
            x += ch.width(l, "figure") + GAPS.0;
        }
        over.push(text(x, baseline, size, v.clone(), *color, "figure"));
        x += ch.width(v, "figure") + GAPS.1;
    }
    x - GAPS.1
}

/// R0's "C <close>" of the last visible bar, in the direction colour.
fn close(ch: &Chart<'_>, close_x: i32, last_bar: usize, over: &mut Vec<Prim>) -> i32 {
    let (fr, theme) = (ch.fr, ch.theme());
    let title_px = ch.size("title");
    let lb = &fr.bars[last_bar];
    let label = format!("C {:.2}", lb.c);
    let right = close_x + ch.width(&label, "title");
    let color = if lb.c >= lb.o { theme.up } else { theme.down };
    let baseline = ch.look.offsets.title_baseline;
    over.push(super::text(
        close_x, baseline, title_px, label, color, "title",
    ));
    right
}
