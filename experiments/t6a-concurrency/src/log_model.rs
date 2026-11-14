//! Model A: one authority per document, an ordered operation log, item-level optimistic
//! concurrency (`base`), inverse changes for undo (`docs/PROTOCOL.md` §8). Uses the real
//! `qp-schema` protocol types on the wire (`ClientMsg::Command`, `ServerMsg::Ack/Error/
//! Notify`, `LogEntry`, `ChangeOp`).

use std::collections::{BTreeMap, BTreeSet};

use qp_schema::protocol::{
    ChangeOp, ClientMsg, ErrorCode, Event, LogEntry, Op as POp, ProtocolError, QueryResult, Seq,
    ServerMsg, Show, WriteMode,
};
use qp_schema::{
    Author, Capsule, CapsuleState, CompareOp, Condition, Depth, Document, Finding, Intent,
    Layer, LedgerEntry, Mark, Outcome, Provenance, Series, SeriesSource,
};

use crate::common::{
    study_param_names, validate, Code, Intent as WIntent, SCapsule, SFinding, SLayer, SLedger,
    SSeries, SemOp, Snap, Who, FACT,
};
use crate::sim::{Exp, Model, Reconnect, UndoStats, Wire};

const T0: i64 = 1_760_000_000_000;

fn author(who: Who) -> Author {
    match who {
        Who::Arkin => Author::Human {
            name: who.name().into(),
        },
        _ => Author::Agent {
            name: who.name().into(),
            model: None,
        },
    }
}

fn prov(who: Who, at: i64) -> Provenance {
    Provenance {
        author: author(who),
        at,
        based_on: vec![],
        approved_by: None,
    }
}

fn code_of(c: Code) -> ErrorCode {
    match c {
        Code::NotFound => ErrorCode::NotFound,
        Code::Duplicate => ErrorCode::Duplicate,
        Code::Invalid => ErrorCode::Invalid,
        Code::InUse => ErrorCode::InUse,
        Code::Forbidden => ErrorCode::Forbidden,
        Code::Conflict => ErrorCode::Conflict,
        Code::UndoConflict => ErrorCode::UndoConflict,
        Code::Unsupported => ErrorCode::Unsupported,
    }
}

// ---------------------------------------------------------------------------
// Engine: a document plus the change interpreter and op expander.
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct Engine {
    pub doc: Document,
    /// Claim of the finding at the time each ledger entry was pushed.
    pub decided: Vec<String>,
}

fn id_exists(d: &Document, id: &str) -> bool {
    d.series.iter().any(|x| x.id == id)
        || d.views.iter().any(|x| x.id == id)
        || d.facts.iter().any(|x| x.id == id)
        || d.findings.iter().any(|x| x.id == id)
        || d.capsules.iter().any(|x| x.id == id)
        || d.intents.iter().any(|x| x.id == id)
        || d.narratives.iter().any(|x| x.id == id)
        || d.proposals.iter().any(|x| x.id == id)
}

fn upsert<T: Clone>(v: &mut Vec<T>, item: T, id: impl Fn(&T) -> &str) {
    let key = id(&item).to_string();
    if let Some(x) = v.iter_mut().find(|x| id(x) == key) {
        *x = item;
    } else {
        v.push(item);
    }
}

impl Engine {
    pub fn apply(&mut self, ch: &ChangeOp) {
        let d = &mut self.doc;
        match ch {
            ChangeOp::PutSeries { series } => upsert(&mut d.series, series.clone(), |x| &x.id),
            ChangeOp::PutFinding { finding } => {
                upsert(&mut d.findings, finding.clone(), |x| &x.id)
            }
            ChangeOp::PutCapsule { capsule } => {
                upsert(&mut d.capsules, capsule.clone(), |x| &x.id)
            }
            ChangeOp::PutIntent { intent } => upsert(&mut d.intents, intent.clone(), |x| &x.id),
            ChangeOp::InsertLayer { view, index, layer } => {
                let v = d.views.iter_mut().find(|v| &v.id == view).expect("view");
                v.layers.insert(*index as usize, layer.clone());
            }
            ChangeOp::DeleteLayer { view, index } => {
                let v = d.views.iter_mut().find(|v| &v.id == view).expect("view");
                v.layers.remove(*index as usize);
            }
            ChangeOp::SetCapsuleState { capsule, state } => {
                let c = d.capsules.iter_mut().find(|c| &c.id == capsule).expect("cap");
                c.state = *state;
            }
            ChangeOp::PushLedger { entry } => {
                let claim = d
                    .findings
                    .iter()
                    .find(|f| f.id == entry.finding)
                    .map(|f| f.claim.clone())
                    .unwrap_or_default();
                d.ledger.entries.push(entry.clone());
                self.decided.push(claim);
            }
            ChangeOp::PopLedger => {
                d.ledger.entries.pop();
                self.decided.pop();
            }
            ChangeOp::Delete { id } => {
                d.series.retain(|x| &x.id != id);
                d.facts.retain(|x| &x.id != id);
                d.findings.retain(|x| &x.id != id);
                d.capsules.retain(|x| &x.id != id);
                d.intents.retain(|x| &x.id != id);
            }
            other => panic!("change not used in this experiment: {other:?}"),
        }
    }

    /// Study inputs, facts, finding invalidations and intent triggers naming `id`.
    fn has_dependents(&self, id: &str) -> bool {
        let d = &self.doc;
        let cond = |c: &Condition| match c {
            Condition::Threshold { series, .. } => series == id,
            Condition::Compare { left, right, .. } => left == id || right == id,
            Condition::Text { .. } => false,
        };
        d.series.iter().any(|s| {
            matches!(&s.source, SeriesSource::Study { inputs, .. } if inputs.iter().any(|i| i == id))
        }) || d.facts.iter().any(|f| f.series == id)
            || d.findings.iter().any(|f| cond(&f.invalidation))
            || d.intents.iter().any(|i| i.trigger.as_ref().is_some_and(cond))
    }

