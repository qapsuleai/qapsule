//! The live path on the session: bar commands, settling, the live index and live
//! evaluation of conditions, intents, invalidations and view events.

use super::*;

impl Session {
    #[allow(clippy::too_many_lines)]
    /// A direct command whose ops are all bar writes (`append_bars`, `update_last_bar`),
    /// applied to the document in place (no copy). Each op is expanded against the
    /// document as the earlier ops leave it and applied; a failing op (or `base`) rolls
    /// the applied ones back. The log entry is the one the copying path writes.
    pub(crate) fn bar_command(
        &mut self,
        ci: usize,
        id: &str,
        ops: &[Op],
        (base, reason): (Option<Seq>, Option<String>),
        resolved: BTreeMap<String, String>,
    ) -> PResult<Vec<OutboundLine>> {
        let role = self.conns[ci].role;
        let perms = permissions(role);
        let principal = self.conns[ci].principal.clone();
        let conn = self.conns[ci].id.clone();
        let author = self
            .principal(&principal)
            .map(|p| p.author.clone())
            .ok_or_else(|| {
                E::new(
                    qp_schema::protocol::ErrorCode::Handshake,
                    "principal vanished",
                )
                .build()
            })?;
        let seq = self.seq() + 1;
        let mut changes = Vec::new();
        let mut groups: Vec<Vec<ChangeOp>> = Vec::new();
        let mut origin = Vec::new();
        let (created, undoes) = {
            let mut ctx = Ctx {
                author,
                approved_by: None,
                at: self.clock,
                seq,
                id_seq: seq,
                n: 0,
                created: Vec::new(),
                speaker: &principal,
                is_owner: role == qp_schema::protocol::Role::Owner,
                can_decide: perms.decide,
                log: &self.log,
                undoes: None,
                data: self.rd.as_ref().map(|rd| Reader {
                    rd,
                    plane: &self.plane,
                }),
                plane: &self.plane,
            };
            let restore = Restore {
                initial: &self.initial,
                log: &self.log,
            };
            let doc = &mut self.doc;
            for (i, op) in ops.iter().enumerate() {
                let p = ptr("/ops", i);
                let cs = match expand_op(doc, op, &p, &mut ctx) {
                    Ok(cs) => cs,
                    Err(e) => {
                        roll_back(doc, &groups, restore);
                        return Err(e);
                    }
                };
                match apply_in_place(doc, &cs, restore) {
                    Ok(inv) => groups.push(inv),
                    Err(m) => {
                        roll_back(doc, &groups, restore);
                        return Err(E::new(
                            qp_schema::protocol::ErrorCode::Invalid,
                            format!("cannot apply: {m}"),
                        )
                        .at(p)
                        .build());
                    }
                }
                origin.extend(std::iter::repeat_n(i, cs.len()));
                changes.extend(cs);
            }
            (ctx.created, ctx.undoes)
        };
        if let Err(e) = self.base_check(base, &changes, &origin) {
            let restore = Restore {
                initial: &self.initial,
                log: &self.log,
            };
            roll_back(&mut self.doc, &groups, restore);
            return Err(e);
        }
        let inverse = groups.into_iter().rev().flatten().collect();
        let pv = self.public_view(&principal);
        self.committed(LogEntry {
            reason,
            seq,
            at: self.clock,
            author: principal,
            approved_by: None,
            cause: Some(id.to_owned()),
            request: ops.to_vec(),
            changes,
            inverse,
            undoes,
            summary: summarize_ops(ops, &created),
            evidence: None,
        });
        let mut lines = vec![out(
            &conn,
            ServerMsg::Ack {
                resolved,
                id: id.to_owned(),
                status: AckStatus::Applied,
                // (R3/K1b) A public-view writer: the public entry's own number.
                seq: Some(if pv {
                    self.pub_of(seq).unwrap_or_else(|| self.pub_len())
                } else {
                    seq
                }),
                created,
                proposal: None,
                undoes,
            },
        )];
        lines.extend(self.settle(seq));
        Ok(lines)
    }

