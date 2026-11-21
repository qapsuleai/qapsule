//! (R5B/T2) `qp mcp serve`: the Qapsule MCP server over a running authority session
//! (`rounds/R5B/PLAN.md` §5, `docs/PROTOCOL.md` §3, §21).
//!
//! - Session: the synthetic replay (`qp_session::demo_init`, default) or `--init`; paused
//!   unless `--running` (then it moves with the wall clock at `--speed`); `--seek` moves
//!   it to a day first. `--source sim|live` is prepared, not served yet (R6 `livehost`).
//! - Identity: `--principal` (default `claude`) must be in the init with an `agent`
//!   author; the synthetic init adds `claude` (for `arkin`) and `claude-guest` (for
//!   `guest`). The server speaks only as `--principal`: an `X-Qp-Principal` naming
//!   another is `403`. Several principals (measurements) need `--principals a,b` and a
//!   loopback `--listen`; on another address, one token per principal (`--token-env
//!   ID=VAR,…`) and the identity is the token's (the header is ignored). A legacy session
//!   is bound at `initialize` to its principal; asking for another in it is `403`.
//! - Transports: stdio (default; newline-delimited JSON-RPC) and Streamable HTTP
//!   (`--http`): `POST /mcp` (one JSON-RPC message; `application/json` answer, or an
//!   SSE stream for `subscriptions/listen` and for a request with a `logLevel`), `GET
//!   /mcp` (a legacy session's `notifications/message` stream), `DELETE /mcp` (ends a
//!   legacy session). `Mcp-Session-Id` is issued at `initialize` (legacy revisions);
//!   modern (2026-07-28) requests are stateless (one endpoint per principal).
//! - Local only by default: bound to `127.0.0.1`; `Host` must name this server; an
//!   `Origin`, when sent, must be `http://127.0.0.1[:port]` or `http://localhost[:port]`
//!   (DNS rebinding), else `403`. Another `--listen` address needs `--token-env VAR`:
//!   every request then carries `Authorization: Bearer <$VAR>` (constant-time check), else
//!   `401`. The token is never printed or logged.
//! - One thread owns the hub (authority + every MCP endpoint); HTTP connections are
//!   threads that hand their request over a channel ([`Job`]) and wait for the answer.
//!
//! The same HTTP endpoint is served by `qp demo --mcp` (`crate::demo`).

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

use qp_mcp::{calllog, surfaces, Config, McpServer, NotifyMode, Options, SharedHub};
use qp_schema::protocol::{Principal, SessionInit};
use serde_json::{json, Value};

/// Agent principals the synthetic init adds: (id, the person it works for).
pub const AGENTS: [(&str, &str); 2] = [("claude", "arkin"), ("claude-guest", "guest")];

