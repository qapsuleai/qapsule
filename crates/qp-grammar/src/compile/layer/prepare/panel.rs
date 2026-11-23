//! The panel-positioned marks: a panel-relative `text` and the capsule's `glyph`.

use super::*;

/// A `text` placed panel-relative (`x` or `y`): a text field of a fact or finding.
pub(super) fn panel_text(
    cx: &Ctx<'_, '_>,
    st: &mut PrepState,
    opacity: f64,
) -> Result<Geom, GrammarError> {
    let (fr, enc, theme) = (cx.fr, &cx.layer.encoding, &cx.fr.theme);
    let geom = Geom::Text;
    st.anchor = Anchor {
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
    let f = text_field_of(cx)?;
    let y = if st.anchor.y.is_some() {
        None
    } else {
        Some(main_or_err(cx, "text")?)
    };
    let size = panel_text_size(cx)?;
    let rule = cx.color_rule(ColorRule::Const(theme.text))?;
    let mut rows = Vec::new();
    // A panel-relative `x` has no bar: the layer draws its first row only.
    let src_rows: Vec<&Row> = if st.anchor.x.is_some() {
        cx.all_rows.iter().take(1).collect()
    } else {
        cx.visible_rows().collect()
    };
    for r in src_rows {
        let v = match &y {
            Some((c, _)) => match c.at(r.slot, Some(r)) {
                Some(v) => Some(v),
                None => {
                    st.skipped += 1;
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
    st.traces.push(Trace {
        rows,
        split: Split::None,
        base,
        lod_line: false,
    });
    if let Some((_, k)) = y {
        st.fields.push(k.clone());
        st.keys.push(k);
    }
    st.fields.push(f);
    Ok(geom)
}

/// A panel-positioned `text`'s size: a constant > 0, 11 when absent.
fn panel_text_size(cx: &Ctx<'_, '_>) -> Result<f64, GrammarError> {
    let size = match &cx.layer.encoding.size {
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
    Ok(size)
}

/// `glyph`: the meaning glyph of the layer's finding, at `x`/`y` or atop the panel.
pub(super) fn glyph(cx: &Ctx<'_, '_>, st: &mut PrepState) -> Result<Geom, GrammarError> {
    let (fr, layer, theme) = (cx.fr, cx.layer, &cx.fr.theme);
    let enc = &layer.encoding;
    let geom = Geom::Glyph;
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
    glyph_channels(cx)?;
    let spec = fr.p.glyph(finding).ok_or_else(|| {
        cx.err(
            ErrorKind::NotFound,
            "/finding",
            format!(
                "finding `{finding}` has no meaning glyph (no R3 capsule with a story presents it)"
            ),
        )
    })?;
    let size = glyph_size(cx)?;
    // `y` absent: at the top of the panel (a glyph strip over the chart).
    st.anchor = Anchor {
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
    let y = if st.anchor.y.is_some() {
        None
    } else {
        cx.position(enc.y.as_ref(), "/encoding/y", "y")?
    };
    let mut rows = Vec::new();
    if let Some(r) = cx.all_rows.first() {
        let visible = st.anchor.x.is_some() || (r.slot >= fr.from && r.slot <= fr.to);
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
            st.glyphs.push(spec.into_owned());
        } else {
            st.skipped += 1;
        }
    }
    st.traces.push(Trace {
        rows,
        split: Split::None,
        base: theme.text,
        lod_line: false,
    });
    st.fields.push("glyph".to_owned());
    if let Some((_, k)) = y {
        st.fields.push(k.clone());
        st.keys.push(k);
    }
    Ok(geom)
}

/// A glyph takes only `x`, `y` and `size`.
fn glyph_channels(cx: &Ctx<'_, '_>) -> Result<(), GrammarError> {
    let enc = &cx.layer.encoding;
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
    Ok(())
}

/// A glyph's size: a constant > 0 in pixels, the look's default when absent.
fn glyph_size(cx: &Ctx<'_, '_>) -> Result<f64, GrammarError> {
    let size = match &cx.layer.encoding.size {
        None => cx.fr.look.glyph.default_px,
        Some(SizeSpec::Constant(s)) if s.is_finite() && *s > 0.0 => *s,
        Some(_) => {
            return Err(cx.err(
                ErrorKind::Invalid,
                "/encoding/size",
                "a glyph takes a constant size > 0 (pixels)",
            ))
        }
    };
    Ok(size)
}
