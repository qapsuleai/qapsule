//! Accepting and committing entries: base checks, commit, writers, anchors.

use super::*;

impl Session {
    /// The acceptance of proposal `pid` by `decider` as entry `seq`, expanded and
    /// `base`-checked in the world the fields hold.
    pub(crate) fn accept_expand(
        &self,
        pid: &str,
        note: Option<String>,
        decider: &str,
        base: Option<Seq>,
        seq: Seq,
    ) -> PResult<Accepted> {
        let pr = need_proposal(&self.doc, pid, "/ops/0/proposal")?.clone();
        let proposer = self.principal_of(&pr.provenance.author);
        let proposer_owner = self
            .principal(&proposer)
            .is_some_and(|p| p.role == Role::Owner);
        let decider_author = self
            .principal(decider)
            .map(|p| p.author.clone())
            .ok_or_else(|| E::new(ErrorCode::Handshake, "principal vanished").build())?;
        let id_seq = pid
            .strip_prefix("prop-")
            .and_then(|s| s.parse::<Seq>().ok())
            .unwrap_or(seq);
        let mut ctx = Ctx {
            author: pr.provenance.author.clone(),
            approved_by: Some(decider_author.clone()),
            at: self.clock,
            seq,
            id_seq,
            n: 0,
            created: Vec::new(),
            speaker: &proposer,
            is_owner: proposer_owner,
            can_decide: false,
            log: &self.log,
            undoes: None,
            data: self.reader(),
            plane: &self.plane,
        };
        let ex =
            expand_ops(&self.registry, &self.doc, &pr.ops, "/ops", &mut ctx).map_err(|mut e| {
                e.message = format!(
                    "proposal `{pid}` cannot be applied now: {} (at {} of the proposal)",
                    e.message,
                    e.path.as_deref().unwrap_or("/ops")
                );
                e.path = Some("/ops/0".to_owned());
                e.hint = Some(
                    "The proposal stays pending; reject it or fix the document first.".to_owned(),
                );
                e
            })?;
        let created = ctx.created;
        let mut decided = pr.clone();
        decided.state = ProposalState::Accepted;
        decided.decided_by = Some(decider_author);
        decided.note = note;
        decided.decided_seq = Some(seq);
        let mut doc = ex.doc;
        let mut changes = ex.changes;
        let tail = vec![ChangeOp::PutProposal {
            proposal: decided,
            index: None,
        }];
        let mut inverse = apply_all(&mut doc, &tail)
            .map_err(|m| invalid(format!("cannot store proposal: {m}"), "/ops/0"))?;
        inverse.extend(ex.inverse);
        changes.extend(tail);
        let origin = vec![0; changes.len()];
        self.base_check(base, &changes, &origin)?;
        Ok(Accepted {
            doc,
            changes,
            inverse,
            created,
            proposer,
            reason: pr.reason.clone(),
            ops: pr.ops,
        })
    }

    /// `base` check (§8.2): an item the command touches changed in an entry after
    /// `base` that is not undone (undo pairs both after `base` cancel out).
    pub(crate) fn base_check(
        &self,
        base: Option<Seq>,
        changes: &[ChangeOp],
        origin: &[usize],
    ) -> PResult<()> {
        let Some(b) = base else { return Ok(()) };
        if b > self.seq() {
            // A base from the future cannot be checked; accepting it would silently
            // switch conflict detection off (audit O2), e.g. for a stale epoch (§8.4).
            return Err(E::new(
                ErrorCode::Invalid,
                format!("base {b} is after the current log position {}", self.seq()),
            )
            .at("/base")
            .expected([format!("0..={}", self.seq())])
            .hint("Use the seq from `welcome` or from a `result`; after a hand-over, reconnect.")
            .build());
        }
        let undone = undone_set(&self.log);
        let mut conflicts = Vec::new();
        let mut first: Option<(String, Seq, String, usize)> = None;
        for e in self.log.iter().filter(|e| e.seq > b) {
            if undone.contains(&e.seq) || e.undoes.is_some_and(|x| x > b) {
                continue;
            }
            let t = touched_keys(&e.changes);
            for (k, c) in changes.iter().enumerate() {
                let key = std::iter::once(touched_key(c)).collect();
                if let Some(id) = first_overlap(&key, &t) {
                    if first.is_none() {
                        first = Some((id, e.seq, e.author.clone(), origin[k]));
                    }
                    conflicts.push(e.seq);
                    break;
                }
            }
        }
        let Some((id, s, by, op)) = first else {
            return Ok(());
        };
        conflicts.sort_unstable();
        conflicts.dedup();
        let latest = conflicts.last().copied().unwrap_or(s);
        Err(E::new(
            ErrorCode::Conflict,
            format!("`{id}` changed after base {b} (seq {s} by {by})"),
        )
        .at(ptr("/ops", op))
        .conflicts(conflicts)
        .hint(format!(
            "Read the item again and resend with base={latest}, or omit base."
        ))
        .build())
    }

