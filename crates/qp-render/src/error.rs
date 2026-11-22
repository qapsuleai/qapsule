//! The render core's error type (RG1: moved out of `lib.rs`).

use std::fmt;

/// Any failure while parsing input or rendering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderError(String);

impl RenderError {
    pub(crate) fn new(msg: impl Into<String>) -> Self {
        Self(msg.into())
    }
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for RenderError {}
