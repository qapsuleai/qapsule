//! `qp demo` (R4/T5): the local demo host — a [`ReplayHost`] (authority + replay), a
//! static file server for the web shell and the protocol connection, all on one origin
//! (`docs/ARCHITECTURE.md` "Kabuk sınırı (R4)"). No dependency beyond `std`.
//!
//! Transport (same origin, HTTP/1.1):
//! - `GET /events?principal=…&lang=…&mode=…&feed_rule=…` — a server-sent event stream; one stream is
//!   one protocol connection. (RP3/M15, `docs/PROTOCOL.md` §22) `mode=viewer` (the
//!   default) makes it a viewer: its `start` frame carries no bars (`mode`, the universe's
//!   `instruments`), its `data` frames only the feeds its `viewer` subscription names (none
//!   on the feed screen) and a `{"bind":{feeds, events}}` frame brings a feed's initial
//!   bars and events when it subscribes to one; `mode=replica` is the full replica of R4
//!   (every bar, every event). (R5/M19) `feed_rule=` (`rate55`, `none`) sets
//!   the coin-flip rule for the page's principal over the host's `--feed-rule`;
//!   (R5/S) `feed_exempt=0` drops the rule's K42 exemption for that principal (the human
//!   test's second form; with no `feed_rule=`, the host's rule without it). Frames (one JSON object per `data:` line):
//!   `{"start":{conn, principal, lang, data, events, info}}` first (the replica's data:
//!   the initial bars and the data events applied since, and the page's notes),
//!   `{"msg":ServerMsg}` (protocol messages of this connection),
//!   `{"data":[DataEvent…],"replay":ReplayState}` (the data plane, §18.1 — not a protocol
//!   message: a separate frame) and `{"start":…}` again after a backward seek (a new
//!   authority, epoch + 1: the page starts over and says `hello` again).
//! - `POST /send?conn=…` — a JSON array of protocol messages of that connection, in order
//!   (answers come on the stream). `204`. It needs the connection's secret (header
//!   `X-Qp-Token`, given only in that connection's `start` frame), `Content-Type:
//!   application/json` and a same-origin `Origin`; otherwise `403`/`415`.
//! - Every request's `Host` must be this host (`localhost`, `127.0.0.1`, `[::1]`, the
//!   address listened on or the printed LAN address, with the port), else `403` (DNS
//!   rebinding). Default address `127.0.0.1`; `--lan` listens on `0.0.0.0` for phones.
//!
//! The per-connection secret is 128 bits from the operating system (`/dev/urandom`):
//! host code, not compute code — it never reaches the authority, the document or any
//! digest, so replay determinism is unaffected.
//! - (R5/G, `--gallery`) `GET /gallery/layout?lang=…&theme=…` — the comparison gallery's
//!   layout (JSON, `qp_wasm::gallery::Layout`) and `GET /gallery/img/<name>.png` — the
//!   images it names, painted by the core over the synthetic world at a fixed moment
//!   (`qp_wasm::gallery::build`; built once per language and theme, kept in memory). The
//!   page is the static `gallery.html`.
//! - (R5B, `--mcp`) `POST`/`GET`/`DELETE /mcp` — the same session for agents over MCP
//!   Streamable HTTP (`crate::mcp_serve`: this machine's origins only, agent principals
//!   only, default `claude`). The authority lives in the MCP adapter's hub
//!   (`qp_mcp::Hub`), shared by the shell's connections and the MCP endpoints.
//! - `GET /<file>` — the shell's static files (`--web`, default `shells/web`), `/` →
//!   `index.html`. Nothing else is served; the page loads nothing from elsewhere (CSP).
//!   Pre-compressed `<file>.br`/`<file>.gz` are served by `Accept-Encoding`, with an
//!   `ETag` and revalidation (RP3/M13, `demo_static.rs`).

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream, UdpSocket};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use qp_schema::data::FeedKey;
use qp_schema::protocol::InitData;
use qp_session::ReplayHost;
use qp_shellcore::shell::{demo_session_init, DemoOptions};
use serde_json::{json, Value};

mod http;
mod hub;
mod setup;

use http::*;
use setup::*;

