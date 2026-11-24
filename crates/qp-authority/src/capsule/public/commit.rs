//! The public world's entries: reversed changes, public entries, invalidations and intents
//! in public, atomic application and commit.

use super::*;

impl Session {
    /// (In the authority's world, R3/K1b) a public-view writer's changes `changes`
    /// (inverse `inverse`, of the public world, not yet applied there) as the
    /// authority's document may take them, **all or nothing**: `None` when any change
    /// names an item the two worlds do not hold alike — an engine item (fact with a
    /// meaning, engine finding, capsule with a story) of either document, or an id the
    /// two documents hold in different collections or only one of them holds — or when
    /// the translation would drop or empty a change. Only people's own items (new
    /// ones, or ones both worlds hold) pass, so the authority's items are never written
    /// from the public world. The caller applies the result atomically.
    pub(crate) fn reverse_changes(
        &self,
        changes: &[ChangeOp],
        inverse: &[ChangeOp],
    ) -> Option<Vec<ChangeOp>> {
        let w = self.other.as_ref()?;
        let p = Proj {
            hidden: only_in(&w.doc, &self.doc),
            ..Proj::default()
        };
        let engine_item = |d: &Document, id: &str| {
            d.facts.iter().any(|f| f.id == id && f.meaning.is_some())
                || d.findings
                    .iter()
                    .any(|f| f.id == id && engine_author(&f.provenance.author))
                || d.capsules.iter().any(|c| c.id == id && c.story.is_some())
        };
        // (R3/K1c) Alike: absent from both documents (truly new), or present in both as
        // the same item, byte for byte (canonical JSON) — and never an engine item or a
        // proposal.
        let alike = |id: &str| {
            let a = qp_ops::apply::find_collection(&self.doc, id);
            let b = qp_ops::apply::find_collection(&w.doc, id);
            if a != b
                || a == Some(qp_schema::protocol::Collection::Proposals)
                || engine_item(&self.doc, id)
                || engine_item(&w.doc, id)
            {
                return false;
            }
            a.is_none() || item_json(&self.doc, id) == item_json(&w.doc, id)
        };
        let mut out = Vec::with_capacity(changes.len());
        for c in changes {
            let ok = match c {
                // A public-view person's proposals and their decisions live in the public
                // world only.
                ChangeOp::PutProposal { .. } => false,
                ChangeOp::PushLedger { entry } => {
                    alike(&entry.finding) && entry.capsule.as_deref().is_none_or(alike)
                }
                ChangeOp::PutAudience { principal, .. } => {
                    self.doc.audiences.get(principal) == w.doc.audiences.get(principal)
                }
                _ => alike(qp_ops::apply::touched(c)),
            };
            if !ok {
                return None;
            }
            let t = translate(c, inverse, &self.doc, &p)?;
            if let ChangeOp::PutFinding { finding, .. } = &t {
                if finding.facts.is_empty() {
                    return None;
                }
            }
            out.push(t);
        }
        Some(out)
    }

    /// The public world's entry for authority entry `e` (about to be logged): the
    /// writer's own ([`PubNext`]), or — when it made none — the entry itself when it only
    /// writes series data (bars, an anchor stamp), else nothing.
    pub(crate) fn public_entry(&mut self, e: &LogEntry) -> Option<LogEntry> {
        let base = LogEntry {
            reason: e.reason.clone(),
            seq: 0,
            at: e.at,
            author: e.author.clone(),
            approved_by: e.approved_by.clone(),
            cause: e.cause.clone(),
            request: e.request.clone(),
            changes: Vec::new(),
            inverse: Vec::new(),
            undoes: None,
            summary: e.summary.clone(),
            evidence: None,
        };
        match self.pending_pub.take() {
            Some(PubNext::Skip) => None,
            Some(PubNext::Entry(p)) => Some(LogEntry {
                request: p.request.unwrap_or_else(|| base.request.clone()),
                changes: p.changes,
                summary: p.summary.unwrap_or(base.summary.clone()),
                evidence: p.evidence,
                undoes: p.undoes,
                ..base
            }),
            None => {
                let series = |c: &ChangeOp| bar_op(c) || matches!(c, ChangeOp::PutSeries { .. });
                let data = e.changes.iter().chain(&e.inverse).all(series)
                    && (e.author == qp_ops::ENGINE_PRINCIPAL
                        || e.changes.iter().chain(&e.inverse).all(bar_op));
                data.then(|| LogEntry {
                    changes: e.changes.clone(),
                    evidence: e.evidence.clone(),
                    ..base
                })
            }
        }
    }

