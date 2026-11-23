//! Position origins: fact and bound values, labels, candidates and their resolution.

use super::*;

/// The `value` of fact `f` when its series rests on a bound series (bars not in the
/// document): from the reading document, gated by the writer's entitlement (§18.3).
/// `Ok(None)`: not a bound fact, or not computable (the caller says why).
fn bound_fact_value(
    doc: &Document,
    f: &Fact,
    reads: Option<Reads<'_>>,
    path: &str,
) -> PResult<Option<OriginValue>> {
    let bound = qp_plane::plane::bar_root(doc, &f.series)
        .is_some_and(|r| r.bars().is_none() && matches!(r.source, SeriesSource::Bind { .. }));
    if !bound {
        return Ok(None);
    }
    let Some(r) = reads else {
        return Ok(Some(OriginValue::Unknown));
    };
    if let Some(b) = r.reader.plane.gate(doc, Some(r.writer), &f.series) {
        return Err(b.error(&f.series, &ptr(path, "fact")));
    }
    let v = values(r.reader.rd, &mut Cache::default(), &f.series)
        .ok()
        .and_then(|v| {
            v.main_values()
                .get(usize::try_from(f.bar).ok()?)
                .copied()
                .flatten()
        });
    Ok(v.map(OriginValue::Value))
}

/// Whether origin `o` is an evidence value of a licensed fact (R3, K33): `bar`, `t` and
/// geometry bars are not values of the licensed data.
fn licensed_value(doc: &Document, o: &Origin) -> bool {
    let Origin::Fact { fact, field } = o else {
        return false;
    };
    field != "bar"
        && field != "t"
        && !field.starts_with("geometry.")
        && doc.facts.iter().any(|f| {
            &f.id == fact
                && f.meaning
                    .as_ref()
                    .is_some_and(|m| m.license.class == qp_schema::capsule::LicenseClass::Licensed)
        })
}

/// The value of an origin.
fn origin_value(
    doc: &Document,
    o: &Origin,
    path: &str,
    reads: Option<Reads<'_>>,
) -> PResult<OriginValue> {
    match o {
        Origin::Fact { fact, field } => {
            let f = need_fact(doc, fact, &ptr(path, "fact"))?;
            if field == "value" && !f.evidence.contains_key("value") {
                if let Some(v) = bound_fact_value(doc, f, reads, path)? {
                    return Ok(v);
                }
            }
            match fact_value(doc, f, field) {
                Some(v) => Ok(OriginValue::Value(v)),
                None => {
                    let mut ok: Vec<String> = vec!["bar".to_owned(), "t".to_owned()];
                    ok.extend(f.evidence.keys().cloned());
                    if !ok.iter().any(|x| x == "value") {
                        ok.push("value".to_owned());
                    }
                    Err(E::new(
                        ErrorCode::Invalid,
                        format!("fact `{fact}` has no numeric field `{field}`"),
                    )
                    .at(ptr(path, "field"))
                    .dym(field, ok.iter().map(String::as_str))
                    .expected(ok)
                    .build())
                }
            }
        }
        Origin::Threshold { name } => threshold(name).map(OriginValue::Value).ok_or_else(|| {
            E::new(ErrorCode::NotFound, format!("no named threshold `{name}`"))
                .at(ptr(path, "name"))
                .dym(name, THRESHOLDS.iter().map(|(n, _)| *n))
                .expected(THRESHOLDS.iter().map(|(n, _)| (*n).to_owned()))
                .hint("{\"kind\":\"describe\"} lists the thresholds.")
                .build()
        }),
        Origin::Series { series, field, .. } => {
            let s = need_series(doc, series, &ptr(path, "series"))?;
            if let Some(f) = field {
                check_field(doc, Data::Series(s), f, &ptr(path, "field"))?;
            }
            Ok(OriginValue::Live)
        }
    }
}

/// A numeric field of a fact: `bar`, `t`, an evidence key, or `value` (the main output
/// of the fact's series at the fact's bar when the evidence has no `value`).
#[allow(clippy::cast_precision_loss)]
pub(crate) fn fact_value(doc: &Document, f: &Fact, field: &str) -> Option<f64> {
    match field {
        "bar" => Some(f.bar as f64),
        "t" => Some(f.t as f64),
        k if f.evidence.contains_key(k) => f.evidence.get(k).copied(),
        // R3: a bar of the fact's evidence geometry (GRAMMAR §18).
        k if k.starts_with("geometry.") => qp_grammar::depth::geometry_field(f, k),
        "value" => {
            let v = values(doc, &mut Cache::default(), &f.series).ok()?;
            v.main_values()
                .get(usize::try_from(f.bar).ok()?)
                .copied()
                .flatten()
        }
        _ => None,
    }
}

