//! Building protocol errors (`docs/PROTOCOL.md` §13).

use qp_schema::protocol::{ErrorCode, ProtocolError, Seq};

use crate::suggest::did_you_mean;

/// Shorthand for results whose error is a protocol error.
pub type PResult<T> = Result<T, ProtocolError>;

/// Builder for [`ProtocolError`].
pub struct E(ProtocolError);

impl E {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self(ProtocolError {
            candidates: Vec::new(),
            code,
            message: message.into(),
            path: None,
            expected: Vec::new(),
            did_you_mean: Vec::new(),
            conflicts: Vec::new(),
            hint: None,
            reason: None,
            reasons: Vec::new(),
        })
    }

    pub fn at(mut self, path: impl Into<String>) -> Self {
        let p = path.into();
        self.0.path = Some(p);
        self
    }

    pub fn expected<I, S>(mut self, items: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.0.expected = items.into_iter().map(Into::into).collect();
        self
    }

    pub fn dym<'a, I>(mut self, given: &str, candidates: I) -> Self
    where
        I: IntoIterator<Item = &'a str>,
    {
        self.0.did_you_mean = did_you_mean(given, candidates);
        self
    }

    /// Sets `did_you_mean` to exactly these suggestions.
    pub fn suggest<I, S>(mut self, items: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.0.did_you_mean = items.into_iter().map(Into::into).collect();
        self
    }

    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.0.hint = Some(hint.into());
        self
    }

    /// (R5B, §25) The machine-readable sub-reason.
    pub fn reason(mut self, reason: impl Into<String>) -> Self {
        self.0.reason = Some(reason.into());
        self
    }

    pub fn conflicts(mut self, seqs: Vec<Seq>) -> Self {
        self.0.conflicts = seqs;
        self
    }

    pub fn build(self) -> ProtocolError {
        self.0
    }
}

impl From<E> for ProtocolError {
    fn from(e: E) -> Self {
        e.0
    }
}

pub fn invalid(message: impl Into<String>, path: impl Into<String>) -> ProtocolError {
    E::new(ErrorCode::Invalid, message).at(path).build()
}

/// Appends a JSON pointer segment (escaped per RFC 6901).
pub fn ptr(base: &str, seg: impl std::fmt::Display) -> String {
    let s = seg.to_string().replace('~', "~0").replace('/', "~1");
    format!("{base}/{s}")
}

/// Formats an f64 the way Rust prints it (`70` for 70.0, `0.5` for 0.5).
pub fn num(v: f64) -> String {
    format!("{v}")
}
