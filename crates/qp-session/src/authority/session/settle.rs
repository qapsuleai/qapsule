//! Data events, bindings and the settle point (engine entries).

use super::*;

impl Session {
    /// The host's data path (§18.4): applies a data event to the bound series of its
    /// feed (duplicate, gap, stale, outside-window and rule checks by
    /// `qp_source::FeedStore`), then, if bars or a binding state changed, evaluates live
    /// capsules' invalidations, condition subscriptions and intents as a bar write does
    /// (engine entries and notifications in the order of §15). The event is not a
    /// protocol message: it never enters the log and causes no `change` by itself.
    ///
    /// Incremental (§18.8): the reading document takes the changed bar in place, the
    /// study series on the ticked bound series advance their streaming state, and only
    /// the watchers, live capsules and `view` subscriptions resting on it are evaluated.
    ///
    /// # Errors
    /// [`DataError`]: unknown feed or an event the rules reject (nothing applied to that
    /// store; the host re-resolves after the gap that follows, §18.4).
    pub fn data_event(
        &mut self,
        ev: &qp_schema::data::DataEvent,
    ) -> Result<Vec<OutboundLine>, DataError> {
        self.host_data_event(ev).map(|o| o.lines)
    }

    /// [`Session::data_event`] with the notices for the host (§18.4): a `seq` gap or a
    /// binding that became `stale` is not an error (the event was taken: nothing to
    /// apply), but the host must re-resolve ([`Session::refresh_bindings`]) or fill the
    /// gap. A host should use this entry point; `qp serve --host-frames` writes each
    /// notice as a host frame.
    ///
    /// # Errors
    /// As [`Session::data_event`].
    pub fn host_data_event(
        &mut self,
        ev: &qp_schema::data::DataEvent,
    ) -> Result<DataOutcome, DataError> {
        let outcome = self.plane.apply_event(ev)?;
        let notices = outcome.notices;
        if outcome.changed.is_empty() {
            return Ok(DataOutcome {
                lines: Vec::new(),
                notices,
            });
        }
        self.data_changed(outcome.changed);
        let mut lines = self.settle_data();
        lines.extend(self.serve_test_source());
        // (RP3/M15, §22) Viewers, unless the host batches (a replay host: once per step).
        if !self.viewer_batch && self.has_pushers() {
            lines.extend(self.viewer_flush());
        }
        Ok(DataOutcome { lines, notices })
    }

    /// Resolves again every bound series that is not ready (after a gap, a limit, a
    /// provider that answered late) and returns the notifications this causes (an anchor
    /// stamp entry, invalidations, conditions).
    pub fn refresh_bindings(&mut self) -> Vec<OutboundLine> {
        let (stamps, changed) = self.plane.refresh();
        if changed.is_empty() {
            return Vec::new();
        }
        self.pending_stamps = stamps;
        let changed = changed
            .into_iter()
            .map(|id| (id, crate::values::Delta::Rebuild { from: 0 }))
            .collect();
        self.data_changed(changed);
        match self.stamp_anchors() {
            Some(seq) => self.settle(seq),
            None => self.settle_data(),
        }
    }

    /// The host's binding state of a bound series (`None`: not a bound series).
    #[must_use]
    pub fn binding_state(&self, series: &str) -> Option<qp_schema::data::BindingState> {
        self.plane.host_state(series).map(|(s, _)| s)
    }

    /// After the data plane changed bound series (`changed`: series, how): the reading
    /// document takes the change in place (a new bar is pushed, a revised bar replaced;
    /// a state change re-injects or removes the bars), the values cache follows
    /// incrementally (or is cleared on the reference path), and the change waits for
    /// `view` subscribers.
    pub(crate) fn data_changed(&mut self, changed: Vec<(String, crate::values::Delta)>) {
        // (RP3/M15) The headlines' refresh compares the bars of these series only.
        self.head_state
            .dirty
            .extend(changed.iter().map(|(id, _)| id.clone()));
        if let Some(w) = self.other.as_mut() {
            w.head_state
                .dirty
                .extend(changed.iter().map(|(id, _)| id.clone()));
        }
        match self.rd.as_mut() {
            Some(rd) => {
                for (id, d) in &changed {
                    crate::live::mirror(rd, &self.plane, id, *d);
                }
            }
            None => self.rd = reading_doc(&self.doc, &self.plane),
        }
        // (R3/K1) The public world reads the same bars.
        debug_assert!(!self.in_public);
        if let Some(w) = self.other.as_mut() {
            match w.rd.as_mut() {
                Some(rd) => {
                    for (id, d) in &changed {
                        crate::live::mirror(rd, &self.plane, id, *d);
                    }
                }
                None => w.rd = reading_doc(&w.doc, &self.plane),
            }
        }
        if self.incremental {
            self.cache.begin_tick();
            for (id, d) in &changed {
                self.cache.apply_delta(rd!(self), id, *d);
            }
        } else {
            self.cache.clear();
        }
        self.claims_after_bars(&changed);
        self.note_closed(&changed);
        let seq = self.seq();
        for (id, d) in changed {
            self.pending_views.push((id, d, seq));
        }
    }