    fn main_layers(&self) -> &Vec<Layer> {
        &self.doc.views.iter().find(|v| v.id == "main").expect("main").layers
    }

    /// Expand one op into changes, applying them as it goes. Pushes inverses in
    /// application order (caller reverses).
    fn expand(
        &mut self,
        who: Who,
        op: &POp,
        seq: Seq,
        created: &mut Vec<String>,
        ch: &mut Vec<ChangeOp>,
        inv: &mut Vec<ChangeOp>,
    ) -> Result<(), Code> {
        let at = T0 + seq as i64 * 1000;
        let mut emit = |eng: &mut Engine, c: ChangeOp, i: ChangeOp| {
            eng.apply(&c);
            ch.push(c);
            inv.push(i);
        };
        match op {
            POp::AddStudy {
                id,
                study,
                inputs,
                params,
                show,
            } => {
                let id = id.clone().unwrap_or_else(|| format!("series-{seq}-{}", created.len()));
                if id_exists(&self.doc, &id) {
                    return Err(Code::Duplicate);
                }
                let names = study_param_names(study);
                if names.is_empty() || params.keys().any(|k| !names.contains(&k.as_str())) {
                    return Err(Code::Invalid);
                }
                let px = self
                    .doc
                    .series
                    .iter()
                    .find(|s| inputs.first() == Some(&s.id))
                    .ok_or(Code::NotFound)?;
                let series = Series {
                    id: id.clone(),
                    instrument: px.instrument.clone(),
                    interval: px.interval.clone(),
                    source: SeriesSource::Study {
                        study: study.clone(),
                        inputs: inputs.clone(),
                        params: params.clone(),
                        output: None,
                    },
                    data: None,
                };
                if let Some(show) = show {
                    let max = self.main_layers().iter().map(|l| l.panel).max().unwrap_or(0);
                    let panel = show.panel.unwrap_or(max + 1);
                    if panel > max + 1 {
                        return Err(Code::Invalid);
                    }
                    emit(
                        self,
                        ChangeOp::PutSeries { series },
                        ChangeOp::Delete { id: id.clone() },
                    );
                    let index = self.main_layers().len() as u32;
                    emit(
                        self,
                        ChangeOp::InsertLayer {
                            view: "main".into(),
                            index,
                            layer: Layer {
                                mark: Mark::Line,
                                series: Some(id.clone()),
                                panel,
                                encoding: BTreeMap::new(),
                            },
                        },
                        ChangeOp::DeleteLayer {
                            view: "main".into(),
                            index,
                        },
                    );
                } else {
                    emit(
                        self,
                        ChangeOp::PutSeries { series },
                        ChangeOp::Delete { id: id.clone() },
                    );
                }
                created.push(id);
            }
            POp::SetParams { series, params } => {
                let old = self
                    .doc
                    .series
                    .iter()
                    .find(|s| &s.id == series)
                    .ok_or(Code::NotFound)?
                    .clone();
                let mut new = old.clone();
                let SeriesSource::Study {
                    study, params: p, ..
                } = &mut new.source
                else {
                    return Err(Code::Invalid);
                };
                let names = study_param_names(study);
                if params.keys().any(|k| !names.contains(&k.as_str())) {
                    return Err(Code::Invalid);
                }
                for (k, v) in params {
                    p.insert(k.clone(), *v);
                }
                emit(
                    self,
                    ChangeOp::PutSeries { series: new },
                    ChangeOp::PutSeries { series: old },
                );
            }
            POp::AddLayer { view, layer, index } => {
                if view != "main" {
                    return Err(Code::NotFound);
                }
                if let Some(s) = &layer.series {
                    if !self.doc.series.iter().any(|x| &x.id == s) {
                        return Err(Code::NotFound);
                    }
                }
                let len = self.main_layers().len() as u32;
                let index = index.unwrap_or(len);
                if index > len {
                    return Err(Code::Invalid);
                }
                emit(
                    self,
                    ChangeOp::InsertLayer {
                        view: view.clone(),
                        index,
                        layer: layer.clone(),
                    },
                    ChangeOp::DeleteLayer {
                        view: view.clone(),
                        index,
                    },
                );
            }
            POp::RemoveLayer { view, index } => {
                let layers = self.main_layers();
                let old = layers.get(*index as usize).ok_or(Code::Invalid)?.clone();
                emit(
                    self,
                    ChangeOp::DeleteLayer {
                        view: view.clone(),
                        index: *index,
                    },
                    ChangeOp::InsertLayer {
                        view: view.clone(),
                        index: *index,
                        layer: old,
                    },
                );
            }
            POp::Remove { id } => {
                if let Some(old) = self.doc.series.iter().find(|s| &s.id == id).cloned() {
                    if self.has_dependents(id) {
                        return Err(Code::InUse);
                    }
                    // Remove the layers that draw it, from the end.
                    let idxs: Vec<usize> = self
                        .main_layers()
                        .iter()
                        .enumerate()
                        .filter(|(_, l)| l.series.as_deref() == Some(id))
                        .map(|(i, _)| i)
                        .rev()
                        .collect();
                    for i in idxs {
                        let layer = self.main_layers()[i].clone();
                        emit(
                            self,
                            ChangeOp::DeleteLayer {
                                view: "main".into(),
                                index: i as u32,
                            },
                            ChangeOp::InsertLayer {
                                view: "main".into(),
                                index: i as u32,
                                layer,
                            },
                        );
                    }
                    emit(
                        self,
                        ChangeOp::Delete { id: id.clone() },
                        ChangeOp::PutSeries { series: old },
                    );
                } else if let Some(old) = self.doc.findings.iter().find(|f| &f.id == id).cloned()
                {
                    if self.doc.ledger.entries.iter().any(|e| &e.finding == id) {
                        return Err(Code::InUse);
                    }
                    let caps: Vec<Capsule> = self
                        .doc
                        .capsules
                        .iter()
                        .filter(|c| &c.finding == id)
                        .cloned()
                        .collect();
                    for c in caps {
                        emit(
                            self,
                            ChangeOp::Delete { id: c.id.clone() },
                            ChangeOp::PutCapsule { capsule: c },
                        );
                    }
                    emit(
                        self,
                        ChangeOp::Delete { id: id.clone() },
                        ChangeOp::PutFinding { finding: old },
                    );
                } else {
                    return Err(Code::NotFound);
                }
            }
            POp::AddFinding {
                id,
                claim,
                facts,
                confidence,
                invalidation,
                capsule,
            } => {
                let id = id.clone().unwrap_or_else(|| format!("fnd-{seq}-{}", created.len()));
                if id_exists(&self.doc, &id) {
                    return Err(Code::Duplicate);
                }
                for f in facts {
                    if !self.doc.facts.iter().any(|x| &x.id == f) {
                        return Err(Code::NotFound);
                    }
                }
                if let Condition::Threshold { series, .. } = invalidation {
                    if !self.doc.series.iter().any(|x| &x.id == series) {
                        return Err(Code::NotFound);
                    }
                }
                if !(0.0..=1.0).contains(confidence) {
                    return Err(Code::Invalid);
                }
                let f = Finding {
                    id: id.clone(),
                    claim: claim.clone(),
                    facts: facts.clone(),
                    confidence: *confidence,
                    invalidation: invalidation.clone(),
                    provenance: Provenance {
                        based_on: facts.clone(),
                        ..prov(who, at)
                    },
                };
                emit(
                    self,
                    ChangeOp::PutFinding { finding: f },
                    ChangeOp::Delete { id: id.clone() },
                );
                created.push(id.clone());
                if let Some(h) = capsule {
                    let cid = format!("cap-{seq}-{}", created.len());
                    let c = Capsule {
                        id: cid.clone(),
                        finding: id,
                        state: CapsuleState::Live,
                        revision: 0,
                        headline: h.clone(),
                        glyphs: vec![],
                        view: None,
                        depth: Depth::Capsule,
                        expires_at: None,
                        provenance: prov(who, at),
                    };
                    emit(
                        self,
                        ChangeOp::PutCapsule { capsule: c },
                        ChangeOp::Delete { id: cid.clone() },
                    );
                    created.push(cid);
                }
            }
            POp::UpdateFinding {
                finding,
                claim,
                confidence,
                invalidation,
            } => {
                let old = self
                    .doc
                    .findings
                    .iter()
                    .find(|f| &f.id == finding)
                    .ok_or(Code::NotFound)?
                    .clone();
                // A decided finding is frozen: changing it would silently change what the
                // decider approved or rejected.
                if self
                    .doc
                    .capsules
                    .iter()
                    .any(|c| &c.finding == finding && c.state != CapsuleState::Live)
                {
                    return Err(Code::Invalid);
                }
                let mut new = old.clone();
                if let Some(c) = claim {
                    new.claim = c.clone();
                }
                if let Some(c) = confidence {
                    new.confidence = *c;
                }
                if let Some(i) = invalidation {
                    new.invalidation = i.clone();
                }
                new.provenance = Provenance {
                    based_on: old.provenance.based_on.clone(),
                    ..prov(who, at)
                };
                emit(
                    self,
                    ChangeOp::PutFinding { finding: new },
                    ChangeOp::PutFinding { finding: old },
                );
            }
            POp::ApproveFinding { finding, note } | POp::RejectFinding { finding, note } => {
                if !who.may_decide() {
                    return Err(Code::Forbidden);
                }
                if !self.doc.findings.iter().any(|f| &f.id == finding) {
                    return Err(Code::NotFound);
                }
                let cap = self
                    .doc
                    .capsules
                    .iter()
                    .find(|c| &c.finding == finding)
                    .ok_or(Code::NotFound)?
                    .clone();
                if cap.state != CapsuleState::Live {
                    return Err(Code::Invalid);
                }
                let state = if matches!(op, POp::ApproveFinding { .. }) {
                    CapsuleState::Approved
                } else {
                    CapsuleState::Rejected
                };
                emit(
                    self,
                    ChangeOp::SetCapsuleState {
                        capsule: cap.id.clone(),
                        state,
                    },
                    ChangeOp::SetCapsuleState {
                        capsule: cap.id.clone(),
                        state: cap.state,
                    },
                );
                emit(
                    self,
                    ChangeOp::PushLedger {
                        entry: LedgerEntry {
                            finding: finding.clone(),
                            state: Some(state),
                            outcome: Outcome::Pending,
                            recorded_at: at,
                            note: note.clone(),
                        },
                    },
                    ChangeOp::PopLedger,
                );
            }
            POp::AddIntent { id, text, trigger } => {
                let id = id
                    .clone()
                    .unwrap_or_else(|| format!("intent-{seq}-{}", created.len()));
                if id_exists(&self.doc, &id) {
                    return Err(Code::Duplicate);
                }
                if let Some(Condition::Threshold { series, .. }) = trigger {
                    if !self.doc.series.iter().any(|x| &x.id == series) {
                        return Err(Code::NotFound);
                    }
                }
                let i = Intent {
                    id: id.clone(),
                    text: text.clone(),
                    trigger: trigger.clone(),
                    active: true,
                    provenance: prov(who, at),
                };
                emit(
                    self,
                    ChangeOp::PutIntent { intent: i },
                    ChangeOp::Delete { id: id.clone() },
                );
                created.push(id);
            }
            _ => return Err(Code::Unsupported),
        }
        Ok(())
    }

