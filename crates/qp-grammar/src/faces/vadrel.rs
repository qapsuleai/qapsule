//! R4/M8 face design **`vadrel`**: the sentence leads.
//!
//! The face is one large, short, everyday sentence that says what happened, when (the
//! last move), and whether it still holds; the glyph sits beside it, small and calm; one
//! line of additional information (the shortened ledger) sits in the face's top row.
//! Someone who reads only the sentence still gets what, when and still-valid.
//!
//! Layout (390 × [`HEIGHT`] CSS px; the compiler's plot margins apply: plot x 8..318,
//! top 34, bottom `HEIGHT − 24`):
//!
//! - top row (the view's title row, 16 px): the ledger, e.g. "Benzerleri: 74'te 36
//!   tuttu" / "Past: 36 of 74 played out" — the face's one additional item;
//! - panel `face` (panel-positioned only, so the compiler draws no grid or axes):
//!   - `glyph` ([`GLYPH_PX`] px) at the top left;
//!   - `headline` (the finding's `headline` field = [`headline`], [`HEADLINE_PX`] px)
//!     right of the glyph, wrapping at the panel's edge; full text colour while the claim
//!     holds, `muted` when it no longer holds or has closed (the words say which).
//!
//! No price line, no chart: nothing on the face but the sentence, the glyph and the
//! ledger row. The x domain is the last bar only, so no date label is drawn.
//!
//! Without a capsule story the face falls back to the reference card rule
//! ([`depth::capsule_view`]); without headline pieces the headline is the capsule's
//! sentence (`faces::headline_text`) and the top row is empty.

use qp_schema::capsule::{Stage, Validity};
use qp_schema::grammar::{
    ColorSpec, Layer, Mark, Panel, PanelPos, Position, Scale, ScaleKind, SizeSpec, View,
};
use qp_schema::protocol::Language;

use crate::depth;
use crate::error::GrammarError;
use crate::provider::{DataProvider, HeadlineParts};

/// Face height (CSS px at the phone width 390): title row 34 + five 20 px headline lines
/// (26 px each; an 80-character headline wraps into at most five beside the glyph) +
/// bottom margin 24.
pub const HEIGHT: u32 = 188;
/// Headline size (CSS px): large text (≥ 18 px).
pub const HEADLINE_PX: f64 = 20.0;
/// Glyph size (CSS px).
pub const GLYPH_PX: f64 = 32.0;
/// Gap between the glyph and the headline (CSS px).
const GAP_PX: f64 = 10.0;
/// Headline character limit (SPEC §2(b)).
pub const MAX_CHARS: usize = 80;

fn layer(id: &str, mark: Mark, finding: &str) -> Layer {
    let mut l = Layer::new(mark, 0);
    l.id = Some(id.to_owned());
    l.finding = Some(finding.to_owned());
    l
}

fn at(px: f64) -> Position {
    Position::Panel(PanelPos { panel: 0.0, px })
}

/// The face view of `finding` (see the module doc).
///
/// # Errors
/// Unknown finding/fact, a finding without facts, or a series without bars.
pub fn view(
    finding: &str,
    max_points: u32,
    p: &dyn DataProvider,
    id: &str,
) -> Result<View, GrammarError> {
    let story = p.capsule(finding).is_some_and(|c| c.story.is_some());
    if !story {
        return depth::capsule_view(finding, max_points, p, id);
    }
    // The reference card over one point: its x domain is the last bar only.
    let card = depth::capsule_card_view(finding, 1, p, id)?;
    let hp = p.headline_parts(finding);

    let mut glyph = layer("glyph", Mark::Glyph, finding);
    glyph.encoding.x = Some(at(0.0));
    glyph.encoding.y = Some(at(0.0));
    glyph.encoding.size = Some(SizeSpec::Constant(GLYPH_PX));

    let mut head = layer("headline", Mark::Text, finding);
    head.encoding.text = Some("headline".to_owned());
    head.encoding.x = Some(at(GLYPH_PX + GAP_PX));
    head.encoding.y = Some(at(0.0));
    head.encoding.size = Some(SizeSpec::Constant(HEADLINE_PX));
    if hp.as_ref().is_some_and(|h| h.validity != Validity::Valid) {
        head.encoding.color = Some(ColorSpec::Constant("muted".to_owned()));
    }

    Ok(View {
        id: id.to_owned(),
        title: Some(hp.as_ref().map(ledger_row).unwrap_or_default()),
        display_tz: None,
        theme: None,
        x: card.x,
        y: Vec::new(),
        panels: vec![Panel {
            id: Some(0),
            name: Some("face".to_owned()),
            weight: Some(1.0),
            y: Scale {
                kind: ScaleKind::Linear,
                domain: None,
                id: Some("y0".to_owned()),
            },
        }],
        layers: vec![glyph, head],
        emphasis: Vec::new(),
    })
}