fn origin_label(o: &Origin) -> String {
    match o {
        Origin::Fact { fact, field } => format!("{fact}.{field}"),
        Origin::Threshold { name } => name.clone(),
        Origin::Series { series, field, .. } => match field {
            Some(f) => format!("{series}.{f} last"),
            None => format!("{series} last"),
        },
    }
}

/// Short text for an origin, e.g. `rsi.overbought`.
pub fn origin_text(o: &Origin) -> String {
    origin_label(o)
}

/// Studies drawn in panel `panel` (layers of `view` plus `extra`).
fn panel_studies<'a>(doc: &'a Document, view: &View, extra: &Layer) -> BTreeSet<&'a str> {
    let mut out = BTreeSet::new();
    let others = view
        .layers
        .iter()
        .filter(|l| l.id != extra.id && l.panel == extra.panel);
    for l in others.chain(std::iter::once(extra)) {
        if let Some(Series {
            source: SeriesSource::Study { study, .. },
            ..
        }) = l
            .series
            .as_ref()
            .and_then(|s| doc.series.iter().find(|x| &x.id == s))
        {
            out.insert(study.as_str());
        }
    }
    out
}

/// Series drawn in the panel of `extra`.
fn panel_series<'a>(view: &'a View, extra: &'a Layer) -> BTreeSet<&'a str> {
    view.layers
        .iter()
        .filter(|l| l.id != extra.id && l.panel == extra.panel)
        .chain(std::iter::once(extra))
        .filter_map(|l| l.series.as_deref())
        .collect()
}

fn origin_json(o: &Origin) -> String {
    serde_json::to_string(o).unwrap_or_default()
}

/// Candidate origins for constant `v` in the panel of `l`: thresholds of the panel's
/// studies and numeric evidence of facts on the panel's series, nearest value first.
fn candidates(doc: &Document, view: &View, l: &Layer, v: f64) -> Vec<Candidate> {
    let mut c: Vec<(f64, String, String)> = Vec::new();
    let studies = panel_studies(doc, view, l);
    for (name, tv) in THRESHOLDS {
        let study = name.split('.').next().unwrap_or_default();
        if studies.contains(study) {
            let o = Origin::Threshold {
                name: name.to_owned(),
            };
            c.push((
                (tv - v).abs(),
                origin_json(&o),
                format!("{name} = {}", num(tv)),
            ));
        }
    }
    let series = panel_series(view, l);
    for f in &doc.facts {
        if !series.contains(f.series.as_str()) {
            continue;
        }
        for (k, ev) in &f.evidence {
            if !ev.is_finite() {
                continue;
            }
            let o = Origin::Fact {
                fact: f.id.clone(),
                field: k.clone(),
            };
            c.push((
                (ev - v).abs(),
                origin_json(&o),
                format!("{} {k} {} (bar {})", f.kind, num(*ev), f.bar),
            ));
        }
    }
    c.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    c.into_iter()
        .take(10)
        .map(|(_, id, about)| Candidate { id, about })
        .collect()
}

/// Pointers of the layer that may carry an origin.
fn origin_slots(l: &Layer) -> Vec<String> {
    let mut v: Vec<String> = ["/encoding/x", "/encoding/x2", "/encoding/y", "/encoding/y2"]
        .map(str::to_owned)
        .to_vec();
    match &l.encoding.color {
        Some(ColorSpec::When(w)) if matches!(w.when.than, Operand::Number(_)) => {
            v.push("/encoding/color/when/than".to_owned());
        }
        Some(ColorSpec::Steps(s)) => {
            for i in 0..s.thresholds.len() {
                v.push(format!("/encoding/color/thresholds/{i}"));
            }
        }
        _ => {}
    }
    v
}

fn position_slot<'a>(l: &'a mut Layer, pointer: &str) -> Option<&'a mut Option<Position>> {
    match pointer {
        "/encoding/x" => Some(&mut l.encoding.x),
        "/encoding/x2" => Some(&mut l.encoding.x2),
        "/encoding/y" => Some(&mut l.encoding.y),
        "/encoding/y2" => Some(&mut l.encoding.y2),
        _ => None,
    }
}

