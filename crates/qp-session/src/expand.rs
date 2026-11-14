//! Expansion of protocol operations ([`Op`]) into replayable changes ([`ChangeOp`]),
//! with validation (`docs/PROTOCOL.md` §5.1). Each op is expanded against a scratch copy
//! of the document that already contains the effects of the earlier ops of the same
//! command; the caller discards the copy if any op fails (all or nothing).

use std::collections::{BTreeMap, BTreeSet};

use qp_schema::protocol::{
    ChangeOp, ErrorCode, LogEntry, Narrative, Op, ProposalState, Seq, Show, Target,
};
use qp_schema::{
    Author, Capsule, CapsuleState, Condition, Depth, Document, Fact, Finding, Intent, Layer,
    LedgerEntry, Mark, Outcome, Provenance, Scale, ScaleKind, Series, SeriesData, SeriesSource,
    View,
};

use crate::apply::{all_ids, apply_all, find_collection, touched_ids};
use crate::error::{invalid, num, ptr, PResult, E};
use crate::values::{bar_series, bars_of, condition_series, find_series, study_outputs};

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
}

impl Ctx<'_> {
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
    if input.bars().is_none() {
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

/// The change that puts `layer` into `view` at `index` (appended if absent): an
/// `insert_layer`, or a `put_view` when the layer's panel needs new y scales.
fn layer_change(view: &View, index: Option<u32>, layer: Layer) -> ChangeOp {
    let idx = index.unwrap_or(u32::try_from(view.layers.len()).unwrap_or(u32::MAX));
    if (layer.panel as usize) < view.y.len() {
        ChangeOp::InsertLayer {
            view: view.id.clone(),
            index: idx,
            layer,
        }
    } else {
        let mut v = view.clone();
        while v.y.len() <= layer.panel as usize {
            v.y.push(Scale {
                kind: ScaleKind::Linear,
                domain: None,
            });
        }
        v.layers.insert(idx as usize, layer);
        ChangeOp::PutView {
            view: v,
            index: None,
        }
    }
}

fn check_target(doc: &Document, t: &Target, path: &str) -> PResult<()> {
    if let Some(v) = &t.view {
        need_view(doc, v, &ptr(path, "view"))?;
    }
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
                    _ => {
                        return Err(E::new(
                            ErrorCode::Unsupported,
                            "an inline series must carry bars (`data.kind = \"bars\"`)",
                        )
                        .at(ptr(&sp, "data"))
                        .hint("v0 supports inline bar series and study series.")
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
                SeriesSource::File { .. } | SeriesSource::Matriks { .. } | SeriesSource::Nats { .. } => {
                    return Err(E::new(
                        ErrorCode::Unsupported,
                        "this authority accepts only `inline` and `study` series sources",
                    )
                    .at(ptr(&src, "kind"))
                    .expected(["inline", "study"])
                    .hint("File and feed bindings are opened by the host configuration, not by the protocol.")
                    .build())
                }
            }
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
            if let Some((view, panel)) = show_view {
                let layer = Layer {
                    mark: Mark::Line,
                    series: Some(sid),
                    panel,
                    encoding: BTreeMap::new(),
                };
                out.push(layer_change(view, None, layer));
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
            for (j, l) in view.layers.iter().enumerate() {
                let lp = ptr(&ptr(&vp, "layers"), j);
                if let Some(s) = &l.series {
                    need_series(doc, s, &ptr(&lp, "series"))?;
                }
                if l.panel as usize >= view.y.len() {
                    return Err(E::new(
                        ErrorCode::Invalid,
                        format!(
                            "panel {} has no y scale (the view has {})",
                            l.panel,
                            view.y.len()
                        ),
                    )
                    .at(ptr(&lp, "panel"))
                    .build());
                }
            }
            if !doc.views.iter().any(|v| v.id == view.id) {
                ctx.note_created(&view.id);
            }
            Ok(vec![ChangeOp::PutView {
                view: view.clone(),
                index: None,
            }])
        }
        Op::AddLayer { view, layer, index } => {
            let v = need_view(doc, view, &ptr(p, "view"))?;
            if let Some(s) = &layer.series {
                need_series(doc, s, &ptr(&ptr(p, "layer"), "series"))?;
            }
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
            Ok(vec![layer_change(v, *index, layer.clone())])
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
            Ok(vec![ChangeOp::DeleteLayer {
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
            let bars = view_bar_series(doc, v)
                .and_then(Series::bars)
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
            let bars = bars_of(doc, series).ok_or_else(|| {
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
            let fid = ctx.new_id(doc, "fact", id.as_ref(), &ptr(p, "id"))?;
            let fact = Fact {
                id: fid,
                kind: kind.clone(),
                series: series.clone(),
                bar: *bar,
                t,
                text: text.clone(),
                evidence: evidence.clone(),
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
                },
            });
            Ok(out)
        }
        Op::AddIntent { id, text, trigger } => {
            if let Some(t) = trigger {
                check_condition(doc, t, &ptr(p, "trigger"))?;
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
            let built = crate::present::presenter_view(doc, f, &vid)
                .map_err(|m| invalid(m, ptr(p, "finding")))?;
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
        Op::Undo { seq } => expand_undo(*seq, p, ctx),
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

fn inline_bars(s: &Series) -> Option<&[qp_schema::Bar]> {
    if matches!(s.source, SeriesSource::Inline {}) {
        s.bars()
    } else {
        None
    }
}

/// Validates `show` and returns the view and the panel.
fn check_show<'a>(doc: &'a Document, sh: &Show, p: &str) -> PResult<(&'a View, u32)> {
    let v = need_view(doc, &sh.view, &ptr(p, "view"))?;
    let max_panel = v.layers.iter().map(|l| l.panel).max();
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
        return Ok((v, l.panel));
    }
    if let Some(panel) = sh.panel {
        let limit = max_panel.map_or(0, |m| m + 1);
        if panel > limit {
            return Err(E::new(
                ErrorCode::Invalid,
                format!("panel {panel} does not exist; the next new panel is {limit}"),
            )
            .at(ptr(p, "panel"))
            .expected([format!("0..={limit}")])
            .build());
        }
        return Ok((v, panel));
    }
    Ok((v, max_panel.map_or(0, |m| m + 1)))
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
        C::Capsules | C::Intents | C::Narratives => {}
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

fn expand_undo(seq: Option<Seq>, p: &str, ctx: &mut Ctx<'_>) -> PResult<Vec<ChangeOp>> {
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
    let touched = touched_ids(&target.changes);
    let mut conflicts = Vec::new();
    let mut by = String::new();
    for e in &ctx.log[usize::try_from(target.seq)
        .unwrap_or(usize::MAX)
        .min(ctx.log.len())..]
    {
        if undone.contains(&e.seq) || e.undoes.is_some() {
            continue;
        }
        let t = touched_ids(&e.changes);
        if let Some(id) = t.intersection(&touched).next() {
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
    Ok(target.inverse.clone())
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
