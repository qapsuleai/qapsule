//! Viewer connections (`görüntüleyici`; RP3/M15, K48, `docs/PROTOCOL.md` §22; RF1 S3.3,
//! `rounds/RF1/PLAN.md` §3.3 row 7): the viewer subsystem of the authority (the `viewer`
//! topic, `visible` messages and the pushed updates: [`VIEWER`]), the device's side of the
//! open capsule ([`OpenSide`], over a replica) and the scene, slice and check functions both
//! sides compile with.
//!
//! The authority knows none of it: it calls the viewer as a pushed topic
//! (`qp_subsystem::TopicHandler`) after every input, in each viewer's reader world; the
//! viewers' state is a per-connection slot. The composition (`qp-session`) registers it in
//! its standard list.

// Protocol errors are returned on the cold path only and are built to be read by an
// agent (message, path, expected, suggestions, hint); boxing them buys nothing here
// (the same allowance as `qp-authority`).
#![allow(clippy::result_large_err)]

#[doc(hidden)]
pub mod tape;
mod viewer;

pub use viewer::*;
