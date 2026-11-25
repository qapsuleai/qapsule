//! The entry point of text → IR.

use qp_schema::program::ProgramDoc;
use qp_schema::protocol::{Language, ProgramError, Span};
use serde_json::Value;

use crate::diag::{render_all, Diag, Reason};
use crate::limits::TextLimits;
use crate::map::SourceMap;
use crate::msg::Msg;

/// A lowered text file.
#[derive(Debug, Clone, PartialEq)]
pub struct Lowered {
    /// The raw IR: a program object, or `{"reject": {"code", "reason"}}`. Fields the IR
    /// types would refuse (a fractional `period`, a missing `side`) are kept as written
    /// for the checker to answer with its codes.
    pub ir: Value,
    /// IR JSON pointer → text span.
    pub map: SourceMap,
}

impl Lowered {
    /// The IR as typed document.
    ///
    /// # Errors
    /// When the raw IR does not fit the types (the checker reports why, with codes).
    pub fn doc(&self) -> Result<ProgramDoc, serde_json::Error> {
        serde_json::from_value(self.ir.clone())
    }

    /// Whether the file is a refusal.
    #[must_use]
    pub fn is_reject(&self) -> bool {
        self.ir.get("reject").is_some()
    }
}

/// Lowers program text (PR §12.3). Messages are in `lang`.
///
/// # Errors
/// Every text error (syntax, unknown name or function, a non-literal in a number field,
/// a duplicate text name, a misplaced `reject`), in position order, each with `span`.
pub fn lower(src: &str, lang: Language) -> Result<Lowered, Vec<ProgramError>> {
    lower_with(src, lang, &TextLimits::default())
}

/// [`lower`] with explicit size limits.
///
/// # Errors
/// As [`lower`]; a source over `max_source_chars` is one `E_TOO_LARGE` error and is not
/// read further.
pub fn lower_with(
    src: &str,
    lang: Language,
    limits: &TextLimits,
) -> Result<Lowered, Vec<ProgramError>> {
    if src.chars().nth(limits.max_source_chars).is_some() {
        let span = Span {
            line: 1,
            col: 1,
            len: 1,
        };
        let d = Diag::new(
            Reason::TooLarge,
            Msg::SourceTooLong(limits.max_source_chars),
            span,
        );
        return Err(render_all(vec![d], lang));
    }
    let tokens = crate::lex::lex(src, limits.max_token_chars);
    let parsed = crate::parse::parse(&tokens, limits.max_depth);
    let (ir, map, diags) = crate::lower::lower(parsed);
    if diags.is_empty() {
        Ok(Lowered { ir, map })
    } else {
        Err(render_all(diags, lang))
    }
}