    /// After a data change without a log entry: watchers at the current position, then
    /// invalidations of every live capsule (§8.1, §18.4); incremental: only those resting
    /// on the changed series (§18.8).
    pub(crate) fn settle_data(&mut self) -> Vec<OutboundLine> {
        if self.incremental {
            return self.settle_live(None);
        }
        let seq = self.seq();
        let (mut conds, mut intents) = self.eval_watchers(seq);
        let mut entries = Vec::new();
        for s in self.invalidate(true, &BTreeSet::new()) {
            entries.push(s);
            let (c, i) = self.eval_watchers(s);
            conds.extend(c);
            intents.extend(i);
        }
        let mut lines = Vec::new();
        for s in entries {
            lines.extend(self.entry_events(s));
        }
        lines.extend(self.invalidate_in_public());
        lines.extend(self.run_capsules());
        lines.extend(self.view_events());
        lines.extend(conds);
        lines.extend(intents);
        lines.extend(self.claim_events());
        lines
    }

    /// Reader of the data plane for command expansion.
    pub(crate) fn reader(&self) -> Option<Reader<'_>> {
        self.rd.as_ref().map(|rd| Reader {
            rd,
            plane: &self.plane,
        })
    }

    /// Writes an engine entry (author `qp`).
    pub(super) fn engine_entry(&mut self, changes: Vec<ChangeOp>, summary: String) -> Option<Seq> {
        self.engine_entry_with(changes, summary, None)
    }

    /// Writes an engine entry (author `qp`) carrying a claim's evidence (§19).
    pub(crate) fn engine_entry_with(
        &mut self,
        changes: Vec<ChangeOp>,
        summary: String,
        evidence: Option<qp_schema::protocol::ClaimEvidence>,
    ) -> Option<Seq> {
        // In place, no document copy: a change that does not fit rolls the earlier ones
        // back.
        let restore = crate::live::Restore {
            initial: &self.initial,
            log: &self.log,
        };
        let inverse = crate::live::apply_in_place(&mut self.doc, &changes, restore).ok()?;
        let seq = self.seq() + 1;
        self.committed(LogEntry {
            reason: None,
            seq,
            at: self.clock,
            author: ENGINE_PRINCIPAL.to_owned(),
            approved_by: None,
            cause: None,
            request: Vec::new(),
            changes,
            inverse,
            undoes: None,
            summary,
            evidence,
        });
        Some(seq)
    }

    /// After entry `seq`: engine follow-up entries and all notifications, in the order of
    /// §15 (per entry: change, finding_state, proposal; then condition_met, then
    /// intent_triggered).
    pub(crate) fn settle(&mut self, seq: Seq) -> Vec<OutboundLine> {
        if self.incremental && self.pending_stamps.is_empty() && self.bars_only(seq) {
            return self.settle_live(Some(seq));
        }
        let mut entries = vec![seq];
        let (mut conds, mut intents) = self.eval_watchers(seq);
        let mut data_entries = vec![seq];
        if let Some(s) = self.stamp_anchors() {
            entries.push(s);
            data_entries.push(s);
            let (c, i) = self.eval_watchers(s);
            conds.extend(c);
            intents.extend(i);
        }
        for d in data_entries {
            for s in self.invalidations(d) {
                entries.push(s);
                let (c, i) = self.eval_watchers(s);
                conds.extend(c);
                intents.extend(i);
            }
        }
        let mut lines = Vec::new();
        for s in entries {
            lines.extend(self.entry_events(s));
        }
        lines.extend(self.invalidate_in_public());
        lines.extend(self.run_capsules());
        lines.extend(self.view_events());
        lines.extend(conds);
        lines.extend(intents);
        lines.extend(self.claim_events());
        lines
    }
}
