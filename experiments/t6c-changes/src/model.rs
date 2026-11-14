//! A minimal authority for the experiment: expands `Op` into `ChangeOp` with domain
//! validation, applies `ChangeOp`, derives inverses, keeps a log with touched ids and
//! implements `undo` with `undo_conflict` (PROTOCOL.md §5.1, §5.3, §8.3).
//!
//! This is NOT the real authority (built in parallel); it implements just enough of the
//! v0 rules to produce realistic change streams.

use std::collections::{BTreeMap, BTreeSet};

use qp_schema::protocol::{ChangeOp, LogEntry, Op};
use qp_schema::{
    Author, Bar, Capsule, CapsuleState, Condition, Depth, Document, Fact, Finding, Intent,
    Layer, LedgerEntry, Outcome, Provenance, Series, SeriesData, SeriesSource,
};

/// Expansion "version": changing these simulates a newer authority build (used to
/// show that an `Op` log needs the same expansion code to replay).
#[derive(Clone, Debug)]
pub struct ExpandCfg {
    /// Default parameters per study.
    pub defaults: BTreeMap<&'static str, Vec<(&'static str, f64)>>,
    /// v2 rule: a new layer goes in front of rule layers instead of at the end.
    pub insert_before_rules: bool,
}

impl ExpandCfg {
    pub fn v1() -> Self {
        let mut d = BTreeMap::new();
        d.insert("sma", vec![("period", 20.0)]);
        d.insert("ema", vec![("period", 20.0)]);
        d.insert("rsi", vec![("period", 14.0)]);
        d.insert("macd", vec![("fast", 12.0), ("slow", 26.0), ("signal", 9.0)]);
        d.insert("bollinger", vec![("period", 20.0), ("k", 2.0)]);
        Self {
            defaults: d,
            insert_before_rules: false,
        }
    }
    /// A plausible later build: bollinger default k=2.5, layers inserted before rules.
    pub fn v2() -> Self {
        let mut c = Self::v1();
        c.defaults
            .insert("bollinger", vec![("period", 20.0), ("k", 2.5)]);
        c.insert_before_rules = true;
        c
    }
}

pub fn author_of(principal: &str) -> Author {
    match principal {
        "arkin" => Author::Human {
            name: principal.to_owned(),
        },
        "feed" => Author::Engine {
            name: "feed".to_owned(),
            version: "0".to_owned(),
        },
        _ => Author::Agent {
            name: principal.to_owned(),
            model: None,
        },
    }
}

fn prov(principal: &str, at: i64, based_on: Vec<String>) -> Provenance {
    Provenance {
        author: author_of(principal),
        at,
        based_on,
        approved_by: None,
    }
}

// ---------------------------------------------------------------------------
// Lookups
// ---------------------------------------------------------------------------

pub fn all_ids(doc: &Document) -> BTreeSet<String> {
    let mut s = BTreeSet::new();
    s.extend(doc.series.iter().map(|x| x.id.clone()));
    s.extend(doc.views.iter().map(|x| x.id.clone()));
    s.extend(doc.facts.iter().map(|x| x.id.clone()));
    s.extend(doc.findings.iter().map(|x| x.id.clone()));
    s.extend(doc.capsules.iter().map(|x| x.id.clone()));
    s.extend(doc.intents.iter().map(|x| x.id.clone()));
    s.extend(doc.narratives.iter().map(|x| x.id.clone()));
    s.extend(doc.proposals.iter().map(|x| x.id.clone()));
    s
}

pub fn series<'a>(doc: &'a Document, id: &str) -> Option<&'a Series> {
    doc.series.iter().find(|s| s.id == id)
}

/// Bars backing a series (a study series uses its first input's bars).
pub fn backing_bars<'a>(doc: &'a Document, id: &str) -> Option<&'a [Bar]> {
    let s = series(doc, id)?;
    match &s.source {
        SeriesSource::Study { inputs, .. } => backing_bars(doc, inputs.first()?),
        _ => s.bars(),
    }
}

fn condition_series(c: &Condition) -> Vec<&str> {
    match c {
        Condition::Threshold { series, .. } => vec![series.as_str()],
        Condition::Compare { left, right, .. } => vec![left.as_str(), right.as_str()],
        Condition::Text { .. } => vec![],
    }
}

fn check_condition(doc: &Document, c: &Condition) -> Result<(), String> {
    for s in condition_series(c) {
        if series(doc, s).is_none() {
            return Err(format!("not_found: condition series `{s}`"));
        }
    }
    Ok(())
}

