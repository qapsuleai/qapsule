//! The loopback HTTP token service ([`MockTokenService`]; split out of `mock.rs` for size).

use super::*;

struct TokenShared {
    expected_basic: String,
    requests: AtomicU64,
    lifetime_secs: AtomicI64,
    fail_next: AtomicU32,
    refuse_next: AtomicU32,
    issued: Mutex<Vec<String>>,
    stop: AtomicBool,
}

/// HTTP token service on 127.0.0.1. Issues unsigned JWTs with `exp = now + lifetime`.
pub struct MockTokenService {
    addr: SocketAddr,
    shared: Arc<TokenShared>,
    thread: Option<JoinHandle<()>>,
}

impl fmt::Debug for MockTokenService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MockTokenService").finish_non_exhaustive()
    }
}

impl MockTokenService {
    /// Accepts Basic `username:password`; tokens live `lifetime_secs`.
    pub fn start(username: &str, password: &str, lifetime_secs: i64) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
        let addr = listener.local_addr().expect("local addr");
        let shared = Arc::new(TokenShared {
            expected_basic: format!(
                "Basic {}",
                STANDARD.encode(format!("{username}:{password}"))
            ),
            requests: AtomicU64::new(0),
            lifetime_secs: AtomicI64::new(lifetime_secs),
            fail_next: AtomicU32::new(0),
            refuse_next: AtomicU32::new(0),
            issued: Mutex::new(Vec::new()),
            stop: AtomicBool::new(false),
        });
        let s = shared.clone();
        let thread = std::thread::spawn(move || {
            for conn in listener.incoming() {
                if s.stop.load(Ordering::SeqCst) {
                    break;
                }
                if let Ok(stream) = conn {
                    handle_token_request(&s, stream);
                }
            }
        });
        Self {
            addr,
            shared,
            thread: Some(thread),
        }
    }

    /// Uses the fake credentials of [`mock_config`].
    pub fn start_default(lifetime_secs: i64) -> Self {
        Self::start(FAKE_USERNAME, FAKE_PASSWORD, lifetime_secs)
    }

    pub fn url(&self) -> String {
        format!("http://{}/issue", self.addr)
    }

    pub fn requests(&self) -> u64 {
        self.shared.requests.load(Ordering::SeqCst)
    }

    pub fn issued(&self) -> Vec<String> {
        self.shared.issued.lock().unwrap().clone()
    }

    pub fn set_lifetime_secs(&self, secs: i64) {
        self.shared.lifetime_secs.store(secs, Ordering::SeqCst);
    }

    /// Next `n` requests answer HTTP 500 (body echoes the Authorization header, to prove
    /// bodies are never logged).
    pub fn fail_next(&self, n: u32) {
        self.shared.fail_next.store(n, Ordering::SeqCst);
    }

    /// Next `n` requests answer `success: false`.
    pub fn refuse_next(&self, n: u32) {
        self.shared.refuse_next.store(n, Ordering::SeqCst);
    }
}

impl Drop for MockTokenService {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.addr);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

fn take_one(a: &AtomicU32) -> bool {
    a.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |v| v.checked_sub(1))
        .is_ok()
}

fn handle_token_request(s: &TokenShared, mut stream: TcpStream) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    let header_end = loop {
        if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break i + 4;
        }
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => return,
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
        }
    };
    let head = String::from_utf8_lossy(&buf[..header_end]).to_string();
    let mut lines = head.lines();
    let request_line = lines.next().unwrap_or("");
    let mut content_length = 0usize;
    let mut auth = String::new();
    for l in lines {
        if let Some((k, v)) = l.split_once(':') {
            match k.trim().to_ascii_lowercase().as_str() {
                "content-length" => content_length = v.trim().parse().unwrap_or(0),
                "authorization" => auth = v.trim().to_string(),
                _ => {}
            }
        }
    }
    while buf.len() < header_end + content_length {
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
        }
    }
    s.requests.fetch_add(1, Ordering::SeqCst);
    let body = &buf[header_end..buf.len().min(header_end + content_length)];
    let parsed: serde_json::Value = serde_json::from_slice(body).unwrap_or_default();
    let (status, out) = if !request_line.starts_with("POST ") {
        (405, "{}".to_string())
    } else if auth != s.expected_basic {
        (401, format!("{{\"error\":\"bad credentials {auth}\"}}"))
    } else if parsed.get("typeCode").and_then(|v| v.as_str()).is_none()
        || parsed.get("hardwareId").and_then(|v| v.as_str()).is_none()
    {
        (400, "{}".to_string())
    } else if take_one(&s.fail_next) {
        (500, format!("{{\"error\":\"internal {auth}\"}}"))
    } else if take_one(&s.refuse_next) {
        (
            200,
            format!("{{\"success\":false,\"code\":\"E_DENIED\",\"message\":\"no {auth}\"}}"),
        )
    } else {
        let n = s.requests.load(Ordering::SeqCst);
        let hwid = parsed["hardwareId"].as_str().unwrap_or("");
        let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"none","typ":"JWT"}"#);
        let claims = serde_json::json!({
            "sub": format!("QTX-{hwid}"),
            "exp": unix_now() + s.lifetime_secs.load(Ordering::SeqCst),
            "n": n,
        });
        let payload = URL_SAFE_NO_PAD.encode(claims.to_string().as_bytes());
        let jwt = format!("{header}.{payload}.mocksig{n}");
        s.issued.lock().unwrap().push(jwt.clone());
        (
            200,
            serde_json::json!({
                "success": true,
                "code": "0",
                "result": {"token": jwt, "userName": "x", "password": "y", "typeCode": "H"},
            })
            .to_string(),
        )
    };
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        405 => "Method Not Allowed",
        _ => "Internal Server Error",
    };
    let resp = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{out}",
        out.len()
    );
    let _ = stream.write_all(resp.as_bytes());
    let _ = stream.flush();
    let _ = stream.shutdown(Shutdown::Both);
}
