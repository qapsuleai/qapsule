//! The two worlds (public and private writers): expansion, direct writes, proposals.

use super::*;

impl Session {
    /// (R3/K1b) `ops` of writer `w` expanded in the public world as its next entry (public
    /// sequence number and ids); `base` checked there when given.
    fn pub_expand(&mut self, w: &Writer<'_>, ops: &[Op], base: Option<Seq>) -> PResult<PubEx> {
        let author = w.author.clone();
        let (role, perms) = (w.role, w.perms.clone());
        self.in_world(true, |s| {
            let seq = s.seq() + 1;
            let mut ctx = Ctx {
                author,
                approved_by: None,
                at: s.clock,
                seq,
                id_seq: seq,
                n: 0,
                created: Vec::new(),
                speaker: w.principal,
                is_owner: role == Role::Owner,
                can_decide: perms.decide,
                log: &s.log,
                undoes: None,
                data: s.reader(),
                plane: &s.plane,
            };
            let ex = expand_ops(&s.doc, ops, "/ops", &mut ctx)?;
            s.base_check(base, &ex.changes, &ex.origin)?;
            Ok(PubEx {
                seq,
                changes: ex.changes,
                inverse: ex.inverse,
                created: ctx.created,
                undoes: ctx.undoes,
            })
        })
    }

    /// (R3/K1b) A direct command with a public world. Each world takes it on its own:
    /// the public world its own expansion (an entitled writer's `undo` mapped to the
    /// public entry it names), the authority's world its own (an entitled writer) or the
    /// public changes translated back, all or nothing (a public-view writer). The writer
    /// gets the answer of the world it reads; the other world's outcome leaves no trace
    /// in it.
    pub(super) fn direct_two_worlds(
        &mut self,
        w: &Writer<'_>,
        ops: &[Op],
        (base, reason): (Option<Seq>, Option<String>),
        resolved: BTreeMap<String, String>,
    ) -> PResult<Vec<OutboundLine>> {
        let summary = |ops: &[Op], undoes: Option<Seq>, created: &[String]| match undoes {
            Some(u) => format!("undo {u}"),
            None => summarize_ops(ops, created),
        };
        let pub_entry = |s: &Self, ops: &[Op], p: &PubEx| LogEntry {
            reason: reason.clone(),
            seq: p.seq,
            at: s.clock,
            author: w.principal.to_owned(),
            approved_by: None,
            cause: Some(w.id.to_owned()),
            request: ops.to_vec(),
            changes: p.changes.clone(),
            inverse: Vec::new(),
            undoes: p.undoes,
            summary: summary(ops, p.undoes, &p.created),
            evidence: None,
        };
        let ack = |seq: Seq, created: Vec<String>, undoes: Option<Seq>| {
            out(
                w.conn,
                ServerMsg::Ack {
                    resolved: resolved.clone(),
                    id: w.id.to_owned(),
                    status: AckStatus::Applied,
                    seq: Some(seq),
                    created,
                    proposal: None,
                    undoes,
                },
            )
        };
        if w.public {
            let p = self.pub_expand(w, ops, base)?;
            let pe = pub_entry(self, ops, &p);
            let mut lines = vec![ack(p.seq, p.created.clone(), p.undoes)];
            lines.extend(self.commit_public_writer(pe, &p.inverse));
            return Ok(lines);
        }
        // An entitled writer.
        let mut pub_ops = self.ops_to_pub(ops);
        // (R5B, §25.4) A comment enters the public world only if a public reader could
        // have written it (no licensed number, every cite public).
        if ops.iter().any(|o| matches!(o, Op::AddComment { .. })) {
            let ci = self.conn_index(w.conn);
            if !ci.is_some_and(|ci| self.comment_public_ok(ci, ops)) {
                pub_ops = None;
            }
        }
        let pub_side = match pub_ops.as_deref() {
            Some(o) => self.pub_expand(w, o, None).ok().map(|p| (o.to_vec(), p)),
            None => None,
        };
        let seq = self.seq() + 1;
        let mut ctx = Ctx {
            author: w.author.clone(),
            approved_by: None,
            at: self.clock,
            seq,
            id_seq: seq,
            n: 0,
            created: Vec::new(),
            speaker: w.principal,
            is_owner: w.role == Role::Owner,
            can_decide: w.perms.decide,
            log: &self.log,
            undoes: None,
            data: self.reader(),
            plane: &self.plane,
        };
        let full = expand_ops(&self.doc, ops, "/ops", &mut ctx)
            .map(|ex| (ex, ctx.created, ctx.undoes))
            .and_then(|(ex, c, u)| {
                self.base_check(base, &ex.changes, &ex.origin)?;
                Ok((ex, c, u))
            });
        match full {
            Ok((ex, created, undoes)) => {
                self.pending_pub = Some(match &pub_side {
                    Some((o, p)) => {
                        let pe = pub_entry(self, o, p);
                        PubNext::Entry(PubPending {
                            changes: pe.changes,
                            summary: Some(pe.summary),
                            evidence: None,
                            undoes: pe.undoes,
                            request: Some(pe.request),
                        })
                    }
                    None => PubNext::Skip,
                });
                let entry = LogEntry {
                    reason: reason.clone(),
                    seq,
                    at: self.clock,
                    author: w.principal.to_owned(),
                    approved_by: None,
                    cause: Some(w.id.to_owned()),
                    request: ops.to_vec(),
                    changes: ex.changes,
                    inverse: ex.inverse,
                    undoes,
                    summary: summary(ops, undoes, &created),
                    evidence: None,
                };
                self.commit(entry, ex.doc);
                let mut lines = vec![ack(seq, created, undoes)];
                lines.extend(self.settle(seq));
                Ok(lines)
            }
            Err(e) => {
                let mut lines = vec![err_line(w.conn, Some(w.id.to_owned()), e)];
                if let Some((o, p)) = pub_side {
                    let pe = pub_entry(self, &o, &p);
                    if let Some(ps) = self.commit_public(pe) {
                        lines.extend(self.entry_events_pub(ps));
                    }
                    lines.extend(self.invalidate_in_public());
                }
                Ok(lines)
            }
        }
    }

