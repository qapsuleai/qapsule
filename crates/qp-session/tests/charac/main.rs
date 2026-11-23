//! RF1/S1 characterisation tests: today's behaviour of `qp-session`, locked finely before
//! the session is split (S4: `expand_op` one function per arm; S2: ops, apply, values and
//! the data plane to new crates; S3: the subsystem registry and phases, with watch, read,
//! viewer and replica moved out). Written against the stable public surface only
//! (`Session::new`/`handle_raw`/`data_event`, the log and documents, `replay`,
//! `ReplayHost`, `Replica`), so they move with the code.
//!
//! Expectations live in `tests/charac/expected/<book>.txt`: one line per case, a SHA-256
//! digest prefix of the output and a readable summary (see `snap.rs`).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::cognitive_complexity
)]

mod agent;
mod opcase;
mod ops_doc;
mod ops_flow;
mod order;
mod rig;
mod snap;
mod transcripts;
mod worlds;
