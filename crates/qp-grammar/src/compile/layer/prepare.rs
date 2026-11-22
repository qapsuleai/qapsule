//! [`prepare`]: one layer's data, channels, constants and colours into value rows. A
//! single function, alone in its file (RG1 exception request: it is longer than a file).

use super::*;

#[allow(clippy::cognitive_complexity)]
/// Prepares one layer. `palette` counts uncoloured line/area/band traces in the panel.
#[allow(clippy::too_many_lines)]
pub(crate) fn prepare(
    fr: &Frame<'_>,
    layer: &Layer,
    id: &str,
    palette: &mut usize,
) -> Result<Prepared, GrammarError> {
    let n_data = [&layer.series, &layer.fact, &layer.finding]
        .iter()
        .filter(|d| d.is_some())
        .count();
    if n_data > 1 {
        return Err(GrammarError::in_layer(
            ErrorKind::Invalid,
            id,
            "",
            "a layer has at most one of `series`, `fact`, `finding`",
        ));
    }
    let constants = resolve_constants(fr, layer, id)?;
    let mut all_rows = Vec::new();
    let (src, data) = if let Some(s) = &layer.series {
        if fr.kind(s).is_none() {
            return Err(GrammarError::in_layer(
                ErrorKind::NotFound,
                id,
                "/series",
                format!("series `{s}` does not exist"),
            ));
        }
        (
            Src::Slots(s.as_str()),
            DataSource::Series { series: s.clone() },
        )
    } else if let Some(f) = &layer.fact {
        let fact = fr.p.fact(f).ok_or_else(|| {
            GrammarError::in_layer(
                ErrorKind::NotFound,
                id,
                "/fact",
                format!("fact `{f}` does not exist"),
            )
        })?;
        all_rows.push(fr.fact_row(&fact, id, "/fact")?);
        (Src::Rows, DataSource::Fact { fact: f.clone() })
    } else if let Some(f) = &layer.finding {
        let finding = fr.p.finding(f).ok_or_else(|| {
            GrammarError::in_layer(
                ErrorKind::NotFound,
                id,
                "/finding",
                format!("finding `{f}` does not exist"),
            )
        })?;
        for fid in &finding.facts {
            let fact = fr.p.fact(fid).ok_or_else(|| {
                GrammarError::in_layer(
                    ErrorKind::NotFound,
                    id,
                    "/finding",
                    format!("fact `{fid}` of finding `{f}` does not exist"),
                )
            })?;
            let mut row = fr.fact_row(&fact, id, "/finding")?;
            row.nums.insert("confidence".to_owned(), finding.confidence);
            row.texts.insert("claim".to_owned(), finding.claim.clone());
            all_rows.push(row);
        }
        // R3 capsule texts (the capsule card): its sentence and its ledger line. (R4/H-FP)
        // In a face scene whose design has a headline template, the template's sentence.
        // (R5/m17-int) Under a voice every capsule scene speaks from it: its headline,
        // its ledger sentence and its detail (`docs/LANGUAGE.md`).
        let voice = fr.p.voice(f);
        let headline = match &voice {
            Some(v) => Some(v.headline.clone()),
            None => crate::faces::headline_text(fr.headline, f, fr.p),
        };
        let ledger = match &voice {
            Some(v) => Some(v.ledger.clone()),
            None => fr.p.capsule(f).and_then(|c| {
                c.story
                    .as_ref()
                    .map(|s| crate::depth::ledger_text(&s.ledger_at_birth))
            }),
        };
        let detail = voice.map(|v| v.detail);
        // (R5/M20) An open capsule design's text field (`ask/…`, `crate::open`): resolved
        // here from the provider, only for the field the layer reads.
        let asked = layer
            .encoding
            .text
            .as_deref()
            .and_then(|k| crate::open::field_text(k, f, fr.p).map(|t| (k.to_owned(), t)));
        for row in &mut all_rows {
            if let Some((k, t)) = &asked {
                row.texts.insert(k.clone(), t.clone());
            }
            if let Some(h) = &headline {
                row.texts.insert("headline".to_owned(), h.clone());
            }
            if let Some(l) = &ledger {
                row.texts.insert("ledger".to_owned(), l.clone());
            }
            if let Some(d) = &detail {
                row.texts.insert("detail".to_owned(), d.clone());
            }
        }
        // (R5/M20) An open capsule design's own field (`open.*`, `crate::open::text`).
        if let Some(k) = layer.encoding.text.as_deref() {
            if let Some(t) = crate::open::text(k, f, fr.p) {
                for row in &mut all_rows {
                    row.texts.insert(k.to_owned(), t.clone());
                }
            }
        }
        (Src::Rows, DataSource::Finding { finding: f.clone() })
    } else {
        (Src::Nothing, DataSource::Constant {})
    };
    let cx = Ctx {
        fr,
        id,
        layer,
        src,
        all_rows,
        constants,
    };
    let enc = &layer.encoding;
    let opacity = enc.opacity.unwrap_or_else(|| default_opacity(layer.mark));
    if !(0.0..=1.0).contains(&opacity) {
        return Err(cx.err(
            ErrorKind::Invalid,
            "/encoding/opacity",
            "opacity must be in 0..=1",
        ));
    }
    if let Some(w) = enc.width {
        if !(w.is_finite() && w > 0.0) {
            return Err(cx.err(ErrorKind::Invalid, "/encoding/width", "width must be > 0"));
        }
    }
    let x2_given = enc.x2.is_some() || cx.constants.contains_key("/encoding/x2");
    let x_ok = match layer.mark {
        Mark::Rule => true,
        Mark::Band | Mark::Area => x2_given,
        Mark::Point => !matches!(cx.src, Src::Rows | Src::Slots(_)),
        Mark::Text | Mark::Glyph => is_panel(enc.x.as_ref()),
        _ => false,
    };
    if enc.x.is_some() && !x_ok {
        return Err(cx.err(
            ErrorKind::Invalid,
            "/encoding/x",
            "`x` is used by `rule` (vertical rule, or with `x2` a segment), by `band`/`area` with `x2` (a bar extent), by a data-less `point` (its bar) and, panel-relative, by `text` and `glyph`; other marks are placed at their row's bar",
        ));
    }
    // (R5/M20) A panel-positioned `text` may take a panel-relative `x2`: its wrap edge.
    let text_edge =
        layer.mark == Mark::Text && is_panel(enc.x.as_ref()) && is_panel(enc.x2.as_ref());
    if x2_given && !text_edge && !matches!(layer.mark, Mark::Rule | Mark::Band | Mark::Area) {
        return Err(cx.err(
            ErrorKind::Invalid,
            "/encoding/x2",
            "`x2` (the last bar of an extent) is used by `rule`, `band` and `area`",
        ));
    }
    let extent = if x2_given && layer.mark != Mark::Rule && !text_edge {
        let a = cx
            .bar_const("/encoding/x", enc.x.as_ref())?
            .ok_or_else(|| {
                cx.err(
                    ErrorKind::Invalid,
                    "/encoding/x",
                    "`x2` needs `x` (the first bar)",
                )
            })?;
        let b = cx.bar_const("/encoding/x2", enc.x2.as_ref())?.unwrap_or(a);
        Some((a.min(b), a.max(b)))
    } else {
        None
    };
    let mut anchor = Anchor::default();
    let mut glyphs: Vec<GlyphSpec> = Vec::new();
    let mut seg_extent: Option<(u64, u64)> = None;
    let label = layer.label.clone().or_else(|| {
        layer
            .series
            .as_ref()
            .map(|s| fr.p.label(s).unwrap_or_else(|| s.clone()))
    });
    let theme = &fr.theme;
    let mut skipped = 0u64;
    let mut keys: Vec<String> = Vec::new();
    let mut fields: Vec<String> = Vec::new();
    let mut traces: Vec<Trace> = Vec::new();
    let geom;
    let mut width = enc.width.unwrap_or(1.0);

    fn main_or_err<'f>(cx: &Ctx<'f, '_>, what: &str) -> Result<(Chan<'f>, String), GrammarError> {
        match cx.position(cx.layer.encoding.y.as_ref(), "/encoding/y", "y")? {
            Some(c) => Ok(c),
            None => {
                let m = cx.main().ok_or_else(|| {
                    cx.err(
                        ErrorKind::Invalid,
                        "/encoding/y",
                        format!("`{what}` needs `y`"),
                    )
                })?;
                Ok((cx.field(&m, "/encoding/y")?, m))
            }
        }
    }
    let mut next_palette = |rule_given: bool| {
        if rule_given {
            None
        } else {
            let c = theme.palette(*palette);
            *palette += 1;
            Some(c)
        }
    };

    match layer.mark {
        Mark::Candle => {
            geom = Geom::Candle;
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
                        skipped += 1;
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
            keys = vec!["o".into(), "h".into(), "l".into(), "c".into()];
            fields.clone_from(&keys);
            traces.push(Trace {
                rows,
                split: Split::None,
                base: with_opacity(theme.up, opacity),
                lod_line: false,
            });
        }
        Mark::Line => {
            geom = Geom::Line;
            width = enc.width.unwrap_or(1.5);
            let chans: Vec<(Chan<'_>, String)> =
                match cx.position(enc.y.as_ref(), "/encoding/y", "y")? {
                    Some(c) => vec![c],
                    None => match cx.src {
                        Src::Slots(series) => {
                            let pf = fr.plot_fields(series);
                            pf.into_iter()
                                .map(|f| Ok((cx.field(&f, "/encoding/y")?, f)))
                                .collect::<Result<_, GrammarError>>()?
                        }
                        _ => vec![main_or_err(&cx, "line")?],
                    },
                };
            for (chan, key) in chans {
                let default = match next_palette(enc.color.is_some()) {
                    Some(c) => ColorRule::Const(c),
                    None => ColorRule::Const(theme.accent),
                };
                let rule = cx.color_rule(default)?;
                let (rows, sk, lod_line) = series_rows(&cx, &chan, &rule, opacity, RowAgg::Line);
                skipped += sk;
                let base = rule.at(fr.to, None, None).0;
                traces.push(Trace {
                    rows,
                    split: rule.split(opacity),
                    base,
                    lod_line,
                });
                fields.push(key.clone());
                keys.push(key);
            }
        }
        Mark::Area | Mark::Band | Mark::Bar => {
            let (y, ykey) = if layer.mark == Mark::Band {
                cx.position(enc.y.as_ref(), "/encoding/y", "y")?
                    .ok_or_else(|| cx.err(ErrorKind::Invalid, "/encoding/y", "`band` needs `y`"))?
            } else {
                main_or_err(
                    &cx,
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
                None if layer.mark == Mark::Bar => {
                    (Some(Chan::Const(0.0)), Some("base".into()), true)
                }
                None => (None, None, false),
            };
            let _ = base_default;
            geom = match layer.mark {
                Mark::Area => Geom::Area,
                Mark::Band => Geom::Band,
                _ => Geom::Bar,
            };
            let default = if layer.mark == Mark::Bar {
                ColorRule::Const(theme.muted)
            } else {
                ColorRule::Const(next_palette(enc.color.is_some()).unwrap_or(theme.accent))
            };
            let rule = cx.color_rule(default)?;
            let agg = match layer.mark {
                Mark::Bar => RowAgg::MaxAbs,
                _ => RowAgg::Envelope,
            };
            let (rows, sk) = pair_rows(&cx, &y, y2.as_ref(), &rule, opacity, agg, extent);
            skipped += sk;
            let base = rule.at(fr.to, None, None).0;
            traces.push(Trace {
                rows,
                split: if layer.mark == Mark::Bar {
                    Split::None
                } else {
                    rule.split(opacity)
                },
                base,
                lod_line: false,
            });
            fields.push(ykey.clone());
            keys.push(ykey);
            if let Some(k) = y2key {
                if !(layer.mark == Mark::Bar && k == "base") {
                    fields.push(k.clone());
                }
                keys.push(k);
            }
        }
        Mark::Rule => {
            let rule = cx.color_rule(ColorRule::Const(theme.muted))?;
            let mut rows = Vec::new();
            let mk = |slot: u64, v: Option<f64>, row: Option<&Row>| {
                let (color, ckey) = rule.at(slot, row, None);
                PRow {
                    slot,
                    n: 1,
                    t: fr.slot_time(slot),
                    v: [v, None, None, None],
                    color: with_opacity(color, opacity),
                    ckey,
                    size: 0.0,
                    text: None,
                }
            };
            if x2_given {
                // A segment from bar `x` to bar `x2` at `y` (sloped to `y2`): evidence
                // geometry (R3). Every value is a constant with an origin.
                geom = Geom::Segment;
                let a = cx
                    .bar_const("/encoding/x", enc.x.as_ref())?
                    .ok_or_else(|| {
                        cx.err(
                            ErrorKind::Invalid,
                            "/encoding/x",
                            "`x2` needs `x` (the first bar)",
                        )
                    })?;
                let b = cx.bar_const("/encoding/x2", enc.x2.as_ref())?.unwrap_or(a);
                let konst = |ptr: &str, pos: Option<&Position>, key: &str| match cx
                    .position(pos, ptr, key)?
                {
                    Some((Chan::Const(v), _)) => Ok(Some(v)),
                    None => Ok(None),
                    Some(_) => Err(cx.err(
                        ErrorKind::Invalid,
                        ptr,
                        "a segment's `y`/`y2` are constants with an origin",
                    )),
                };
                let y0 = konst("/encoding/y", enc.y.as_ref(), "y")?.ok_or_else(|| {
                    cx.err(ErrorKind::Invalid, "/encoding/y", "a segment needs `y`")
                })?;
                let y1 = konst("/encoding/y2", enc.y2.as_ref(), "y2")?;
                let mut row = mk(a, Some(y0), None);
                row.v[1] = y1;
                rows.push(row);
                seg_extent = Some((a, b));
                keys.push("y".into());
                if y1.is_some() {
                    keys.push("y2".into());
                }
            } else if let Some((y, key)) = cx.position(enc.y.as_ref(), "/encoding/y", "y")? {
                geom = Geom::RuleH;
                match (&y, &cx.src) {
                    (Chan::Const(v), _) => rows.push(mk(fr.to, Some(*v), None)),
                    (_, Src::Slots(_)) => {
                        // A rule over series data: one rule at the last visible value.
                        if let Some((s, v)) = (fr.from..=fr.to)
                            .rev()
                            .find_map(|s| y.at(s, None).map(|v| (s, v)))
                        {
                            rows.push(mk(s, Some(v), None));
                        }
                    }
                    _ => {
                        for r in cx.visible_rows() {
                            match y.at(r.slot, Some(r)) {
                                Some(v) => rows.push(mk(r.slot, Some(v), Some(r))),
                                None => skipped += 1,
                            }
                        }
                    }
                }
                if !matches!(y, Chan::Const(_)) {
                    fields.push(key.clone());
                }
                keys.push(key);
            } else if let Some((x, key)) = cx.position(enc.x.as_ref(), "/encoding/x", "x")? {
                geom = Geom::RuleV;
                let by_bar = matches!(
                    cx.constants.get("/encoding/x").map(|c| &c.origin),
                    Some(Origin::Fact { field, .. }) if field == "bar"
                ) || key == "bar";
                let to_slot = |v: f64| -> Option<u64> {
                    if by_bar {
                        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                        (v >= 0.0).then(|| v.round() as u64)
                    } else {
                        fr.time_slot(v)
                    }
                };
                match (&x, &cx.src) {
                    (Chan::Const(v), _) => {
                        if let Some(s) = to_slot(*v).filter(|s| *s >= fr.from && *s <= fr.to) {
                            rows.push(mk(s, Some(*v), None));
                        }
                    }
                    (_, Src::Rows) => {
                        for r in cx.visible_rows() {
                            if let Some(s) = x.at(r.slot, Some(r)).and_then(to_slot) {
                                if s >= fr.from && s <= fr.to {
                                    rows.push(mk(s, x.at(r.slot, Some(r)), Some(r)));
                                }
                            }
                        }
                    }
                    _ => {
                        return Err(cx.err(
                            ErrorKind::Invalid,
                            "/encoding/x",
                            "a vertical rule's `x` is a constant or a fact/finding field",
                        ))
                    }
                }
                keys.push(key);
            } else if matches!(cx.src, Src::Rows) {
                geom = Geom::RuleV;
                for r in cx.visible_rows() {
                    rows.push(mk(r.slot, None, Some(r)));
                }
                keys.push("x".into());
            } else {
                return Err(cx.err(
                    ErrorKind::Invalid,
                    "/encoding",
                    "`rule` needs `y` (horizontal) or `x` (vertical), or fact/finding data",
                ));
            }
            let base = rule.at(fr.to, None, None).0;
            traces.push(Trace {
                rows,
                split: Split::None,
                base,
                lod_line: false,
            });
        }
        Mark::Text if is_panel(enc.x.as_ref()) || is_panel(enc.y.as_ref()) => {
            geom = Geom::Text;
            anchor = Anchor {
                x: panel_pos(enc.x.as_ref()),
                y: panel_pos(enc.y.as_ref()),
                x2: panel_pos(enc.x2.as_ref()),
            };
            if !matches!(cx.src, Src::Rows) {
                return Err(cx.err(
                    ErrorKind::Invalid,
                    "/encoding/text",
                    "`text` reads a text field of a fact or finding",
                ));
            }
            let f = text_field_of(&cx)?;
            let y = if anchor.y.is_some() {
                None
            } else {
                Some(main_or_err(&cx, "text")?)
            };
            let size = match &enc.size {
                None => 11.0,
                Some(SizeSpec::Constant(s)) if s.is_finite() && *s > 0.0 => *s,
                Some(_) => {
                    return Err(cx.err(
                        ErrorKind::Invalid,
                        "/encoding/size",
                        "a panel-positioned `text` takes a constant size > 0",
                    ))
                }
            };
            let rule = cx.color_rule(ColorRule::Const(theme.text))?;
            let mut rows = Vec::new();
            // A panel-relative `x` has no bar: the layer draws its first row only.
            let src_rows: Vec<&Row> = if anchor.x.is_some() {
                cx.all_rows.iter().take(1).collect()
            } else {
                cx.visible_rows().collect()
            };
            for r in src_rows {
                let v = match &y {
                    Some((c, _)) => match c.at(r.slot, Some(r)) {
                        Some(v) => Some(v),
                        None => {
                            skipped += 1;
                            continue;
                        }
                    },
                    None => None,
                };
                let (color, ckey) = rule.at(r.slot, Some(r), None);
                rows.push(PRow {
                    slot: r.slot,
                    n: 1,
                    t: fr.slot_time(r.slot),
                    v: [v, None, None, None],
                    color: with_opacity(color, opacity),
                    ckey,
                    size,
                    text: r.texts.get(&f).cloned(),
                });
            }
            let base = rule.at(fr.to, None, None).0;
            traces.push(Trace {
                rows,
                split: Split::None,
                base,
                lod_line: false,
            });
            if let Some((_, k)) = y {
                fields.push(k.clone());
                keys.push(k);
            }
            fields.push(f);
        }
        Mark::Glyph => {
            geom = Geom::Glyph;
            let Some(finding) = &layer.finding else {
                return Err(cx.err(
                    ErrorKind::Invalid,
                    if layer.fact.is_some() || layer.series.is_some() {
                        "/mark"
                    } else {
                        "/finding"
                    },
                    "`glyph` draws the capsule of a finding: bind the layer to its `finding`",
                ));
            };
            for (bad, set) in [
                ("/encoding/color", enc.color.is_some()),
                ("/encoding/opacity", enc.opacity.is_some()),
                ("/encoding/width", enc.width.is_some()),
                ("/encoding/y2", enc.y2.is_some()),
                ("/encoding/text", enc.text.is_some()),
            ] {
                if set {
                    return Err(cx.err(
                        ErrorKind::Invalid,
                        bad,
                        "a glyph takes only `x`, `y` and `size`; its colours come from the theme and its other channels from the capsule",
                    ));
                }
            }
            let spec = fr.p.glyph(finding).ok_or_else(|| {
                cx.err(
                    ErrorKind::NotFound,
                    "/finding",
                    format!("finding `{finding}` has no meaning glyph (no R3 capsule with a story presents it)"),
                )
            })?;
            let size = match &enc.size {
                None => fr.look.glyph.default_px,
                Some(SizeSpec::Constant(s)) if s.is_finite() && *s > 0.0 => *s,
                Some(_) => {
                    return Err(cx.err(
                        ErrorKind::Invalid,
                        "/encoding/size",
                        "a glyph takes a constant size > 0 (pixels)",
                    ))
                }
            };
            // `y` absent: at the top of the panel (a glyph strip over the chart).
            anchor = Anchor {
                x: panel_pos(enc.x.as_ref()),
                y: match &enc.y {
                    None => Some(PanelPos {
                        panel: 0.0,
                        px: fr.look.offsets.glyph_top,
                    }),
                    p => panel_pos(p.as_ref()),
                },
                x2: None,
            };
            let y = if anchor.y.is_some() {
                None
            } else {
                cx.position(enc.y.as_ref(), "/encoding/y", "y")?
            };
            let mut rows = Vec::new();
            if let Some(r) = cx.all_rows.first() {
                let visible = anchor.x.is_some() || (r.slot >= fr.from && r.slot <= fr.to);
                let v = y.as_ref().and_then(|(c, _)| c.at(r.slot, Some(r)));
                if visible && (y.is_none() || v.is_some()) {
                    rows.push(PRow {
                        slot: r.slot,
                        n: 1,
                        t: fr.slot_time(r.slot),
                        v: [v, None, None, None],
                        color: [0; 4],
                        ckey: None,
                        size,
                        text: None,
                    });
                    glyphs.push(spec.into_owned());
                } else {
                    skipped += 1;
                }
            }
            traces.push(Trace {
                rows,
                split: Split::None,
                base: theme.text,
                lod_line: false,
            });
            fields.push("glyph".to_owned());
            if let Some((_, k)) = y {
                fields.push(k.clone());
                keys.push(k);
            }
        }
        Mark::Point | Mark::Text => {
            geom = if layer.mark == Mark::Point {
                Geom::Point
            } else {
                Geom::Text
            };
            let (y, ykey) = main_or_err(
                &cx,
                if layer.mark == Mark::Point {
                    "point"
                } else {
                    "text"
                },
            )?;
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
            let default = if layer.mark == Mark::Point {
                theme.accent
            } else {
                theme.text
            };
            let rule = cx.color_rule(ColorRule::Const(default))?;
            let default_size = if layer.mark == Mark::Point { 6.0 } else { 11.0 };
            let size_chan = match &enc.size {
                None => None,
                Some(SizeSpec::Constant(s)) => {
                    if !(s.is_finite() && *s > 0.0) {
                        return Err(cx.err(
                            ErrorKind::Invalid,
                            "/encoding/size",
                            "size must be > 0",
                        ));
                    }
                    None
                }
                Some(SizeSpec::ByField(b)) => {
                    Some((cx.field(&b.field, "/encoding/size/field")?, b.range))
                }
            };
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
            match cx.src {
                Src::Slots(_) => {
                    for (s0, n) in fr.buckets() {
                        // Last valid value of the bucket.
                        let s = (s0..s0 + n).rev().find(|s| y.at(*s, None).is_some());
                        match s {
                            Some(s) => push(s, n, None, &mut skipped),
                            None => skipped += n.min(fr.n_bars().saturating_sub(s0)),
                        }
                    }
                }
                Src::Rows => {
                    for r in cx.visible_rows() {
                        push(r.slot, 1, Some(r), &mut skipped);
                    }
                }
                Src::Nothing => {
                    // A data-less point sits on the bar its `x` constant names (R3
                    // evidence geometry: a swing point).
                    match cx.bar_const("/encoding/x", enc.x.as_ref())? {
                        Some(s) if layer.mark == Mark::Point => {
                            if s >= fr.from && s <= fr.to {
                                push(s, 1, None, &mut skipped);
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
            if let Some((_, range)) = &size_chan {
                let vals: Vec<f64> = rows.iter().filter_map(|(_, s)| *s).collect();
                let lo = vals.iter().copied().fold(f64::INFINITY, f64::min);
                let hi = vals.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                for (r, s) in &mut rows {
                    r.size = match *s {
                        Some(s) if hi > lo => {
                            range[0] + (s - lo) / (hi - lo) * (range[1] - range[0])
                        }
                        Some(_) => range[0],
                        None => const_size,
                    }
                    .max(1.0);
                }
            }
            let base = rule.at(fr.to, None, None).0;
            traces.push(Trace {
                rows: rows.into_iter().map(|(r, _)| r).collect(),
                split: Split::None,
                base,
                lod_line: false,
            });
            fields.push(ykey.clone());
            keys.push(ykey);
            if let Some(f) = text_field {
                fields.push(f);
            }
        }
    }
    Ok(Prepared {
        id: id.to_owned(),
        mark: layer.mark,
        panel: layer.panel,
        data,
        fields,
        constants: cx.constants,
        label,
        geom,
        traces,
        keys,
        width,
        edge: enc.width.is_some(),
        skipped,
        has_data: n_data == 1,
        anchor,
        extent: seg_extent,
        glyphs,
    })
}
