//! The time axis (P5, OV-6): R0's months, or two levels on one row: minor labels (days,
//! months, clock times) `muted`, the major unit (month, year, day) `text`. The level is the
//! finest whose labels are on average a label and a gap apart; labels are placed major
//! first ("a date beats the 12:00 beside it"), then the first bar's context, then minor,
//! each left out when it would meet a placed label, the date pill or the frame edge.

use super::format::{civil, day_month, hm, month, DAY_MS};
use super::*;

/// Least average room between two time labels when choosing the level (px).
pub(super) const LABEL_GAP: i32 = 8;
/// Least room between two placed labels (px).
pub(super) const PLACE_GAP: i32 = 6;

#[derive(Debug, Clone, Copy)]
enum Level {
    Minutes(i64),
    Days(i64),
    Months(i64),
    Years(i64),
}

/// Steps of the levels (minutes, days, months, years), finest first.
const STEPS: ([i64; 11], [i64; 5], [i64; 3], [i64; 4]) = (
    [1, 2, 5, 10, 15, 30, 60, 120, 180, 360, 720],
    [1, 2, 5, 10, 15],
    [1, 3, 6],
    [1, 2, 5, 10],
);

/// The levels, finest first.
fn levels() -> impl Iterator<Item = Level> {
    let (mi, d, mo, y) = STEPS;
    (mi.into_iter().map(Level::Minutes))
        .chain(d.into_iter().map(Level::Days))
        .chain(mo.into_iter().map(Level::Months))
        .chain(y.into_iter().map(Level::Years))
}

impl Level {
    /// The minor bucket and the major unit a time falls in.
    fn keys(self, t: i64) -> (i64, i64) {
        let (y, m, d) = civil(t);
        let day = t.div_euclid(DAY_MS);
        match self {
            Level::Minutes(s) => (day * 10_000 + t.rem_euclid(DAY_MS) / 60_000 / s, day),
            Level::Days(s) => ((y * 12 + m) * 100 + (d - 1) / s, y * 12 + m),
            Level::Months(s) => (y * 100 + (m - 1) / s, y),
            Level::Years(s) => (y.div_euclid(s), y.div_euclid(s)),
        }
    }

    fn label(self, t: i64, major: bool, lang: Language) -> String {
        let (y, m, d) = civil(t);
        match (self, major) {
            (Level::Minutes(_), false) => hm(t),
            (Level::Minutes(_), true) => day_month(t, lang),
            (Level::Days(_), false) => d.to_string(),
            (Level::Days(_), true) if m != 1 => month(m, lang).to_owned(),
            (Level::Months(_), false) => month(m, lang).to_owned(),
            _ => y.to_string(),
        }
    }

    /// The widest minor label of this level (for the spacing test).
    fn sample(self, ch: &Chart<'_>) -> i32 {
        let lang = ch.lang();
        match self {
            Level::Minutes(_) => ch.width("00:00", "label"),
            Level::Days(_) => ch.width("28", "label"),
            Level::Months(_) => (1..=12)
                .map(|m| ch.width(month(m, lang), "label"))
                .max()
                .unwrap_or(0),
            Level::Years(_) => ch.width("2000", "label"),
        }
    }
}

/// One time tick: slot, pixel column, local time, major, context (the first visible bar).
type Tick = (u64, i32, i64, bool, bool);

/// The ticks of `level` in the visible range (first slot of each LOD bucket).
fn ticks(ch: &Chart<'_>, level: Level) -> Vec<Tick> {
    let fr = ch.fr;
    let mut out = Vec::new();
    let mut prev: Option<(i64, i64)> = None;
    for (s, _) in fr.buckets() {
        let Some(t) = fr.slot_time(s).map(|t| ch.local(t)) else {
            continue;
        };
        let k = level.keys(t);
        let before = if s == fr.from && s > 0 {
            fr.slot_time(s - 1).map(|t| level.keys(ch.local(t)))
        } else {
            prev
        };
        let minor = before.is_none_or(|b| b.0 != k.0);
        let major = before.is_none_or(|b| b.1 != k.1);
        if minor || s == fr.from {
            #[allow(clippy::cast_possible_truncation)]
            let x = crate::scale::slot_x(ch.x, s) as i32;
            out.push((s, x, t, major || s == fr.from, s == fr.from && !major));
        }
        prev = Some(k);
    }
    out
}

