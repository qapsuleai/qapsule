//! Error and halt types. Every text field is already masked when constructed (fields are
//! private; constructors take the [`Redactor`]), so `Display` and `Debug` never leak.

use std::fmt;

use crate::config::ConfigError;
use crate::lock::{EndpointKey, LockError};
use crate::redact::Redactor;

/// Masked free text from a lower layer (I/O, TLS, HTTP library).
#[derive(Clone, PartialEq, Eq)]
pub struct Masked(String);

impl Masked {
    pub fn new(redactor: &Redactor, raw: &str) -> Self {
        let mut s = redactor.scrub(raw);
        if s.len() > 300 {
            let mut cut = 300;
            while !s.is_char_boundary(cut) {
                cut -= 1;
            }
            s.truncate(cut);
            s.push('…');
        }
        Self(s)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Masked {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Debug for Masked {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

/// Why a connection stopped for good. After a halt there is no automatic retry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HaltReason {
    /// The server said another session with the same identity connected to this endpoint.
    /// The endpoint stays closed for the life of the process (and, through the marker file,
    /// for later processes) until a person resets it.
    MultiConnect,
    /// Consecutive connection failures reached the limit.
    TooManyFailures { failures: u32 },
    /// CONNACK refused credentials (code 4/5) again after one token refresh.
    AuthRejected { code: u8 },
    /// The token service failed repeatedly; token requests stopped.
    AuthUnavailable,
    /// CONNACK refused the protocol version (1) or client identifier (2): configuration fault.
    ProtocolRejected { code: u8 },
}

impl fmt::Display for HaltReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HaltReason::MultiConnect => f.write_str(
                "multi-connect notice from the server (another session with the same identity); \
                 not retrying; needs a person to reset",
            ),
            HaltReason::TooManyFailures { failures } => {
                write!(f, "{failures} consecutive connection failures; stopped")
            }
            HaltReason::AuthRejected { code } => write!(
                f,
                "server refused the credentials (CONNACK {code}) after one token refresh; stopped"
            ),
            HaltReason::AuthUnavailable => {
                f.write_str("token service failed repeatedly; token requests stopped")
            }
            HaltReason::ProtocolRejected { code } => write!(
                f,
                "server refused protocol or client id (CONNACK {code}); check configuration"
            ),
        }
    }
}

/// Token service error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AuthError {
    /// Requests are rate limited; try again after `wait`.
    RateLimited { wait: std::time::Duration },
    /// Too many consecutive failures; no further requests until reset.
    Halted { failures: u32 },
    /// Non-200 HTTP status (body is never logged).
    Http { status: u16 },
    /// Non-200 HTTP status whose JSON body carried a `code` (sanitized like
    /// [`AuthError::Refused`]); nothing else of the body is kept.
    HttpCode { status: u16, code: String },
    /// `success: false`; `code` sanitized to `[A-Za-z0-9_-]{0,32}`.
    Refused { code: String },
    /// Response shape not understood (no body text included).
    Malformed(&'static str),
    /// Network / TLS failure, masked.
    Transport(Masked),
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthError::RateLimited { wait } => {
                write!(f, "token request rate limited; next allowed in {wait:?}")
            }
            AuthError::Halted { failures } => write!(
                f,
                "token requests stopped after {failures} consecutive failures"
            ),
            AuthError::Http { status } => write!(f, "token service answered HTTP {status}"),
            AuthError::HttpCode { status, code } => {
                write!(f, "token service answered HTTP {status} (code {code:?})")
            }
            AuthError::Refused { code } => write!(f, "token service refused (code {code:?})"),
            AuthError::Malformed(w) => write!(f, "token service response not understood: {w}"),
            AuthError::Transport(m) => write!(f, "token service unreachable: {m}"),
        }
    }
}

impl std::error::Error for AuthError {}

/// Top-level error of this crate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LiveError {
    Config(ConfigError),
    Lock(LockError),
    Auth(AuthError),
    /// The endpoint halted (now or earlier in this process / marker file present).
    Halted {
        key: EndpointKey,
        reason: HaltReason,
    },
    Io(Masked),
}

impl fmt::Display for LiveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LiveError::Config(e) => write!(f, "configuration: {e}"),
            LiveError::Lock(e) => write!(f, "lock: {e}"),
            LiveError::Auth(e) => write!(f, "auth: {e}"),
            LiveError::Halted { key, reason } => write!(f, "endpoint {key} halted: {reason}"),
            LiveError::Io(m) => write!(f, "i/o: {m}"),
        }
    }
}

impl std::error::Error for LiveError {}

impl From<ConfigError> for LiveError {
    fn from(e: ConfigError) -> Self {
        LiveError::Config(e)
    }
}

impl From<LockError> for LiveError {
    fn from(e: LockError) -> Self {
        LiveError::Lock(e)
    }
}

impl From<AuthError> for LiveError {
    fn from(e: AuthError) -> Self {
        LiveError::Auth(e)
    }
}
