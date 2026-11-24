//! Replay host (`yeniden oynatma barındırıcısı`; R4/T2, K44; `docs/PROTOCOL.md` §21): the
//! host that runs an authority on a replay source (RF1 S3.3, `rounds/RF1/PLAN.md` §3.3
//! row 9). A client of the authority: it builds its sessions with the subsystem list it
//! is given (the composition's `qp_session::ReplayHost` gives the standard one) and reaches
//! the authority only through its public (and a few `#[doc(hidden)]`) methods.

// Protocol errors are returned on the cold path only and are built to be read by an
// agent (message, path, expected, suggestions, hint); boxing them buys nothing here
// (the same allowance as `qp-authority`).
#![allow(clippy::result_large_err)]

mod replayhost;

pub use replayhost::*;
