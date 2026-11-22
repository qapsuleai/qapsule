//! The demo server's HTTP side: static files, routes, the event stream and the gallery ([`connection`]; RG1: split out of `demo.rs`).

use super::*;

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
pub(super) struct Gate {
    /// (R6/T7) The live control secret (`X-Qp-Live-Control`); `None`: not live.
    pub(super) live_control: Option<String>,
    pub(super) hosts: Vec<String>,
    /// (RP3) Host values that arrive through the HTTPS reverse proxy (`--https-host`):
    /// their pages' origin is `https://NAME`.
    pub(super) https_hosts: Vec<String>,
    /// (R5B) The MCP endpoint's rules (`--mcp`).
    pub(super) mcp: Option<crate::mcp_serve::HttpGate>,
    /// (R5B audit B2) With `--mcp`: the launch key a shell stream needs (`/events?key=`),
    /// printed only in the address on the terminal. A local process (an agent with a
    /// shell, say) that does not have it cannot open a person's connection and decide a
    /// proposal there.
    pub(super) shell_key: Option<String>,
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

pub(super) fn connection(
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
    if path == "/mcp" {
        let Some(mg) = &gate.mcp else {
            respond(
                &mut s,
                "404 Not Found",
                "text/plain",
                b"mcp off (qp demo --mcp)",
            );
            return;
        };
        let mut body = vec![0u8; len.min(8 << 20)];
        if r.read_exact(&mut body).is_err() {
            respond(&mut s, "400 Bad Request", "text/plain", b"short body");
            return;
        }
        crate::mcp_serve::handle(&mut s, method, &headers, &body, mg, &|j| {
            let _ = tx.send(Cmd::Mcp(j));
        });
        return;
    }
    match (method, path) {
        ("GET", "/events") => {
            if let Some(k) = &gate.shell_key {
                if query_param(q, "key").as_deref() != Some(k.as_str()) {
                    respond(
                        &mut s,
                        "403 Forbidden",
                        "text/plain",
                        b"key: open the address printed by qp demo",
                    );
                    return;
                }
            }
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
        let endpoint = query_param(q, "endpoint");
        let _ = tx.send(Cmd::LiveControl {
            drop_secs,
            endpoint,
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
pub(super) struct GalleryStore {
    builds:
        std::sync::Mutex<BTreeMap<(String, String), std::sync::Arc<qp_shellcore::gallery::Build>>>,
}

impl GalleryStore {
    pub(super) fn get(
        &self,
        lang: &str,
        theme: &str,
    ) -> std::sync::Arc<qp_shellcore::gallery::Build> {
        let lang = if lang == "en" { "en" } else { "tr" };
        let theme = if theme == "light" { "light" } else { "dark" };
        // One build at a time (the lock is held while building: a second request for the
        // same key waits and then finds it).
        let mut m = self
            .builds
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        m.entry((lang.to_owned(), theme.to_owned()))
            .or_insert_with(|| std::sync::Arc::new(qp_shellcore::gallery::build(lang, theme)))
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
