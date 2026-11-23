//! Capsule voices (R5/M17, `docs/LANGUAGE.md`): languages that write a capsule's four
//! surfaces — headline, detail, track-record (ledger) line and glossary — from engine
//! data. Each voice is a named module with one entry point; the R3 sentence
//! ([`qp_capsule::realize::Template`]) and the R4 face pieces ([`qp_capsule::realize::parts`]) stay
//! as they are.
//!
//! **Registry (R5/m17-int).** [`VOICES`] lists the names a connection may choose (the
//! `style` gesture's `voice`, `docs/PROTOCOL.md` §4.6.2): `r4` (the default: no voice
//! module; faces keep their own headline templates and the capsule its R3 sentence, so
//! bare connections, transcripts and existing goldens stay byte-identical), then M17's
//! first two, `marla` (the presentation's voice, [`PRESENTED_VOICE`]) and `selvi`. The
//! eliminated candidate `tunca` is not in the product tree; its code is on branch
//! `r5/m17-tunca` (`rounds/R5/experiments/M17/RESULTS.md`).
//!
//! [`speak`] is the one entry point the authority calls: a pure function of the
//! product's own data, `None` for `r4`.

pub mod marla;
pub mod marla_glossary;
pub mod selvi;
pub mod selvi_glossary;

mod registry;
mod surfaces;

pub use registry::{is_voice, names, DEFAULT_VOICE, PRESENTED_VOICE, VOICES};
pub use surfaces::{speak, text_face, unnumber, Gloss, Input, Surfaces};

#[cfg(test)]
mod tests;
