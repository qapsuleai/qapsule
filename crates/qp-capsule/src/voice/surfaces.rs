//! A capsule's four surfaces and [`speak`], the one entry point the authority calls (with
//! the licence guard [`unnumber`] and the no-face text [`text_face`]).

use qp_calendar::ExchangeCalendar;

use super::marla;
use super::selvi;
use crate::realize::elapsed::Period;
use crate::realize::parts::Parts;
use crate::{Lang, SentenceInput};

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
    /// (RP4/T7) The row's time stamp (`[tr, en]`, `crate::realize::stamp`) to put before
    /// the headline (`"<stamp> · <headline>"`); the voice keeps the whole within its
    /// headline limit. `None`: the headline as before.
    pub stamp: Option<[&'a str; 2]>,
}

/// The surfaces of voice `name` in Turkish and English (`[tr, en]`); `None` for `r4`
/// (and an unknown name). Deterministic: no clock, no randomness, no model.
///
/// **Licence guard (K33, A32).** For a redacted reader every surface leaves here without a
/// single digit (the instrument label aside): a voice writes its public rule parameters in
/// words for a redacted reader, and any digit run a voice still wrote (a public parameter
/// such as "20 günlük") is turned into number words here ([`unnumber`]).
#[must_use]
pub fn speak(name: &str, inp: &Input<'_>) -> Option<[Surfaces; 2]> {
    let out = match inp.stamp {
        None => speak_raw(name, inp, marla::MAX_HEADLINE)?,
        Some(st) => {
            // The stamp and its separator take their characters off the headline's room.
            let take = st.iter().map(|s| s.chars().count()).max().unwrap_or(0) + 3;
            let [mut tr, mut en] = speak_raw(name, inp, marla::MAX_HEADLINE.saturating_sub(take))?;
            tr.headline = crate::realize::stamp::with_stamp(st[0], &tr.headline);
            en.headline = crate::realize::stamp::with_stamp(st[1], &en.headline);
            [tr, en]
        }
    };
    if !(inp.sentence.redacted || inp.parts.redacted) {
        return Some(out);
    }
    let sym = inp.parts.instrument.as_str();
    let [tr, en] = out;
    Some([
        unnumber_all(tr, sym, Lang::Tr),
        unnumber_all(en, sym, Lang::En),
    ])
}

fn unnumber_all(s: Surfaces, sym: &str, lang: Lang) -> Surfaces {
    Surfaces {
        headline: unnumber(&s.headline, sym, lang),
        detail: unnumber(&s.detail, sym, lang),
        ledger: unnumber(&s.ledger, sym, lang),
        glossary: s
            .glossary
            .into_iter()
            .map(|g| Gloss {
                term: unnumber(&g.term, sym, lang),
                text: unnumber(&g.text, sym, lang),
            })
            .collect(),
    }
}

/// `text` with every run of ASCII digits outside the instrument label `sym` written as
/// number words (Turkish: a suffix after an apostrophe joins the word, "20'nin" →
/// "yirminin", the suffix already follows the spoken numeral). Runs of four or more digits
/// and digits joined by a decimal or thousands mark do not occur in a redacted voice; they
/// are written digit by digit in words all the same, so no digit ever leaves.
#[must_use]
pub fn unnumber(text: &str, sym: &str, lang: Lang) -> String {
    let mut out = String::new();
    let mut rest = text;
    while !rest.is_empty() {
        if !sym.is_empty() && rest.starts_with(sym) {
            out.push_str(sym);
            rest = &rest[sym.len()..];
            continue;
        }
        let c = rest.chars().next().unwrap_or(' ');
        if c.is_ascii_digit() {
            let n = rest.bytes().take_while(u8::is_ascii_digit).count();
            let run = &rest[..n];
            rest = &rest[n..];
            let words = |x: u32| match lang {
                Lang::Tr => marla::words_tr(x),
                Lang::En => marla::words_en(x),
            };
            match run.parse::<u32>() {
                Ok(x) if x < 1000 => out.push_str(&words(x)),
                _ => {
                    let w: Vec<String> = run.bytes().map(|b| words(u32::from(b - b'0'))).collect();
                    out.push_str(&w.join(" "));
                }
            }
            if lang == Lang::Tr && (rest.starts_with('\'') || rest.starts_with('’')) {
                let a = rest.chars().next().map_or(0, char::len_utf8);
                rest = &rest[a..];
            }
            continue;
        }
        out.push(c);
        rest = &rest[c.len_utf8()..];
    }
    out
}

fn speak_raw(name: &str, inp: &Input<'_>, room: usize) -> Option<[Surfaces; 2]> {
    match name {
        "marla" => Some(
            marla::speak_both_in(inp.sentence, inp.parts, room).map(|v| Surfaces {
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
            }),
        ),
        "selvi" => {
            let v = selvi::speak_in(
                &selvi::VoiceInput {
                    sentence: inp.sentence,
                    parts: inp.parts,
                    calendar: inp.calendar,
                    period: inp.period,
                },
                room,
            );
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
