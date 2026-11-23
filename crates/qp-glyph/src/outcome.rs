//! Outcome styles (R4/M11): how a glyph's **outcome mark** — the channel that tells
//! "realized" (gerçekleşti) from "failed" (tutmadı) on a closed capsule's glyph
//! (`docs/CONCEPTS.md` "Sonuç işareti") — is drawn, selectable **by name**.
//!
//! An [`OutcomeStyle`] wraps the alphabet: the `glyph` mark (`docs/GRAMMAR.md` §18.1) calls
//! [`OutcomeStyle::draw`] with the product alphabet, so one style works with every
//! alphabet (`siluet`, `rozet`, …) and can draw differently per alphabet
//! ([`Alphabet::name`]). The selection comes from the compile options
//! (`CompileOptions::outcome`, a view setting like the theme override); absent = the
//! default [`DEFAULT_OUTCOME`].
//!
//! [`R3`] (`"r3"`) is the reference and the default: it is **exactly** the alphabet's own
//! drawing (siluet: a solid disc with a check / a bare cross in the corner; rozet: a
//! thick bezel / one slash), so every existing golden image stays byte-identical.
//!
//! # Adding a style (M11 alternatives) — this file only
//!
//! 1. Write a unit type implementing [`OutcomeStyle`] below the `--- M11 alternatives ---`
//!    marker (or in a new module `glyph/outcome/<name>.rs` declared here). `name()` is the
//!    setting value (`[a-z][a-z0-9_-]*`, not an ordinary word: it must never appear in a
//!    reader prompt). `draw()` must keep the rules of `glyph` (deterministic, IEEE basic
//!    operations only, theme colours only, every primitive inside the box) and should
//!    change only closed phases (`Realized`, `Failed`; `Retracted` optionally); for open
//!    phases return `alphabet.draw(spec, bx, theme)` unchanged. Expired stories reach the
//!    glyph as `Failed` (`GlyphPhase: From<Stage>`), so "expired" has no separate mark.
//!    A style typically draws the open-glyph look with [`open_look`] and adds its own
//!    marks, or post-processes `alphabet.draw(spec, …)`.
//! 2. `legend(alphabet)`: the style's one-sentence explanation for readers (Turkish, at
//!    most [`LEGEND_MAX_WORDS`] words; shown only in the M11 `legend` reading condition;
//!    it must not contain the style's name).
//! 3. Add one line to [`STYLES`].
//!
//! Nothing else changes: the M11 harness (`rounds/R4/experiments/harness/m11/`) measures
//! every name in [`STYLES`].

use qp_schema::capsule::{GlyphPhase, GlyphSpec};
use qp_schema::scene::Prim;

use super::{Alphabet, GlyphBox};
use qp_look::error::{ErrorKind, GrammarError};
use qp_look::theme::Theme;

/// The default outcome style (the R3 drawing; `ViewState.outcome` and
/// `CompileOptions::outcome` absent).
pub const DEFAULT_OUTCOME: &str = "r3";

/// (R4/S1) The style the presentation shell asks for when the reader chose none (M11's
/// pick, `rounds/R4/experiments/M11/RESULTS.md`). Sent by the shell core with the `style`
/// gesture at start; [`DEFAULT_OUTCOME`] stays the protocol's and the compiler's default.
pub const PRESENTED_OUTCOME: &str = "tg5";

/// Word limit of [`OutcomeStyle::legend`] (SPEC M11 (b): one sentence, ≤ 30 words).
pub const LEGEND_MAX_WORDS: usize = 30;

/// How the outcome mark of a glyph is drawn (M11).
pub trait OutcomeStyle: Sync {
    /// Setting name (`CompileOptions::outcome`).
    fn name(&self) -> &'static str;
    /// Draw `spec` into `bx` with `alphabet` and `theme`. For open phases a style returns
    /// the alphabet's own drawing.
    fn draw(
        &self,
        alphabet: &dyn Alphabet,
        spec: &GlyphSpec,
        bx: GlyphBox,
        theme: &Theme,
    ) -> Vec<Prim>;
    /// One-sentence explanation (Turkish, ≤ [`LEGEND_MAX_WORDS`] words) for `alphabet`
    /// (`siluet`, `rozet`, …), shown only in the M11 `legend` reading condition.
    fn legend(&self, alphabet: &str) -> &'static str;
}

/// The reference: the alphabet's own R3 outcome drawing, unchanged.
#[derive(Debug, Clone, Copy, Default)]
pub struct R3;

