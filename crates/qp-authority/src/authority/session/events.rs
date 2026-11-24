//! After an entry: invalidations, watchers, intents and entry events.

use super::*;

impl Session {
    /// Engine invalidations triggered by entry `seq` (§8.1): after a change of series
    /// data or study parameters, every live capsule; after a finding change, that
    /// finding's live capsules.
    pub(super) fn invalidations(&mut self, seq: Seq) -> Vec<Seq> {
        let entry = &self.log[usize::try_from(seq - 1).unwrap_or(0)];
        let all = entry.changes.iter().any(|c| {
            matches!(
                c,
                ChangeOp::PutSeries { .. }
                    | ChangeOp::AppendBars { .. }
                    | ChangeOp::TruncateBars { .. }
                    | ChangeOp::SetLastBar { .. }
            )
        });
        let findings: BTreeSet<String> = entry
            .changes
            .iter()
            .filter_map(|c| match c {
                ChangeOp::PutFinding { finding, .. } => Some(finding.id.clone()),
                _ => None,
            })
            .collect();
        // Capsules the entry makes live (an undo of an invalidation, a capsule put back):
        // evaluated now, so that a capsule never stays live with a holding invalidation
        // until a series it rests on ticks (RP2 T5 audit O1).
        let revived: BTreeSet<String> = entry
            .changes
            .iter()
            .filter_map(|c| match c {
                ChangeOp::SetCapsuleState {
                    capsule,
                    state: CapsuleState::Live,
                } => Some(capsule.clone()),
                ChangeOp::PutCapsule { capsule, .. } if capsule.state == CapsuleState::Live => {
                    Some(capsule.id.clone())
                }
                _ => None,
            })
            .collect();
        if !all && findings.is_empty() && revived.is_empty() {
            return Vec::new();
        }
        let candidates: Vec<(String, String)> = self
            .doc
            .capsules
            .iter()
            .filter(|c| {
                c.state == CapsuleState::Live
                    && (all || findings.contains(&c.finding) || revived.contains(&c.id))
            })
            .map(|c| (c.id.clone(), c.finding.clone()))
            .collect();
        self.invalidate_these(candidates)
    }

    /// Engine entries invalidating live capsules (all, or those of `findings`) whose
    /// invalidation holds now. A condition resting on a bound series that is not ready
    /// is not evaluated (§18.3).
    pub(super) fn invalidate(&mut self, all: bool, findings: &BTreeSet<String>) -> Vec<Seq> {
        let candidates: Vec<(String, String)> = self
            .doc
            .capsules
            .iter()
            .filter(|c| c.state == CapsuleState::Live && (all || findings.contains(&c.finding)))
            .map(|c| (c.id.clone(), c.finding.clone()))
            .collect();
        self.invalidate_these(candidates)
    }

