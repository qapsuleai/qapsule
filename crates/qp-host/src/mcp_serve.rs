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

mod host;
mod http;

pub(crate) use host::*;
pub(crate) use http::*;

/// Agent principals the synthetic init adds: (id, the person it works for).
pub(crate) const AGENTS: [(&str, &str); 2] = [("claude", "arkin"), ("claude-guest", "guest")];

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

#[allow(clippy::expect_used)]
/// Adds the agent principals [`AGENTS`] (agent author, role `proposer`) when missing;
/// with explicit replay entitlements, each gets those of the person it works for.
pub(crate) fn add_agent_principals(init: &mut SessionInit) {
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
pub(crate) fn agent_principals(init: &SessionInit) -> Vec<String> {
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
pub(crate) fn check_principal(init: &SessionInit, principal: &str) -> Result<(), String> {
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
pub(crate) fn build_hub(
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

#[allow(clippy::too_many_lines)]
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
pub(crate) struct HostCfg {
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
