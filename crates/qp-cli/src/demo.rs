//! `qp demo` (R4/T5): the local demo host — a [`ReplayHost`] (authority + replay), a
//! static file server for the web shell and the protocol connection, all on one origin
//! (`docs/ARCHITECTURE.md` "Kabuk sınırı (R4)"). No dependency beyond `std`.
//!
//! Transport (same origin, HTTP/1.1):
//! - `GET /events?principal=…&lang=…` — a server-sent event stream; one stream is one
//!   protocol connection. Frames (one JSON object per `data:` line):
//!   `{"start":{conn, principal, lang, data, events, info}}` first (the replica's data:
//!   the initial bars and the data events applied since, and the page's notes),
//!   `{"msg":ServerMsg}` (protocol messages of this connection),
//!   `{"data":[DataEvent…],"replay":ReplayState}` (the data plane, §18.1 — not a protocol
//!   message: a separate frame) and `{"start":…}` again after a backward seek (a new
//!   authority, epoch + 1: the page starts over and says `hello` again).
//! - `POST /send?conn=…` — a JSON array of protocol messages of that connection, in order
//!   (answers come on the stream). `204`.
//! - `GET /<file>` — the shell's static files (`--web`, default `shells/web`), `/` →
//!   `index.html`. Nothing else is served; the page loads nothing from elsewhere (CSP).

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream, UdpSocket};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

use qp_schema::protocol::InitData;
use qp_session::ReplayHost;
use qp_wasm::shell::{demo_session_init, DemoOptions};
use serde_json::{json, Value};

/// `qp demo` arguments.
#[derive(Debug, clap::Args)]
pub struct DemoArgs {
    /// Port.
    #[arg(long, default_value_t = 8390)]
    pub port: u16,
    /// Address to listen on (`0.0.0.0`: phones on the same network can open it).
    #[arg(long, default_value = "0.0.0.0")]
    pub bind: String,
    /// The web shell's directory (index.html, main.js, pkg/).
    #[arg(long, default_value = "shells/web")]
    pub web: PathBuf,
    /// Local bars directory (default: `fixtures/local/bars` or `QAPSULE_LOCAL_BARS`); the
    /// synthetic market is used, and the page says so, when there is no data.
    #[arg(long, conflicts_with = "synthetic")]
    pub data: Option<PathBuf>,
    /// Use the synthetic market even if local data exists.
    #[arg(long)]
    pub synthetic: bool,
    /// Steps per second.
    #[arg(long, default_value_t = 4.0)]
    pub speed: f64,
    /// Start date of the replay (`YYYY-MM-DD`, exchange time zone).
    #[arg(long)]
    pub from: Option<String>,
    /// Instruments.
    #[arg(long, default_value_t = 30)]
    pub instruments: u32,
    /// Bars of history before the replay starts.
    #[arg(long, default_value_t = 120)]
    pub history: u32,
    /// Synthetic trading days.
    #[arg(long, default_value_t = 260)]
    pub days: u32,
    /// Start paused.
    #[arg(long)]
    pub paused: bool,
    /// Repository root (relative paths of the session init).
    #[arg(long, default_value = ".")]
    pub root: PathBuf,
}

/// `qp drive` arguments: the shell-less driver.
#[derive(Debug, clap::Args)]
pub struct DriveArgs {
    /// Script (JSON array of steps; see `qp_wasm::drive`).
    #[arg(long)]
    pub script: PathBuf,
    /// Principal.
    #[arg(long, default_value = "arkin")]
    pub principal: String,
    /// Language (`tr`, `en`).
    #[arg(long, default_value = "tr")]
    pub lang: String,
    /// Output file (default: standard output).
    #[arg(long)]
    pub out: Option<PathBuf>,
}

