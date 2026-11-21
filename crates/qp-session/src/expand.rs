//! Expansion of protocol operations ([`Op`]) into replayable changes ([`ChangeOp`]),
//! with validation (`docs/PROTOCOL.md` §5.1). Each op is expanded against a scratch copy
//! of the document that already contains the effects of the earlier ops of the same
//! command; the caller discards the copy if any op fails (all or nothing).

use std::collections::{BTreeMap, BTreeSet};

use qp_schema::grammar::{ColorSpec, Operand, Origin, Panel, Position};
use qp_schema::protocol::{
    ChangeOp, ErrorCode, LogEntry, Narrative, Op, ProposalState, Seq, Show, Target,
};
use qp_schema::{
    Author, Capsule, CapsuleState, Condition, Depth, Document, Fact, Finding, Intent, Layer,
    LedgerEntry, Mark, Outcome, Provenance, Scale, ScaleKind, Series, SeriesData, SeriesSource,
    View,
};

use crate::apply::{all_ids, apply_all, find_collection, first_overlap, touched_keys};
use crate::canon::{canon_view, limit_panels};
use crate::error::{invalid, num, ptr, PResult, E};
use crate::gops::{expand_grammar, layer_in_use, new_layer};
use crate::values::{bar_series, condition_series, find_series, study_outputs};

/// Who writes, when, and how ids are numbered.
pub(crate) struct Ctx<'a> {
    pub author: Author,
    pub approved_by: Option<Author>,
    pub at: i64,
    /// Log position of the entry being written.
    pub seq: Seq,
    /// `seq` used in assigned ids (the proposal's seq when accepting a proposal).
    pub id_seq: Seq,
    pub n: u32,
    pub created: Vec<String>,
    /// Principal id of the speaker (for `undo`).
    pub speaker: &'a str,
    pub is_owner: bool,
    /// Whether the speaker may decide (needed to undo decisions).
    pub can_decide: bool,
    pub log: &'a [LogEntry],
    /// Set when the command is an undo.
    pub undoes: Option<Seq>,
    /// The data plane, when the document has bound series (§18): bound bars for
    /// `add_fact` / `set_view_range`, the speaker's entitlement for `add_fact`.
    pub data: Option<crate::plane::Reader<'a>>,
    /// The data plane, always: the speaker's data entitlement for conditions on bound
    /// series (§18.3) and the session's `max_bind_bars` (§18.7).
    pub plane: &'a crate::plane::DataPlane,
}

impl<'a> Ctx<'a> {
    /// Bound data for origins of constants (§18.3), read as the speaker.
    pub(crate) fn reads(&self) -> Option<crate::canon::Reads<'a>> {
        self.data.map(|reader| crate::canon::Reads {
            reader,
            writer: self.speaker,
        })
    }

    fn prov(&self, based_on: Vec<String>) -> Provenance {
        Provenance {
            author: self.author.clone(),
            at: self.at,
            based_on,
            approved_by: self.approved_by.clone(),
        }
    }

    /// Takes the given id (checked) or assigns `<kind>-<seq>-<n>`; records it as created.
    fn new_id(
        &mut self,
        doc: &Document,
        kind: &str,
        given: Option<&String>,
        path: &str,
    ) -> PResult<String> {
        let id = match given {
            Some(g) => {
                check_id(g, path)?;
                g.clone()
            }
            None => format!("{kind}-{}-{}", self.id_seq, self.n),
        };
        if let Some(c) = find_collection(doc, &id) {
            return Err(E::new(
                ErrorCode::Duplicate,
                format!("id `{id}` already exists (in {})", coll_name(c)),
            )
            .at(path)
            .hint("Choose another id or omit it to have one assigned.")
            .into());
        }
        self.n += 1;
        self.created.push(id.clone());
        Ok(id)
    }

    fn note_created(&mut self, id: &str) {
        self.n += 1;
        self.created.push(id.to_owned());
    }
}

pub(crate) fn coll_name(c: qp_schema::protocol::Collection) -> &'static str {
    use qp_schema::protocol::Collection as C;
    match c {
        C::Series => "series",
        C::Views => "views",
        C::Facts => "facts",
        C::Findings => "findings",
        C::Capsules => "capsules",
        C::Intents => "intents",
        C::Narratives => "narratives",
        C::Comments => "comments",
        C::Proposals => "proposals",
    }
}

/// Id format `^[A-Za-z0-9][A-Za-z0-9_.:-]{0,63}$`.
pub(crate) fn valid_id(id: &str) -> bool {
    let b = id.as_bytes();
    !b.is_empty()
        && b.len() <= 64
        && b[0].is_ascii_alphanumeric()
        && b[1..]
            .iter()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'.' | b':' | b'-'))
}

fn check_id(id: &str, path: &str) -> PResult<()> {
    if valid_id(id) {
        Ok(())
    } else {
        Err(E::new(ErrorCode::Invalid, format!("invalid id `{id}`"))
            .at(path)
            .expected(["^[A-Za-z0-9][A-Za-z0-9_.:-]{0,63}$"])
            .build())
    }
}

fn series_ids(doc: &Document) -> Vec<&str> {
    doc.series.iter().map(|s| s.id.as_str()).collect()
}

pub(crate) fn need_series<'a>(doc: &'a Document, id: &str, path: &str) -> PResult<&'a Series> {
    find_series(doc, id).ok_or_else(|| {
        E::new(ErrorCode::NotFound, format!("series `{id}` does not exist"))
            .at(path)
            .dym(id, series_ids(doc))
            .hint("Use an existing series id; list them with {\"kind\":\"list\",\"collection\":\"series\"}.")
            .build()
    })
}

