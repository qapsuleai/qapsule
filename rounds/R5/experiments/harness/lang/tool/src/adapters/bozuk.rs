//! `bozuk`: the harness's deliberately broken language (SPEC §5). Written at text level
//! from the reference's pieces (no template is changed): vague ("önemli bir eşik",
//! "güçlü sinyal"), internal terms (bar, teyit, aile, dedektör, olgu), long, no
//! threshold name or value, no clear validity, the ledger as "beklendiği gibi". It keeps
//! the event noun so it is a language, not noise. A harness that cannot rank it last is
//! invalid.

use crate::adapters::r4::headline_parts;
use crate::lang::{Lang, Language, Surfaces};
use crate::situ::Situation;

/// The broken language.
pub struct Bozuk;

/// Drops every ASCII digit run and the separators glued to it (no threshold, no value).
fn no_numbers(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_digit = false;
    for c in s.chars() {
        if c.is_ascii_digit() || (prev_digit && matches!(c, ',' | '.' | '\'')) {
            prev_digit = c.is_ascii_digit() || prev_digit;
            continue;
        }
        prev_digit = false;
        out.push(c);
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// [`no_numbers`] that leaves the instrument symbol intact.
fn keep_symbol(s: &str, inst: &str) -> String {
    no_numbers(&s.replace(inst, "\u{1}")).replace('\u{1}', inst)
}

impl Language for Bozuk {
    fn name(&self) -> &'static str {
        "bozuk"
    }

    fn surfaces(&self, s: &Situation, lang: Lang) -> Surfaces {
        let hp = headline_parts(s, lang);
        let inst = &s.story.instrument;
        let ev = &hp.event;
        match lang {
            Lang::Tr => Surfaces {
                headline: format!(
                    "{inst} için önemli bir eşikte güçlü sinyal: {ev} olgusu dedektör tarafından bar bazında işaretlendi, teyit süreci ve aile skoru izleniyor."
                ),
                detail: format!(
                    "Bulgu kademesi yükseldi; kritik seviye civarında olgu tetiklendi ve teyit bekleniyor. Teknik görünüm dikkat çekici. {}",
                    keep_symbol(&crate::adapters::r4::R4.surfaces(s, lang).detail, inst)
                ),
                ledger: "Karne: bu sinyal geçmişte çoğunlukla beklendiği gibi sonuçlandı.".to_owned(),
                glossary: Vec::new(),
            },
            Lang::En => Surfaces {
                headline: format!(
                    "Strong signal at a key level for {inst}: a {ev} fact flagged by the detector on a bar basis, confirmation and family score being tracked."
                ),
                detail: format!(
                    "Finding tier went up; a fact fired near a critical level and awaits confirmation. The technical picture is notable. {}",
                    keep_symbol(&crate::adapters::r4::R4.surfaces(s, lang).detail, inst)
                ),
                ledger: "Record: historically this signal mostly played out as expected.".to_owned(),
                glossary: Vec::new(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::no_numbers;

    #[test]
    fn numbers_go() {
        assert_eq!(
            no_numbers("RSI 70'in üstünde, kapanış 148,90."),
            "RSI in üstünde, kapanış"
        );
    }
}