    /// (R3/K1b) §8.1 invalidations of the public world's own capsules (people's, without
    /// a story), evaluated in the public world with the rules of
    /// [`Session::invalidate_these`]: each one that holds is one public entry (author
    /// `qp`, `set_capsule_state`, summary and evidence); the authority's world writes
    /// nothing. Returns the notifications.
    pub(crate) fn invalidate_in_public(&mut self) -> Vec<qp_schema::protocol::OutboundLine> {
        if self.pub_caps.is_empty() || self.other.is_none() || self.in_public {
            return Vec::new();
        }
        enum Look {
            Gone,
            Quiet,
            Holds(String, Option<ClaimEvidence>),
        }
        let mut lines = Vec::new();
        let ids: Vec<String> = self.pub_caps.iter().cloned().collect();
        for cap in ids {
            let look = self.in_world(true, |s| {
                let Some(c) = s.doc_capsule(&cap) else {
                    return Look::Gone;
                };
                if c.story.is_some() {
                    return Look::Gone;
                }
                if c.state != qp_schema::CapsuleState::Live {
                    return Look::Quiet;
                }
                let fnd = c.finding.clone();
                let Some(f) = s.doc.findings.iter().find(|f| f.id == fnd) else {
                    return Look::Quiet;
                };
                let cond = f.invalidation.clone();
                if s.cond_blocked(&cond) {
                    return Look::Quiet;
                }
                let closure = qp_ops::live::closure_of(&s.doc, &cond);
                let who = s.responsible(f, &closure);
                if !who
                    .iter()
                    .all(|p| crate::live::hears(&s.plane, &s.doc, p, &cond))
                {
                    return Look::Quiet;
                }
                let rd = s.rd.as_ref().unwrap_or(&s.doc);
                if !qp_plane::values::holds(rd, &mut s.cache, &cond) {
                    return Look::Quiet;
                }
                let ev = crate::claims::evidence(rd, &mut s.cache, &s.claims, &s.plane, &cond);
                Look::Holds(fnd, ev)
            });
            match look {
                Look::Gone => {
                    self.pub_caps.remove(&cap);
                }
                Look::Quiet => {}
                Look::Holds(fnd, evidence) => {
                    let p = self.commit_public(LogEntry {
                        reason: None,
                        seq: 0,
                        at: self.clock,
                        author: qp_ops::ENGINE_PRINCIPAL.to_owned(),
                        approved_by: None,
                        cause: None,
                        request: Vec::new(),
                        changes: vec![ChangeOp::SetCapsuleState {
                            capsule: cap.clone(),
                            state: qp_schema::CapsuleState::Invalidated,
                        }],
                        inverse: Vec::new(),
                        undoes: None,
                        summary: format!("invalidate {cap}: invalidation of {fnd} holds"),
                        evidence,
                    });
                    if let Some(p) = p {
                        lines.extend(self.entry_events_pub(p));
                    }
                }
            }
        }
        lines
    }

    /// (R3/K1c) The public world's intents, evaluated there (its own intents, active
    /// flags and edge state; the rules of `eval_intents`, without the authority's claim
    /// registry): the `intent_triggered` events, built from the public document, to the
    /// public-view connections subscribed to `intents`, under the public sequence number.
    pub(crate) fn eval_intents_public(&mut self) -> Vec<qp_schema::protocol::OutboundLine> {
        use qp_schema::protocol::{Event, ServerMsg, Topic};
        if self.other.is_none() || self.in_public {
            return Vec::new();
        }
        self.in_world(true, |s| {
            let seq = s.seq();
            let mut fired = Vec::new();
            let mut state = BTreeMap::new();
            let intents: Vec<_> = s
                .doc
                .intents
                .iter()
                .filter(|i| i.active)
                .filter_map(|i| i.trigger.clone().map(|t| (i.id.clone(), t)))
                .collect();
            for (id, trig) in intents {
                if matches!(trig, qp_schema::Condition::Text { .. }) {
                    continue;
                }
                if s.cond_blocked(&trig) {
                    if let Some(prev) = s.intent_state.get(&id) {
                        state.insert(id, *prev);
                    }
                    continue;
                }
                let rd = s.rd.as_ref().unwrap_or(&s.doc);
                let r = qp_plane::values::eval_condition(rd, &mut s.cache, &trig);
                let now = r.is_some_and(|x| x.0);
                if s.intent_state.get(&id) == Some(&false) && now {
                    if let Some((_, bar, t)) = r {
                        let rd = s.rd.as_ref().unwrap_or(&s.doc);
                        let evidence =
                            crate::claims::evidence(rd, &mut s.cache, &s.claims, &s.plane, &trig);
                        fired.push((
                            trig.clone(),
                            Event::IntentTriggered {
                                intent: id.clone(),
                                bar,
                                t,
                                seq,
                                evidence,
                            },
                        ));
                    }
                }
                state.insert(id, now);
            }
            s.intent_state = state;
            s.intent_epoch += 1;
            let mut lines = Vec::new();
            for (trig, ev) in fired {
                for c in &s.conns {
                    if !s.public_view(&c.principal)
                        || !crate::live::hears(&s.plane, &s.doc, &c.principal, &trig)
                    {
                        continue;
                    }
                    for sub in &c.subs {
                        if matches!(sub.topic, Topic::Intents {}) {
                            lines.push(crate::session::out(
                                &c.id,
                                ServerMsg::Notify {
                                    sub: sub.id.clone(),
                                    event: crate::claims::event_for(
                                        &s.doc,
                                        &s.plane,
                                        &c.principal,
                                        &ev,
                                    ),
                                },
                            ));
                        }
                    }
                }
            }
            lines
        })
    }