    pub(super) fn commit(&mut self, entry: LogEntry, doc: Document) {
        self.doc = doc;
        self.committed(entry);
    }

    /// Records `entry`, whose changes `self.doc` already holds (applied in place or by
    /// replacing the document): the data plane, the reading document, the values cache
    /// and the view index follow, then the entry joins the log.
    pub(crate) fn committed(&mut self, entry: LogEntry) {
        let series_set = entry
            .changes
            .iter()
            .any(|c| matches!(c, ChangeOp::PutSeries { .. } | ChangeOp::Delete { .. }));
        let bars = crate::live::bar_deltas(&self.doc, &entry.changes);
        if !self.incremental && (series_set || !bars.is_empty()) {
            self.cache.clear();
        }
        if series_set {
            // The data plane follows the document (§18.3): bindings put, changed or
            // deleted are resolved or dropped; the reading document mirrors the document.
            self.pending_stamps = self.plane.sync(&self.doc);
            self.rd = reading_doc(&self.doc, &self.plane);
            if self.incremental {
                let mut ids: Vec<&str> = entry
                    .changes
                    .iter()
                    .filter_map(|c| match c {
                        ChangeOp::PutSeries { series, .. } => Some(series.id.as_str()),
                        ChangeOp::Delete { id } => Some(id.as_str()),
                        _ => None,
                    })
                    .collect();
                ids.extend(bars.iter().map(|(id, _)| id.as_str()));
                self.cache.forget(rd!(self), &ids);
            }
            self.view_reads = None;
        } else {
            if let Some(rd) = self.rd.as_mut() {
                if apply_all(rd, &entry.changes).is_err() {
                    self.rd = reading_doc(&self.doc, &self.plane);
                    self.head_state.dirty_all = true;
                }
            }
            if !entry.changes.iter().all(crate::live::keeps_view_reads) {
                self.view_reads = None;
            }
            if self.incremental && !bars.is_empty() {
                self.cache.begin_tick();
                for (id, d) in &bars {
                    self.cache.apply_delta(rd!(self), id, *d);
                }
            }
        }
        if !bars.is_empty() {
            self.claims_after_bars(&bars);
        }
        for (id, d) in bars {
            self.pending_views.push((id, d, entry.seq));
        }
        if !entry.changes.iter().all(|c| {
            matches!(
                c,
                ChangeOp::AppendBars { .. }
                    | ChangeOp::SetLastBar { .. }
                    | ChangeOp::TruncateBars { .. }
            )
        }) {
            self.struct_gen += 1;
        }
        self.note_writers(&entry);
        self.note_audiences(&entry);
        // (R3/K1) The public world takes its own version of the entry (same seq).
        debug_assert!(
            !self.in_public,
            "entries are committed in the authority's world"
        );
        let public = self.other.is_some().then(|| self.public_entry(&entry));
        self.log.push(entry);
        if let Some(pe) = public {
            let p = pe.and_then(|pe| self.commit_public(pe));
            self.full_to_pub.push(p);
            self.note_pub_len();
        }
        self.phase(Phase::AfterCommit, Moment::Entry { seq: self.seq() });
    }

