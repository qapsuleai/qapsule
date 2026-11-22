//! [`ComputeError`] (out of `lib.rs`).

use std::fmt;

/// Any study construction or input error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComputeError(String);

impl ComputeError {
    pub(crate) fn new(msg: impl Into<String>) -> Self {
        Self(msg.into())
    }
}

impl fmt::Display for ComputeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ComputeError {}
