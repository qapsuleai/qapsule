//! (R6) Matriks REST with the same service token: settled bars ("kesin bar çekme",
//! `docs/DATA_ACCESS.md`) and the symbol list (meta). Sequential, rate-limited calls; the
//! token goes only into the `Authorization` header and never into a message.
//!
//! Base: `https://api.matriksdata.com` (the only allowed REST host, `docs/DATA_ACCESS.md`).

use std::io::Read;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::auth::AuthClient;
use crate::redact::Redactor;

const BASE: &str = "https://api.matriksdata.com";
/// Minimum pause between two REST calls.
const MIN_GAP: Duration = Duration::from_millis(700);

/// One REST bar as served (`time` epoch ms; `volume` TL value, `totalQuantity` lots).
#[derive(Debug, Clone, PartialEq)]
pub struct RestBar {
    /// Bar open, epoch ms (daily: Istanbul midnight).
    pub time: i64,
    /// Open.
    pub open: f64,
    /// High.
    pub high: f64,
    /// Low.
    pub low: f64,
    /// Close.
    pub close: f64,
    /// Traded value (TL).
    pub volume: f64,
    /// Traded quantity (lots), if served.
    pub quantity: Option<f64>,
}

/// REST error; its text is masked.
#[derive(Debug)]
pub struct RestError(pub String);

impl std::fmt::Display for RestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for RestError {}

/// A REST client over the connection's [`AuthClient`].
pub struct RestClient<'a> {
    auth: &'a AuthClient,
    redactor: Redactor,
    agent: ureq::Agent,
    last: Mutex<Option<Instant>>,
    /// Calls made.
    pub calls: std::sync::atomic::AtomicU64,
}

impl<'a> RestClient<'a> {
    /// A client; `redactor` masks every error text.
    pub fn new(auth: &'a AuthClient, redactor: Redactor) -> Self {
        let config = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(60)))
            .http_status_as_error(false)
            .max_redirects(0)
            .proxy(None)
            .build();
        Self {
            auth,
            redactor,
            agent: ureq::Agent::new_with_config(config),
            last: Mutex::new(None),
            calls: std::sync::atomic::AtomicU64::new(0),
        }
    }

    fn pace(&self) {
        let mut l = self
            .last
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(t) = *l {
            let e = t.elapsed();
            if e < MIN_GAP {
                std::thread::sleep(MIN_GAP - e);
            }
        }
        *l = Some(Instant::now());
    }

    fn err(&self, what: &str, e: impl std::fmt::Display) -> RestError {
        RestError(self.redactor.scrub(&format!("rest {what}: {e}")))
    }

    /// GET `path?query` with the token; the body (gzip-decoded if needed) as bytes.
    fn get(&self, path: &str, query: &[(&str, String)]) -> Result<Vec<u8>, RestError> {
        let tok = self.auth.token().map_err(|e| self.err(path, e))?;
        self.pace();
        self.calls
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let mut req = self
            .agent
            .get(format!("{BASE}{path}"))
            .header("Authorization", format!("jwt {}", tok.bearer()))
            .header("Accept-Encoding", "gzip");
        for (k, v) in query {
            req = req.query(*k, v);
        }
        let resp = req.call().map_err(|e| self.err(path, e))?;
        let status = resp.status().as_u16();
        let mut body = Vec::new();
        resp.into_body()
            .into_reader()
            .read_to_end(&mut body)
            .map_err(|e| self.err(path, e))?;
        if status != 200 {
            return Err(RestError(format!("rest {path}: HTTP {status}")));
        }
        if body.starts_with(&[0x1f, 0x8b]) {
            let mut out = Vec::new();
            flate2::read::GzDecoder::new(&body[..])
                .read_to_end(&mut out)
                .map_err(|e| self.err(path, e))?;
            body = out;
        }
        Ok(body)
    }

    /// Bars of `symbol` for `period` (`1min`, `5min`, `1hour`, `1day`, or a derived
    /// period: `10min`, `15min`, `30min`, `120min`, `240min`, `weekly`…) over local dates
    /// `start..=end` (`YYYY-MM-DD`); `adjusted = false` sends `divide=false`.
    ///
    /// # Errors
    /// Token, HTTP or JSON failure (masked text).
    pub fn bars(
        &self,
        symbol: &str,
        period: &str,
        start: &str,
        end: &str,
        adjusted: bool,
    ) -> Result<Vec<RestBar>, RestError> {
        let path = if matches!(period, "1min" | "5min" | "1hour" | "1day") {
            "/dumrul/v1/tick/bar.gz"
        } else {
            "/dumrul/v1/tick/derived-bar"
        };
        let mut q = vec![
            ("symbol", symbol.to_owned()),
            ("period", period.to_owned()),
            ("start", start.to_owned()),
            ("end", end.to_owned()),
        ];
        if !adjusted {
            q.push(("divide", "false".to_owned()));
        }
        let body = self.get(path, &q)?;
        let v: Value = serde_json::from_slice(&body).map_err(|e| self.err(path, e))?;
        let rows = v
            .as_array()
            .ok_or_else(|| RestError(format!("rest {path}: not a JSON array")))?;
        let f = |r: &Value, k: &str| r.get(k).and_then(Value::as_f64);
        Ok(rows
            .iter()
            .filter_map(|r| {
                Some(RestBar {
                    time: r.get("time")?.as_i64()?,
                    open: f(r, "open")?,
                    high: f(r, "high")?,
                    low: f(r, "low")?,
                    close: f(r, "close")?,
                    volume: f(r, "volume").unwrap_or(0.0),
                    quantity: f(r, "totalQuantity"),
                })
            })
            .collect())
    }

    /// The symbol list (`/dumrul/v4/meta/symbols.gz`) as served (JSON value).
    ///
    /// # Errors
    /// Token, HTTP or JSON failure (masked text).
    pub fn symbols(&self) -> Result<Value, RestError> {
        let path = "/dumrul/v4/meta/symbols.gz";
        let body = self.get(path, &[])?;
        serde_json::from_slice(&body).map_err(|e| self.err(path, e))
    }
}
