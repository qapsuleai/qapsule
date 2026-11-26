//! (RL1, `docs/PROGRAMS-SESSION.md`) The program subsystem (`qp-program`) through the
//! composition: the program ops on the command path, policy and the live budget, the
//! guards of the `Check` phase, the program queries, log replay and the legend.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod accept;
mod agent;
mod audit;
mod legend;
mod long;
mod ops;
mod queries;
mod rig;
