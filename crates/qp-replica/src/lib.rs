//! The client replica (`istemci yansısı`; RP2/T8, `docs/PROTOCOL.md` §19; RF1 S3.3,
//! `rounds/RF1/PLAN.md` §3.3 row 6): a device-side follower of an authority ([`Replica`]:
//! following the log, its own data, the display rule) and the authority ↔ replica check
//! ([`tape`]).
//!
//! A client of the authority: it holds a follower authority built with the subsystem list
//! it is given (the composition's `FOLLOWER`) and changes it only through the authority's
//! narrow hidden methods (`build_follower`, `follow_start`, `take_provider`, `mirror_apply`,
//! `mirror_commit`, `follow_data`, `follow_resync`, `eval_parts`). The authority knows none
//! of it.

// Protocol errors are returned on the cold path only and are built to be read by an
// agent (message, path, expected, suggestions, hint); boxing them buys nothing here
// (the same allowance as `qp-authority`).
#![allow(clippy::result_large_err)]

mod replica;
#[doc(hidden)]
pub mod tape;

pub use replica::*;