#[allow(clippy::too_many_lines)]
/// Resolves the origins of a layer's constants (`docs/GRAMMAR.md` §10): explicit
/// origins must resolve and match their constant bit for bit; a missing constant is
/// written from its origin; a constant without an origin is bound to the one named
/// threshold of the panel's studies with exactly its value, else `no_origin`.
pub(super) fn resolve_origins(
    doc: &Document,
    l: &mut Layer,
    consts: &[(String, f64)],
    site: &Site<'_>,
) -> PResult<()> {
    let slots = origin_slots(l);
    let given: Vec<(String, Origin)> = l
        .origin
        .iter()
        .map(|(k, o)| (k.clone(), o.clone()))
        .collect();
    for (k, o) in &given {
        let op = ptr(&site.at("/origin"), k);
        if !slots.contains(k) {
            return Err(E::new(
                ErrorCode::Invalid,
                format!("`{k}` is not a constant of this layer"),
            )
            .at(op)
            .dym(k, slots.iter().map(String::as_str))
            .expected(slots.clone())
            .build());
        }
        let value = origin_value(doc, o, &op, site.reads)?;
        let literal = consts.iter().find(|(p, _)| p == k).map(|(_, v)| *v);
        let is_position = position_slot(l, k).is_some();
        if is_position {
            let field_there = position_slot(l, k)
                .and_then(|s| s.as_ref())
                .is_some_and(|p| position_constant(p).is_none());
            if field_there {
                return Err(E::new(
                    ErrorCode::Invalid,
                    format!("`{k}` draws a field, not a constant; it takes no origin"),
                )
                .at(op)
                .build());
            }
        }
        match (value, literal) {
            (OriginValue::Unknown, Some(_)) => {}
            (OriginValue::Unknown, None) => {
                return Err(E::new(
                    ErrorCode::Invalid,
                    format!(
                        "`{}` rests on bound data the authority cannot read here: write the constant at `{k}`",
                        origin_label(o)
                    ),
                )
                .at(site.at(k))
                .build())
            }
            (OriginValue::Live, Some(_)) => {
                return Err(E::new(
                    ErrorCode::Invalid,
                    format!("a `series` origin is live: do not write a constant at `{k}`"),
                )
                .at(site.at(k))
                .hint("Remove the constant, or bind it to a fact or a named threshold.")
                .build())
            }
            (OriginValue::Live, None) => {
                if !is_position {
                    return Err(E::new(
                        ErrorCode::Invalid,
                        format!("a `series` origin can only bind a position channel, not `{k}`"),
                    )
                    .at(op)
                    .expected(["/encoding/x", "/encoding/y", "/encoding/y2"])
                    .build());
                }
            }
            (OriginValue::Value(v), Some(lit)) => {
                if v.to_bits() != lit.to_bits() {
                    return Err(E::new(
                        ErrorCode::Invalid,
                        format!(
                            "constant {} at `{k}` differs from its origin `{}` = {}",
                            num(lit),
                            origin_label(o),
                            num(v)
                        ),
                    )
                    .at(site.at(k))
                    .expected([num(v)])
                    .hint("Write the origin's value, or omit the constant and the authority writes it.")
                    .build());
                }
            }
            (OriginValue::Value(v), None) => {
                // (R3, S-B §3) A licensed fact's evidence value is never written into a
                // stored view: the position keeps its origin only and the compiler
                // reads the value from the reader's provider (absent for a reader
                // without the entitlement: the geometry is not drawn).
                if licensed_value(doc, o) {
                    continue;
                }
                if let Some(slot) = position_slot(l, k) {
                    *slot = Some(Position::Number(v));
                }
            }
        }
    }
    for (k, v) in consts {
        if l.origin.contains_key(k) {
            continue;
        }
        let studies = panel_studies(doc, site.view, l);
        let names: BTreeSet<&str> = THRESHOLDS
            .iter()
            .filter(|(n, tv)| {
                tv.to_bits() == v.to_bits()
                    && studies.contains(n.split('.').next().unwrap_or_default())
            })
            .map(|(n, _)| *n)
            .collect();
        if names.len() == 1 {
            let name = names.into_iter().next().unwrap_or_default().to_owned();
            l.origin.insert(k.clone(), Origin::Threshold { name });
            continue;
        }
        let lid = l.id.clone().unwrap_or_default();
        let mut err = E::new(
            ErrorCode::NoOrigin,
            format!(
                "layer `{lid}` of view `{}`: constant {} at {k} has no origin",
                site.view.id,
                num(*v)
            ),
        )
        .at(site.at(k))
        .hint(format!(
            "Bind it to a fact (add one with add_fact, evidence {{\"level\":{}}}) or a named threshold, via the layer's `origin`.",
            num(*v)
        ))
        .build();
        err.candidates = candidates(doc, site.view, l, *v);
        return Err(err);
    }
    Ok(())
}
