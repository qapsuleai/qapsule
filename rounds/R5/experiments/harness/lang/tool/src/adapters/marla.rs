//! `marla` (R5/M17 alternative): "the order of the reader's five questions, and context
//! for every number" — `crates/qp-capsule/src/voice/marla.rs`.
//!
//! The adapter only calls the product: it builds the face's headline pieces
//! (`qp_capsule::realize::parts::parts`, exactly as the `r4` adapter does) and hands them
//! with the product's `SentenceInput` to `qp_capsule::voice::marla::speak`. Every text
//! (headline, detail, ledger, glossary) is the product's; nothing is written here.

use qp_capsule::realize::parts::parts;
use qp_capsule::voice::marla::speak;

use crate::lang::{GlossEntry, Lang, Language, Surfaces};
use crate::situ::Situation;

/// The `marla` language.
pub struct Marla;

impl Language for Marla {
    fn name(&self) -> &'static str {
        "marla"
    }

    fn surfaces(&self, s: &Situation, lang: Lang) -> Surfaces {
        let cal = s.exchange_calendar();
        let p = parts(&s.story, s.now_t, s.validity, &cal, s.period(), s.redacted);
        let v = speak(&s.input(), &p, lang.capsule());
        Surfaces {
            headline: v.headline,
            detail: v.detail,
            ledger: v.ledger,
            glossary: v
                .glossary
                .into_iter()
                .map(|g| GlossEntry {
                    term: g.term,
                    text: g.text,
                })
                .collect(),
        }
    }
}
