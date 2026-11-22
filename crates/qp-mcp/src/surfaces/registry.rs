//! The surface registry: every surface by name, one line each in [`registry`].

use super::{mixed, query, task, thin, ToolSurface};

/// Builds a surface.
pub type Factory = fn() -> Box<dyn ToolSurface>;

/// The surfaces, by name (the first is the default). Add a surface with one line here.
#[must_use]
pub fn registry() -> &'static [(&'static str, Factory)] {
    &[
        ("thin", thin::new),
        ("task", task::new),
        ("query", query::new),
        ("mixed", mixed::new),
    ]
}

/// The surface named `name`.
#[must_use]
pub fn by_name(name: &str) -> Option<Box<dyn ToolSurface>> {
    registry()
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, f)| f())
}

/// The registered names, in order.
#[must_use]
pub fn names() -> Vec<&'static str> {
    registry().iter().map(|(n, _)| *n).collect()
}