    /// All-or-nothing expansion of a command.
    #[allow(clippy::type_complexity)]
    pub fn expand_all(
        &mut self,
        who: Who,
        ops: &[POp],
        seq: Seq,
    ) -> Result<(Vec<ChangeOp>, Vec<ChangeOp>, Vec<String>), Code> {
        let mut ch = Vec::new();
        let mut inv = Vec::new();
        let mut created = Vec::new();
        for op in ops {
            if let Err(e) = self.expand(who, op, seq, &mut created, &mut ch, &mut inv) {
                for i in inv.iter().rev() {
                    self.apply(i);
                }
                return Err(e);
            }
        }
        inv.reverse();
        Ok((ch, inv, created))
    }
}

/// Semantic snapshot of an engine's document.
pub fn to_snap(e: &Engine) -> Snap {
    let d = &e.doc;
    let name = |v: serde_json::Value| v.as_str().unwrap_or_default().to_string();
    let mut s = Snap::default();
    for x in &d.series {
        let ss = match &x.source {
            SeriesSource::Study {
                study,
                inputs,
                params,
                ..
            } => SSeries {
                kind: "study".into(),
                study: study.clone(),
                inputs: inputs.clone(),
                params: params.clone(),
            },
            _ => SSeries {
                kind: "inline".into(),
                ..SSeries::default()
            },
        };
        s.series.insert(x.id.clone(), ss);
    }
    if let Some(v) = d.views.iter().find(|v| v.id == "main") {
        s.layers = v
            .layers
            .iter()
            .map(|l| SLayer {
                mark: name(serde_json::to_value(l.mark).expect("mark")),
                series: l.series.clone(),
                panel: l.panel,
            })
            .collect();
    }
    for f in &d.facts {
        s.facts.insert(f.id.clone(), f.series.clone());
    }
    for f in &d.findings {
        let inval_series = match &f.invalidation {
            Condition::Threshold { series, .. } => series.clone(),
            Condition::Compare { left, .. } => left.clone(),
            Condition::Text { .. } => String::new(),
        };
        s.findings.insert(
            f.id.clone(),
            SFinding {
                claim: f.claim.clone(),
                confidence: f.confidence,
                inval_series,
                facts: f.facts.clone(),
            },
        );
    }
    for c in &d.capsules {
        s.capsules.insert(
            c.id.clone(),
            SCapsule {
                finding: c.finding.clone(),
                state: name(serde_json::to_value(c.state).expect("state")),
            },
        );
    }
    for i in &d.intents {
        let t = match &i.trigger {
            Some(Condition::Threshold { series, .. }) => Some(series.clone()),
            _ => None,
        };
        s.intents.insert(i.id.clone(), t);
    }
    for (l, claim) in d.ledger.entries.iter().zip(&e.decided) {
        s.ledger.push(SLedger {
            finding: l.finding.clone(),
            state: l
                .state
                .map(|x| name(serde_json::to_value(x).expect("state")))
                .unwrap_or_default(),
            decided_claim: claim.clone(),
        });
    }
    s
}

