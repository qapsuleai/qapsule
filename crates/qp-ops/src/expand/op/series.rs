//! (RF1 S4) `expand_op` arms for series and bars: `put_series`, `add_study`, `set_params`,
//! `append_bars`, `update_last_bar`.

use super::*;

/// `put_series`: a series (inline, study or bound) checked and stored.
pub(super) fn put_series(
    doc: &Document,
    series: &Series,
    p: &str,
    ctx: &mut Ctx<'_>,
) -> PResult<Vec<ChangeOp>> {
    let sp = ptr(p, "series");
    check_id(&series.id, &ptr(&sp, "id"))?;
    match find_collection(doc, &series.id) {
        Some(qp_schema::protocol::Collection::Series) | None => {}
        Some(c) => {
            return Err(E::new(
                ErrorCode::Duplicate,
                format!("id `{}` already exists (in {})", series.id, coll_name(c)),
            )
            .at(ptr(&sp, "id"))
            .build())
        }
    }
    let mut s = series.clone();
    put_series_source(doc, series, &mut s, &sp, ctx)?;
    rebind_entitled(doc, ctx, &s, &sp)?;
    if find_series(doc, &s.id).is_none() {
        ctx.note_created(&s.id);
    }
    Ok(vec![ChangeOp::PutSeries {
        series: s,
        index: None,
    }])
}

/// `put_series`, its source: an inline series' data, a study's inputs, parameters and
/// output (`s` gets the full parameters), a binding; other sources are not accepted.
fn put_series_source(
    doc: &Document,
    series: &Series,
    s: &mut Series,
    sp: &str,
    ctx: &Ctx<'_>,
) -> PResult<()> {
    let src = ptr(sp, "source");
    match &series.source {
        SeriesSource::Inline {} => match &series.data {
            Some(SeriesData::Bars { bars }) => {
                if let Err(e) = qp_schema::validate_bars(bars) {
                    return Err(invalid(e, ptr(sp, "data")));
                }
            }
            Some(SeriesData::Points { points }) => {
                if let Err((i, why)) = check_points(points) {
                    return Err(invalid(why, ptr(&ptr(&ptr(sp, "data"), "points"), i)));
                }
            }
            None => {
                return Err(E::new(
                    ErrorCode::Unsupported,
                    "an inline series must carry data (`data.kind = \"bars\"` or `\"points\"`)",
                )
                .at(ptr(sp, "data"))
                .hint("v0 supports inline bar series, inline point series and study series.")
                .build())
            }
        },
        SeriesSource::Study {
            study,
            inputs,
            params,
            output,
        } => {
            let full = check_study(doc, study, inputs, params, &src, "study")?;
            if let Some(o) = output {
                let outs = study_outputs(study);
                if !outs.contains(o) {
                    return Err(E::new(
                        ErrorCode::Invalid,
                        format!("`{study}` has no output `{o}`"),
                    )
                    .at(ptr(&src, "output"))
                    .dym(o, outs.iter().map(String::as_str))
                    .expected(outs.clone())
                    .build());
                }
            }
            if series.data.is_some() {
                return Err(invalid("a study series carries no data", ptr(sp, "data")));
            }
            s.source = SeriesSource::Study {
                study: study.clone(),
                inputs: inputs.clone(),
                params: full,
                output: output.clone(),
            };
        }
        SeriesSource::Bind { .. } => {
            check_bind(doc, series, sp, ctx.plane.max_bind_bars)?;
        }
        SeriesSource::File { .. } | SeriesSource::Matriks { .. } | SeriesSource::Nats { .. } => {
            return Err(E::new(
                ErrorCode::Unsupported,
                "this authority accepts only `inline`, `study` and `bind` series sources",
            )
            .at(ptr(&src, "kind"))
            .expected(["inline", "study", "bind"])
            .hint(
                "File and feed bindings are opened by the host configuration, not by the protocol.",
            )
            .build())
        }
    }
    Ok(())
}

/// `add_study`: a study series over inputs, optionally shown in a view.
#[allow(clippy::ptr_arg)] // the match's field bindings (`&String`, `&Vec<_>`); bodies unchanged
pub(super) fn add_study(
    arm: Arm<'_, '_>,
    id: &Option<String>,
    study: &String,
    inputs: &Vec<String>,
    params: &BTreeMap<String, f64>,
    show: &Option<qp_schema::protocol::Show>,
) -> PResult<Vec<ChangeOp>> {
    let Arm { doc, p, ctx } = arm;
    let full = check_study(doc, study, inputs, params, p, "study")?;
    // Validate `show` before assigning the id so errors point at the cause.
    let show_view = match show {
        Some(sh) => Some(check_show(doc, sh, &ptr(p, "show"))?),
        None => None,
    };
    let sid = ctx.new_id(doc, "series", id.as_ref(), &ptr(p, "id"))?;
    let input = find_series(doc, &inputs[0]);
    let series = Series {
        id: sid.clone(),
        instrument: input.and_then(|s| s.instrument.clone()),
        interval: input.and_then(|s| s.interval.clone()),
        source: SeriesSource::Study {
            study: study.clone(),
            inputs: inputs.clone(),
            params: full,
            output: None,
        },
        data: None,
    };
    let mut out = vec![ChangeOp::PutSeries {
        series,
        index: None,
    }];
    if let Some((view, panel, new_panel)) = show_view {
        let mut scratch = doc.clone();
        apply_all(&mut scratch, &out).map_err(|m| invalid(m, p))?;
        let mut v = view.clone();
        if new_panel {
            let pn = Panel {
                id: Some(panel),
                name: None,
                weight: None,
                y: Scale {
                    id: Some(format!("y{panel}")),
                    ..Scale::new(ScaleKind::Linear)
                },
            };
            out.push(ChangeOp::PutPanel {
                view: v.id.clone(),
                index: u32::try_from(v.panels.len()).unwrap_or(u32::MAX),
                panel: pn.clone(),
            });
            v.panels.push(pn);
        }
        let mut layer = Layer::new(Mark::Line, panel);
        layer.series = Some(sid);
        let index = v.layers.len();
        let l = new_layer(&scratch, &v, &layer, index, &ptr(p, "show"), ctx.reads())?;
        out.push(ChangeOp::InsertLayer {
            view: v.id.clone(),
            index: u32::try_from(index).unwrap_or(u32::MAX),
            layer: l,
        });
    }
    Ok(out)
}

