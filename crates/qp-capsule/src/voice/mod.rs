//! Capsule languages (R5/M17): named modules that write a capsule's four surfaces —
//! headline, detail, ledger sentence, glossary — from the product's own data. The
//! protocol default (`r3` sentences, [`crate::realize`]) is unchanged; a language here is
//! chosen by name.

pub mod marla;
pub mod marla_glossary;