    /// Notes the principals whose capsule preferences or watches `entry` writes (other
    /// readers get projections, §20, §25.5).
    fn note_audiences(&mut self, entry: &LogEntry) {
        for c in entry.changes.iter().chain(&entry.inverse) {
            if let ChangeOp::PutAudience { principal, .. } = c {
                self.audience_seen.insert(principal.clone());
            }
            // (R5B, §25.5) A watch is personal too: other readers get projections.
            if let ChangeOp::PutIntent { intent, .. } = c {
                if let Some(w) = &intent.watch {
                    self.audience_seen.insert(w.for_.clone());
                }
            }
        }
    }

    /// Notes who wrote the series and finding invalidations `entry` puts (§18.3: the
    /// principals responsible for a condition). Engine entries write no meaning of their
    /// own (an anchor stamp keeps the binding's writer).
    pub(crate) fn note_writers(&mut self, entry: &LogEntry) {
        if entry.author == ENGINE_PRINCIPAL {
            return;
        }
        for c in &entry.changes {
            match c {
                ChangeOp::PutSeries { series, .. } => {
                    self.series_writer
                        .insert(series.id.clone(), entry.author.clone());
                }
                ChangeOp::PutFinding { finding, .. } => {
                    let before = entry.inverse.iter().find_map(|i| match i {
                        ChangeOp::PutFinding { finding: old, .. } if old.id == finding.id => {
                            Some(&old.invalidation)
                        }
                        _ => None,
                    });
                    if before != Some(&finding.invalidation) {
                        self.invalidation_writer
                            .insert(finding.id.clone(), entry.author.clone());
                    }
                }
                ChangeOp::Delete { id } => {
                    self.series_writer.remove(id);
                    self.invalidation_writer.remove(id);
                }
                _ => {}
            }
        }
    }

    /// The principals of this session responsible for finding `f`'s invalidation, who must
    /// all be entitled to every bound feed it reads for it to be evaluated (§18.3): the
    /// finding's author, whoever last wrote its invalidation, and whoever last wrote each
    /// series it rests on (`closure`, through every study input). Authors that are not
    /// principals of this session (another document, the engine) are left out: the
    /// authority's own view.
    pub(crate) fn responsible(&self, f: &qp_schema::Finding, closure: &[String]) -> Vec<String> {
        let mut who: Vec<String> = Vec::new();
        let mut add = |p: &str| {
            if self.principal(p).is_some() && !who.iter().any(|w| w == p) {
                who.push(p.to_owned());
            }
        };
        add(&self.principal_of(&f.provenance.author));
        if let Some(p) = self.invalidation_writer.get(&f.id) {
            add(p);
        }
        for id in closure {
            if let Some(p) = self.series_writer.get(id) {
                add(p);
            }
        }
        who
    }

    /// Writes the pending anchor stamps (§18.3) as one engine entry: `put_series` of each
    /// bound series with `anchor_t` set to the first bar of its first ready resolution.
    pub(crate) fn stamp_anchors(&mut self) -> Option<Seq> {
        let stamps = std::mem::take(&mut self.pending_stamps);
        let mut changes = Vec::new();
        let mut names = Vec::new();
        for (id, t) in stamps {
            let Some(mut s) = self.doc.series.iter().find(|s| s.id == id).cloned() else {
                continue;
            };
            if let qp_schema::SeriesSource::Bind { anchor_t, .. } = &mut s.source {
                if anchor_t.is_some() {
                    continue;
                }
                *anchor_t = Some(t);
            } else {
                continue;
            }
            names.push(format!("{id} at t={t}"));
            changes.push(ChangeOp::PutSeries {
                series: s,
                index: None,
            });
        }
        if changes.is_empty() {
            return None;
        }
        self.engine_entry(changes, format!("anchor {}", names.join(", ")))
    }
}
