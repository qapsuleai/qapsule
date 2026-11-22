//! Volume (OV-10): a soft up/down histogram along the bottom `chart.volume_share` of the
//! price panel, under the candles (OpenChart, Vela). A view's own volume layer keeps its
//! pane and takes the same soft alpha.

use super::*;
use crate::draw::body_width;

/// Alpha of volume bars: read after the price, not before it.
pub(super) const VOLUME_ALPHA: f64 = 0.35;

/// Whether `pl` draws the bar series' volume.
fn draws_volume(pl: &Prepared, bar_series: &str) -> bool {
    pl.geom == Geom::Bar
        && matches!(&pl.data, DataSource::Series { series } if series == bar_series)
        && pl.fields.iter().any(|f| f == "v")
}

/// The histogram under the first panel's candles, when the view has no volume of its own.
pub(in crate::compile) fn volume(ch: &Chart<'_>, prepared: &[Prepared], under: &mut Vec<Prim>) {
    let (fr, share) = (ch.fr, ch.look.chart.volume_share);
    let candles = prepared.iter().any(|pl| {
        pl.panel == ch.panel0
            && pl.geom == Geom::Candle
            && matches!(&pl.data, DataSource::Series { series } if *series == fr.bar_series)
    });
    if !ch.look.familiar()
        || share <= 0.0
        || !candles
        || prepared.iter().any(|pl| draws_volume(pl, &fr.bar_series))
    {
        return;
    }
    let buckets: Vec<(u64, u64, f64, bool)> = fr
        .buckets()
        .filter_map(|(s, n)| {
            let bars = fr.bars.get(usize::try_from(s).ok()?..)?;
            let bars = &bars[..usize::try_from(n).ok()?.min(bars.len())];
            let v: f64 = bars.iter().map(|b| b.v).filter(|v| v.is_finite()).sum();
            let up = bars.last()?.c >= bars.first()?.o;
            Some((s, n, v, up))
        })
        .collect();
    let vmax = buckets.iter().map(|b| b.2).fold(0.0, f64::max);
    if vmax <= 0.0 {
        return;
    }
    let rc = ch.rects[0];
    let room = f64::from(rc.bottom - rc.top) * share.min(1.0);
    #[allow(clippy::cast_precision_loss)]
    let bw = body_width(ch.x.slot * fr.k as f64, ch.look.chart.body_ratio);
    let theme = ch.theme();
    for (s, n, v, up) in buckets {
        let h = px(v / vmax * room);
        #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
        let cx = (ch.x.left + ((s - fr.from) as f64 + n as f64 / 2.0) * ch.x.slot).floor() as i32;
        let color = with_opacity(if up { theme.up } else { theme.down }, VOLUME_ALPHA);
        under.push(rect(cx - bw / 2, rc.bottom - h, bw, h, color));
    }
}

/// The soft alpha on the bars of a view's own volume layer; a level's origin stroke.
pub(in crate::compile) fn soften(ch: &Chart<'_>, pl: &Prepared, prims: &mut Vec<Prim>) {
    super::levels::stroke(ch, pl, prims);
    if !ch.look.familiar()
        || ch.look.chart.volume_share <= 0.0
        || !draws_volume(pl, &ch.fr.bar_series)
    {
        return;
    }
    for p in prims {
        if let Prim::Rect { color, .. } = p {
            *color = with_opacity(*color, VOLUME_ALPHA);
        }
    }
}
