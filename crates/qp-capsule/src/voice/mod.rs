//! Capsule voices (R5/M17, `docs/LANGUAGE.md`): languages that write a capsule's four
//! surfaces — headline, detail, track-record (ledger) line and glossary — from engine
//! data. Each voice is a named module with one entry point; the R3 sentence
//! ([`crate::realize::Template`]) and the R4 face pieces ([`crate::realize::parts`]) stay
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

use qp_calendar::ExchangeCalendar;

use crate::realize::elapsed::Period;
use crate::realize::parts::Parts;
use crate::{Lang, SentenceInput};

pub mod marla;
pub mod marla_glossary;
pub mod selvi;
pub mod selvi_glossary;

/// Voice names a connection may choose; the first is the default.
pub const VOICES: [&str; 3] = ["r4", "marla", "selvi"];

/// The default voice: today's text (no voice module).
pub const DEFAULT_VOICE: &str = "r4";

/// The voice the presentation shell asks for when the reader chose none (M17's pick,
/// `rounds/R5/experiments/M17/RESULTS.md` §6). Like `qp_grammar::faces::PRESENTED_FACE`
/// it is sent by the shell core with the `style` gesture; the protocol default
/// ([`DEFAULT_VOICE`]) does not change.
pub const PRESENTED_VOICE: &str = "marla";

/// Whether `name` is a known voice.
#[must_use]
pub fn is_voice(name: &str) -> bool {
    VOICES.contains(&name)
}

/// Registered names, in registry order.
#[must_use]
pub fn names() -> Vec<&'static str> {
    VOICES.to_vec()
}

/// One glossary entry of a screen: the term as the screen writes it and its short
/// explanation (no digit: a redacted reader gets it too).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gloss {
    /// The term.
    pub term: String,
    /// Its explanation.
    pub text: String,
}

/// A capsule's four surfaces in one language.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Surfaces {
    /// Başlık: what changed, with the threshold, when and whether it still holds.
    pub headline: String,
    /// Ayrıntı: why, and where things stand now (built from the face's pieces, A33).
    pub detail: String,
    /// Karne: the expectation and how often it played out before.
    pub ledger: String,
    /// Sözlük: the terms on this screen.
    pub glossary: Vec<Gloss>,
}

/// What every voice needs: the product's sentence input, the face's headline pieces
/// (validity, "when" and the ending come only from here, A33), the exchange calendar and
/// the bar period of the story's series.
#[derive(Clone, Copy)]
pub struct Input<'a> {
    /// The story, its lead and latest facts, now, the condition's current value and
    /// whether the reader is redacted.
    pub sentence: &'a SentenceInput<'a>,
    /// The face's headline pieces for the same reader.
    pub parts: &'a Parts,
    /// The exchange calendar of the story's series.
    pub calendar: &'a ExchangeCalendar,
    /// The bar period of the story's series.
    pub period: Period,
}

/// The surfaces of voice `name` in Turkish and English (`[tr, en]`); `None` for `r4`
/// (and an unknown name). Deterministic: no clock, no randomness, no model.
#[must_use]
pub fn speak(name: &str, inp: &Input<'_>) -> Option<[Surfaces; 2]> {
    match name {
        "marla" => Some(marla::speak_both(inp.sentence, inp.parts).map(|v| {
            Surfaces {
                headline: v.headline,
                detail: v.detail,
                ledger: v.ledger,
                glossary: v
                    .glossary
                    .into_iter()
                    .map(|g| Gloss {
                        term: g.term,
                        text: g.text,
                    })
                    .collect(),
            }
        })),
        "selvi" => {
            let v = selvi::speak(&selvi::VoiceInput {
                sentence: inp.sentence,
                parts: inp.parts,
                calendar: inp.calendar,
                period: inp.period,
            });
            Some([Lang::Tr, Lang::En].map(|l| {
                let s = v.get(l);
                Surfaces {
                    headline: s.headline.clone(),
                    detail: s.detail.clone(),
                    ledger: s.ledger.clone(),
                    glossary: s
                        .glossary
                        .iter()
                        .map(|g| Gloss {
                            term: g.term.clone(),
                            text: g.text.clone(),
                        })
                        .collect(),
                }
            }))
        }
        _ => None,
    }
}

/// The text face of a reader who gets no face scene (A32: a reader without the data
/// licence, or without the series' data): the voice's headline (what, when, whether it
/// still holds), its ledger line and its detail (for a redacted reader: which licence
/// opens the numbers and who computed them), one per line. For a redacted reader every
/// part is digit-free (the voice's rule, K33).
#[must_use]
pub fn text_face(s: &Surfaces) -> String {
    format!("{}\n{}\n{}", s.headline, s.ledger, s.detail)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_is_well_formed() {
        assert_eq!(VOICES[0], DEFAULT_VOICE);
        assert!(is_voice(PRESENTED_VOICE));
        assert_ne!(PRESENTED_VOICE, DEFAULT_VOICE);
        assert!(!is_voice("tunca"), "tunca was eliminated (M17)");
        assert!(!is_voice("r3"));
        assert_eq!(names(), ["r4", "marla", "selvi"]);
    }
}
