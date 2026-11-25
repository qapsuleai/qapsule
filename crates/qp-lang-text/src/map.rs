//! Source map (PR §12.4): JSON pointers of the lowered IR to text spans, so that the
//! checker's type, clock and cost errors (which carry `at`) land on their source line.

use qp_schema::protocol::{ProgramError, Span};

/// JSON pointer (relative to the program root) → span of the text that produced it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SourceMap {
    entries: Vec<(String, Span)>,
}

impl SourceMap {
    pub(crate) fn add(&mut self, at: String, span: Span) {
        self.entries.push((at, span));
    }

    /// Every entry, in lowering order.
    #[must_use]
    pub fn entries(&self) -> &[(String, Span)] {
        &self.entries
    }

    /// The span of the longest mapped pointer that is `at` itself or a prefix of it at a
    /// `/` boundary (`/nodes/3/period` falls back to `/nodes/3`).
    #[must_use]
    pub fn span_at(&self, at: &str) -> Option<Span> {
        self.entries
            .iter()
            .filter(|(p, _)| {
                at.strip_prefix(p.as_str())
                    .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
            })
            .max_by_key(|(p, _)| p.len())
            .map(|(_, s)| *s)
    }

    /// Fills `span` of every error that has an `at` and no span yet.
    pub fn attach(&self, errors: &mut [ProgramError]) {
        for e in errors.iter_mut().filter(|e| e.span.is_none()) {
            e.span = e.at.as_deref().and_then(|at| self.span_at(at));
        }
    }
}
