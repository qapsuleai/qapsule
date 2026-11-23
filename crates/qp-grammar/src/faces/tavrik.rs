//! R4/M8 face design `tavrik` — "the mark first".
//!
//! The face's dominant element is a large meaning glyph: what happened, its direction
//! and whether it still holds are carried by the glyph's shape (the reader's alphabet,
//! `siluet` or `rozet`, and its outcome style), so a feed of these faces reads as a
//! column of marks the eye can scan while scrolling. Beside it, one short headline
//! sentence from the headline pieces (`HeadlineParts`): instrument, what happened, when
//! it last moved, whether it still holds, and the ledger shortened but honest. No other
//! text layer and no price panel: zero additional-information units.
//!
//! Layout (one card panel, every layer panel-positioned, so the panel has no grid or
//! axis): the glyph at the top left ([`GLYPH_PX`]); the headline to its right
//! ([`HEADLINE_PX`], the theme's text colour), wrapping inside the panel.

use qp_schema::grammar::{Layer, Mark, PanelPos, Position, SizeSpec, View};
use qp_schema::protocol::Language;

use crate::depth;
use crate::provider::{DataProvider, HeadlineParts};
use qp_look::error::GrammarError;
use qp_look::look::Look;

/// Glyph size (CSS px): the face's dominant element.
pub const GLYPH_PX: f64 = 76.0;
/// Headline text size (CSS px).
pub const HEADLINE_PX: f64 = 15.0;
/// Gap between the glyph and the headline (CSS px).
const GAP_PX: f64 = 14.0;
/// The face height (CSS px at 390 wide).
pub const HEIGHT: u32 = 156;
/// The headline's hard limit (SPEC §2(b)).
const MAX_CHARS: usize = 80;
/// Above this length the template shortens the ledger (keeps the face at four lines).
const SOFT_CHARS: usize = 78;

fn panel_at(panel: f64, px: f64) -> Position {
    Position::Panel(PanelPos { panel, px })
}

/// The face view: the R3 card's first panel only, its glyph enlarged, its sentence (the
/// template's headline in this face's scene) beside the glyph; the ledger line and the
/// price panel are left out. A finding without a capsule story gets the reference view.
///
/// # Errors
/// The card view's errors (unknown finding/fact, no bars).
pub fn view(
    look: &Look,
    finding: &str,
    max_points: u32,
    p: &dyn DataProvider,
    id: &str,
) -> Result<View, GrammarError> {
    if p.capsule(finding).is_none_or(|c| c.story.is_none()) {
        return depth::capsule_view_in(look, finding, max_points, p, id);
    }
    let mut v = depth::capsule_card_view_in(look, finding, max_points, p, id)?;
    // Title row: the instrument alone (the default title adds the bar period, "1d").
    let bars_of = v
        .layers
        .iter()
        .find(|l| l.id.as_deref() == Some("line"))
        .and_then(|l| l.series.clone());
    if let Some(bs) = &bars_of {
        v.title = p.symbol(bs);
        // The face draws no time axis: the visible range is the last bar alone, so no
        // month label is drawn under the card.
        if let Some(t) = p.bars(bs).and_then(|b| b.last().map(|x| x.t)) {
            #[allow(clippy::cast_precision_loss)]
            let t = t as f64;
            v.x.domain = Some([t, t]);
        }
    }
    v.panels.retain(|pl| pl.id == Some(0));
    if let Some(pl) = v.panels.first_mut() {
        pl.weight = Some(1.0);
    }
    let mut layers: Vec<Layer> = Vec::new();
    for mut l in std::mem::take(&mut v.layers) {
        match l.id.as_deref() {
            Some("glyph") => {
                l.encoding.x = Some(panel_at(0.0, 0.0));
                l.encoding.y = Some(panel_at(0.0, 2.0));
                l.encoding.size = Some(SizeSpec::Constant(GLYPH_PX));
                layers.push(l);
            }
            Some("sentence") if l.mark == Mark::Text => {
                l.encoding.x = Some(panel_at(0.0, GLYPH_PX + GAP_PX));
                l.encoding.y = Some(panel_at(0.0, 4.0));
                l.encoding.size = Some(SizeSpec::Constant(HEADLINE_PX));
                l.encoding.color = None;
                layers.push(l);
            }
            _ => {}
        }
    }
    v.layers = layers;
    Ok(v)
}

