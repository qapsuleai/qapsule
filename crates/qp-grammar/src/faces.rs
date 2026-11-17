//! R4/H-M8910: capsule face designs (M8; `docs/PROTOCOL.md` §4.6.2, the `style` gesture's
//! `face`).
//!
//! A face design is a grammar rule that builds the [`View`] of a capsule's face in the feed
//! (open depth `sentence`) and **states its own height** (CSS px at the phone width): the
//! shell lays faces out at the height the core reads from here, it never computes one.
//! The authority (`qp-session` `interact.rs`, hit testing and openings) and the shell's
//! replica (`qp-wasm` `shell.rs` `face_scene`) both call [`face_view`] by name, so they
//! compile the same scene.
//!
//! The reference `r3` is the R3 capsule card (§18.4, [`depth::capsule_view`]) at 240 px,
//! the T5 shell's face. `card` is its earlier name (accepted, means `r3`).
//!
//! **Adding an alternative (M8):** in this file only — write the view function below the
//! `M8 alternatives` marker (or declare `mod <name>;` there and put it in
//! `src/faces/<name>.rs`), then add one line to [`FACES`]. Nothing else changes; the
//! settings page and the harness list the names from here.
//!
//! **Headline template (R4/H-FP, SPEC E5):** a design may also write its own headline
//! ([`FaceDesign::headline`], a [`HeadlineFn`] over [`HeadlineParts`]). In that design's
//! face scene — and only there: the compile option [`crate::CompileOptions::face`] names
//! it — the finding's `headline` text field is the template's output (the harness
//! measures the headline from that field); without a template, or when the provider has
//! no pieces for the finding, it is the capsule's sentence as before. The capsule's
//! protocol `headline`, the open capsule, the chart and every transcript are untouched.

use qp_schema::View;

use crate::depth;
use crate::look::Look;
use crate::error::{ErrorKind, GrammarError};
use crate::provider::{DataProvider, HeadlineParts};

/// Builds a face's view: `(look, finding, max_points, provider, view id) -> View`.
/// `max_points` is the connection profile's point budget; the view id is
/// `capsule-<finding>`. (R5/T3) Sizes, gaps and the glyph's place come from `look`.
pub type FaceViewFn = fn(
    look: &Look,
    finding: &str,
    max_points: u32,
    p: &dyn DataProvider,
    id: &str,
) -> Result<View, GrammarError>;

/// A face design's headline template (R4/H-FP): the headline sentence from the reader's
/// headline pieces, in the pieces' language ([`HeadlineParts::lang`]). Deterministic;
/// at most 80 characters (SPEC §2(b)); for a redacted reader ([`HeadlineParts::redacted`])
/// it must not add digits (the pieces have none).
pub type HeadlineFn = fn(&HeadlineParts) -> String;

/// One face design.
#[derive(Debug, Clone, Copy)]
pub struct FaceDesign {
    /// Setting name (`[a-z][a-z0-9_-]*`; not an ordinary word, it must not appear in a
    /// reader's prompt).
    pub name: &'static str,
    /// (R5/T3) Face height in CSS px at the phone width (390) in a look; 120..=844.
    pub height: fn(&Look) -> u32,
    /// The view rule.
    pub view: FaceViewFn,
    /// The headline template; `None`: the capsule's sentence (`DataProvider::headline`).
    pub headline: Option<HeadlineFn>,
}

/// The default face design.
pub const DEFAULT_FACE: &str = "r3";

/// (R4/S2) The face the presentation shell asks for when the reader chose none (M8's
/// pick, `docs/FACE.md`, `rounds/R4/experiments/M8/RESULTS.md`). The shell core sends it
/// with the `style` gesture at start, like `feeds::PRESENTED_FEED`; the protocol default
/// ([`DEFAULT_FACE`]) does not change, so bare connections, transcripts and existing
/// goldens stay byte-identical.
pub const PRESENTED_FACE: &str = "vadrel";

/// Earlier names still accepted: `(alias, name)`.
pub const ALIASES: [(&str, &str); 1] = [("card", "r3")];

// --- M8 alternatives: view functions (or `mod <name>;`) go below this line -------------
mod tavrik;

mod vadrel;

mod fotolay;

/// Face designs a connection may choose; the first is the default. (R4/S2) After the
/// reference, M8's ranking: the first two (`vadrel`, `fotolay`) go to the human test.
pub static FACES: &[FaceDesign] = &[
    FaceDesign {
        name: "r3",
        height: |_| 240,
        view: depth::capsule_view_in,
        headline: None,
    },
    // --- M8 alternatives: one line each
    FaceDesign {
        name: "vadrel",
        height: vadrel::height,
        view: vadrel::view,
        headline: Some(vadrel::headline),
    },
    FaceDesign {
        name: "fotolay",
        height: |_| fotolay::HEIGHT,
        view: fotolay::view,
        headline: Some(fotolay::headline),
    },
    FaceDesign {
        name: "tavrik",
        height: |_| tavrik::HEIGHT,
        view: tavrik::view,
        headline: Some(tavrik::headline),
    },
];