// ---------------------------------------------------------------------------
// Authority
// ---------------------------------------------------------------------------

pub struct Authority {
    pub eng: Engine,
    pub log: Vec<LogEntry>,
    touched: Vec<BTreeSet<String>>,
    pub undone: BTreeSet<Seq>,
    /// Register field -> (seq, writer) of the last write (blind-overwrite accounting).
    last_writer: BTreeMap<String, (Seq, Who)>,
    pub own_excluded: bool,
}

fn touched_of(changes: &[ChangeOp], extra: &[String]) -> BTreeSet<String> {
    let mut t: BTreeSet<String> = extra.iter().cloned().collect();
    for c in changes {
        match c {
            ChangeOp::PutSeries { series } => {
                t.insert(series.id.clone());
            }
            ChangeOp::PutFinding { finding } => {
                t.insert(finding.id.clone());
            }
            ChangeOp::PutCapsule { capsule } => {
                t.insert(capsule.id.clone());
            }
            ChangeOp::PutIntent { intent } => {
                t.insert(intent.id.clone());
            }
            ChangeOp::SetCapsuleState { capsule, .. } => {
                t.insert(capsule.clone());
            }
            ChangeOp::InsertLayer { view, .. } | ChangeOp::DeleteLayer { view, .. } => {
                t.insert(format!("view:{view}"));
            }
            ChangeOp::PushLedger { .. } | ChangeOp::PopLedger => {
                t.insert("ledger".into());
            }
            ChangeOp::Delete { id } => {
                t.insert(id.clone());
            }
            _ => {}
        }
    }
    t
}

pub struct Submitted {
    pub seq: Seq,
}

pub struct Rejected {
    pub code: Code,
    pub conflicts: Vec<Seq>,
}

impl Authority {
    pub fn seq(&self) -> Seq {
        self.log.len() as Seq
    }

