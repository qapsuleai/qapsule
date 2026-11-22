//! Messages and replay control: frames, `replay` controls, rebuild, steps and pumping.

use super::*;

impl ReplayHost {
    /// One raw inbound message on `conn` (the in-process transport): a `replay` message
    /// of a connection that said `hello` is answered here, everything else goes to the
    /// authority unchanged (which also reports a malformed `replay` message).
    pub fn handle_raw(&mut self, conn: &str, line: &str) -> Vec<OutboundLine> {
        match self.as_replay(conn, line) {
            Some((id, control)) => self.control_msg(conn, id, control),
            None => self.session.handle_raw(conn, line),
        }
    }

    /// One line of the line transport (`{"conn":…,"msg":…}`), as
    /// [`Session::handle_frame`]; framing errors are the authority's.
    pub fn handle_frame(&mut self, line: &str) -> Vec<OutboundLine> {
        let v: Option<serde_json::Value> = serde_json::from_str(line).ok();
        let framed = v.as_ref().and_then(|v| {
            let o = v.as_object()?;
            if o.keys().any(|k| k != "conn" && k != "msg") {
                return None;
            }
            let conn = o.get("conn")?.as_str()?;
            let text = match o.get("msg")? {
                serde_json::Value::String(s) => s.clone(),
                m => m.to_string(),
            };
            Some((conn.to_owned(), text))
        });
        match framed {
            Some((conn, text)) => self.handle_raw(&conn, &text),
            None => self.session.handle_frame(line),
        }
    }

    /// A data event on the host's path, outside the replay (a transcript's `data` step).
    ///
    /// # Errors
    /// The authority rejected it.
    pub fn data_event(&mut self, ev: &DataEvent) -> Result<Vec<OutboundLine>, crate::DataError> {
        let mut lines = self.session.data_event(ev)?;
        lines.extend(self.session.viewer_flush());
        Ok(lines)
    }

    /// (RP3/M15, §22) What a host serves a viewer for feed `key` when it subscribes to it:
    /// the feed's initial bars and the data events applied since (while recording,
    /// [`ReplayHost::set_record_events`]). `None`: no such feed.
    #[must_use]
    pub fn feed_data(&self, key: &qp_schema::data::FeedKey) -> Option<(InitFeed, Vec<DataEvent>)> {
        let f = self
            .init
            .data
            .as_ref()?
            .feeds
            .iter()
            .find(|f| &f.feed == key)?
            .clone();
        Some((f, self.recorded.of_feed(key)))
    }

    fn as_replay(&self, conn: &str, line: &str) -> Option<(String, ReplayControl)> {
        if line.len() as u64 > self.session.limits().max_message_bytes
            || !line.contains("\"replay\"")
        {
            return None;
        }
        let v: serde_json::Value = serde_json::from_str(line).ok()?;
        if v.get("type").and_then(serde_json::Value::as_str) != Some("replay") {
            return None;
        }
        self.session.conns.iter().find(|c| c.id == conn)?;
        match serde_json::from_value::<ClientMsg>(v).ok()? {
            ClientMsg::Replay { id, control } => Some((id, control)),
            _ => None,
        }
    }

    fn error(conn: &str, id: &str, e: ProtocolError) -> Vec<OutboundLine> {
        vec![OutboundLine {
            conn: conn.to_owned(),
            msg: ServerMsg::Error {
                id: Some(id.to_owned()),
                error: e,
            },
        }]
    }

