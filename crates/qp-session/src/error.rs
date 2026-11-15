//! Building protocol errors (`docs/PROTOCOL.md` §13).

use qp_schema::protocol::{ErrorCode, ProtocolError, Seq};

use crate::suggest::did_you_mean;

/// Shorthand for results whose error is a protocol error.
pub(crate) type PResult<T> = Result<T, ProtocolError>;

/// Builder for [`ProtocolError`].
pub(crate) struct E(ProtocolError);

impl E {
    pub(crate) fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self(ProtocolError {
            candidates: Vec::new(),
            code,
            message: message.into(),
            path: None,
            expected: Vec::new(),
            did_you_mean: Vec::new(),
            conflicts: Vec::new(),
            hint: None,
        })
    }

    pub(crate) fn at(mut self, path: impl Into<String>) -> Self {
        let p = path.into();
        self.0.path = Some(p);
        self
    }

    pub(crate) fn expected<I, S>(mut self, items: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.0.expected = items.into_iter().map(Into::into).collect();
        self
    }

    pub(crate) fn dym<'a, I>(mut self, given: &str, candidates: I) -> Self
    where
        I: IntoIterator<Item = &'a str>,
    {
        self.0.did_you_mean = did_you_mean(given, candidates);
        self
    }

    /// Sets `did_you_mean` to exactly these suggestions.
    pub(crate) fn suggest<I, S>(mut self, items: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.0.did_you_mean = items.into_iter().map(Into::into).collect();
        self
    }

    pub(crate) fn hint(mut self, hint: impl Into<String>) -> Self {
        self.0.hint = Some(hint.into());
        self
    }

    pub(crate) fn conflicts(mut self, seqs: Vec<Seq>) -> Self {
        self.0.conflicts = seqs;
        self
    }

    pub(crate) fn build(self) -> ProtocolError {
        self.0
    }
}

impl From<E> for ProtocolError {
    fn from(e: E) -> Self {
        e.0
    }
}

pub(crate) fn invalid(message: impl Into<String>, path: impl Into<String>) -> ProtocolError {
    E::new(ErrorCode::Invalid, message).at(path).build()
}

/// Appends a JSON pointer segment (escaped per RFC 6901).
pub(crate) fn ptr(base: &str, seg: impl std::fmt::Display) -> String {
    let s = seg.to_string().replace('~', "~0").replace('/', "~1");
    format!("{base}/{s}")
}

/// Formats an f64 the way Rust prints it (`70` for 70.0, `0.5` for 0.5).
pub(crate) fn num(v: f64) -> String {
    format!("{v}")
}
