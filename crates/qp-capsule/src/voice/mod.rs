//! Capsule voices (R5/M17): languages that write a capsule's four surfaces — headline,
//! detail, track-record (ledger) line and glossary — from engine data. Each voice is a
//! named module with one entry point; the R3 sentence ([`crate::realize::Template`]) and
//! the R4 face pieces ([`crate::realize::parts`]) stay as they are.

pub mod selvi;
pub mod selvi_glossary;
pub mod tunca;
pub mod tunca_glossary;
pub mod marla;
pub mod marla_glossary;
