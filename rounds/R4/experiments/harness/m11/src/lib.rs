//! R4/H-M11 — measurement harness for M11: "gerçekleşti" (realized) vs "tutmadı" (failed)
//! on the glyph's outcome mark.
//!
//! Binding specification: `rounds/R4/experiments/harness/SPEC.md` §0, §1, §5, §7. Written
//! **before any alternative existed**. Alternatives do not change this crate: they add an
//! [`qp_grammar::glyph::outcome::OutcomeStyle`] and one line to
//! `qp_grammar::glyph::outcome::STYLES`; the harness measures every registered name.
//!
//! - [`alphabets`]: the two alphabets every style must work with (`siluet`, `rozet`);
//! - [`render`]: one glyph through `OutcomeStyle::draw` and the project CPU rasterizer;
//! - [`color`], [`distinct`]: SPEC §5(a), the R3/M1 distinctness method (copied from
//!   `rounds/R3/experiments/harness/`), on the realized ↔ failed pairs only;
//! - [`sheets`]: SPEC §5(b) persona sheets, hidden keys and prompts (blinded codes);
//! - [`blind`]: codes and the forbidden-word check of prompts (SPEC §0.3);
//! - [`score`]: reader scoring and the M11 selection rule.
//!
//! Deterministic: seeded (SplitMix64), no clock in any record; `score` reads raw files only.

pub mod alphabets;
pub mod blind;
pub mod color;
pub mod distinct;
pub mod io;
pub mod render;
pub mod rng;
pub mod score;
pub mod sheets;

/// Harness version written into every record.
pub const HARNESS_VERSION: &str = "r4-h-m11/1";

/// The reference outcome style (R3's check / cross), SPEC §5.
pub const REFERENCE: &str = "r3";
