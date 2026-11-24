//! The `command` message and new proposals.

use super::*;

impl Session {
    // -----------------------------------------------------------------------
    // Commands
    // -----------------------------------------------------------------------

    #[allow(clippy::too_many_lines)]
    pub(super) fn command(
        &mut self,
        ci: usize,
        id: &str,
        ops: &[Op],
        mode: WriteMode,
        (base, reason): (Option<Seq>, Option<String>),
        resolved: BTreeMap<String, String>,
    ) -> PResult<Vec<OutboundLine>> {
        if ops.is_empty() {
            return Err(invalid("a command needs at least one op", "/ops"));
        }
        // (R5B, §25) Human decisions and the agent write rule, before roles and modes.
        let mode = self.agent_check(ci, ops, mode)?;
        if ops.len() > 1 {
            if let Some(i) = ops
                .iter()
                .position(|o| matches!(o, Op::AcceptProposal { .. } | Op::Undo { .. }))
            {
                return Err(invalid(
                    format!("`{}` must be the only op of its command", op_name(&ops[i])),
                    ptr("/ops", i),
                ));
            }
        }
        let role = self.conns[ci].role;
        let perms = permissions(role);
        let principal = self.conns[ci].principal.clone();
        let forbid = |i: usize, why: String, hint: &str| -> ProtocolError {
            E::new(ErrorCode::Forbidden, why)
                .at(ptr("/ops", i))
                .hint(hint)
                .build()
        };
        let role_name = role_str(role);
        let first_denied = ops.iter().position(|o| !allowed(o, &perms));
        let first_unproposable = ops.iter().position(|o| !proposable(o));
        let direct = match mode {
            WriteMode::Direct => {
                if let Some(i) = first_denied {
                    let hint = if perms.propose && proposable(&ops[i]) {
                        "Use mode auto (or propose) to create a proposal instead."
                    } else {
                        "Ask a principal with the required role."
                    };
                    return Err(forbid(
                        i,
                        format!("role `{role_name}` may not `{}` directly", op_name(&ops[i])),
                        hint,
                    ));
                }
                true
            }
            WriteMode::Auto => match first_denied {
                None => true,
                Some(i) => {
                    if !perms.propose {
                        return Err(forbid(
                            i,
                            format!("role `{role_name}` may not `{}`", op_name(&ops[i])),
                            "Ask a principal with the required role.",
                        ));
                    }
                    if let Some(j) = first_unproposable {
                        return Err(forbid(
                            j,
                            format!(
                                "role `{role_name}` may not `{}`, and it cannot be proposed",
                                op_name(&ops[j])
                            ),
                            "Ask a principal with the required role.",
                        ));
                    }
                    false
                }
            },
            WriteMode::Propose => {
                if !perms.propose {
                    return Err(forbid(
                        0,
                        format!("role `{role_name}` may not propose"),
                        "Ask a principal with the propose permission.",
                    ));
                }
                if let Some(j) = first_unproposable {
                    return Err(forbid(
                        j,
                        format!("`{}` cannot be proposed", op_name(&ops[j])),
                        "Send it with mode direct if you have the permission.",
                    ));
                }
                false
            }
        };
        let author = self
            .principal(&principal)
            .map(|p| p.author.clone())
            .ok_or_else(|| E::new(ErrorCode::Handshake, "principal vanished").build())?;
        let seq = self.seq() + 1;
        let conn = self.conns[ci].id.clone();
        // (R3, §20) A principal that may not see some engine items gets the errors the
        // document it sees would give (no existence oracle): a dry run on it first.
        let public = self.public_view(&principal);
        // (R5B, §25.5) With another person's watches in the document, the reader's own
        // document answers too (no existence oracle through errors or suggestions).
        let watches = self.doc.intents.iter().any(|i| i.watch.is_some())
            && self.reader_proj(&principal).is_some();
        if (self.runner.is_some() && (public || self.proj_for(&principal).is_some())) || watches {
            let lang = self.lang_of(ci);
            let author = author.clone();
            let pr = principal.clone();
            self.as_reader(&principal, lang, |s| {
                let mut ctx = Ctx {
                    author,
                    approved_by: None,
                    at: s.clock,
                    seq: if public { s.pub_len() + 1 } else { seq },
                    id_seq: if public { s.pub_len() + 1 } else { seq },
                    n: 0,
                    created: Vec::new(),
                    speaker: &pr,
                    is_owner: role == Role::Owner,
                    can_decide: perms.decide,
                    log: &s.log,
                    undoes: None,
                    data: s.reader(),
                    plane: &s.plane,
                };
                if ops
                    .iter()
                    .any(|o| matches!(o, Op::Undo { .. } | Op::AcceptProposal { .. }))
                {
                    return Ok(());
                }
                expand_ops(&s.doc, ops, "/ops", &mut ctx).map(|_| ())
            })?;
        }
        // (R5B, §25.4) Comments are validated against what the writer may read: after
        // the role and mode checks and after the dry run in the writer's own document (a
        // hidden capsule is `not_found` like a missing one).
        self.check_comments(ci, ops)?;

        if direct {
            if self.incremental
                && ops
                    .iter()
                    .all(|o| matches!(o, Op::AppendBars { .. } | Op::UpdateLastBar { .. }))
            {
                return self.bar_command(ci, id, ops, (base, reason), resolved);
            }
            if let [Op::AcceptProposal { proposal, note }] = ops {
                return self.accept(
                    &conn,
                    id,
                    (proposal, note.clone()),
                    &principal,
                    (base, reason),
                    resolved,
                );
            }
            // (R3/K1b) With a public world: each world takes the command on its own.
            if self.other.is_some() {
                let w = Writer {
                    conn: &conn,
                    id,
                    principal: &principal,
                    author,
                    role,
                    perms: &perms,
                    public,
                };
                return self.direct_two_worlds(&w, ops, (base, reason), resolved);
            }
            let mut ctx = Ctx {
                author,
                approved_by: None,
                at: self.clock,
                seq,
                id_seq: seq,
                n: 0,
                created: Vec::new(),
                speaker: &principal,
                is_owner: role == Role::Owner,
                can_decide: perms.decide,
                log: &self.log,
                undoes: None,
                data: self.reader(),
                plane: &self.plane,
            };
            let ex = expand_ops(&self.doc, ops, "/ops", &mut ctx)?;
            let created = ctx.created;
            let undoes = ctx.undoes;
            self.base_check(base, &ex.changes, &ex.origin)?;
            let entry = LogEntry {
                reason: reason.clone(),
                seq,
                at: self.clock,
                author: principal.clone(),
                approved_by: None,
                cause: Some(id.to_owned()),
                request: ops.to_vec(),
                changes: ex.changes,
                inverse: ex.inverse,
                undoes,
                summary: match undoes {
                    Some(u) => format!("undo {u}"),
                    None => summarize_ops(ops, &created),
                },
                evidence: None,
            };
            self.commit(entry, ex.doc);
            let mut lines = vec![out(
                &conn,
                ServerMsg::Ack {
                    resolved,
                    id: id.to_owned(),
                    status: AckStatus::Applied,
                    seq: Some(seq),
                    created,
                    proposal: None,
                    undoes,
                },
            )];
            lines.extend(self.settle(seq));
            return Ok(lines);
        }

        // Proposal: validate by a dry run (ids assigned now, reused on acceptance).
        if self.other.is_some() {
            let w = Writer {
                conn: &conn,
                id,
                principal: &principal,
                author,
                role,
                perms: &perms,
                public,
            };
            return self.propose_two_worlds(&w, ops, (base, reason), resolved);
        }
        let mut ctx = Ctx {
            author: author.clone(),
            approved_by: None,
            at: self.clock,
            seq,
            id_seq: seq,
            n: 0,
            created: Vec::new(),
            speaker: &principal,
            is_owner: role == Role::Owner,
            can_decide: perms.decide,
            log: &self.log,
            undoes: None,
            data: self.reader(),
            plane: &self.plane,
        };
        let ex = expand_ops(&self.doc, ops, "/ops", &mut ctx)?;
        let created = ctx.created;
        self.base_check(base, &ex.changes, &ex.origin)?;
        let pid = format!("prop-{seq}");
        let changes = vec![ChangeOp::PutProposal {
            proposal: self.new_proposal(&pid, ops, reason.clone(), author),
            index: None,
        }];
        let restore = crate::live::Restore {
            initial: &self.initial,
            log: &self.log,
        };
        let inverse = crate::live::apply_in_place(&mut self.doc, &changes, restore)
            .map_err(|m| invalid(format!("cannot store proposal: {m}"), "/ops"))?;
        let entry = LogEntry {
            reason: reason.clone(),
            seq,
            at: self.clock,
            author: principal,
            approved_by: None,
            cause: Some(id.to_owned()),
            request: ops.to_vec(),
            changes,
            inverse,
            undoes: None,
            summary: format!("propose {pid}: {}", summarize_ops(ops, &created)),
            evidence: None,
        };
        self.committed(entry);
        let mut lines = vec![out(
            &conn,
            ServerMsg::Ack {
                resolved,
                id: id.to_owned(),
                status: AckStatus::Proposed,
                seq: Some(seq),
                created,
                proposal: Some(pid),
                undoes: None,
            },
        )];
        lines.extend(self.settle(seq));
        Ok(lines)
    }

    /// A pending proposal `pid` of `ops` by `author`, made now.
    pub(super) fn new_proposal(
        &self,
        pid: &str,
        ops: &[Op],
        reason: Option<String>,
        author: qp_schema::Author,
    ) -> Proposal {
        Proposal {
            id: pid.to_owned(),
            ops: ops.to_vec(),
            reason,
            state: ProposalState::Pending,
            provenance: qp_schema::Provenance {
                author,
                at: self.clock,
                based_on: Vec::new(),
                approved_by: None,
            },
            expires_at: self.ttl.map(|t| {
                self.clock
                    .saturating_add(i64::try_from(t).unwrap_or(i64::MAX))
            }),
            decided_by: None,
            note: None,
            decided_seq: None,
        }
    }
}
