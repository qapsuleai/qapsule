//! Following the authority: messages, events, mirroring the log, data events and resync.

use super::*;

impl Replica {
    /// One message from the authority on this replica's connection.
    pub fn on_message(&mut self, msg: &ServerMsg) {
        match msg {
            ServerMsg::Welcome { .. } => {
                if self.s.is_none() && !self.doc_asked {
                    self.doc_asked = true;
                    for (id, kind) in [(SUB_CHANGES, "changes"), (SUB_CLAIMS, "claims")] {
                        self.outbox.push(serde_json::json!({
                            "type": "subscribe", "id": id, "topic": {"kind": kind}
                        }));
                    }
                    self.outbox.push(serde_json::json!({
                        "type": "query", "id": Q_DOC, "query": {"kind": "document"}
                    }));
                }
            }
            ServerMsg::Result { id, seq, data, .. } => match data {
                QueryResult::Document { document } if id == Q_DOC => {
                    self.load((**document).clone(), *seq);
                }
                QueryResult::Log { entries } if id.starts_with(Q_LOG) => {
                    self.log_asked = false;
                    for e in entries {
                        self.buffered.entry(e.seq).or_insert_with(|| e.clone());
                    }
                    self.drain();
                }
                _ => {}
            },
            ServerMsg::Notify { event, .. } => self.on_event(event),
            _ => {}
        }
        self.reconcile();
    }

    /// One outbound line of the authority (`{"conn":…,"msg":…}`) or a bare message, as
    /// JSON text.
    ///
    /// # Errors
    /// Not JSON or not a server message.
    pub fn on_line(&mut self, line: &str) -> Result<(), String> {
        let v: serde_json::Value = serde_json::from_str(line).map_err(|e| e.to_string())?;
        let m = v.get("msg").cloned().unwrap_or(v);
        let msg: ServerMsg = serde_json::from_value(m).map_err(|e| e.to_string())?;
        self.on_message(&msg);
        Ok(())
    }

    fn on_event(&mut self, event: &Event) {
        match event {
            Event::Change {
                seq,
                at,
                author,
                approved_by,
                cause,
                changes,
                evidence,
            } => {
                let entry = LogEntry {
                    seq: *seq,
                    at: *at,
                    author: author.clone(),
                    approved_by: approved_by.clone(),
                    cause: cause.clone(),
                    reason: None,
                    request: Vec::new(),
                    changes: changes.clone(),
                    inverse: Vec::new(),
                    undoes: None,
                    summary: String::new(),
                    evidence: evidence.clone(),
                };
                self.buffered.entry(*seq).or_insert(entry);
                self.drain();
            }
            Event::Claim {
                claim,
                state,
                evidence,
                ..
            } => self.learn(claim.clone(), *state, evidence.clone(), true),
            _ => {}
        }
    }

    /// Records the authority's word on a claim. `authoritative`: from the `claims` topic
    /// (replaces); else a fallback (only for a claim not known yet).
    fn learn(
        &mut self,
        claim: ClaimRef,
        state: ClaimState,
        ev: ClaimEvidence,
        authoritative: bool,
    ) {
        let key = (claim, ev.t);
        if !authoritative && self.claims.contains_key(&key) {
            return;
        }
        self.counter += 1;
        self.claims.insert(
            key,
            Known {
                state,
                ev,
                n: self.counter,
            },
        );
        if self.claims.len() > MAX_CLAIMS {
            if let Some(k) = self
                .claims
                .iter()
                .min_by_key(|(_, k)| k.n)
                .map(|(k, _)| k.clone())
            {
                self.claims.remove(&k);
            }
        }
    }

    pub(super) fn load(&mut self, doc: Document, seq: Seq) {
        let provider = match self.provider.take() {
            Some(p) => p,
            None => self.s.take().map_or(Provider::None, |mut s| {
                std::mem::replace(&mut s.plane.provider, Provider::None)
            }),
        };
        match follower(doc, &self.principal, provider) {
            Ok(mut s) => {
                s.set_headline_calendar(self.headline_cal.clone());
                self.s = Some(s);
                self.seq = seq;
                self.doc_asked = false;
                self.buffered.retain(|k, _| *k > seq);
                self.drain();
            }
            Err(_) => self.provider = Some(Provider::None),
        }
    }

    /// Applies buffered entries in order; a hole asks for the `log` after `seq`.
    fn drain(&mut self) {
        if self.s.is_none() {
            return;
        }
        self.buffered.retain(|k, _| *k > self.seq);
        while let Some(e) = self.buffered.remove(&(self.seq + 1)) {
            if !self.mirror(e) {
                // The mirror does not fit (should not happen): reload the document.
                self.buffered.clear();
                if !self.doc_asked {
                    self.doc_asked = true;
                    self.outbox.push(serde_json::json!({
                        "type": "query", "id": Q_DOC, "query": {"kind": "document"}
                    }));
                }
                return;
            }
        }
        if !self.buffered.is_empty() && !self.log_asked {
            self.log_asked = true;
            self.outbox.push(serde_json::json!({
                "type": "query", "id": format!("{Q_LOG}:{}", self.seq),
                "query": {"kind": "log", "after_seq": self.seq}
            }));
        }
    }

