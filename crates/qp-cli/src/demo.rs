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
use qp_wasm::shell::{demo_session_init, DemoOptions};
use serde_json::{json, Value};

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
        feed_rule: Some(a.feed_rule.clone()),
    };
    let init = demo_session_init(&o);
    let dir = if a.synthetic {
        PathBuf::new()
    } else {
        a.data.clone().unwrap_or_else(qp_source::local_bars_dir)
    };
    // (R4/T6) `--history` also bounds the history before `--from` (else every earlier
    // bar: on the full local history the page's first capsule took seconds).
    let mut h = ReplayHost::with_window(init, &a.root, &dir, Some(a.history as usize))?;
    h.set_record_events(true);
    Ok(h)
}

/// (R6/T7) What the live demo needs once the hub exists.
struct LiveParts {
    control: String,
    delayed: bool,
    symbols: usize,
    interval: String,
    delay_ms: i64,
    cal: qp_calendar::ExchangeCalendar,
    source: std::sync::Arc<dyn crate::livehost::BarSource>,
    to_hub: std::sync::Arc<dyn Fn(crate::livehost::LiveIn) + Send + Sync>,
    spec: crate::livehost::ConnSpec,
    sink: std::sync::Arc<crate::livehost::HubSink>,
}

impl LiveParts {
    /// The runtime (feeds seeded at the history's last bars) and the connection thread.
    fn start(
        self,
        host: &ReplayHost,
    ) -> Result<
        (
            crate::livehost::LiveRuntime,
            Sender<crate::livehost::ConnCtl>,
        ),
        String,
    > {
        let feed = Box::new(crate::livehost::MatriksFeed::new(
            self.cal.clone(),
            &self.interval,
        ));
        let mut rt = crate::livehost::LiveRuntime::new(
            host,
            feed,
            self.source,
            self.to_hub.clone(),
            self.cal,
            &self.interval,
            self.delay_ms,
        )?;
        rt.baseline_capsules(host);
        let ctl = crate::livehost::spawn_connection(self.spec, self.sink, self.to_hub)?;
        Ok((rt, ctl))
    }
}

