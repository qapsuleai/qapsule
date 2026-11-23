//! [`prepare`]: one layer's data, channels, constants and colours into value rows, in
//! named steps (RF1/G3): the data source (`source`), the encoding checks, the rows of the
//! layer's mark (`series`, `rule`, `panel`, `point`) and the finished [`Prepared`].

use qp_look::theme::Theme;

use super::*;

mod panel;
mod point;
mod rule;
mod series;
mod source;

/// The step results `prepare` carries from the encoding checks to [`Prepared`].
struct PrepState {
    anchor: Anchor,
    glyphs: Vec<GlyphSpec>,
    seg_extent: Option<(u64, u64)>,
    label: Option<String>,
    skipped: u64,
    keys: Vec<String>,
    fields: Vec<String>,
    traces: Vec<Trace>,
    width: f64,
}

/// What the encoding checks decide for the mark step.
struct Encoding {
    opacity: f64,
    x2_given: bool,
    extent: Option<(u64, u64)>,
}

/// Prepares one layer. `palette` counts uncoloured line/area/band traces in the panel.
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
    let (src, data, all_rows) = source::layer_source(fr, layer, id)?;
    let cx = Ctx {
        fr,
        id,
        layer,
        src,
        all_rows,
        constants,
    };
    let checked = check_encoding(&cx)?;
    let mut st = start_state(fr, layer);
    let geom = mark_rows(&cx, &mut st, palette, &checked)?;
    Ok(finish(cx, st, geom, data, n_data))
}

/// Opacity, width, `x` and `x2` rules of the layer's encoding; the bar extent of `x`..`x2`.
fn check_encoding(cx: &Ctx<'_, '_>) -> Result<Encoding, GrammarError> {
    let layer = cx.layer;
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
    Ok(Encoding {
        opacity,
        x2_given,
        extent,
    })
}

/// The empty state before the mark step, with the layer's legend label.
fn start_state(fr: &Frame<'_>, layer: &Layer) -> PrepState {
    let label = layer.label.clone().or_else(|| {
        layer
            .series
            .as_ref()
            .map(|s| fr.p.label(s).unwrap_or_else(|| s.clone()))
    });
    PrepState {
        anchor: Anchor::default(),
        glyphs: Vec::new(),
        seg_extent: None,
        label,
        skipped: 0u64,
        keys: Vec::new(),
        fields: Vec::new(),
        traces: Vec::new(),
        width: layer.encoding.width.unwrap_or(1.0),
    }
}

/// The rows of the layer's mark: one step per mark.
fn mark_rows(
    cx: &Ctx<'_, '_>,
    st: &mut PrepState,
    palette: &mut usize,
    checked: &Encoding,
) -> Result<Geom, GrammarError> {
    let enc = &cx.layer.encoding;
    let opacity = checked.opacity;
    match cx.layer.mark {
        Mark::Candle => series::candle(cx, st, opacity),
        Mark::Line => series::line(cx, st, palette, opacity),
        Mark::Area | Mark::Band | Mark::Bar => {
            series::area_band_bar(cx, st, palette, opacity, checked.extent)
        }
        Mark::Rule => rule::rule(cx, st, opacity, checked.x2_given),
        Mark::Text if is_panel(enc.x.as_ref()) || is_panel(enc.y.as_ref()) => {
            panel::panel_text(cx, st, opacity)
        }
        Mark::Glyph => panel::glyph(cx, st),
        Mark::Point | Mark::Text => point::point_text(cx, st, opacity),
    }
}

/// The main value channel: `y`, or the source's main field.
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

/// The next panel palette colour, unless the layer gives its own colour rule.
fn next_palette(theme: &Theme, palette: &mut usize, rule_given: bool) -> Option<Rgba> {
    if rule_given {
        None
    } else {
        let c = theme.palette(*palette);
        *palette += 1;
        Some(c)
    }
}

/// The prepared layer from the steps' results.
fn finish(cx: Ctx<'_, '_>, st: PrepState, geom: Geom, data: DataSource, n_data: usize) -> Prepared {
    let layer = cx.layer;
    Prepared {
        id: cx.id.to_owned(),
        mark: layer.mark,
        panel: layer.panel,
        data,
        fields: st.fields,
        constants: cx.constants,
        label: st.label,
        geom,
        traces: st.traces,
        keys: st.keys,
        width: st.width,
        edge: layer.encoding.width.is_some(),
        skipped: st.skipped,
        has_data: n_data == 1,
        anchor: st.anchor,
        extent: st.seg_extent,
        glyphs: st.glyphs,
    }
}