/// The top row: the shortened ledger ("Benzerleri: 74'te 36 tuttu" / "Past: 36 of 74
/// played out"; few cases keep their "(az örnek)" / "(few cases)").
#[must_use]
pub fn ledger_row(hp: &HeadlineParts) -> String {
    match (hp.lang, hp.ledger_n) {
        (Language::Tr, Some(0)) => "Benzerleri: henüz örnek yok".to_owned(),
        (Language::En, Some(0)) => "Past: no cases yet".to_owned(),
        (Language::Tr, _) => format!("Benzerleri: {}", hp.ledger_short),
        (Language::En, _) => format!("Past: {}", hp.ledger_short),
    }
}

/// What happened, as an everyday clause with the instrument as subject, in the past
/// tense (the status sentence after it says whether it still holds). Keyed by the
/// pieces' event phrase (`qp_capsule::realize` `noun_state` / `noun_bare`); an unknown
/// phrase falls back to "<instrument>: <event>".
fn clause(hp: &HeadlineParts) -> String {
    let i = &hp.instrument;
    let tr = |s: &str| format!("{i} {s}");
    let en = |s: &str| format!("{i} {s}");
    match (hp.lang, hp.event.as_str()) {
        (Language::Tr, "yukarı kırılım") => tr("önemli bir eşiğin üstüne çıktı"),
        (Language::Tr, "aşağı kırılım") => tr("önemli bir eşiğin altına indi"),
        (Language::Tr, "kırılım") => tr("önemli bir eşiği aştı"),
        (Language::Tr, "yukarı dönüş") => tr("düşüşten yukarı döndü"),
        (Language::Tr, "aşağı dönüş") => tr("yükselişten aşağı döndü"),
        (Language::Tr, "dönüş işareti") => tr("yön değiştirdi"),
        (Language::Tr, "yükseliş eğilimi") => tr("yükselişini sürdürdü"),
        (Language::Tr, "düşüş eğilimi") => tr("düşüşünü sürdürdü"),
        (Language::Tr, "eğilim") => tr("gidişini sürdürdü"),
        (Language::Tr, "sıkışma (yön belirsiz)") => tr("dar bir aralığa sıkıştı"),
        (Language::Tr, "aşırı alım") => tr("kısa sürede aşırı yükseldi"),
        (Language::Tr, "aşırı satım") => tr("kısa sürede aşırı düştü"),
        (Language::Tr, "aşırılık") => tr("uç bir noktaya ulaştı"),
        (Language::Tr, "para girişi") => tr("için büyük alım geldi"),
        (Language::Tr, "para çıkışı") => tr("için büyük satış geldi"),
        (Language::Tr, "olağandışı para akışı") => tr("için olağandışı para hareketi oldu"),
        (Language::En, "upside breakout") => en("broke above a key level"),
        (Language::En, "downside breakout") => en("broke below a key level"),
        (Language::En, "breakout") => en("broke through a key level"),
        (Language::En, "upward reversal") => en("turned up after falling"),
        (Language::En, "downward reversal") => en("turned down after rising"),
        (Language::En, "reversal") => en("changed direction"),
        (Language::En, "uptrend") => en("kept climbing"),
        (Language::En, "downtrend") => en("kept sliding"),
        (Language::En, "trend") => en("held its course"),
        (Language::En, "squeeze (direction open)") => en("got squeezed into a narrow range"),
        (Language::En, "overbought") => en("rose too far too fast"),
        (Language::En, "oversold") => en("fell too far too fast"),
        (Language::En, "extreme") => en("hit an unusual extreme"),
        (Language::En, "money inflow") => en("saw unusually heavy buying"),
        (Language::En, "money outflow") => en("saw unusually heavy selling"),
        (Language::En, "unusual money flow") => en("saw unusual money flow"),
        _ => short_clause(hp),
    }
}

/// The product's own noun phrase: "<instrument>: <event>" (the shorter fallback).
fn short_clause(hp: &HeadlineParts) -> String {
    format!("{}: {}", hp.instrument, hp.event)
}

/// First letter upper-cased (Turkish dotted i → İ).
fn cap(s: &str, lang: Language) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some('i') if lang == Language::Tr => format!("İ{}", c.as_str()),
        Some(f) => f.to_uppercase().chain(c).collect(),
    }
}

