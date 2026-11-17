//! R5/T2 M17 capsule-language harness (`rounds/R5/experiments/harness/lang/SPEC.md`).
//!
//! Situations from the engine over synthetic data ([`situ`]), the answer key from engine
//! data and detector parameters ([`key`]), languages behind one interface ([`lang`],
//! adapters in [`adapters`]), deterministic scans ([`scan`]), the dump and its tables
//! ([`table`]), blind readers and critics ([`readers`], [`stats`]).

pub mod adapters;
pub mod key;
pub mod lang;
pub mod readers;
pub mod scan;
pub mod situ;
pub mod stats;
pub mod table;
pub mod text;
