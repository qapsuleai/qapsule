//! Lowering (PR §12.3): statements to the raw JSON IR the checker consumes. Binding names
//! become node ids; unnamed nodes of nested expressions get `_1, _2, ...` in post-order
//! over the whole program; equal sub-expressions stay separate nodes.
//!
//! Only what the IR cannot represent is an error here (syntax, unknown names and
//! functions, a non-literal in a number field, duplicate text names); everything the IR
//! can carry (types, clocks, ranges, missing rule fields) is left to the checker, whose
//! `at` pointers the [`SourceMap`] turns back into spans.

mod args;
mod expr;
mod run;
mod state;
mod stmt;

pub(crate) use run::lower;
