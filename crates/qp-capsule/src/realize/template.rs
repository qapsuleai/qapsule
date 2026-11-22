//! The R3 realizer ([`Template`]): renders a [`Plan`] and guards a redacted reader.

use super::{en, tr, Plan};
use crate::{Lang, SentenceInput};

/// The realizer.
#[derive(Debug, Clone, Copy, Default)]
pub struct Template;

impl crate::Realizer for Template {
    fn name(&self) -> &'static str {
        "template"
    }

    fn sentence(&self, inp: &SentenceInput<'_>, lang: Lang) -> String {
        let p = Plan::new(inp);
        let s = render(&p, lang);
        if p.redacted && s.bytes().any(|b| b.is_ascii_digit()) {
            // Safety net (K33): never let a digit through for a redacted reader.
            return fallback_redacted(&p, lang);
        }
        s
    }
}

/// Renders a plan (no guard).
#[must_use]
pub fn render(p: &Plan<'_>, lang: Lang) -> String {
    match lang {
        Lang::Tr => tr::render(p),
        Lang::En => en::render(p),
    }
}

fn fallback_redacted(p: &Plan<'_>, lang: Lang) -> String {
    let inst = if p.inst.bytes().any(|b| b.is_ascii_digit()) {
        match lang {
            Lang::Tr => "Bu hisse",
            Lang::En => "This stock",
        }
    } else {
        p.inst
    };
    match lang {
        Lang::Tr => format!(
            "{inst} için sunucunun doğruladığı bir {} sinyali var; ayrıntılar veri lisansı gerektiriyor.",
            tr::noun_state(p.kind, p.dir)
        ),
        Lang::En => format!(
            "There is a server-verified {} signal for {inst}; the details require a data license.",
            en::noun_bare(p.kind, p.dir)
        ),
    }
}
