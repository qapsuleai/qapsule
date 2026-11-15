//! Qapsule visual grammar compiler (R2).
//!
//! [`compile`] turns a [`View`](qp_schema::grammar::View) and its data (from a
//! [`DataProvider`]) into a [`SceneDesc`](qp_schema::scene::SceneDesc): the
//! renderer-independent scene the reference renderer draws and semantic reading
//! (`screen`, `hit`) reads. [`depth`] holds the presenter's depth rules.
//!
//! Rules (`CLAUDE.md`, `docs/GRAMMAR.md` §14):
//! - no computation of studies here: values come from the provider (the authority runs
//!   the compute core);
//! - determinism: layout uses only `+ − × ÷`, `floor`, `round`, `min`, `max` and integer
//!   math; the log scale uses [`detmath::log10`]; numbers are formatted with `core::fmt`.
//!   The same view and data give the same scene, bit for bit, on every target.

pub mod compile;
pub mod depth;
pub mod detmath;
mod draw;
pub mod error;
mod frame;
mod layer;
pub mod provider;
pub mod scale;
pub mod theme;

pub use compile::{compile, CompileOptions};
pub use error::{ErrorKind, GrammarError};
pub use provider::{DataProvider, MemProvider, SeriesKind};
pub use theme::Theme;
