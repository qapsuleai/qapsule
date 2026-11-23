//! Qapsule data plane of the authority (`docs/PROTOCOL.md` §18; RF1 S2, split out of
//! `qp-session`): the plane ([`plane`]), the values cache ([`values`]), protocol error
//! building ([`error`], [`suggest`]) and the pure capsule indices the authority keeps per
//! reader ([`capidx`], [`proj`], [`sentences`], [`headparts`]). Knows no session.

// Protocol errors are returned on the cold path only and are built to be read by an
// agent (message, path, expected, suggestions, hint); boxing them buys nothing here.
#![allow(clippy::result_large_err)]
// (RF1 S2) `values::Computed::len` became `pub` at the crate boundary (the presenter in
// `qp-ops` reads it); it has no `is_empty` and gets none (pure move).
#![allow(clippy::len_without_is_empty)]

pub mod capidx;
pub mod error;
pub mod headparts;
pub mod plane;
pub mod proj;
pub mod sentences;
pub mod suggest;
pub mod values;

pub use plane::{DataError, DataOutcome, HostNotice};
pub use suggest::{did_you_mean, edit_distance};