/// `qp mcp serve` arguments.
#[derive(Debug, clap::Args)]
#[allow(clippy::struct_excessive_bools)]
pub struct ServeArgs {
    /// Serve over stdin/stdout (the default).
    #[arg(long, conflicts_with = "http")]
    pub stdio: bool,
    /// Serve Streamable HTTP at `/mcp`.
    #[arg(long)]
    pub http: bool,
    /// HTTP port (0: any free port; the address is written to stderr as
    /// `{"listening":"http://127.0.0.1:<port>/mcp"}`).
    #[arg(long, default_value_t = 0)]
    pub port: u16,
    /// HTTP address to listen on. Anything but this machine needs `--token-env`.
    #[arg(long, default_value = "127.0.0.1")]
    pub listen: String,
    /// Bearer token every HTTP request must carry (`Authorization: Bearer …`), read from
    /// the environment. `VAR`: the token of `--principal`. `ID=VAR,ID=VAR`: one token per
    /// principal; a request speaks as the principal of its token (`X-Qp-Principal` is
    /// ignored). Required with a non-local `--listen`.
    #[arg(long, value_name = "VAR|ID=VAR,…")]
    pub token_env: Option<String>,
    /// Session source: `synthetic` (default), `local` (real bars under
    /// `fixtures/local/`), `sim` or `live` (not yet: R6).
    #[arg(long, default_value = "synthetic")]
    pub source: String,
    /// A ready session init (`SessionInit` JSON) instead of the synthetic replay.
    #[arg(long, conflicts_with_all = ["instruments", "days", "seed", "history"])]
    pub init: Option<PathBuf>,
    /// Directory that paths in the init are relative to (and confined to).
    #[arg(long, default_value = ".")]
    pub root: PathBuf,
    /// Synthetic replay: instruments (default 30).
    #[arg(long)]
    pub instruments: Option<u32>,
    /// Synthetic replay: trading days (default 260).
    #[arg(long)]
    pub days: Option<u32>,
    /// Synthetic replay: seed (default 1).
    #[arg(long)]
    pub seed: Option<u64>,
    /// Synthetic replay: bars of history before the first replayed bar (default 120).
    #[arg(long)]
    pub history: Option<u32>,
    /// Replay speed, steps per wall-clock second.
    #[arg(long)]
    pub speed: Option<f64>,
    /// Run the replay with the wall clock (default: paused; agents move it with the
    /// `replay_*` tools).
    #[arg(long)]
    pub running: bool,
    /// Move the replay to this day before serving (`YYYY-MM-DD`).
    #[arg(long, value_name = "YYYY-MM-DD")]
    pub seek: Option<String>,
    /// Principal the server speaks as (an `agent` principal of the init).
    #[arg(long, default_value = "claude")]
    pub principal: String,
    /// (Measurements) Further agent principals an HTTP request may pick with
    /// `X-Qp-Principal`. Loopback `--listen` only; elsewhere use `--token-env ID=VAR,…`.
    #[arg(long, value_delimiter = ',', value_name = "ID,ID")]
    pub principals: Vec<String>,
    /// Tool surface (`thin`; see `crates/qp-mcp/src/surfaces/README.md`).
    #[arg(long, default_value = "thin")]
    pub surface: String,
    /// (R5B/M29, measurement) Default capsule format for tools that take `format`
    /// (`card`, `json`, `layered`) when a call gives none.
    #[arg(long, value_parser = ["card", "json", "layered"])]
    pub capsule_format: Option<String>,
    /// Device surface kind sent in the `hello`: desktop, phone, watch, voice, headless.
    #[arg(long, default_value = "headless")]
    pub device: String,
    /// Measurement log: one JSON line per tool call (sizes and time only).
    #[arg(long)]
    pub log: Option<PathBuf>,
    /// Measurement run label of the log lines (stdio; HTTP takes `X-Qp-Run`).
    #[arg(long)]
    pub run: Option<String>,
    /// (Measurements only) Writes the authority's document — the owner's view, not
    /// projected to any reader — to this file after every request that wrote to the log
    /// (replaced atomically). Never for a served product.
    #[arg(long)]
    pub dump: Option<PathBuf>,
}

/// `--dump`: the authority's document after a write (log length changed).
struct Dump {
    path: PathBuf,
    seen: Option<usize>,
}

impl Dump {
    fn new(path: Option<&PathBuf>) -> Option<Self> {
        path.map(|p| Self {
            path: p.clone(),
            seen: None,
        })
    }

    fn update(&mut self, hub: &SharedHub) -> Result<(), String> {
        let h = hub.borrow();
        let s = h.session();
        let n = s.log().len();
        if self.seen == Some(n) {
            return Ok(());
        }
        let mut text = serde_json::to_string_pretty(s.document()).map_err(|e| e.to_string())?;
        text.push('\n');
        let tmp = self.path.with_extension("tmp");
        std::fs::write(&tmp, text).map_err(|e| format!("--dump {}: {e}", tmp.display()))?;
        std::fs::rename(&tmp, &self.path)
            .map_err(|e| format!("--dump {}: {e}", self.path.display()))?;
        self.seen = Some(n);
        Ok(())
    }
}

/// Adds the agent principals [`AGENTS`] (agent author, role `proposer`) when missing;
/// with explicit replay entitlements, each gets those of the person it works for.
pub fn add_agent_principals(init: &mut SessionInit) {
    for (id, behalf) in AGENTS {
        if init.principals.iter().any(|p| p.id == id) {
            continue;
        }
        // (R5B, PROTOCOL §25.1) The agent reads as the person it works for, when that
        // person is a human principal of the init who works for nobody.
        let person_ok = init.principals.iter().any(|q| {
            q.id == behalf
                && q.on_behalf.is_none()
                && matches!(q.author, qp_schema::Author::Human { .. })
        });
        let mut v = json!({
            "id": id, "author": {"kind": "agent", "name": id}, "role": "proposer"
        });
        if person_ok {
            v["on_behalf"] = json!(behalf);
        }
        let p: Principal = serde_json::from_value(v).expect("agent principal");
        init.principals.push(p);
        if let Some(r) = init.replay.as_mut() {
            if let Some(e) = r.entitlements.get(behalf).cloned() {
                r.entitlements.insert(id.to_owned(), e);
            }
        }
    }
}

