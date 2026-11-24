//! Message dispatch: `handle`, the resolved handler, `seq` and `hello`.

use super::*;

impl Session {
    /// Handles a parsed message (its id fields hold ids; selectors are resolved by
    /// [`Session::handle_raw`]).
    pub fn handle(&mut self, conn: &str, msg: ClientMsg) -> Vec<OutboundLine> {
        self.handle_msg(conn, msg, BTreeMap::new())
    }

    #[allow(clippy::too_many_lines)]
    /// Handles a parsed message whose selectors resolved to `resolved` (§17).
    pub(super) fn handle_resolved(
        &mut self,
        conn: &str,
        msg: ClientMsg,
        resolved: BTreeMap<String, String>,
    ) -> Vec<OutboundLine> {
        let ci = self.conn_index(conn);
        if let ClientMsg::Hello {
            protocol,
            principal,
            surface,
            features,
        } = msg
        {
            return self.dispatch_hello(conn, ci, &protocol, &principal, surface, &features);
        }
        let Some(ci) = ci else {
            let id = msg_id(&msg);
            let e = E::new(
                ErrorCode::Handshake,
                "the first message on a connection must be `hello`",
            )
            .hint("Send {\"type\":\"hello\",\"protocol\":\"qp/0\",\"principal\":…,\"surface\":{\"kind\":…}} first.")
            .build();
            return vec![err_line(conn, id, e)];
        };
        match msg {
            ClientMsg::Hello { .. } => unreachable!("handled above"),
            ClientMsg::Command {
                id,
                ops,
                mode,
                base,
                reason,
            } => {
                let (full_len, pub_len) = (self.log.len(), self.pub_len() as usize);
                match self.command(ci, &id, &ops, mode, (base, reason), resolved) {
                    Ok(mut lines) => {
                        // (R5B, §25.4) New comments to `comments` subscribers.
                        if ops.iter().any(|o| matches!(o, Op::AddComment { .. })) {
                            lines.extend(self.comment_events(full_len, pub_len));
                        }
                        lines
                    }
                    Err(e) => vec![err_line(conn, Some(id), e)],
                }
            }
            ClientMsg::Query { id, query } => {
                if !permissions(self.conns[ci].role).read {
                    return vec![err_line(conn, Some(id), forbidden_read())];
                }
                let (principal, lang) = (self.conns[ci].principal.clone(), self.lang_of(ci));
                match self.as_reader(&principal, lang, |s| s.dispatch_query(ci, &query)) {
                    Ok(data) => vec![out(
                        conn,
                        ServerMsg::Result {
                            id,
                            seq: self.seq_to(&principal, self.seq()),
                            now: self.clock(),
                            data,
                            resolved,
                        },
                    )],
                    Err(e) => vec![err_line(conn, Some(id), e)],
                }
            }
            ClientMsg::Subscribe { id, topic } => {
                let (principal, lang) = (self.conns[ci].principal.clone(), self.lang_of(ci));
                let viewer = match &topic {
                    Topic::Viewer { limit, width } => Some((*limit, *width)),
                    _ => None,
                };
                let stories = matches!(topic, Topic::Stories { .. });
                let r = self.as_reader(&principal, lang, |s| s.dispatch_subscribe(ci, &id, topic));
                match r {
                    Ok(()) => {
                        let mut a = ack(id.clone(), AckStatus::Subscribed);
                        if let ServerMsg::Ack { resolved: r, .. } = &mut a {
                            *r = resolved;
                        }
                        let mut v = vec![out(conn, a)];
                        if let Some((limit, width)) = viewer {
                            v.extend(self.viewer_start(ci, &id, limit, width));
                        }
                        if stories {
                            self.stories_start(ci, &id);
                        }
                        v
                    }
                    Err(e) => vec![err_line(conn, Some(id), e)],
                }
            }
            ClientMsg::Unsubscribe { id, sub } => {
                let c = &mut self.conns[ci];
                if let Some(i) = c.subs.iter().position(|s| s.id == sub) {
                    c.subs.remove(i);
                    self.unsubscribed(conn, &sub);
                    vec![out(conn, ack(id, AckStatus::Unsubscribed))]
                } else {
                    let e = E::new(ErrorCode::NotFound, format!("no subscription `{sub}`"))
                        .at("/sub")
                        .dym(&sub, c.subs.iter().map(|s| s.id.as_str()))
                        .build();
                    vec![err_line(conn, Some(id), e)]
                }
            }
            ClientMsg::Input { id, gesture } => {
                if !permissions(self.conns[ci].role).read {
                    return vec![err_line(conn, Some(id), forbidden_read())];
                }
                let (principal, lang) = (self.conns[ci].principal.clone(), self.lang_of(ci));
                match self.as_reader(&principal, lang, |s| s.dispatch_input(ci, &gesture)) {
                    Ok((data, mut events)) => {
                        let mut v = vec![out(
                            conn,
                            ServerMsg::Result {
                                id,
                                seq: self.seq_to(&principal, self.seq()),
                                now: self.clock(),
                                data,
                                resolved,
                            },
                        )];
                        v.append(&mut events);
                        v
                    }
                    Err(e) => vec![err_line(conn, Some(id), e)],
                }
            }
            ClientMsg::Attend { id, directive } => {
                let (principal, lang) = (self.conns[ci].principal.clone(), self.lang_of(ci));
                let r = self.as_reader(&principal, lang, |s| s.attend(ci, directive));
                match r {
                    Ok((did, mut events)) => {
                        let mut v = vec![out(
                            conn,
                            ServerMsg::Ack {
                                resolved,
                                id,
                                status: AckStatus::Delivered,
                                seq: None,
                                created: vec![did],
                                proposal: None,
                                undoes: None,
                            },
                        )];
                        v.append(&mut events);
                        v
                    }
                    Err(e) => vec![err_line(conn, Some(id), e)],
                }
            }
            ClientMsg::Clock { now } => match self.tick(ci, now) {
                Ok(lines) => lines,
                Err(e) => vec![err_line(conn, None, e)],
            },
            ClientMsg::Dispute {
                id,
                claim,
                t,
                values,
                anchor_t,
                digest,
            } => {
                if !permissions(self.conns[ci].role).read {
                    return vec![err_line(conn, Some(id), forbidden_read())];
                }
                let d = crate::dispute::DisputeIn {
                    claim,
                    t,
                    values,
                    anchor_t,
                    digest,
                };
                match self.dispatch_dispute(ci, &id, d) {
                    Ok(lines) => lines,
                    Err(e) => vec![err_line(conn, Some(id), e)],
                }
            }
            ClientMsg::Visible { id, sub, visible } => {
                if !permissions(self.conns[ci].role).read {
                    return vec![err_line(conn, Some(id), forbidden_read())];
                }
                self.visible_msg(ci, id, &sub, visible)
            }
            // (R4, §21) Answered by a replay host (`crate::replayhost`) before it gets
            // here; a bare authority has no replay source.
            ClientMsg::Replay { id, .. } => vec![err_line(
                conn,
                Some(id),
                E::new(ErrorCode::Unsupported, "this authority has no replay source")
                    .hint("Replay controls work on a session started with `init.replay` (docs/PROTOCOL.md §21).")
                    .build(),
            )],
        }
    }

