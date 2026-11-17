//! M17 alternative `tunca`: "a very short headline + scannable layers"
//! (`qp_capsule::voice::tunca`).
//!
//! Calls the product only: the headline pieces (`qp_capsule::realize::parts::parts`, the
//! same call the authority makes) and the voice's single entry point
//! `qp_capsule::voice::tunca::speak`. No text is produced here.

use qp_capsule::realize::parts::parts;
use qp_capsule::voice::tunca::{speak, VoiceInput};

use crate::lang::{GlossEntry, Lang, Language, Surfaces};
use crate::situ::Situation;

/// The `tunca` language.
pub struct Tunca;

impl Language for Tunca {
    fn name(&self) -> &'static str {
        "tunca"
    }

    fn surfaces(&self, s: &Situation, lang: Lang) -> Surfaces {
        let cal = s.exchange_calendar();
        let p = parts(&s.story, s.now_t, s.validity, &cal, s.period(), s.redacted);
        let inp = s.input();
        let v = speak(
            &VoiceInput {
                sentence: &inp,
                parts: &p,
                period: s.period(),
            },
            lang.capsule(),
        );
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