impl OutcomeStyle for R3 {
    fn name(&self) -> &'static str {
        "r3"
    }

    fn draw(
        &self,
        alphabet: &dyn Alphabet,
        spec: &GlyphSpec,
        bx: GlyphBox,
        theme: &Theme,
    ) -> Vec<Prim> {
        alphabet.draw(spec, bx, theme)
    }

    fn legend(&self, alphabet: &str) -> &'static str {
        match alphabet {
            "rozet" => {
                "Kalın çerçeveli rozet gerçekleşti demek; üstü eğik çizgiyle çizili rozet \
                 tutmadı demek; ikisi de yoksa olay hâlâ açık."
            }
            _ => {
                "Gri işaretin sağ alt köşesinde dolu daire içinde onay işareti gerçekleşti, \
                 çıplak çarpı tutmadı demek; köşe işareti yoksa olay hâlâ açık."
            }
        }
    }
}

// --- M11 alternatives: one type each below this line, then one line in `STYLES` ---

mod muhrak;
pub use muhrak::Muhrak;
mod tg5;
pub use tg5::Tg5;

/// Every outcome style, by setting name. **Alternatives add one line here.**
pub static STYLES: &[&dyn OutcomeStyle] = &[
    &R3, // --- M11 alternatives: one line each, e.g. `&MyStyle,`
    &Muhrak, &Tg5,
];

/// The names in [`STYLES`], in order.
#[must_use]
pub fn names() -> Vec<&'static str> {
    STYLES.iter().map(|s| s.name()).collect()
}

/// The style named `name`.
///
/// # Errors
/// `invalid` at `(options)/outcome` for an unknown name (the hint lists the known ones).
pub fn by_name(name: &str) -> Result<&'static dyn OutcomeStyle, GrammarError> {
    STYLES
        .iter()
        .copied()
        .find(|s| s.name() == name)
        .ok_or_else(|| {
            GrammarError::new(
                ErrorKind::Invalid,
                "(options)/outcome",
                format!(
                    "unknown outcome style `{name}` (known: {})",
                    names().join(", ")
                ),
            )
        })
}

/// The default style ([`DEFAULT_OUTCOME`]).
#[must_use]
pub fn default_style() -> &'static dyn OutcomeStyle {
    &R3
}

/// The glyph as `alphabet` draws it while the story is still open (phase `Final`, same
/// validity unless it is `Closed`, which becomes absent): the base a style adds its
/// outcome marks to.
#[must_use]
pub fn open_look(
    alphabet: &dyn Alphabet,
    spec: &GlyphSpec,
    bx: GlyphBox,
    theme: &Theme,
) -> Vec<Prim> {
    let mut s = spec.clone();
    s.phase = GlyphPhase::Final;
    if s.validity == Some(qp_schema::capsule::Validity::Closed) {
        s.validity = None;
    }
    alphabet.draw(&s, bx, theme)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::siluet::Siluet;
    use qp_schema::capsule::{Direction, MeaningKind};

    #[test]
    fn registry_names_are_unique_and_valid() {
        let n = names();
        assert_eq!(n[0], DEFAULT_OUTCOME);
        assert!(by_name(PRESENTED_OUTCOME).is_ok());
        let mut s = n.clone();
        s.sort_unstable();
        s.dedup();
        assert_eq!(s.len(), n.len(), "duplicate outcome style name");
        for name in n {
            let b = name.as_bytes();
            assert!(b[0].is_ascii_lowercase(), "{name}");
            assert!(
                b.iter().all(|c| c.is_ascii_lowercase()
                    || c.is_ascii_digit()
                    || *c == b'_'
                    || *c == b'-'),
                "{name}"
            );
            let st = by_name(name).unwrap();
            for a in ["siluet", "rozet"] {
                let l = st.legend(a);
                assert!(!l.is_empty());
                assert!(
                    l.split_whitespace().count() <= LEGEND_MAX_WORDS,
                    "{name}/{a}"
                );
                assert!(!l.to_lowercase().contains(name), "{name}/{a}");
            }
        }
        assert!(by_name("nope").is_err());
    }

    #[test]
    fn r3_is_the_alphabet() {
        let bx = GlyphBox {
            x: 1.0,
            y: 2.0,
            size: 24.0,
        };
        for phase in [
            GlyphPhase::Candidate,
            GlyphPhase::Final,
            GlyphPhase::Realized,
            GlyphPhase::Failed,
            GlyphPhase::Retracted,
        ] {
            let spec = GlyphSpec {
                kind: MeaningKind::Breakout,
                direction: Direction::Up,
                strength: 0.6,
                confidence: Some(0.6),
                phase,
                freshness: 1.0,
                confirmations: 2,
                server: false,
                validity: None,
            };
            let t = Theme::light();
            assert_eq!(R3.draw(&Siluet, &spec, bx, &t), Siluet.draw(&spec, bx, &t));
            let open = open_look(&Siluet, &spec, bx, &t);
            let fin = GlyphSpec {
                phase: GlyphPhase::Final,
                ..spec
            };
            assert_eq!(open, Siluet.draw(&fin, bx, &t));
        }
    }
}