    /// (R3/K1b) Applies `changes` to the document the fields hold **atomically**: all of
    /// them (returned with their inverse) or, when one does not fit, none (the document
    /// is rolled back; `None`): never a partial entry.
    pub(crate) fn apply_atomic(
        &mut self,
        changes: Vec<ChangeOp>,
    ) -> Option<(Vec<ChangeOp>, Vec<ChangeOp>)> {
        let restore = crate::live::Restore {
            initial: &self.initial,
            log: &self.log,
        };
        crate::live::apply_in_place(&mut self.doc, &changes, restore)
            .ok()
            .map(|inverse| (changes, inverse))
    }

    /// Applies `pe` to the public world and logs it there with the next public sequence
    /// number, atomically: when its changes do not fit the public document, nothing is
    /// logged (`None`). Returns its public sequence number.
    pub(crate) fn commit_public(&mut self, mut pe: LogEntry) -> Option<Seq> {
        let r = self.in_world(true, |s| {
            let (changes, inverse) = s.apply_atomic(std::mem::take(&mut pe.changes))?;
            pe.changes = changes;
            pe.inverse = inverse;
            pe.seq = s.log.len() as Seq + 1;
            let series_set = pe
                .changes
                .iter()
                .any(|c| matches!(c, ChangeOp::PutSeries { .. } | ChangeOp::Delete { .. }));
            if series_set {
                s.rd = reading_doc(&s.doc, &s.plane);
                s.view_reads = None;
            } else if let Some(rd) = s.rd.as_mut() {
                if qp_ops::apply::apply_all(rd, &pe.changes).is_err() {
                    s.rd = reading_doc(&s.doc, &s.plane);
                    s.head_state.dirty_all = true;
                }
            }
            if !pe.changes.iter().all(crate::live::keeps_view_reads) {
                s.view_reads = None;
            }
            s.note_writers(&pe);
            let seq = pe.seq;
            let caps: Vec<(String, bool)> = pe
                .changes
                .iter()
                .filter_map(|c| match c {
                    ChangeOp::PutCapsule { capsule, .. } if capsule.story.is_none() => {
                        Some((capsule.id.clone(), true))
                    }
                    ChangeOp::Delete { id } => Some((id.clone(), false)),
                    _ => None,
                })
                .collect();
            s.log.push(pe);
            Some((seq, caps))
        });
        let (seq, caps) = r?;
        for (c, put) in caps {
            if put {
                self.pub_caps.insert(c);
            } else {
                self.pub_caps.remove(&c);
            }
        }
        self.note_pub_len();
        Some(seq)
    }

    /// Records the public log's length for the authority log's current length.
    pub(crate) fn note_pub_len(&mut self) {
        let full = self.log.len();
        let p = self.pub_len();
        if self.pub_at.len() < full + 1 {
            self.pub_at.resize(full + 1, p);
        }
        self.pub_at[full] = p;
    }

    /// Notifications of public entry `p` to the public-view connections.
    pub(crate) fn entry_events_pub(&mut self, p: Seq) -> Vec<qp_schema::protocol::OutboundLine> {
        let parts = self.in_world(true, |s| s.entry_events_in(p, true));
        parts.into_iter().flatten().map(|(_, l)| l).collect()
    }
}
