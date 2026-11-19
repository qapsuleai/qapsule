//! Token acquisition. The token lives in memory only (never written to disk, never shared
//! between processes). Its lifetime comes from the JWT `exp` claim (read without signature
//! check, only for timing), else `result.expiresIn`, else a default.
//!
//! Requests are rate limited: at least `min_interval` between two requests; after a failure
//! the gap doubles per consecutive failure (capped); after `max_failures` consecutive
//! failures no further request is made until [`AuthClient::reset_after_halt`].
//! Response bodies are never put into any text.

use std::fmt;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine as _;

use crate::config::LiveConfig;
use crate::error::{AuthError, Masked};
use crate::redact::{Redactor, Secret};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthPolicy {
    /// Fetch a new token when less than this is left before expiry.
    pub refresh_before: Duration,
    /// Minimum gap between two token requests.
    pub min_interval: Duration,
    /// Upper bound of the gap after failures.
    pub max_interval: Duration,
    /// Consecutive failures after which requests stop.
    pub max_failures: u32,
    /// Lifetime assumed when neither `exp` nor `expiresIn` is present.
    pub default_lifetime: Duration,
    /// Whole-request timeout.
    pub http_timeout: Duration,
}

impl Default for AuthPolicy {
    fn default() -> Self {
        Self {
            refresh_before: Duration::from_secs(300),
            min_interval: Duration::from_secs(30),
            max_interval: Duration::from_secs(15 * 60),
            max_failures: 5,
            default_lifetime: Duration::from_secs(3600),
            http_timeout: Duration::from_secs(10),
        }
    }
}

/// An access token. `Debug` hides the value.
#[derive(Clone)]
pub struct AccessToken {
    value: Secret,
    expires_at: SystemTime,
}

impl fmt::Debug for AccessToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AccessToken")
            .field("value", &self.value)
            .field("expires_in", &self.remaining())
            .finish()
    }
}

impl AccessToken {
    pub fn expires_at(&self) -> SystemTime {
        self.expires_at
    }

    /// Time left until expiry (zero if expired).
    pub fn remaining(&self) -> Duration {
        self.expires_at
            .duration_since(SystemTime::now())
            .unwrap_or(Duration::ZERO)
    }

    pub fn is_expired(&self) -> bool {
        self.remaining() == Duration::ZERO
    }

    /// True if both hold the same token value.
    pub fn same_value(&self, other: &AccessToken) -> bool {
        self.value == other.value
    }

    pub(crate) fn expose(&self) -> &str {
        self.value.expose()
    }
}

struct State {
    token: Option<AccessToken>,
    last_request: Option<Instant>,
    failures: u32,
    requests: u64,
}

/// Token client for one identity. Share it (`Arc`) between the connections of one process;
/// it is never shared between processes.
pub struct AuthClient {
    url: Secret,
    basic: Secret,
    type_code: String,
    hardware_id: String,
    policy: AuthPolicy,
    redactor: Redactor,
    agent: ureq::Agent,
    state: Mutex<State>,
}

impl fmt::Debug for AuthClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AuthClient")
            .field("policy", &self.policy)
            .field("requests", &self.request_count())
            .finish_non_exhaustive()
    }
}

impl AuthClient {
    pub fn new(cfg: &LiveConfig, policy: AuthPolicy) -> Self {
        let raw = format!("{}:{}", cfg.username().expose(), cfg.password().expose());
        let encoded = STANDARD.encode(raw.as_bytes());
        let redactor = cfg.redactor().clone();
        redactor.add_secret(&encoded);
        let config = ureq::Agent::config_builder()
            .timeout_global(Some(policy.http_timeout))
            .http_status_as_error(false)
            // Never follow a redirect with the Basic credentials, never use a proxy.
            .max_redirects(0)
            .proxy(None)
            .build();
        Self {
            url: cfg.token_service_url().clone(),
            basic: Secret::new(format!("Basic {encoded}")),
            type_code: cfg.type_code().to_string(),
            hardware_id: cfg.hardware_id().to_string(),
            policy,
            redactor,
            agent: ureq::Agent::new_with_config(config),
            state: Mutex::new(State {
                token: None,
                last_request: None,
                failures: 0,
                requests: 0,
            }),
        }
    }

    pub fn policy(&self) -> &AuthPolicy {
        &self.policy
    }

    /// Number of token requests sent so far.
    pub fn request_count(&self) -> u64 {
        self.lock().requests
    }

    /// Consecutive failures so far.
    pub fn failures(&self) -> u32 {
        self.lock().failures
    }

    /// The cached token, if any (may be near expiry).
    pub fn cached(&self) -> Option<AccessToken> {
        self.lock().token.clone()
    }

    /// True when the cached token is missing or within `refresh_before` of expiry.
    pub fn needs_refresh(&self) -> bool {
        let st = self.lock();
        self.stale(st.token.as_ref())
    }

    /// A token valid for at least `refresh_before`, fetching one only if needed. If a fetch
    /// is needed but not allowed yet (rate limit) or fails, a not-yet-expired cached token
    /// is returned instead.
    pub fn token(&self) -> Result<AccessToken, AuthError> {
        let mut st = self.lock();
        if !self.stale(st.token.as_ref()) {
            if let Some(t) = &st.token {
                return Ok(t.clone());
            }
        }
        match self.fetch(&mut st) {
            Ok(t) => Ok(t),
            Err(e) => match &st.token {
                Some(t) if !t.is_expired() && !matches!(e, AuthError::Halted { .. }) => {
                    Ok(t.clone())
                }
                _ => Err(e),
            },
        }
    }

