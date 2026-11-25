//! Text errors (PR §11.1, §12.4): a sub-code, a message in two languages, a span, and
//! `expected` / `did_you_mean` lists. Rendered into the protocol's `ProgramError`.

use qp_schema::protocol::{ErrorCode, Language, ProgramError, Span};

use crate::msg::Msg;

/// A sub-code the text layer gives (PR §11.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Reason {
    Parse,
    UnknownOp,
    Type,
    MissingArg,
    UnknownRef,
    UnknownParam,
    DuplicateName,
    Unsupported,
    TooLarge,
}

impl Reason {
    /// The sub-code string.
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Reason::Parse => "E_PARSE",
            Reason::UnknownOp => "E_UNKNOWN_OP",
            Reason::Type => "E_TYPE",
            Reason::MissingArg => "E_MISSING_ARG",
            Reason::UnknownRef => "E_UNKNOWN_REF",
            Reason::UnknownParam => "E_UNKNOWN_PARAM",
            Reason::DuplicateName => "E_DUPLICATE_NAME",
            Reason::Unsupported => "E_UNSUPPORTED",
            Reason::TooLarge => "E_TOO_LARGE",
        }
    }

    /// The closed protocol code of the sub-code (PR §11.2 table).
    pub(crate) const fn code(self) -> ErrorCode {
        match self {
            Reason::Parse | Reason::UnknownOp | Reason::Type | Reason::MissingArg => {
                ErrorCode::Schema
            }
            Reason::UnknownRef | Reason::UnknownParam => ErrorCode::NotFound,
            Reason::DuplicateName => ErrorCode::Duplicate,
            Reason::Unsupported => ErrorCode::Unsupported,
            Reason::TooLarge => ErrorCode::Limit,
        }
    }
}

/// One text error before rendering.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Diag {
    pub reason: Reason,
    pub msg: Msg,
    pub span: Span,
    pub expected: Vec<String>,
    pub did_you_mean: Vec<String>,
    pub hint: Option<Msg>,
}

impl Diag {
    /// An error with no lists and no hint.
    pub(crate) fn new(reason: Reason, msg: Msg, span: Span) -> Diag {
        Diag {
            reason,
            msg,
            span,
            expected: Vec::new(),
            did_you_mean: Vec::new(),
            hint: None,
        }
    }

    /// With `expected`.
    pub(crate) fn expected<S: AsRef<str>>(mut self, list: &[S]) -> Diag {
        self.expected = list.iter().map(|s| s.as_ref().to_owned()).collect();
        self
    }

    /// With `did_you_mean`.
    pub(crate) fn suggest(mut self, list: Vec<String>) -> Diag {
        self.did_you_mean = list;
        self
    }

    /// With a hint.
    pub(crate) fn hint(mut self, hint: Msg) -> Diag {
        self.hint = Some(hint);
        self
    }

    /// The protocol error in `lang`.
    pub(crate) fn render(&self, lang: Language) -> ProgramError {
        ProgramError {
            code: Some(self.reason.code()),
            reason: self.reason.name().to_owned(),
            message: self.msg.text(lang),
            at: None,
            span: Some(self.span),
            expected: self.expected.clone(),
            did_you_mean: self.did_you_mean.clone(),
            hint: self.hint.as_ref().map(|h| h.text(lang)),
        }
    }
}

/// Renders errors in position order, then sub-code (PR §11.1).
pub(crate) fn render_all(mut diags: Vec<Diag>, lang: Language) -> Vec<ProgramError> {
    diags.sort_by(|a, b| {
        (a.span.line, a.span.col, a.reason.name()).cmp(&(b.span.line, b.span.col, b.reason.name()))
    });
    diags.iter().map(|d| d.render(lang)).collect()
}