fn join(parts: &[&str]) -> String {
    parts
        .iter()
        .filter(|s| !s.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join(" · ")
}

/// The ledger, shorter: few cases (or none) say only that, without the counts.
fn ledger_shorter(hp: &HeadlineParts) -> String {
    match (hp.ledger_n, hp.lang) {
        (Some(0), Language::Tr) => "karnesi yok".into(),
        (Some(0), Language::En) => "no record yet".into(),
        _ if hp.few_cases => match hp.lang {
            Language::Tr => "az örnek".into(),
            Language::En => "few cases".into(),
        },
        _ => hp.ledger_short.clone(),
    }
}

/// The ledger as a past record: "geçmişte 74'te 36 tuttu" / "past: 36 of 74 played
/// out" (so it is not read as the story's own outcome); no record stays as it is.
fn ledger_past(hp: &HeadlineParts) -> String {
    if hp.ledger_n == Some(0) {
        return hp.ledger_short.clone();
    }
    match hp.lang {
        Language::Tr => format!("geçmişte {}", hp.ledger_short),
        Language::En => format!("past: {}", hp.ledger_short),
    }
}

/// Upper-case first letter (Turkish dotted/dotless i respected).
fn capital(s: &str, lang: Language) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some('i') if lang == Language::Tr => format!("İ{}", c.as_str()),
        Some(f) => f.to_uppercase().chain(c).collect(),
    }
}

/// The headline template: `<Event> · <when it last moved> · <validity> · <past record>`,
/// e.g. "Yukarı kırılım · bugün · hâlâ geçerli · geçmişte 30'da 18 tuttu" / "Upside
/// breakout · today · still holds · past: 18 of 30 played out". The instrument is the face's title row, not part
/// of the sentence. Longer than [`SOFT_CHARS`]: the ledger loses its "past" word and, with
/// few cases, its counts (never its "few cases"); the result is at most [`MAX_CHARS`] characters. Adds no digit.
#[must_use]
pub fn headline(hp: &HeadlineParts) -> String {
    let event = capital(&hp.event, hp.lang);
    let full = join(&[&event, &hp.moved, &hp.validity_phrase, &ledger_past(hp)]);
    if full.chars().count() <= SOFT_CHARS {
        return full;
    }
    let last = join(&[&event, &hp.moved, &hp.validity_phrase, &ledger_shorter(hp)]);
    if last.chars().count() <= MAX_CHARS {
        last
    } else {
        // Not reached with today's pieces (longest ≈ 70); keep the hard limit anyway.
        last.chars().take(MAX_CHARS - 1).collect::<String>() + "…"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qp_schema::capsule::{Stage, Validity};

    fn hp(lang: Language, n: Option<u32>, few: bool) -> HeadlineParts {
        let tr = lang == Language::Tr;
        HeadlineParts {
            lang,
            instrument: "SYN03".into(),
            event: if tr {
                "olağandışı para akışı"
            } else {
                "squeeze (direction open)"
            }
            .into(),
            direction_word: if tr { "yönsüz" } else { "no direction" }.into(),
            when: if tr {
                "6 işlem günü önce"
            } else {
                "6 trading days ago"
            }
            .into(),
            moved: if tr {
                "6 işlem günü önce"
            } else {
                "6 trading days ago"
            }
            .into(),
            validity_phrase: if tr {
                "artık geçerli değil"
            } else {
                "did not play out"
            }
            .into(),
            ledger_short: match (tr, few) {
                (true, true) => "17'de 3 tuttu (az örnek)",
                (false, true) => "3 of 17 played out (few cases)",
                (true, false) => "130'da 78 tuttu",
                (false, false) => "78 of 130 played out",
            }
            .into(),
            stage: Stage::Weakened,
            validity: Validity::NoLonger,
            redacted: false,
            ledger_n: n,
            few_cases: few,
        }
    }

    #[test]
    fn headline_fits_and_stays_honest() {
        for lang in [Language::Tr, Language::En] {
            for (n, few) in [(Some(17), true), (Some(130), false), (Some(0), true)] {
                let h = headline(&hp(lang, n, few));
                assert!(h.chars().count() <= MAX_CHARS, "{h}");
                if few {
                    let lw = h.to_lowercase();
                    assert!(
                        lw.contains("az örnek")
                            || lw.contains("few cases")
                            || lw.contains("karnesi yok")
                            || lw.contains("no record"),
                        "{h}"
                    );
                }
                assert_eq!(h, headline(&hp(lang, n, few)));
            }
        }
        let mut s = hp(Language::Tr, Some(30), false);
        s.event = "yukarı kırılım".into();
        s.moved = "bugün".into();
        s.validity_phrase = "hâlâ geçerli".into();
        s.ledger_short = "30'da 18 tuttu".into();
        assert_eq!(
            headline(&s),
            "Yukarı kırılım · bugün · hâlâ geçerli · geçmişte 30'da 18 tuttu"
        );
    }
}
