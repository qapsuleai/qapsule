//! Qapsule looks (R5/T3): the design records ([`look`]), the themes ([`theme`]), the
//! grammar error type ([`error`]) and the deterministic math ([`detmath`]) that the
//! grammar compiler, the glyphs and the designs share. Split out of `qp-grammar` in RF1
//! (`rounds/RF1/PLAN.md` §3.1).

#[rustfmt::skip]
pub mod advances;
pub mod detmath;
pub mod error;
pub mod look;
pub mod theme;

pub use error::{ErrorKind, GrammarError};
pub use theme::Theme;
