//! The text language of Qapsule programs (`docs/PROGRAMS.md` §12): a small, line-based
//! writing surface that lowers to the `qp-ir/0` IR, and the canonical printer back.
//!
//! - [`lower`]: text → raw JSON IR (what the checker consumes) + a [`SourceMap`] that
//!   turns the checker's `at` pointers into spans. Syntax errors come all at once, one
//!   per statement, in position order, with `span`, sub-code and suggestions.
//! - [`print`] / [`print_doc`]: IR → canonical text (deterministic; `_` nodes used once
//!   are inlined). `lower(print(lower(t))) == lower(t)`.
//!
//! Hand-written lexer and parser, no dependency beyond `qp-schema` and `serde_json`. No
//! clock, randomness or I/O. Checking types, clocks, ranges and cost is not here
//! (`qp-lang`).

mod ast;
mod diag;
mod lex;
mod limits;
mod lower;
mod lowered;
mod map;
mod msg;
mod names;
mod parse;
mod print;

pub use limits::{
    TextLimits, DEFAULT_MAX_DEPTH, DEFAULT_MAX_SOURCE_CHARS, DEFAULT_MAX_TOKEN_CHARS,
};
pub use lowered::{lower, lower_with, Lowered};
pub use map::SourceMap;
pub use names::{FUNCTIONS, REJECT_CODES};
pub use print::{print, print_doc, print_reject, print_with, PrintError};