    pub(crate) fn seq(&self) -> Seq {
        self.log.len() as Seq
    }

    pub(super) fn hello(
        &mut self,
        conn: &str,
        ci: Option<usize>,
        protocol: &str,
        principal: &str,
        surface: Surface,
        features: &[String],
    ) -> Vec<OutboundLine> {
        let fail = |e: E| vec![err_line(conn, None, e.build())];
        if ci.is_some() {
            return fail(E::new(
                ErrorCode::Handshake,
                format!("connection `{conn}` already said hello"),
            ));
        }
        if protocol != PROTOCOL_VERSION {
            return fail(
                E::new(
                    ErrorCode::Handshake,
                    format!("unsupported protocol `{protocol}`"),
                )
                .at("/protocol")
                .expected([PROTOCOL_VERSION]),
            );
        }
        let Some(p) = self.principal(principal).cloned() else {
            return fail(
                E::new(
                    ErrorCode::Handshake,
                    format!("unknown principal `{principal}`"),
                )
                .at("/principal")
                .dym(principal, self.principals.iter().map(|p| p.id.as_str())),
            );
        };
        let profile = profile_for(surface.kind, surface.max_depth);
        let mut granted: Vec<String> = Vec::new();
        for f in features {
            if FEATURES.contains(&f.as_str()) && !granted.contains(f) {
                granted.push(f.clone());
            }
        }
        self.conns.push(Conn {
            id: conn.to_owned(),
            principal: p.id.clone(),
            role: p.role,
            surface,
            profile: profile.clone(),
            subs: Vec::new(),
        });
        self.looks.push(qp_schema::protocol::ViewState::default());
        vec![out(
            conn,
            ServerMsg::Welcome {
                // Only principals that may read get the summary (a feed may not).
                summary: permissions(p.role).read.then(|| {
                    let lang = self
                        .conns
                        .last()
                        .and_then(|c| c.surface.lang)
                        .unwrap_or_default();
                    let pid = p.id.clone();
                    self.as_reader(&pid, lang, |s| s.summary_for(&pid))
                }),
                protocol: PROTOCOL_VERSION.to_owned(),
                doc: self.doc.id.clone(),
                principal: p.id.clone(),
                role: p.role,
                permissions: permissions(p.role),
                seq: self.seq_to(&p.id, self.seq()),
                now: self.clock(),
                epoch: self.epoch,
                profile,
                features: granted,
                limits: self.limits.clone(),
            },
        )]
    }
}