/// The init's principals whose author is an agent.
#[must_use]
pub fn agent_principals(init: &SessionInit) -> Vec<String> {
    init.principals
        .iter()
        .filter(|p| matches!(p.author, qp_schema::Author::Agent { .. }))
        .map(|p| p.id.clone())
        .collect()
}

/// `principal` is an agent principal of the init.
///
/// # Errors
/// Unknown principal, or not an agent.
pub fn check_principal(init: &SessionInit, principal: &str) -> Result<(), String> {
    let Some(p) = init.principals.iter().find(|p| p.id == principal) else {
        return Err(format!(
            "--principal `{principal}` is not a principal of the session (agents: {})",
            agent_principals(init).join(", ")
        ));
    };
    if matches!(p.author, qp_schema::Author::Agent { .. }) {
        Ok(())
    } else {
        Err(format!(
            "--principal `{principal}` is not an agent (its author is not `agent`); \
             qp mcp serve speaks only as an agent principal (agents: {})",
            agent_principals(init).join(", ")
        ))
    }
}

/// The session init `a` names.
fn session_init(a: &ServeArgs) -> Result<SessionInit, String> {
    use qp_schema::protocol::{ReplayInit, ReplaySourceKind};
    let source = match a.source.as_str() {
        "synthetic" => ReplaySourceKind::Synthetic,
        "local" => ReplaySourceKind::Local,
        "sim" | "live" => {
            // R6: `livehost` (the live and simulated hosts of `qp demo --live/--sim`)
            // is connected here.
            return Err(format!(
                "--source {}: unsupported yet (R6 livehost); use synthetic or local",
                a.source
            ));
        }
        s => {
            return Err(format!(
                "--source `{s}`: expected synthetic, local, sim or live"
            ))
        }
    };
    let mut init: SessionInit = match &a.init {
        Some(p) => qp_session::read_json(p)?,
        None => {
            let mut init = qp_session::demo_init(ReplayInit {
                source,
                instruments: a.instruments,
                days: a.days,
                seed: a.seed,
                history: a.history,
                ..ReplayInit::default()
            });
            add_agent_principals(&mut init);
            init
        }
    };
    if let Some(r) = init.replay.as_mut() {
        if let Some(s) = a.speed {
            r.speed = Some(s);
        }
        if a.running {
            r.running = true;
        }
    } else if a.running || a.speed.is_some() || a.seek.is_some() {
        return Err("--running, --speed and --seek need a replay (`init.replay`)".to_owned());
    }
    Ok(init)
}

/// Builds the hub (authority) of `init`; with `seek`, moves the replay there.
///
/// # Errors
/// The session cannot start or the seek is invalid.
pub fn build_hub(
    init: SessionInit,
    root: &std::path::Path,
    seek: Option<&str>,
) -> Result<SharedHub, String> {
    let hub = match &init.replay {
        Some(r) => {
            let dir = if r.source == qp_schema::protocol::ReplaySourceKind::Local {
                qp_source::local_bars_dir()
            } else {
                PathBuf::new()
            };
            qp_mcp::Hub::shared_replay(qp_session::ReplayHost::with_local_dir(init, root, &dir)?)
        }
        None => qp_mcp::Hub::shared(qp_session::Session::new(init, root)?),
    };
    if let Some(d) = seek {
        let c: qp_schema::protocol::ReplayControl =
            serde_json::from_value(json!({"kind": "seek", "date": d}))
                .map_err(|e| format!("--seek: {e}"))?;
        hub.borrow_mut()
            .host_control(&c)
            .map_err(|e| format!("--seek {d}: {e}"))?;
    }
    Ok(hub)
}

fn device(kind: &str) -> Result<qp_schema::protocol::Surface, String> {
    serde_json::from_value(json!({ "kind": kind })).map_err(|e| format!("--device `{kind}`: {e}"))
}