/// `qp demo` arguments.
#[derive(Debug, clap::Args)]
pub struct DemoArgs {
    /// Port.
    #[arg(long, default_value_t = 8390)]
    pub port: u16,
    /// Address to listen on. Default `127.0.0.1` (this machine only); see `--lan`.
    #[arg(long, default_value = "127.0.0.1")]
    pub bind: String,
    /// Listen on every interface (`0.0.0.0`) so that a phone on the same network can open
    /// the page. Devices on the local network can then see the demo.
    #[arg(long)]
    pub lan: bool,
    /// (RP3) The name of an HTTPS reverse proxy in front of this host (e.g. a private
    /// tunnel that terminates TLS and forwards to 127.0.0.1:PORT keeping `Host`): that
    /// name (with or without `:443`) is accepted as `Host`, and `https://NAME` as the
    /// `Origin` of `/send`. Off by default. A phone needs a secure context (`https://`)
    /// for the JIT on iOS 27 and for WebGPU (rounds/RP3/CALIBRATION.md); whoever can
    /// reach the proxy can see the demo.
    #[arg(long, value_name = "NAME")]
    pub https_host: Option<String>,
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
    /// (R5/M19, K53) The host's feed rule (`rate55`, `none`; M19's pick by
    /// default). A page may ask for another with `?feed_rule=` (for its principal).
    #[arg(long, default_value = "rate55")]
    pub feed_rule: String,
    /// (RP4, `docs/CAPSULE.md` §9.9) The capsule engine's named constant record (`r3`,
    /// the protocol default; `rp4`), in every mode (replay, `--live`, `--sim`).
    #[arg(long, default_value = "r3", value_parser = crate::sim::check_rule_arg)]
    pub rule: String,
    /// Steps per second.
    #[arg(long, default_value_t = 4.0)]
    pub speed: f64,
    /// Start date of the replay (`YYYY-MM-DD`, exchange time zone).
    #[arg(long)]
    pub from: Option<String>,
    /// Instruments.
    #[arg(long, default_value_t = 30)]
    pub instruments: u32,
    /// Bars of history before the replay starts (with `--from` too: the bars just
    /// before that date).
    #[arg(long, default_value_t = 120)]
    pub history: u32,
    /// Synthetic trading days.
    #[arg(long, default_value_t = 260)]
    pub days: u32,
    /// Start paused.
    #[arg(long)]
    pub paused: bool,
    /// (R5/G) Also serve the comparison gallery (`/gallery.html`): every topic's
    /// candidates one under the other, painted by the core over the synthetic, paused
    /// world (independent of this demo's data), in both languages and themes.
    #[arg(long)]
    pub gallery: bool,
    /// Repository root (relative paths of the session init).
    #[arg(long, default_value = ".")]
    pub root: PathBuf,
    /// (R5B) Also serve the session to agents at `/mcp` (Streamable HTTP, the same rules
    /// as `qp mcp serve --http`: this machine's origins only), as principal `claude`
    /// (only; another `X-Qp-Principal` is refused). The person in the web shell and the
    /// agent read and write the same document.
    #[arg(long, conflicts_with_all = ["live", "sim"])]
    pub mcp: bool,
    /// (R6/T7) Live data: the universe's history from REST (`--history` bars of
    /// `--interval`), then one live connection (environment variables,
    /// `docs/DATA_ACCESS.md`). This machine only: refused with `--lan`, `--https-host`
    /// or another `--bind` (real data does not go to the local network).
    #[arg(long, conflicts_with_all = ["synthetic", "data", "from", "gallery"])]
    pub live: bool,
    /// (R6/T7) The live universe: comma-separated symbols or `@file` (one per line).
    #[arg(long, default_value = "@fixtures/local/live/u30.txt")]
    pub symbols: String,
    /// (R6/T7) The live bars' interval (`1m`, `5m`, `15m`, `30m`, `1h`, `1d`).
    #[arg(long, default_value = "5m")]
    pub interval: String,
    /// (R6/T7) Read history and repair bars from this directory
    /// (`<SYMBOL>.<period>.unadj.json`) instead of REST: offline tests, a cached history.
    #[arg(long, requires = "live", value_name = "DIR")]
    pub live_bars: Option<PathBuf>,
    /// (RP4, L-prep, K66) With `--live`: bars from the trade endpoint (`/trade`, a second
    /// connection with its own lock; `mx/trade/<SYM>`, level 1) while the market endpoint
    /// gives the session phases, the day fields and the clock. Trade gaps (the opening,
    /// every trade reconnect) are filled from the REST trade history. `POST
    /// /live/drop?endpoint=trade` drops the trade connection.
    #[arg(long, requires = "live")]
    pub trades: bool,
    /// (RP4, L-prep) With `--trades`: also subscribe special trades (`mx/tradeex/<SYM>`;
    /// they do not join the bars).
    #[arg(long, requires = "trades")]
    pub tradeex: bool,
    /// (RP4, L-prep) Unit of the live trade `timestamp`: `ns_from_midnight` (documented,
    /// unverified), `epoch_ms`, `epoch_us`, `epoch_ns` or `auto` (`qp live inspect` shows
    /// which one the stream uses).
    #[arg(long, requires = "trades", default_value = "ns_from_midnight")]
    pub trade_time_unit: String,
    /// (RP4, L-prep) Trade history from cached files (`<DIR>/<SYM>/*.csv.gz`, the `qp live
    /// trades` cache) instead of REST: offline tests.
    #[arg(long, requires = "trades", value_name = "DIR")]
    pub live_trades: Option<PathBuf>,
    /// (RP4/T4) Accelerated simulation (`qp sim`) shown live: cached trades of
    /// `--start`..`--end` for `--symbols` through the live path on a simulated exchange
    /// clock at `--sim-speed`. No connection, no token. This machine only: refused with
    /// `--lan`, `--https-host` or another `--bind` (real data does not go to the local
    /// network).
    #[arg(long, conflicts_with_all = ["live", "synthetic", "data", "from", "gallery"])]
    pub sim: bool,
    /// (RP4/T4) First simulated day (`YYYY-MM-DD`).
    #[arg(long, requires = "sim")]
    pub start: Option<String>,
    /// (RP4/T4) Last simulated day (default: `--start`).
    #[arg(long, requires = "sim")]
    pub end: Option<String>,
    /// (RP4/T6) Simulated start time on the first day (`HH:MM`; default: midnight). That
    /// day's earlier trades are history.
    #[arg(long, requires = "sim", value_name = "HH:MM")]
    pub sim_at: Option<String>,
    /// (RP4/T4) Simulation pace: `N` (N× real time in sessions) or `max`.
    #[arg(long, default_value = "60")]
    pub sim_speed: String,
    /// (RP4/T4) Cached trades (`<dir>/<SYM>/<start>_<end>.csv.gz`).
    #[arg(long, default_value = "fixtures/local/trades")]
    pub sim_trades: PathBuf,
    /// (RP4/T4) Cached REST bars (history).
    #[arg(long, default_value = "fixtures/local/live/rest")]
    pub sim_rest: PathBuf,
}

