//! Qapsule visual grammar compiler (R2).
//!
//! [`compile`] turns a [`View`](qp_schema::grammar::View) and its data (from a
//! [`DataProvider`]) into a [`SceneDesc`](qp_schema::scene::SceneDesc): the
//! renderer-independent scene the reference renderer draws and semantic reading
//! (`screen`, `hit`) reads. [`depth`] holds the presenter's depth rules.
//!
//! (RF1) The named designs (faces, open capsule, story charts, feed layouts, transitions,
//! motion) are in `qp-designs`; the compiler reaches them only through the design
//! registry it is handed ([`Designs`], [`CompileOptions::designs`]).
//!
//! Rules (`CLAUDE.md`, `docs/GRAMMAR.md` §14):
//! - no computation of studies here: values come from the provider (the authority runs
//!   the compute core);
//! - determinism: layout uses only `+ − × ÷`, `floor`, `round`, `min`, `max` and integer
//!   math; the log scale uses `qp_look::detmath::log10`; numbers are formatted with `core::fmt`.
//!   The same view and data give the same scene, bit for bit, on every target.

pub mod compile;
pub mod designs;
mod present;
pub mod provider;

pub use compile::{draw, scale, terms};
pub(crate) use compile::{frame, layer};
pub use present::{depth, gesture};

pub use compile::{compile, CompileOptions, Cursor};
pub use designs::Designs;
pub use provider::{DataProvider, HeadlineParts, MemProvider, SeriesKind};
