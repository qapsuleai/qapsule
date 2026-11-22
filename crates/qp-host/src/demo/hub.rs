//! The demo hub's commands ([`Hub`]; RG1: split out of `demo.rs`).

use super::*;

impl Hub {
    pub(super) fn wall(&self) -> u64 {
        u64::try_from(self.start.elapsed().as_millis()).unwrap_or(u64::MAX)
    }

    fn start_frame(
        &self,
        conn: &str,
        token: &str,
        principal: &str,
        lang: &str,
        viewer: bool,
    ) -> String {
        let mut data: InitData = host_of(&self.mcp).init().data.clone().unwrap_or_default();
        data.entitlements.retain(|p, _| p == principal);
        if viewer {
            // (RP3/M15, §22) No bars and no events: the feed comes from the authority's
            // viewer subscription; the open instrument's data with a `bind` frame.
            let instruments: Vec<&str> =
                data.feeds.iter().map(|f| f.feed.symbol.as_str()).collect();
            return json!({"start": {
                "conn": conn, "token": token, "principal": principal, "lang": lang,
                "mode": "viewer", "data": {"feeds": [], "entitlements": data.entitlements},
                "events": [], "instruments": instruments, "info": self.info,
            }})
            .to_string();
        }
        json!({"start": {
            "conn": conn, "token": token, "principal": principal, "lang": lang,
            "data": data, "events": host_of(&self.mcp).recorded_events(),
            "info": self.info,
        }})
        .to_string()
    }

    pub(super) fn route(&mut self, lines: Vec<qp_schema::protocol::OutboundLine>) {
        let mut mcp = Vec::new();
        for l in lines {
            if let Some(c) = self.clients.get(&l.conn) {
                let _ = c.tx.send(json!({"msg": l.msg}).to_string());
            } else if self.mcp_host.is_some() && crate::mcp_serve::McpHost::is_mcp_conn(&l.conn) {
                mcp.push(l);
            }
        }
        // (R5B) Notifications for the MCP endpoints wait in the adapter's hub.
        if !mcp.is_empty() {
            self.mcp.borrow_mut().queue(mcp);
        }
    }

    /// (R5B) Messages the MCP endpoints' commands caused for the web shell's
    /// connections (queued in the adapter's hub) go to their streams.
    fn route_queued(&mut self) {
        for (id, c) in &self.clients {
            for q in self.mcp.borrow_mut().take(id) {
                let _ = c.tx.send(json!({"msg": q.msg}).to_string());
            }
        }
    }

    /// Sends new data events (or, after a backward seek, a new start) to every client;
    /// (RP3/M15) a viewer gets only the events of the feeds it is served, then a `bind`
    /// frame for each feed its viewer subscription newly names.
    pub(super) fn broadcast(&mut self) {
        let st = host_of(&self.mcp).state();
        if st.epoch != self.epoch {
            self.epoch = st.epoch;
            self.sent_events = host_of(&self.mcp).recorded_len();
            let frames: Vec<(Sender<String>, String)> = self
                .clients
                .iter()
                .map(|(id, c)| {
                    (
                        c.tx.clone(),
                        self.start_frame(id, &c.token, &c.principal, &c.lang, c.viewer),
                    )
                })
                .collect();
            for c in self.clients.values_mut() {
                c.served.clear();
            }
            for (tx, f) in frames {
                let _ = tx.send(f);
            }
            return;
        }
        let total = host_of(&self.mcp).recorded_len();
        if total > self.sent_events {
            let new = &host_of(&self.mcp).recorded_since(self.sent_events);
            let frame = json!({"data": new, "replay": st}).to_string();
            for c in self.clients.values() {
                if c.viewer {
                    let mine: Vec<&qp_schema::data::DataEvent> =
                        new.iter().filter(|e| c.served.contains(&e.feed)).collect();
                    let _ = c.tx.send(json!({"data": mine, "replay": st}).to_string());
                } else {
                    let _ = c.tx.send(frame.clone());
                }
            }
            self.sent_events = total;
        }
        self.bind();
    }

    /// (RP3/M15, §22) A `bind` frame for every feed a viewer's subscription newly names
    /// (its initial bars and the events since); a feed it no longer names is dropped.
    fn bind(&mut self) {
        let ids: Vec<String> = self.clients.keys().cloned().collect();
        for id in ids {
            let Some(c) = self.clients.get(&id).filter(|c| c.viewer) else {
                continue;
            };
            let want: Vec<FeedKey> = host_of(&self.mcp)
                .session()
                .viewer_data(&id)
                .map(<[FeedKey]>::to_vec)
                .unwrap_or_default();
            if want == c.served {
                continue;
            }
            let mut feeds = Vec::new();
            let mut events = Vec::new();
            for k in want.iter().filter(|k| !c.served.contains(k)) {
                if let Some((f, e)) = host_of(&self.mcp).feed_data(k) {
                    feeds.push(f);
                    events.extend(e);
                }
            }
            if !feeds.is_empty() {
                let _ =
                    c.tx.send(json!({"bind": {"feeds": feeds, "events": events}}).to_string());
            }
            if let Some(c) = self.clients.get_mut(&id) {
                c.served = want;
            }
        }
    }