    /// Engine entries invalidating those of `candidates` (capsule, finding) whose
    /// invalidation holds now (not evaluated when resting on a bound series that is not
    /// ready).
    pub(crate) fn invalidate_these(&mut self, candidates: Vec<(String, String)>) -> Vec<Seq> {
        let mut out = Vec::new();
        for (cap, fnd) in candidates {
            // (R3, §20.1) A capsule of the running engine (a story on a watched series)
            // ends by the outcome rule only, never by a document condition (a person's
            // `update_finding` cannot make one). (R3/K1) So no invalidation entry is ever
            // about an engine item, whose world differs between the authority and the
            // public world. (A story capsule of a document without an engine keeps §8.1.)
            if self.runs_capsule(&cap) {
                continue;
            }
            let Some(f) = self.doc.findings.iter().find(|f| f.id == fnd) else {
                continue;
            };
            let cond = f.invalidation.clone();
            if self.cond_blocked(&cond) {
                continue;
            }
            // A principal of this session without the data entitlement of the feeds its
            // condition reads would learn them from its capsule's state (§18.3): its
            // condition is not evaluated unless every principal responsible for it (the
            // author, whoever last wrote the invalidation, whoever last wrote a series it
            // rests on) is entitled (e.g. its series was rebound after it was written, an
            // undo brought a binding back, an editor rewrote the invalidation).
            let closure = qp_ops::live::closure_of(&self.doc, &cond);
            let who = self.responsible(f, &closure);
            if !who
                .iter()
                .all(|p| crate::live::hears(&self.plane, &self.doc, p, &cond))
            {
                continue;
            }
            if self.registry.holds(rd!(self), &mut self.cache, &cond) {
                let changes = vec![ChangeOp::SetCapsuleState {
                    capsule: cap.clone(),
                    state: CapsuleState::Invalidated,
                }];
                let ev = crate::claims::evidence(
                    rd!(self),
                    &mut self.cache,
                    &self.claims,
                    &self.plane,
                    &cond,
                );
                if let Some(s) = self.engine_entry_with(
                    changes,
                    format!("invalidate {cap}: invalidation of {fnd} holds"),
                    ev.clone(),
                ) {
                    out.push(s);
                    if let Some(ev) = ev {
                        self.claims.assert(
                            crate::claims::Owner::Doc,
                            qp_schema::protocol::ClaimRef::Capsule {
                                capsule: cap.clone(),
                                finding: fnd.clone(),
                            },
                            cond,
                            ev,
                        );
                    }
                }
            }
        }
        out
    }

    /// Edge-triggered condition subscriptions and intents after entry `seq`.
    pub(super) fn eval_watchers(&mut self, seq: Seq) -> (Vec<OutboundLine>, Vec<OutboundLine>) {
        let mut conds = Vec::new();
        for ci in 0..self.conns.len() {
            for si in 0..self.conns[ci].subs.len() {
                let Topic::Condition { condition } = &self.conns[ci].subs[si].topic else {
                    continue;
                };
                let condition = condition.clone();
                if !crate::live::hears(
                    &self.plane,
                    &self.doc,
                    &self.conns[ci].principal,
                    &condition,
                ) {
                    // Not evaluated for a subscriber without the data entitlement (§18.3).
                    continue;
                }
                if self.cond_blocked(&condition) {
                    // Not evaluated; the edge state is kept (§18.3).
                    continue;
                }
                let r = self
                    .registry
                    .eval_condition(rd!(self), &mut self.cache, &condition);
                let now = r.is_some_and(|x| x.0);
                let prev = self.conns[ci].subs[si].last.replace(now);
                if prev == Some(false) && now {
                    if let Some((_, bar, t)) = r {
                        let evidence = self.claim_asserted(
                            crate::claims::Owner::Conn(self.conns[ci].id.clone()),
                            qp_schema::protocol::ClaimRef::Condition {
                                sub: self.conns[ci].subs[si].id.clone(),
                            },
                            &condition,
                        );
                        conds.push(out(
                            &self.conns[ci].id,
                            ServerMsg::Notify {
                                sub: self.conns[ci].subs[si].id.clone(),
                                event: Event::ConditionMet {
                                    bar,
                                    t,
                                    seq: self.seq_to(&self.conns[ci].principal, seq),
                                    evidence: evidence.map(|ev| {
                                        crate::claims::for_principal(
                                            &self.doc,
                                            &self.plane,
                                            &self.conns[ci].principal,
                                            &ev,
                                        )
                                    }),
                                },
                            },
                        ));
                    }
                }
            }
        }
        let fired = self.eval_intents(seq);
        let mut intents = self.intent_lines(fired);
        intents.extend(self.eval_intents_public());
        (conds, intents)
    }