/// `qp drive` arguments: the shell-less driver.
#[derive(Debug, clap::Args)]
pub struct DriveArgs {
    /// (RP3/M15) `replica` (the R4 full replica, default) or `viewer` (§22).
    #[arg(long, default_value = "replica")]
    pub mode: String,
    /// (RP3/M15) Instruments of the synthetic universe.
    #[arg(long, default_value_t = 30)]
    pub instruments: u32,
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

enum Cmd {
    Connect {
        principal: String,
        lang: String,
        viewer: bool,
        /// (R5/M19) The page's `?feed_rule=` (a known rule name), if any.
        feed_rule: Option<String>,
        /// (R5/S) The page's `?feed_exempt=0`: the rule without the K42 exemption.
        no_exempt: bool,
        tx: Sender<String>,
    },
    Send {
        conn: String,
        token: String,
        body: String,
        reply: Sender<bool>,
    },
    Gone {
        conn: String,
    },
    /// (R6/T7) The live side's input (connection thread, repair thread).
    Live(crate::livehost::LiveIn),
    /// (R5B) A request to the MCP endpoint (`--mcp`).
    Mcp(crate::mcp_serve::Job),
    /// (R6/T7) `GET /live/stats`.
    LiveStats {
        reply: Sender<String>,
    },
    /// (R6/T7) `POST /live/drop` (`secs`) and `POST /live/quit` (`None`).
    LiveControl {
        drop_secs: Option<u64>,
        /// (RP4, L-prep) Which connection `drop` closes (`market` by default, `trade`).
        endpoint: Option<String>,
        reply: Sender<String>,
    },
}

struct Client {
    principal: String,
    lang: String,
    token: String,
    tx: Sender<String>,
    /// (RP3/M15) A viewer connection (§22) and the feeds its data plane carries now.
    viewer: bool,
    served: Vec<FeedKey>,
}

/// 128 bits from the operating system's randomness, as 32 hex digits.
pub(crate) fn secret() -> Result<String, String> {
    let mut b = [0u8; 16];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut b))
        .map_err(|e| format!("no OS randomness for the connection secret: {e}"))?;
    Ok(b.iter().map(|x| format!("{x:02x}")).collect())
}

