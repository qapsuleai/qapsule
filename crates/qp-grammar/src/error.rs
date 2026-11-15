//! Compiler errors: every error says which layer (if any), where inside it (a JSON
//! pointer) and why.

use std::fmt;

/// Error category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// A referenced series, field, fact, finding, panel, layer or threshold does not exist.
    NotFound,
    /// The view or an option is malformed (wrong mark for the data, bad colour, bad
    /// domain, size out of range...).
    Invalid,
    /// A numeric data constant has no origin, or its origin does not match it.
    NoOrigin,
}

/// A compile error: `layer` (when the error is inside a layer), `path` (JSON pointer,
/// inside the layer when `layer` is set, inside the view otherwise) and `reason`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrammarError {
    /// Category.
    pub kind: ErrorKind,
    /// Layer id, when the error is inside a layer.
    pub layer: Option<String>,
    /// JSON pointer: inside the layer (`/encoding/y`) when `layer` is set, otherwise inside
    /// the view (`/x/domain`) or empty for options.
    pub path: String,
    /// Human-readable reason.
    pub reason: String,
}

impl GrammarError {
    pub(crate) fn new(kind: ErrorKind, path: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            kind,
            layer: None,
            path: path.into(),
            reason: reason.into(),
        }
    }

    pub(crate) fn in_layer(
        kind: ErrorKind,
        layer: &str,
        path: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            layer: Some(layer.to_owned()),
            path: path.into(),
            reason: reason.into(),
        }
    }

    pub(crate) fn with_layer(mut self, layer: &str) -> Self {
        if self.layer.is_none() {
            self.layer = Some(layer.to_owned());
        }
        self
    }
}

impl fmt::Display for GrammarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (&self.layer, self.path.is_empty()) {
            (Some(l), false) => write!(f, "layer `{l}` {}: {}", self.path, self.reason),
            (Some(l), true) => write!(f, "layer `{l}`: {}", self.reason),
            (None, false) => write!(f, "{}: {}", self.path, self.reason),
            (None, true) => f.write_str(&self.reason),
        }
    }
}

impl std::error::Error for GrammarError {}