/// (R6/T7) The live demo's host: the universe's history from REST is the initial
/// resolution; the connection is opened later ([`LiveParts::start`]).
fn live_setup(a: &DemoArgs, tx: &Sender<Cmd>) -> Result<(ReplayHost, LiveParts), String> {
    use std::sync::Arc;
    let symbols = crate::live::parse_symbols(&a.symbols).map_err(|e| {
        format!("--symbols: {e} (default: the universe file fixtures/local/live/u30.txt)")
    })?;
    crate::livehost::rest_period(&a.interval)
        .ok_or_else(|| format!("--interval: no REST period for `{}`", a.interval))?;
    let cfg = qp_live::LiveConfig::from_env().map_err(|e| format!("--live: {e}"))?;
    if cfg.hardware_id_warning() {
        eprintln!("live: warning: MATRIKS_HARDWARE_ID does not start with `qapsule-`");
    }
    let xist = qp_calendar::builtin("XIST").ok_or("no XIST calendar")?;
    let cal = qp_calendar::ExchangeCalendar::new(&xist).map_err(|e| e.to_string())?;
    let delayed = cfg.mode() == qp_live::Mode::Delayed;
    let delay_ms = if delayed { 15 * 60_000 } else { 0 };
    let auth = Arc::new(qp_live::AuthClient::new(
        &cfg,
        qp_live::AuthPolicy::default(),
    ));
    let source = Arc::new(crate::livehost::RestSource {
        cfg: cfg.clone(),
        auth: auth.clone(),
        cal: cal.clone(),
    });
    println!(
        "geçmiş / history: {} symbols x {} bars of {} (REST, unadjusted)…",
        symbols.len(),
        a.history,
        a.interval
    );
    let _ = std::io::stdout().flush();
    let cutoff = crate::live::now_us() / 1000 - delay_ms;
    let (hist, missing) = crate::livehost::history(
        source.as_ref(),
        &symbols,
        &a.interval,
        a.history as usize,
        cutoff,
        &cal,
    )?;
    if !missing.is_empty() {
        println!("geçmiş yok / no history: {}", missing.join(", "));
    }
    if hist.is_empty() {
        return Err("--live: no symbol has history".to_owned());
    }
    let n = hist.len();
    let topics: Vec<String> = hist
        .iter()
        .map(|(s, _)| format!("mx/symbol/{s}@lvl2"))
        .collect();
    let feeds: Vec<qp_source::ReplayFeed> = hist
        .into_iter()
        .map(|(s, bars)| qp_source::ReplayFeed {
            key: FeedKey {
                symbol: s,
                interval: a.interval.clone(),
                feed: None,
            },
            bars,
            fine: Vec::new(),
        })
        .collect();
    let mut init = demo_session_init(&DemoOptions {
        local: true,
        instruments: u32::try_from(n).unwrap_or(u32::MAX),
        history: a.history,
        days: a.days,
        speed: a.speed,
        from_t: None,
        running: true,
        feed_rule: Some(a.feed_rule.clone()),
    });
    if let Some(r) = init.replay.as_mut() {
        r.interval = Some(a.interval.clone());
        r.fine = None;
    }
    let mut h = ReplayHost::live(init, &a.root, feeds)?;
    h.set_note(Some(format!(
        "canlı / live · {n} · {}{}",
        a.interval,
        if delayed {
            " · gecikmeli 15 dk / delayed 15 min"
        } else {
            ""
        }
    )));
    let root = crate::live::repo_root();
    let raw_path = root.join("fixtures/local/live/raw").join(format!(
        "{}-demo.qpraw",
        crate::live::utc_stamp(crate::live::now_us())
    ));
    let txc = Mutex::new(tx.clone());
    let to_hub: Arc<dyn Fn(crate::livehost::LiveIn) + Send + Sync> = Arc::new(move |i| {
        let _ = txc
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .send(Cmd::Live(i));
    });
    let sink = Arc::new(crate::livehost::HubSink::new(
        crate::live::RawSink::create(&raw_path)?,
        to_hub.clone(),
    ));
    let control = match std::env::var("QP_LIVE_CONTROL") {
        Ok(v) if v.len() >= 16 => v,
        _ => {
            let s = secret()?;
            println!("canlı denetim belirteci / live control secret: {s}");
            s
        }
    };
    let parts = LiveParts {
        control,
        delayed,
        symbols: n,
        interval: a.interval.clone(),
        delay_ms,
        cal,
        source,
        to_hub,
        spec: crate::livehost::ConnSpec {
            cfg,
            auth,
            topics,
            lock_dir: root.join("fixtures/local"),
        },
        sink,
    };
    Ok((h, parts))
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
    /// (R6/T7) `GET /live/stats`.
    LiveStats {
        reply: Sender<String>,
    },
    /// (R6/T7) `POST /live/drop` (`secs`) and `POST /live/quit` (`None`).
    LiveControl {
        drop_secs: Option<u64>,
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
fn secret() -> Result<String, String> {
    let mut b = [0u8; 16];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut b))
        .map_err(|e| format!("no OS randomness for the connection secret: {e}"))?;
    Ok(b.iter().map(|x| format!("{x:02x}")).collect())
}

/// Constant-time comparison of two secrets.
fn same_secret(a: &str, b: &str) -> bool {
    a.len() == b.len()
        && a.bytes()
            .zip(b.bytes())
            .fold(0u8, |acc, (x, y)| acc | (x ^ y))
            == 0
}

struct Hub {
    host: ReplayHost,
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
    conn_ctl: Option<Sender<crate::livehost::ConnCtl>>,
    pending: Vec<qp_schema::protocol::OutboundLine>,
    quit: bool,
}