/// Constant-time comparison of two secrets.
pub(crate) fn same_secret(a: &str, b: &str) -> bool {
    a.len() == b.len()
        && a.bytes()
            .zip(b.bytes())
            .fold(0u8, |acc, (x, y)| acc | (x ^ y))
            == 0
}

struct Hub {
    /// The authority (a replay host) in the MCP adapter's hub: (R5B, `--mcp`) the MCP
    /// endpoints share it with the web shell's connections.
    mcp: qp_mcp::SharedHub,
    /// (R5B) The MCP endpoint (`--mcp`).
    mcp_host: Option<crate::mcp_serve::McpHost>,
    clients: BTreeMap<String, Client>,
    next: u64,
    sent_events: usize,
    epoch: u64,
    info: Value,
    start: Instant,
    /// (R5/S) The host's feed rule name (`--feed-rule`), for a page's `feed_exempt=0`.
    feed_rule: String,
    /// (R6/T7) The live side, its connection thread's control and the notifications
    /// its inputs caused since the last broadcast.
    live: Option<crate::livehost::LiveRuntime>,
    conn_ctl: Vec<(&'static str, Sender<crate::livehost::ConnCtl>)>,
    pending: Vec<qp_schema::protocol::OutboundLine>,
    quit: bool,
}

#[allow(clippy::expect_used)]
/// The replay host inside the hub.
fn host_of(h: &qp_mcp::SharedHub) -> std::cell::RefMut<'_, ReplayHost> {
    std::cell::RefMut::map(h.borrow_mut(), |h| {
        h.replay_host().expect("the demo hub wraps a replay host")
    })
}

/// Runs the demo host until interrupted.
///
/// # Errors
/// The host cannot start or the port cannot be bound.
/// (R6/M24) Least interval between two viewer recomputations in a live host.
const VIEWER_FLUSH_MS: u64 = 250;

