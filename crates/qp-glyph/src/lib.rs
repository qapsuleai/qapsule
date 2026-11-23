//! Meaning glyphs (R3): the alphabet contract ([`Alphabet`], [`GlyphBox`]), the alphabets
//! ([`siluet`], [`rozet`]) and the outcome mark styles ([`outcome`]). Split out of
//! `qp-grammar` in RF1 (`rounds/RF1/PLAN.md` §3.1).

mod contract;
pub mod outcome;
pub mod rozet;
pub mod siluet;

pub use contract::{alphabet, alphabet_named, Alphabet, GlyphBox, ALPHABETS};
