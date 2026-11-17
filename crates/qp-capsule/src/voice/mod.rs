//! Capsule voices (R5/M17): alternative capsule languages, each a pure function from the
//! product's own data (story, facts, evidence, ledger, headline pieces) to the four
//! surfaces a capsule speaks on — headline, detail, ledger line, glossary.
//!
//! The R3 sentence ([`crate::realize::Template`]) and the R4 face template stay as they
//! are (protocol default, transcripts, golden images); a voice is added next to them.

pub mod tunca;
pub mod tunca_glossary;
