//! Which series a view draws: the first bar series, point series and the slot times after
//! the last bar.

use super::*;

/// The view's first bar series (`docs/GRAMMAR.md` §1).
pub(super) fn first_bar_series(view: &View, p: &dyn DataProvider) -> Option<String> {
    let bars = |s: &str| p.series_kind(s) == Some(SeriesKind::Bars);
    view.layers
        .iter()
        .filter(|l| l.mark == Mark::Candle)
        .find_map(|l| l.series.clone().filter(|s| bars(s)))
        .or_else(|| {
            view.layers
                .iter()
                .find_map(|l| l.series.clone().filter(|s| bars(s)))
        })
        .or_else(|| {
            view.layers.iter().find_map(|l| {
                let s = l.series.as_ref()?;
                (p.series_kind(s) == Some(SeriesKind::Values))
                    .then(|| p.input_bars(s))
                    .flatten()
                    .filter(|b| bars(b))
            })
        })
        .or_else(|| {
            // A fact/finding layer: the bar series of its (first) fact's series.
            view.layers.iter().find_map(|l| {
                let fact = match (&l.fact, &l.finding) {
                    (Some(f), _) => p.fact(f),
                    (_, Some(f)) => p
                        .finding(f)
                        .and_then(|x| x.facts.first().and_then(|f| p.fact(f))),
                    _ => None,
                }?;
                match p.series_kind(&fact.series)? {
                    SeriesKind::Bars => Some(fact.series.clone()),
                    SeriesKind::Values => p.input_bars(&fact.series).filter(|b| bars(b)),
                    SeriesKind::Points => None,
                }
            })
        })
}

/// Every point series the view reads (layer data and channel references).
/// Times of the slots after the last bar (`last_t`): the k-th point after the last bar
/// of any point series the view draws sits in slot `last bar + k` (`docs/GRAMMAR.md` §3);
/// the first series to reach a slot gives its time.
pub(super) fn ext_times(view: &View, provider: &dyn DataProvider, last_t: i64) -> Vec<i64> {
    let mut out: Vec<i64> = Vec::new();
    for s in point_series(view, provider) {
        if let Some(pts) = provider.points(&s) {
            for (k, pt) in pts.iter().filter(|pt| pt.t > last_t).enumerate() {
                if k >= out.len() {
                    out.push(pt.t);
                }
            }
        }
    }
    out
}

/// Times of the view's future slots (slots after the last bar of its bar series, taken
/// by point series such as a forecast fan; `docs/GRAMMAR.md` §3), in slot order. Empty
/// when the view has no bar series, no bars or no point after the last bar. The
/// authority uses it to let a connection's viewport reach the future slots (K25).
#[must_use]
pub fn future_times(view: &View, provider: &dyn DataProvider) -> Vec<i64> {
    let Some(bs) = first_bar_series(view, provider) else {
        return Vec::new();
    };
    match provider
        .bars(&bs)
        .as_deref()
        .and_then(<[qp_schema::Bar]>::last)
    {
        Some(last) => ext_times(view, provider, last.t),
        None => Vec::new(),
    }
}

fn point_series(view: &View, p: &dyn DataProvider) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut add = |s: &str| {
        if p.series_kind(s) == Some(SeriesKind::Points) && !out.iter().any(|x| x == s) {
            out.push(s.to_owned());
        }
    };
    for l in &view.layers {
        if let Some(s) = &l.series {
            add(s);
        }
        for pos in [&l.encoding.x, &l.encoding.y, &l.encoding.y2]
            .into_iter()
            .flatten()
        {
            if let Position::Ref(r) = pos {
                if let Some(s) = &r.series {
                    add(s);
                }
            }
        }
        if let Some(ColorSpec::When(w)) = &l.encoding.color {
            if let Operand::Ref(r) = &w.when.than {
                if let Some(s) = &r.series {
                    add(s);
                }
            }
        }
    }
    out
}
