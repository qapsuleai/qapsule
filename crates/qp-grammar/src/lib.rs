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

#[rustfmt::skip]
pub mod advances;
pub mod compile;
pub mod detmath;
pub mod error;
pub mod faces;
pub mod feeds;
pub mod glyph;
pub mod look;
pub mod open;
mod present;
pub mod provider;
pub mod stories;
pub mod theme;
pub mod transitions;

pub(crate) use compile::{draw, frame, layer};
pub use compile::{scale, terms};
pub use present::{depth, gesture, motion};

pub use compile::{compile, CompileOptions};
pub use error::{ErrorKind, GrammarError};
pub use provider::{DataProvider, HeadlineParts, MemProvider, SeriesKind};
pub use theme::Theme;
