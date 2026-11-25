//! Canonical printer (PR §12.3): IR → text, deterministic. Parameters first, then one
//! line per node that is not inlined, in IR order, with each output line placed where its
//! own (inlined) nodes stood, so that text → IR → text → IR is the identity. A node whose
//! id starts with `_` and that is used exactly once is written where it is used; other
//! `_` nodes (and ids the text cannot spell) get the smallest unused `v<k>`.

mod check;
mod expr;
mod lit;
mod plan;
mod text;

pub use check::PrintError;
pub use text::{print, print_doc, print_reject, print_with};