pub(crate) fn need_view<'a>(doc: &'a Document, id: &str, path: &str) -> PResult<&'a View> {
    doc.views.iter().find(|v| v.id == id).ok_or_else(|| {
        E::new(ErrorCode::NotFound, format!("view `{id}` does not exist"))
            .at(path)
            .dym(id, doc.views.iter().map(|v| v.id.as_str()))
            .hint("Use an existing view id; list them with {\"kind\":\"list\",\"collection\":\"views\"}.")
            .build()
    })
}

pub(crate) fn need_finding<'a>(doc: &'a Document, id: &str, path: &str) -> PResult<&'a Finding> {
    doc.findings.iter().find(|v| v.id == id).ok_or_else(|| {
        E::new(
            ErrorCode::NotFound,
            format!("finding `{id}` does not exist"),
        )
        .at(path)
        .dym(id, doc.findings.iter().map(|v| v.id.as_str()))
        .hint("List findings with {\"kind\":\"list\",\"collection\":\"findings\"}.")
        .build()
    })
}

pub(crate) fn need_item(doc: &Document, id: &str, path: &str) -> PResult<()> {
    if find_collection(doc, id).is_some() {
        Ok(())
    } else {
        Err(
            E::new(ErrorCode::NotFound, format!("item `{id}` does not exist"))
                .at(path)
                .dym(id, all_ids(doc))
                .build(),
        )
    }
}

/// Validates the series a condition names.
pub(crate) fn check_condition(doc: &Document, c: &Condition, path: &str) -> PResult<()> {
    for (field, id) in condition_series(c) {
        need_series(doc, id, &ptr(path, field))?;
    }
    if let Condition::Threshold { value, .. } = c {
        if !value.is_finite() {
            return Err(invalid(
                "threshold value must be finite",
                ptr(path, "value"),
            ));
        }
    }
    Ok(())
}

/// The first bar series drawn in a view, by kind (a bound series even when its bars are
/// not available).
pub(crate) fn view_bar_series_any<'a>(doc: &'a Document, view: &View) -> Option<&'a Series> {
    view.layers
        .iter()
        .filter_map(|l| l.series.as_deref())
        .find_map(|s| crate::plane::bar_root(doc, s))
}

/// The first bar series drawn in a view.
pub(crate) fn view_bar_series<'a>(doc: &'a Document, view: &View) -> Option<&'a Series> {
    view.layers
        .iter()
        .filter_map(|l| l.series.as_deref())
        .find_map(|s| bar_series(doc, s))
}