    /// Apply one command. `fields`: register fields it writes; `seen`: the writer's last
    /// seen seq (for blind-overwrite accounting only; never used for decisions unless
    /// `base` is given).
    pub fn submit(
        &mut self,
        who: Who,
        cause: &str,
        ops: &[POp],
        base: Option<Seq>,
        fields: &[String],
        seen: Seq,
    ) -> Result<(Submitted, usize), Rejected> {
        let seq = self.seq() + 1;
        if let [POp::Undo { .. }] = ops {
            return self.undo(who, cause, ops).map(|s| (s, 0));
        }
        let (changes, inverse, _created) =
            self.eng
                .expand_all(who, ops, seq)
                .map_err(|code| Rejected {
                    code,
                    conflicts: vec![],
                })?;
        let mut extra = Vec::new();
        for op in ops {
            if let POp::ApproveFinding { finding, .. } | POp::RejectFinding { finding, .. } = op {
                extra.push(finding.clone());
            }
        }
        let touched = touched_of(&changes, &extra);
        if let Some(base) = base {
            let conflicts: Vec<Seq> = self
                .log
                .iter()
                .filter(|e| e.seq > base && !self.undone.contains(&e.seq))
                .filter(|e| !(self.own_excluded && e.author == who.name()))
                .filter(|e| !self.touched[e.seq as usize - 1].is_disjoint(&touched))
                .map(|e| e.seq)
                .collect();
            if !conflicts.is_empty() {
                for i in &inverse {
                    self.eng.apply(i);
                }
                return Err(Rejected {
                    code: Code::Conflict,
                    conflicts,
                });
            }
        }
        let mut blind = 0;
        for f in fields {
            if let Some((s, w)) = self.last_writer.get(f) {
                if *w != who && *s > seen {
                    blind += 1;
                }
            }
            self.last_writer.insert(f.clone(), (seq, who));
        }
        let summary = format!("{} by {}", ops_summary(ops), who.name());
        self.log.push(LogEntry {
            seq,
            at: T0 + seq as i64 * 1000,
            author: who.name().into(),
            approved_by: None,
            cause: Some(cause.into()),
            request: ops.to_vec(),
            changes,
            inverse,
            undoes: None,
            summary,
        });
        self.touched.push(touched);
        Ok((Submitted { seq }, blind))
    }

    fn undo(&mut self, who: Who, cause: &str, ops: &[POp]) -> Result<Submitted, Rejected> {
        let target = self.log.iter().rev().find(|e| {
            (e.author == who.name() || e.approved_by.as_deref() == Some(who.name()))
                && e.undoes.is_none()
                && !self.undone.contains(&e.seq)
        });
        let Some(target) = target else {
            return Err(Rejected {
                code: Code::UndoConflict,
                conflicts: vec![],
            });
        };
        let ts = target.seq;
        let tt = self.touched[ts as usize - 1].clone();
        let conflicts: Vec<Seq> = self
            .log
            .iter()
            .filter(|e| e.seq > ts && !self.undone.contains(&e.seq))
            .filter(|e| !self.touched[e.seq as usize - 1].is_disjoint(&tt))
            .map(|e| e.seq)
            .collect();
        if !conflicts.is_empty() {
            return Err(Rejected {
                code: Code::UndoConflict,
                conflicts,
            });
        }
        let changes = target.inverse.clone();
        let inverse = target.changes.clone();
        for c in &changes {
            self.eng.apply(c);
        }
        let seq = self.seq() + 1;
        self.undone.insert(ts);
        self.log.push(LogEntry {
            seq,
            at: T0 + seq as i64 * 1000,
            author: who.name().into(),
            approved_by: None,
            cause: Some(cause.into()),
            request: ops.to_vec(),
            changes,
            inverse,
            undoes: Some(ts),
            summary: format!("undo {ts} by {}", who.name()),
        });
        self.touched.push(tt);
        Ok(Submitted { seq })
    }
}

/// Kinds of the entries a conflict hit (for the per-kind breakdown).
fn ops_summary_pub(log: &[LogEntry], e: &Rejected) -> String {
    let mut kinds: Vec<String> = e
        .conflicts
        .iter()
        .map(|s| ops_summary(&log[*s as usize - 1].request))
        .collect();
    kinds.sort();
    kinds.dedup();
    kinds.join(",")
}

fn ops_summary(ops: &[POp]) -> String {
    ops.iter()
        .map(|o| {
            serde_json::to_value(o).expect("op")["op"]
                .as_str()
                .unwrap_or("?")
                .to_string()
        })
        .collect::<Vec<_>>()
        .join("+")
}

// ---------------------------------------------------------------------------
// Client side: SemOp -> protocol ops (with re-resolution after a conflict)
// ---------------------------------------------------------------------------

/// Translate a semantic op into protocol ops. With `fresh` (after a re-read), positional
/// targets are re-resolved by identity (anchor / series) instead of the stale index.
pub fn to_pops(op: &SemOp, fresh: Option<&Snap>) -> Option<Vec<POp>> {
    Some(vec![match op {
        SemOp::AddStudy {
            id,
            study,
            params,
            show_panel,
        } => POp::AddStudy {
            id: Some(id.clone()),
            study: study.clone(),
            inputs: vec!["px".into()],
            params: params.clone(),
            show: show_panel.map(|p| Show {
                view: "main".into(),
                with: None,
                panel: Some(p),
            }),
        },
        SemOp::SetParams { series, params } => POp::SetParams {
            series: series.clone(),
            params: params.clone(),
        },
        SemOp::AddLayer {
            series,
            panel,
            index,
            anchor,
        } => {
            let index = match fresh {
                None => *index,
                Some(s) => match anchor {
                    None => s.layers.len() as u32,
                    Some(a) => match s.layers.iter().position(|l| l.series.as_ref() == Some(a)) {
                        Some(p) => p as u32,
                        None => (*index).min(s.layers.len() as u32),
                    },
                },
            };
            POp::AddLayer {
                view: "main".into(),
                layer: Layer {
                    mark: Mark::Line,
                    series: Some(series.clone()),
                    panel: *panel,
                    encoding: BTreeMap::new(),
                },
                index: Some(index),
            }
        }
        SemOp::RemoveLayer {
            index,
            series,
            mark,
        } => {
            let index = match fresh {
                None => *index,
                Some(s) => {
                    // Nearest layer with the same identity.
                    let mut best: Option<(u32, u32)> = None;
                    for (i, l) in s.layers.iter().enumerate() {
                        if &l.series == series && &l.mark == mark {
                            let dist = (i as i64 - *index as i64).unsigned_abs() as u32;
                            if best.is_none_or(|(d, _)| dist < d) {
                                best = Some((dist, i as u32));
                            }
                        }
                    }
                    best?.1
                }
            };
            POp::RemoveLayer {
                view: "main".into(),
                index,
            }
        }
        SemOp::RemoveSeries { id } | SemOp::RemoveFinding { id } => POp::Remove { id: id.clone() },
        SemOp::AddFinding {
            id,
            claim,
            confidence,
            inval_series,
        } => POp::AddFinding {
            id: Some(id.clone()),
            claim: claim.clone(),
            facts: vec![FACT.into()],
            confidence: *confidence,
            invalidation: Condition::Threshold {
                series: inval_series.clone(),
                op: CompareOp::Gt,
                value: 80.0,
            },
            capsule: Some(format!("{claim} (kapsül)")),
        },
        SemOp::UpdateFinding {
            finding,
            claim,
            confidence,
        } => POp::UpdateFinding {
            finding: finding.clone(),
            claim: claim.clone(),
            confidence: *confidence,
            invalidation: None,
        },
        SemOp::Decide {
            finding,
            approve,
            note,
        } => {
            if *approve {
                POp::ApproveFinding {
                    finding: finding.clone(),
                    note: Some(note.clone()),
                }
            } else {
                POp::RejectFinding {
                    finding: finding.clone(),
                    note: Some(note.clone()),
                }
            }
        }
        SemOp::AddIntent {
            id,
            text,
            trigger_series,
        } => POp::AddIntent {
            id: Some(id.clone()),
            text: text.clone(),
            trigger: Some(Condition::Threshold {
                series: trigger_series.clone(),
                op: CompareOp::Gt,
                value: 70.0,
            }),
        },
        SemOp::Undo => POp::Undo { seq: None },
    }])
}

