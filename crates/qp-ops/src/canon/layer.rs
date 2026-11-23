//! Layer checks: fields, references, positions, channels, view targets and emphasis.

use super::*;

fn unknown_field(field: &str, of: &str, fields: &[String], path: &str) -> ProtocolErr {
    if fields.is_empty() {
        return E::new(
            ErrorCode::Invalid,
            format!("{of} has no fields, so `{field}` cannot be drawn"),
        )
        .at(path)
        .hint("Give the layer a series, fact or finding, or use a constant with an origin.")
        .build();
    }
    E::new(ErrorCode::Invalid, format!("{of} has no field `{field}`"))
        .at(path)
        .dym(field, fields.iter().map(String::as_str))
        .expected(fields.iter().cloned())
        .build()
}

type ProtocolErr = qp_schema::protocol::ProtocolError;

fn data_name(d: Data<'_>) -> String {
    match d {
        Data::None => "a layer without data".to_owned(),
        Data::Series(s) => format!("series `{}`", s.id),
        Data::Fact(f) => format!("fact `{}`", f.id),
        Data::Finding(f) => format!("finding `{}`", f.id),
    }
}

pub(super) fn check_field(doc: &Document, d: Data<'_>, field: &str, path: &str) -> PResult<()> {
    let fields = data_fields(doc, d);
    if fields.iter().any(|f| f == field) {
        Ok(())
    } else {
        Err(unknown_field(field, &data_name(d), &fields, path))
    }
}

fn check_ref(doc: &Document, d: Data<'_>, r: &DataRef, path: &str) -> PResult<()> {
    let target = match &r.series {
        Some(s) => Data::Series(need_series(doc, s, &ptr(path, "series"))?),
        None => d,
    };
    if matches!(target, Data::None) {
        return Err(E::new(
            ErrorCode::Invalid,
            "a field reference without `series` needs layer data",
        )
        .at(path)
        .hint("Name the series: {\"series\":…,\"field\":…}.")
        .build());
    }
    if let Some(f) = &r.field {
        check_field(doc, target, f, &ptr(path, "field"))?;
    }
    Ok(())
}

/// Checks a position channel; returns its constant, if it is one.
fn check_position(doc: &Document, d: Data<'_>, p: &Position, path: &str) -> PResult<Option<f64>> {
    match p {
        Position::Number(v) => Ok(Some(*v)),
        Position::Name(s) => {
            if let Some(v) = numeric(s) {
                if !v.is_finite() {
                    return Err(invalid(format!("constant `{s}` is not finite"), path));
                }
                return Ok(Some(v));
            }
            check_field(doc, d, s, path)?;
            Ok(None)
        }
        Position::Ref(r) => {
            check_ref(doc, d, r, path)?;
            Ok(None)
        }
        // A layout position (R3, GRAMMAR §18): not a data constant; the compiler checks
        // which marks take it.
        Position::Panel(_) => Ok(None),
    }
}

