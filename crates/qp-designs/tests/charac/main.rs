//! RF1/G1 characterisation tests: today's output of `qp-grammar`, locked finely before the
//! grammar is split (G2: look/glyph down, the design table, `qp-designs`; G3: `prepare`
//! in named steps). Written against the public API only, so they move with the code.
//!
//! Expectations live in `tests/charac/expected/<book>.txt`: one line per case, a SHA-256
//! digest prefix of the output and a short readable summary (see `snap.rs`).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::cognitive_complexity
)]

#[path = "../support/mod.rs"]
mod support;

mod designs;
mod fixture;
mod prepare;
mod present;
mod snap;