pub fn known_params(cfg: &ExpandCfg, study: &str) -> Option<Vec<&'static str>> {
    cfg.defaults
        .get(study)
        .map(|v| v.iter().map(|(k, _)| *k).collect())
}

fn check_id(doc: &Document, id: &str) -> Result<(), String> {
    let ok = !id.is_empty()
        && id.len() <= 64
        && id.chars().next().is_some_and(|c| c.is_ascii_alphanumeric())
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "_.:-".contains(c));
    if !ok {
        return Err(format!("invalid: id `{id}`"));
    }
    if all_ids(doc).contains(id) {
        return Err(format!("duplicate: id `{id}`"));
    }
    Ok(())
}

fn check_bars(prev_t: Option<i64>, bars: &[Bar]) -> Result<(), String> {
    let mut p = prev_t;
    for b in bars {
        if [b.o, b.h, b.l, b.c, b.v].iter().any(|v| !v.is_finite())
            || b.h < b.o.max(b.c)
            || b.l > b.o.min(b.c)
        {
            return Err("invalid: inconsistent OHLC".into());
        }
        if p.is_some_and(|p| b.t <= p) {
            return Err("invalid: bar time not after last bar".into());
        }
        p = Some(b.t);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Expansion: Op -> ChangeOp (with validation)
// ---------------------------------------------------------------------------

pub struct Ctx<'a> {
    pub principal: &'a str,
    pub at: i64,
    pub seq: u64,
    pub cfg: &'a ExpandCfg,
}

/// Expands one op against `doc` (state after earlier ops of the same command).
/// `n` is the per-entry counter for assigned ids.
pub fn expand(doc: &Document, op: &Op, cx: &Ctx, n: &mut u32) -> Result<Vec<ChangeOp>, String> {
    let mut assign = |kind: &str, given: &Option<String>| -> String {
        given.clone().unwrap_or_else(|| {
            let id = format!("{kind}-{}-{}", cx.seq, *n);
            *n += 1;
            id
        })
    };
    // Permission: the feed principal may only write bars (role `feed`).
    if cx.principal == "feed" && !matches!(op, Op::AppendBars { .. } | Op::UpdateLastBar { .. })
    {
        return Err("forbidden: feed may only append or update bars".into());
    }
    match op {
        Op::PutSeries { series: s } => Ok(vec![ChangeOp::PutSeries { series: s.clone() }]),
        Op::AddStudy {
            id,
            study,
            inputs,
            params,
            show,
        } => {
            let id = assign("series", id);
            check_id(doc, &id)?;
            let known = known_params(cx.cfg, study)
                .ok_or_else(|| format!("invalid: unknown study `{study}`"))?;
            if inputs.len() != 1 {
                return Err("invalid: built-in studies take one input".into());
            }
            for i in inputs {
                let s = series(doc, i).ok_or_else(|| format!("not_found: input `{i}`"))?;
                if s.bars().is_none() {
                    return Err(format!("invalid: input `{i}` is not a bar series"));
                }
            }
            let mut p = BTreeMap::new();
            for (k, v) in &cx.cfg.defaults[study.as_str()] {
                p.insert((*k).to_owned(), *v);
            }
            for (k, v) in params {
                if !known.contains(&k.as_str()) {
                    return Err(format!("invalid: unknown param `{k}` for `{study}`"));
                }
                p.insert(k.clone(), *v);
            }
            let mut out = vec![ChangeOp::PutSeries {
                series: Series {
                    id: id.clone(),
                    instrument: None,
                    interval: None,
                    source: SeriesSource::Study {
                        study: study.clone(),
                        inputs: inputs.clone(),
                        params: p,
                        output: None,
                    },
                    data: None,
                },
            }];
            if let Some(sh) = show {
                let v = doc
                    .views
                    .iter()
                    .find(|v| v.id == sh.view)
                    .ok_or_else(|| format!("not_found: view `{}`", sh.view))?;
                let panel = if let Some(w) = &sh.with {
                    v.layers
                        .iter()
                        .find(|l| l.series.as_deref() == Some(w))
                        .map(|l| l.panel)
                        .ok_or_else(|| format!("not_found: no layer draws `{w}`"))?
                } else if let Some(p) = sh.panel {
                    p
                } else {
                    v.layers.iter().map(|l| l.panel).max().unwrap_or(0) + 1
                };
                let index = if cx.cfg.insert_before_rules {
                    v.layers
                        .iter()
                        .position(|l| l.series.is_none())
                        .unwrap_or(v.layers.len())
                } else {
                    v.layers.len()
                };
                out.push(ChangeOp::InsertLayer {
                    view: sh.view.clone(),
                    index: index as u32,
                    layer: Layer {
                        mark: qp_schema::Mark::Line,
                        series: Some(id),
                        panel,
                        encoding: BTreeMap::new(),
                    },
                });
            }
            Ok(out)
        }
        Op::SetParams { series: sid, params } => {
            let s = series(doc, sid).ok_or_else(|| format!("not_found: series `{sid}`"))?;
            let mut s = s.clone();
            let SeriesSource::Study {
                study, params: p, ..
            } = &mut s.source
            else {
                return Err(format!("invalid: `{sid}` is not a study series"));
            };
            let known = known_params(cx.cfg, study).unwrap_or_default();
            for (k, v) in params {
                if !known.contains(&k.as_str()) {
                    return Err(format!("invalid: unknown param `{k}` for `{study}`"));
                }
                if !(v.is_finite() && *v > 0.0) {
                    return Err(format!("invalid: param `{k}` must be > 0"));
                }
                p.insert(k.clone(), *v);
            }
            Ok(vec![ChangeOp::PutSeries { series: s }])
        }
        Op::PutView { view } => Ok(vec![ChangeOp::PutView { view: view.clone() }]),
        Op::AddLayer { view, layer, index } => {
            let v = doc
                .views
                .iter()
                .find(|v| &v.id == view)
                .ok_or_else(|| format!("not_found: view `{view}`"))?;
            if let Some(s) = &layer.series {
                if series(doc, s).is_none() {
                    return Err(format!("not_found: layer series `{s}`"));
                }
            }
            let idx = index.unwrap_or(v.layers.len() as u32);
            if idx as usize > v.layers.len() {
                return Err(format!("invalid: layer index {idx} out of range"));
            }
            Ok(vec![ChangeOp::InsertLayer {
                view: view.clone(),
                index: idx,
                layer: layer.clone(),
            }])
        }
        Op::RemoveLayer { view, index } => {
            let v = doc
                .views
                .iter()
                .find(|v| &v.id == view)
                .ok_or_else(|| format!("not_found: view `{view}`"))?;
            if *index as usize >= v.layers.len() {
                return Err(format!("not_found: layer index {index}"));
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
            let v = doc
                .views
                .iter()
                .find(|v| &v.id == view)
                .ok_or_else(|| format!("not_found: view `{view}`"))?;
            let bars = v
                .layers
                .iter()
                .filter_map(|l| l.series.as_deref())
                .find_map(|s| series(doc, s).and_then(Series::bars))
                .ok_or("invalid: view has no bar series")?;
            if from_bar > to_bar || *to_bar as usize >= bars.len() {
                return Err("invalid: bar range".into());
            }
            Ok(vec![ChangeOp::SetXDomain {
                view: view.clone(),
                domain: Some([
                    bars[*from_bar as usize].t as f64,
                    bars[*to_bar as usize].t as f64,
                ]),
            }])
        }
        Op::AddFact {
            id,
            kind,
            series: sid,
            bar,
            text,
            evidence,
        } => {
            let id = assign("fact", id);
            check_id(doc, &id)?;
            let bars = backing_bars(doc, sid).ok_or_else(|| format!("not_found: `{sid}`"))?;
            let b = bars
                .get(*bar as usize)
                .ok_or_else(|| format!("invalid: bar {bar} out of range"))?;
            Ok(vec![ChangeOp::PutFact {
                fact: Fact {
                    id,
                    kind: kind.clone(),
                    series: sid.clone(),
                    bar: *bar,
                    t: b.t,
                    text: text.clone(),
                    evidence: evidence.clone(),
                    provenance: prov(cx.principal, cx.at, vec![sid.clone()]),
                },
            }])
        }
        Op::AddFinding {
            id,
            claim,
            facts,
            confidence,
            invalidation,
            capsule,
        } => {
            let id = assign("fnd", id);
            check_id(doc, &id)?;
            for f in facts {
                if !doc.facts.iter().any(|x| &x.id == f) {
                    return Err(format!("not_found: fact `{f}`"));
                }
            }
            if !(0.0..=1.0).contains(confidence) {
                return Err("invalid: confidence outside [0,1]".into());
            }
            check_condition(doc, invalidation)?;
            let p = prov(cx.principal, cx.at, facts.clone());
            let mut out = vec![ChangeOp::PutFinding {
                finding: Finding {
                    id: id.clone(),
                    claim: claim.clone(),
                    facts: facts.clone(),
                    confidence: *confidence,
                    invalidation: invalidation.clone(),
                    provenance: p.clone(),
                },
            }];
            if let Some(h) = capsule {
                let cid = assign("cap", &None);
                out.push(ChangeOp::PutCapsule {
                    capsule: Capsule {
                        id: cid,
                        finding: id,
                        state: CapsuleState::Live,
                        revision: 0,
                        headline: h.clone(),
                        glyphs: vec![],
                        view: None,
                        depth: Depth::Capsule,
                        expires_at: None,
                        provenance: p,
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
            let mut f = doc
                .findings
                .iter()
                .find(|f| &f.id == finding)
                .ok_or_else(|| format!("not_found: finding `{finding}`"))?
                .clone();
            if let Some(c) = claim {
                f.claim.clone_from(c);
            }
            if let Some(c) = confidence {
                if !(0.0..=1.0).contains(c) {
                    return Err("invalid: confidence outside [0,1]".into());
                }
                f.confidence = *c;
            }
            if let Some(c) = invalidation {
                check_condition(doc, c)?;
                f.invalidation = c.clone();
            }
            Ok(vec![ChangeOp::PutFinding { finding: f }])
        }
        Op::ApproveFinding { finding, note } | Op::RejectFinding { finding, note } => {
            let state = if matches!(op, Op::ApproveFinding { .. }) {
                CapsuleState::Approved
            } else {
                CapsuleState::Rejected
            };
            if !doc.findings.iter().any(|f| &f.id == finding) {
                return Err(format!("not_found: finding `{finding}`"));
            }
            let cap = doc
                .capsules
                .iter()
                .find(|c| &c.finding == finding)
                .ok_or_else(|| format!("not_found: capsule of `{finding}`"))?;
            if !matches!(cap.state, CapsuleState::Live | CapsuleState::Draft) {
                return Err(format!(
                    "invalid: capsule `{}` is {:?}, cannot become {:?}",
                    cap.id, cap.state, state
                ));
            }
            Ok(vec![
                ChangeOp::SetCapsuleState {
                    capsule: cap.id.clone(),
                    state,
                },
                ChangeOp::PushLedger {
                    entry: LedgerEntry {
                        finding: finding.clone(),
                        state: Some(state),
                        outcome: Outcome::Pending,
                        recorded_at: cx.at,
                        note: note.clone(),
                    },
                },
            ])
        }
        Op::AddIntent { id, text, trigger } => {
            let id = assign("intent", id);
            check_id(doc, &id)?;
            if let Some(t) = trigger {
                check_condition(doc, t)?;
            }
            Ok(vec![ChangeOp::PutIntent {
                intent: Intent {
                    id,
                    text: text.clone(),
                    trigger: trigger.clone(),
                    active: true,
                    provenance: prov(cx.principal, cx.at, vec![]),
                },
            }])
        }
        Op::SetIntentActive { intent, active } => {
            let mut i = doc
                .intents
                .iter()
                .find(|i| &i.id == intent)
                .ok_or_else(|| format!("not_found: intent `{intent}`"))?
                .clone();
            i.active = *active;
            Ok(vec![ChangeOp::PutIntent { intent: i }])
        }
        Op::Remove { id } => expand_remove(doc, id),
        Op::AppendBars { series: sid, bars } => {
            let s = series(doc, sid).ok_or_else(|| format!("not_found: `{sid}`"))?;
            let cur = s.bars().ok_or("invalid: not a bar series")?;
            if bars.is_empty() {
                return Err("invalid: no bars".into());
            }
            check_bars(cur.last().map(|b| b.t), bars)?;
            Ok(vec![ChangeOp::AppendBars {
                series: sid.clone(),
                bars: bars.clone(),
            }])
        }
        Op::UpdateLastBar { series: sid, bar } => {
            let s = series(doc, sid).ok_or_else(|| format!("not_found: `{sid}`"))?;
            let cur = s.bars().ok_or("invalid: not a bar series")?;
            let last = cur.last().ok_or("invalid: no bars")?;
            if last.t != bar.t {
                return Err("invalid: update_last_bar must keep `t`".into());
            }
            check_bars(None, std::slice::from_ref(bar))?;
            Ok(vec![ChangeOp::SetLastBar {
                series: sid.clone(),
                bar: bar.clone(),
            }])
        }
        _ => Err("unsupported: op not implemented in experiment".into()),
    }
}

fn expand_remove(doc: &Document, id: &str) -> Result<Vec<ChangeOp>, String> {
    if series(doc, id).is_some() {
        let mut users = vec![];
        for s in &doc.series {
            if let SeriesSource::Study { inputs, .. } = &s.source {
                if inputs.iter().any(|i| i == id) {
                    users.push(s.id.clone());
                }
            }
        }
        users.extend(doc.facts.iter().filter(|f| f.series == id).map(|f| f.id.clone()));
        users.extend(
            doc.findings
                .iter()
                .filter(|f| condition_series(&f.invalidation).contains(&id))
                .map(|f| f.id.clone()),
        );
        users.extend(
            doc.intents
                .iter()
                .filter(|i| {
                    i.trigger
                        .as_ref()
                        .is_some_and(|t| condition_series(t).contains(&id))
                })
                .map(|i| i.id.clone()),
        );
        if !users.is_empty() {
            return Err(format!("in_use: `{id}` used by {}", users.join(", ")));
        }
        let mut out = vec![];
        for v in &doc.views {
            // Descending so earlier deletes don't shift later indices.
            for (i, l) in v.layers.iter().enumerate().rev() {
                if l.series.as_deref() == Some(id) {
                    out.push(ChangeOp::DeleteLayer {
                        view: v.id.clone(),
                        index: i as u32,
                    });
                }
            }
        }
        out.push(ChangeOp::Delete { id: id.to_owned() });
        return Ok(out);
    }
    if doc.facts.iter().any(|f| f.id == id) {
        if let Some(f) = doc.findings.iter().find(|f| f.facts.iter().any(|x| x == id)) {
            return Err(format!("in_use: `{id}` used by {}", f.id));
        }
    } else if doc.findings.iter().any(|f| f.id == id) {
        if let Some(c) = doc.capsules.iter().find(|c| c.finding == id) {
            return Err(format!("in_use: `{id}` used by {}", c.id));
        }
    } else if !all_ids(doc).contains(id) {
        return Err(format!("not_found: `{id}`"));
    }
    Ok(vec![ChangeOp::Delete { id: id.to_owned() }])
}

// ---------------------------------------------------------------------------
// Apply + inverse
// ---------------------------------------------------------------------------

fn put<T: Clone>(v: &mut Vec<T>, item: T, id_of: impl Fn(&T) -> &str) {
    let id = id_of(&item).to_owned();
    if let Some(slot) = v.iter_mut().find(|x| id_of(x) == id) {
        *slot = item;
    } else {
        v.push(item);
    }
}

fn bars_mut<'a>(doc: &'a mut Document, sid: &str) -> Result<&'a mut Vec<Bar>, String> {
    let s = doc
        .series
        .iter_mut()
        .find(|s| s.id == sid)
        .ok_or_else(|| format!("no series {sid}"))?;
    match &mut s.data {
        Some(SeriesData::Bars { bars }) => Ok(bars),
        _ => Err("not bars".into()),
    }
}

pub fn apply(doc: &mut Document, c: &ChangeOp) -> Result<(), String> {
    match c {
        ChangeOp::PutSeries { series } => put(&mut doc.series, series.clone(), |x| &x.id),
        ChangeOp::PutView { view } => put(&mut doc.views, view.clone(), |x| &x.id),
        ChangeOp::InsertLayer { view, index, layer } => {
            let v = doc.views.iter_mut().find(|v| &v.id == view).ok_or("no view")?;
            if *index as usize > v.layers.len() {
                return Err("index".into());
            }
            v.layers.insert(*index as usize, layer.clone());
        }
        ChangeOp::DeleteLayer { view, index } => {
            let v = doc.views.iter_mut().find(|v| &v.id == view).ok_or("no view")?;
            if *index as usize >= v.layers.len() {
                return Err("index".into());
            }
            v.layers.remove(*index as usize);
        }
        ChangeOp::SetXDomain { view, domain } => {
            let v = doc.views.iter_mut().find(|v| &v.id == view).ok_or("no view")?;
            v.x.domain = *domain;
        }
        ChangeOp::PutFact { fact } => put(&mut doc.facts, fact.clone(), |x| &x.id),
        ChangeOp::PutFinding { finding } => put(&mut doc.findings, finding.clone(), |x| &x.id),
        ChangeOp::PutCapsule { capsule } => put(&mut doc.capsules, capsule.clone(), |x| &x.id),
        ChangeOp::SetCapsuleState { capsule, state } => {
            doc.capsules
                .iter_mut()
                .find(|c| &c.id == capsule)
                .ok_or("no capsule")?
                .state = *state;
        }
        ChangeOp::PutIntent { intent } => put(&mut doc.intents, intent.clone(), |x| &x.id),
        ChangeOp::PushLedger { entry } => doc.ledger.entries.push(entry.clone()),
        ChangeOp::PopLedger => {
            doc.ledger.entries.pop().ok_or("empty ledger")?;
        }
        ChangeOp::PutNarrative { narrative } => {
            put(&mut doc.narratives, narrative.clone(), |x| &x.id);
        }
        ChangeOp::PutProposal { proposal } => put(&mut doc.proposals, proposal.clone(), |x| &x.id),
        ChangeOp::Delete { id } => {
            let before = all_ids(doc).len();
            doc.series.retain(|x| &x.id != id);
            doc.views.retain(|x| &x.id != id);
            doc.facts.retain(|x| &x.id != id);
            doc.findings.retain(|x| &x.id != id);
            doc.capsules.retain(|x| &x.id != id);
            doc.intents.retain(|x| &x.id != id);
            doc.narratives.retain(|x| &x.id != id);
            doc.proposals.retain(|x| &x.id != id);
            if all_ids(doc).len() == before {
                return Err(format!("no item {id}"));
            }
        }
        ChangeOp::AppendBars { series, bars } => bars_mut(doc, series)?.extend(bars.iter().cloned()),
        ChangeOp::TruncateBars { series, count } => {
            let b = bars_mut(doc, series)?;
            let n = b.len().checked_sub(*count as usize).ok_or("truncate")?;
            b.truncate(n);
        }
        ChangeOp::SetLastBar { series, bar } => {
            *bars_mut(doc, series)?.last_mut().ok_or("no bars")? = bar.clone();
        }
    }
    Ok(())
}

/// Inverse of one change, computed against the state *before* it is applied.
pub fn inverse_one(doc: &Document, c: &ChangeOp) -> Vec<ChangeOp> {
    match c {
        ChangeOp::PutSeries { series: s } => match series(doc, &s.id) {
            Some(old) => vec![ChangeOp::PutSeries { series: old.clone() }],
            None => vec![ChangeOp::Delete { id: s.id.clone() }],
        },
        ChangeOp::PutView { view } => match doc.views.iter().find(|v| v.id == view.id) {
            Some(old) => vec![ChangeOp::PutView { view: old.clone() }],
            None => vec![ChangeOp::Delete { id: view.id.clone() }],
        },
        ChangeOp::InsertLayer { view, index, .. } => vec![ChangeOp::DeleteLayer {
            view: view.clone(),
            index: *index,
        }],
        ChangeOp::DeleteLayer { view, index } => {
            let v = doc.views.iter().find(|v| &v.id == view).expect("view");
            vec![ChangeOp::InsertLayer {
                view: view.clone(),
                index: *index,
                layer: v.layers[*index as usize].clone(),
            }]
        }
        ChangeOp::SetXDomain { view, .. } => {
            let v = doc.views.iter().find(|v| &v.id == view).expect("view");
            vec![ChangeOp::SetXDomain {
                view: view.clone(),
                domain: v.x.domain,
            }]
        }
        ChangeOp::PutFact { fact } => match doc.facts.iter().find(|x| x.id == fact.id) {
            Some(o) => vec![ChangeOp::PutFact { fact: o.clone() }],
            None => vec![ChangeOp::Delete { id: fact.id.clone() }],
        },
        ChangeOp::PutFinding { finding } => match doc.findings.iter().find(|x| x.id == finding.id) {
            Some(o) => vec![ChangeOp::PutFinding { finding: o.clone() }],
            None => vec![ChangeOp::Delete {
                id: finding.id.clone(),
            }],
        },
        ChangeOp::PutCapsule { capsule } => match doc.capsules.iter().find(|x| x.id == capsule.id) {
            Some(o) => vec![ChangeOp::PutCapsule { capsule: o.clone() }],
            None => vec![ChangeOp::Delete {
                id: capsule.id.clone(),
            }],
        },
        ChangeOp::SetCapsuleState { capsule, .. } => {
            let c = doc.capsules.iter().find(|c| &c.id == capsule).expect("cap");
            vec![ChangeOp::SetCapsuleState {
                capsule: capsule.clone(),
                state: c.state,
            }]
        }
        ChangeOp::PutIntent { intent } => match doc.intents.iter().find(|x| x.id == intent.id) {
            Some(o) => vec![ChangeOp::PutIntent { intent: o.clone() }],
            None => vec![ChangeOp::Delete {
                id: intent.id.clone(),
            }],
        },
        ChangeOp::PushLedger { .. } => vec![ChangeOp::PopLedger],
        ChangeOp::PopLedger => vec![ChangeOp::PushLedger {
            entry: doc.ledger.entries.last().expect("ledger").clone(),
        }],
        ChangeOp::PutNarrative { .. } | ChangeOp::PutProposal { .. } => vec![],
        ChangeOp::Delete { id } => {
            if let Some(x) = series(doc, id) {
                vec![ChangeOp::PutSeries { series: x.clone() }]
            } else if let Some(x) = doc.views.iter().find(|x| &x.id == id) {
                vec![ChangeOp::PutView { view: x.clone() }]
            } else if let Some(x) = doc.facts.iter().find(|x| &x.id == id) {
                vec![ChangeOp::PutFact { fact: x.clone() }]
            } else if let Some(x) = doc.findings.iter().find(|x| &x.id == id) {
                vec![ChangeOp::PutFinding { finding: x.clone() }]
            } else if let Some(x) = doc.capsules.iter().find(|x| &x.id == id) {
                vec![ChangeOp::PutCapsule { capsule: x.clone() }]
            } else if let Some(x) = doc.intents.iter().find(|x| &x.id == id) {
                vec![ChangeOp::PutIntent { intent: x.clone() }]
            } else {
                vec![]
            }
        }
        ChangeOp::AppendBars { series, bars } => vec![ChangeOp::TruncateBars {
            series: series.clone(),
            count: bars.len() as u32,
        }],
        ChangeOp::TruncateBars { series: sid, count } => {
            let b = series(doc, sid).and_then(Series::bars).expect("bars");
            vec![ChangeOp::AppendBars {
                series: sid.clone(),
                bars: b[b.len() - *count as usize..].to_vec(),
            }]
        }
        ChangeOp::SetLastBar { series: sid, .. } => {
            let b = series(doc, sid).and_then(Series::bars).expect("bars");
            vec![ChangeOp::SetLastBar {
                series: sid.clone(),
                bar: b.last().expect("bar").clone(),
            }]
        }
    }
}

/// Applies changes in order, deriving the entry's inverse in place (each change's
/// inverse is computed against the state just before it).
pub fn apply_with_inverse(doc: &mut Document, changes: &[ChangeOp]) -> Result<Vec<ChangeOp>, String> {
    let mut inv = vec![];
    for c in changes {
        let mut i = inverse_one(doc, c);
        i.reverse();
        inv.extend(i);
        apply(doc, c)?;
    }
    inv.reverse();
    Ok(inv)
}

/// Ids a change touches (for `base` conflicts and `undo_conflict`).
pub fn touched(c: &ChangeOp) -> Vec<String> {
    match c {
        ChangeOp::PutSeries { series } => vec![series.id.clone()],
        ChangeOp::PutView { view } => vec![view.id.clone()],
        ChangeOp::InsertLayer { view, .. }
        | ChangeOp::DeleteLayer { view, .. }
        | ChangeOp::SetXDomain { view, .. } => vec![view.clone()],
        ChangeOp::PutFact { fact } => vec![fact.id.clone()],
        ChangeOp::PutFinding { finding } => vec![finding.id.clone()],
        ChangeOp::PutCapsule { capsule } => vec![capsule.id.clone()],
        ChangeOp::SetCapsuleState { capsule, .. } => vec![capsule.clone()],
        ChangeOp::PutIntent { intent } => vec![intent.id.clone()],
        ChangeOp::PushLedger { .. } | ChangeOp::PopLedger => vec!["ledger".into()],
        ChangeOp::PutNarrative { narrative } => vec![narrative.id.clone()],
        ChangeOp::PutProposal { proposal } => vec![proposal.id.clone()],
        ChangeOp::Delete { id } => vec![id.clone()],
        ChangeOp::AppendBars { series, .. }
        | ChangeOp::TruncateBars { series, .. }
        | ChangeOp::SetLastBar { series, .. } => vec![series.clone()],
    }
}

// ---------------------------------------------------------------------------
// Authority
// ---------------------------------------------------------------------------

pub struct Authority {
    pub doc: Document,
    pub log: Vec<LogEntry>,
    pub touched: Vec<BTreeSet<String>>,
    pub undone: BTreeSet<u64>,
    pub cfg: ExpandCfg,
}

impl Authority {
    pub fn new(doc: Document, cfg: ExpandCfg) -> Self {
        Self {
            doc,
            log: vec![],
            touched: vec![],
            undone: BTreeSet::new(),
            cfg,
        }
    }

    pub fn next_seq(&self) -> u64 {
        self.log.len() as u64 + 1
    }

    /// Expands a command (all-or-nothing) without mutating the document; returns the
    /// changes and their inverse. Undo is handled here too.
    pub fn plan(
        &self,
        principal: &str,
        at: i64,
        ops: &[Op],
    ) -> Result<(Vec<ChangeOp>, Option<u64>), String> {
        if let [Op::Undo { seq }] = ops {
            let target = seq.ok_or("experiment: undo needs seq")?;
            let e = self
                .log
                .get(target as usize - 1)
                .ok_or("not_found: seq")?;
            if self.undone.contains(&target) {
                return Err("invalid: already undone".into());
            }
            let t = &self.touched[target as usize - 1];
            let conflicts: Vec<u64> = (target + 1..self.next_seq())
                .filter(|s| !self.undone.contains(s))
                .filter(|s| !self.touched[*s as usize - 1].is_disjoint(t))
                .collect();
            if !conflicts.is_empty() {
                return Err(format!("undo_conflict: {conflicts:?}"));
            }
            return Ok((e.inverse.clone(), Some(target)));
        }
        let cx = Ctx {
            principal,
            at,
            seq: self.next_seq(),
            cfg: &self.cfg,
        };
        let mut n = 0;
        if ops.len() == 1 {
            return Ok((expand(&self.doc, &ops[0], &cx, &mut n)?, None));
        }
        // Multi-op commands need the intermediate state.
        let mut scratch = self.doc.clone();
        let mut changes = vec![];
        for op in ops {
            for c in expand(&scratch, op, &cx, &mut n)? {
                apply(&mut scratch, &c)?;
                changes.push(c);
            }
        }
        Ok((changes, None))
    }

    /// Executes a command. Returns the log entry.
    pub fn execute(
        &mut self,
        principal: &str,
        at: i64,
        ops: &[Op],
        summary: &str,
    ) -> Result<&LogEntry, String> {
        let (changes, undoes) = self.plan(principal, at, ops)?;
        let inverse = apply_with_inverse(&mut self.doc, &changes)?;
        Ok(self.record(principal, at, ops, changes, inverse, undoes, summary))
    }

    /// Placeholder entry (keeps seq numbering when a step is skipped in a replay).
    pub fn skip(&mut self) {
        self.touched.push(BTreeSet::new());
        self.log.push(LogEntry {
            seq: self.next_seq(),
            at: 0,
            author: "-".into(),
            approved_by: None,
            cause: None,
            request: vec![],
            changes: vec![],
            inverse: vec![],
            undoes: None,
            summary: "skipped".into(),
        });
        self.undone.insert(self.log.len() as u64);
    }

    /// Records an entry whose changes were already applied to `self.doc`.
    #[allow(clippy::too_many_arguments)]
    pub fn record(
        &mut self,
        principal: &str,
        at: i64,
        ops: &[Op],
        changes: Vec<ChangeOp>,
        inverse: Vec<ChangeOp>,
        undoes: Option<u64>,
        summary: &str,
    ) -> &LogEntry {
        let t: BTreeSet<String> = changes.iter().flat_map(touched).collect();
        if let Some(u) = undoes {
            self.undone.insert(u);
        }
        self.touched.push(t);
        self.log.push(LogEntry {
            seq: self.next_seq(),
            at,
            author: principal.to_owned(),
            approved_by: None,
            cause: None,
            request: ops.to_vec(),
            changes,
            inverse,
            undoes,
            summary: summary.to_owned(),
        });
        self.log.last().expect("entry")
    }
}