/// Constants of a layer (pointer → value), in pointer order of the encoding.
#[allow(clippy::too_many_lines)]
fn check_channels(
    doc: &Document,
    d: Data<'_>,
    l: &Layer,
    site: &Site<'_>,
) -> PResult<Vec<(String, f64)>> {
    let e = &l.encoding;
    let mut consts = Vec::new();
    for (name, ch) in [("x", &e.x), ("x2", &e.x2), ("y", &e.y), ("y2", &e.y2)] {
        if let Some(p) = ch {
            let pointer = format!("/encoding/{name}");
            if let Some(v) = check_position(doc, d, p, &site.at(&pointer))? {
                consts.push((pointer, v));
            }
        }
    }
    if let Some(c) = &e.color {
        match c {
            ColorSpec::Constant(s) => check_color(s, &site.at("/encoding/color"))?,
            ColorSpec::When(w) => {
                check_field(
                    doc,
                    d,
                    &w.when.field,
                    &site.at("/encoding/color/when/field"),
                )?;
                match &w.when.than {
                    Operand::Number(v) => {
                        consts.push(("/encoding/color/when/than".to_owned(), *v));
                    }
                    Operand::Ref(r) => check_ref(doc, d, r, &site.at("/encoding/color/when/than"))?,
                }
                check_color(&w.then, &site.at("/encoding/color/then"))?;
                check_color(&w.otherwise, &site.at("/encoding/color/else"))?;
            }
            ColorSpec::Steps(s) => {
                check_field(doc, d, &s.field, &site.at("/encoding/color/field"))?;
                for (i, t) in s.thresholds.iter().enumerate() {
                    let tp = site.at(&format!("/encoding/color/thresholds/{i}"));
                    if !t.is_finite() {
                        return Err(invalid("thresholds must be finite", tp));
                    }
                    if i > 0 && *t <= s.thresholds[i - 1] {
                        return Err(E::new(
                            ErrorCode::Invalid,
                            "colour thresholds must be strictly ascending",
                        )
                        .at(tp)
                        .expected([format!("> {}", num(s.thresholds[i - 1]))])
                        .build());
                    }
                    consts.push((format!("/encoding/color/thresholds/{i}"), *t));
                }
                if s.colors.len() != s.thresholds.len() + 1 {
                    return Err(E::new(
                        ErrorCode::Invalid,
                        format!(
                            "{} thresholds need {} colours (got {})",
                            s.thresholds.len(),
                            s.thresholds.len() + 1,
                            s.colors.len()
                        ),
                    )
                    .at(site.at("/encoding/color/colors"))
                    .expected([(s.thresholds.len() + 1).to_string()])
                    .build());
                }
                for (i, c) in s.colors.iter().enumerate() {
                    check_color(c, &site.at(&format!("/encoding/color/colors/{i}")))?;
                }
            }
        }
    }
    if let Some(o) = e.opacity {
        if !(o.is_finite() && (0.0..=1.0).contains(&o)) {
            return Err(E::new(
                ErrorCode::Invalid,
                format!("opacity {} is outside [0, 1]", num(o)),
            )
            .at(site.at("/encoding/opacity"))
            .expected(["0..=1"])
            .build());
        }
    }
    if let Some(w) = e.width {
        positive(w, "width", &site.at("/encoding/width"))?;
    }
    match &e.size {
        Some(SizeSpec::Constant(s)) => positive(*s, "size", &site.at("/encoding/size"))?,
        Some(SizeSpec::ByField(s)) => {
            check_field(doc, d, &s.field, &site.at("/encoding/size/field"))?;
            let [a, b] = s.range;
            positive(a, "size range min", &site.at("/encoding/size/range/0"))?;
            positive(b, "size range max", &site.at("/encoding/size/range/1"))?;
            if a > b {
                return Err(E::new(ErrorCode::Invalid, "size range must be [min, max]")
                    .at(site.at("/encoding/size/range"))
                    .expected(["min <= max"])
                    .build());
            }
        }
        None => {}
    }
    if let Some(t) = &e.text {
        check_field(doc, d, t, &site.at("/encoding/text"))?;
    }
    Ok(consts)
}