struct Client {
    mirror: Engine,
    seq: Seq,
    /// While offline: the base seq, an optimistic local copy and the queue.
    offline: Option<(Seq, Engine, Vec<SemOp>)>,
}

// ---------------------------------------------------------------------------
// The model
// ---------------------------------------------------------------------------

/// `BASE`: clients send `base`. `OWN`: the conflict check ignores the sender's own
/// entries (a proposed refinement of §8.2; the spec as written is `OWN = false`).
pub struct LogModel<const BASE: bool, const OWN: bool> {
    auth: Authority,
    clients: [Client; 3],
    wire: Wire,
    undo: UndoStats,
    extra: BTreeMap<String, usize>,
    req: u64,
    start: Document,
}

impl<const BASE: bool, const OWN: bool> LogModel<BASE, OWN> {
    fn next_id(&mut self) -> String {
        self.req += 1;
        format!("c{}", self.req)
    }

    fn reply_bytes(id: &str, r: &Result<(Submitted, usize), Rejected>) -> usize {
        let msg = match r {
            Ok((s, _)) => ServerMsg::Ack {
                id: id.into(),
                status: qp_schema::protocol::AckStatus::Applied,
                seq: Some(s.seq),
                created: vec![],
                proposal: None,
            },
            Err(e) => ServerMsg::Error {
                id: Some(id.into()),
                error: ProtocolError {
                    code: code_of(e.code),
                    message: format!("{:?}", e.code),
                    path: Some("/ops/0".into()),
                    expected: vec![],
                    did_you_mean: vec![],
                    conflicts: e.conflicts.clone(),
                    hint: None,
                },
            },
        };
        serde_json::to_vec(&msg).expect("reply").len()
    }

    /// Send one command; returns the result and the bytes sent.
    fn send(
        &mut self,
        who: Who,
        ops: Vec<POp>,
        base: Option<Seq>,
        fields: &[String],
        seen: Seq,
    ) -> (Result<(Submitted, usize), Rejected>, usize) {
        let id = self.next_id();
        let msg = ClientMsg::Command {
            id: id.clone(),
            ops,
            mode: WriteMode::Auto,
            base,
            reason: None,
        };
        let bytes = serde_json::to_vec(&msg).expect("cmd");
        // The authority parses what arrives on the wire.
        let ClientMsg::Command { ops, base, .. } =
            serde_json::from_slice::<ClientMsg>(&bytes).expect("parse")
        else {
            unreachable!()
        };
        let r = self.auth.submit(who, &id, &ops, base, fields, seen);
        self.wire.replies += Self::reply_bytes(&id, &r);
        match &r {
            Ok((_, blind)) => {
                *self.extra.entry("blind_overwrites".into()).or_default() += blind;
            }
            Err(e) if e.code == Code::Conflict => {
                *self.extra.entry("conflicts".into()).or_default() += 1;
                *self
                    .extra
                    .entry(format!("conflicts:{} vs {}", ops_summary(&ops), ops_summary_pub(&self.auth.log, e)))
                    .or_default() += 1;
                let own = e.conflicts.iter().all(|s| {
                    self.auth.log[*s as usize - 1].author == who.name()
                });
                if own {
                    *self.extra.entry("conflicts_with_own_entries".into()).or_default() += 1;
                }
            }
            Err(_) => {
                *self.extra.entry("validation_errors".into()).or_default() += 1;
            }
        }
        (r, bytes.len())
    }

    /// Deliver entries after the client's seq as `notify` messages.
    fn deliver(&mut self, who: Who) {
        let from = self.clients[who.idx()].seq as usize;
        for e in &self.auth.log[from..] {
            let msg = ServerMsg::Notify {
                sub: "changes".into(),
                event: Event::Change {
                    seq: e.seq,
                    at: e.at,
                    author: e.author.clone(),
                    approved_by: e.approved_by.clone(),
                    cause: e.cause.clone(),
                    changes: e.changes.clone(),
                },
            };
            let bytes = serde_json::to_vec(&msg).expect("notify");
            self.wire.down.push(bytes.len());
            let ServerMsg::Notify {
                event: Event::Change { changes, seq, .. },
                ..
            } = serde_json::from_slice::<ServerMsg>(&bytes).expect("parse")
            else {
                unreachable!()
            };
            let c = &mut self.clients[who.idx()];
            assert_eq!(seq, c.seq + 1, "gapless");
            for ch in &changes {
                c.mirror.apply(ch);
            }
            c.seq = seq;
        }
    }