    /// `intent_triggered` events to every `intents` subscriber entitled to the bound feeds
    /// the intent's trigger reads (§18.3), by connection and subscription order.
    pub(crate) fn intent_lines(&self, fired: Vec<Event>) -> Vec<OutboundLine> {
        let mut lines = Vec::new();
        for ev in fired {
            let trigger = match &ev {
                Event::IntentTriggered { intent, .. } => self
                    .doc
                    .intents
                    .iter()
                    .find(|i| &i.id == intent)
                    .and_then(|i| i.trigger.as_ref()),
                _ => None,
            };
            for c in &self.conns {
                if let Some(t) = trigger {
                    if !crate::live::hears(&self.plane, &self.doc, &c.principal, t) {
                        continue;
                    }
                }
                // (R3/K1c) A public-view reader hears the public world's intents only
                // (`eval_intents_public`).
                if self.public_view(&c.principal) {
                    continue;
                }
                for s in &c.subs {
                    if matches!(s.topic, Topic::Intents {}) {
                        let mut event =
                            crate::claims::event_for(&self.doc, &self.plane, &c.principal, &ev);
                        if let Event::IntentTriggered { seq, .. } = &mut event {
                            *seq = self.seq_to(&c.principal, *seq);
                        }
                        lines.push(out(
                            &c.id,
                            ServerMsg::Notify {
                                sub: s.id.clone(),
                                event,
                            },
                        ));
                    }
                }
            }
        }
        lines
    }