    /// True if entry `seq` only writes bars.
    pub(crate) fn bars_only(&self, seq: Seq) -> bool {
        let e = &self.log[usize::try_from(seq - 1).unwrap_or(0)];
        !e.changes.is_empty()
            && e.changes.iter().all(|c| {
                matches!(
                    c,
                    ChangeOp::AppendBars { .. }
                        | ChangeOp::SetLastBar { .. }
                        | ChangeOp::TruncateBars { .. }
                )
            })
    }

    /// After a bar change (entry `entry`, or a data event: `None`): only what rests on
    /// the changed bar series — condition subscriptions and intents (at the current
    /// position), then live capsules' invalidations (engine entries) — and the
    /// notifications in the order of §15. Watchers not resting on a changed series keep
    /// their value (their series did not change), so skipping them changes no output.
    pub(crate) fn settle_live(&mut self, entry: Option<Seq>) -> Vec<OutboundLine> {
        let seq = self.seq();
        let mut changed: Vec<String> = Vec::new();
        for (id, _, _) in &self.pending_views {
            if !changed.contains(id) {
                changed.push(id.clone());
            }
        }
        let changed_ref: Vec<&str> = changed.iter().map(String::as_str).collect();
        let conds = self.eval_conditions_live(seq, &changed_ref);
        let fired = self.eval_intents_live(seq, &changed_ref);
        let mut entries: Vec<Seq> = entry.into_iter().collect();
        entries.extend(self.invalidate_live(&changed_ref));
        let mut lines = Vec::new();
        for s in entries {
            lines.extend(self.entry_events(s));
        }
        // (R3/K1b) The public world's own capsules' invalidations.
        lines.extend(self.invalidate_in_public());
        lines.extend(self.run_capsules());
        lines.extend(self.view_events());
        lines.extend(conds);
        let claims = self.claim_events();
        lines.extend(self.intent_lines(fired));
        lines.extend(self.eval_intents_public());
        lines.extend(claims);
        lines
    }

    /// The live index of the current structure generation (built if needed).
    pub(crate) fn take_index(&mut self) -> LiveIndex {
        match self.live_index.take() {
            Some(i) if i.gen == self.struct_gen => i,
            _ => self.build_index(),
        }
    }

    /// Builds the live index of the document: series, feeds, active intents with a
    /// value trigger and live capsules (with the principals who must hear them).
    fn build_index(&self) -> LiveIndex {
        let doc = &self.doc;
        let mut idx = LiveIndex::of_series(doc, self.struct_gen);
        for i in doc.intents.iter().filter(|i| i.active) {
            let Some(trig) = &i.trigger else { continue };
            if matches!(trig, Condition::Text { .. }) {
                continue;
            }
            let info = idx.cond_info(doc, trig);
            idx.intents.push(IntentWatch {
                id: i.id.clone(),
                trigger: trig.clone(),
                info,
                memo: None,
                last: None,
                stamp: 0,
            });
        }
        // (R3/perf) Findings by id (first occurrence, as `iter().find` gives it).
        let mut finding_at: std::collections::HashMap<&str, usize> =
            std::collections::HashMap::with_capacity(doc.findings.len());
        for (i, f) in doc.findings.iter().enumerate() {
            finding_at.entry(f.id.as_str()).or_insert(i);
        }
        for c in doc
            .capsules
            .iter()
            .filter(|c| c.state == qp_schema::CapsuleState::Live)
        {
            let Some(f) = finding_at
                .get(c.finding.as_str())
                .map(|&i| &doc.findings[i])
            else {
                continue;
            };
            let info = idx.cond_info(doc, &f.invalidation);
            // (R3/perf) A condition resting on no series (a text condition, as every
            // engine finding's) is never evaluated by `invalidate_live`: no watch.
            if info.closure.is_empty() {
                continue;
            }
            let who = self
                .responsible(f, info.closure())
                .iter()
                .filter_map(|p| self.principals.iter().position(|x| x.id == *p))
                .collect();
            idx.capsules.push(CapsuleWatch {
                capsule: c.id.clone(),
                finding: c.finding.clone(),
                invalidation: f.invalidation.clone(),
                info,
                who,
                memo: None,
            });
        }
        idx
    }