    /// Catch up through a `log` query (reconnect path). Returns bytes received.
    fn catch_up_by_query(&mut self, who: Who) -> usize {
        let from = self.clients[who.idx()].seq as usize;
        let msg = ServerMsg::Result {
            id: "q-catchup".into(),
            seq: self.auth.seq(),
            data: QueryResult::Log {
                entries: self.auth.log[from..].to_vec(),
            },
        };
        let bytes = serde_json::to_vec(&msg).expect("log");
        let ServerMsg::Result {
            data: QueryResult::Log { entries },
            ..
        } = serde_json::from_slice::<ServerMsg>(&bytes).expect("parse")
        else {
            unreachable!()
        };
        let c = &mut self.clients[who.idx()];
        for e in entries {
            for ch in &e.changes {
                c.mirror.apply(ch);
            }
            c.seq = e.seq;
        }
        bytes.len()
    }

    /// Send with the client's policy: on conflict or a validation error, re-read and
    /// re-resolve the intent by identity, validate locally, resend (at most twice).
    fn send_with_retry(&mut self, who: Who, op: &SemOp, base: Seq, catch_up_query: bool) -> (Exp, usize, usize) {
        let fields = op.written_fields();
        let mut up = 0;
        let mut down = 0;
        let mut codes = Vec::new();
        let Some(ops) = to_pops(op, None) else {
            return (Exp::Refused(vec![Code::NotFound]), 0, 0);
        };
        let (r, b) = self.send(who, ops, BASE.then_some(base), &fields, base);
        up += b;
        let mut r = r;
        for _attempt in 0..2 {
            match r {
                Ok(_) => {
                    return (
                        if codes.is_empty() {
                            Exp::Applied
                        } else {
                            Exp::AppliedAfterRetry(codes)
                        },
                        up,
                        down,
                    )
                }
                Err(e) => {
                    codes.push(e.code);
                    if e.code == Code::UndoConflict || e.code == Code::Forbidden {
                        return (Exp::Refused(codes), up, down);
                    }
                    if catch_up_query {
                        down += self.catch_up_by_query(who);
                    } else {
                        self.deliver(who);
                    }
                    let view = to_snap(&self.clients[who.idx()].mirror);
                    if let Err(c) = validate(&view, who, op) {
                        codes.push(c);
                        return (Exp::Refused(codes), up, down);
                    }
                    let Some(ops) = to_pops(op, Some(&view)) else {
                        codes.push(Code::NotFound);
                        return (Exp::Refused(codes), up, down);
                    };
                    let seen = self.clients[who.idx()].seq;
                    let (r2, b) = self.send(who, ops, BASE.then_some(seen), &fields, seen);
                    up += b;
                    r = r2;
                }
            }
        }
        match r {
            Ok(_) => (Exp::AppliedAfterRetry(codes), up, down),
            Err(e) => {
                codes.push(e.code);
                (Exp::Refused(codes), up, down)
            }
        }
    }
}

impl<const BASE: bool, const OWN: bool> LogModel<BASE, OWN> {
    /// Provenance from the log: author (and approver) of the latest live entry that
    /// wrote `field` of item `id` (`field` = a parameter name, `claim`, `state`).
    pub fn writer_of(&self, id: &str, field: &str) -> String {
        for e in self.auth.log.iter().rev() {
            if self.auth.undone.contains(&e.seq) || e.undoes.is_some() {
                continue;
            }
            let hit = e.request.iter().any(|op| match op {
                POp::SetParams { series, params } => series == id && params.contains_key(field),
                POp::AddStudy { id: Some(x), .. } => x == id,
                POp::UpdateFinding {
                    finding,
                    claim,
                    confidence,
                    ..
                } => {
                    finding == id
                        && ((field == "claim" && claim.is_some())
                            || (field == "confidence" && confidence.is_some()))
                }
                POp::AddFinding { id: Some(x), .. } => x == id,
                POp::ApproveFinding { finding, .. } | POp::RejectFinding { finding, .. } => {
                    finding == id && field == "state"
                }
                _ => false,
            });
            if hit {
                return match &e.approved_by {
                    Some(a) => format!("{} (approved by {a}) @seq {}", e.author, e.seq),
                    None => format!("{} @seq {}", e.author, e.seq),
                };
            }
        }
        "initial document".into()
    }
}

