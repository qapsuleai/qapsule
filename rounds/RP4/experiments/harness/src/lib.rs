//! RP4 measurement harness for M25 (outcome rule) and M26 (detector thresholds);
//! `rounds/RP4/experiments/harness/SPEC.md` (binding) and `../SPLIT.md`.
//!
//! Written before any alternative existed: the only strategies inside are the
//! reference `r3` (product outcome rule and detector set). Deterministic: seeded integer
//! LCG for the bootstrap, no clock in any recorded number, threads only split independent
//! symbols and results are collected in symbol order.

pub mod boot;
pub mod common;
pub mod data;
pub mod m25;
pub mod m26;
pub mod registry;
pub mod report;
pub mod sim;
pub mod synth;
pub mod tick;