/// Builds the demo's replay host.
fn host(a: &DemoArgs) -> Result<ReplayHost, String> {
    let from_t = match &a.from {
        Some(d) => Some(qp_source::date_start(d).map_err(|e| format!("--from: {e}"))?),
        None => None,
    };
    let o = DemoOptions {
        local: !a.synthetic,
        instruments: a.instruments,
        history: a.history,
        days: a.days,
        speed: a.speed,
        from_t,
        running: !a.paused,
    };
    let init = demo_session_init(&o);
    let dir = if a.synthetic {
        PathBuf::new()
    } else {
        a.data.clone().unwrap_or_else(qp_source::local_bars_dir)
    };
    let mut h = ReplayHost::with_local_dir(init, &a.root, &dir)?;
    h.set_record_events(true);
    Ok(h)
}

/// The machine's address on the local network (a routing lookup; no packet is sent).
fn local_ip() -> Option<String> {
    let s = UdpSocket::bind("0.0.0.0:0").ok()?;
    s.connect("10.254.254.254:1").ok()?;
    let ip = s.local_addr().ok()?.ip();
    (!ip.is_loopback() && !ip.is_unspecified()).then(|| ip.to_string())
}

enum Cmd {
    Connect {
        principal: String,
        lang: String,
        tx: Sender<String>,
    },
    Send {
        conn: String,
        body: String,
    },
    Gone {
        conn: String,
    },
}

struct Client {
    principal: String,
    lang: String,
    tx: Sender<String>,
}

struct Hub {
    host: ReplayHost,
    clients: BTreeMap<String, Client>,
    next: u64,
    sent_events: usize,
    epoch: u64,
    info: Value,
    start: Instant,
}

impl Hub {
    fn wall(&self) -> u64 {
        u64::try_from(self.start.elapsed().as_millis()).unwrap_or(u64::MAX)
    }

    fn start_frame(&self, conn: &str, principal: &str, lang: &str) -> String {
        let mut data: InitData = self.host.init().data.clone().unwrap_or_default();
        data.entitlements.retain(|p, _| p == principal);
        json!({"start": {
            "conn": conn, "principal": principal, "lang": lang,
            "data": data, "events": self.host.recorded_events(),
            "info": self.info,
        }})
        .to_string()
    }

    fn route(&mut self, lines: Vec<qp_schema::protocol::OutboundLine>) {
        for l in lines {
            if let Some(c) = self.clients.get(&l.conn) {
                let _ = c.tx.send(json!({"msg": l.msg}).to_string());
            }
        }
    }

    /// Sends new data events (or, after a backward seek, a new start) to every client.
    fn broadcast(&mut self) {
        let st = self.host.state();
        if st.epoch != self.epoch {
            self.epoch = st.epoch;
            self.sent_events = self.host.recorded_events().len();
            let frames: Vec<(Sender<String>, String)> = self
                .clients
                .iter()
                .map(|(id, c)| (c.tx.clone(), self.start_frame(id, &c.principal, &c.lang)))
                .collect();
            for (tx, f) in frames {
                let _ = tx.send(f);
            }
            return;
        }
        let evs = self.host.recorded_events();
        if evs.len() > self.sent_events {
            let frame = json!({"data": &evs[self.sent_events..], "replay": st}).to_string();
            self.sent_events = evs.len();
            for c in self.clients.values() {
                let _ = c.tx.send(frame.clone());
            }
        }
    }

    fn handle(&mut self, cmd: Cmd) {
        match cmd {
            Cmd::Connect {
                principal,
                lang,
                tx,
            } => {
                self.next += 1;
                let conn = format!("c{}", self.next);
                let _ = tx.send(self.start_frame(&conn, &principal, &lang));
                self.clients.insert(
                    conn,
                    Client {
                        principal,
                        lang,
                        tx,
                    },
                );
            }
            Cmd::Send { conn, body } => {
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
                    let lines = self.host.handle_raw(&conn, &text);
                    self.route(lines);
                    self.broadcast();
                }
            }
            Cmd::Gone { conn } => {
                self.clients.remove(&conn);
            }
        }
    }
}