/// What the last move did to an open story, if it was not its birth.
fn moved_verb(stage: Stage, lang: Language) -> Option<&'static str> {
    match (stage, lang) {
        (Stage::Strengthened | Stage::Confirmed, Language::Tr) => Some("güçlendi"),
        (Stage::Strengthened | Stage::Confirmed, Language::En) => Some("strengthened"),
        (Stage::Weakened, Language::Tr) => Some("zayıfladı"),
        (Stage::Weakened, Language::En) => Some("weakened"),
        _ => None,
    }
}

/// One headline form over a what-clause (`what`); `time_first`: the compact form (open:
/// the time leads and the last move's verb is left out; closed: no "closed" label).
fn form(hp: &HeadlineParts, what: &str, time_first: bool) -> String {
    let (l, m, v) = (hp.lang, hp.moved.as_str(), hp.validity_phrase.as_str());
    if hp.validity == Validity::Closed {
        // The compact form drops the "closed" label (the ending says it).
        return match (l, time_first) {
            (Language::Tr, false) => format!("{what}. Kapandı: {m} {v}."),
            (Language::En, false) => format!("{what}. Closed: {v} {m}."),
            (Language::Tr, true) => format!("{what}. {} {v}.", cap(m, l)),
            (Language::En, true) => format!("{what}. {} {m}.", cap(v, l)),
        };
    }
    let status = cap(v, l);
    match moved_verb(hp.stage, l) {
        Some(verb) if !time_first => match l {
            Language::Tr => format!("{what}; {m} {verb}. {status}."),
            Language::En => format!("{what}; {verb} {m}. {status}."),
        },
        _ => match l {
            Language::Tr => format!("{} {what}. {status}.", cap(m, l)),
            Language::En => format!("{}, {what}. {status}.", cap(m, l)),
        },
    }
}