    pub(super) fn handle(&mut self, cmd: Cmd) {
        match cmd {
            Cmd::Connect {
                principal,
                lang,
                viewer,
                feed_rule,
                no_exempt,
                tx,
            } => {
                let Ok(token) = secret() else {
                    return;
                };
                // (R5/M19) The page's feed rule for its principal (else the host's);
                // (R5/S) `feed_exempt=0`: that rule (or the host's) without the K42
                // exemption.
                let name = feed_rule.or_else(|| no_exempt.then(|| self.feed_rule.clone()));
                match name
                    .as_deref()
                    .and_then(|n| qp_capsule::feedrule::FeedRule::named(n, !no_exempt))
                {
                    Some(r) => host_of(&self.mcp).set_principal_coin_flip_rule(&principal, Some(r)),
                    None => host_of(&self.mcp).clear_principal_coin_flip_rule(&principal),
                }
                self.next += 1;
                let conn = format!("c{}", self.next);
                let _ = tx.send(self.start_frame(&conn, &token, &principal, &lang, viewer));
                self.clients.insert(
                    conn,
                    Client {
                        principal,
                        lang,
                        token,
                        tx,
                        viewer,
                        served: Vec::new(),
                    },
                );
            }
            Cmd::Send {
                conn,
                token,
                body,
                reply,
            } => {
                let ok = self
                    .clients
                    .get(&conn)
                    .is_some_and(|c| same_secret(&c.token, &token));
                let _ = reply.send(ok);
                if !ok {
                    return;
                }
                let msgs: Vec<Value> = match serde_json::from_str::<Value>(&body) {
                    Ok(Value::Array(a)) => a,
                    Ok(v) => vec![v],
                    Err(_) => vec![Value::String(body)],
                };
                for m in msgs {
                    let text = match m {
                        Value::String(s) => s,
                        v => v.to_string(),
                    };
                    let lines = host_of(&self.mcp).handle_raw(&conn, &text);
                    // (RP3/M15) Data and `bind` frames first: a viewer's open capsule
                    // finds its instrument's data when its slice arrives.
                    self.broadcast();
                    self.route(lines);
                }
            }
            Cmd::Gone { conn } => {
                self.clients.remove(&conn);
            }
            Cmd::Mcp(job) => {
                if let Some(m) = self.mcp_host.as_mut() {
                    m.job(&self.mcp, job);
                }
                // A replay control or a command of the agent: data frames, a new start
                // after a backward seek, the shell connections' notifications.
                self.broadcast();
                self.route_queued();
            }
            Cmd::Live(input) => {
                if let Some(l) = self.live.as_mut() {
                    let lines = l.on_in(&mut host_of(&self.mcp), input);
                    self.pending.extend(lines);
                }
            }
            Cmd::LiveStats { reply } => {
                let v = self
                    .live
                    .as_ref()
                    .map_or(Value::Null, |l| l.stats(&host_of(&self.mcp)));
                let _ = reply.send(v.to_string());
            }
            Cmd::LiveControl {
                drop_secs,
                endpoint,
                reply,
            } => {
                if self.conn_ctl.is_empty() {
                    let _ = reply.send(r#"{"error":"not live"}"#.to_owned());
                    return;
                }
                match drop_secs {
                    Some(secs) => {
                        let name = endpoint.as_deref().unwrap_or("market");
                        let Some((_, ctl)) = self.conn_ctl.iter().find(|(n, _)| *n == name) else {
                            let _ = reply.send(json!({"error": "no such endpoint"}).to_string());
                            return;
                        };
                        let _ = ctl.send(crate::livehost::ConnCtl::Drop(Duration::from_secs(
                            secs.min(600),
                        )));
                        if let Some(l) = self.live.as_mut() {
                            l.drops_asked += 1;
                        }
                        let _ = reply
                            .send(json!({"drop": secs.min(600), "endpoint": name}).to_string());
                    }
                    None => {
                        self.quit = true;
                        let _ = reply.send(r#"{"quit":true}"#.to_owned());
                    }
                }
            }
        }
    }

    /// (R6/T7) Closes the live connections (DISCONNECT) and prints the summary.
    pub(super) fn shutdown_live(&mut self) {
        for (_, ctl) in std::mem::take(&mut self.conn_ctl) {
            let (tx, rx) = mpsc::channel();
            let _ = ctl.send(crate::livehost::ConnCtl::Stop(tx));
            let _ = rx.recv_timeout(Duration::from_secs(10));
        }
        if let Some(l) = &self.live {
            let st = l.stats(&host_of(&self.mcp));
            println!("canlı özet / live summary: {st}");
        }
    }
}
