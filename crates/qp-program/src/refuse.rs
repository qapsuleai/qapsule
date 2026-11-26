//! Program errors as protocol errors (PS §3.1 "Hatalar hepsi birden"): the top-level error
//! carries the first error's code, reason, path, expected, close names and hint; `errors`
//! carries every one.

use qp_lang::Reason;
use qp_plane::error::E;
use qp_schema::protocol::{ErrorCode, ProgramError, ProtocolError};

/// A program error with the message path it points at.
#[derive(Debug, Clone)]
pub(crate) struct Located {
    /// The error (its `at` relative to the IR).
    pub(crate) err: ProgramError,
    /// JSON pointer into the message.
    pub(crate) path: String,
}

/// The checker's errors, located under `ir_path` (the message's `program` field); one
/// without a place in the IR (a bound value, the budget) at `none_path`.
pub(crate) fn under(errors: &[ProgramError], ir_path: &str, none_path: &str) -> Vec<Located> {
    errors
        .iter()
        .map(|e| Located {
            err: e.clone(),
            path: match &e.at {
                Some(at) => format!("{ir_path}{at}"),
                None => none_path.to_owned(),
            },
        })
        .collect()
}

/// A session-level error of `reason` at message path `path`.
pub(crate) fn session(reason: Reason, path: &str, message: String) -> Located {
    Located {
        err: reason.error(None, message),
        path: path.to_owned(),
    }
}

/// The protocol error of `all` (non-empty; an empty list is an internal `invalid`).
pub(crate) fn refuse(all: Vec<Located>) -> ProtocolError {
    let Some(first) = all.first() else {
        return E::new(ErrorCode::Invalid, "program refused").build();
    };
    let f = &first.err;
    let mut e = E::new(f.code.unwrap_or(ErrorCode::Invalid), f.message.clone())
        .at(first.path.clone())
        .reason(f.reason.clone());
    if !f.expected.is_empty() {
        e = e.expected(f.expected.clone());
    }
    if let Some(h) = &f.hint {
        e = e.hint(h.clone());
    }
    let mut out = e.build();
    out.did_you_mean.clone_from(&f.did_you_mean);
    out.errors = all.into_iter().map(|l| l.err).collect();
    out
}

/// One session-level refusal.
pub(crate) fn one(reason: Reason, path: &str, message: String) -> ProtocolError {
    refuse(vec![session(reason, path, message)])
}
