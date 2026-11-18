//! M17 alternative `selvi` ("konuşur gibi tek cümle"): every surface comes from the product
//! module `qp_capsule::voice::selvi::speak` over the situation's sentence input and the
//! face's building blocks (`qp_capsule::realize::parts::parts`, as the authority fills
//! them). The adapter writes no text of its own; it only maps the product's surfaces.

use qp_capsule::realize::parts::parts;
use qp_capsule::voice::selvi::{speak, VoiceInput};

use crate::lang::{GlossEntry, Lang, Language, Surfaces};
use crate::situ::Situation;

/// The `selvi` voice.
pub struct Selvi;

impl Language for Selvi {
    fn name(&self) -> &'static str {
        "selvi"
    }

    fn surfaces(&self, s: &Situation, lang: Lang) -> Surfaces {
        let cal = s.exchange_calendar();
        let p = parts(&s.story, s.now_t, s.validity, &cal, s.period(), s.redacted);
        let input = s.input();
        let v = speak(&VoiceInput {
            sentence: &input,
            parts: &p,
            calendar: &cal,
            period: s.period(),
        });
        let o = v.get(lang.capsule());
        Surfaces {
            headline: o.headline.clone(),
            detail: o.detail.clone(),
            ledger: o.ledger.clone(),
            glossary: o
                .glossary
                .iter()
                .map(|g| GlossEntry {
                    term: g.term.clone(),
                    text: g.text.clone(),
                })
                .collect(),
        }
    }
}