/// `qp mcp serve`.
///
/// # Errors
/// Usage errors (a non-agent principal, a non-local address without a token, an
/// unknown surface), the session's errors, I/O errors.
pub fn serve(a: &ServeArgs) -> Result<(), String> {
    if surfaces::by_name(&a.surface).is_none() {
        return Err(format!(
            "--surface `{}`: expected one of {}",
            a.surface,
            surfaces::names().join(", ")
        ));
    }
    let dev = device(&a.device)?;
    let tokens = match &a.token_env {
        Some(spec) => Some(read_tokens(spec)?),
        None => None,
    };
    if a.http && !is_local(&a.listen) && tokens.is_none() {
        return Err(format!(
            "--listen {}: a non-local address needs --token-env VAR (bearer token); \
             the default 127.0.0.1 serves this machine only",
            a.listen
        ));
    }
    if !a.principals.is_empty() {
        if !a.http {
            return Err("--principals: HTTP only (--http); stdio speaks as --principal".into());
        }
        if !is_local(&a.listen) {
            return Err(format!(
                "--principals: several principals are served only on a loopback --listen \
                 (not {}); on another address give each its own token: \
                 --token-env ID=VAR,ID=VAR",
                a.listen
            ));
        }
        if matches!(tokens, Some(Tokens::PerPrincipal(_))) {
            return Err(
                "--principals with --token-env ID=VAR: the token names the principal; \
                 give only one of them"
                    .into(),
            );
        }
    }
    let init = session_init(a)?;
    check_principal(&init, &a.principal)?;
    let mut allowed = vec![a.principal.clone()];
    for p in &a.principals {
        check_principal(&init, p).map_err(|e| e.replacen("--principal ", "--principals ", 1))?;
        if !allowed.contains(p) {
            allowed.push(p.clone());
        }
    }
    let (token, principal_tokens) = match tokens {
        None => (None, Vec::new()),
        Some(Tokens::Single(t)) => (Some(t), Vec::new()),
        Some(Tokens::PerPrincipal(ts)) => {
            for (p, _) in &ts {
                check_principal(&init, p)
                    .map_err(|e| e.replacen("--principal ", "--token-env ", 1))?;
            }
            (None, ts)
        }
    };
    let log = match &a.log {
        Some(p) => Some(calllog::CallLog::append(p)?.shared()),
        None => None,
    };
    let hub = build_hub(init, &a.root, a.seek.as_deref())?;
    let cfg = HostCfg {
        default_principal: a.principal.clone(),
        allowed,
        surface: a.surface.clone(),
        capsule_format: a.capsule_format.clone(),
        device: dev,
        log,
    };
    let mut dump = Dump::new(a.dump.as_ref());
    if let Some(d) = dump.as_mut() {
        d.update(&hub)?;
    }
    if a.http {
        serve_http(&hub, cfg, a, token, principal_tokens, dump)
    } else {
        serve_stdio(&hub, &cfg, a.run.clone(), dump)
    }
}

/// `--token-env`: `VAR` (the token of `--principal`) or `ID=VAR,ID=VAR` (one per
/// principal).
enum Tokens {
    Single(String),
    PerPrincipal(Vec<(String, String)>),
}

fn read_token(var: &str) -> Result<String, String> {
    let t = std::env::var(var).map_err(|_| format!("--token-env: `{var}` is not set"))?;
    if t.trim().len() < 16 {
        return Err(format!(
            "--token-env: `{var}` is shorter than 16 characters"
        ));
    }
    Ok(t.trim().to_owned())
}

fn read_tokens(spec: &str) -> Result<Tokens, String> {
    if !spec.contains('=') {
        return read_token(spec.trim()).map(Tokens::Single);
    }
    let mut out: Vec<(String, String)> = Vec::new();
    for part in spec.split(',').map(str::trim).filter(|p| !p.is_empty()) {
        let Some((id, var)) = part.split_once('=') else {
            return Err(format!(
                "--token-env `{part}`: expected ID=VAR (every entry, or a single VAR)"
            ));
        };
        let (id, var) = (id.trim(), var.trim());
        if id.is_empty() || var.is_empty() {
            return Err(format!("--token-env `{part}`: expected ID=VAR"));
        }
        if out.iter().any(|(p, _)| p == id) {
            return Err(format!("--token-env: `{id}` is named twice"));
        }
        let t = read_token(var)?;
        if out.iter().any(|(_, x)| crate::demo::same_secret(x, &t)) {
            return Err(format!(
                "--token-env: `{var}` holds the same token as another principal's"
            ));
        }
        out.push((id.to_owned(), t));
    }
    if out.is_empty() {
        return Err("--token-env: no ID=VAR entry".into());
    }
    Ok(Tokens::PerPrincipal(out))
}

/// A loopback address (`localhost`, `127.0.0.0/8`, `::1`).
fn is_local(addr: &str) -> bool {
    addr.eq_ignore_ascii_case("localhost")
        || addr
            .trim_start_matches('[')
            .trim_end_matches(']')
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback())
}

