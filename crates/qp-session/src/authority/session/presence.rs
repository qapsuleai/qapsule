//! Subscriptions, viewports, attention and per-principal summaries.

use super::*;

impl Session {
    // -----------------------------------------------------------------------
    // Subscriptions, clock
    // -----------------------------------------------------------------------

    pub(super) fn subscribe(&mut self, ci: usize, id: &str, topic: Topic) -> PResult<()> {
        if !permissions(self.conns[ci].role).read {
            return Err(forbidden_read());
        }
        let c = &self.conns[ci];
        if c.subs.iter().any(|s| s.id == id) {
            return Err(E::new(
                ErrorCode::Duplicate,
                format!("subscription `{id}` already exists on this connection"),
            )
            .at("/id")
            .build());
        }
        if c.subs.len() >= self.limits.max_subscriptions as usize {
            return Err(E::new(
                ErrorCode::Limit,
                format!(
                    "a connection holds at most {} subscriptions",
                    self.limits.max_subscriptions
                ),
            )
            .expected([self.limits.max_subscriptions.to_string()])
            .hint("Unsubscribe from something first.")
            .build());
        }
        let mut last = None;
        match &topic {
            Topic::Finding { finding } => {
                need_finding(&self.doc, finding, "/topic/finding")?;
            }
            Topic::Condition { condition } => {
                if matches!(condition, qp_schema::Condition::Text { .. }) {
                    return Err(invalid(
                        "text conditions are not machine-checkable",
                        "/topic/condition/kind",
                    ));
                }
                check_condition(&self.doc, condition, "/topic/condition")?;
                crate::expand::condition_entitled(
                    &self.plane,
                    &self.doc,
                    &self.conns[ci].principal,
                    condition,
                    "/topic/condition",
                )?;
                last = Some(holds(rd!(self), &mut self.cache, condition));
            }
            Topic::Attention { principal: Some(p) } => {
                if self.principal(p).is_none() {
                    return Err(
                        E::new(ErrorCode::NotFound, format!("unknown principal `{p}`"))
                            .at("/topic/principal")
                            .dym(p, self.principals.iter().map(|x| x.id.as_str()))
                            .build(),
                    );
                }
            }
            Topic::View { view } => {
                crate::expand::need_view(&self.doc, view, "/topic/view")?;
            }
            Topic::Viewer { limit, width } => {
                self.viewer_check(ci, *limit, *width)?;
            }
            Topic::Changes { .. }
            | Topic::Proposals {}
            | Topic::Attention { principal: None }
            | Topic::Intents {}
            | Topic::Claims {}
            | Topic::Capsules { .. } => {}
            Topic::Stories { instrument, series } => {
                self.stories_check(instrument, series.as_deref())?;
            }
            // (R5B, §25.4) Comments: any filter (a capsule or an instrument may come later).
            Topic::Comments { .. } => {}
        }
        self.conns[ci].subs.push(Sub {
            id: id.to_owned(),
            topic,
            last,
            memo: None,
            watch: None,
            stories: None,
        });
        Ok(())
    }

    /// True if `c` reads a series resting on a bound series that is not ready for the
    /// authority (pending, no data, stale): the condition is not evaluated (§18.3).
    pub(crate) fn cond_blocked(&self, c: &qp_schema::Condition) -> bool {
        crate::live::cond_blocked(&self.plane, &self.doc, c)
    }

    /// The `welcome` summary for `principal`: over the reading document (`len` is the
    /// number of bars the authority holds), each bound series with its binding state for
    /// that principal (§18.2).
    pub(crate) fn summary_for(&self, principal: &str) -> qp_schema::protocol::DocSummary {
        let mut sum = crate::summary::summarize_doc(rd!(self));
        for s in &mut sum.series {
            if let Some((state, _)) = self.plane.state_for(&s.id, Some(principal)) {
                s.binding = Some(state);
            }
        }
        sum
    }

    /// Notifications of a viewport change of `principal` to its attention watchers.
    pub(crate) fn viewport_events(&self, principal: &str, vp: &Viewport) -> Vec<OutboundLine> {
        let mut lines = Vec::new();
        for c in &self.conns {
            for s in &c.subs {
                if let Topic::Attention { principal: p } = &s.topic {
                    if p.as_deref().unwrap_or(&c.principal) == principal {
                        lines.push(out(
                            &c.id,
                            ServerMsg::Notify {
                                sub: s.id.clone(),
                                event: Event::Viewport {
                                    principal: principal.to_owned(),
                                    viewport: vp.clone(),
                                },
                            },
                        ));
                    }
                }
            }
        }
        lines
    }

    /// Attention directive delivery (§4.8).
    pub(super) fn attend(
        &mut self,
        ci: usize,
        mut d: AttentionDirective,
    ) -> PResult<(String, Vec<OutboundLine>)> {
        if !permissions(self.conns[ci].role).attend {
            return Err(E::new(
                ErrorCode::Forbidden,
                format!(
                    "role `{}` may not send attention directives",
                    role_str(self.conns[ci].role)
                ),
            )
            .build());
        }
        if self.principal(&d.to).is_none() {
            return Err(
                E::new(ErrorCode::NotFound, format!("unknown principal `{}`", d.to))
                    .at("/directive/to")
                    .dym(&d.to, self.principals.iter().map(|x| x.id.as_str()))
                    .build(),
            );
        }
        if d.steps.is_empty() {
            return Err(invalid(
                "a directive needs at least one step",
                "/directive/steps",
            ));
        }
        for (j, s) in d.steps.iter().enumerate() {
            crate::query::check_target_pub(
                &self.doc,
                &s.target,
                &format!("/directive/steps/{j}/target"),
            )?;
        }
        if let Some(i) = &d.id {
            if !crate::expand::valid_id(i) {
                return Err(invalid(format!("invalid id `{i}`"), "/directive/id"));
            }
        }
        self.att_counter += 1;
        let id =
            d.id.clone()
                .unwrap_or_else(|| format!("att-{}", self.att_counter));
        d.id = Some(id.clone());
        d.from = Some(self.conns[ci].principal.clone());
        d.at = Some(self.clock);
        let list = self.directives.entry(d.to.clone()).or_default();
        list.push(d.clone());
        if list.len() > MAX_DIRECTIVES {
            let extra = list.len() - MAX_DIRECTIVES;
            list.drain(..extra);
        }
        let mut lines = Vec::new();
        for c in &self.conns {
            for s in &c.subs {
                if let Topic::Attention { principal: p } = &s.topic {
                    if p.as_deref().unwrap_or(&c.principal) == d.to {
                        lines.push(out(
                            &c.id,
                            ServerMsg::Notify {
                                sub: s.id.clone(),
                                event: Event::Attention {
                                    directive: d.clone(),
                                },
                            },
                        ));
                    }
                }
            }
        }
        Ok((id, lines))
    }
}
