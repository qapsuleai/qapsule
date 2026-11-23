//! `expand_op`: one operation expanded to its changes (a single function, kept whole).

use super::*;

#[allow(clippy::cognitive_complexity)]
/// Expands one op into changes. `doc` is the scratch document *before* this op.
#[allow(clippy::too_many_lines)]
pub fn expand_op(doc: &Document, op: &Op, p: &str, ctx: &mut Ctx<'_>) -> PResult<Vec<ChangeOp>> {
    match op {
        Op::PutSeries { series } => {
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
            let src = ptr(&sp, "source");
            match &series.source {
                SeriesSource::Inline {} => match &series.data {
                    Some(SeriesData::Bars { bars }) => {
                        if let Err(e) = qp_schema::validate_bars(bars) {
                            return Err(invalid(e, ptr(&sp, "data")));
                        }
                    }
                    Some(SeriesData::Points { points }) => {
                        if let Err((i, why)) = check_points(points) {
                            return Err(invalid(why, ptr(&ptr(&ptr(&sp, "data"), "points"), i)));
                        }
                    }
                    None => {
                        return Err(E::new(
                            ErrorCode::Unsupported,
                            "an inline series must carry data (`data.kind = \"bars\"` or `\"points\"`)",
                        )
                        .at(ptr(&sp, "data"))
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
                        return Err(invalid(
                            "a study series carries no data",
                            ptr(&sp, "data"),
                        ));
                    }
                    s.source = SeriesSource::Study {
                        study: study.clone(),
                        inputs: inputs.clone(),
                        params: full,
                        output: output.clone(),
                    };
                }
                SeriesSource::Bind { .. } => {
                    check_bind(doc, series, &sp, ctx.plane.max_bind_bars)?;
                }
                SeriesSource::File { .. }
                | SeriesSource::Matriks { .. }
                | SeriesSource::Nats { .. } => {
                    return Err(E::new(
                        ErrorCode::Unsupported,
                        "this authority accepts only `inline`, `study` and `bind` series sources",
                    )
                    .at(ptr(&src, "kind"))
                    .expected(["inline", "study", "bind"])
                    .hint("File and feed bindings are opened by the host configuration, not by the protocol.")
                    .build())
                }
            }
            rebind_entitled(doc, ctx, &s, &sp)?;
            if find_series(doc, &s.id).is_none() {
                ctx.note_created(&s.id);
            }
            Ok(vec![ChangeOp::PutSeries {
                series: s,
                index: None,
            }])
        }
        Op::AddStudy {
            id,
            study,
            inputs,
            params,
            show,
        } => {
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
        Op::SetParams { series, params } => {
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
        Op::PutView { view } => {
            let vp = ptr(p, "view");
            check_id(&view.id, &ptr(&vp, "id"))?;
            match find_collection(doc, &view.id) {
                Some(qp_schema::protocol::Collection::Views) | None => {}
                Some(c) => {
                    return Err(E::new(
                        ErrorCode::Duplicate,
                        format!("id `{}` already exists (in {})", view.id, coll_name(c)),
                    )
                    .at(ptr(&vp, "id"))
                    .build())
                }
            }
            let canonical = canon_view(doc, view, &vp, ctx.reads())?;
            if !doc.views.iter().any(|v| v.id == view.id) {
                ctx.note_created(&view.id);
            }
            Ok(vec![ChangeOp::PutView {
                view: canonical,
                index: None,
            }])
        }
        Op::AddLayer { view, layer, index } => {
            let v = need_view(doc, view, &ptr(p, "view"))?;
            if let Some(i) = index {
                if *i as usize > v.layers.len() {
                    return Err(E::new(
                        ErrorCode::Invalid,
                        format!("index {i} is past the end ({} layers)", v.layers.len()),
                    )
                    .at(ptr(p, "index"))
                    .expected([format!("0..={}", v.layers.len())])
                    .build());
                }
            }
            let at = index.map_or(v.layers.len(), |i| i as usize);
            let l = new_layer(doc, v, layer, at, &ptr(p, "layer"), ctx.reads())?;
            Ok(vec![ChangeOp::InsertLayer {
                view: view.clone(),
                index: u32::try_from(at).unwrap_or(u32::MAX),
                layer: l,
            }])
        }
        Op::RemoveLayer { view, index } => {
            let v = need_view(doc, view, &ptr(p, "view"))?;
            if *index as usize >= v.layers.len() {
                return Err(E::new(
                    ErrorCode::Invalid,
                    format!("view `{view}` has {} layers", v.layers.len()),
                )
                .at(ptr(p, "index"))
                .expected([format!("0..={}", v.layers.len().saturating_sub(1))])
                .build());
            }
            let l = &v.layers[*index as usize];
            if let Some(id) = &l.id {
                layer_in_use(v, id, &ptr(p, "index"))?;
            }
            Ok(vec![ChangeOp::DeleteLayer {
                layer: l.id.clone(),
                view: view.clone(),
                index: *index,
            }])
        }
        Op::SetViewRange {
            view,
            from_bar,
            to_bar,
        } => {
            let v = need_view(doc, view, &ptr(p, "view"))?;
            let bars = view_bar_series_any(doc, v)
                .and_then(|s| qp_plane::plane::bars_for(doc, ctx.data.as_ref(), &s.id))
                .map(|(_, b)| b)
                .ok_or_else(|| {
                    invalid(format!("view `{view}` draws no bar series"), ptr(p, "view"))
                })?;
            let last = bars.len() as u64 - 1;
            if *to_bar > last {
                return Err(E::new(
                    ErrorCode::Invalid,
                    format!("to_bar {to_bar} is past the last bar {last}"),
                )
                .at(ptr(p, "to_bar"))
                .expected([format!("0..={last}")])
                .build());
            }
            if from_bar > to_bar {
                return Err(E::new(
                    ErrorCode::Invalid,
                    format!("from_bar {from_bar} is after to_bar {to_bar}"),
                )
                .at(ptr(p, "from_bar"))
                .expected([format!("0..={to_bar}")])
                .build());
            }
            #[allow(clippy::cast_precision_loss)]
            let domain = [
                bars[*from_bar as usize].t as f64,
                bars[*to_bar as usize].t as f64,
            ];
            Ok(vec![ChangeOp::SetXDomain {
                view: view.clone(),
                domain: Some(domain),
            }])
        }
        Op::AddFact {
            id,
            kind,
            series,
            bar,
            text,
            evidence,
        } => {
            need_series(doc, series, &ptr(p, "series"))?;
            // `add_fact` reads the fact's bar time: the speaker needs the data
            // entitlement of a bound series (§18.3).
            if let Some(r) = &ctx.data {
                if let Some(b) = r.plane.gate(doc, Some(ctx.speaker), series) {
                    return Err(b.error(series, &ptr(p, "series")));
                }
            }
            let (bs, bars) =
                qp_plane::plane::bars_for(doc, ctx.data.as_ref(), series).ok_or_else(|| {
                    invalid(format!("series `{series}` has no bars"), ptr(p, "series"))
                })?;
            let Some(b) = bars.get(usize::try_from(*bar).unwrap_or(usize::MAX)) else {
                return Err(E::new(
                    ErrorCode::Invalid,
                    format!("bar {bar} is past the last bar {}", bars.len() - 1),
                )
                .at(ptr(p, "bar"))
                .expected([format!("0..={}", bars.len() - 1)])
                .build());
            };
            if evidence.values().any(|v| !v.is_finite()) {
                return Err(invalid(
                    "evidence values must be finite",
                    ptr(p, "evidence"),
                ));
            }
            let t = b.t;
            check_evidence_values(doc, ctx, series, *bar, evidence, p)?;
            // Frozen evidence (§18.5): bars 0..=bar of the bar series, by digest.
            let evidence_ref = Some(qp_plane::plane::evidence_ref(
                bs,
                bars,
                usize::try_from(*bar).unwrap_or(0),
            ));
            let fid = ctx.new_id(doc, "fact", id.as_ref(), &ptr(p, "id"))?;
            let fact = Fact {
                id: fid,
                kind: kind.clone(),
                series: series.clone(),
                bar: *bar,
                t,
                text: text.clone(),
                evidence: evidence.clone(),
                evidence_ref,
                meaning: None,
                provenance: ctx.prov(vec![series.clone()]),
            };
            Ok(vec![ChangeOp::PutFact { fact, index: None }])
        }
        Op::AddFinding {
            id,
            claim,
            facts,
            confidence,
            invalidation,
            capsule,
        } => {
            if facts.is_empty() {
                return Err(invalid(
                    "a finding rests on at least one fact",
                    ptr(p, "facts"),
                ));
            }
            for (j, f) in facts.iter().enumerate() {
                if !doc.facts.iter().any(|x| &x.id == f) {
                    return Err(
                        E::new(ErrorCode::NotFound, format!("fact `{f}` does not exist"))
                            .at(ptr(&ptr(p, "facts"), j))
                            .dym(f, doc.facts.iter().map(|x| x.id.as_str()))
                            .build(),
                    );
                }
            }
            check_confidence(*confidence, &ptr(p, "confidence"))?;
            check_condition(doc, invalidation, &ptr(p, "invalidation"))?;
            cond_entitled(doc, ctx, invalidation, &ptr(p, "invalidation"))?;
            let fid = ctx.new_id(doc, "fnd", id.as_ref(), &ptr(p, "id"))?;
            let finding = Finding {
                id: fid.clone(),
                claim: claim.clone(),
                facts: facts.clone(),
                confidence: *confidence,
                invalidation: invalidation.clone(),
                provenance: ctx.prov(facts.clone()),
            };
            let mut out = vec![ChangeOp::PutFinding {
                finding,
                index: None,
            }];
            if let Some(headline) = capsule {
                let cid = ctx.new_id(doc, "cap", None, &ptr(p, "capsule"))?;
                out.push(ChangeOp::PutCapsule {
                    index: None,
                    capsule: Capsule {
                        id: cid,
                        finding: fid.clone(),
                        state: CapsuleState::Live,
                        revision: 0,
                        headline: headline.clone(),
                        glyphs: Vec::new(),
                        view: None,
                        story: None,
                        depth: Depth::Capsule,
                        expires_at: None,
                        provenance: ctx.prov(vec![fid]),
                    },
                });
            }
            Ok(out)
        }
        Op::UpdateFinding {
            finding,
            claim,
            confidence,
            invalidation,
        } => {
            let f = need_finding(doc, finding, &ptr(p, "finding"))?;
            if claim.is_none() && confidence.is_none() && invalidation.is_none() {
                return Err(E::new(ErrorCode::Invalid, "nothing to update")
                    .at(p)
                    .expected(["claim", "confidence", "invalidation"])
                    .build());
            }
            let mut nf = f.clone();
            if let Some(c) = claim {
                nf.claim.clone_from(c);
            }
            if let Some(c) = confidence {
                check_confidence(*c, &ptr(p, "confidence"))?;
                nf.confidence = *c;
            }
            if let Some(c) = invalidation {
                check_condition(doc, c, &ptr(p, "invalidation"))?;
                cond_entitled(doc, ctx, c, &ptr(p, "invalidation"))?;
                nf.invalidation = c.clone();
            }
            Ok(vec![ChangeOp::PutFinding {
                finding: nf,
                index: None,
            }])
        }
        Op::ApproveFinding { finding, note } | Op::RejectFinding { finding, note } => {
            need_finding(doc, finding, &ptr(p, "finding"))?;
            let state = if matches!(op, Op::ApproveFinding { .. }) {
                CapsuleState::Approved
            } else {
                CapsuleState::Rejected
            };
            let mut out: Vec<ChangeOp> = doc
                .capsules
                .iter()
                .filter(|c| &c.finding == finding)
                .map(|c| ChangeOp::SetCapsuleState {
                    capsule: c.id.clone(),
                    state,
                })
                .collect();
            out.push(ChangeOp::PushLedger {
                entry: LedgerEntry {
                    finding: finding.clone(),
                    state: Some(state),
                    outcome: Outcome::Pending,
                    recorded_at: ctx.at,
                    note: note.clone(),
                    capsule: None,
                    key: None,
                    stage: None,
                    bar: None,
                },
            });
            Ok(out)
        }
        Op::AddIntent { id, text, trigger } => {
            if let Some(t) = trigger {
                check_condition(doc, t, &ptr(p, "trigger"))?;
                cond_entitled(doc, ctx, t, &ptr(p, "trigger"))?;
            }
            let iid = ctx.new_id(doc, "intent", id.as_ref(), &ptr(p, "id"))?;
            Ok(vec![ChangeOp::PutIntent {
                index: None,
                intent: Intent {
                    id: iid,
                    text: text.clone(),
                    trigger: trigger.clone(),
                    active: true,
                    provenance: ctx.prov(trigger.as_ref().map_or_else(Vec::new, |t| {
                        condition_series(t)
                            .into_iter()
                            .map(|(_, s)| s.to_owned())
                            .collect()
                    })),
                    watch: None,
                    state: None,
                    matches: 0,
                },
            }])
        }
        Op::SetIntentActive { intent, active } => {
            // (R5B, §25.5) Another person's watch reads as a missing intent; suggestions
            // come from the intents the speaker may see.
            let visible = |x: &&Intent| crate::watch::intent_visible(x, ctx);
            let i = doc
                .intents
                .iter()
                .filter(visible)
                .find(|x| &x.id == intent)
                .ok_or_else(|| {
                    E::new(
                        ErrorCode::NotFound,
                        format!("intent `{intent}` does not exist"),
                    )
                    .at(ptr(p, "intent"))
                    .dym(
                        intent,
                        doc.intents.iter().filter(visible).map(|x| x.id.as_str()),
                    )
                    .build()
                })?;
            if i.watch.is_some() {
                return Err(E::new(
                    ErrorCode::Invalid,
                    format!("`{intent}` is a watch: it ends with `remove_watch`"),
                )
                .at(ptr(p, "intent"))
                .hint("Send remove_watch with this intent id to end a watch.")
                .build());
            }
            let mut ni = i.clone();
            ni.active = *active;
            Ok(vec![ChangeOp::PutIntent {
                intent: ni,
                index: None,
            }])
        }
        Op::AddWatch { .. } | Op::RemoveWatch { .. } => crate::watch::expand_watch(doc, op, p, ctx),
        Op::AddComment {
            id,
            about,
            lang,
            text,
            cites,
            external,
        } => {
            let ap = ptr(p, "about");
            let about = match (&about.capsule, &about.instrument) {
                (Some(c), None) => {
                    let cap = doc
                        .capsules
                        .iter()
                        .find(|x| &x.id == c || &x.finding == c)
                        .ok_or_else(|| {
                            E::new(ErrorCode::NotFound, format!("no capsule `{c}`"))
                                .at(ptr(&ap, "capsule"))
                                .dym(c, doc.capsules.iter().map(|x| x.id.as_str()))
                                .build()
                        })?;
                    qp_schema::protocol::CommentAbout {
                        capsule: Some(cap.id.clone()),
                        instrument: None,
                    }
                }
                (None, Some(i)) => {
                    let known = doc.series.iter().any(|s| s.symbol() == Some(i.as_str()));
                    if !known {
                        return Err(E::new(
                            ErrorCode::NotFound,
                            format!("no series of instrument `{i}`"),
                        )
                        .at(ptr(&ap, "instrument"))
                        .dym(i, doc.series.iter().filter_map(|s| s.symbol()))
                        .build());
                    }
                    about.clone()
                }
                _ => {
                    return Err(E::new(
                        ErrorCode::Invalid,
                        "`about` names exactly one of `capsule` and `instrument`",
                    )
                    .at(ap)
                    .expected(["capsule", "instrument"])
                    .build())
                }
            };
            // Cites are the validator's (`unknown_id`, the same for a hidden id).
            let cid = ctx.new_id(doc, "cmt", id.as_ref(), &ptr(p, "id"))?;
            Ok(vec![ChangeOp::PutComment {
                index: None,
                comment: qp_schema::protocol::Comment {
                    id: cid,
                    about,
                    lang: *lang,
                    text: text.clone(),
                    cites: cites.clone(),
                    external: external.clone(),
                    strictness: ctx.plane.comment_strictness,
                    provenance: ctx.prov(cites.clone()),
                    on_behalf: ctx.plane.delegates.get(ctx.speaker).cloned(),
                },
            }])
        }
        Op::AddNarrative { id, about, steps } => {
            need_item(doc, about, &ptr(p, "about"))?;
            if steps.is_empty() {
                return Err(invalid(
                    "a narrative needs at least one step",
                    ptr(p, "steps"),
                ));
            }
            let sp = ptr(p, "steps");
            for (j, s) in steps.iter().enumerate() {
                let jp = ptr(&sp, j);
                if j > 0 && s.at_ms <= steps[j - 1].at_ms {
                    return Err(E::new(
                        ErrorCode::Invalid,
                        format!(
                            "steps must be in ascending at_ms order: step {j} at {} ms is not after step {} at {} ms",
                            s.at_ms,
                            j - 1,
                            steps[j - 1].at_ms
                        ),
                    )
                    .at(ptr(&jp, "at_ms"))
                    .expected([format!("> {}", steps[j - 1].at_ms)])
                    .build());
                }
                check_target(doc, &s.target, &ptr(&jp, "target"))?;
            }
            let nid = ctx.new_id(doc, "narr", id.as_ref(), &ptr(p, "id"))?;
            Ok(vec![ChangeOp::PutNarrative {
                index: None,
                narrative: Narrative {
                    id: nid,
                    about: about.clone(),
                    steps: steps.clone(),
                    provenance: ctx.prov(vec![about.clone()]),
                },
            }])
        }
        Op::ShowFinding { finding, view } => {
            let f = need_finding(doc, finding, &ptr(p, "finding"))?;
            let vid = match view {
                Some(v) => {
                    check_id(v, &ptr(p, "view"))?;
                    match find_collection(doc, v) {
                        None => {
                            ctx.note_created(v);
                        }
                        Some(qp_schema::protocol::Collection::Views) => {}
                        Some(c) => {
                            return Err(E::new(
                                ErrorCode::Duplicate,
                                format!("id `{v}` already exists (in {})", coll_name(c)),
                            )
                            .at(ptr(p, "view"))
                            .build())
                        }
                    }
                    v.clone()
                }
                None => ctx.new_id(doc, "view", None, &ptr(p, "view"))?,
            };
            // The view reads the bar times of the finding's facts' series: on a bound
            // series the speaker needs its data entitlement (§18.3), as for `add_fact`.
            if let Some(r) = &ctx.data {
                for fid in &f.facts {
                    if let Some(x) = doc.facts.iter().find(|x| &x.id == fid) {
                        if let Some(b) = r.plane.gate(doc, Some(ctx.speaker), &x.series) {
                            return Err(b.error(&x.series, &ptr(p, "finding")));
                        }
                    }
                }
            }
            let built = crate::present::presenter_view(doc, ctx.data, f, &vid)
                .map_err(|m| invalid(m, ptr(p, "finding")))?;
            let built = canon_view(doc, &built, &ptr(p, "finding"), ctx.reads())?;
            let mut out = vec![ChangeOp::PutView {
                view: built,
                index: None,
            }];
            if let Some(c) = doc.capsules.iter().find(|c| &c.finding == finding) {
                if c.view.as_deref() != Some(vid.as_str()) {
                    let mut nc = c.clone();
                    nc.view = Some(vid);
                    out.push(ChangeOp::PutCapsule {
                        capsule: nc,
                        index: None,
                    });
                }
            }
            Ok(out)
        }
        Op::Remove { id } => {
            // (R5B, §25.5) A watch: another person's reads as missing; one's own ends
            // with `remove_watch`.
            if let Some(i) = doc
                .intents
                .iter()
                .find(|i| &i.id == id && i.watch.is_some())
            {
                if !crate::watch::intent_visible(i, ctx) {
                    return Err(
                        E::new(ErrorCode::NotFound, format!("item `{id}` does not exist"))
                            .at(ptr(p, "id"))
                            .dym(id, crate::watch::visible_ids(doc, ctx))
                            .build(),
                    );
                }
                return Err(E::new(
                    ErrorCode::Invalid,
                    format!("`{id}` is a watch: it ends with `remove_watch`"),
                )
                .at(ptr(p, "id"))
                .hint("Send remove_watch with this intent id to end a watch.")
                .build());
            }
            if find_collection(doc, id).is_none() {
                return Err(
                    E::new(ErrorCode::NotFound, format!("item `{id}` does not exist"))
                        .at(ptr(p, "id"))
                        .dym(id, crate::watch::visible_ids(doc, ctx))
                        .build(),
                );
            }
            expand_remove(doc, id, p)
        }
        Op::AppendBars { series, bars } => {
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
        Op::UpdateLastBar { series, bar } => {
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
        Op::Undo { seq } => expand_undo(doc, *seq, p, ctx),
        Op::SetAudience {
            principal,
            audience,
        } => {
            let who = audience_target(principal.as_deref(), p, ctx)?;
            for (sym, q) in &audience.positions {
                if !q.is_finite() {
                    return Err(invalid(
                        format!("position of `{sym}` must be a finite number"),
                        ptr(&ptr(&ptr(p, "audience"), "positions"), sym),
                    ));
                }
            }
            if let Some(i) = audience.watchlist.iter().position(String::is_empty) {
                return Err(invalid(
                    "a watch-list symbol must not be empty",
                    ptr(&ptr(&ptr(p, "audience"), "watchlist"), i),
                ));
            }
            Ok(vec![ChangeOp::PutAudience {
                principal: who,
                audience: Some(audience.clone()),
            }])
        }
        Op::SetSensitivity {
            principal,
            sensitivity,
        } => {
            let who = audience_target(principal.as_deref(), p, ctx)?;
            let mut a = doc
                .audiences
                .get(&who)
                .cloned()
                .unwrap_or(qp_schema::capsule::Audience {
                    group: qp_schema::capsule::UserGroup::Habituated,
                    sensitivity: None,
                    watchlist: Vec::new(),
                    positions: BTreeMap::new(),
                });
            a.sensitivity = *sensitivity;
            Ok(vec![ChangeOp::PutAudience {
                principal: who,
                audience: Some(a),
            }])
        }
        Op::InsertLayer { .. }
        | Op::DeleteLayer { .. }
        | Op::SplitLayer { .. }
        | Op::MoveLayer { .. }
        | Op::SetEncoding { .. }
        | Op::InsertPanel { .. }
        | Op::DeletePanel { .. }
        | Op::MovePanel { .. }
        | Op::SetPanel { .. }
        | Op::SetScale { .. }
        | Op::SetEmphasis { .. } => expand_grammar(doc, op, p, ctx),
        Op::RejectProposal { proposal, note } => {
            let pr = need_proposal(doc, proposal, &ptr(p, "proposal"))?;
            let mut np = pr.clone();
            np.state = ProposalState::Rejected;
            np.decided_by = Some(ctx.author.clone());
            np.note.clone_from(note);
            np.decided_seq = Some(ctx.seq);
            Ok(vec![ChangeOp::PutProposal {
                proposal: np,
                index: None,
            }])
        }
        Op::AcceptProposal { .. } => Err(invalid(
            "accept_proposal must be the only op of its command",
            p,
        )),
    }
}
