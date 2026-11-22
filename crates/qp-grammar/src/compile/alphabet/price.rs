//! The price axis (P4, OV-5): R0's `height / 90` divisions, or one label about every
//! `chart.price_label_px` (nearest step on [`STEPS`] × 10^k) inside its panel and the gutter, none under the last-value tag or the price pill. A
//! short pane whose labels all meet the tag keeps one label from a finer step.

use super::format::NumFmt;
use super::*;

/// Step multipliers of a price axis. Finer than 1-2-5 (neighbours at most 1.5× apart), so
/// the spacing stays within ±22 % of the look's target at every panel height (P4).
const STEPS: [f64; 9] = [1.0, 1.5, 2.0, 2.5, 3.0, 4.0, 5.0, 7.5, 10.0];

/// Grid lines and value labels of every panel with a value scale.
pub(in crate::compile) fn price_axis(ch: &Chart<'_>, under: &mut Vec<Prim>) {
    if !ch.look.familiar() {
        legacy(ch, under);
        return;
    }
    let theme = ch.theme();
    let fig = ch.size("figure");
    for (pi, rc) in ch.rects.iter().enumerate().filter(|(i, _)| !ch.card[*i]) {
        let y = &ch.ymaps[pi];
        let half = f64::from(ch.pill_h() / 2);
        let tag = ch.tags[pi].filter(|_| ch.look.chart.last_value_tag);
        let pill = ch.at.and_then(|a| a.price).filter(|_| pi == 0);
        let levels = ch.levels.iter().filter(|l| l.0 == pi).map(|l| l.1);
        let keep_out: Vec<f64> = [tag.map(|t| t.0), pill.map(|p| p.1)]
            .into_iter()
            .flatten()
            .chain(levels)
            .map(f64::from)
            .collect();
        let x = rc.right + PILL_PAD_X;
        let mut drawn = false;
        // Pass 0: the grid and its labels; passes 1–2 (halved steps): one label at most.
        for pass in 0..3 {
            let h = f64::from(rc.bottom - rc.top) * f64::from(1 << pass);
            let (vals, step) = ticks_at(y, h, ch.look.chart);
            let fmt = NumFmt::new(y.min.abs().max(y.max.abs()), step, ch.lang());
            for v in vals {
                let Some(yy) = value_y(y, v) else { continue };
                let yy = px(yy);
                if yy <= rc.top || yy >= rc.bottom || (pass > 0 && drawn) {
                    continue;
                }
                if pass == 0 {
                    under.push(rect(rc.left, yy, rc.right - rc.left, 1, theme.grid));
                }
                let label = fmt.fmt(v);
                let baseline = yy + ch.mid();
                let (top, bottom) = ch.vbox(baseline, "figure");
                let clear = keep_out
                    .iter()
                    .all(|k| bottom <= k - half || top >= k + half);
                let inside = top >= f64::from(rc.top) && bottom <= f64::from(rc.bottom);
                if clear && inside && x + ch.width(&label, "figure") < ch.w {
                    under.push(text(x, baseline, fig, label, theme.muted, "figure"));
                    drawn = true;
                }
            }
            if drawn {
                break;
            }
        }
    }
}

/// Tick values of a panel `h` px high and the step between them (log: none).
fn ticks_at(y: &YMap, h: f64, chart: crate::look::ChartLook) -> (Vec<f64>, Option<f64>) {
    let divs = h / chart.price_label_px;
    if y.kind == ScaleKind::Log {
        return (ticks(y, divs.round().max(1.0)), None);
    }
    let raw = (y.max - y.min) / divs;
    let Some(e) = detmath::log10(raw).filter(|e| e.is_finite()) else {
        return (Vec::new(), None);
    };
    #[allow(clippy::cast_possible_truncation)]
    let mag = detmath::pow10_int(e.floor() as i32);
    let off = |c: &&f64| {
        let r = *c * mag / raw;
        r.max(1.0 / r)
    };
    let Some(m) = STEPS.iter().min_by(|a, b| off(a).total_cmp(&off(b))) else {
        return (Vec::new(), None);
    };
    let step = m * mag;
    let k0 = (y.min / step).ceil();
    if !(step.is_finite() && step > 0.0 && k0.is_finite() && k0.abs() < MAX_EXACT_TICK) {
        return (Vec::new(), None);
    }
    let vals = (0..MAX_TICKS)
        .map(|i| (k0 + f64::from(i)) * step)
        .take_while(|v| *v <= y.max)
        .collect();
    (vals, Some(step))
}

/// R0's horizontal grid and value labels (line, label, line, label…), a label under the
/// tag left out.
fn legacy(ch: &Chart<'_>, under: &mut Vec<Prim>) {
    let theme = ch.theme();
    let figure_px = ch.size("figure");
    for (((rc, y), tag), _) in ch
        .rects
        .iter()
        .zip(ch.ymaps)
        .zip(ch.tags)
        .zip(ch.card)
        .filter(|(_, c)| !**c)
    {
        let divs = (f64::from(rc.bottom - rc.top) / 90.0)
            .round()
            .clamp(2.0, 6.0);
        for v in ticks(y, divs) {
            let Some(yy) = value_y(y, v) else { continue };
            let yy = px(yy);
            if yy > rc.top && yy < rc.bottom {
                under.push(rect(rc.left, yy, rc.right - rc.left, 1, theme.grid));
                // The label's box (baseline yy + 4, figure size tall) under the tag's box
                // (ly - 8 .. ly + 8): the tag wins, the label is left out.
                #[allow(clippy::cast_possible_truncation)]
                let label_top = yy + 4 - figure_px as i32;
                if tag.is_some_and(|(ly, _, _)| label_top < ly + 8 && yy + 4 > ly - 8) {
                    continue;
                }
                let label = format!("{v:.2}");
                under.push(text(
                    rc.right + 6,
                    yy + 4,
                    figure_px,
                    label,
                    theme.muted,
                    "figure",
                ));
            }
        }
    }
}
