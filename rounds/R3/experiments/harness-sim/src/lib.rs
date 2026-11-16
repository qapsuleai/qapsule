//! R3 measurement harness for M3 (ranking), M4 (theme identity), M5 (outcome definition)
//! and M6 (server-fact trust path); `rounds/R3/experiments/harness/SPEC.md`.
//!
//! Written before any alternative existed. The strategies inside ([`reference`],
//! [`control`]) are the shared reference and the self-test controls, **not candidates**.
//!
//! Deterministic: seeded integer LCG, no clock in any recorded number; threads only
//! split independent jobs and results are collected in input order.

pub mod common;
pub mod control;
pub mod episodes;
pub mod m3;
pub mod m4;
pub mod m5;
pub mod m6;
pub mod reference;
pub mod registry;
pub mod sim;
