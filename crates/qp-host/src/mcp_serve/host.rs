//! The MCP host shared by the HTTP sessions ([`McpHost`]; RG1: split out of `mcp_serve.rs`).

use super::*;

/// The hub thread's side of the HTTP endpoint: sessions, endpoints, streams.
pub(crate) struct McpHost {
    cfg: HostCfg,
    /// By `Mcp-Session-Id`; `""` is the stateless (modern) session.
    sessions: BTreeMap<String, HttpSession>,
    next_conn: u64,
}

impl McpHost {
    /// A host with no session yet.
    #[must_use]
    pub fn new(cfg: HostCfg) -> Self {
        Self {
            cfg,
            sessions: BTreeMap::new(),
            next_conn: 0,
        }
    }

    /// Protocol connections of the MCP endpoints (`mcp-<n>`).
    #[must_use]
    pub fn is_mcp_conn(conn: &str) -> bool {
        conn.starts_with("mcp-")
    }

    /// Answers one request.
    pub fn job(&mut self, hub: &SharedHub, job: Job) {
        match job {
            Job::Post {
                headers,
                principal,
                body,
                reply,
            } => {
                let a = self.post(hub, &headers, principal, &body);
                let _ = reply.send(a);
            }
            Job::Get {
                session,
                asked,
                reply,
            } => {
                let session = session.filter(|id| !id.is_empty());
                if let Some(a) = session
                    .as_deref()
                    .and_then(|id| self.wrong_principal(id, asked.as_deref()))
                {
                    let _ = reply.send(a);
                    return;
                }
                let a = match session.and_then(|id| self.sessions.get_mut(&id).map(|s| (id, s))) {
                    Some((id, s)) => {
                        let (stx, srx) = mpsc::channel();
                        s.sse.push(stx);
                        Answer::Stream {
                            first: Vec::new(),
                            rx: srx,
                            session: Some(id),
                        }
                    }
                    None => Answer::Body {
                        status: 405,
                        body: String::new(),
                        session: None,
                    },
                };
                let _ = reply.send(a);
                self.flush();
            }
            Job::Delete {
                session,
                asked,
                reply,
            } => {
                if let Some(a) = session
                    .as_deref()
                    .filter(|id| !id.is_empty())
                    .and_then(|id| self.wrong_principal(id, asked.as_deref()))
                {
                    let _ = reply.send(a);
                    return;
                }
                let status = match session {
                    Some(id) if !id.is_empty() && self.sessions.remove(&id).is_some() => 204,
                    Some(_) => 404,
                    None => 405,
                };
                let _ = reply.send(Answer::Body {
                    status,
                    body: String::new(),
                    session: None,
                });
            }
        }
    }

    /// `403` when session `sid` is bound to another principal than the one asked for
    /// (no principal asked: the bound one is meant).
    fn wrong_principal(&self, sid: &str, asked: Option<&str>) -> Option<Answer> {
        let bound = self.sessions.get(sid)?.bound.as_deref()?;
        match asked {
            Some(p) if p != bound => Some(Answer::Body {
                status: 403,
                body: rpc_error_body(
                    -32001,
                    &format!("this Mcp-Session-Id speaks as `{bound}`, not `{p}`"),
                ),
                session: None,
            }),
            _ => None,
        }
    }

