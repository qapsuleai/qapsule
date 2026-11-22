//! Streamable HTTP: the gate, the jobs, requests and responses ([`HttpGate`], [`handle`]; RG1: split out of `mcp_serve.rs`).

use super::*;

// ---------------------------------------------------------------------------
// Streamable HTTP
// ---------------------------------------------------------------------------

/// What an HTTP request must look like.
#[derive(Debug, Clone)]
pub(crate) struct HttpGate {
    /// The server's port (allowed `Origin`s carry it, or no port).
    pub port: u16,
    /// Allowed `Host` values; `None`: any (a token-protected non-local address, or a
    /// caller that checked `Host` itself).
    pub hosts: Option<Vec<String>>,
    /// Bearer token every request must carry.
    pub token: Option<String>,
    /// One bearer token per principal: a request must carry one of them and speaks as
    /// its principal (`X-Qp-Principal` is ignored). Non-empty: `token` is not used.
    pub principal_tokens: Vec<(String, String)>,
}

impl HttpGate {
    fn origin_ok(&self, origin: Option<&str>) -> bool {
        let Some(o) = origin else { return true };
        let o = o.to_ascii_lowercase();
        ["http://127.0.0.1", "http://localhost", "http://[::1]"]
            .iter()
            .any(|base| o == *base || o == format!("{base}:{}", self.port))
    }

    fn host_ok(&self, host: Option<&str>) -> bool {
        match &self.hosts {
            None => true,
            Some(hs) => host.is_some_and(|h| hs.iter().any(|x| x.eq_ignore_ascii_case(h))),
        }
    }

    /// The bearer check: `Err` when refused; `Ok(Some(p))` when the token names the
    /// principal `p`.
    fn authenticate(&self, auth: Option<&str>) -> Result<Option<String>, ()> {
        let given = auth.and_then(|a| a.strip_prefix("Bearer ")).map(str::trim);
        if !self.principal_tokens.is_empty() {
            let Some(given) = given else { return Err(()) };
            // Every token is compared (each in constant time), whichever matches.
            let mut found = None;
            for (p, t) in &self.principal_tokens {
                if crate::demo::same_secret(given, t) {
                    found = Some(p.clone());
                }
            }
            return found.map(Some).ok_or(());
        }
        match &self.token {
            None => Ok(None),
            Some(t) if given.is_some_and(|g| crate::demo::same_secret(g, t)) => Ok(None),
            Some(_) => Err(()),
        }
    }
}

/// An HTTP request handed to the hub's thread.
pub(crate) enum Job {
    /// `POST /mcp`.
    Post {
        /// Lower-case header names.
        headers: BTreeMap<String, String>,
        /// The principal its bearer token names (then `X-Qp-Principal` is ignored).
        principal: Option<String>,
        /// The JSON-RPC message.
        body: Value,
        /// Where the answer goes.
        reply: Sender<Answer>,
    },
    /// `GET /mcp` (a legacy session's notification stream).
    Get {
        /// `Mcp-Session-Id`.
        session: Option<String>,
        /// The principal the request asks for: its token's, else `X-Qp-Principal`.
        asked: Option<String>,
        /// Where the answer goes.
        reply: Sender<Answer>,
    },
    /// `DELETE /mcp` (ends a legacy session).
    Delete {
        /// `Mcp-Session-Id`.
        session: Option<String>,
        /// The principal the request asks for: its token's, else `X-Qp-Principal`.
        asked: Option<String>,
        /// Where the answer goes.
        reply: Sender<Answer>,
    },
}

/// The hub's answer to a [`Job`].
pub(crate) enum Answer {
    /// A complete response.
    Body {
        /// HTTP status code.
        status: u16,
        /// JSON body (empty: none).
        body: String,
        /// `Mcp-Session-Id` to return.
        session: Option<String>,
    },
    /// An SSE stream: first frames, then whatever arrives until the sender is dropped.
    Stream {
        /// Frames written at once (JSON-RPC messages).
        first: Vec<String>,
        /// Later frames.
        rx: Receiver<String>,
        /// `Mcp-Session-Id` to return.
        session: Option<String>,
    },
}

fn status_text(code: u16) -> &'static str {
    match code {
        200 => "200 OK",
        202 => "202 Accepted",
        204 => "204 No Content",
        400 => "400 Bad Request",
        401 => "401 Unauthorized",
        403 => "403 Forbidden",
        404 => "404 Not Found",
        405 => "405 Method Not Allowed",
        415 => "415 Unsupported Media Type",
        503 => "503 Service Unavailable",
        _ => "500 Internal Server Error",
    }
}

