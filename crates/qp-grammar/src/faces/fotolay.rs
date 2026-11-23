//! R4/M8 face design `fotolay`: **the picture of the event**.
//!
//! The closed face is a small, plain drawing of what the price did around the event: the
//! price line, a thin upright line at the moment it happened, the level it is about (the
//! event's own evidence geometry when it has one) and a dot where the price stood at that
//! moment; the meaning glyph stands in its own strip right above that moment. A short
//! headline says what the picture shows: what happened, when it last moved, whether it
//! still holds, and how similar events turned out before (honest about few cases).
//!
//! When the claim no longer holds or the story closed, the whole picture is drawn in the
//! muted colour (the line, the moment, the level, the dot): a faded picture is a story
//! that is over or no longer in force; the glyph and the headline say which.
//!
//! Layout (390 CSS px wide, [`HEIGHT`]): the title row (instrument), panel `head` (the
//! headline), panel `mark` (the glyph strip; the glyph at the event's bar), panel `pic`
//! (the picture). `head` and `mark` hold only panel-positioned layers (no scale chrome);
//! `pic` is a data panel. Extra units (SPEC §2(b)): the picture panel (1).
//!
//! Deterministic; colours are theme tokens only; no internal terms in the headline.

use qp_schema::capsule::Validity;
use qp_schema::grammar::{
    ColorSpec, DataRef, Layer, Mark, Panel, PanelPos, Position, Scale, ScaleKind, SizeSpec, View,
};
use qp_schema::protocol::Language;
use qp_schema::{Bar, Fact};

use crate::depth;
use crate::provider::{DataProvider, HeadlineParts, SeriesKind};
use qp_look::error::{ErrorKind, GrammarError};
use qp_look::look::Look;

/// Face height in CSS px at the phone width.
pub const HEIGHT: u32 = 228;
/// Headline text size (CSS px).
const HEAD_PX: f64 = 15.0;
/// Glyph size (CSS px).
const GLYPH_PX: f64 = 28.0;
/// Bars shown before the event.
const BEFORE: u64 = 60;
/// Fewest bars in the picture (the event near the right edge still has a run-up).
const MIN_BARS: u64 = 80;
/// Panel heights in px (the plot of a 228 px face is 228 − 34 − 24 − 2·8 = 154 px; the
/// headline gets three lines of 20 px).
const HEAD_H: f64 = 60.0;
const MARK_H: f64 = 30.0;
const PIC_H: f64 = 64.0;

fn not_found(why: String) -> GrammarError {
    GrammarError::new(ErrorKind::NotFound, "/finding", why)
}

fn layer(id: &str, mark: Mark, panel: u32) -> Layer {
    let mut l = Layer::new(mark, panel);
    l.id = Some(id.to_owned());
    l
}

fn at(panel: f64, px: f64) -> Position {
    Position::Panel(PanelPos { panel, px })
}

fn color(token: &str) -> Option<ColorSpec> {
    Some(ColorSpec::Constant(token.to_owned()))
}

fn panel(id: u32, name: &str, weight: f64) -> Panel {
    Panel {
        id: Some(id),
        name: Some(name.to_owned()),
        weight: Some(weight),
        y: Scale {
            kind: ScaleKind::Linear,
            domain: None,
            id: Some(format!("y{id}")),
        },
    }
}

/// The finding's first fact and the bar series it sits on, with its bars.
fn fact_and_bars(
    finding: &str,
    p: &dyn DataProvider,
) -> Result<(Fact, String, Vec<Bar>), GrammarError> {
    let f = p
        .finding(finding)
        .ok_or_else(|| not_found(format!("finding `{finding}` does not exist")))?;
    let fid = f
        .facts
        .first()
        .ok_or_else(|| not_found(format!("finding `{finding}` has no facts to present")))?;
    let fact = p
        .fact(fid)
        .ok_or_else(|| not_found(format!("fact `{fid}` does not exist")))?
        .into_owned();
    let bs = match p.series_kind(&fact.series) {
        Some(SeriesKind::Bars) => Some(fact.series.clone()),
        Some(SeriesKind::Values) => p.input_bars(&fact.series),
        _ => None,
    }
    .ok_or_else(|| not_found(format!("series `{}` has no bars", fact.series)))?;
    let bars = match p.bars(&bs) {
        Some(b) if !b.is_empty() => b.into_owned(),
        _ => return Err(not_found(format!("series `{bs}` has no bars"))),
    };
    Ok((fact, bs, bars))
}