/// Whether the visible bars are less than a day apart.
fn intraday(ch: &Chart<'_>) -> bool {
    let fr = ch.fr;
    (fr.from..fr.to).any(
        |s| matches!((fr.slot_time(s), fr.slot_time(s + 1)), (Some(a), Some(b)) if b - a < DAY_MS),
    )
}

/// The vertical grid and the time labels.
pub(in crate::compile) fn time_axis(ch: &Chart<'_>, under: &mut Vec<Prim>) {
    if !ch.look.familiar() {
        legacy(ch, under);
        return;
    }
    let plot_w = ch.x.slot * f64::from(u32::try_from(ch.fr.to - ch.fr.from + 1).unwrap_or(1));
    let daily = !intraday(ch);
    let levels = levels().filter(|l| !(daily && matches!(l, Level::Minutes(_))));
    let mut chosen = (Level::Years(10), Vec::new());
    for l in levels {
        let ts = ticks(ch, l);
        #[allow(clippy::cast_precision_loss)]
        let real = ts.iter().filter(|t| !t.4).count().max(1) as f64;
        if plot_w / real >= f64::from(l.sample(ch) + LABEL_GAP) {
            chosen = (l, ts);
            break;
        }
    }
    let (level, ts) = chosen;
    let ts = if ts.is_empty() { ticks(ch, level) } else { ts };
    place(ch, level, &ts, under);
}

/// Places the labels of `ts` in priority order and draws them with their grid lines.
fn place(ch: &Chart<'_>, level: Level, ts: &[Tick], under: &mut Vec<Prim>) {
    let theme = ch.theme();
    let (size, lang) = (ch.size("label"), ch.lang());
    let baseline = ch.time_baseline();
    let mut taken: Vec<(i32, i32)> = ch
        .date_pill()
        .map(|(x, w, _)| (x, x + w))
        .into_iter()
        .collect();
    // Priority: the major unit, then the context label, then minor labels; in time order.
    let mut order: Vec<&Tick> = ts.iter().collect();
    order.sort_by_key(|t| {
        (
            if t.4 {
                1
            } else if t.3 {
                0
            } else {
                2
            },
            t.0,
        )
    });
    let mut placed: Vec<(Tick, i32, String)> = Vec::new();
    for t in order {
        let label = level.label(t.2, t.3, lang);
        let w = ch.width(&label, "label");
        let l = (t.1 - w / 2).clamp(0, (ch.w - w).max(0));
        let free = taken
            .iter()
            .all(|(a, b)| l + w + PLACE_GAP <= *a || l >= b + PLACE_GAP);
        if free {
            taken.push((l, l + w));
            placed.push((*t, l, label));
        }
    }
    placed.sort_by_key(|p| p.0 .0);
    for (t, l, label) in placed {
        if !t.4 {
            for (rc, _) in ch.rects.iter().zip(ch.card).filter(|(_, c)| !**c) {
                under.push(rect(t.1, rc.top, 1, rc.bottom - rc.top, theme.grid));
            }
        }
        let color = if t.3 { theme.text } else { theme.muted };
        under.push(text(l, baseline, size, label, color, "label"));
    }
}

/// R0's vertical grid and month labels (labels under the bottom panel with a scale).
fn legacy(ch: &Chart<'_>, under: &mut Vec<Prim>) {
    let (fr, theme) = (ch.fr, ch.theme());
    let last_rect = ch.last_rect();
    let mut prev_month: Option<(i64, i64)> = None;
    let mut last_label_x = i32::MIN;
    for s in fr.from..=fr.to {
        let Some(t) = fr.slot_time(s) else { continue };
        let ym = year_month(t.div_euclid(86_400_000));
        if prev_month.is_some_and(|p| p != ym) {
            #[allow(clippy::cast_possible_truncation)]
            let x = crate::scale::slot_x(ch.x, s) as i32;
            if x.saturating_sub(last_label_x) >= 72 {
                for (rc, _) in ch.rects.iter().zip(ch.card).filter(|(_, c)| !**c) {
                    under.push(rect(x, rc.top, 1, rc.bottom - rc.top, theme.grid));
                }
                let label = format!("{:04}-{:02}", ym.0, ym.1);
                let (b, size) = (last_rect.bottom + 16, ch.size("label"));
                under.push(text(x - 21, b, size, label, theme.muted, "label"));
                last_label_x = x;
            }
        }
        prev_month = Some(ym);
    }
}
