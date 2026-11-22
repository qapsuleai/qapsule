//! Value rows of a series or a pair of channels, LOD-aggregated when the frame says so, and
//! the text field of a `text` layer.

use super::*;

/// The text field of a `text` layer: the `text` channel, else `claim` (finding) / `text`
/// (fact); it must exist in the layer's rows.
pub(super) fn text_field_of(cx: &Ctx<'_, '_>) -> Result<String, GrammarError> {
    let layer = cx.layer;
    let f = layer
        .encoding
        .text
        .clone()
        .or_else(|| match &layer.finding {
            Some(_) => Some("claim".to_owned()),
            None => layer.fact.as_ref().map(|_| "text".to_owned()),
        });
    let f = f.ok_or_else(|| {
        cx.err(
            ErrorKind::Invalid,
            "/encoding/text",
            "`text` needs the `text` channel",
        )
    })?;
    if !cx.all_rows.iter().any(|r| r.texts.contains_key(&f)) {
        let mut names: Vec<&str> = Vec::new();
        for r in &cx.all_rows {
            for k in r.texts.keys() {
                if !names.contains(&k.as_str()) {
                    names.push(k);
                }
            }
        }
        return Err(cx.err(
            ErrorKind::NotFound,
            "/encoding/text",
            format!("text field `{f}` not found (fields: {})", names.join(", ")),
        ));
    }
    Ok(f)
}

#[derive(Clone, Copy)]
pub(super) enum RowAgg {
    Line,
    Envelope,
    MaxAbs,
}

#[allow(clippy::too_many_lines)]
/// Rows of a single-valued channel (lines). Returns rows, skipped count, LOD-line flag.
pub(super) fn series_rows(
    cx: &Ctx<'_, '_>,
    y: &Chan<'_>,
    rule: &ColorRule<'_>,
    opacity: f64,
    _agg: RowAgg,
) -> (Vec<PRow>, u64, bool) {
    let fr = cx.fr;
    let mut rows = Vec::new();
    let mut skipped = 0u64;
    let mk = |slot: u64, n: u64, v: [Option<f64>; 4], at: u64, row: Option<&Row>| {
        let (color, ckey) = rule.at(at, row, None);
        PRow {
            slot,
            n,
            t: fr.slot_time(slot),
            v,
            color: with_opacity(color, opacity),
            ckey,
            size: 0.0,
            text: None,
        }
    };
    match cx.src {
        Src::Rows => {
            for r in cx.visible_rows() {
                match y.at(r.slot, Some(r)) {
                    Some(v) => {
                        rows.push(mk(r.slot, 1, [Some(v), None, None, None], r.slot, Some(r)))
                    }
                    None => skipped += 1,
                }
            }
            (rows, skipped, false)
        }
        _ if fr.k <= 1 => {
            for s in fr.from..=fr.to {
                match y.at(s, None) {
                    Some(v) => rows.push(mk(s, 1, [Some(v), None, None, None], s, None)),
                    None => {
                        if y.has(s) {
                            skipped += 1;
                        }
                        rows.push(mk(s, 1, [None; 4], s, None));
                    }
                }
            }
            (rows, skipped, false)
        }
        _ => {
            for (s0, n) in fr.buckets() {
                let mut first = None;
                let mut last = None;
                let mut lo: Option<(u64, f64)> = None;
                let mut hi: Option<(u64, f64)> = None;
                let mut last_slot = s0;
                for s in s0..s0 + n {
                    match y.at(s, None) {
                        Some(v) => {
                            if first.is_none() {
                                first = Some(v);
                            }
                            last = Some(v);
                            last_slot = s;
                            if lo.is_none_or(|(_, x)| v < x) {
                                lo = Some((s, v));
                            }
                            if hi.is_none_or(|(_, x)| v > x) {
                                hi = Some((s, v));
                            }
                        }
                        None => {
                            if y.has(s) {
                                skipped += 1;
                            }
                        }
                    }
                }
                let v = match (first, lo, hi, last) {
                    (Some(f), Some((ls, l)), Some((hs, h)), Some(z)) => {
                        let (a, b) = if ls <= hs { (l, h) } else { (h, l) };
                        [Some(f), Some(a), Some(b), Some(z)]
                    }
                    _ => [None; 4],
                };
                rows.push(mk(s0, n, v, last_slot, None));
            }
            (rows, skipped, true)
        }
    }
}

#[allow(clippy::too_many_lines)]
/// Rows of a two-valued mark (area, band, bar): `v = [y, y2]`.
pub(super) fn pair_rows(
    cx: &Ctx<'_, '_>,
    y: &Chan<'_>,
    y2: Option<&Chan<'_>>,
    rule: &ColorRule<'_>,
    opacity: f64,
    agg: RowAgg,
    extent: Option<(u64, u64)>,
) -> (Vec<PRow>, u64) {
    let inside = |s: u64| extent.is_none_or(|(a, b)| s >= a && s <= b);
    let fr = cx.fr;
    let mut rows = Vec::new();
    let mut skipped = 0u64;
    let both = |s: u64, row: Option<&Row>| -> Option<(f64, Option<f64>)> {
        let a = y.at(s, row)?;
        match y2 {
            Some(c) => c.at(s, row).map(|b| (a, Some(b))),
            None => Some((a, None)),
        }
    };
    let mk = |slot: u64, n: u64, v: Option<(f64, Option<f64>)>, at: u64, row: Option<&Row>| {
        let (color, ckey) = rule.at(at, row, None);
        PRow {
            slot,
            n,
            t: fr.slot_time(slot),
            v: match v {
                Some((a, b)) => [Some(a), b, None, None],
                None => [None; 4],
            },
            color: with_opacity(color, opacity),
            ckey,
            size: 0.0,
            text: None,
        }
    };
    let is_bar = matches!(agg, RowAgg::MaxAbs);
    match cx.src {
        Src::Rows => {
            for r in cx.visible_rows().filter(|r| inside(r.slot)) {
                match both(r.slot, Some(r)) {
                    Some(v) => rows.push(mk(r.slot, 1, Some(v), r.slot, Some(r))),
                    None => skipped += 1,
                }
            }
        }
        _ => {
            for (s0, n) in fr.buckets() {
                let mut acc: Option<(f64, Option<f64>)> = None;
                let mut at = s0;
                if extent.is_some() && !(s0..s0 + n).any(inside) {
                    continue;
                }
                for s in (s0..s0 + n).filter(|s| inside(*s)) {
                    match both(s, None) {
                        Some((a, b)) => {
                            acc = Some(match (acc, agg) {
                                (None, _) => (a, b),
                                (Some((pa, pb)), RowAgg::MaxAbs) => {
                                    if a.abs() > pa.abs() {
                                        at = s;
                                        (a, b)
                                    } else {
                                        (pa, pb)
                                    }
                                }
                                (Some((pa, pb)), _) => {
                                    at = s;
                                    (
                                        pa.max(a),
                                        match (pb, b) {
                                            (Some(x), Some(y)) => Some(x.min(y)),
                                            _ => None,
                                        },
                                    )
                                }
                            });
                            if acc.is_some_and(|_| n == 1) {
                                at = s;
                            }
                        }
                        None => {
                            if y.has(s) || y2.is_some_and(|c| c.has(s)) {
                                skipped += 1;
                            }
                        }
                    }
                }
                if acc.is_none() && is_bar {
                    continue;
                }
                rows.push(mk(s0, n, acc, at, None));
            }
        }
    }
    (rows, skipped)
}
