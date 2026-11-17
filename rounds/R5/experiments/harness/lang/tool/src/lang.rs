//! The interface a capsule language implements for the harness (SPEC §1).
//!
//! The harness never reads a language's code: it asks a [`Language`] for the
//! [`Surfaces`] of a [`Situation`] in TR and EN and measures the texts. A language
//! alternative (another branch changes `crates/qp-capsule/` templates) adds **one
//! adapter file** under `src/adapters/` that calls its product functions, and one line in
//! [`registry`]. Scans, tables, reader pages and scoring do not change.

use serde::{Deserialize, Serialize};

use crate::situ::Situation;

/// Language of a text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    /// Turkish.
    Tr,
    /// English.
    En,
}

impl Lang {
    /// Both, in order.
    pub const ALL: [Lang; 2] = [Lang::Tr, Lang::En];

    /// Short name.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Lang::Tr => "tr",
            Lang::En => "en",
        }
    }

    /// The product's language value.
    #[must_use]
    pub fn capsule(self) -> qp_capsule::Lang {
        match self {
            Lang::Tr => qp_capsule::Lang::Tr,
            Lang::En => qp_capsule::Lang::En,
        }
    }
}

/// One glossary entry shown on the screen ("dokununca açılan" term).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlossEntry {
    /// The term as written in the sentence ("RSI", "hareketli ortalama").
    pub term: String,
    /// Its explanation.
    pub text: String,
}

/// Every text a language puts on the capsule's screens for one situation in one language.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Surfaces {
    /// Başlık: the face's headline sentence.
    pub headline: String,
    /// Ayrıntı cümlesi: the open capsule's detail sentence(s).
    pub detail: String,
    /// Karne cümlesi: the track-record sentence (on the face and/or open capsule).
    pub ledger: String,
    /// Sözlük girdileri for the terms on this screen (empty: no glossary).
    pub glossary: Vec<GlossEntry>,
}

/// A capsule language as the harness sees it.
pub trait Language {
    /// Registry name (`[a-z][a-z0-9_-]*`; never shown to a reader).
    fn name(&self) -> &'static str;
    /// The surfaces of `s` in `lang`. Must be a pure function of its inputs.
    fn surfaces(&self, s: &Situation, lang: Lang) -> Surfaces;
}

/// Registered languages: the reference `r4` (today's product language), the harness's
/// own deliberately broken language `bozuk` (SPEC §5; secretly added to every reader run),
/// then the alternatives (one line each).
#[must_use]
pub fn registry() -> Vec<Box<dyn Language>> {
    vec![
        Box::new(crate::adapters::r4::R4),
        Box::new(crate::adapters::bozuk::Bozuk),
        // --- M17 alternatives: one line each (Box::new(crate::adapters::<name>::<Type>)),
        Box::new(crate::adapters::tunca::Tunca),
    ]
}

/// The language named `name`.
#[must_use]
pub fn by_name(name: &str) -> Option<Box<dyn Language>> {
    registry().into_iter().find(|l| l.name() == name)
}
