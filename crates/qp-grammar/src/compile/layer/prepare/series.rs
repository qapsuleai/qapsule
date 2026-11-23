//! The series marks: `candle`, `line` and the pair marks `area`, `band`, `bar`.

use super::*;

/// `candle`: one OHLC row per bucket of a bar series.
pub(super) fn candle(
    cx: &Ctx<'_, '_>,
    st: &mut PrepState,
    opacity: f64,
) -> Result<Geom, GrammarError> {
    let (fr, id, theme) = (cx.fr, cx.id, &cx.fr.theme);
    let geom = Geom::Candle;
    let Src::Slots(series) = cx.src else {
        return Err(cx.err(ErrorKind::Invalid, "/mark", "`candle` needs a bar series"));
    };
    if fr.kind(series) != Some(SeriesKind::Bars) {
        return Err(cx.err(
            ErrorKind::Invalid,
            "/series",
            format!("`candle` needs a bar series; `{series}` is not one"),
        ));
    }
    let cols: Vec<Column<'_>> = ["o", "h", "l", "c"]
        .iter()
        .map(|f| fr.column(series, f, id, "/series"))
        .collect::<Result<_, _>>()?;
    let rule = cx.color_rule(ColorRule::Candle {
        up: theme.up,
        down: theme.down,
    })?;
    let mut rows = Vec::new();
    for (s0, n) in fr.buckets() {
        let (mut o, mut h, mut l, mut c) = (None, None::<f64>, None::<f64>, None);
        for s in s0..s0 + n {
            let (vo, vh, vl, vc) = (
                cols[0].get(s),
                cols[1].get(s),
                cols[2].get(s),
                cols[3].get(s),
            );
            if let (Some(vo), Some(vh), Some(vl), Some(vc)) = (vo, vh, vl, vc) {
                if o.is_none() {
                    o = Some(vo);
                }
                h = Some(h.map_or(vh, |x| x.max(vh)));
                l = Some(l.map_or(vl, |x| x.min(vl)));
                c = Some(vc);
            } else if s < fr.n_bars() {
                st.skipped += 1;
            }
        }
        let (Some(o), Some(h), Some(l), Some(c)) = (o, h, l, c) else {
            continue;
        };
        let last = s0 + n - 1;
        let (color, ckey) = rule.at(last, None, Some((o, c)));
        rows.push(PRow {
            slot: s0,
            n,
            t: fr.slot_time(s0),
            v: [Some(o), Some(h), Some(l), Some(c)],
            color: with_opacity(color, opacity),
            ckey,
            size: 0.0,
            text: None,
        });
    }
    st.keys = vec!["o".into(), "h".into(), "l".into(), "c".into()];
    st.fields.clone_from(&st.keys);
    st.traces.push(Trace {
        rows,
        split: Split::None,
        base: with_opacity(theme.up, opacity),
        lod_line: false,
    });
    Ok(geom)
}

/// `line`: one trace per plotted field, each with the next palette colour.
pub(super) fn line(
    cx: &Ctx<'_, '_>,
    st: &mut PrepState,
    palette: &mut usize,
    opacity: f64,
) -> Result<Geom, GrammarError> {
    let (fr, enc, theme) = (cx.fr, &cx.layer.encoding, &cx.fr.theme);
    let geom = Geom::Line;
    st.width = enc.width.unwrap_or(1.5);
    let chans: Vec<(Chan<'_>, String)> = match cx.position(enc.y.as_ref(), "/encoding/y", "y")? {
        Some(c) => vec![c],
        None => match cx.src {
            Src::Slots(series) => {
                let pf = fr.plot_fields(series);
                pf.into_iter()
                    .map(|f| Ok((cx.field(&f, "/encoding/y")?, f)))
                    .collect::<Result<_, GrammarError>>()?
            }
            _ => vec![main_or_err(cx, "line")?],
        },
    };
    for (chan, key) in chans {
        let default = match next_palette(theme, palette, enc.color.is_some()) {
            Some(c) => ColorRule::Const(c),
            None => ColorRule::Const(theme.accent),
        };
        let rule = cx.color_rule(default)?;
        let (rows, sk, lod_line) = series_rows(cx, &chan, &rule, opacity, RowAgg::Line);
        st.skipped += sk;
        let base = rule.at(fr.to, None, None).0;
        st.traces.push(Trace {
            rows,
            split: rule.split(opacity),
            base,
            lod_line,
        });
        st.fields.push(key.clone());
        st.keys.push(key);
    }
    Ok(geom)
}

/// `area`, `band` and `bar`: `y` paired with `y2` (a bar's base is 0).
pub(super) fn area_band_bar(
    cx: &Ctx<'_, '_>,
    st: &mut PrepState,
    palette: &mut usize,
    opacity: f64,
    extent: Option<(u64, u64)>,
) -> Result<Geom, GrammarError> {
    let (fr, layer, theme) = (cx.fr, cx.layer, &cx.fr.theme);
    let enc = &layer.encoding;
    let (y, ykey) = if layer.mark == Mark::Band {
        cx.position(enc.y.as_ref(), "/encoding/y", "y")?
            .ok_or_else(|| cx.err(ErrorKind::Invalid, "/encoding/y", "`band` needs `y`"))?
    } else {
        main_or_err(
            cx,
            if layer.mark == Mark::Bar {
                "bar"
            } else {
                "area"
            },
        )?
    };
    let y2 = cx.position(enc.y2.as_ref(), "/encoding/y2", "y2")?;
    if layer.mark == Mark::Band && y2.is_none() {
        return Err(cx.err(ErrorKind::Invalid, "/encoding/y2", "`band` needs `y2`"));
    }
    let (y2, y2key, base_default) = match y2 {
        Some((c, k)) => (Some(c), Some(k), false),
        None if layer.mark == Mark::Bar => (Some(Chan::Const(0.0)), Some("base".into()), true),
        None => (None, None, false),
    };
    let _ = base_default;
    let geom = match layer.mark {
        Mark::Area => Geom::Area,
        Mark::Band => Geom::Band,
        _ => Geom::Bar,
    };
    let default = if layer.mark == Mark::Bar {
        ColorRule::Const(theme.muted)
    } else {
        ColorRule::Const(next_palette(theme, palette, enc.color.is_some()).unwrap_or(theme.accent))
    };
    let rule = cx.color_rule(default)?;
    let agg = match layer.mark {
        Mark::Bar => RowAgg::MaxAbs,
        _ => RowAgg::Envelope,
    };
    let (rows, sk) = pair_rows(cx, &y, y2.as_ref(), &rule, opacity, agg, extent);
    st.skipped += sk;
    let base = rule.at(fr.to, None, None).0;
    st.traces.push(Trace {
        rows,
        split: if layer.mark == Mark::Bar {
            Split::None
        } else {
            rule.split(opacity)
        },
        base,
        lod_line: false,
    });
    st.fields.push(ykey.clone());
    st.keys.push(ykey);
    if let Some(k) = y2key {
        if !(layer.mark == Mark::Bar && k == "base") {
            st.fields.push(k.clone());
        }
        st.keys.push(k);
    }
    Ok(geom)
}
