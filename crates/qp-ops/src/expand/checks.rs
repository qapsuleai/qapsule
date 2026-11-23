//! Validation helpers for ids, studies, targets, points, confidence and evidence values.

use super::*;

pub(super) fn check_id(id: &str, path: &str) -> PResult<()> {
    // (R5B, §25.5) The authority's watch ids are reserved: no given id may take (or
    // probe) one.
    if id.starts_with(crate::watch::WATCH_PREFIX) {
        return Err(E::new(
            ErrorCode::Invalid,
            format!(
                "ids starting with `{}` are reserved",
                crate::watch::WATCH_PREFIX
            ),
        )
        .at(path)
        .hint("Choose another id or omit it to have one assigned.")
        .build());
    }
    if valid_id(id) {
        Ok(())
    } else {
        Err(E::new(ErrorCode::Invalid, format!("invalid id `{id}`"))
            .at(path)
            .expected(["^[A-Za-z0-9][A-Za-z0-9_.:-]{0,63}$"])
            .build())
    }
}

/// Validates a study and returns its full parameters (defaults merged).
pub(super) fn check_study(
    doc: &Document,
    study: &str,
    inputs: &[String],
    params: &BTreeMap<String, f64>,
    path: &str,
    study_key: &str,
) -> PResult<BTreeMap<String, f64>> {
    let Some(defaults) = qp_compute::default_params(study) else {
        return Err(
            E::new(ErrorCode::Invalid, format!("unknown study `{study}`"))
                .at(ptr(path, study_key))
                .expected(qp_compute::STUDY_NAMES)
                .dym(study, qp_compute::STUDY_NAMES)
                .build(),
        );
    };
    if inputs.len() != 1 {
        return Err(E::new(
            ErrorCode::Invalid,
            format!("`{study}` takes exactly one input bar series"),
        )
        .at(ptr(path, "inputs"))
        .build());
    }
    let ip = ptr(&ptr(path, "inputs"), 0);
    let input = need_series(doc, &inputs[0], &ip)?;
    if !qp_plane::plane::is_bar_series(input) {
        return Err(invalid(
            format!("input `{}` is not a bar series", inputs[0]),
            ip,
        ));
    }
    let pp = ptr(path, "params");
    for (k, v) in params {
        if !defaults.contains_key(k) {
            return Err(E::new(
                ErrorCode::Invalid,
                format!("unknown parameter `{k}` for `{study}`"),
            )
            .at(ptr(&pp, k))
            .expected(defaults.keys().cloned())
            .dym(k, defaults.keys().map(String::as_str))
            .build());
        }
        let one: BTreeMap<String, f64> = [(k.clone(), *v)].into_iter().collect();
        if let Err(e) = qp_compute::create(study, &one, qp_compute::Field::Close) {
            return Err(invalid(e.to_string(), ptr(&pp, k)));
        }
    }
    match qp_compute::create(study, params, qp_compute::Field::Close) {
        Ok(s) => Ok(s.params()),
        Err(e) => Err(invalid(e.to_string(), pp)),
    }
}

pub(super) fn check_target(doc: &Document, t: &Target, path: &str) -> PResult<()> {
    if let Some(v) = &t.view {
        need_view(doc, v, &ptr(path, "view"))?;
    }
    check_target_layer(doc, t, path)?;
    if let Some(s) = &t.series {
        need_series(doc, s, &ptr(path, "series"))?;
    }
    if let Some(i) = &t.item {
        need_item(doc, i, &ptr(path, "item"))?;
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

/// Checks an inline point series: not empty, strictly ascending `t`, finite `v` (a
/// `null` value is allowed, as in study output). Returns the offending index and why.
pub(super) fn check_points(points: &[qp_schema::Point]) -> Result<(), (usize, String)> {
    if points.is_empty() {
        return Err((0, "point list is empty".to_owned()));
    }
    let mut prev: Option<i64> = None;
    for (i, pt) in points.iter().enumerate() {
        if pt.v.is_some_and(|v| !v.is_finite()) {
            return Err((i, format!("point {i}: non-finite value")));
        }
        if prev.is_some_and(|p| pt.t <= p) {
            return Err((i, format!("point {i}: timestamp not increasing")));
        }
        prev = Some(pt.t);
    }
    Ok(())
}

pub(super) fn check_confidence(c: f64, path: &str) -> PResult<()> {
    if c.is_finite() && (0.0..=1.0).contains(&c) {
        Ok(())
    } else {
        Err(E::new(
            ErrorCode::Invalid,
            format!("confidence {c} is outside [0, 1]"),
        )
        .at(path)
        .expected(["0..=1"])
        .build())
    }
}

/// The engine says the number (§18.5): on a study series, the evidence values `value`
/// and `prev` given by the client must be bit-equal to the study's main output at the
/// fact's bar and at the bar before it; otherwise `invalid` with the engine's value in
/// `expected`.
pub(super) fn check_evidence_values(
    doc: &Document,
    ctx: &Ctx<'_>,
    series: &str,
    bar: u64,
    evidence: &BTreeMap<String, f64>,
    p: &str,
) -> PResult<()> {
    if !evidence.contains_key("value") && !evidence.contains_key("prev") {
        return Ok(());
    }
    let Some(s) = find_series(doc, series) else {
        return Ok(());
    };
    if !matches!(s.source, SeriesSource::Study { .. }) {
        return Ok(());
    }
    // Inline bars are in `doc`; bound bars in the reading document (as of the command's
    // start: a study put earlier in the same command over a bound series is not there,
    // and is not checked).
    let bound = qp_plane::plane::bar_root(doc, series).is_some_and(|r| r.bars().is_none());
    let src = if bound {
        match &ctx.data {
            Some(r) if find_series(r.rd, series).is_some_and(|x| x.source == s.source) => r.rd,
            _ => return Ok(()),
        }
    } else {
        doc
    };
    let Ok(v) = qp_plane::values::values(src, &mut qp_plane::values::Cache::default(), series)
    else {
        return Ok(());
    };
    let main = v.main_values();
    let i = usize::try_from(bar).unwrap_or(usize::MAX);
    for (key, back) in [("value", 0usize), ("prev", 1usize)] {
        let Some(given) = evidence.get(key).copied() else {
            continue;
        };
        let at = i.checked_sub(back);
        let engine = at.and_then(|j| main.get(j).copied().flatten());
        let kp = ptr(&ptr(p, "evidence"), key);
        match engine {
            Some(n) if n.to_bits() == given.to_bits() => {}
            Some(n) => {
                return Err(E::new(
                    ErrorCode::Invalid,
                    format!(
                        "evidence `{key}` {} is not the engine's value of `{series}` at bar {}: {}",
                        num(given),
                        at.unwrap_or(0),
                        num(n)
                    ),
                )
                .at(kp)
                .expected([num(n)])
                .hint("The engine says the number: write its value (query `study` or `at`), or omit the key.")
                .build());
            }
            None => {
                return Err(E::new(
                    ErrorCode::Invalid,
                    match at {
                        Some(j) => format!(
                            "evidence `{key}`: `{series}` has no value at bar {j} (warm-up)"
                        ),
                        None => format!("evidence `{key}`: bar {bar} has no previous bar"),
                    },
                )
                .at(kp)
                .hint("Omit the key: the engine has no number for it.")
                .build());
            }
        }
    }
    Ok(())
}