    /// (R3/K1b) A proposal with a public world: each world validates and stores it on its
    /// own (its own `prop-<seq>`); a public-view proposer's stays in the public world.
    pub(super) fn propose_two_worlds(
        &mut self,
        w: &Writer<'_>,
        ops: &[Op],
        (base, reason): (Option<Seq>, Option<String>),
        resolved: BTreeMap<String, String>,
    ) -> PResult<Vec<OutboundLine>> {
        let pub_entry = |s: &Self, ops: &[Op], p: &PubEx| {
            let pid = format!("prop-{}", p.seq);
            LogEntry {
                reason: reason.clone(),
                seq: p.seq,
                at: s.clock,
                author: w.principal.to_owned(),
                approved_by: None,
                cause: Some(w.id.to_owned()),
                request: ops.to_vec(),
                changes: vec![ChangeOp::PutProposal {
                    proposal: s.new_proposal(&pid, ops, reason.clone(), w.author.clone()),
                    index: None,
                }],
                inverse: Vec::new(),
                undoes: None,
                summary: format!("propose {pid}: {}", summarize_ops(ops, &p.created)),
                evidence: None,
            }
        };
        let ack = |seq: Seq, created: Vec<String>| {
            out(
                w.conn,
                ServerMsg::Ack {
                    resolved: resolved.clone(),
                    id: w.id.to_owned(),
                    status: AckStatus::Proposed,
                    seq: Some(seq),
                    created,
                    proposal: Some(format!("prop-{seq}")),
                    undoes: None,
                },
            )
        };
        if w.public {
            let p = self.pub_expand(w, ops, base)?;
            let pe = pub_entry(self, ops, &p);
            let pid = format!("prop-{}", p.seq);
            let mut lines = vec![ack(p.seq, p.created.clone())];
            // (R3/K1c) It stays in the public world: `reverse_changes` never takes a
            // proposal.
            lines.extend(self.commit_public_writer(pe, &[]));
            let _ = pid;
            return Ok(lines);
        }
        let pub_ops = self.ops_to_pub(ops);
        let pub_side = match pub_ops.as_deref() {
            Some(o) => self.pub_expand(w, o, None).ok().map(|p| (o.to_vec(), p)),
            None => None,
        };
        let seq = self.seq() + 1;
        let mut ctx = Ctx {
            author: w.author.clone(),
            approved_by: None,
            at: self.clock,
            seq,
            id_seq: seq,
            n: 0,
            created: Vec::new(),
            speaker: w.principal,
            is_owner: w.role == Role::Owner,
            can_decide: w.perms.decide,
            log: &self.log,
            undoes: None,
            data: self.reader(),
            plane: &self.plane,
        };
        let full = expand_ops(&self.doc, ops, "/ops", &mut ctx)
            .map(|ex| (ex, ctx.created))
            .and_then(|(ex, c)| {
                self.base_check(base, &ex.changes, &ex.origin)?;
                Ok(c)
            });
        let created = match full {
            Ok(c) => c,
            Err(e) => {
                let mut lines = vec![err_line(w.conn, Some(w.id.to_owned()), e)];
                if let Some((o, p)) = pub_side {
                    let pe = pub_entry(self, &o, &p);
                    if let Some(ps) = self.commit_public(pe) {
                        lines.extend(self.entry_events_pub(ps));
                    }
                }
                return Ok(lines);
            }
        };
        let pid = format!("prop-{seq}");
        let changes = vec![ChangeOp::PutProposal {
            proposal: self.new_proposal(&pid, ops, reason.clone(), w.author.clone()),
            index: None,
        }];
        let restore = crate::live::Restore {
            initial: &self.initial,
            log: &self.log,
        };
        let inverse = crate::live::apply_in_place(&mut self.doc, &changes, restore)
            .map_err(|m| invalid(format!("cannot store proposal: {m}"), "/ops"))?;
        self.pending_pub = Some(match &pub_side {
            Some((o, p)) => {
                let pe = pub_entry(self, o, p);
                PubNext::Entry(PubPending {
                    changes: pe.changes,
                    summary: Some(pe.summary),
                    evidence: None,
                    undoes: None,
                    request: Some(pe.request),
                })
            }
            None => PubNext::Skip,
        });
        let entry = LogEntry {
            reason: reason.clone(),
            seq,
            at: self.clock,
            author: w.principal.to_owned(),
            approved_by: None,
            cause: Some(w.id.to_owned()),
            request: ops.to_vec(),
            changes,
            inverse,
            undoes: None,
            summary: format!("propose {pid}: {}", summarize_ops(ops, &created)),
            evidence: None,
        };
        self.committed(entry);
        // (R3/K1c) Both worlds hold this proposal only when this command's own entries
        // (the authority's just committed, and the public one it made) put it.
        let put = |e: Option<&LogEntry>, id: &str| {
            e.is_some_and(|e| {
                e.changes.iter().any(
                    |c| matches!(c, ChangeOp::PutProposal { proposal, .. } if proposal.id == id),
                )
            })
        };
        if put(self.log.last().filter(|e| e.seq == seq), &pid) {
            if let Some(p) = self.pub_of(seq) {
                let ppid = format!("prop-{p}");
                let ok = self
                    .other
                    .as_ref()
                    .is_some_and(|w| put(w.log.last().filter(|e| e.seq == p), &ppid));
                if ok {
                    self.prop_map.insert(pid, ppid);
                }
            }
        }
        let mut lines = vec![ack(seq, created)];
        lines.extend(self.settle(seq));
        Ok(lines)
    }