/// The `vadrel` headline template: what happened (an everyday clause), when it last
/// moved, and whether it still holds — or how and when it closed. E.g.
/// "Dün SYN03 önemli bir eşiğin üstüne çıktı. Hâlâ geçerli." /
/// "Yesterday, SYN03 broke above a key level. Still holds.";
/// "SYN12 önemli bir eşiğin altına indi; 2 saat önce güçlendi. Hâlâ geçerli.";
/// "SYN25 önemli bir eşiğin altına indi. Kapandı: 15 Mayıs'ta gerçekleşti.".
/// At most [`MAX_CHARS`] characters: a longer form gives way to the product's noun phrase,
/// then to the compact forms. Adds no digit (a redacted reader's pieces have none).
#[must_use]
pub fn headline(hp: &HeadlineParts) -> String {
    let what = clause(hp);
    let short = short_clause(hp);
    let tries = [
        form(hp, &what, false),
        form(hp, &short, false),
        form(hp, &what, true),
        form(hp, &short, true),
    ];
    let fits = |s: &&String| s.chars().count() <= MAX_CHARS;
    tries.iter().find(fits).unwrap_or(&tries[3]).clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    const EVENTS: [(&str, &str); 16] = [
        ("yukarı kırılım", "upside breakout"),
        ("aşağı kırılım", "downside breakout"),
        ("kırılım", "breakout"),
        ("yukarı dönüş", "upward reversal"),
        ("aşağı dönüş", "downward reversal"),
        ("dönüş işareti", "reversal"),
        ("yükseliş eğilimi", "uptrend"),
        ("düşüş eğilimi", "downtrend"),
        ("eğilim", "trend"),
        ("sıkışma (yön belirsiz)", "squeeze (direction open)"),
        ("aşırı alım", "overbought"),
        ("aşırı satım", "oversold"),
        ("aşırılık", "extreme"),
        ("para girişi", "money inflow"),
        ("para çıkışı", "money outflow"),
        ("olağandışı para akışı", "unusual money flow"),
    ];

    /// The longest time phrases `qp_capsule::realize::elapsed` writes.
    const TIMES: [(&str, &str); 6] = [
        ("3 işlem günü önce", "3 trading days ago"),
        ("çarşamba 17:00'de", "on Wednesday at 17:00"),
        ("15 Eylül 2025'te", "on 15 September 2025"),
        ("dün 17:00'de", "yesterday at 17:00"),
        ("12 dakika önce", "12 minutes ago"),
        ("bugün", "today"),
    ];

    fn hp(lang: Language, ev: usize, t: usize, stage: Stage, validity: Validity) -> HeadlineParts {
        let tr = lang == Language::Tr;
        let ending = |tr_: &str, en_: &str| if tr { tr_ } else { en_ }.to_owned();
        let validity_phrase = match (validity, stage) {
            (Validity::Valid, _) => ending("hâlâ geçerli", "still holds"),
            (Validity::NoLonger, _) => ending("artık geçerli değil", "no longer holds"),
            (Validity::Closed, Stage::Realized) => ending("gerçekleşti", "played out"),
            (Validity::Closed, Stage::Invalidated) => ending("tutmadı", "did not play out"),
            (Validity::Closed, Stage::Expired) => ending("süresi doldu", "ran out of time"),
            (Validity::Closed, _) => ending("geri çekildi", "withdrawn"),
        };
        let time = if tr { TIMES[t].0 } else { TIMES[t].1 };
        HeadlineParts {
            lang,
            instrument: "THYAO6".into(),
            event: if tr { EVENTS[ev].0 } else { EVENTS[ev].1 }.into(),
            direction_word: String::new(),
            when: time.into(),
            moved: time.into(),
            validity_phrase,
            ledger_short: ending(
                "19'da 19 tuttu (az örnek)",
                "19 of 19 played out (few cases)",
            ),
            stage,
            validity,
            redacted: false,
            ledger_n: Some(19),
            few_cases: true,
        }
    }

    const CASES: [(Stage, Validity); 10] = [
        (Stage::Born, Validity::Valid),
        (Stage::Strengthened, Validity::Valid),
        (Stage::Confirmed, Validity::Valid),
        (Stage::Weakened, Validity::Valid),
        (Stage::Born, Validity::NoLonger),
        (Stage::Weakened, Validity::NoLonger),
        (Stage::Realized, Validity::Closed),
        (Stage::Invalidated, Validity::Closed),
        (Stage::Expired, Validity::Closed),
        (Stage::Retracted, Validity::Closed),
    ];

    #[test]
    fn every_headline_fits_and_names_time_and_validity() {
        for lang in [Language::Tr, Language::En] {
            for ev in 0..EVENTS.len() {
                for t in 0..TIMES.len() {
                    for (stage, validity) in CASES {
                        let p = hp(lang, ev, t, stage, validity);
                        let h = headline(&p);
                        assert!(h.chars().count() <= MAX_CHARS, "{} {h}", h.chars().count());
                        assert!(h.to_lowercase().contains(&p.moved.to_lowercase()), "{h}");
                        assert!(
                            h.to_lowercase().contains(&p.validity_phrase.to_lowercase()),
                            "{h}"
                        );
                        assert!(h.contains(&p.instrument), "{h}");
                        let low = h.to_lowercase();
                        for bad in ["bar", "aile", "teyit", "family"] {
                            assert!(!low.contains(bad), "{h}");
                        }
                        assert_eq!(h, headline(&p));
                    }
                }
            }
        }
    }

    #[test]
    fn every_event_has_an_everyday_clause() {
        for lang in [Language::Tr, Language::En] {
            for ev in 0..EVENTS.len() {
                let p = hp(lang, ev, 5, Stage::Born, Validity::Valid);
                assert_ne!(clause(&p), short_clause(&p), "{}", p.event);
            }
        }
    }

    #[test]
    fn examples() {
        let p = hp(Language::Tr, 0, 5, Stage::Born, Validity::Valid);
        assert_eq!(
            headline(&p),
            "Bugün THYAO6 önemli bir eşiğin üstüne çıktı. Hâlâ geçerli."
        );
        let p = hp(Language::En, 1, 3, Stage::Strengthened, Validity::Valid);
        assert_eq!(
            headline(&p),
            "THYAO6 broke below a key level; strengthened yesterday at 17:00. Still holds."
        );
        let p = hp(Language::Tr, 1, 2, Stage::Realized, Validity::Closed);
        assert_eq!(
            headline(&p),
            "THYAO6 önemli bir eşiğin altına indi. Kapandı: 15 Eylül 2025'te gerçekleşti."
        );
        assert_eq!(cap("işlem", Language::Tr), "İşlem");
        assert_eq!(ledger_row(&p), "Benzerleri: 19'da 19 tuttu (az örnek)");
    }

    #[test]
    fn redacted_headline_adds_no_digit() {
        for lang in [Language::Tr, Language::En] {
            for (stage, validity) in CASES {
                let mut p = hp(lang, 0, 5, stage, validity);
                p.instrument = "SYN".into();
                p.redacted = true;
                p.ledger_n = None;
                p.ledger_short = "az örnek".into();
                let h = headline(&p);
                assert!(!h.bytes().any(|b| b.is_ascii_digit()), "{h}");
                assert!(!ledger_row(&p).bytes().any(|b| b.is_ascii_digit()));
            }
        }
    }

    const _: () = assert!(HEADLINE_PX >= 18.0 && HEIGHT >= 120 && HEIGHT <= 844);
}