fn respond(s: &mut TcpStream, code: u16, body: &str, session: Option<&str>) {
    let mut head = format!(
        "HTTP/1.1 {}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n",
        status_text(code),
        body.len()
    );
    if !body.is_empty() {
        head.push_str("Content-Type: application/json\r\n");
    }
    if code == 401 {
        head.push_str("WWW-Authenticate: Bearer\r\n");
    }
    if code == 405 {
        head.push_str("Allow: POST, GET, DELETE\r\n");
    }
    if let Some(id) = session {
        head.push_str(&format!("Mcp-Session-Id: {id}\r\n"));
    }
    head.push_str("\r\n");
    let _ = s.write_all(head.as_bytes());
    let _ = s.write_all(body.as_bytes());
}

pub(super) fn rpc_error_body(code: i64, message: &str) -> String {
    json!({"jsonrpc": "2.0", "id": null, "error": {"code": code, "message": message}}).to_string()
}

/// Serves one `/mcp` request (the request line and headers are read; `body` is the
/// body): checks `Host`, `Origin` and the token, hands the request to the hub's thread
/// and writes its answer.
pub(crate) fn handle(
    s: &mut TcpStream,
    method: &str,
    headers: &BTreeMap<String, String>,
    body: &[u8],
    gate: &HttpGate,
    send: &dyn Fn(Job),
) {
    if !gate.host_ok(headers.get("host").map(String::as_str)) {
        respond(s, 403, &rpc_error_body(-32600, "unknown Host"), None);
        return;
    }
    if !gate.origin_ok(headers.get("origin").map(String::as_str)) {
        respond(s, 403, &rpc_error_body(-32600, "Origin not allowed"), None);
        return;
    }
    let Ok(principal) = gate.authenticate(headers.get("authorization").map(String::as_str)) else {
        respond(
            s,
            401,
            &rpc_error_body(-32001, "bearer token required"),
            None,
        );
        return;
    };
    let session = headers.get("mcp-session-id").cloned();
    let (tx, rx) = mpsc::channel();
    match method {
        "POST" => {
            if !headers
                .get("content-type")
                .is_some_and(|c| c.to_ascii_lowercase().starts_with("application/json"))
            {
                respond(
                    s,
                    415,
                    &rpc_error_body(-32600, "application/json only"),
                    None,
                );
                return;
            }
            let Ok(v) = serde_json::from_slice::<Value>(body) else {
                respond(s, 400, &rpc_error_body(-32700, "parse error"), None);
                return;
            };
            if let (Some(h), Some(b)) = (
                headers.get("mcp-protocol-version"),
                v.pointer("/params/_meta/io.modelcontextprotocol~1protocolVersion")
                    .and_then(Value::as_str),
            ) {
                if h != b {
                    respond(
                        s,
                        400,
                        &rpc_error_body(-32600, "HeaderMismatch: MCP-Protocol-Version"),
                        None,
                    );
                    return;
                }
            }
            send(Job::Post {
                headers: headers.clone(),
                principal,
                body: v,
                reply: tx,
            });
        }
        "GET" | "DELETE" => {
            // (Security review) The session's principal is checked like on POST: a
            // stream or an end of session only for the principal it is bound to.
            let asked = principal.or_else(|| headers.get("x-qp-principal").cloned());
            if method == "GET" {
                send(Job::Get {
                    session,
                    asked,
                    reply: tx,
                });
            } else {
                send(Job::Delete {
                    session,
                    asked,
                    reply: tx,
                });
            }
        }
        _ => {
            respond(s, 405, "", None);
            return;
        }
    }
    match rx.recv_timeout(Duration::from_secs(120)) {
        Ok(Answer::Body {
            status,
            body,
            session,
        }) => respond(s, status, &body, session.as_deref()),
        Ok(Answer::Stream { first, rx, session }) => stream(s, &first, &rx, session.as_deref()),
        Err(_) => respond(s, 503, &rpc_error_body(-32603, "busy"), None),
    }
}