    pub(super) fn eval_intents(&mut self, seq: Seq) -> Vec<Event> {
        let mut fired = Vec::new();
        let mut state = BTreeMap::new();
        let intents: Vec<_> = self
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
            if self.cond_blocked(&trig) {
                if let Some(prev) = self.intent_state.get(&id) {
                    state.insert(id, *prev);
                }
                continue;
            }
            let r = self
                .registry
                .eval_condition(rd!(self), &mut self.cache, &trig);
            let now = r.is_some_and(|x| x.0);
            if self.intent_state.get(&id) == Some(&false) && now {
                if let Some((_, bar, t)) = r {
                    let evidence = self.claim_asserted(
                        crate::claims::Owner::Doc,
                        qp_schema::protocol::ClaimRef::Intent { intent: id.clone() },
                        &trig,
                    );
                    fired.push(Event::IntentTriggered {
                        intent: id.clone(),
                        bar,
                        t,
                        seq,
                        evidence,
                    });
                }
            }
            state.insert(id, now);
        }
        self.intent_state = state;
        self.intent_epoch += 1;
        fired
    }

    /// Notifications for one log entry: those of the authority's world to the readers
    /// that see it, and (R3/K1) those of the public world's version of the entry to the
    /// public-view readers ([`Session::public_view`]). Each connection's lines come from
    /// one world, in the order of one world.
    pub(crate) fn entry_events(&mut self, seq: Seq) -> Vec<OutboundLine> {
        let mut parts = self.entry_events_in(seq, false);
        if self.other.is_some() && self.conns.iter().any(|c| self.public_view(&c.principal)) {
            // (R3/K1b) The public entry this entry made (its own number), if any.
            if let Some(p) = self.pub_of(seq) {
                let pubs = self.in_world(true, |s| s.entry_events_in(p, true));
                for (k, p) in pubs.into_iter().enumerate() {
                    parts[k].extend(p);
                }
            }
            // Change events in connection order, as one world gives them.
            parts[0].sort_by_key(|(ci, _)| *ci);
        }
        parts.into_iter().flatten().map(|(_, l)| l).collect()
    }

    #[allow(clippy::too_many_lines)]
    /// [`Session::entry_events`] of the world the fields hold, for the connections of
    /// that world (`public`; without a public world, every connection), in three groups
    /// (changes, finding states, proposals), each line with its connection index.
    pub(crate) fn entry_events_in(
        &self,
        seq: Seq,
        public: bool,
    ) -> [Vec<(usize, OutboundLine)>; 3] {
        let e = &self.log[usize::try_from(seq - 1).unwrap_or(0)];
        let mut lines = Vec::new();
        let mut states = Vec::new();
        let mut props = Vec::new();
        let mine = |c: &Conn| self.other.is_none() || self.public_view(&c.principal) == public;
        let touched = touched_ids(&e.changes);
        for (ci, c) in self.conns.iter().enumerate() {
            if !mine(c) {
                continue;
            }
            // (R3, §20) What this subscriber may receive of the engine's items.
            let proj = if c
                .subs
                .iter()
                .any(|s| matches!(s.topic, Topic::Changes { .. }))
            {
                self.reader_proj(&c.principal)
            } else {
                None
            };
            let projected = proj
                .as_ref()
                .map(|p| self.project_entry(e, p, self.lang_of(ci)));
            let shown = projected.as_ref().unwrap_or(e);
            for s in &c.subs {
                if let Topic::Changes { ids } = &s.topic {
                    if ids.is_empty() || ids.iter().any(|i| touched.contains(i)) {
                        if proj.is_some() && !ids.is_empty() {
                            let vis = touched_ids(&shown.changes);
                            if !ids.iter().any(|i| vis.contains(i)) {
                                continue;
                            }
                        }
                        lines.push((
                            ci,
                            out(
                                &c.id,
                                ServerMsg::Notify {
                                    sub: s.id.clone(),
                                    event: Event::Change {
                                        seq,
                                        at: e.at,
                                        author: e.author.clone(),
                                        approved_by: e.approved_by.clone(),
                                        cause: shown.cause.clone(),
                                        changes: shown.changes.clone(),
                                        evidence: e.evidence.as_ref().map(|ev| {
                                            crate::claims::for_principal(
                                                &self.doc,
                                                &self.plane,
                                                &c.principal,
                                                ev,
                                            )
                                        }),
                                    },
                                },
                            ),
                        ));
                    }
                }
            }
        }
        for ch in &e.changes {
            let ChangeOp::SetCapsuleState { capsule, state } = ch else {
                continue;
            };
            let Some(finding) = self
                .doc
                .capsules
                .iter()
                .find(|c| &c.id == capsule)
                .map(|c| c.finding.clone())
            else {
                continue;
            };
            for (ci, c) in self.conns.iter().enumerate() {
                if !mine(c) {
                    continue;
                }
                for s in &c.subs {
                    if matches!(&s.topic, Topic::Finding { finding: f } if *f == finding) {
                        if self
                            .proj_for(&c.principal)
                            .is_some_and(|p| p.hidden.contains(capsule))
                        {
                            continue;
                        }
                        states.push((
                            ci,
                            out(
                                &c.id,
                                ServerMsg::Notify {
                                    sub: s.id.clone(),
                                    event: Event::FindingState {
                                        finding: finding.clone(),
                                        capsule: capsule.clone(),
                                        state: *state,
                                        seq,
                                        evidence: e.evidence.as_ref().map(|ev| {
                                            crate::claims::for_principal(
                                                &self.doc,
                                                &self.plane,
                                                &c.principal,
                                                ev,
                                            )
                                        }),
                                    },
                                },
                            ),
                        ));
                    }
                }
            }
        }
        for ch in &e.changes {
            let ChangeOp::PutProposal { proposal, .. } = ch else {
                continue;
            };
            let author = self.principal_of(&proposal.provenance.author);
            for (ci, c) in self.conns.iter().enumerate() {
                if !mine(c) {
                    continue;
                }
                for s in &c.subs {
                    if matches!(s.topic, Topic::Proposals {}) {
                        props.push((
                            ci,
                            out(
                                &c.id,
                                ServerMsg::Notify {
                                    sub: s.id.clone(),
                                    event: Event::Proposal {
                                        proposal: proposal.id.clone(),
                                        state: proposal.state,
                                        author: author.clone(),
                                        seq,
                                    },
                                },
                            ),
                        ));
                    }
                }
            }
        }
        [lines, states, props]
    }
}