impl<const BASE: bool, const OWN: bool> Model for LogModel<BASE, OWN> {
    const NAME: &'static str = match (BASE, OWN) {
        (true, true) => "A-log(base,own-excl)",
        (true, false) => "A-log(base,strict)",
        (false, _) => "A-log(no-base)",
    };

    fn new(doc: &Document) -> Self {
        let eng = Engine {
            doc: doc.clone(),
            decided: vec![String::new(); doc.ledger.entries.len()],
        };
        let client = || Client {
            mirror: eng.clone(),
            seq: 0,
            offline: None,
        };
        LogModel {
            auth: Authority {
                eng: eng.clone(),
                log: Vec::new(),
                touched: Vec::new(),
                undone: BTreeSet::new(),
                last_writer: BTreeMap::new(),
                own_excluded: OWN,
            },
            clients: [client(), client(), client()],
            wire: Wire::default(),
            undo: UndoStats::default(),
            extra: BTreeMap::new(),
            req: 0,
            start: doc.clone(),
        }
    }

    fn view(&self, who: Who) -> Snap {
        let c = &self.clients[who.idx()];
        match &c.offline {
            Some((_, local, _)) => to_snap(local),
            None => to_snap(&c.mirror),
        }
    }

    fn write(&mut self, who: Who, _it: &WIntent, op: &SemOp, online: bool, _view: &Snap) -> Exp {
        if !online {
            // Offline: apply optimistically to a local copy and queue.
            let c = &mut self.clients[who.idx()];
            if c.offline.is_none() {
                c.offline = Some((c.seq, c.mirror.clone(), Vec::new()));
            }
            let (base, local, queue) = c.offline.as_mut().expect("offline");
            if let Some(ops) = to_pops(op, None) {
                if local.expand_all(who, &ops, *base + 1).is_ok() {
                    queue.push(op.clone());
                    return Exp::Queued;
                }
            }
            return Exp::Refused(vec![Code::Invalid]);
        }
        if matches!(op, SemOp::Undo) {
            self.undo.attempted += 1;
        }
        let base = self.clients[who.idx()].seq;
        let (exp, up, _down) = self.send_with_retry(who, op, base, false);
        self.wire.up.push(up);
        if matches!(op, SemOp::Undo) {
            match &exp {
                Exp::Applied | Exp::AppliedAfterRetry(_) => self.undo.applied += 1,
                Exp::Refused(cs) if cs.contains(&Code::UndoConflict) => {
                    // No target at all is also reported as undo_conflict (no conflicts).
                    let last = self.auth.log.last();
                    let _ = last;
                    self.undo.refused_conflict += 1
                }
                _ => self.undo.nothing_or_unsupported += 1,
            }
        }
        exp
    }

    fn end_round(&mut self, online: [bool; 3]) {
        for who in crate::common::ALL {
            if online[who.idx()] && self.clients[who.idx()].offline.is_none() {
                self.deliver(who);
            }
        }
    }

    fn reconnect(&mut self, who: Who) -> Reconnect {
        let mut rc = Reconnect::default();
        let Some((base, _local, queue)) = self.clients[who.idx()].offline.take() else {
            return rc;
        };
        rc.queued = queue.len();
        rc.round_trips = 0;
        let replies_before = self.wire.replies;
        for op in &queue {
            // Queued ops were composed against the offline view: they keep the old base.
            let (exp, up, down) = self.send_with_retry(who, op, base, true);
            rc.bytes_up += up;
            rc.bytes_down += down;
            self.wire.up.push(up);
            rc.round_trips += 1 + match &exp {
                Exp::AppliedAfterRetry(cs) | Exp::Refused(cs) => cs.len(),
                _ => 0,
            };
            match &exp {
                Exp::Applied => rc.applied += 1,
                Exp::AppliedAfterRetry(cs) => {
                    rc.applied_after_retry += 1;
                    for c in cs {
                        *rc.codes.entry(format!("{c:?}")).or_default() += 1;
                    }
                }
                Exp::Refused(cs) => {
                    rc.refused += 1;
                    for c in cs {
                        *rc.codes.entry(format!("{c:?}")).or_default() += 1;
                    }
                }
                Exp::Queued => {}
            }
            rc.details.push(format!("{}: {:?}", op.kind(), exp));
        }
        rc.bytes_down += self.wire.replies - replies_before;
        // Alternatives a client could choose for the catch-up (reported, not used):
        // the missed `change` notifications only, or a fresh `document` snapshot.
        let from = self.clients[who.idx()].seq as usize;
        let changes_only: usize = self.auth.log[from..]
            .iter()
            .map(|e| {
                serde_json::to_vec(&ServerMsg::Notify {
                    sub: "changes".into(),
                    event: Event::Change {
                        seq: e.seq,
                        at: e.at,
                        author: e.author.clone(),
                        approved_by: e.approved_by.clone(),
                        cause: e.cause.clone(),
                        changes: e.changes.clone(),
                    },
                })
                .expect("n")
                .len()
            })
            .sum();
        let snapshot = serde_json::to_vec(&self.auth.eng.doc).expect("doc").len();
        self.extra.insert("reconnect_final_catchup_changes_only_bytes".into(), changes_only);
        self.extra.insert("reconnect_final_catchup_snapshot_bytes".into(), snapshot);
        // Final catch-up of everything not yet seen.
        let q = self.catch_up_by_query(who);
        self.extra.insert("reconnect_final_catchup_log_query_bytes".into(), q);
        rc.bytes_down += q;
        rc.round_trips += 1;
        rc
    }

    fn snaps(&self) -> Vec<Snap> {
        let mut v = vec![to_snap(&self.auth.eng)];
        for c in &self.clients {
            v.push(to_snap(&c.mirror));
        }
        // Byte-level check of the replay invariant (§2): mirrors equal the authority.
        let a = serde_json::to_vec(&self.auth.eng.doc).expect("doc");
        for c in &self.clients {
            assert_eq!(a, serde_json::to_vec(&c.mirror.doc).expect("doc"));
        }
        v
    }

    fn history_bytes(&mut self) -> Vec<u8> {
        // Replay invariant (§2): start + changes in order == authority document, bytewise.
        let mut replay = Engine {
            doc: self.start.clone(),
            decided: vec![String::new(); self.start.ledger.entries.len()],
        };
        for e in &self.auth.log {
            for c in &e.changes {
                replay.apply(c);
            }
        }
        assert_eq!(
            serde_json::to_vec(&replay.doc).expect("doc"),
            serde_json::to_vec(&self.auth.eng.doc).expect("doc"),
            "replay invariant"
        );
        // Start document + full log (with inverses) = what persists with history.
        let mut b = serde_json::to_vec(&self.start).expect("doc");
        b.extend(serde_json::to_vec(&self.auth.log).expect("log"));
        b
    }

    fn state_bytes(&mut self) -> usize {
        serde_json::to_vec(&self.auth.eng.doc).expect("doc").len()
    }

    fn wire(&self) -> &Wire {
        &self.wire
    }

    fn undo_stats(&self) -> &UndoStats {
        &self.undo
    }

    fn extra(&self) -> BTreeMap<String, usize> {
        let mut e = self.extra.clone();
        e.insert("log_entries".into(), self.auth.log.len());
        e
    }
}