/// Whether the capsule's claim is in force now: the glyph's validity (the engine's), else
/// the headline pieces', else the story's stage.
fn in_force(finding: &str, p: &dyn DataProvider) -> bool {
    if let Some(v) = p.glyph(finding).and_then(|g| g.validity) {
        return v == Validity::Valid;
    }
    if let Some(hp) = p.headline_parts(finding) {
        return hp.validity == Validity::Valid;
    }
    !p.capsule(finding)
        .and_then(|c| c.story.as_ref().map(|s| s.stage.is_closed()))
        .unwrap_or(false)
}

/// The face view (`faces::FaceViewFn`).
///
/// # Errors
/// Unknown finding/fact, a finding without facts, or a series without bars.
pub fn view(
    look: &Look,
    finding: &str,
    max_points: u32,
    p: &dyn DataProvider,
    id: &str,
) -> Result<View, GrammarError> {
    let (fact, bs, bars) = fact_and_bars(finding, p)?;
    let n = bars.len() as u64;
    let last = n - 1;
    let event = fact.bar.min(last);
    // The event with its run-up; the window ends at the newest bar ("now").
    let budget = u64::from(max_points.max(MIN_BARS as u32));
    let from = event
        .saturating_sub(BEFORE)
        .min(last.saturating_sub(MIN_BARS - 1))
        .max(last.saturating_sub(budget - 1).min(event.saturating_sub(4)));
    let live = in_force(finding, p);
    let (ink, mark_ink) = if live {
        ("text", "accent")
    } else {
        ("muted", "muted")
    };

    // Panel 0: the headline.
    let mut head = layer("headline", Mark::Text, 0);
    head.finding = Some(finding.to_owned());
    head.encoding.text = Some("headline".to_owned());
    head.encoding.x = Some(at(0.0, 0.0));
    head.encoding.y = Some(at(0.0, 0.0));
    head.encoding.size = Some(SizeSpec::Constant(HEAD_PX));
    head.encoding.color = color("text");

    // Panel 1: the glyph strip, the glyph over the event's bar.
    let mut glyph = layer("glyph", Mark::Glyph, 1);
    glyph.finding = Some(finding.to_owned());
    glyph.encoding.y = Some(at(1.0, -GLYPH_PX));
    glyph.encoding.size = Some(SizeSpec::Constant(GLYPH_PX));

    // Panel 2: the picture.
    // The price trail: a dot per bar (a point mark has no legend and no last-value tag,
    // so the picture carries no price number of its own).
    let mut line = layer("price", Mark::Point, 2);
    line.series = Some(bs.clone());
    line.encoding.y = Some(Position::Name("c".to_owned()));
    line.encoding.color = color(ink);
    line.encoding.size = Some(SizeSpec::Constant(if live { 3.5 } else { 3.0 }));
    let mut moment = layer("moment", Mark::Rule, 2);
    moment.fact = Some(fact.id.clone());
    moment.encoding.color = color(mark_ink);
    moment.encoding.width = Some(1.0);
    let mut layers = vec![head, glyph, line, moment];
    if fact.series == bs {
        let mut taken: Vec<String> = layers.iter().filter_map(|l| l.id.clone()).collect();
        for mut l in depth::evidence_layers(look, &fact, 2, &mut taken) {
            l.encoding.color = color(mark_ink);
            if l.mark != Mark::Band {
                l.encoding.width = Some(1.5);
            }
            layers.push(l);
        }
    }
    let mut dot = layer("dot", Mark::Point, 2);
    dot.fact = Some(fact.id.clone());
    dot.encoding.y = Some(Position::Ref(DataRef {
        series: Some(bs),
        field: Some("c".to_owned()),
    }));
    dot.encoding.color = color(mark_ink);
    dot.encoding.size = Some(SizeSpec::Constant(8.0));
    layers.push(dot);

    let i = |b: u64| usize::try_from(b).unwrap_or(0);
    #[allow(clippy::cast_precision_loss)]
    let x = Scale {
        kind: ScaleKind::Time,
        domain: Some([bars[i(from)].t as f64, bars[i(last)].t as f64]),
        id: Some("x".to_owned()),
    };
    Ok(View {
        id: id.to_owned(),
        title: p.symbol(&fact.series).or_else(|| Some(String::new())),
        display_tz: None,
        theme: None,
        x,
        y: Vec::new(),
        panels: vec![
            panel(0, "head", HEAD_H),
            panel(1, "mark", MARK_H),
            panel(2, "pic", PIC_H),
        ],
        layers,
        emphasis: Vec::new(),
    })
}