    /// Mirrors one log entry into the follower (the same application and bookkeeping as
    /// the authority's own entries).
    fn mirror(&mut self, mut e: LogEntry) -> bool {
        let Some(s) = self.s.as_mut() else {
            return false;
        };
        let restore = crate::live::Restore {
            initial: &s.initial,
            log: &s.log,
        };
        let Ok(inverse) = crate::live::apply_in_place(&mut s.doc, &e.changes, restore) else {
            return false;
        };
        e.inverse = inverse;
        if let Some(ev) = e.evidence.clone() {
            for c in &e.changes {
                if let qp_schema::protocol::ChangeOp::SetCapsuleState { capsule, .. } = c {
                    let finding = s
                        .doc
                        .capsules
                        .iter()
                        .find(|x| &x.id == capsule)
                        .map(|x| x.finding.clone())
                        .unwrap_or_default();
                    let claim = ClaimRef::Capsule {
                        capsule: capsule.clone(),
                        finding,
                    };
                    self.pending_learn.push((claim, ev.clone()));
                }
            }
        }
        self.seq = e.seq;
        s.committed(e);
        s.pending_views.clear();
        let _ = s.claims.take();
        let learn = std::mem::take(&mut self.pending_learn);
        for (c, ev) in learn {
            self.learn(c, ClaimState::Asserted, ev, false);
        }
        true
    }

    /// A data event from this replica's own feed (§18.4 rules; a gap makes the binding
    /// stale until the host re-resolves: [`Replica::resync`]). The display is re-checked
    /// in the same step.
    ///
    /// # Errors
    /// [`DataError`] as on the authority (unknown feed, rejected event).
    pub fn data_event(&mut self, ev: &DataEvent) -> Result<(), DataError> {
        let Some(s) = self.s.as_mut() else {
            // No document yet: the provider (memory) keeps the event for the resolution.
            if let Some(Provider::Memory(m)) = self.provider.as_mut() {
                let _ = m.apply(ev);
            }
            return Ok(());
        };
        let outcome = s.plane.apply_event(ev)?;
        if outcome.changed.is_empty() {
            return Ok(());
        }
        let ids: Vec<String> = outcome.changed.iter().map(|(id, _)| id.clone()).collect();
        s.data_changed(outcome.changed);
        s.pending_views.clear();
        self.reconcile_on(Some(&ids));
        Ok(())
    }

    /// Resolves every bound series again from the provider (the host after a gap, after a
    /// settled fetch it put into its provider, or after the provider caught up).
    pub fn resync(&mut self) {
        if let Some(s) = self.s.as_mut() {
            let ids: Vec<String> = s.plane.series.keys().cloned().collect();
            s.plane.series.clear();
            let _ = s.plane.sync(&s.doc);
            s.pending_stamps.clear();
            s.data_changed(
                ids.into_iter()
                    .map(|id| (id, Delta::Rebuild { from: 0 }))
                    .collect(),
            );
            s.pending_views.clear();
        }
        self.reconcile();
    }

    /// The host declares that its data of bar series `series` from `from_t` on may lack
    /// events (a gap it has not repaired); `None` clears it.
    pub fn set_unclean(&mut self, series: &str, from_t: Option<i64>) {
        if self.unclean.get(series).copied() == from_t {
            return;
        }
        match from_t {
            Some(t) => {
                self.unclean.insert(series.to_owned(), t);
            }
            None => {
                self.unclean.remove(series);
            }
        }
        self.reconcile_on(Some(&[series.to_owned()]));
    }

    /// The host declares that bars of `series` up to `t` hold settled values (from a
    /// settled fetch), although its chart has not closed the last of them.
    pub fn set_settled(&mut self, series: &str, t: i64) {
        if self.settled.get(series) == Some(&t) {
            return;
        }
        self.settled.insert(series.to_owned(), t);
        self.reconcile_on(Some(&[series.to_owned()]));
    }

    /// Settled refetches the replica asks the host for: (bar series, claim bar time).
    /// The host fetches settled bars up to `t`, applies them ([`Replica::resync`] or data
    /// events) and answers with [`Replica::refetched`].
    pub fn take_refetch(&mut self) -> Vec<(String, i64)> {
        std::mem::take(&mut self.refetch_new)
    }

    /// The host has refetched settled data of `series` up to `t`.
    pub fn refetched(&mut self, series: &str, t: i64) {
        self.refetched.insert((series.to_owned(), t));
        self.reconcile();
    }
}