    #[allow(clippy::expect_used, clippy::too_many_lines)]
    /// Edge-triggered condition subscriptions resting on a changed series. A watcher is
    /// evaluated when a cache slot of its series changed in this tick, when its slots are
    /// not known (first time, or after the cache dropped slots), or when a series it
    /// reads is not computable and rests on a changed series.
    fn eval_conditions_live(&mut self, seq: Seq, changed: &[&str]) -> Vec<OutboundLine> {
        let idx = self.take_index();
        // (R3/K1b) The sequence number each connection may receive.
        let conditions = self.conns.iter().any(|c| {
            c.subs
                .iter()
                .any(|s| matches!(s.topic, Topic::Condition { .. }))
        });
        let seqs: Vec<Seq> = if self.other.is_some() && conditions {
            self.conns
                .iter()
                .map(|c| self.seq_to(&c.principal, seq))
                .collect()
        } else {
            vec![seq; self.conns.len()]
        };
        let rd = self.rd.as_ref().unwrap_or(&self.doc);
        let cache = &mut self.cache;
        let ready = all_ready(&self.plane);
        let mut ents = Ents::new(&self.plane, &idx.feeds, &self.principals);
        let mut lines = Vec::new();
        for (ci, c) in self.conns.iter_mut().enumerate() {
            let mut row = None;
            for s in &mut c.subs {
                let Topic::Condition { condition } = &s.topic else {
                    continue;
                };
                if s.watch.as_ref().is_none_or(|(g, _)| *g != idx.gen) {
                    s.watch = Some((idx.gen, idx.cond_info(&self.doc, condition)));
                }
                let info = &s.watch.as_ref().expect("set above").1;
                let ids = cond_ids(condition);
                let affected = match s.memo.filter(|(g, _)| *g == cache.gen()) {
                    None => true,
                    Some((_, slots)) => ids.iter().zip(slots).any(|(id, slot)| match (id, slot) {
                        (Some(_), Some(i)) => cache.touched_now(i),
                        (Some(_), None) => info.rests_on_any(changed),
                        (None, _) => false,
                    }),
                };
                if !affected {
                    continue;
                }
                let who = *row.get_or_insert_with(|| {
                    self.principals
                        .iter()
                        .position(|p| p.id == c.principal)
                        .ok_or(c.principal.as_str())
                });
                if !ents.hears(who, info) {
                    // Not evaluated for a subscriber without the data entitlement (§18.3).
                    continue;
                }
                if info.blocked(&self.plane, ready) {
                    // Not evaluated; the edge state is kept (§18.3).
                    continue;
                }
                let r = eval_memo(rd, cache, condition, &mut s.memo);
                let now = r.is_some_and(|x| x.0);
                let prev = s.last.replace(now);
                if prev == Some(false) && now {
                    if let Some((_, bar, t)) = r {
                        let evidence = crate::claims::evidence(
                            rd,
                            cache,
                            &self.claims,
                            &self.plane,
                            condition,
                        );
                        if let Some(ev) = &evidence {
                            self.claims.assert(
                                crate::claims::Owner::Conn(c.id.clone()),
                                qp_schema::protocol::ClaimRef::Condition { sub: s.id.clone() },
                                condition.clone(),
                                ev.clone(),
                            );
                        }
                        lines.push(out(
                            &c.id,
                            ServerMsg::Notify {
                                sub: s.id.clone(),
                                event: Event::ConditionMet {
                                    bar,
                                    t,
                                    seq: seqs[ci],
                                    evidence: evidence.map(|ev| {
                                        crate::claims::for_principal(
                                            &self.doc,
                                            &self.plane,
                                            &c.principal,
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
        drop(ents);
        self.live_index = Some(idx);
        lines
    }

    /// Active intents whose trigger rests on a changed series; the others keep their
    /// state.
    fn eval_intents_live(&mut self, seq: Seq, changed: &[&str]) -> Vec<Event> {
        let mut idx = self.take_index();
        let rd = self.rd.as_ref().unwrap_or(&self.doc);
        let ready = all_ready(&self.plane);
        let mut fired = Vec::new();
        for w in &mut idx.intents {
            if !w.info.rests_on_any(changed) || w.info.blocked(&self.plane, ready) {
                continue;
            }
            let r = eval_memo(rd, &mut self.cache, &w.trigger, &mut w.memo);
            let now = r.is_some_and(|x| x.0);
            let prev = if w.stamp == self.intent_epoch {
                w.last
            } else {
                self.intent_state.get(&w.id).copied()
            };
            if prev == Some(false) && now {
                if let Some((_, bar, t)) = r {
                    let evidence = crate::claims::evidence(
                        rd,
                        &mut self.cache,
                        &self.claims,
                        &self.plane,
                        &w.trigger,
                    );
                    if let Some(ev) = &evidence {
                        self.claims.assert(
                            crate::claims::Owner::Doc,
                            qp_schema::protocol::ClaimRef::Intent {
                                intent: w.id.clone(),
                            },
                            w.trigger.clone(),
                            ev.clone(),
                        );
                    }
                    fired.push(Event::IntentTriggered {
                        intent: w.id.clone(),
                        bar,
                        t,
                        seq,
                        evidence,
                    });
                }
            }
            if prev != Some(now) {
                self.intent_state.insert(w.id.clone(), now);
            }
            w.last = Some(now);
            w.stamp = self.intent_epoch;
        }
        self.live_index = Some(idx);
        fired
    }

    /// Engine entries invalidating live capsules whose invalidation rests on a changed
    /// series and holds now (evaluated from the index; the ones that hold go through
    /// [`Session::invalidate_these`], which checks them again and writes the entries).
    fn invalidate_live(&mut self, changed: &[&str]) -> Vec<Seq> {
        let mut idx = self.take_index();
        let rd = self.rd.as_ref().unwrap_or(&self.doc);
        let ready = all_ready(&self.plane);
        let mut ents = Ents::new(&self.plane, &idx.feeds, &self.principals);
        let mut holding = Vec::new();
        for w in &mut idx.capsules {
            if !w.info.rests_on_any(changed) || w.info.blocked(&self.plane, ready) {
                continue;
            }
            if !w.who.iter().all(|&k| ents.hears(Ok(k), &w.info)) {
                continue;
            }
            if eval_memo(rd, &mut self.cache, &w.invalidation, &mut w.memo).is_some_and(|r| r.0) {
                holding.push((w.capsule.clone(), w.finding.clone()));
            }
        }
        drop(ents);
        self.live_index = Some(idx);
        if holding.is_empty() {
            return Vec::new();
        }
        self.invalidate_these(holding)
    }

    #[allow(clippy::expect_used)]
    /// `view_changed` for every pending bar change, to the `view` subscribers whose view
    /// reads the changed series (a bound series only to principals entitled to its feed),
    /// per change in order, then by connection and subscription order (§15).
    pub(crate) fn view_events(&mut self) -> Vec<OutboundLine> {
        let pend = std::mem::take(&mut self.pending_views);
        if pend.is_empty()
            || !self
                .conns
                .iter()
                .any(|c| c.subs.iter().any(|s| matches!(s.topic, Topic::View { .. })))
        {
            return Vec::new();
        }
        let rd = self.rd.as_ref().unwrap_or(&self.doc);
        if self.view_reads.is_none() || !self.incremental {
            self.view_reads = Some(build_view_reads(rd));
        }
        let reads = self.view_reads.as_ref().expect("built above");
        let mut lines = Vec::new();
        for (series, d, seq) in &pend {
            let n = bars_len(rd, series);
            let feed = self
                .plane
                .series
                .get(series)
                .map(|bs| bs.binding.feed_key());
            for c in &self.conns {
                if let Some(k) = &feed {
                    if !self.plane.entitled(&c.principal, k) {
                        continue;
                    }
                }
                for s in &c.subs {
                    let Topic::View { view } = &s.topic else {
                        continue;
                    };
                    if !reads.get(view).is_some_and(|r| r.contains(series)) {
                        continue;
                    }
                    lines.push(out(
                        &c.id,
                        ServerMsg::Notify {
                            sub: s.id.clone(),
                            event: Event::ViewChanged {
                                view: view.clone(),
                                series: series.clone(),
                                from_bar: d.first_bar() as u64,
                                to_bar: n.checked_sub(1).map(|x| x as u64),
                                seq: self.seq_to(&c.principal, *seq),
                            },
                        },
                    ));
                }
            }
        }
        lines
    }
}
