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

/// (RP4/T1) Columns of the historical trade CSV that may be kept (level 1 only, K66).
/// Anything else — in particular `buyer` and `seller` (level 2, AKDE) — is dropped
/// before the bytes leave this module; the trade endpoint used here does not serve them,
/// and `trade_bs` is never called.
pub const TRADE_KEEP_COLUMNS: [&str; 7] = [
    "symbol",
    "date_time",
    "timestamp_ns",
    "bidask",
    "mx_id",
    "price",
    "quantity",
];

/// Keep only [`TRADE_KEEP_COLUMNS`] of a trade CSV (header row first). Rows are copied
/// field by field in the kept columns' header order; a header without any kept column
/// yields an empty result.
#[must_use]
pub fn trade_csv_level1(csv: &[u8]) -> Vec<u8> {
    let text = String::from_utf8_lossy(csv);
    let mut lines = text.lines();
    let Some(header) = lines.next() else {
        return Vec::new();
    };
    let cols: Vec<&str> = header.split(',').map(str::trim).collect();
    let keep: Vec<usize> = cols
        .iter()
        .enumerate()
        .filter(|(_, c)| TRADE_KEEP_COLUMNS.contains(c))
        .map(|(i, _)| i)
        .collect();
    if keep.is_empty() {
        return Vec::new();
    }
    let mut out = String::with_capacity(csv.len());
    let pick = |fields: &[&str], out: &mut String| {
        for (n, &i) in keep.iter().enumerate() {
            if n > 0 {
                out.push(',');
            }
            out.push_str(fields.get(i).copied().unwrap_or(""));
        }
        out.push('\n');
    };
    pick(&cols, &mut out);
    for l in lines {
        if l.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = l.split(',').collect();
        pick(&f, &mut out);
    }
    out.into_bytes()
}

/// Paces and retries for the trade history (RP4: "ölçülü": limited retries).
const TRADE_RETRIES: [Duration; 2] = [Duration::from_secs(5), Duration::from_secs(20)];

impl RestClient<'_> {
    /// (RP4/T1) Historical trades of `symbol` over local dates `start..=end`
    /// (`YYYY-MM-DD`, at most 15 days, `GET /dumrul/v1/tick/trade.gz`) as a level-1 CSV
    /// ([`trade_csv_level1`]). Retries at most twice on a server error or a transport
    /// failure (5 s, 20 s), never on HTTP 4xx.
    ///
    /// # Errors
    /// Token, HTTP or decode failure after the retries (masked text).
    pub fn trades_csv(&self, symbol: &str, start: &str, end: &str) -> Result<Vec<u8>, RestError> {
        self.trades_csv_at("/dumrul/v1/tick/trade.gz", symbol, start, end)
    }

    /// (RP4/M27) Special (reported) trades, `GET /dumrul/v1/tick/trade_ex` (at most 16
    /// days), as a level-1 CSV like [`RestClient::trades_csv`].
    ///
    /// # Errors
    /// Token, HTTP or decode failure after the retries (masked text).
    pub fn trades_ex_csv(
        &self,
        symbol: &str,
        start: &str,
        end: &str,
    ) -> Result<Vec<u8>, RestError> {
        self.trades_csv_at("/dumrul/v1/tick/trade_ex", symbol, start, end)
    }

    fn trades_csv_at(
        &self,
        path: &str,
        symbol: &str,
        start: &str,
        end: &str,
    ) -> Result<Vec<u8>, RestError> {
        let q = [
            ("symbol", symbol.to_owned()),
            ("start", start.to_owned()),
            ("end", end.to_owned()),
        ];
        let mut attempt = 0;
        loop {
            match self.get(path, &q) {
                Ok(body) => return Ok(trade_csv_level1(&body)),
                Err(e) => {
                    let client_error = e.0.contains("HTTP 4");
                    if client_error || attempt >= TRADE_RETRIES.len() {
                        return Err(e);
                    }
                    std::thread::sleep(TRADE_RETRIES[attempt]);
                    attempt += 1;
                }
            }
        }
    }
}

#[cfg(test)]
mod trade_tests {
    use super::*;

    #[test]
    fn level2_columns_never_survive() {
        let csv = b"symbol,date_time,timestamp_ns,bidask,mx_id,price,quantity,buyer,seller\n\
                    AAA,2026-11-20 10:00:00,36000000000000,B,1,10.5,100,X1,Y2\n";
        let out = String::from_utf8(trade_csv_level1(csv)).unwrap();
        assert!(!out.contains("buyer") && !out.contains("seller"));
        assert!(!out.contains("X1") && !out.contains("Y2"));
        assert_eq!(
            out,
            "symbol,date_time,timestamp_ns,bidask,mx_id,price,quantity\n\
             AAA,2026-11-20 10:00:00,36000000000000,B,1,10.5,100\n"
        );
    }

    #[test]
    fn unknown_header_keeps_nothing() {
        assert!(trade_csv_level1(b"a,b\n1,2\n").is_empty());
        assert!(trade_csv_level1(b"").is_empty());
    }
}
