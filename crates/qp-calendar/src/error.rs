//! [`CalendarError`] (out of `lib.rs`).

use std::fmt;

/// Any calendar error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarError(String);

impl CalendarError {
    pub(crate) fn new(msg: impl Into<String>) -> Self {
        Self(msg.into())
    }
}

impl fmt::Display for CalendarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for CalendarError {}
