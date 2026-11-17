//! Reference `r4`: today's product language (R4, `docs/FACE.md`, `docs/CAPSULE.md`).
//!
//! - headline: the presented face `vadrel`'s headline template
//!   (`qp_grammar::faces::headline_template("vadrel")`) over the headline pieces
//!   `qp_capsule::realize::parts`, filled as the authority fills them
//!   (`qp-session` `headparts::from_parts`, replicated here because it is crate-private);
//! - detail: the capsule's sentence (`qp_capsule::realize::Template`, the R3 sentence the
//!   open capsule shows; A33);
//! - ledger: the face's ledger row (`vadrel::ledger_row`, private, replicated:
//!   "Benzerleri: …" / "Past: …");
//! - glossary: none (the product has no glossary yet).

use qp_capsule::realize::parts::parts;
use qp_capsule::realize::Template;
use qp_capsule::Realizer;
use qp_grammar::depth::GLYPH_MIN_LEDGER_N;
use qp_grammar::HeadlineParts;
use qp_schema::protocol::Language as PLang;

use crate::lang::{Lang, Language, Surfaces};
use crate::situ::Situation;

/// The reference language.
pub struct R4;

/// The headline pieces of `s` in `lang`, as the authority fills them.
#[must_use]
pub fn headline_parts(s: &Situation, lang: Lang) -> HeadlineParts {
    let cal = s.exchange_calendar();
    let p = parts(&s.story, s.now_t, s.validity, &cal, s.period(), s.redacted);
    let l = lang.capsule();
    HeadlineParts {
        lang: match lang {
            Lang::Tr => PLang::Tr,
            Lang::En => PLang::En,
        },
        instrument: p.instrument.clone(),
        event: p.event(l).to_owned(),
        direction_word: p.direction_word(l).to_owned(),
        when: p.when(l),
        moved: p.moved(l),
        validity_phrase: p.validity_phrase(l).to_owned(),
        ledger_short: p.ledger_short(l),
        stage: p.stage,
        validity: p.validity,
        redacted: p.redacted,
        ledger_n: (!p.redacted).then_some(p.ledger.n),
        few_cases: p.ledger.n < GLYPH_MIN_LEDGER_N,
    }
}

/// `vadrel::ledger_row` (private in the product; same text).
#[must_use]
pub fn ledger_row(hp: &HeadlineParts) -> String {
    match (hp.lang, hp.ledger_n) {
        (PLang::Tr, Some(0)) => "Benzerleri: henüz örnek yok".to_owned(),
        (PLang::En, Some(0)) => "Past: no cases yet".to_owned(),
        (PLang::Tr, _) => format!("Benzerleri: {}", hp.ledger_short),
        (PLang::En, _) => format!("Past: {}", hp.ledger_short),
    }
}

impl Language for R4 {
    fn name(&self) -> &'static str {
        "r4"
    }

    fn surfaces(&self, s: &Situation, lang: Lang) -> Surfaces {
        let hp = headline_parts(s, lang);
        let tmpl = qp_grammar::faces::headline_template(Some("vadrel"))
            .ok()
            .flatten()
            .expect("vadrel has a headline template");
        Surfaces {
            headline: tmpl(&hp),
            detail: Template.sentence(&s.input(), lang.capsule()),
            ledger: ledger_row(&hp),
            glossary: Vec::new(),
        }
    }
}
