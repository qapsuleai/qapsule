//! R3/H — measurement harness for M1 (meaning-glyph alphabet) and M2 (sentence realizer).
//!
//! Binding specification: `rounds/R3/experiments/harness/SPEC.md` ("Ortak ilkeler", "M1",
//! "M2"). Written **before any alternative existed**; the only implementation of each trait
//! inside this crate is `control`, a deliberately trivial self-test, **not a candidate**.
//!
//! Principles (SPEC "Ortak ilkeler"):
//! - deterministic: seeded (SplitMix64), no clock in any raw record; the same alternative
//!   and the same seed give byte-identical raw files; `score` re-reads raw files only;
//! - an alternative implements one trait (`qp_grammar::glyph::Alphabet` or
//!   `qp_capsule::Realizer`) and adds one line to the registry
//!   ([`m1::registry`], [`m2::registry`]); measurements read only its output.
//!
//! The only non-deterministic output is the timing file of M2 (`cost.json`), which is
//! never part of a byte-equality claim.

pub mod io;
pub mod m1;
pub mod m2;
pub mod rng;

/// Harness version written into every raw record.
pub const HARNESS_VERSION: &str = "r3-h-m12/1";