    /// A `replay` message of connection `conn` (it said `hello`).
    fn control_msg(&mut self, conn: &str, id: String, control: ReplayControl) -> Vec<OutboundLine> {
        let Some((principal, role)) = self
            .session
            .conns
            .iter()
            .find(|c| c.id == conn)
            .map(|c| (c.principal.clone(), c.role))
        else {
            return Vec::new();
        };
        let status = matches!(control, ReplayControl::Status);
        // (R5B audit B6, §25.3) an agent never controls the shared timeline, whatever its
        // role: a marked connection or an agent principal reads the status only.
        let agent = self
            .session
            .conns
            .iter()
            .position(|c| c.id == conn)
            .is_some_and(|ci| self.session.agent_origin(ci));
        let allowed = if status {
            permissions(role).read
        } else {
            matches!(role, Role::Host | Role::Owner) && !agent
        };
        if !allowed {
            let e = if status {
                E::new(
                    ErrorCode::Forbidden,
                    "this role may not read the replay status",
                )
            } else {
                E::new(
                    ErrorCode::Forbidden,
                    if agent {
                        "an agent does not control the replay (only a person or the host)"
                    } else {
                        "only the roles `host` and `owner` control the replay"
                    },
                )
                .hint("Ask for `{\"kind\":\"status\"}`; the timeline is shared by every reader.")
            };
            return Self::error(conn, &id, e.at("/control").build());
        }
        let (state, lines) = match self.host_control(&control) {
            Ok(x) => x,
            Err(e) => return Self::error(conn, &id, e),
        };
        let seq = if state.reset {
            self.session.log().len() as u64
        } else {
            self.session
                .seq_to(&principal, self.session.log().len() as u64)
        };
        let mut out = vec![OutboundLine {
            conn: conn.to_owned(),
            msg: ServerMsg::Result {
                id,
                seq,
                now: self.session.clock(),
                data: QueryResult::Replay { replay: state },
                resolved: BTreeMap::new(),
            },
        }];
        out.extend(lines);
        out
    }

    /// Applies a control on the host's own authority (no connection, no permission
    /// check): the state after it and the notifications its steps caused. State-changing
    /// controls are recorded ([`ReplayHost::controls`]).
    ///
    /// # Errors
    /// An invalid control (`invalid`, with the JSON pointer of the field).
    pub fn host_control(
        &mut self,
        control: &ReplayControl,
    ) -> Result<(ReplayState, Vec<OutboundLine>), ProtocolError> {
        let at = self.replay.cursor().step;
        let mut lines = Vec::new();
        let mut reset = false;
        if self.live.is_some() && !matches!(control, ReplayControl::Status) {
            return Err(
                E::new(ErrorCode::Invalid, "a live host has no timeline to control")
                    .at("/control")
                    .expected(["status"])
                    .build(),
            );
        }
        match control {
            ReplayControl::Status => return Ok((self.state(), lines)),
            ReplayControl::Pause => {
                self.paused = true;
                self.anchor = None;
            }
            ReplayControl::Resume => {
                self.paused = false;
                self.anchor = None;
            }
            ReplayControl::Speed { steps_per_s } => {
                let s = *steps_per_s;
                if !(s > 0.0 && s <= MAX_SPEED) {
                    return Err(E::new(
                        ErrorCode::Invalid,
                        format!("speed {s} is not in (0, {MAX_SPEED}] steps per second"),
                    )
                    .at("/control/steps_per_s")
                    .expected([format!("0 < steps_per_s <= {MAX_SPEED}")])
                    .build());
                }
                self.speed = s;
                self.anchor = None;
            }
            ReplayControl::Step { n } => {
                let n = n.unwrap_or(1);
                if n == 0 || n > MAX_STEP_N {
                    return Err(E::new(
                        ErrorCode::Invalid,
                        format!("`n` is {n}; take 1 to {MAX_STEP_N} steps"),
                    )
                    .at("/control/n")
                    .expected([format!("1..={MAX_STEP_N}")])
                    .build());
                }
                lines = self.take(u64::from(n));
                self.anchor = None;
            }
            ReplayControl::Seek { t, date, step } => {
                let target = match (t, date, step) {
                    (Some(t), None, None) => self.replay.steps_until(*t),
                    (None, Some(d), None) => {
                        let start = qp_source::date_start(d).map_err(|e| {
                            E::new(ErrorCode::Invalid, format!("`date`: {e}"))
                                .at("/control/date")
                                .expected(["YYYY-MM-DD"])
                                .build()
                        })?;
                        self.replay.steps_until(start - 1)
                    }
                    (None, None, Some(s)) => *s,
                    _ => {
                        return Err(E::new(
                            ErrorCode::Invalid,
                            "`seek` takes exactly one of `t`, `date`, `step`",
                        )
                        .at("/control")
                        .expected(["t", "date", "step"])
                        .build())
                    }
                };
                let target = target.min(self.replay.total_steps());
                if target < at {
                    self.rebuild()
                        .map_err(|e| E::new(ErrorCode::Invalid, e).build())?;
                    let _ = self.take(target);
                    reset = true;
                } else {
                    lines = self.take(target - at);
                }
                self.anchor = None;
            }
        }
        self.controls.push((at, control.clone()));
        let mut state = self.state();
        state.reset = reset;
        Ok((state, lines))
    }