    /// (R3/K1b) Commits public-view writer's entry `pe` (of the public world, not yet
    /// applied there; `inverse` its changes' inverse): the authority's world takes the
    /// changes translated back, all or nothing ([`Session::reverse_changes`]), as its own
    /// entry, which the public world then takes as `pe`; else only the public world
    /// takes `pe`. Returns the notifications.
    fn commit_public_writer(&mut self, pe: LogEntry, inverse: &[ChangeOp]) -> Vec<OutboundLine> {
        let rev = self
            .reverse_changes(&pe.changes, inverse)
            .filter(|r| !r.is_empty());
        let full_seq = self.seq() + 1;
        match rev.and_then(|r| self.apply_atomic(r)) {
            Some((changes, inverse)) => {
                self.pending_pub = Some(PubNext::Entry(PubPending {
                    changes: pe.changes.clone(),
                    summary: Some(pe.summary.clone()),
                    evidence: None,
                    undoes: pe.undoes,
                    request: Some(pe.request.clone()),
                }));
                self.committed(LogEntry {
                    seq: full_seq,
                    undoes: None,
                    changes,
                    inverse,
                    ..pe
                });
                self.settle(full_seq)
            }
            None => {
                let mut lines = Vec::new();
                if let Some(ps) = self.commit_public(pe) {
                    lines.extend(self.entry_events_pub(ps));
                }
                lines.extend(self.invalidate_in_public());
                lines
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn accept(
        &mut self,
        conn: &str,
        id: &str,
        (pid, note): (&str, Option<String>),
        decider: &str,
        (base, reason): (Option<Seq>, Option<String>),
        resolved: BTreeMap<String, String>,
    ) -> PResult<Vec<OutboundLine>> {
        let ack = |seq: Seq, created: Vec<String>| {
            out(
                conn,
                ServerMsg::Ack {
                    resolved: resolved.clone(),
                    id: id.to_owned(),
                    status: AckStatus::Applied,
                    seq: Some(seq),
                    created,
                    proposal: None,
                    undoes: None,
                },
            )
        };
        let entry_of = |a: &Accepted, seq: Seq, at: i64, pid: &str| LogEntry {
            reason: reason.clone().or_else(|| a.reason.clone()),
            seq,
            at,
            author: a.proposer.clone(),
            approved_by: Some(decider.to_owned()),
            cause: Some(id.to_owned()),
            request: a.ops.clone(),
            changes: a.changes.clone(),
            inverse: a.inverse.clone(),
            undoes: None,
            summary: format!("accept {pid}: {}", summarize_ops(&a.ops, &a.created)),
            evidence: None,
        };
        // (R3/K1b) With a public world each world accepts on its own: a public-view
        // decider's acceptance (of a public proposal) only in the public world, with its
        // answer; an entitled decider's in the authority's world (its answer) and, for
        // the public proposal the same entry made, in the public world.
        if self.other.is_some() && self.public_view(decider) {
            let n = note.clone();
            let a = self.in_world(true, |s| {
                let seq = s.seq() + 1;
                s.accept_expand(pid, n, decider, base, seq)
                    .map(|a| (a, seq))
            })?;
            let (a, ps) = a;
            let pe = entry_of(&a, ps, self.clock, pid);
            let mut lines = vec![ack(ps, a.created.clone())];
            lines.extend(self.commit_public_writer(pe, &a.inverse));
            return Ok(lines);
        }
        let seq = self.seq() + 1;
        let pub_side = if self.other.is_some() {
            self.proposal_to_pub(pid).and_then(|pp| {
                let n = note.clone();
                self.in_world(true, |s| {
                    let seq = s.seq() + 1;
                    s.accept_expand(&pp, n, decider, None, seq)
                        .ok()
                        .map(|a| (a, seq, pp.clone()))
                })
            })
        } else {
            None
        };
        let full = self.accept_expand(pid, note, decider, base, seq);
        let a = match full {
            Ok(a) => a,
            Err(e) => {
                let Some((p, ps, pp)) = pub_side else {
                    return Err(e);
                };
                let mut lines = vec![err_line(conn, Some(id.to_owned()), e)];
                let pe = entry_of(&p, ps, self.clock, &pp);
                if let Some(ps) = self.commit_public(pe) {
                    lines.extend(self.entry_events_pub(ps));
                }
                return Ok(lines);
            }
        };
        if self.other.is_some() {
            self.pending_pub = Some(match &pub_side {
                Some((p, ps, pp)) => {
                    let pe = entry_of(p, *ps, self.clock, pp);
                    PubNext::Entry(PubPending {
                        changes: pe.changes,
                        summary: Some(pe.summary),
                        evidence: None,
                        undoes: None,
                        request: Some(pe.request),
                    })
                }
                None => PubNext::Skip,
            });
        }
        let entry = entry_of(&a, seq, self.clock, pid);
        let created = a.created.clone();
        self.commit(entry, a.doc);
        let mut lines = vec![ack(seq, created)];
        lines.extend(self.settle(seq));
        Ok(lines)
    }
}