    /// Discards the cached token and fetches a new one (after the server refused it).
    /// Still rate limited.
    pub fn refresh(&self) -> Result<AccessToken, AuthError> {
        let mut st = self.lock();
        let r = self.fetch(&mut st);
        if r.is_err() {
            st.token = None;
        }
        r
    }

    /// Explicit, human-initiated reset after the token service halted.
    pub fn reset_after_halt(&self) {
        let mut st = self.lock();
        st.failures = 0;
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn stale(&self, t: Option<&AccessToken>) -> bool {
        match t {
            None => true,
            Some(t) => t.remaining() <= self.policy.refresh_before,
        }
    }

    fn required_gap(&self, failures: u32) -> Duration {
        let mut gap = self.policy.min_interval;
        for _ in 0..failures {
            gap = gap.saturating_mul(2);
            if gap >= self.policy.max_interval {
                return self.policy.max_interval;
            }
        }
        gap
    }

    fn fetch(&self, st: &mut State) -> Result<AccessToken, AuthError> {
        if st.failures >= self.policy.max_failures {
            return Err(AuthError::Halted {
                failures: st.failures,
            });
        }
        if let Some(last) = st.last_request {
            let gap = self.required_gap(st.failures);
            let elapsed = last.elapsed();
            if elapsed < gap {
                return Err(AuthError::RateLimited {
                    wait: gap - elapsed,
                });
            }
        }
        st.last_request = Some(Instant::now());
        st.requests += 1;
        match self.request() {
            Ok(t) => {
                st.failures = 0;
                self.redactor.add_secret(t.expose());
                st.token = Some(t.clone());
                Ok(t)
            }
            Err(e) => {
                st.failures += 1;
                Err(e)
            }
        }
    }

    fn request(&self) -> Result<AccessToken, AuthError> {
        let body = serde_json::json!({
            "typeCode": self.type_code,
            "hardwareId": self.hardware_id,
        })
        .to_string();
        let mut resp = self
            .agent
            .post(self.url.expose())
            .header("Content-Type", "application/json")
            .header("Authorization", self.basic.expose())
            .send(body.as_bytes())
            .map_err(|e| AuthError::Transport(Masked::new(&self.redactor, &e.to_string())))?;
        let status = resp.status().as_u16();
        if status != 200 {
            return Err(AuthError::Http { status });
        }
        let text = resp
            .body_mut()
            .with_config()
            .limit(256 * 1024)
            .read_to_string()
            .map_err(|_| AuthError::Malformed("body could not be read"))?;
        let v: serde_json::Value =
            serde_json::from_str(&text).map_err(|_| AuthError::Malformed("not JSON"))?;
        if v.get("success").and_then(|s| s.as_bool()) != Some(true) {
            let code = v
                .get("code")
                .map(|c| match c {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                })
                .unwrap_or_default();
            return Err(AuthError::Refused {
                code: sanitize_code(&code),
            });
        }
        let result = v.get("result").ok_or(AuthError::Malformed("no result"))?;
        let token = result
            .get("token")
            .and_then(|t| t.as_str())
            .filter(|t| !t.is_empty())
            .ok_or(AuthError::Malformed("no token"))?;
        let now = SystemTime::now();
        let expires_at = match jwt_expiry(token) {
            Some(exp) => UNIX_EPOCH + Duration::from_secs(exp),
            None => match result.get("expiresIn").and_then(|e| e.as_u64()) {
                Some(secs) => now + Duration::from_secs(secs),
                None => now + self.policy.default_lifetime,
            },
        };
        Ok(AccessToken {
            value: Secret::new(token),
            expires_at,
        })
    }
}

fn sanitize_code(code: &str) -> String {
    code.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .take(32)
        .collect()
}

/// `exp` claim (unix seconds) of a JWT, without verifying the signature.
pub fn jwt_expiry(token: &str) -> Option<u64> {
    let mut parts = token.split('.');
    let _header = parts.next()?;
    let payload = parts.next()?;
    let bytes = URL_SAFE_NO_PAD
        .decode(payload.trim_end_matches('=').as_bytes())
        .ok()?;
    let v: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    let exp = v.get("exp")?;
    exp.as_u64()
        .or_else(|| exp.as_f64().map(|f| f.max(0.0) as u64))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_exp_without_verifying() {
        let payload = URL_SAFE_NO_PAD.encode(br#"{"sub":"x","exp":1900000000}"#);
        let t = format!("eyJhbGciOiJub25lIn0.{payload}.sig");
        assert_eq!(jwt_expiry(&t), Some(1_900_000_000));
        assert_eq!(jwt_expiry("not-a-jwt"), None);
        let no_exp = URL_SAFE_NO_PAD.encode(br#"{"sub":"x"}"#);
        assert_eq!(jwt_expiry(&format!("a.{no_exp}.b")), None);
    }

    #[test]
    fn code_is_sanitized() {
        assert_eq!(sanitize_code("E_401 <script>"), "E_401script");
        assert_eq!(sanitize_code(&"x".repeat(100)).len(), 32);
    }
}