impl Hub {
    fn wall(&self) -> u64 {
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
        let mut data: InitData = self.host.init().data.clone().unwrap_or_default();
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

    /// Sends new data events (or, after a backward seek, a new start) to every client;
    /// (RP3/M15) a viewer gets only the events of the feeds it is served, then a `bind`
    /// frame for each feed its viewer subscription newly names.
    fn broadcast(&mut self) {
        let st = self.host.state();
        if st.epoch != self.epoch {
            self.epoch = st.epoch;
            self.sent_events = self.host.recorded_events().len();
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
        let evs = self.host.recorded_events();
        if evs.len() > self.sent_events {
            let new = &evs[self.sent_events..];
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
            self.sent_events = evs.len();
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
            let want: Vec<FeedKey> = self
                .host
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
                if let Some((f, e)) = self.host.feed_data(k) {
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

    fn handle(&mut self, cmd: Cmd) {
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
                    Some(r) => self.host.set_principal_coin_flip_rule(&principal, Some(r)),
                    None => self.host.clear_principal_coin_flip_rule(&principal),
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
                    let lines = self.host.handle_raw(&conn, &text);
                    // (RP3/M15) Data and `bind` frames first: a viewer's open capsule
                    // finds its instrument's data when its slice arrives.
                    self.broadcast();
                    self.route(lines);
                }
            }
            Cmd::Gone { conn } => {
                self.clients.remove(&conn);
            }
            Cmd::Live(input) => {
                if let Some(l) = self.live.as_mut() {
                    let lines = l.on_in(&mut self.host, input);
                    self.pending.extend(lines);
                }
            }
            Cmd::LiveStats { reply } => {
                let v = self
                    .live
                    .as_ref()
                    .map_or(Value::Null, |l| l.stats(&self.host));
                let _ = reply.send(v.to_string());
            }
            Cmd::LiveControl { drop_secs, reply } => {
                let Some(ctl) = &self.conn_ctl else {
                    let _ = reply.send(r#"{"error":"not live"}"#.to_owned());
                    return;
                };
                match drop_secs {
                    Some(secs) => {
                        if let Some(l) = self.live.as_mut() {
                            l.drops_asked += 1;
                        }
                        let _ = ctl.send(crate::livehost::ConnCtl::Drop(Duration::from_secs(
                            secs.min(600),
                        )));
                        let _ = reply.send(json!({"drop": secs.min(600)}).to_string());
                    }
                    None => {
                        self.quit = true;
                        let _ = reply.send(r#"{"quit":true}"#.to_owned());
                    }
                }
            }
        }
    }

    /// (R6/T7) Closes the live connection (DISCONNECT) and prints the summary.
    fn shutdown_live(&mut self) {
        if let Some(ctl) = self.conn_ctl.take() {
            let (tx, rx) = mpsc::channel();
            let _ = ctl.send(crate::livehost::ConnCtl::Stop(tx));
            let _ = rx.recv_timeout(Duration::from_secs(10));
        }
        if let Some(l) = &self.live {
            let st = l.stats(&self.host);
            println!("canlı özet / live summary: {st}");
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

/// What a request must look like to be served (`Host` values allowed).
struct Gate {
    /// (R6/T7) The live control secret (`X-Qp-Live-Control`); `None`: not live.
    live_control: Option<String>,
    hosts: Vec<String>,
    /// (RP3) Host values that arrive through the HTTPS reverse proxy (`--https-host`):
    /// their pages' origin is `https://NAME`.
    https_hosts: Vec<String>,
}

impl Gate {
    fn host_ok(&self, host: Option<&str>) -> bool {
        host.is_some_and(|h| {
            self.hosts.iter().any(|x| x.eq_ignore_ascii_case(h))
                || self.https_hosts.iter().any(|x| x.eq_ignore_ascii_case(h))
        })
    }
    fn origin_ok(&self, origin: Option<&str>, host: Option<&str>) -> bool {
        match (origin, host) {
            (Some(o), Some(h)) if self.https_hosts.iter().any(|x| x.eq_ignore_ascii_case(h)) => {
                let name = h.strip_suffix(":443").unwrap_or(h);
                o.eq_ignore_ascii_case(&format!("https://{name}"))
            }
            (Some(o), Some(h)) => o.eq_ignore_ascii_case(&format!("http://{h}")),
            _ => false,
        }
    }
}

fn connection(
    mut s: TcpStream,
    web: &Path,
    tx: &Sender<Cmd>,
    gate: &Gate,
    gallery: Option<&GalleryStore>,
) {
    let Ok(peer) = s.try_clone() else { return };
    let mut r = BufReader::new(peer);
    let mut line = String::new();
    if r.read_line(&mut line).is_err() {
        return;
    }
    let mut parts = line.split_whitespace();
    let (method, target) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
    let mut len = 0usize;
    let mut headers: BTreeMap<String, String> = BTreeMap::new();
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
    let host = headers.get("host").map(String::as_str);
    if !gate.host_ok(host) {
        respond(&mut s, "403 Forbidden", "text/plain", b"unknown Host");
        return;
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
            // (RP3/M15) Viewer unless the page asks for the full replica.
            let viewer = query_param(q, "mode").as_deref() != Some("replica");
            // (R5/M19) `feed_rule=`: one of the host's rule names, else ignored.
            let feed_rule = query_param(q, "feed_rule")
                .filter(|n| qp_capsule::feedrule::FEED_RULES.iter().any(|(k, _)| k == n));
            // (R5/S) `feed_exempt=0`: the rule without the K42 exemption.
            let no_exempt = query_param(q, "feed_exempt").as_deref() == Some("0");
            let (ftx, frx) = mpsc::channel::<String>();
            let _ = tx.send(Cmd::Connect {
                principal,
                lang: lang.to_owned(),
                viewer,
                feed_rule,
                no_exempt,
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
            if !gate.origin_ok(headers.get("origin").map(String::as_str), host) {
                respond(&mut s, "403 Forbidden", "text/plain", b"cross-origin");
                return;
            }
            if !headers
                .get("content-type")
                .is_some_and(|c| c.to_ascii_lowercase().starts_with("application/json"))
            {
                respond(
                    &mut s,
                    "415 Unsupported Media Type",
                    "text/plain",
                    b"json only",
                );
                return;
            }
            let (Some(conn), Some(token)) = (query_param(q, "conn"), headers.get("x-qp-token"))
            else {
                respond(
                    &mut s,
                    "403 Forbidden",
                    "text/plain",
                    b"no connection secret",
                );
                return;
            };
            let (rtx, rrx) = mpsc::channel();
            let _ = tx.send(Cmd::Send {
                conn,
                token: token.clone(),
                body: String::from_utf8_lossy(&body).into_owned(),
                reply: rtx,
            });
            if rrx.recv_timeout(Duration::from_secs(30)).unwrap_or(false) {
                respond(&mut s, "204 No Content", "text/plain", b"");
            } else {
                respond(
                    &mut s,
                    "403 Forbidden",
                    "text/plain",
                    b"wrong connection secret",
                );
            }
        }
        ("GET", "/live/stats") | ("POST", "/live/drop" | "/live/quit") => {
            live_route(&mut s, method, path, q, &headers, tx, gate);
        }
        ("GET", p) if gallery_route(&mut s, p, q, gallery) => {}
        ("GET", p) => {
            // RP3/M13: pre-compressed variants and revalidation (demo_static.rs).
            let served = static_path(web, p).is_some_and(|f| {
                crate::demo_static::serve(&mut s, &f, content_type(&f), CSP, &headers)
            });
            if !served {
                respond(&mut s, "404 Not Found", "text/plain", b"not found");
            }
        }
        _ => respond(&mut s, "405 Method Not Allowed", "text/plain", b""),
    }
}

/// (R6/T7) The live host's local routes: `GET /live/stats` (aggregate counts, no values),
/// `POST /live/drop?secs=N` (close the connection, wait N s, open again: M23 by hand) and
/// `POST /live/quit` (close the connection and end). Only from this machine (a loopback
/// peer) and, for the two `POST`s, with the control secret in `X-Qp-Live-Control`.
fn live_route(
    s: &mut TcpStream,
    method: &str,
    path: &str,
    q: &str,
    headers: &BTreeMap<String, String>,
    tx: &Sender<Cmd>,
    gate: &Gate,
) {
    let local = s.peer_addr().is_ok_and(|a| a.ip().is_loopback());
    let Some(secret) = gate.live_control.as_deref().filter(|_| local) else {
        respond(s, "404 Not Found", "text/plain", b"not found");
        return;
    };
    let (rtx, rrx) = mpsc::channel();
    if method == "GET" {
        let _ = tx.send(Cmd::LiveStats { reply: rtx });
    } else {
        let ok = headers
            .get("x-qp-live-control")
            .is_some_and(|t| same_secret(t, secret));
        if !ok {
            respond(
                s,
                "403 Forbidden",
                "text/plain",
                b"live control secret needed",
            );
            return;
        }
        let drop_secs = (path == "/live/drop").then(|| {
            query_param(q, "secs")
                .and_then(|x| x.parse().ok())
                .unwrap_or(30)
        });
        let _ = tx.send(Cmd::LiveControl {
            drop_secs,
            reply: rtx,
        });
    }
    match rrx.recv_timeout(Duration::from_secs(10)) {
        Ok(body) => respond(s, "200 OK", "application/json", body.as_bytes()),
        Err(_) => respond(s, "503 Service Unavailable", "text/plain", b"busy"),
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

/// (R5/G) The gallery builds, by `(lang, theme)`, made on first request (or ahead, in the
/// background) and kept.
#[derive(Default)]
struct GalleryStore {
    builds: std::sync::Mutex<BTreeMap<(String, String), std::sync::Arc<qp_wasm::gallery::Build>>>,
}

impl GalleryStore {
    fn get(&self, lang: &str, theme: &str) -> std::sync::Arc<qp_wasm::gallery::Build> {
        let lang = if lang == "en" { "en" } else { "tr" };
        let theme = if theme == "light" { "light" } else { "dark" };
        // One build at a time (the lock is held while building: a second request for the
        // same key waits and then finds it).
        let mut m = self
            .builds
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        m.entry((lang.to_owned(), theme.to_owned()))
            .or_insert_with(|| std::sync::Arc::new(qp_wasm::gallery::build(lang, theme)))
            .clone()
    }

    /// `/gallery/img/<lang>-<theme>-…png`: the image, from the build its name starts with.
    fn image(&self, name: &str) -> Option<Vec<u8>> {
        let mut it = name.splitn(3, '-');
        let (lang, theme) = (it.next()?, it.next()?);
        if !["tr", "en"].contains(&lang) || !["dark", "light"].contains(&theme) {
            return None;
        }
        self.get(lang, theme).images.get(name).cloned()
    }
}

/// (R5/G) The gallery's routes; `false` when `path` is not one of them.
fn gallery_route(s: &mut TcpStream, path: &str, q: &str, store: Option<&GalleryStore>) -> bool {
    let is_layout = path == "/gallery/layout";
    let img = path.strip_prefix("/gallery/img/");
    if !is_layout && img.is_none() {
        return false;
    }
    let Some(store) = store else {
        respond(
            s,
            "404 Not Found",
            "text/plain",
            b"gallery off (qp demo --gallery)",
        );
        return true;
    };
    if is_layout {
        let lang = query_param(q, "lang").unwrap_or_default();
        let theme = query_param(q, "theme").unwrap_or_default();
        let b = store.get(&lang, &theme);
        let body = serde_json::to_vec(&b.layout).unwrap_or_default();
        respond(s, "200 OK", "application/json", &body);
    } else {
        match img.and_then(|n| store.image(n)) {
            Some(png) => respond(s, "200 OK", "image/png", &png),
            None => respond(s, "404 Not Found", "text/plain", b"not found"),
        }
    }
    true
}

/// Runs the demo host until interrupted.
///
/// # Errors
/// The host cannot start or the port cannot be bound.
pub fn demo(a: &DemoArgs) -> Result<(), String> {
    if a.https_host.as_deref().is_some_and(|n| n.trim().is_empty()) {
        return Err("--https-host: empty name".to_owned());
    }
    if a.live && (a.lan || a.https_host.is_some() || a.bind != "127.0.0.1") {
        return Err(
            "--live: real market data stays on this machine; --lan, --https-host and \
--bind other than 127.0.0.1 are refused with --live (to view it elsewhere, run your own \
private proxy, e.g. Tailscale, to 127.0.0.1 — see docs/matriks/LIVE.md)"
                .to_owned(),
        );
    }
    let (tx, rx) = mpsc::channel::<Cmd>();
    let mut live_parts = None;
    let host = if a.live {
        let (h, parts) = live_setup(a, &tx)?;
        live_parts = Some(parts);
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
        live_control: live_parts.as_ref().map(|p| p.control.clone()),
        hosts,
        https_hosts,
    });
    let mut info = json!({"urls": urls, "phone": phone, "lan": bind != "127.0.0.1",
        "synthetic": st.synthetic, "note": st.note});
    if let Some(p) = &live_parts {
        info["live"] = json!(true);
        info["delayed"] = json!(p.delayed);
        info["symbols"] = json!(p.symbols);
    }
    println!("qp demo: {}", urls[0]);
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
        None => println!("telefondan açmak için: scripts/demo.sh --lan / to open on a phone: scripts/demo.sh --lan"),
    }
    if let Some(p) = &live_parts {
        println!(
            "veri: canlı / data: live ({} symbols, {}, {})",
            p.symbols,
            a.interval,
            if p.delayed { "delayed" } else { "real time" }
        );
        println!("canlı denetim / live control: GET /live/stats ; POST /live/drop?secs=N, POST /live/quit (header X-Qp-Live-Control)");
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
    let mut hub = Hub {
        epoch: host.state().epoch,
        sent_events: host.recorded_events().len(),
        host,
        clients: BTreeMap::new(),
        next: 0,
        info,
        start: Instant::now(),
        feed_rule: a.feed_rule.clone(),
        live: None,
        conn_ctl: None,
        pending: Vec::new(),
        quit: false,
    };
    if let Some(p) = live_parts {
        let (rt, ctl) = p.start(&hub.host)?;
        hub.live = Some(rt);
        hub.conn_ctl = Some(ctl);
    }
    loop {
        let w = hub.wall();
        let wait = hub.host.next_due_in(w).unwrap_or(40).clamp(1, 40);
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
        let mut lines = hub.host.pump(w);
        if let Some(l) = hub.live.as_mut() {
            lines.extend(std::mem::take(&mut hub.pending));
            lines.extend(l.tick(&mut hub.host));
            lines.extend(hub.host.live_flush());
        }
        hub.broadcast();
        hub.route(lines);
        if let Some(l) = hub.live.as_mut() {
            l.after_broadcast();
            l.watch_capsules(&hub.host);
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
    let mut host = qp_wasm::drive::parity_host_n(a.instruments);
    let out = match a.mode.as_str() {
        "replica" => qp_wasm::drive::shell_less(&mut host, &a.principal, &a.lang, &script),
        "viewer" => qp_wasm::drive::viewer_less(&mut host, &a.principal, &a.lang, &script),
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