    #[allow(clippy::expect_used, clippy::too_many_lines)]
    fn post(
        &mut self,
        hub: &SharedHub,
        headers: &BTreeMap<String, String>,
        token_principal: Option<String>,
        body: &Value,
    ) -> Answer {
        let err = |status: u16, code: i64, msg: &str| Answer::Body {
            status,
            body: json!({"jsonrpc": "2.0", "id": body.get("id").cloned().unwrap_or(Value::Null),
                "error": {"code": code, "message": msg}})
            .to_string(),
            session: None,
        };
        let method = body.get("method").and_then(Value::as_str).unwrap_or("");
        let sid = match headers.get("mcp-session-id") {
            Some(id) if self.sessions.contains_key(id) => id.clone(),
            Some(_) => return err(404, -32001, "unknown Mcp-Session-Id"),
            None if method == "initialize" => match crate::demo::secret() {
                Ok(id) => id,
                Err(e) => return err(500, -32603, &e),
            },
            None => String::new(),
        };
        // The principal: the token's (the header is ignored), else `X-Qp-Principal`
        // (an allowed one only), else the one the session is bound to, else the default.
        let token_derived = token_principal.is_some();
        let asked = token_principal.or_else(|| headers.get("x-qp-principal").cloned());
        let bound = self.sessions.get(&sid).and_then(|s| s.bound.clone());
        let principal = match (asked, bound) {
            (Some(p), Some(b)) if p != b => {
                return err(
                    403,
                    -32001,
                    &format!(
                        "this Mcp-Session-Id speaks as `{b}` (bound at initialize); \
                         `{p}` needs its own session"
                    ),
                );
            }
            (Some(p), _) => p,
            (None, Some(b)) => b,
            (None, None) => self.cfg.default_principal.clone(),
        };
        if !token_derived && !self.cfg.allowed.contains(&principal) {
            return err(
                403,
                -32001,
                &format!(
                    "X-Qp-Principal `{principal}` is not served here (served: {}); \
                     several agent principals need --principals on a loopback address",
                    self.cfg.allowed.join(", ")
                ),
            );
        }
        let session = self.sessions.entry(sid.clone()).or_default();
        if !sid.is_empty() && session.bound.is_none() {
            session.bound = Some(principal.clone());
        }
        if !session.servers.contains_key(&principal) {
            self.next_conn += 1;
            let conn = format!("mcp-{}", self.next_conn);
            match self.cfg.open(hub, &conn, &principal) {
                Ok(mut s) => {
                    s.set_legacy(session.legacy.clone());
                    session.servers.insert(principal.clone(), s);
                }
                Err(e) => return err(500, -32603, &e),
            }
        }
        let server = session.servers.get_mut(&principal).expect("just opened");
        server.set_run(headers.get("x-qp-run").cloned());
        let out = server.handle_message(body);
        if method == "initialize" {
            session.legacy = server.legacy().map(str::to_owned);
        }
        let issued = (!sid.is_empty()).then(|| sid.clone());
        if method == "subscriptions/listen" && server.listening() {
            let (stx, srx) = mpsc::channel();
            session.listens.insert(principal, stx);
            let a = Answer::Stream {
                first: out.iter().map(Value::to_string).collect(),
                rx: srx,
                session: issued,
            };
            self.flush();
            return a;
        }
        let wants_sse = headers
            .get("accept")
            .is_some_and(|a| a.contains("text/event-stream"));
        match out.len() {
            0 => Answer::Body {
                status: 202,
                body: String::new(),
                session: issued,
            },
            1 => Answer::Body {
                status: 200,
                body: out[0].to_string(),
                session: issued,
            },
            _ if wants_sse => {
                let (_tx, rx) = mpsc::channel();
                Answer::Stream {
                    first: out.iter().map(Value::to_string).collect(),
                    rx,
                    session: issued,
                }
            }
            _ => Answer::Body {
                status: 200,
                body: out.last().map(Value::to_string).unwrap_or_default(),
                session: issued,
            },
        }
    }

    /// Sends what the endpoints have for their streams (legacy `notifications/message`
    /// on `GET` streams; `notifications/resources/updated` on listen streams). Streams
    /// that closed are dropped.
    pub fn flush(&mut self) {
        for s in self.sessions.values_mut() {
            for (principal, server) in &mut s.servers {
                let d = server.deliver(!s.sse.is_empty());
                for m in &d.messages {
                    let text = m.to_string();
                    s.sse.retain(|tx| tx.send(text.clone()).is_ok());
                }
                if let Some(u) = d.updated {
                    let alive = s
                        .listens
                        .get(principal)
                        .is_some_and(|tx| tx.send(u.to_string()).is_ok());
                    if !alive {
                        s.listens.remove(principal);
                        server.stop_listening();
                    }
                }
            }
        }
    }
}