    /// A new authority from the init (`epoch + 1`), the replay back at its start.
    fn rebuild(&mut self) -> Result<(), String> {
        self.resets += 1;
        let mut init = self.init.clone();
        init.epoch += self.resets;
        self.session = Session::new(init, &self.root)?;
        self.session
            .set_headline_calendar(crate::headparts::replay_calendar(self.info.kind));
        self.session.set_viewer_batch(true);
        for (p, r) in &self.principal_rules {
            self.session.set_principal_coin_flip_rule(p, *r);
        }
        if let Some(m) = &self.market {
            // No viewer yet on the new authority: nothing to tell.
            let _ = self.session.set_market_status(m.clone());
        }
        self.replay.reset();
        self.recorded.clear();
        Ok(())
    }

    /// Applies one replay step to the authority: the clock, then the events in order.
    fn apply(&mut self, st: &ReplayStep) -> Vec<OutboundLine> {
        let mut lines = match self.session.host_clock(st.t) {
            Ok(l) => l,
            Err(e) => {
                self.notices.push(format!("clock {}: {}", st.t, e.message));
                Vec::new()
            }
        };
        if self.record {
            for ev in &st.events {
                self.recorded.push(ev);
            }
        }
        for ev in &st.events {
            match self.session.host_data_event(ev) {
                Ok(o) => {
                    for n in &o.notices {
                        self.notices.push(n.frame().to_string());
                    }
                    lines.extend(o.lines);
                }
                Err(e) => self.notices.push(format!(
                    "event {}/{} seq {}: {e}",
                    ev.feed.symbol, ev.feed.interval, ev.seq
                )),
            }
        }
        lines
    }

    /// Takes up to `n` steps; returns the notifications they caused.
    fn take(&mut self, n: u64) -> Vec<OutboundLine> {
        let mut lines = Vec::new();
        for _ in 0..n {
            let Some(st) = self.replay.step() else { break };
            lines.extend(self.apply(&st));
        }
        // (RP3/M15, §22) Viewers once, after the steps (what they show now).
        if n > 0 {
            lines.extend(self.session.viewer_flush());
        }
        lines
    }

    /// Takes one step and returns it with the notifications it caused (`None` at the
    /// end): for a host that also feeds a replica from the same steps.
    pub fn take_step(&mut self) -> Option<(ReplayStep, Vec<OutboundLine>)> {
        let st = self.replay.step()?;
        let mut lines = self.apply(&st);
        lines.extend(self.session.viewer_flush());
        Some((st, lines))
    }

    /// The wall clock moved to `wall_ms` (any monotone millisecond clock): while running,
    /// takes the steps that are due at the current speed (at most
    /// [`MAX_STEPS_PER_PUMP`]) and returns their notifications. The first call after a
    /// start, resume, speed change, step or seek only sets the anchor.
    pub fn pump(&mut self, wall_ms: u64) -> Vec<OutboundLine> {
        if self.paused || self.replay.is_done() {
            self.anchor = None;
            return Vec::new();
        }
        let step = self.replay.cursor().step;
        let Some((w0, s0)) = self.anchor else {
            self.anchor = Some((wall_ms, step));
            return Vec::new();
        };
        let elapsed = wall_ms.saturating_sub(w0);
        // Steps due since the anchor; `as` after `floor` of a bounded positive value.
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            clippy::cast_precision_loss
        )]
        let due = s0 + ((elapsed as f64) * self.speed / 1000.0).floor() as u64;
        let n = due.saturating_sub(step).min(MAX_STEPS_PER_PUMP);
        if due.saturating_sub(step) > MAX_STEPS_PER_PUMP {
            // Fell behind: catch up from here instead of bursting.
            self.anchor = Some((wall_ms, step + n));
        }
        self.take(n)
    }

    /// Wall milliseconds from `wall_ms` until the next step is due (`None`: paused,
    /// done, or no anchor yet — call [`ReplayHost::pump`] now).
    #[must_use]
    pub fn next_due_in(&self, wall_ms: u64) -> Option<u64> {
        if self.paused || self.replay.is_done() {
            return None;
        }
        let (w0, s0) = self.anchor?;
        let next = self.replay.cursor().step + 1 - s0;
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            clippy::cast_precision_loss
        )]
        let at = w0 + ((next as f64) * 1000.0 / self.speed).ceil() as u64;
        Some(at.saturating_sub(wall_ms))
    }
}