/// `set_params`: new parameters of a study series.
pub(super) fn set_params(
    doc: &Document,
    series: &String,
    params: &BTreeMap<String, f64>,
    p: &str,
) -> PResult<Vec<ChangeOp>> {
    let s = need_series(doc, series, &ptr(p, "series"))?;
    let SeriesSource::Study {
        study,
        inputs,
        params: old,
        output,
    } = &s.source
    else {
        return Err(invalid(
            format!("series `{series}` is not a study series"),
            ptr(p, "series"),
        ));
    };
    let mut merged = old.clone();
    for (k, v) in params {
        merged.insert(k.clone(), *v);
    }
    // Report unknown/invalid keys at the client's own params path.
    let defaults = qp_compute::default_params(study).unwrap_or_default();
    let pp = ptr(p, "params");
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
    let full = match qp_compute::create(study, &merged, qp_compute::Field::Close) {
        Ok(st) => st.params(),
        Err(e) => return Err(invalid(e.to_string(), pp)),
    };
    let mut ns = s.clone();
    ns.source = SeriesSource::Study {
        study: study.clone(),
        inputs: inputs.clone(),
        params: full,
        output: output.clone(),
    };
    Ok(vec![ChangeOp::PutSeries {
        series: ns,
        index: None,
    }])
}

/// `append_bars`: bars appended to an inline bar series.
#[allow(clippy::ptr_arg)] // the match's field bindings (`&String`, `&Vec<_>`); bodies unchanged
pub(super) fn append_bars(
    doc: &Document,
    series: &String,
    bars: &Vec<qp_schema::Bar>,
    p: &str,
) -> PResult<Vec<ChangeOp>> {
    let s = need_series(doc, series, &ptr(p, "series"))?;
    bound_no_bar_writes(s, &ptr(p, "series"))?;
    let Some(old) = inline_bars(s) else {
        return Err(invalid(
            format!("series `{series}` is not an inline bar series"),
            ptr(p, "series"),
        ));
    };
    if bars.is_empty() {
        return Err(invalid("no bars to append", ptr(p, "bars")));
    }
    let bp = ptr(p, "bars");
    let mut prev = old.last().map(|b| b.t);
    for (k, b) in bars.iter().enumerate() {
        if prev.is_some_and(|t| b.t <= t) {
            return Err(E::new(
                ErrorCode::Invalid,
                format!("bar time {} is not after the previous bar", b.t),
            )
            .at(ptr(&ptr(&bp, k), "t"))
            .expected([format!("> {}", prev.unwrap_or_default())])
            .build());
        }
        if let Err(e) = qp_schema::validate_bars(std::slice::from_ref(b)) {
            return Err(invalid(
                e.replace("bar 0", &format!("bar {k}")),
                ptr(&bp, k),
            ));
        }
        prev = Some(b.t);
    }
    Ok(vec![ChangeOp::AppendBars {
        series: series.clone(),
        bars: bars.clone(),
    }])
}

/// `update_last_bar`: the last bar of an inline bar series replaced.
pub(super) fn update_last_bar(
    doc: &Document,
    series: &String,
    bar: &qp_schema::Bar,
    p: &str,
) -> PResult<Vec<ChangeOp>> {
    let s = need_series(doc, series, &ptr(p, "series"))?;
    bound_no_bar_writes(s, &ptr(p, "series"))?;
    let Some(last) = inline_bars(s).and_then(<[qp_schema::Bar]>::last) else {
        return Err(invalid(
            format!("series `{series}` is not an inline bar series with bars"),
            ptr(p, "series"),
        ));
    };
    if bar.t != last.t {
        return Err(E::new(
            ErrorCode::Invalid,
            format!(
                "bar time {} differs from the last bar's time {}",
                bar.t, last.t
            ),
        )
        .at(ptr(&ptr(p, "bar"), "t"))
        .expected([last.t.to_string()])
        .hint("To add a new bar use append_bars.")
        .build());
    }
    if let Err(e) = qp_schema::validate_bars(std::slice::from_ref(bar)) {
        return Err(invalid(e, ptr(p, "bar")));
    }
    Ok(vec![ChangeOp::SetLastBar {
        series: series.clone(),
        bar: bar.clone(),
    }])
}