/// Longest headline (SPEC §2(b)).
const MAX_CHARS: usize = 80;

/// The headline template (`faces::HeadlineFn`): what happened, when it last moved, whether
/// it still holds; then how similar events turned out before. The longest variant that
/// fits in 80 characters: the ledger's lead-in goes first, then the few-cases note is
/// shortened; the ledger itself is never dropped while the sentence fits without it.
#[must_use]
pub fn headline(hp: &HeadlineParts) -> String {
    let lead = format!(
        "{}, {}: {}.",
        capitalize(&hp.event),
        hp.moved,
        hp.validity_phrase
    );
    let candidates = ledger_variants(hp);
    for l in &candidates {
        let s = format!("{lead} {l}");
        if s.chars().count() <= MAX_CHARS {
            return s;
        }
    }
    lead
}

/// The ledger sentence, longest first.
fn ledger_variants(hp: &HeadlineParts) -> Vec<String> {
    let (none, before, few, short) = match hp.lang {
        Language::Tr => (
            "Henüz kapanmış benzeri yok.",
            "Benzerleri",
            "(az örnek)",
            "(az)",
        ),
        Language::En => (
            "No similar case has closed yet.",
            "Before:",
            "(few cases)",
            "(few)",
        ),
    };
    if hp.ledger_n == Some(0) {
        let short_none = match hp.lang {
            Language::Tr => "Benzeri yok.",
            Language::En => "No record yet.",
        };
        return vec![none.to_owned(), short_none.to_owned()];
    }
    let ls = hp.ledger_short.trim();
    let compact = ls.replace(few, short);
    let (past, slash) = match hp.lang {
        Language::Tr => ("Geçmişte", compact.clone()),
        Language::En => ("Past:", compact.replacen(" of ", "/", 1)),
    };
    vec![
        format!("{before} {ls}."),
        format!("{past} {ls}."),
        format!("{past} {compact}."),
        format!("{past} {slash}."),
        format!("{}.", capitalize(&slash)),
    ]
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => {
            let up: String = match f {
                'i' => "İ".to_owned(),
                _ => f.to_uppercase().collect(),
            };
            up + c.as_str()
        }
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qp_schema::capsule::Stage;

    fn hp(
        lang: Language,
        event: &str,
        moved: &str,
        validity: &str,
        ledger: &str,
        n: Option<u32>,
    ) -> HeadlineParts {
        HeadlineParts {
            lang,
            instrument: "SYN03".into(),
            event: event.into(),
            direction_word: String::new(),
            when: moved.into(),
            moved: moved.into(),
            validity_phrase: validity.into(),
            ledger_short: ledger.into(),
            stage: Stage::Born,
            validity: Validity::Valid,
            redacted: n.is_none(),
            ledger_n: n,
            few_cases: n.is_none_or(|n| n < 20),
        }
    }

    /// The long cases of the S8 set stay within 80 characters and keep the ledger.
    #[test]
    fn headline_fits_and_keeps_the_ledger() {
        let cases = [
            hp(
                Language::En,
                "downside breakout",
                "on 10 February",
                "did not play out",
                "4 of 5 played out (few cases)",
                Some(5),
            ),
            hp(
                Language::En,
                "downward reversal",
                "yesterday at 17:00",
                "still holds",
                "9 of 17 played out (few cases)",
                Some(17),
            ),
            hp(
                Language::En,
                "squeeze (direction open)",
                "yesterday",
                "still holds",
                "2 of 7 played out (few cases)",
                Some(7),
            ),
            hp(
                Language::Tr,
                "sıkışma (yön belirsiz)",
                "dün",
                "hâlâ geçerli",
                "7'de 2 tuttu (az örnek)",
                Some(7),
            ),
            hp(
                Language::En,
                "unusual money flow",
                "yesterday",
                "still holds",
                "no record yet",
                Some(0),
            ),
            hp(
                Language::Tr,
                "aşağı kırılım",
                "birkaç gün önce",
                "hâlâ geçerli",
                "az örnek",
                None,
            ),
        ];
        for c in &cases {
            let h = headline(c);
            assert!(h.chars().count() <= MAX_CHARS, "{h}");
            assert!(
                h.contains('/')
                    || h.contains(" of ")
                    || h.contains("'")
                    || h.contains("yok")
                    || h.contains("record")
                    || h.contains("closed")
                    || h.contains("az"),
                "{h}"
            );
            assert_eq!(h, headline(c));
        }
        assert_eq!(
            headline(&cases[3]),
            "Sıkışma (yön belirsiz), dün: hâlâ geçerli. Benzerleri 7'de 2 tuttu (az örnek)."
        );
        // A redacted reader's pieces have no digits; the template adds none.
        assert!(!headline(&cases[5]).chars().any(|c| c.is_ascii_digit()));
    }

    #[allow(clippy::too_many_lines)]
    /// The face compiles with both alphabets of the human test: one glyph over the
    /// event's bar, the headline from the template, no legend or last-value tag.
    #[test]
    fn compiles_with_both_alphabets() {
        use crate::{compile, CompileOptions, MemProvider};
        use qp_schema::capsule::{Direction, GlyphPhase, GlyphSpec, MeaningKind};
        use qp_schema::scene::Prim;
        use qp_schema::{Author, Fact, Finding, Provenance};
        let json = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/bars/sample.json"
        ))
        .expect("sample bars");
        let bars = qp_schema::parse_bar_series(&json)
            .expect("valid sample")
            .bars()
            .expect("bars")
            .to_vec();
        let prov = Provenance {
            author: Author::Engine {
                name: "t".into(),
                version: "0".into(),
            },
            at: 0,
            based_on: Vec::new(),
            approved_by: None,
        };
        let n = bars.len();
        let fact = Fact {
            id: "f".into(),
            kind: "breakout.range.up".into(),
            series: "px".into(),
            bar: (n - 3) as u64,
            t: bars[n - 3].t,
            text: "breakout".into(),
            evidence: std::collections::BTreeMap::new(),
            evidence_ref: None,
            meaning: None,
            provenance: prov.clone(),
        };
        let finding = Finding {
            id: "fnd".into(),
            claim: "breakout".into(),
            facts: vec!["f".into()],
            confidence: 0.5,
            invalidation: qp_schema::Condition::Text { text: "-".into() },
            provenance: prov,
        };
        let spec = GlyphSpec {
            kind: MeaningKind::Breakout,
            direction: Direction::Up,
            strength: 0.7,
            confidence: Some(0.6),
            phase: GlyphPhase::Final,
            freshness: 1.0,
            confirmations: 1,
            server: false,
            validity: Some(Validity::Valid),
        };
        let parts = hp(
            Language::Tr,
            "yukarı kırılım",
            "dün",
            "hâlâ geçerli",
            "30'da 18 tuttu",
            Some(30),
        );
        let want = headline(&parts);
        let p = MemProvider::new()
            .with_bars("px", bars, Some("SYN03"), Some("1d"))
            .with_fact(fact)
            .with_finding(finding)
            .with_headline("fnd", "The long sentence.")
            .with_glyph("fnd", spec)
            .with_headline_parts("fnd", parts);
        let v = view(&qp_look::look::R4, "fnd", 120, &p, "capsule-fnd").unwrap();
        for alphabet in qp_glyph::ALPHABETS {
            let mut o = CompileOptions::new(390, HEIGHT);
            o.face = Some("fotolay".into());
            o.alphabet = Some(alphabet.into());
            let sc = compile(&v, &p, &o).unwrap();
            let glyphs: usize = sc
                .panels
                .iter()
                .flat_map(|pn| &pn.layers)
                .filter(|l| l.mark == Mark::Glyph)
                .map(|l| l.items.len())
                .sum();
            assert_eq!(glyphs, 1, "{alphabet}");
            let texts: Vec<String> = sc
                .prims()
                .filter_map(|x| match x {
                    Prim::Text { text, .. } => Some(text.clone()),
                    _ => None,
                })
                .collect();
            let all = texts.join(" ");
            assert!(all.contains(&want), "{all}");
            assert!(!all.contains("fiyat") && !all.contains("price"), "{all}");
        }
    }
}