/// What an endpoint is opened with.
pub struct HostCfg {
    /// Principal when a request names none.
    pub default_principal: String,
    /// Principals a request may name with `X-Qp-Principal`: `default_principal`, plus
    /// `--principals` (loopback only). A token-derived principal is not checked here.
    pub allowed: Vec<String>,
    /// Tool surface name.
    pub surface: String,
    /// Device surface of the `hello`.
    pub device: qp_schema::protocol::Surface,
    /// Measurement log.
    pub log: Option<calllog::SharedLog>,
    /// (M29) Default capsule format (`--capsule-format`).
    pub capsule_format: Option<String>,
}

impl HostCfg {
    fn open(&self, hub: &SharedHub, conn: &str, principal: &str) -> Result<McpServer, String> {
        let surface = surfaces::by_name(&self.surface)
            .ok_or_else(|| format!("unknown surface `{}`", self.surface))?;
        McpServer::open(
            hub.clone(),
            conn,
            Config::new(principal, self.device.clone()),
            surface,
            Options {
                notify: NotifyMode::Spec,
                common_tools: true,
                log: self.log.clone(),
                capsule_format: self.capsule_format.clone(),
            },
        )
        .map_err(|e| format!("hello as `{principal}` refused: {e}"))
    }
}

fn write_line(out: &mut impl Write, v: &Value) -> Result<(), String> {
    writeln!(out, "{v}").map_err(|e| format!("stdout: {e}"))
}

/// stdio: one endpoint; the replay moves with the wall clock between lines.
fn serve_stdio(
    hub: &SharedHub,
    cfg: &HostCfg,
    run: Option<String>,
    mut dump: Option<Dump>,
) -> Result<(), String> {
    let mut server = cfg.open(hub, "mcp-1", &cfg.default_principal)?;
    server.set_run(run);
    let (tx, rx) = mpsc::channel::<Result<String, String>>();
    std::thread::spawn(move || {
        for line in std::io::stdin().lock().lines() {
            if tx.send(line.map_err(|e| format!("stdin: {e}"))).is_err() {
                break;
            }
        }
    });
    let start = Instant::now();
    let wall = || u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    loop {
        let wait = hub.borrow().next_due_in(wall());
        let got = match wait {
            Some(ms) => match rx.recv_timeout(Duration::from_millis(ms)) {
                Ok(l) => Some(l),
                Err(RecvTimeoutError::Timeout) => None,
                Err(RecvTimeoutError::Disconnected) => break,
            },
            None => match rx.recv() {
                Ok(l) => Some(l),
                Err(_) => break,
            },
        };
        let mut response = Vec::new();
        if let Some(line) = got {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            response = match serde_json::from_str::<Value>(&line) {
                Ok(v) => server.handle_message(&v),
                Err(_) => server
                    .handle_line(&line)
                    .iter()
                    .filter_map(|s| serde_json::from_str(s).ok())
                    .collect(),
            };
        }
        hub.borrow_mut().pump(wall());
        if let Some(dp) = dump.as_mut() {
            dp.update(hub)?;
        }
        let d = server.deliver(true);
        for m in d.messages.iter().chain(&response).chain(&d.updated) {
            write_line(&mut out, m)?;
        }
        out.flush().map_err(|e| format!("stdout: {e}"))?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Streamable HTTP
// ---------------------------------------------------------------------------

/// What an HTTP request must look like.
#[derive(Debug, Clone)]
pub struct HttpGate {
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
pub enum Job {
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
pub enum Answer {
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

fn rpc_error_body(code: i64, message: &str) -> String {
    json!({"jsonrpc": "2.0", "id": null, "error": {"code": code, "message": message}}).to_string()
}

/// Serves one `/mcp` request (the request line and headers are read; `body` is the
/// body): checks `Host`, `Origin` and the token, hands the request to the hub's thread
/// and writes its answer.
pub fn handle(
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

fn serve_http(
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
struct HttpSession {
    servers: BTreeMap<String, McpServer>,
    /// The principal a legacy session was initialized as.
    bound: Option<String>,
    /// Legacy revision (the session said `initialize`).
    legacy: Option<String>,
    /// `GET /mcp` streams (legacy `notifications/message`).
    sse: Vec<Sender<String>>,
    /// `subscriptions/listen` streams, by principal.
    listens: BTreeMap<String, Sender<String>>,
}

/// The hub thread's side of the HTTP endpoint: sessions, endpoints, streams.
pub struct McpHost {
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
