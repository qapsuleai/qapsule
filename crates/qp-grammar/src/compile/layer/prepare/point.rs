//! The `point` mark and a bar-placed `text`: one row per bucket or fact/finding row, sized
//! by a constant or by a field.

use super::*;

/// A size channel by field and its pixel range.
type SizeChan<'f> = Option<(Chan<'f>, [f64; 2])>;

/// `point` or a bar-placed `text`.
pub(super) fn point_text(
    cx: &Ctx<'_, '_>,
    st: &mut PrepState,
    opacity: f64,
) -> Result<Geom, GrammarError> {
    let (fr, layer, theme) = (cx.fr, cx.layer, &cx.fr.theme);
    let enc = &layer.encoding;
    let geom = if layer.mark == Mark::Point {
        Geom::Point
    } else {
        Geom::Text
    };
    let (y, ykey) = main_or_err(
        cx,
        if layer.mark == Mark::Point {
            "point"
        } else {
            "text"
        },
    )?;
    let text_field = point_text_field(cx)?;
    let default = if layer.mark == Mark::Point {
        theme.accent
    } else {
        theme.text
    };
    let rule = cx.color_rule(ColorRule::Const(default))?;
    let default_size = if layer.mark == Mark::Point { 6.0 } else { 11.0 };
    let size_chan = point_size_chan(cx)?;
    let const_size = match &enc.size {
        Some(SizeSpec::Constant(s)) => *s,
        _ => default_size,
    };
    let mut rows = Vec::new();
    let mut push = |slot: u64, n: u64, row: Option<&Row>, skipped: &mut u64| {
        let Some(v) = y.at(slot, row) else {
            *skipped += 1;
            return;
        };
        let (color, ckey) = rule.at(slot, row, None);
        let sv = size_chan.as_ref().and_then(|(c, _)| c.at(slot, row));
        rows.push((
            PRow {
                slot,
                n,
                t: fr.slot_time(slot),
                v: [Some(v), None, None, None],
                color: with_opacity(color, opacity),
                ckey,
                size: const_size,
                text: text_field
                    .as_ref()
                    .and_then(|f| row.and_then(|r| r.texts.get(f).cloned())),
            },
            sv,
        ));
    };
    point_source_rows(cx, &y, &mut push, &mut st.skipped)?;
    if let Some((_, range)) = &size_chan {
        scale_sizes(&mut rows, range, const_size);
    }
    let base = rule.at(fr.to, None, None).0;
    st.traces.push(Trace {
        rows: rows.into_iter().map(|(r, _)| r).collect(),
        split: Split::None,
        base,
        lod_line: false,
    });
    st.fields.push(ykey.clone());
    st.keys.push(ykey);
    if let Some(f) = text_field {
        st.fields.push(f);
    }
    Ok(geom)
}

/// A bar-placed `text`'s text field (`text`, or the fact's `text` / the finding's `claim`).
fn point_text_field(cx: &Ctx<'_, '_>) -> Result<Option<String>, GrammarError> {
    let layer = cx.layer;
    let enc = &layer.encoding;
    let text_field = if layer.mark == Mark::Text {
        let f = enc.text.clone().or_else(|| match &layer.finding {
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
        if !matches!(cx.src, Src::Rows) {
            return Err(cx.err(
                ErrorKind::Invalid,
                "/encoding/text",
                "in v0 `text` reads a text field of a fact or finding",
            ));
        }
        if !cx.all_rows.iter().any(|r| r.texts.contains_key(&f)) {
            return Err(cx.err(
                ErrorKind::NotFound,
                "/encoding/text",
                format!(
                    "text field `{f}` not found (fields: text{})",
                    if layer.finding.is_some() {
                        ", claim"
                    } else {
                        ""
                    }
                ),
            ));
        }
        Some(f)
    } else {
        None
    };
    Ok(text_field)
}

/// The size channel: none for a constant (checked > 0), a field with its range otherwise.
fn point_size_chan<'f>(cx: &Ctx<'f, '_>) -> Result<SizeChan<'f>, GrammarError> {
    let size_chan = match &cx.layer.encoding.size {
        None => None,
        Some(SizeSpec::Constant(s)) => {
            if !(s.is_finite() && *s > 0.0) {
                return Err(cx.err(ErrorKind::Invalid, "/encoding/size", "size must be > 0"));
            }
            None
        }
        Some(SizeSpec::ByField(b)) => Some((cx.field(&b.field, "/encoding/size/field")?, b.range)),
    };
    Ok(size_chan)
}

/// Pushes the rows of the source: the last valid value per bucket, each visible row, or a
/// data-less point on the bar its `x` constant names.
fn point_source_rows(
    cx: &Ctx<'_, '_>,
    y: &Chan<'_>,
    push: &mut impl FnMut(u64, u64, Option<&Row>, &mut u64),
    skipped: &mut u64,
) -> Result<(), GrammarError> {
    let fr = cx.fr;
    match cx.src {
        Src::Slots(_) => {
            for (s0, n) in fr.buckets() {
                // Last valid value of the bucket.
                let s = (s0..s0 + n).rev().find(|s| y.at(*s, None).is_some());
                match s {
                    Some(s) => push(s, n, None, skipped),
                    None => *skipped += n.min(fr.n_bars().saturating_sub(s0)),
                }
            }
        }
        Src::Rows => {
            for r in cx.visible_rows() {
                push(r.slot, 1, Some(r), skipped);
            }
        }
        Src::Nothing => {
            // A data-less point sits on the bar its `x` constant names (R3
            // evidence geometry: a swing point).
            match cx.bar_const("/encoding/x", cx.layer.encoding.x.as_ref())? {
                Some(s) if cx.layer.mark == Mark::Point => {
                    if s >= fr.from && s <= fr.to {
                        push(s, 1, None, skipped);
                    }
                }
                _ => {
                    if let Chan::Const(_) = y {
                        return Err(cx.err(
                            ErrorKind::Invalid,
                            "",
                            "`point`/`text` without data has no bar to sit on (a `point` takes its bar from an `x` constant)",
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}

/// Scales field sizes linearly onto `range` (at least 1 px); rows without a value keep
/// `const_size`.
fn scale_sizes(rows: &mut [(PRow, Option<f64>)], range: &[f64; 2], const_size: f64) {
    let vals: Vec<f64> = rows.iter().filter_map(|(_, s)| *s).collect();
    let lo = vals.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = vals.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    for (r, s) in rows.iter_mut() {
        r.size = match *s {
            Some(s) if hi > lo => range[0] + (s - lo) / (hi - lo) * (range[1] - range[0]),
            Some(_) => range[0],
            None => const_size,
        }
        .max(1.0);
    }
}