/// Designs known besides [`FACES`]: none (unit tests register an example design with a
/// headline template here; it never enters the registry).
#[cfg(not(test))]
const UNLISTED: &[FaceDesign] = &[];
#[cfg(test)]
const UNLISTED: &[FaceDesign] = &[tests::TEMPLATED];

fn known() -> impl Iterator<Item = &'static FaceDesign> {
    FACES.iter().chain(UNLISTED.iter())
}

/// Registered names, in registry order.
#[must_use]
pub fn names() -> Vec<&'static str> {
    FACES.iter().map(|f| f.name).collect()
}

/// The canonical name of `name` (an alias resolves), if known.
#[must_use]
pub fn canonical(name: &str) -> Option<&'static str> {
    let n = ALIASES
        .iter()
        .find(|(a, _)| *a == name)
        .map_or(name, |(_, n)| *n);
    known().find(|f| f.name == n).map(|f| f.name)
}

/// Whether `name` is a known face design (or alias).
#[must_use]
pub fn is_face(name: &str) -> bool {
    canonical(name).is_some()
}

/// The design named `name` (`None`: the default).
///
/// # Errors
/// An unknown name (`invalid`, `/face`).
pub fn by_name(name: Option<&str>) -> Result<&'static FaceDesign, GrammarError> {
    let n = name.unwrap_or(DEFAULT_FACE);
    let c = canonical(n).ok_or_else(|| {
        GrammarError::new(
            ErrorKind::Invalid,
            "/face",
            format!("unknown face `{n}` (known: {})", names().join(", ")),
        )
    })?;
    Ok(known().find(|f| f.name == c).unwrap_or(&FACES[0]))
}

/// The face height of design `name` (`None` or unknown: the default's) in the `r4` look.
#[must_use]
pub fn height(name: Option<&str>) -> u32 {
    height_in(&crate::look::R4, name)
}

/// (R5/T3) The face height of design `name` in `look` (what the shell lays faces out at).
#[must_use]
pub fn height_in(look: &Look, name: Option<&str>) -> u32 {
    (by_name(name).unwrap_or(&FACES[0]).height)(look)
}

/// The headline template of design `name` (`None`: the default's; `r3` has none).
///
/// # Errors
/// An unknown name (`invalid`, `/face`).
pub fn headline_template(name: Option<&str>) -> Result<Option<HeadlineFn>, GrammarError> {
    Ok(by_name(name)?.headline)
}

/// The face's `headline` text for `finding`: design `template`'s output over the
/// provider's pieces when both exist, else the capsule's sentence
/// ([`DataProvider::headline`]).
#[must_use]
pub fn headline_text(
    template: Option<HeadlineFn>,
    finding: &str,
    p: &dyn DataProvider,
) -> Option<String> {
    template
        .and_then(|t| p.headline_parts(finding).map(|hp| t(&hp)))
        .or_else(|| p.headline(finding))
}

/// The face view of `finding` by design `face` (`None`: the default).
///
/// # Errors
/// An unknown face name or the design's own errors.
pub fn face_view(
    face: Option<&str>,
    finding: &str,
    max_points: u32,
    p: &dyn DataProvider,
    id: &str,
) -> Result<View, GrammarError> {
    face_view_in(&crate::look::R4, face, finding, max_points, p, id)
}

