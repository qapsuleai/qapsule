//! [`compile`] itself: one function, alone in its file (it is longer than the file budget
//! leaves room for next to anything else).

use super::*;

#[allow(clippy::cognitive_complexity)]
/// Compiles `view` with data from `provider` into a scene description.
///
/// # Errors
/// [`GrammarError`] naming the layer, the JSON pointer and the reason for anything that
/// cannot be resolved: unknown series/field/fact/finding/panel/threshold, a mark without
/// the data it needs, a constant without origin, invalid colours, domains or sizes.
#[allow(clippy::too_many_lines)]
pub fn compile(
    view: &View,
    provider: &dyn DataProvider,
    opts: &CompileOptions,
) -> Result<SceneDesc, GrammarError> {
    let (width, height) = (opts.width, opts.height);
    if !(MIN_WIDTH..=MAX_DIM).contains(&width) || !(MIN_HEIGHT..=MAX_DIM).contains(&height) {
        return Err(GrammarError::new(
            ErrorKind::Invalid,
            "",
            format!("size {width}x{height} out of range ({MIN_WIDTH}x{MIN_HEIGHT} ..= {MAX_DIM}x{MAX_DIM})"),
        ));
    }
    let look = (opts.look_record.map(Ok))
        .unwrap_or_else(|| qp_look::look::by_name(opts.look.as_deref()))?;
    let theme = look.theme(opts.theme.as_deref().or(view.theme.as_deref()))?;
    let outcome = match opts.outcome.as_deref() {
        Some(n) => qp_glyph::outcome::by_name(n)?,
        None => qp_glyph::outcome::default_style(),
    };
    let alphabet = qp_glyph::alphabet_named(opts.alphabet.as_deref()).ok_or_else(|| {
        GrammarError::new(
            ErrorKind::Invalid,
            "",
            format!(
                "unknown glyph alphabet `{}` (known: {})",
                opts.alphabet.as_deref().unwrap_or_default(),
                qp_glyph::ALPHABETS.join(", ")
            ),
        )
    })?;
    let headline = match opts.face.as_deref() {
        Some(n) => crate::faces::headline_template(Some(n))?,
        None => None,
    };
    if view.x.kind != ScaleKind::Time {
        return Err(GrammarError::new(
            ErrorKind::Invalid,
            "/x/kind",
            "the horizontal scale must be `time`",
        ));
    }
    let panels = canonical_panels(view)?;
    let panel_ids: Vec<u32> = panels.iter().map(|p| p.id.unwrap_or_default()).collect();
    for (i, p) in panels.iter().enumerate() {
        if p.y.kind == ScaleKind::Time {
            return Err(GrammarError::new(
                ErrorKind::Invalid,
                format!("/panels/{i}/y/kind"),
                "a panel scale is `linear` or `log`",
            ));
        }
    }
    let ids = layer_ids(&view.layers)?;
    for (l, id) in view.layers.iter().zip(&ids) {
        if !panel_ids.contains(&l.panel) {
            return Err(GrammarError::in_layer(
                ErrorKind::NotFound,
                id,
                "/panel",
                format!(
                    "panel {} does not exist (panels: {})",
                    l.panel,
                    panels
                        .iter()
                        .map(|p| match &p.name {
                            Some(n) => format!("{} ({n})", p.id.unwrap_or_default()),
                            None => p.id.unwrap_or_default().to_string(),
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            ));
        }
    }
    let bar_series = first_bar_series(view, provider).ok_or_else(|| {
        GrammarError::new(
            ErrorKind::NotFound,
            "/layers",
            "the view has no bar series (no candle, bar-series or study layer to take bar slots from)",
        )
    })?;
    let bars: Cow<'_, [qp_schema::Bar]> = provider.bars(&bar_series).ok_or_else(|| {
        GrammarError::new(
            ErrorKind::NotFound,
            "/layers",
            format!("bar series `{bar_series}` has no bars"),
        )
    })?;
    if bars.is_empty() {
        return Err(GrammarError::new(
            ErrorKind::Invalid,
            "/layers",
            format!("bar series `{bar_series}` has no bars"),
        ));
    }
    // Slots after the last bar (point series, `docs/GRAMMAR.md` §3).
    let ext_times = ext_times(view, provider, bars[bars.len() - 1].t);
    let n_bars = bars.len() as u64;
    let total = n_bars + ext_times.len() as u64;

    let (wi, hi) = (width.cast_signed(), height.cast_signed());
    let [m_left, m_right, m_top, m_bottom] = alphabet::margins(look);
    let (left, right, top, bottom) = (m_left, wi - m_right, m_top, hi - m_bottom);
    let types = &look.types;
    let (title_px, label_px) = (types.title.px as f32, types.label.px as f32);
    let adv = |role: &str| look.role_font(role);
    let mut fr = Frame {
        p: provider,
        headline,
        look,
        theme,
        bar_series: bar_series.clone(),
        bars,
        ext_times,
        from: 0,
        to: total - 1,
        k: 1,
    };
    // Visible range: transient viewport, else the persistent range, else everything.
    if let Some((a, b)) = opts.range {
        let (a, b) = (a.min(b), a.max(b));
        if a >= total {
            return Err(GrammarError::new(
                ErrorKind::Invalid,
                "",
                format!(
                    "visible range starts at slot {a}, past the last slot {}",
                    total - 1
                ),
            ));
        }
        fr.from = a;
        fr.to = b.min(total - 1);
    } else if let Some([t0, t1]) = view.x.domain {
        #[allow(clippy::cast_precision_loss)]
        let slots: Vec<u64> = (0..total)
            .filter(|s| {
                fr.slot_time(*s)
                    .is_some_and(|t| (t as f64) >= t0 && (t as f64) <= t1)
            })
            .collect();
        match (slots.first(), slots.last()) {
            (Some(a), Some(b)) => {
                fr.from = *a;
                fr.to = *b;
            }
            _ => {
                return Err(GrammarError::new(
                    ErrorKind::Invalid,
                    "/x/domain",
                    format!("no bar falls in the range [{t0}, {t1}]"),
                ))
            }
        }
    }
    let count = fr.to - fr.from + 1;
    let plot_w = right - left;
    #[allow(clippy::cast_precision_loss)]
    let slot = f64::from(plot_w) / count as f64;
    let mut k = 1u64;
    if opts.lod && slot < 1.0 {
        k = count.div_ceil(u64::from(plot_w.unsigned_abs()));
    }
    if let Some(mp) = opts.max_points.filter(|m| *m > 0) {
        if count > mp {
            k = k.max(count.div_ceil(mp));
        }
    }
    fr.k = k;
    let lod = (k >= 2).then(|| Lod {
        bars_per_bucket: k,
        buckets: crate::frame::bucket_count(count, k),
    });
    let xmap = XMap {
        from_bar: fr.from,
        to_bar: fr.to,
        left: f64::from(left),
        slot,
    };

    // Prepare layers (palette counted per panel).
    let mut palettes = vec![0usize; panels.len()];
    let mut prepared = Vec::with_capacity(view.layers.len());
    for (l, id) in view.layers.iter().zip(&ids) {
        let pi = panel_ids.iter().position(|p| *p == l.panel).unwrap_or(0);
        prepared.push(prepare(&fr, l, id, &mut palettes[pi]).map_err(|e| e.with_layer(id))?);
    }

    // Panels: rectangles and value maps.
    let rects = panel_rects(&panels, left, right, top, bottom, look.space.panel_gap)?;
    let mut ymaps = Vec::with_capacity(panels.len());
    for (i, (p, rc)) in panels.iter().zip(&rects).enumerate() {
        let mut vals = Vec::new();
        for pl in prepared.iter().filter(|pl| pl.panel == panel_ids[i]) {
            domain_values(pl, &mut vals);
        }
        let (min, max) = domain(&p.y, &vals, i)?;
        ymaps.push(YMap {
            kind: p.y.kind,
            min,
            max,
            top: f64::from(rc.top),
            bottom: f64::from(rc.bottom),
        });
    }

    // Emphasis: persistent, then transient.
    let mut elist: Vec<(String, &Emphasis)> = Vec::new();
    for (i, e) in view.emphasis.iter().enumerate() {
        elist.push((format!("/emphasis/{i}"), e));
    }
    for (i, e) in opts.emphasis.iter().enumerate() {
        elist.push((format!("(options)/emphasis/{i}"), e));
    }
    let emph = resolve_emphasis(view, &elist, &prepared, &panel_ids, provider)?;
    let bold: BTreeSet<&str> = emph
        .iter()
        .filter(|e| e.kind == EmphasisKind::Highlight)
        .filter_map(|e| e.layer.as_deref())
        .collect();

    let theme = &fr.theme;
    let mut under: Vec<Prim> = Vec::new();
    let mut over: Vec<Prim> = Vec::new();
    let target_rects = |ps: &[u32]| -> Vec<PixelRect> {
        panel_ids
            .iter()
            .zip(&rects)
            .filter(|(id, _)| ps.is_empty() || ps.contains(id))
            .map(|(_, r)| *r)
            .collect()
    };
    let bar_px = |a: u64, b: u64| -> Option<(i32, i32)> {
        if b < fr.from || a > fr.to {
            return None;
        }
        #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
        let x = |s: u64| (xmap.left + (s as f64 - fr.from as f64) * slot).floor() as i32;
        let x0 = x(a.max(fr.from)).clamp(left, right);
        let x1 = x(b.min(fr.to) + 1).clamp(left, right).max(x0 + 1);
        Some((x0, x1))
    };
    let strip = with_opacity(
        [theme.accent[0], theme.accent[1], theme.accent[2], 255],
        f64::from(look.surface.strip_alpha) / 255.0,
    );
    for e in emph.iter().filter(|e| e.kind == EmphasisKind::Highlight) {
        let span = match e.bars {
            Some((a, b)) => bar_px(a, b),
            None if e.layer.is_none() && !e.panels.is_empty() => Some((left, right)),
            None => None,
        };
        if let Some((x0, x1)) = span {
            for rc in target_rects(&e.panels) {
                under.push(rect(x0, rc.top, x1 - x0, rc.bottom - rc.top, strip));
            }
        }
    }

    // Last-value tags: the first data layer of each panel (pixel row, value, colour).
    // Computed before the grid so a value label under a tag can be left out.
    let tags: Vec<Option<(i32, f64, Rgba)>> = rects
        .iter()
        .enumerate()
        .map(|(pi, rc)| {
            let pl = prepared
                .iter()
                .filter(|pl| pl.panel == panel_ids[pi] && pl.has_data)
                .find(|pl| {
                    matches!(
                        pl.geom,
                        Geom::Candle | Geom::Line | Geom::Area | Geom::Band | Geom::Bar
                    )
                })?;
            let (v, color) = last_value(pl)?;
            let ly = px(value_y(&ymaps[pi], v)?);
            (ly >= rc.top && ly <= rc.bottom).then_some((ly, v, color))
        })
        .collect();
    // A card panel (R3): every layer in it is panel-positioned (a capsule card's glyph,
    // sentence and ledger line). It has no value scale to show: no grid, value labels or
    // axis lines.
    let card: Vec<bool> = panel_ids
        .iter()
        .map(|pid| {
            let mut ls = prepared.iter().filter(|pl| pl.panel == *pid).peekable();
            ls.peek().is_some() && ls.all(|pl| pl.anchor.y.is_some())
        })
        .collect();
    // (RD2/C2a) Axes, tags, legend and crosshair: R0's, or the look's chart conventions.
    let ch = alphabet::Chart {
        fr: &fr,
        look,
        x: &xmap,
        rects: &rects,
        ymaps: &ymaps,
        card: &card,
        tags: &tags,
        panel0: panel_ids[0],
        w: wi,
        at: None,
        tz: view.display_tz.as_deref().and_then(qp_calendar::Zone::get),
        levels: Vec::new(),
    }
    .resolved(opts.cursor, &prepared, &panel_ids);
    alphabet::axes(&ch, &mut under);
    alphabet::volume(&ch, &prepared, &mut under);
    for (rc, _) in rects.iter().zip(&card).filter(|(_, c)| !**c) {
        under.push(rect(
            rc.right,
            rc.top,
            1,
            rc.bottom - rc.top + 1,
            theme.axis,
        ));
        under.push(rect(rc.left, rc.bottom, rc.right - rc.left, 1, theme.axis));
    }

    // Layers.
    let mut scene_panels: Vec<ScenePanel> = panels
        .iter()
        .zip(&rects)
        .zip(&ymaps)
        .map(|((p, rc), y)| ScenePanel {
            id: p.id.unwrap_or_default(),
            name: p.name.clone(),
            rect: *rc,
            y: *y,
            layers: Vec::new(),
        })
        .collect();
    for pl in &prepared {
        let pi = panel_ids.iter().position(|p| *p == pl.panel).unwrap_or(0);
        let g = Geo {
            x: &xmap,
            y: &ymaps[pi],
            rect: rects[pi],
            k,
            bold: bold.contains(pl.id.as_str()),
            theme,
            look,
            outcome,
            alphabet,
        };
        let mut sl = draw(pl, &g);
        alphabet::soften(&ch, pl, &mut sl.prims);
        for p in &mut sl.prims {
            settle_text(p, look, Some(sl.mark), opts.roles);
        }
        scene_panels[pi].layers.push(sl);
    }

    alphabet::tags(&ch, &mut under, &mut over);
    // Title row.
    let title = view.title.clone().unwrap_or_else(|| {
        format!(
            "{} {}",
            provider.symbol(&bar_series).unwrap_or_default(),
            provider.interval(&bar_series).unwrap_or_default()
        )
    });
    // "C <close>" only when the first panel draws the bar series itself (its price).
    let show_close = prepared.iter().any(|pl| {
        pl.panel == panel_ids[0]
            && matches!(&pl.data, DataSource::Series { series } if *series == bar_series)
    });
    let off = &look.offsets;
    let title_x = left + off.title_x;
    let close_x = left + off.close_x;
    let title_room = if show_close {
        close_x - title_x - off.row_edge
    } else {
        wi - off.row_edge - title_x
    };
    let title = fit_text(&title, title_room, f64::from(title_px), adv("title"));
    let mut text_right = title_x + est_width(&title, f64::from(title_px), adv("title"));
    over.push(Prim::Text {
        x: title_x,
        baseline: off.title_baseline,
        px: title_px,
        text: title,
        color: theme.text,
        align: Align::Left,
        font: None,
        role: Some("title".to_owned()),
    });
    if show_close {
        text_right = alphabet::title_figures(&ch, close_x, text_right, &mut over);
    }
    // Legends: panel 0 right-aligned in the title row; other panels top-left inside.
    for (pi, rc) in rects.iter().enumerate() {
        let legends: Vec<(String, Rgba)> = prepared
            .iter()
            .filter(|pl| pl.panel == panel_ids[pi])
            .filter(|pl| matches!(pl.geom, Geom::Line | Geom::Area | Geom::Band))
            .filter_map(|pl| {
                let label = pl.label.clone()?;
                let (v, color) = last_value(pl).unwrap_or((f64::NAN, pl.traces.first()?.base));
                let text = if pl.geom == Geom::Band || pl.traces.len() > 1 || !v.is_finite() {
                    label
                } else {
                    format!("{label} {}", ch.num(v))
                };
                Some((text, pl.traces.first().map_or(color, |t| t.base)))
            })
            .collect();
        // Identical legend texts within a panel: the first one is kept.
        let mut seen = BTreeSet::new();
        let legends: Vec<(String, Rgba)> = legends
            .into_iter()
            .filter(|(t, _)| seen.insert(t.clone()))
            .collect();
        if pi == 0 {
            // Right-aligned from the right edge; the right-most are kept, and a legend
            // that would reach the title/close text is dropped with all left of it.
            let mut placed: Vec<(String, Rgba, i32)> = Vec::new();
            let mut x = wi - off.row_edge;
            for (text, color) in legends.into_iter().rev() {
                let w = est_width(&text, f64::from(label_px), adv("label"));
                if x - w < text_right + off.legend_gap {
                    break;
                }
                placed.push((text, color, x));
                x -= w + off.legend_gap;
            }
            placed.reverse();
            for (text, color, x) in placed {
                over.push(Prim::Text {
                    x,
                    baseline: off.title_baseline,
                    px: label_px,
                    text,
                    color,
                    align: Align::Right,
                    font: None,
                    role: Some("label".to_owned()),
                });
            }
        } else {
            let mut x = rc.left + off.legend_x;
            for (text, color) in legends {
                let w = est_width(&text, f64::from(label_px), adv("label"));
                over.push(Prim::Text {
                    x,
                    baseline: rc.top + off.legend_baseline,
                    px: label_px,
                    text,
                    color,
                    align: Align::Left,
                    font: None,
                    role: Some("label".to_owned()),
                });
                x += w + off.legend_gap;
            }
        }
    }
    alphabet::crosshair(&ch, &mut over);
    // Veils (focus, dim).
    let veil = [
        theme.bg[0],
        theme.bg[1],
        theme.bg[2],
        look.surface.veil_alpha,
    ];
    let layer_prims = |id: &str| -> Vec<Prim> {
        scene_panels
            .iter()
            .flat_map(|p| p.layers.iter())
            .filter(|l| l.id == id)
            .flat_map(|l| l.prims.iter().cloned())
            .collect()
    };
    for e in emph.iter().filter(|e| e.kind != EmphasisKind::Highlight) {
        let all = |over: &mut Vec<Prim>, ps: &[u32]| {
            for rc in target_rects(ps) {
                over.push(rect(
                    rc.left,
                    rc.top,
                    rc.right - rc.left,
                    rc.bottom - rc.top,
                    veil,
                ));
            }
        };
        match (e.kind, &e.layer, e.bars) {
            (EmphasisKind::Focus, Some(l), _) => {
                all(&mut over, &[]);
                over.extend(layer_prims(l));
            }
            (EmphasisKind::Focus, None, bars) => {
                for (id, rc) in panel_ids.iter().zip(&rects) {
                    let target = e.panels.is_empty() || e.panels.contains(id);
                    match (target, bars.and_then(|(a, b)| bar_px(a, b))) {
                        (true, Some((x0, x1))) => {
                            over.push(rect(
                                rc.left,
                                rc.top,
                                x0 - rc.left,
                                rc.bottom - rc.top,
                                veil,
                            ));
                            over.push(rect(x1, rc.top, rc.right - x1, rc.bottom - rc.top, veil));
                        }
                        (true, None) if bars.is_none() => {}
                        _ => over.push(rect(
                            rc.left,
                            rc.top,
                            rc.right - rc.left,
                            rc.bottom - rc.top,
                            veil,
                        )),
                    }
                }
            }
            (EmphasisKind::Dim, Some(l), _) => {
                over.extend(layer_prims(l).into_iter().map(|p| recolor(p, veil)));
            }
            (EmphasisKind::Dim, None, Some((a, b))) => {
                if let Some((x0, x1)) = bar_px(a, b) {
                    for rc in target_rects(&e.panels) {
                        over.push(rect(x0, rc.top, x1 - x0, rc.bottom - rc.top, veil));
                    }
                }
            }
            (EmphasisKind::Dim, None, None) => all(&mut over, &e.panels),
            _ => {}
        }
    }

    // (R5/m17-int, K49) Glossary terms of a capsule scene under a voice.
    let terms = crate::terms::mark(&mut scene_panels, provider, look);

    // (R5/T3) Type roles: each text's font from its role (marks: by size, glyph texts:
    // `glyph`); the role itself stays in the scene only when asked for.
    // (Layer texts were settled when drawn, so a focus copy keeps its font.)
    for p in under.iter_mut().chain(over.iter_mut()) {
        settle_text(p, look, None, opts.roles);
    }

    Ok(SceneDesc {
        view: view.id.clone(),
        width,
        height,
        background: theme.bg,
        x: xmap,
        bars: n_bars,
        lod,
        panels: scene_panels,
        emphasis: emph
            .into_iter()
            .map(|e| SceneEmphasis {
                kind: e.kind,
                panels: e.panels,
                layer: e.layer,
                from_bar: e.bars.map(|b| b.0),
                to_bar: e.bars.map(|b| b.1),
            })
            .collect(),
        under,
        over,
        terms,
    })
}