fn content_type(p: &Path) -> &'static str {
    match p.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "wasm" => "application/wasm",
        "json" => "application/json",
        "css" => "text/css; charset=utf-8",
        "png" => "image/png",
        "svg" => "image/svg+xml",
        "ttf" => "font/ttf",
        "md" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

const CSP: &str = "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; \
connect-src 'self'; img-src 'self' data: blob:; style-src 'self' 'unsafe-inline'; \
font-src 'self'; base-uri 'none'; form-action 'none'";

fn respond(s: &mut TcpStream, code: &str, ctype: &str, body: &[u8]) {
    let head = format!(
        "HTTP/1.1 {code}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\n\
Cache-Control: no-store\r\nContent-Security-Policy: {CSP}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = s.write_all(head.as_bytes());
    let _ = s.write_all(body);
}

fn query_param(q: &str, k: &str) -> Option<String> {
    q.split('&').find_map(|kv| {
        let (a, b) = kv.split_once('=')?;
        (a == k).then(|| b.to_owned())
    })
}

/// A safe file under `web` for a request path (letters, digits, `._-/`, no `..`).
fn static_path(web: &Path, path: &str) -> Option<PathBuf> {
    let rel = path.trim_start_matches('/');
    let rel = if rel.is_empty() { "index.html" } else { rel };
    if rel
        .split('/')
        .any(|seg| seg.is_empty() || seg.starts_with('.'))
        || !rel
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "._-/".contains(c))
    {
        return None;
    }
    Some(web.join(rel))
}