fn stream(s: &mut TcpStream, first: &[String], rx: &Receiver<String>, session: Option<&str>) {
    let mut head = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\
Cache-Control: no-store\r\nX-Accel-Buffering: no\r\nConnection: close\r\n"
        .to_owned();
    if let Some(id) = session {
        head.push_str(&format!("Mcp-Session-Id: {id}\r\n"));
    }
    head.push_str("\r\n");
    if s.write_all(head.as_bytes()).is_err() {
        return;
    }
    let frame = |s: &mut TcpStream, m: &str| {
        s.write_all(format!("event: message\ndata: {m}\n\n").as_bytes())
            .and_then(|()| s.flush())
    };
    for m in first {
        if frame(s, m).is_err() {
            return;
        }
    }
    loop {
        match rx.recv_timeout(Duration::from_secs(15)) {
            Ok(m) => {
                if frame(s, &m).is_err() {
                    break;
                }
            }
            Err(RecvTimeoutError::Timeout) => {
                if s.write_all(b": keep-alive\n\n").is_err() {
                    break;
                }
            }
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
}

/// Reads one HTTP request (line, headers, body up to 8 MiB).
#[allow(clippy::type_complexity)]
fn read_request(s: &TcpStream) -> Option<(String, String, BTreeMap<String, String>, Vec<u8>)> {
    let mut r = BufReader::new(s.try_clone().ok()?);
    let mut line = String::new();
    r.read_line(&mut line).ok()?;
    let mut parts = line.split_whitespace();
    let (method, target) = (parts.next()?.to_owned(), parts.next()?.to_owned());
    let mut headers = BTreeMap::new();
    let mut len = 0usize;
    loop {
        let mut h = String::new();
        if r.read_line(&mut h).is_err() || h == "\r\n" || h.is_empty() {
            break;
        }
        if let Some((k, v)) = h.split_once(':') {
            let k = k.trim().to_ascii_lowercase();
            if k == "content-length" {
                len = v.trim().parse().unwrap_or(0);
            }
            headers.insert(k, v.trim().to_owned());
        }
    }
    let mut body = vec![0u8; len.min(8 << 20)];
    r.read_exact(&mut body).ok()?;
    Some((method, target, headers, body))
}

/// A connection of the HTTP server: only `/mcp`.
fn connection(mut s: TcpStream, gate: &HttpGate, tx: &Sender<Job>) {
    let Some((method, target, headers, body)) = read_request(&s) else {
        return;
    };
    let path = target.split_once('?').map_or(target.as_str(), |(p, _)| p);
    if path != "/mcp" {
        respond(&mut s, 404, "", None);
        return;
    }
    handle(&mut s, &method, &headers, &body, gate, &|j| {
        let _ = tx.send(j);
    });
}

pub(super) fn serve_http(
    hub: &SharedHub,
    cfg: HostCfg,
    a: &ServeArgs,
    token: Option<String>,
    principal_tokens: Vec<(String, String)>,
    mut dump: Option<Dump>,
) -> Result<(), String> {
    let listener = TcpListener::bind((a.listen.as_str(), a.port))
        .map_err(|e| format!("bind {}:{}: {e}", a.listen, a.port))?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let shown = if a.listen.contains(':') {
        format!("[{}]", a.listen)
    } else {
        a.listen.clone()
    };
    let hosts = if is_local(&a.listen) {
        let mut h: Vec<String> = ["localhost", "127.0.0.1", "[::1]"]
            .iter()
            .map(|h| format!("{h}:{port}"))
            .collect();
        h.push(format!("{shown}:{port}"));
        Some(h)
    } else {
        None
    };
    let gate = std::sync::Arc::new(HttpGate {
        port,
        hosts,
        token,
        principal_tokens,
    });
    eprintln!(
        "{}",
        json!({"listening": format!("http://{shown}:{port}/mcp")})
    );
    let (tx, rx) = mpsc::channel::<Job>();
    std::thread::spawn(move || {
        for s in listener.incoming().flatten() {
            let (gate, tx) = (gate.clone(), tx.clone());
            std::thread::spawn(move || connection(s, &gate, &tx));
        }
    });
    let mut host = McpHost::new(cfg);
    let start = Instant::now();
    let wall = || u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
    loop {
        let wait = hub.borrow().next_due_in(wall()).unwrap_or(1000).min(1000);
        match rx.recv_timeout(Duration::from_millis(wait)) {
            Ok(job) => host.job(hub, job),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return Ok(()),
        }
        hub.borrow_mut().pump(wall());
        if let Some(dp) = dump.as_mut() {
            dp.update(hub)?;
        }
        host.flush();
    }
}

/// One MCP session over HTTP: an endpoint per principal and its streams. A legacy
/// session (`Mcp-Session-Id`) has one principal, bound at `initialize`; the stateless
/// session (`""`) has an endpoint per principal a request was allowed to speak as.
#[derive(Default)]
pub(super) struct HttpSession {
    pub(super) servers: BTreeMap<String, McpServer>,
    /// The principal a legacy session was initialized as.
    pub(super) bound: Option<String>,
    /// Legacy revision (the session said `initialize`).
    pub(super) legacy: Option<String>,
    /// `GET /mcp` streams (legacy `notifications/message`).
    pub(super) sse: Vec<Sender<String>>,
    /// `subscriptions/listen` streams, by principal.
    pub(super) listens: BTreeMap<String, Sender<String>>,
}