#[allow(clippy::too_many_lines)]
/// Validates one layer in its view and brings it to canonical form (origins of
/// constants; constants written from origins). The layer's id must be assigned and its
/// panel must be in `site.view.panels`; `site.view.layers` are the other layers (the
/// layer itself may be among them, matched by id).
pub fn check_layer(doc: &Document, l: &mut Layer, site: &Site<'_>) -> PResult<()> {
    let n = usize::from(l.series.is_some())
        + usize::from(l.fact.is_some())
        + usize::from(l.finding.is_some());
    if n > 1 {
        let second = if l.series.is_some() && l.fact.is_some() {
            "/fact"
        } else {
            "/finding"
        };
        return Err(E::new(
            ErrorCode::Invalid,
            "a layer draws at most one of `series`, `fact`, `finding`",
        )
        .at(site.at(second))
        .hint("Keep one data reference; draw the others in their own layers.")
        .build());
    }
    if let Some(s) = &l.series {
        need_series(doc, s, &site.at("/series"))?;
    }
    if let Some(f) = &l.fact {
        need_fact(doc, f, &site.at("/fact"))?;
    }
    if let Some(f) = &l.finding {
        need_finding(doc, f, &site.at("/finding"))?;
    }
    need_panel(site.view, l.panel, &site.at("/panel"))?;
    let d = layer_data(doc, l);
    match l.mark {
        Mark::Candle => {
            let ok = matches!(d, Data::Series(s) if qp_plane::plane::is_bar_series(s));
            if !ok {
                return Err(E::new(ErrorCode::Invalid, "`candle` needs a bar series")
                    .at(site.at(if l.series.is_some() {
                        "/series"
                    } else {
                        "/mark"
                    }))
                    .expected(
                        doc.series
                            .iter()
                            .filter(|s| qp_plane::plane::is_bar_series(s))
                            .map(|s| s.id.clone()),
                    )
                    .build());
            }
        }
        Mark::Band => {
            // A channel may be given by its origin alone (the authority writes the value).
            let has_y = l.encoding.y.is_some() || l.origin.contains_key("/encoding/y");
            let has_y2 = l.encoding.y2.is_some() || l.origin.contains_key("/encoding/y2");
            if !has_y || !has_y2 {
                let missing = if has_y { "y2" } else { "y" };
                return Err(E::new(
                    ErrorCode::Invalid,
                    format!("`band` needs `y` and `y2`; `{missing}` is missing"),
                )
                .at(site.at("/encoding"))
                .expected(["y", "y2"])
                .build());
            }
        }
        Mark::Rule => {
            let has = |p: &Option<Position>, k: &str| {
                p.as_ref().and_then(position_constant).is_some() || l.origin.contains_key(k)
            };
            if !has(&l.encoding.y, "/encoding/y") && !has(&l.encoding.x, "/encoding/x") {
                return Err(E::new(
                    ErrorCode::Invalid,
                    "`rule` needs a constant `y` (horizontal) or `x` (vertical), or an origin for one",
                )
                .at(site.at("/encoding/y"))
                .hint("E.g. \"encoding\":{\"y\":70} with \"origin\":{\"/encoding/y\":{\"kind\":\"threshold\",\"name\":\"rsi.overbought\"}}.")
                .build());
            }
        }
        Mark::Text => {
            if l.encoding.text.is_none() {
                return Err(
                    E::new(ErrorCode::Invalid, "`text` needs the `text` channel")
                        .at(site.at("/encoding"))
                        .expected(["text"])
                        .build(),
                );
            }
        }
        Mark::Glyph => {
            if l.finding.is_none() {
                return Err(E::new(
                    ErrorCode::Invalid,
                    "`glyph` draws the capsule of a finding: bind the layer to its `finding`",
                )
                .at(site.at("/finding"))
                .build());
            }
        }
        Mark::Line | Mark::Area | Mark::Bar | Mark::Point => {}
    }
    let consts = check_channels(doc, d, l, site)?;
    resolve_origins(doc, l, &consts, site)
}

/// Checks a target inside view `view` (emphasis).
pub fn check_view_target(doc: &Document, view: &View, t: &Target, path: &str) -> PResult<()> {
    if let Some(v) = &t.view {
        if *v != view.id {
            return Err(E::new(
                ErrorCode::Invalid,
                format!("an emphasis of view `{}` cannot target view `{v}`", view.id),
            )
            .at(ptr(path, "view"))
            .expected([view.id.clone()])
            .build());
        }
    }
    if let Some(p) = t.panel {
        need_panel(view, p, &ptr(path, "panel"))?;
    }
    if let Some(l) = &t.layer {
        need_layer(view, l, &ptr(path, "layer"))?;
    }
    if let Some(s) = &t.series {
        need_series(doc, s, &ptr(path, "series"))?;
    }
    if let Some(i) = &t.item {
        if find_collection(doc, i).is_none() {
            return Err(
                E::new(ErrorCode::NotFound, format!("item `{i}` does not exist"))
                    .at(ptr(path, "item"))
                    .dym(i, all_ids(doc))
                    .build(),
            );
        }
    }
    if let (Some(a), Some(b)) = (t.from_bar, t.to_bar) {
        if a > b {
            return Err(invalid(
                format!("from_bar {a} is after to_bar {b}"),
                ptr(path, "from_bar"),
            ));
        }
    }
    Ok(())
}

pub fn check_emphasis(
    doc: &Document,
    view: &View,
    emphasis: &[Emphasis],
    path: &str,
) -> PResult<()> {
    for (i, e) in emphasis.iter().enumerate() {
        check_view_target(doc, view, &e.target, &ptr(&ptr(path, i), "target"))?;
    }
    Ok(())
}