fn connection(mut s: TcpStream, web: &Path, tx: &Sender<Cmd>) {
    let Ok(peer) = s.try_clone() else { return };
    let mut r = BufReader::new(peer);
    let mut line = String::new();
    if r.read_line(&mut line).is_err() {
        return;
    }
    let mut parts = line.split_whitespace();
    let (method, target) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
    let mut len = 0usize;
    loop {
        let mut h = String::new();
        if r.read_line(&mut h).is_err() || h == "\r\n" || h.is_empty() {
            break;
        }
        if let Some((k, v)) = h.split_once(':') {
            if k.eq_ignore_ascii_case("content-length") {
                len = v.trim().parse().unwrap_or(0);
            }
        }
    }
    let (path, q) = target.split_once('?').unwrap_or((target, ""));
    match (method, path) {
        ("GET", "/events") => {
            let principal = query_param(q, "principal")
                .filter(|p| p.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'))
                .unwrap_or_else(|| "arkin".to_owned());
            let lang = if query_param(q, "lang").as_deref() == Some("en") {
                "en"
            } else {
                "tr"
            };
            let (ftx, frx) = mpsc::channel::<String>();
            let _ = tx.send(Cmd::Connect {
                principal,
                lang: lang.to_owned(),
                tx: ftx,
            });
            let head = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\
Cache-Control: no-store\r\nConnection: keep-alive\r\n\r\n";
            if s.write_all(head.as_bytes()).is_err() {
                return;
            }
            stream_events(&mut s, &frx, tx);
        }
        ("POST", "/send") => {
            let mut body = vec![0u8; len.min(8 << 20)];
            if r.read_exact(&mut body).is_err() {
                respond(&mut s, "400 Bad Request", "text/plain", b"short body");
                return;
            }
            let Some(conn) = query_param(q, "conn") else {
                respond(&mut s, "400 Bad Request", "text/plain", b"no conn");
                return;
            };
            let _ = tx.send(Cmd::Send {
                conn,
                body: String::from_utf8_lossy(&body).into_owned(),
            });
            respond(&mut s, "204 No Content", "text/plain", b"");
        }
        ("GET", p) => {
            match static_path(web, p).and_then(|f| std::fs::read(&f).ok().map(|b| (f, b))) {
                Some((f, b)) => respond(&mut s, "200 OK", content_type(&f), &b),
                None => respond(&mut s, "404 Not Found", "text/plain", b"not found"),
            }
        }
        _ => respond(&mut s, "405 Method Not Allowed", "text/plain", b""),
    }
}

fn stream_events(s: &mut TcpStream, frx: &Receiver<String>, tx: &Sender<Cmd>) {
    let mut conn = String::new();
    loop {
        match frx.recv_timeout(Duration::from_secs(15)) {
            Ok(f) => {
                if conn.is_empty() {
                    if let Some(c) = serde_json::from_str::<Value>(&f)
                        .ok()
                        .and_then(|v| v["start"]["conn"].as_str().map(str::to_owned))
                    {
                        conn = c;
                    }
                }
                if s.write_all(format!("data: {f}\n\n").as_bytes()).is_err() {
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
    if !conn.is_empty() {
        let _ = tx.send(Cmd::Gone { conn });
    }
}

/// Runs the demo host until interrupted.
///
/// # Errors
/// The host cannot start or the port cannot be bound.
pub fn demo(a: &DemoArgs) -> Result<(), String> {
    let host = host(a)?;
    let st = host.state();
    let listener = TcpListener::bind((a.bind.as_str(), a.port))
        .map_err(|e| format!("bind {}:{}: {e}", a.bind, a.port))?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let mut urls = vec![format!("http://localhost:{port}/")];
    let phone = (a.bind == "0.0.0.0")
        .then(local_ip)
        .flatten()
        .map(|ip| format!("http://{ip}:{port}/"));
    if let Some(u) = &phone {
        urls.push(u.clone());
    }
    let info = json!({"urls": urls, "phone": phone, "synthetic": st.synthetic, "note": st.note});
    println!("qp demo: {}", urls[0]);
    match &phone {
        Some(u) => println!("telefondan aç / open on your phone (same network): {u}"),
        None => println!("telefondan açmak için --bind 0.0.0.0 ile başlatın / start with --bind 0.0.0.0 to open on a phone"),
    }
    if st.synthetic {
        println!(
            "veri: sentetik pazar / data: synthetic market{}",
            st.note
                .as_deref()
                .map(|n| format!(" ({n})"))
                .unwrap_or_default()
        );
    } else {
        println!(
            "veri: yerel geçmiş / data: local history ({} instruments)",
            st.instruments
        );
    }
    let _ = std::io::stdout().flush();
    let (tx, rx) = mpsc::channel::<Cmd>();
    let web = a.web.clone();
    std::thread::spawn(move || {
        for s in listener.incoming().flatten() {
            let (web, tx) = (web.clone(), tx.clone());
            std::thread::spawn(move || connection(s, &web, &tx));
        }
    });
    let mut hub = Hub {
        epoch: host.state().epoch,
        sent_events: host.recorded_events().len(),
        host,
        clients: BTreeMap::new(),
        next: 0,
        info,
        start: Instant::now(),
    };
    loop {
        let w = hub.wall();
        let wait = hub.host.next_due_in(w).unwrap_or(40).clamp(1, 40);
        match rx.recv_timeout(Duration::from_millis(wait)) {
            Ok(cmd) => hub.handle(cmd),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return Ok(()),
        }
        let w = hub.wall();
        let lines = hub.host.pump(w);
        hub.route(lines);
        hub.broadcast();
    }
}

/// `qp drive`: the shell-less driver over the demo's synthetic, paused host.
///
/// # Errors
/// Unreadable script or output.
pub fn drive(a: &DriveArgs) -> Result<(), String> {
    let text =
        std::fs::read_to_string(&a.script).map_err(|e| format!("{}: {e}", a.script.display()))?;
    let script: Vec<Value> = serde_json::from_str(&text).map_err(|e| format!("script: {e}"))?;
    let mut host = qp_wasm::drive::parity_host();
    let out = qp_wasm::drive::shell_less(&mut host, &a.principal, &a.lang, &script);
    let s = serde_json::to_string_pretty(&out).map_err(|e| e.to_string())?;
    match &a.out {
        Some(p) => std::fs::write(p, s + "\n").map_err(|e| format!("{}: {e}", p.display())),
        None => {
            println!("{s}");
            Ok(())
        }
    }
}