#[allow(clippy::too_many_lines, clippy::cognitive_complexity)]
pub fn demo(a: &DemoArgs) -> Result<(), String> {
    if a.https_host.as_deref().is_some_and(|n| n.trim().is_empty()) {
        return Err("--https-host: empty name".to_owned());
    }
    if (a.live || a.sim) && (a.lan || a.https_host.is_some() || a.bind != "127.0.0.1") {
        let flag = if a.sim { "--sim" } else { "--live" };
        return Err(format!(
            "{flag}: real market data stays on this machine; --lan, --https-host and \
--bind other than 127.0.0.1 are refused with {flag} (to view it elsewhere, run your own \
private proxy, e.g. Tailscale, to 127.0.0.1 — see docs/matriks/LIVE.md)"
        ));
    }
    // (R5B audit B3) the agent endpoint has no token in the demo: this machine only
    if a.mcp && (a.lan || a.https_host.is_some() || a.bind != "127.0.0.1") {
        return Err(
            "--mcp: the agent endpoint serves this machine only; --lan, \
--https-host and --bind other than 127.0.0.1 are refused with --mcp (for remote agents use \
`qp mcp serve --http --listen ADDR --token-env VAR`)"
                .to_owned(),
        );
    }
    let (tx, rx) = mpsc::channel::<Cmd>();
    let mut live_parts = None;
    let mut sim_parts = None;
    let host = if a.live {
        let (h, parts) = live_setup(a, &tx)?;
        live_parts = Some(parts);
        h
    } else if a.sim {
        let (h, parts) = sim_setup(a, &tx)?;
        sim_parts = Some(parts);
        h
    } else {
        host(a)?
    };
    let st = host.state();
    let bind = if a.lan { "0.0.0.0" } else { a.bind.as_str() };
    let listener =
        TcpListener::bind((bind, a.port)).map_err(|e| format!("bind {bind}:{}: {e}", a.port))?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let mut urls = vec![format!("http://localhost:{port}/")];
    let lan_ip = (bind == "0.0.0.0").then(local_ip).flatten();
    let phone = lan_ip.as_ref().map(|ip| format!("http://{ip}:{port}/"));
    if let Some(u) = &phone {
        urls.push(u.clone());
    }
    let mut hosts: Vec<String> = ["localhost", "127.0.0.1", "[::1]"]
        .iter()
        .map(|h| format!("{h}:{port}"))
        .collect();
    if bind != "0.0.0.0" && bind != "127.0.0.1" {
        hosts.push(format!("{bind}:{port}"));
    }
    if let Some(ip) = &lan_ip {
        hosts.push(format!("{ip}:{port}"));
    }
    let https_hosts: Vec<String> = a
        .https_host
        .iter()
        .map(|n| n.trim().to_ascii_lowercase())
        .filter(|n| !n.is_empty())
        .flat_map(|n| [n.clone(), format!("{n}:443")])
        .collect();
    if let Some(n) = &a.https_host {
        urls.push(format!("https://{}/", n.trim()));
    }
    let gate = std::sync::Arc::new(Gate {
        live_control: live_parts
            .as_ref()
            .map(|p| p.control.clone())
            .or_else(|| sim_parts.as_ref().map(|p| p.control.clone())),
        hosts,
        https_hosts,
        shell_key: if a.mcp { Some(secret()?) } else { None },
        mcp: a.mcp.then_some(crate::mcp_serve::HttpGate {
            port,
            // `Host` is checked for every request above.
            hosts: None,
            token: None,
            principal_tokens: Vec::new(),
        }),
    });
    let mut info = json!({"urls": urls, "phone": phone, "lan": bind != "127.0.0.1",
        "synthetic": st.synthetic, "note": st.note});
    if let Some(p) = &live_parts {
        info["live"] = json!(true);
        info["delayed"] = json!(p.delayed);
        info["symbols"] = json!(p.symbols);
    }
    if let Some(p) = &sim_parts {
        info["live"] = json!(true);
        info["sim"] = json!(true);
        info["delayed"] = json!(false);
        info["symbols"] = json!(p.symbols);
    }
    match &gate.shell_key {
        Some(k) => println!("qp demo: {}?key={k}", urls[0]),
        None => println!("qp demo: {}", urls[0]),
    }
    if a.gallery {
        for u in &urls {
            println!("galeri / gallery: {u}gallery.html");
        }
    }
    if let Some(n) = &a.https_host {
        println!(
            "https (vekil / proxy): https://{}/ -> 127.0.0.1:{port}",
            n.trim()
        );
    }
    match &phone {
        Some(u) => {
            println!("telefondan aç / open on your phone (same network): {u}");
            println!("uyarı: yerel ağdaki cihazlar gösterimi görebilir / warning: devices on the local network can see the demo");
        }
        None if live_parts.is_none() && sim_parts.is_none() => println!("telefondan açmak için: scripts/demo.sh --lan / to open on a phone: scripts/demo.sh --lan"),
        None => {}
    }
    if let Some(p) = &live_parts {
        println!(
            "veri: canlı / data: live ({} symbols, {}, {})",
            p.symbols,
            a.interval,
            if p.delayed { "delayed" } else { "real time" }
        );
        println!("canlı denetim / live control: GET /live/stats ; POST /live/drop?secs=N, POST /live/quit (header X-Qp-Live-Control)");
    } else if let Some(p) = &sim_parts {
        println!(
            "veri: benzetim / data: simulation ({} symbols, {}, {} trades, {})",
            p.symbols,
            a.interval,
            p.input.trade_count(),
            p.speed.label()
        );
        println!("denetim / control: GET /live/stats (header X-Qp-Live-Control)");
    } else if st.synthetic {
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
    let gallery = a
        .gallery
        .then(|| std::sync::Arc::new(GalleryStore::default()));
    if let Some(g) = &gallery {
        // Built ahead in the background (the page's first language and theme first).
        let g = g.clone();
        std::thread::spawn(move || {
            for (l, t) in [
                ("tr", "dark"),
                ("tr", "light"),
                ("en", "dark"),
                ("en", "light"),
            ] {
                let _ = g.get(l, t);
            }
        });
    }
    let web = a.web.clone();
    std::thread::spawn(move || {
        for s in listener.incoming().flatten() {
            let (web, tx, gate, gallery) = (web.clone(), tx.clone(), gate.clone(), gallery.clone());
            std::thread::spawn(move || connection(s, &web, &tx, &gate, gallery.as_deref()));
        }
    });
    let mcp_host = if a.mcp {
        let init = host.init();
        crate::mcp_serve::check_principal(init, "claude")?;
        println!("mcp (ajan / agent): http://127.0.0.1:{port}/mcp (claude)");
        Some(crate::mcp_serve::McpHost::new(crate::mcp_serve::HostCfg {
            default_principal: "claude".to_owned(),
            // Only `claude`: another `X-Qp-Principal` is 403 (`qp mcp serve` rule 1).
            allowed: vec!["claude".to_owned()],
            surface: "thin".to_owned(),
            device: serde_json::from_value(json!({"kind": "headless"}))
                .map_err(|e| e.to_string())?,
            log: None,
            capsule_format: None,
        }))
    } else {
        None
    };
    let mut hub = Hub {
        epoch: host.state().epoch,
        sent_events: host.recorded_len(),
        mcp: qp_mcp::Hub::shared_replay(host),
        mcp_host,
        clients: BTreeMap::new(),
        next: 0,
        info,
        start: Instant::now(),
        feed_rule: a.feed_rule.clone(),
        live: None,
        conn_ctl: Vec::new(),
        pending: Vec::new(),
        quit: false,
    };
    if let Some(p) = live_parts {
        let (rt, ctl) = p.start(&host_of(&hub.mcp))?;
        hub.live = Some(rt);
        hub.conn_ctl = ctl;
    }
    if let Some(p) = sim_parts {
        hub.live = Some(p.runtime);
        let iv = qp_calendar::Interval::parse(&a.interval).map_err(|e| e.to_string())?;
        crate::sim::spawn_feeder(
            std::sync::Arc::new(p.input),
            iv,
            p.speed,
            p.to_hub,
            Box::new(|st: crate::sim::DriveStats| {
                println!(
                    "benzetim bitti / simulation done: {} trades, {} days",
                    st.trades, st.days
                );
                let _ = std::io::stdout().flush();
            }),
        );
    }
    let mut last_flush = Instant::now();
    loop {
        let w = hub.wall();
        let wait = host_of(&hub.mcp).next_due_in(w).unwrap_or(40).clamp(1, 40);
        match rx.recv_timeout(Duration::from_millis(wait)) {
            Ok(cmd) => {
                hub.handle(cmd);
                // (R6/T7) A live burst is taken as one batch: one broadcast.
                if hub.live.is_some() {
                    for _ in 0..5000 {
                        match rx.try_recv() {
                            Ok(c) => hub.handle(c),
                            Err(_) => break,
                        }
                    }
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return Ok(()),
        }
        let w = hub.wall();
        let mut lines = host_of(&hub.mcp).pump(w);
        if let Some(l) = hub.live.as_mut() {
            lines.extend(std::mem::take(&mut hub.pending));
            lines.extend(l.tick(&mut host_of(&hub.mcp)));
            // (R6/M24) Viewers' feeds and faces are recomputed at most every
            // VIEWER_FLUSH_MS (the profile: recomputing every reader's capsule sentences
            // after each live batch dominated the host's CPU); data frames still go out
            // every turn, so tick-to-frame for the open instrument is unchanged.
            if last_flush.elapsed() >= Duration::from_millis(VIEWER_FLUSH_MS) {
                lines.extend(host_of(&hub.mcp).live_flush());
                last_flush = Instant::now();
            }
        }
        hub.broadcast();
        hub.route(lines);
        if let Some(m) = hub.mcp_host.as_mut() {
            m.flush();
        }
        if let Some(l) = hub.live.as_mut() {
            l.after_broadcast();
            l.watch_capsules(&host_of(&hub.mcp));
        }
        if hub.quit {
            hub.shutdown_live();
            return Ok(());
        }
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
    let mut host = qp_shellcore::drive::parity_host_n(a.instruments);
    let out = match a.mode.as_str() {
        "replica" => qp_shellcore::drive::shell_less(&mut host, &a.principal, &a.lang, &script),
        "viewer" => qp_shellcore::drive::viewer_less(&mut host, &a.principal, &a.lang, &script),
        m => return Err(format!("--mode: `{m}` (replica or viewer)")),
    };
    let s = serde_json::to_string_pretty(&out).map_err(|e| e.to_string())?;
    match &a.out {
        Some(p) => std::fs::write(p, s + "\n").map_err(|e| format!("{}: {e}", p.display())),
        None => {
            println!("{s}");
            Ok(())
        }
    }
}