/// (R5/T3) [`face_view`] in `look`.
///
/// # Errors
/// As [`face_view`].
pub fn face_view_in(
    look: &Look,
    face: Option<&str>,
    finding: &str,
    max_points: u32,
    p: &dyn DataProvider,
    id: &str,
) -> Result<View, GrammarError> {
    (by_name(face)?.view)(look, finding, max_points, p, id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{compile, CompileOptions, MemProvider};
    use qp_schema::capsule::{Stage, Validity};
    use qp_schema::protocol::Language;
    use qp_schema::scene::Prim;
    use qp_schema::{Author, Fact, Finding, Provenance};

    /// An example headline template (tests only; never in [`FACES`]).
    fn example_headline(hp: &HeadlineParts) -> String {
        match hp.lang {
            Language::Tr => format!(
                "{} {}, {} · {}",
                hp.instrument, hp.event, hp.when, hp.validity_phrase
            ),
            Language::En => format!(
                "{} {}, {} · {}",
                hp.instrument, hp.event, hp.when, hp.validity_phrase
            ),
        }
    }

    pub(super) const TEMPLATED: FaceDesign = FaceDesign {
        name: "zz-test-headline",
        height: |_| 240,
        view: depth::capsule_card_view_in,
        headline: Some(example_headline),
    };

    fn hp(lang: Language) -> HeadlineParts {
        let tr = lang == Language::Tr;
        HeadlineParts {
            lang,
            instrument: "SYN03".into(),
            event: if tr {
                "yukarı kırılım"
            } else {
                "upside breakout"
            }
            .into(),
            direction_word: if tr { "yukarı" } else { "up" }.into(),
            when: if tr { "bugün" } else { "today" }.into(),
            moved: if tr { "bugün" } else { "today" }.into(),
            validity_phrase: if tr { "hâlâ geçerli" } else { "still holds" }.into(),
            ledger_short: if tr {
                "30'da 18 tuttu"
            } else {
                "18 of 30 played out"
            }
            .into(),
            stage: Stage::Born,
            validity: Validity::Valid,
            redacted: false,
            ledger_n: Some(30),
            few_cases: false,
        }
    }

    fn provider(parts: Option<HeadlineParts>) -> MemProvider {
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
        let fact = Fact {
            id: "f".into(),
            kind: "breakout.range.up".into(),
            series: "px".into(),
            bar: 180,
            t: bars[180].t,
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
        let p = MemProvider::new()
            .with_bars("px", bars, Some("SYN03"), Some("1d"))
            .with_fact(fact)
            .with_finding(finding)
            .with_headline("fnd", "The capsule's long sentence.");
        match parts {
            Some(hp) => p.with_headline_parts("fnd", hp),
            None => p,
        }
    }

    /// The card view's sentence and price line (the test provider has no capsule story).
    fn card(p: &MemProvider) -> View {
        let mut v = depth::capsule_card_view("fnd", 60, p, "capsule-fnd").unwrap();
        v.layers
            .retain(|l| matches!(l.id.as_deref(), Some("sentence" | "line")));
        v
    }

    /// The scene's texts joined by spaces (a long line wraps into several).
    fn texts(face: Option<&str>, p: &MemProvider) -> String {
        let v = card(p);
        let mut o = CompileOptions::new(390, 240);
        o.face = face.map(str::to_owned);
        compile(&v, p, &o)
            .unwrap()
            .prims()
            .filter_map(|x| match x {
                Prim::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// (R4/H-FP) A face design's headline template writes the finding's `headline` text
    /// in its face scene only; no face, `r3` or a provider without pieces: the sentence.
    #[test]
    fn headline_template_applies_in_its_face_scene_only() {
        let sentence = "The capsule's long sentence.";
        for lang in [Language::Tr, Language::En] {
            let p = provider(Some(hp(lang)));
            let want = example_headline(&hp(lang));
            assert!(want.chars().count() <= 80, "{want}");
            let t = texts(Some("zz-test-headline"), &p);
            assert!(t.contains(&want), "{t}");
            assert!(!t.contains(sentence), "{t}");
            for face in [None, Some("r3"), Some("card")] {
                let t = texts(face, &p);
                assert!(t.contains(sentence), "{face:?} {t}");
                assert!(!t.contains(&want));
            }
            // Deterministic.
            assert_eq!(
                texts(Some("zz-test-headline"), &p),
                texts(Some("zz-test-headline"), &p)
            );
        }
        let t = texts(Some("zz-test-headline"), &provider(None));
        assert!(t.contains(sentence), "{t}");
        let v = card(&provider(None));
        let mut o = CompileOptions::new(390, 240);
        o.face = Some("no-such-face".into());
        assert!(compile(&v, &provider(None), &o).is_err());
        assert!(headline_template(None).unwrap().is_none());
        assert!(!names().contains(&"zz-test-headline"));
    }

    #[test]
    fn registry_is_well_formed() {
        assert_eq!(FACES[0].name, DEFAULT_FACE);
        let mut seen = Vec::new();
        for f in FACES {
            assert!(!seen.contains(&f.name), "duplicate face {}", f.name);
            seen.push(f.name);
            assert!(f.name.starts_with(|c: char| c.is_ascii_lowercase()));
            assert!(f
                .name
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-'));
            for l in crate::look::LOOKS {
                let h = (f.height)(l);
                assert!((120..=844).contains(&h), "{} height in {}", f.name, l.name);
            }
        }
        assert_eq!(canonical("card"), Some("r3"));
        assert_eq!(height(None), 240);
        assert_eq!(height(Some("card")), 240);
        assert!(by_name(Some("nope")).is_err());
        // (R4/S2) the presentation's face is registered and is not the protocol default.
        assert!(is_face(PRESENTED_FACE));
        assert_ne!(PRESENTED_FACE, DEFAULT_FACE);
        assert_eq!(&names()[..3], ["r3", "vadrel", "fotolay"]);
    }
}