/// Validates a study and returns its full parameters (defaults merged).
fn check_study(
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
    if !crate::plane::is_bar_series(input) {
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

fn check_target(doc: &Document, t: &Target, path: &str) -> PResult<()> {
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
fn check_points(points: &[qp_schema::Point]) -> Result<(), (usize, String)> {
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

/// Log positions that are undone (by an undo entry that is itself not undone).
pub(crate) fn undone_set(log: &[LogEntry]) -> BTreeSet<Seq> {
    let mut undone = BTreeSet::new();
    for e in log.iter().rev() {
        if let Some(x) = e.undoes {
            if !undone.contains(&e.seq) {
                undone.insert(x);
            }
        }
    }
    undone
}

/// Expands one op into changes. `doc` is the scratch document *before* this op.
#[allow(clippy::too_many_lines)]
pub(crate) fn expand_op(
    doc: &Document,
    op: &Op,
    p: &str,
    ctx: &mut Ctx<'_>,
) -> PResult<Vec<ChangeOp>> {
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
                .and_then(|s| crate::plane::bars_for(doc, ctx.data.as_ref(), &s.id))
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
                crate::plane::bars_for(doc, ctx.data.as_ref(), series).ok_or_else(|| {
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
            let evidence_ref = Some(crate::plane::evidence_ref(
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
                },
            }])
        }
        Op::SetIntentActive { intent, active } => {
            let i = doc
                .intents
                .iter()
                .find(|x| &x.id == intent)
                .ok_or_else(|| {
                    E::new(
                        ErrorCode::NotFound,
                        format!("intent `{intent}` does not exist"),
                    )
                    .at(ptr(p, "intent"))
                    .dym(intent, doc.intents.iter().map(|x| x.id.as_str()))
                    .build()
                })?;
            let mut ni = i.clone();
            ni.active = *active;
            Ok(vec![ChangeOp::PutIntent {
                intent: ni,
                index: None,
            }])
        }
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
            for (j, c) in cites.iter().enumerate() {
                need_item(doc, c, &ptr(&ptr(p, "cites"), j))?;
            }
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
        Op::Remove { id } => expand_remove(doc, id, p),
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

pub(crate) fn need_proposal<'a>(
    doc: &'a Document,
    id: &str,
    path: &str,
) -> PResult<&'a qp_schema::protocol::Proposal> {
    let pr = doc.proposals.iter().find(|x| x.id == id).ok_or_else(|| {
        E::new(ErrorCode::NotFound, format!("proposal `{id}` does not exist"))
            .at(path)
            .dym(id, doc.proposals.iter().map(|x| x.id.as_str()))
            .hint("List pending proposals with {\"kind\":\"list\",\"collection\":\"proposals\",\"state\":\"pending\"}.")
            .build()
    })?;
    if pr.state != ProposalState::Pending {
        return Err(E::new(
            ErrorCode::NotPending,
            format!("proposal `{id}` is {}", proposal_state_name(pr.state)),
        )
        .at(path)
        .build());
    }
    Ok(pr)
}

pub(crate) fn proposal_state_name(s: ProposalState) -> &'static str {
    match s {
        ProposalState::Pending => "pending",
        ProposalState::Accepted => "accepted",
        ProposalState::Rejected => "rejected",
        ProposalState::Expired => "expired",
    }
}

fn check_confidence(c: f64, path: &str) -> PResult<()> {
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
fn check_evidence_values(
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
    let bound = crate::plane::bar_root(doc, series).is_some_and(|r| r.bars().is_none());
    let src = if bound {
        match &ctx.data {
            Some(r) if find_series(r.rd, series).is_some_and(|x| x.source == s.source) => r.rd,
            _ => return Ok(()),
        }
    } else {
        doc
    };
    let Ok(v) = crate::values::values(src, &mut crate::values::Cache::default(), series) else {
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

/// A condition evaluated on bound data (a finding's invalidation, an intent's trigger)
/// tells its outcome to whoever wrote it: the writer needs the data entitlement of every
/// bound feed the condition reads, through study inputs (§18.3); otherwise
/// `no_entitlement` at the series path.
pub(crate) fn cond_entitled(
    doc: &Document,
    ctx: &Ctx<'_>,
    c: &Condition,
    path: &str,
) -> PResult<()> {
    condition_entitled(ctx.plane, doc, ctx.speaker, c, path)
}

/// [`cond_entitled`] for `principal` (also the `condition` subscription, §18.3).
pub(crate) fn condition_entitled(
    plane: &crate::plane::DataPlane,
    doc: &Document,
    principal: &str,
    c: &Condition,
    path: &str,
) -> PResult<()> {
    for (field, s) in condition_series(c) {
        if let Some(b) = plane.unentitled(doc, principal, s) {
            let mut e = b.error(s, &ptr(path, field));
            e.hint = Some(
                "A condition on bound data would tell you its values through its outcome; the data licence of this feed does not cover you. Ask the host for an entitlement."
                    .to_owned(),
            );
            return Err(e);
        }
    }
    Ok(())
}

/// True if a finding's invalidation or an intent's trigger in `doc` rests on series
/// `id` (itself or through study inputs).
fn read_by_conditions(doc: &Document, id: &str) -> bool {
    let reads = |c: &Condition| crate::live::closure_of(doc, c).iter().any(|s| s == id);
    doc.findings.iter().any(|f| reads(&f.invalidation))
        || doc
            .intents
            .iter()
            .any(|i| i.trigger.as_ref().is_some_and(reads))
}

/// Rewriting a series that a condition rests on (a finding's invalidation, an intent's
/// trigger) re-points that condition at other data: the writer needs the data
/// entitlement of every bound feed the new version rests on (its binding, or through
/// its study inputs), else `no_entitlement` at the series' source (§18.3). Otherwise an
/// unentitled writer could aim a condition someone else wrote, or one they wrote on an
/// inline series, at a feed they may not read.
fn rebind_entitled(doc: &Document, ctx: &Ctx<'_>, s: &Series, sp: &str) -> PResult<()> {
    let src = ptr(sp, "source");
    let blocked = match &s.source {
        SeriesSource::Bind { .. } => s
            .binding()
            .map(|b| b.feed_key())
            .filter(|k| !ctx.plane.entitled(ctx.speaker, k))
            .map(|k| {
                (
                    s.id.clone(),
                    crate::plane::Blocked::unentitled(&s.id, ctx.speaker, &k),
                    src.clone(),
                )
            }),
        SeriesSource::Study { inputs, .. } => inputs.iter().enumerate().find_map(|(i, inp)| {
            ctx.plane
                .unentitled(doc, ctx.speaker, inp)
                .map(|b| (inp.clone(), b, ptr(&ptr(&src, "inputs"), i)))
        }),
        _ => None,
    };
    let Some((about, b, path)) = blocked else {
        return Ok(());
    };
    if !read_by_conditions(doc, &s.id) {
        return Ok(());
    }
    let mut e = b.error(&about, &path);
    e.hint = Some(
        "A condition rests on this series: re-pointing it at a feed would tell you that feed's values through the condition's outcome, and its data licence does not cover you. Ask the host for an entitlement."
            .to_owned(),
    );
    Err(e)
}

/// `append_bars` / `update_last_bar` are for inline series (§18.2).
fn bound_no_bar_writes(s: &Series, path: &str) -> PResult<()> {
    if matches!(s.source, SeriesSource::Bind { .. }) {
        return Err(E::new(
            ErrorCode::Invalid,
            format!(
                "series `{}` is bound: the data of a bound series comes from the data plane",
                s.id
            ),
        )
        .at(path)
        .hint("Bar writes are for inline series; the host delivers bound data as data events.")
        .build());
    }
    Ok(())
}

/// Most bound series a document may hold (`max_ops × 4`, §18.7).
pub(crate) const MAX_BOUND_SERIES: usize = 256;

/// Canonical interval spelling (§18.2): `Nm` only when N is not a multiple of 60, no
/// leading zeros. Returns the canonical form when `s` is a valid but non-canonical one.
fn canonical_interval(s: &str) -> Option<String> {
    let split = s.find(|c: char| !c.is_ascii_digit())?;
    let (num, unit) = s.split_at(split);
    let n: u32 = num.parse().ok()?;
    let canon = if unit == "m" && n.is_multiple_of(60) {
        format!("{}h", n / 60)
    } else {
        format!("{n}{unit}")
    };
    (canon != s).then_some(canon)
}

/// Validates a bound series of `put_series` (§18.2, §18.7).
fn check_bind(doc: &Document, series: &Series, sp: &str, max_bind_bars: u32) -> PResult<()> {
    let SeriesSource::Bind {
        symbol,
        interval,
        window,
        feed,
        anchor_t,
    } = &series.source
    else {
        return Ok(());
    };
    let src = ptr(sp, "source");
    if anchor_t.is_some() {
        return Err(E::new(
            ErrorCode::Invalid,
            "`anchor_t` is written by the authority, never by clients",
        )
        .at(ptr(&src, "anchor_t"))
        .hint("Omit `anchor_t`; the authority anchors the binding on its first resolution.")
        .build());
    }
    qp_source::validate_id("symbol", symbol).map_err(|m| invalid(m, ptr(&src, "symbol")))?;
    if let Some(f) = feed {
        qp_source::validate_id("feed", f).map_err(|m| invalid(m, ptr(&src, "feed")))?;
    }
    if let Err(m) = qp_source::validate_interval(interval) {
        return Err(E::new(ErrorCode::Invalid, m)
            .at(ptr(&src, "interval"))
            .expected([
                "1m…1440m (not a multiple of 60)",
                "1h…24h",
                "1d",
                "1w",
                "1M",
            ])
            .build());
    }
    if let Some(c) = canonical_interval(interval) {
        return Err(E::new(
            ErrorCode::Invalid,
            format!("interval `{interval}` is not written canonically; the same feed is `{c}`"),
        )
        .at(ptr(&src, "interval"))
        .suggest([c])
        .build());
    }
    window
        .validate(max_bind_bars)
        .map_err(|m| invalid(m, ptr(&src, "window")))?;
    if series.data.is_some() {
        return Err(invalid(
            "a bound series carries no data: its bars come from the data plane",
            ptr(sp, "data"),
        ));
    }
    if let Some(i) = &series.instrument {
        if &i.symbol != symbol {
            return Err(E::new(
                ErrorCode::Invalid,
                format!(
                    "instrument symbol `{}` differs from the binding's `{symbol}`",
                    i.symbol
                ),
            )
            .at(ptr(&ptr(sp, "instrument"), "symbol"))
            .expected([symbol.clone()])
            .build());
        }
    }
    if let Some(iv) = &series.interval {
        if iv != interval {
            return Err(E::new(
                ErrorCode::Invalid,
                format!("interval `{iv}` differs from the binding's `{interval}`"),
            )
            .at(ptr(sp, "interval"))
            .expected([interval.clone()])
            .build());
        }
    }
    let bound = doc
        .series
        .iter()
        .filter(|s| s.id != series.id && matches!(s.source, SeriesSource::Bind { .. }))
        .count();
    if bound + 1 > MAX_BOUND_SERIES {
        return Err(E::new(
            ErrorCode::Limit,
            format!("a document holds at most {MAX_BOUND_SERIES} bound series"),
        )
        .at(ptr(&src, "kind"))
        .expected([MAX_BOUND_SERIES.to_string()])
        .build());
    }
    Ok(())
}

fn inline_bars(s: &Series) -> Option<&[qp_schema::Bar]> {
    if matches!(s.source, SeriesSource::Inline {}) {
        s.bars()
    } else {
        None
    }
}

/// Validates `show` and returns the view, the panel id and whether the panel is new
/// (`with`: that series' panel; `panel`: an existing panel id, or the next id to add a
/// panel; neither: a new panel with the next id, at the bottom).
fn check_show<'a>(doc: &'a Document, sh: &Show, p: &str) -> PResult<(&'a View, u32, bool)> {
    let v = need_view(doc, &sh.view, &ptr(p, "view"))?;
    let next = match v.panels.iter().filter_map(|x| x.id).max() {
        None => Some(0),
        Some(m) => m.checked_add(1),
    };
    if let Some(w) = &sh.with {
        need_series(doc, w, &ptr(p, "with"))?;
        let Some(l) = v.layers.iter().find(|l| l.series.as_deref() == Some(w)) else {
            return Err(E::new(
                ErrorCode::Invalid,
                format!("series `{w}` is not drawn in view `{}`", sh.view),
            )
            .at(ptr(p, "with"))
            .expected(
                v.layers
                    .iter()
                    .filter_map(|l| l.series.clone())
                    .collect::<BTreeSet<_>>(),
            )
            .build());
        };
        return Ok((v, l.panel, false));
    }
    let new_panel = |id: Option<u32>, path: &str| -> PResult<(&'a View, u32, bool)> {
        let id = id.ok_or_else(|| invalid("no panel id is left above the largest one", path))?;
        limit_panels(v, 1, path)?;
        Ok((v, id, true))
    };
    if let Some(panel) = sh.panel {
        if v.panel(panel).is_some() {
            return Ok((v, panel, false));
        }
        if Some(panel) == next {
            return new_panel(next, &ptr(p, "panel"));
        }
        let mut expected: Vec<String> = v
            .panels
            .iter()
            .filter_map(|x| x.id)
            .map(|i| i.to_string())
            .collect();
        if let Some(n) = next {
            expected.push(format!("{n} (new panel)"));
        }
        return Err(E::new(
            ErrorCode::Invalid,
            format!(
                "panel {panel} does not exist in view `{}`; the next new panel is {}",
                sh.view,
                next.map_or_else(|| "none".to_owned(), |n| n.to_string())
            ),
        )
        .at(ptr(p, "panel"))
        .expected(expected)
        .build());
    }
    new_panel(next, p)
}

/// Whether a layer refers to series `id` other than through its `series` field (a field
/// of another series in a channel, or a `series` origin).
fn layer_refs_series(l: &Layer, id: &str) -> bool {
    let pos = |p: &Option<Position>| matches!(p, Some(Position::Ref(r)) if r.series.as_deref() == Some(id));
    let e = &l.encoding;
    pos(&e.x)
        || pos(&e.y)
        || pos(&e.y2)
        || matches!(&e.color, Some(ColorSpec::When(w))
            if matches!(&w.when.than, Operand::Ref(r) if r.series.as_deref() == Some(id)))
        || l.origin
            .values()
            .any(|o| matches!(o, Origin::Series { series, .. } if series == id))
}

/// Views whose layers or emphasis depend on item `id` (beyond layers drawing series
/// `id`, which are removed with it).
fn view_deps(doc: &Document, id: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for v in &doc.views {
        let layer_dep = v.layers.iter().any(|l| {
            l.fact.as_deref() == Some(id)
                || l.finding.as_deref() == Some(id)
                || layer_refs_series(l, id)
                || l.origin
                    .values()
                    .any(|o| matches!(o, Origin::Fact { fact, .. } if fact == id))
        });
        let drawn: BTreeSet<&str> = v
            .layers
            .iter()
            .filter(|l| l.series.as_deref() == Some(id))
            .filter_map(|l| l.id.as_deref())
            .collect();
        let emph_dep = v.emphasis.iter().any(|e| {
            e.target.item.as_deref() == Some(id)
                || e.target.series.as_deref() == Some(id)
                || e.target.layer.as_deref().is_some_and(|l| drawn.contains(l))
        });
        if layer_dep || emph_dep {
            out.insert(v.id.clone());
        }
    }
    out
}

/// Checks a target's `layer` against its view (or any view when no view is given).
fn check_target_layer(doc: &Document, t: &Target, path: &str) -> PResult<()> {
    let Some(l) = &t.layer else { return Ok(()) };
    let views: Vec<&View> = match &t.view {
        Some(v) => doc.views.iter().filter(|x| &x.id == v).collect(),
        None => doc.views.iter().collect(),
    };
    if views.iter().any(|v| v.layer(l).is_some()) {
        return Ok(());
    }
    Err(
        E::new(ErrorCode::NotFound, format!("layer `{l}` does not exist"))
            .at(ptr(path, "layer"))
            .dym(
                l,
                views
                    .iter()
                    .flat_map(|v| v.layers.iter().filter_map(|x| x.id.as_deref())),
            )
            .build(),
    )
}

fn expand_remove(doc: &Document, id: &str, p: &str) -> PResult<Vec<ChangeOp>> {
    use qp_schema::protocol::Collection as C;
    let ip = ptr(p, "id");
    let Some(coll) = find_collection(doc, id) else {
        return Err(
            E::new(ErrorCode::NotFound, format!("item `{id}` does not exist"))
                .at(ip)
                .dym(id, all_ids(doc))
                .build(),
        );
    };
    let mut deps: BTreeSet<String> = BTreeSet::new();
    let names = |c: &Condition| condition_series(c).iter().any(|(_, s)| *s == id);
    match coll {
        C::Series => {
            for s in &doc.series {
                if let SeriesSource::Study { inputs, .. } = &s.source {
                    if inputs.iter().any(|i| i == id) {
                        deps.insert(s.id.clone());
                    }
                }
            }
            deps.extend(
                doc.facts
                    .iter()
                    .filter(|f| f.series == id)
                    .map(|f| f.id.clone()),
            );
            deps.extend(
                doc.findings
                    .iter()
                    .filter(|f| names(&f.invalidation))
                    .map(|f| f.id.clone()),
            );
            deps.extend(
                doc.intents
                    .iter()
                    .filter(|i| i.trigger.as_ref().is_some_and(names))
                    .map(|i| i.id.clone()),
            );
        }
        C::Facts => deps.extend(
            doc.findings
                .iter()
                .filter(|f| f.facts.iter().any(|x| x == id))
                .map(|f| f.id.clone()),
        ),
        C::Findings => deps.extend(
            doc.capsules
                .iter()
                .filter(|c| c.finding == id)
                .map(|c| c.id.clone()),
        ),
        C::Views => deps.extend(
            doc.capsules
                .iter()
                .filter(|c| c.view.as_deref() == Some(id))
                .map(|c| c.id.clone()),
        ),
        C::Proposals => {
            return Err(invalid(
                "proposals stay in the document as a trace and cannot be removed",
                ip,
            ))
        }
        C::Capsules | C::Intents | C::Narratives | C::Comments => {}
    }
    if matches!(coll, C::Series | C::Facts | C::Findings) {
        deps.extend(view_deps(doc, id));
    }
    if !deps.is_empty() {
        return Err(E::new(
            ErrorCode::InUse,
            format!(
                "`{id}` is used by {}",
                deps.iter().cloned().collect::<Vec<_>>().join(", ")
            ),
        )
        .at(ip)
        .expected(deps)
        .hint("Remove the dependent items first.")
        .build());
    }
    let mut out = Vec::new();
    if coll == C::Series {
        for v in &doc.views {
            for (i, l) in v.layers.iter().enumerate().rev() {
                if l.series.as_deref() == Some(id) {
                    out.push(ChangeOp::DeleteLayer {
                        layer: l.id.clone(),
                        view: v.id.clone(),
                        index: u32::try_from(i).unwrap_or(u32::MAX),
                    });
                }
            }
        }
    }
    out.push(ChangeOp::Delete { id: id.to_owned() });
    Ok(out)
}

/// Whether an entry carries decision effects: capsule approved/rejected, ledger
/// records, proposal state changes.
fn has_decision(e: &LogEntry) -> bool {
    let decided = |s: &CapsuleState| matches!(s, CapsuleState::Approved | CapsuleState::Rejected);
    e.changes.iter().chain(&e.inverse).any(|c| match c {
        ChangeOp::SetCapsuleState { state, .. } => decided(state),
        ChangeOp::PushLedger { .. } | ChangeOp::PopLedger {} => true,
        ChangeOp::PutProposal { proposal, .. } => proposal.state != ProposalState::Pending,
        _ => false,
    })
}

/// The bound series an engine entry anchors, if it is an anchor stamp (§18.3): author
/// `qp`, every change a `put_series` of a bound series with `anchor_t` whose inverse is
/// the same series without it.
fn anchor_stamps(e: &LogEntry) -> Option<Vec<&str>> {
    if e.author != crate::session::ENGINE_PRINCIPAL || e.changes.is_empty() {
        return None;
    }
    let mut ids = Vec::new();
    for c in &e.changes {
        let ChangeOp::PutSeries { series, .. } = c else {
            return None;
        };
        let SeriesSource::Bind {
            anchor_t: Some(_), ..
        } = &series.source
        else {
            return None;
        };
        let unstamped = e.inverse.iter().any(|i| {
            matches!(i, ChangeOp::PutSeries { series: o, .. } if o.id == series.id
                && matches!(o.source, SeriesSource::Bind { anchor_t: None, .. }))
        });
        if !unstamped {
            return None;
        }
        ids.push(series.id.as_str());
    }
    Some(ids)
}

fn expand_undo(
    doc: &Document,
    seq: Option<Seq>,
    p: &str,
    ctx: &mut Ctx<'_>,
) -> PResult<Vec<ChangeOp>> {
    let undone = undone_set(ctx.log);
    let speaker = ctx.speaker;
    // Only entries the speaker authored are theirs; approving an entry does not make it
    // the approver's to undo (PROTOCOL.md §8.3).
    let mine = |e: &LogEntry| e.author == speaker;
    let target: &LogEntry = match seq {
        Some(s) => {
            let Some(e) = s
                .checked_sub(1)
                .and_then(|i| ctx.log.get(usize::try_from(i).ok()?))
            else {
                return Err(
                    E::new(ErrorCode::NotFound, format!("log entry {s} does not exist"))
                        .at(ptr(p, "seq"))
                        .expected([format!("1..={}", ctx.log.len())])
                        .build(),
                );
            };
            if !mine(e) && !ctx.is_owner {
                return Err(E::new(
                    ErrorCode::Forbidden,
                    format!("entry {s} was written by `{}`; only an owner may undo someone else's entry", e.author),
                )
                .at(ptr(p, "seq"))
                .hint("Undo your own entries, or ask an owner.")
                .build());
            }
            if undone.contains(&s) {
                return Err(E::new(
                    ErrorCode::UndoConflict,
                    format!("entry {s} is already undone"),
                )
                .at(ptr(p, "seq"))
                .hint("To redo, undo the undo entry instead.")
                .build());
            }
            e
        }
        None => ctx
            .log
            .iter()
            .rev()
            .find(|e| mine(e) && !undone.contains(&e.seq) && e.undoes.is_none())
            .ok_or_else(|| {
                E::new(
                    ErrorCode::UndoConflict,
                    format!("`{speaker}` has nothing to undo"),
                )
                .at(p)
                .build()
            })?,
    };
    if !mine(target) && !ctx.is_owner {
        return Err(E::new(
            ErrorCode::Forbidden,
            format!(
                "entry {} was written by `{}`; only an owner may undo someone else's entry",
                target.seq, target.author
            ),
        )
        .at(p)
        .hint("Undo your own entries, or ask an owner.")
        .build());
    }
    if has_decision(target) && !ctx.can_decide {
        return Err(E::new(
            ErrorCode::Forbidden,
            format!(
                "entry {} records a decision (capsule approval/rejection, ledger or proposal state); undoing it needs the decide permission",
                target.seq
            ),
        )
        .at(p)
        .hint("Ask a principal who may decide.")
        .build());
    }
    if let Some(ids) = anchor_stamps(target) {
        return Err(E::new(
            ErrorCode::UndoConflict,
            format!(
                "entry {} is the authority's anchor stamp of bound series {}: an anchor is reset only by a command that rewrites the binding (§18.3), never by undo",
                target.seq,
                ids.iter().map(|i| format!("`{i}`")).collect::<Vec<_>>().join(", ")
            ),
        )
        .at(p)
        .hint("Rewrite the binding with put_series (window, symbol or feed), or undo the entry that put it; the bar numbers of facts and views rest on the anchor.")
        .build());
    }
    // Bindings the target put or rewrote: their anchor stamp (the engine entry right
    // behind) goes with the undo, so it does not block it (§18.3).
    let put: BTreeMap<&str, qp_schema::data::DataBinding> = target
        .changes
        .iter()
        .filter_map(|c| match c {
            ChangeOp::PutSeries { series, .. } => match &series.source {
                SeriesSource::Bind { .. } => Some(series),
                _ => None,
            },
            _ => None,
        })
        .filter_map(|s| Some((s.id.as_str(), crate::plane::binding_of(s)?)))
        .collect();
    let touched = touched_keys(&target.changes);
    let mut conflicts = Vec::new();
    let mut by = String::new();
    for e in &ctx.log[usize::try_from(target.seq)
        .unwrap_or(usize::MAX)
        .min(ctx.log.len())..]
    {
        if undone.contains(&e.seq) || e.undoes.is_some() {
            continue;
        }
        let mut t = touched_keys(&e.changes);
        if !put.is_empty() && anchor_stamps(e).is_some() {
            for c in &e.changes {
                if let ChangeOp::PutSeries { series, .. } = c {
                    let stamps_put = put.get(series.id.as_str()).is_some_and(|b| {
                        crate::plane::binding_of(series)
                            .is_some_and(|s| crate::plane::same_but_anchor(&s, b))
                    });
                    if stamps_put {
                        t.remove(&series.id);
                    }
                }
            }
        }
        if let Some(id) = first_overlap(&t, &touched) {
            if by.is_empty() {
                by = format!("`{id}` changed at seq {} by {}", e.seq, e.author);
            }
            conflicts.push(e.seq);
        }
    }
    if !conflicts.is_empty() {
        return Err(E::new(
            ErrorCode::UndoConflict,
            format!("cannot undo entry {}: {by}", target.seq),
        )
        .at(p)
        .conflicts(conflicts)
        .hint("Undo the later entries first, or change the item with a new command.")
        .build());
    }
    ctx.undoes = Some(target.seq);
    Ok(rebase_positions(doc, &target.inverse))
}

/// Re-anchors the positions of layer and panel changes of an inverse on the current
/// document: later entries that touched *other* layers of the same view may have
/// shifted positions (conflicts are per layer, `docs/PROTOCOL.md` §5.3). A deletion
/// follows its layer id; an insertion keeps its old position, clamped to the list.
fn rebase_positions(doc: &Document, changes: &[ChangeOp]) -> Vec<ChangeOp> {
    let positional = changes.iter().any(|c| {
        matches!(
            c,
            ChangeOp::InsertLayer { .. }
                | ChangeOp::DeleteLayer { .. }
                | ChangeOp::PutLayer { .. }
                | ChangeOp::PutPanel { .. }
        )
    });
    if !positional {
        return changes.to_vec();
    }
    let u = |i: usize| u32::try_from(i).unwrap_or(u32::MAX);
    let mut d = doc.clone();
    let mut out = Vec::with_capacity(changes.len());
    for c in changes {
        let view = |id: &str| d.views.iter().find(|v| v.id == id);
        let c2 = match c {
            ChangeOp::DeleteLayer {
                view: vid,
                layer: Some(id),
                index,
            } => {
                let i = view(vid)
                    .and_then(|v| v.layers.iter().position(|l| l.id.as_deref() == Some(id)))
                    .map_or(*index, u);
                ChangeOp::DeleteLayer {
                    view: vid.clone(),
                    index: i,
                    layer: Some(id.clone()),
                }
            }
            ChangeOp::InsertLayer {
                view: vid,
                index,
                layer,
            } => {
                let n = view(vid).map_or(0, |v| v.layers.len());
                ChangeOp::InsertLayer {
                    view: vid.clone(),
                    index: (*index).min(u(n)),
                    layer: layer.clone(),
                }
            }
            ChangeOp::PutLayer {
                view: vid,
                index,
                layer,
            } => {
                let n = view(vid).map_or(0, |v| {
                    v.layers.len() - usize::from(v.layers.iter().any(|l| l.id == layer.id))
                });
                ChangeOp::PutLayer {
                    view: vid.clone(),
                    index: (*index).min(u(n)),
                    layer: layer.clone(),
                }
            }
            ChangeOp::PutPanel {
                view: vid,
                index,
                panel,
            } => {
                let n = view(vid).map_or(0, |v| {
                    v.panels.len() - usize::from(v.panels.iter().any(|x| x.id == panel.id))
                });
                ChangeOp::PutPanel {
                    view: vid.clone(),
                    index: (*index).min(u(n)),
                    panel: panel.clone(),
                }
            }
            other => other.clone(),
        };
        let _ = crate::apply::apply_change(&mut d, &c2);
        out.push(c2);
    }
    out
}

/// Expands all ops on a scratch copy of `doc`. Returns the new document, the changes,
/// their inverse (reversed) and, per change, the index of the op it came from.
pub(crate) fn expand_ops(
    doc: &Document,
    ops: &[Op],
    base_path: &str,
    ctx: &mut Ctx<'_>,
) -> PResult<Expanded> {
    let mut scratch = doc.clone();
    let mut changes = Vec::new();
    let mut inverse = Vec::new();
    let mut origin = Vec::new();
    for (i, op) in ops.iter().enumerate() {
        let p = ptr(base_path, i);
        let cs = expand_op(&scratch, op, &p, ctx)?;
        let inv = apply_all(&mut scratch, &cs).map_err(|m| {
            let code = if matches!(op, Op::Undo { .. }) {
                ErrorCode::UndoConflict
            } else {
                ErrorCode::Invalid
            };
            E::new(code, format!("cannot apply: {m}"))
                .at(p.clone())
                .build()
        })?;
        origin.extend(std::iter::repeat_n(i, cs.len()));
        changes.extend(cs);
        inverse.push(inv);
    }
    let inverse = inverse.into_iter().rev().flatten().collect();
    Ok(Expanded {
        doc: scratch,
        changes,
        inverse,
        origin,
    })
}

pub(crate) struct Expanded {
    pub doc: Document,
    pub changes: Vec<ChangeOp>,
    pub inverse: Vec<ChangeOp>,
    pub origin: Vec<usize>,
}

/// The principal an audience op writes for (R3, §20): the speaker, or another person
/// for an `owner` only.
fn audience_target(principal: Option<&str>, p: &str, ctx: &Ctx<'_>) -> PResult<String> {
    match principal {
        None => Ok(ctx.speaker.to_owned()),
        Some(w) if w == ctx.speaker => Ok(w.to_owned()),
        Some(w) if ctx.is_owner => {
            check_id(w, &ptr(p, "principal"))?;
            Ok(w.to_owned())
        }
        Some(w) => Err(E::new(
            ErrorCode::Forbidden,
            format!("only an owner sets the capsule preferences of another person (`{w}`)"),
        )
        .at(ptr(p, "principal"))
        .hint("Leave `principal` out to set your own.")
        .build()),
    }
}

/// Short deterministic summary of one op.
pub(crate) fn summarize(op: &Op) -> String {
    fn params(p: &BTreeMap<String, f64>) -> String {
        p.iter()
            .map(|(k, v)| format!("{k}={}", num(*v)))
            .collect::<Vec<_>>()
            .join(",")
    }
    match op {
        Op::PutSeries { series } => format!("put_series {}", series.id),
        Op::AddStudy {
            id,
            study,
            params: p,
            ..
        } => {
            let base = format!("add_study {study}({})", params(p));
            match id {
                Some(i) => format!("{base} -> {i}"),
                None => base,
            }
        }
        Op::SetParams { series, params: p } => format!("set_params {series}({})", params(p)),
        Op::PutView { view } => format!("put_view {}", view.id),
        Op::AddLayer { view, .. } => format!("add_layer {view}"),
        Op::RemoveLayer { view, index } => format!("remove_layer {view}[{index}]"),
        Op::SetViewRange {
            view,
            from_bar,
            to_bar,
        } => format!("set_view_range {view} {from_bar}..={to_bar}"),
        Op::AddFact { series, bar, .. } => format!("add_fact {series}@{bar}"),
        Op::AddFinding { .. } => "add_finding".to_owned(),
        Op::UpdateFinding { finding, .. } => format!("update_finding {finding}"),
        Op::ApproveFinding { finding, .. } => format!("approve_finding {finding}"),
        Op::RejectFinding { finding, .. } => format!("reject_finding {finding}"),
        Op::AddIntent { .. } => "add_intent".to_owned(),
        Op::SetIntentActive { intent, active } => format!("set_intent_active {intent}={active}"),
        Op::AddNarrative { about, .. } => format!("add_narrative about {about}"),
        Op::AddComment { about, .. } => format!(
            "add_comment about {}",
            about
                .capsule
                .as_deref()
                .or(about.instrument.as_deref())
                .unwrap_or_default()
        ),
        Op::ShowFinding { finding, .. } => format!("show_finding {finding}"),
        Op::Remove { id } => format!("remove {id}"),
        Op::AppendBars { series, bars } => format!("append_bars {series} +{}", bars.len()),
        Op::UpdateLastBar { series, .. } => format!("update_last_bar {series}"),
        Op::Undo { seq } => match seq {
            Some(s) => format!("undo {s}"),
            None => "undo".to_owned(),
        },
        Op::AcceptProposal { proposal, .. } => format!("accept_proposal {proposal}"),
        Op::RejectProposal { proposal, .. } => format!("reject_proposal {proposal}"),
        Op::InsertLayer { view, layer, .. } => match &layer.id {
            Some(id) => format!("insert_layer {view} {id}"),
            None => format!("insert_layer {view}"),
        },
        Op::DeleteLayer { view, layer } => format!("delete_layer {view} {layer}"),
        Op::SplitLayer { view, layer } => format!("split_layer {view} {layer}"),
        Op::MoveLayer { view, layer, .. } => format!("move_layer {view} {layer}"),
        Op::SetEncoding { view, layer, .. } => format!("set_encoding {view} {layer}"),
        Op::InsertPanel { view, .. } => format!("insert_panel {view}"),
        Op::DeletePanel { view, panel } => format!("delete_panel {view} {panel}"),
        Op::MovePanel { view, panel, .. } => format!("move_panel {view} {panel}"),
        Op::SetPanel { view, panel, .. } => format!("set_panel {view} {panel}"),
        Op::SetScale { view, scale, .. } => format!("set_scale {view} {scale}"),
        Op::SetEmphasis { view, emphasis } => {
            format!("set_emphasis {view} ({})", emphasis.len())
        }
        Op::SetAudience { principal, .. } => match principal {
            Some(w) => format!("set_audience {w}"),
            None => "set_audience".to_owned(),
        },
        Op::SetSensitivity {
            principal,
            sensitivity,
        } => {
            let level = sensitivity.map_or("default", |s| match s {
                qp_schema::capsule::Sensitivity::Important => "important",
                qp_schema::capsule::Sensitivity::Balanced => "balanced",
                qp_schema::capsule::Sensitivity::All => "all",
            });
            match principal {
                Some(w) => format!("set_sensitivity {w} {level}"),
                None => format!("set_sensitivity {level}"),
            }
        }
    }
}

/// Summary of a whole request, with created ids.
pub(crate) fn summarize_ops(ops: &[Op], created: &[String]) -> String {
    let mut s = ops.iter().map(summarize).collect::<Vec<_>>().join("; ");
    if !created.is_empty() {
        s.push_str(" -> ");
        s.push_str(&created.join(", "));
    }
    s
}
