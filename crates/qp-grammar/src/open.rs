//! (R5/M20) Open capsule designs: what the capsule depth shows (the open capsule and its
//! evidence in small), chosen by name (`style` gesture `open`, `ViewState.open`).
//!
//! A design builds the capsule depth's view from the same inputs as today's card
//! ([`crate::depth::capsule_view_in`]): the finding, its facts, the capsule's story, the
//! reader's voice and the reader's provider (redacted evidence for a reader without the
//! data entitlement, K33). It may state its own frame height ([`OpenDesign::height`]): the
//! shell core then opens the capsule depth at that height and the page scrolls the layer.
//! For a reader who gets no scene (no data entitlement for the bars, A32) a design may
//! write the open layer's text itself ([`OpenDesign::text`]).
//!
//! The reference **`r4`** is today's open capsule (the card, the open layer's headline):
//! the default, absent from the view state; every golden and transcript stays as it was.
//!
//! **Adding a design (M20):** `mod <name>;` with `src/open/<name>.rs` and one line in
//! [`OPENS`]. Every colour, type size and spacing comes from the look.

use qp_schema::capsule::{LedgerStat, Trust, Validity};
use qp_schema::grammar::View;
use qp_schema::protocol::{CapsuleVoice, Language};
use qp_schema::Fact;

use crate::error::GrammarError;
use crate::look::Look;
use crate::provider::DataProvider;

pub mod tesir;

/// (R5/M20, K48) What the device found when it checked the open capsule on its own data
/// (the viewer's check; `qp_session::viewer::TrustState`). The authority has none.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceCheck {
    /// The authority's settled claim; the device did not check it ("verified by the
    /// server").
    Server,
    /// The device's data does not cover it yet ("checking on this device").
    Checking,
    /// Checked on the device's own data ("checked on this device").
    Device,
    /// The device's data contradicts it ("sources differ").
    Differs,
}

/// The capsule depth's view of design `name`.
pub type ViewFn = fn(
    look: &Look,
    finding: &str,
    max_points: u32,
    p: &dyn DataProvider,
    id: &str,
) -> Result<View, GrammarError>;

/// What a design's text column reads for a reader without a scene (A32): the shell core
/// fills it from the feed item and the opening (the same reader rule as the scene).
#[derive(Debug, Clone, Copy)]
pub struct TextInput<'a> {
    /// The reader's language.
    pub lang: Language,
    /// The capsule's sentence for this reader (the feed item's).
    pub headline: &'a str,
    /// The voice surfaces for this reader, if a voice is chosen.
    pub voice: Option<&'a CapsuleVoice>,
    /// The finding's facts as the authority gave them to this reader (redacted evidence
    /// for a reader without the entitlement).
    pub facts: &'a [Fact],
    /// The capsule's trust path.
    pub trust: Trust,
    /// The device's check, if a viewer checked it.
    pub check: Option<DeviceCheck>,
    /// The ledger at birth.
    pub ledger: LedgerStat,
    /// Validity now.
    pub validity: Validity,
}

/// One open capsule design.
#[derive(Debug, Clone, Copy)]
pub struct OpenDesign {
    /// Setting name (`[a-z][a-z0-9_-]*`, not an ordinary word).
    pub name: &'static str,
    /// The capsule depth's view.
    pub view: ViewFn,
    /// The capsule depth's frame height in `look` (CSS px); `None`: the surface's scene
    /// frame (today's).
    pub height: Option<fn(&Look) -> u32>,
    /// The open layer's text for a reader without a scene (lines separated by `\n`);
    /// `None`: today's (the headline).
    pub text: Option<fn(&TextInput<'_>) -> String>,
    /// The open layer's text above a scene; `None`: today's (the headline). A design
    /// whose scene writes the headline itself returns an empty text.
    pub scene_text: Option<fn() -> String>,
}

/// The reference: today's open capsule.
const R4_OPEN: OpenDesign = OpenDesign {
    name: "r4",
    view: crate::depth::capsule_view_in,
    height: None,
    text: None,
    scene_text: None,
};

/// Designs a connection may choose; the first is the default.
pub static OPENS: &[OpenDesign] = &[R4_OPEN, tesir::DESIGN];

/// The default design (`ViewState.open` absent).
pub const DEFAULT_OPEN: &str = "r4";

/// Registered names, in registry order.
#[must_use]
pub fn names() -> Vec<&'static str> {
    OPENS.iter().map(|d| d.name).collect()
}

/// Whether `name` is a registered design.
#[must_use]
pub fn is_open(name: &str) -> bool {
    OPENS.iter().any(|d| d.name == name)
}

/// The design named `name` (`None` or unknown: the default).
#[must_use]
pub fn by_name(name: Option<&str>) -> &'static OpenDesign {
    let n = name.unwrap_or(DEFAULT_OPEN);
    OPENS.iter().find(|d| d.name == n).unwrap_or(&OPENS[0])
}

/// The capsule depth's frame height of design `name` in `look`, if it states one.
#[must_use]
pub fn height_in(look: &Look, name: Option<&str>) -> Option<u32> {
    by_name(name).height.map(|h| h(look))
}

/// A text field of a finding row that an open design reads (`open.*`), for the capsule
/// presenting `finding`; `None` for any other field (the compiler's own fields).
#[must_use]
pub fn text(field: &str, finding: &str, p: &dyn DataProvider) -> Option<String> {
    if !field.starts_with("open.") {
        return None;
    }
    tesir::field(field, finding, p)
}

/// (K49) The glossary entry a design attaches to one of its text layers as a whole (the
/// layer's panel is the tap target; the first line is underlined), searched before the
/// voice's terms; `None` for any other layer.
#[must_use]
pub fn layer_term(
    layer: &str,
    finding: &str,
    p: &dyn DataProvider,
) -> Option<qp_schema::protocol::GlossEntry> {
    tesir::layer_term(layer, finding, p)
}

/// Whether the voice's glossary terms are searched in layer `layer` (a design's layer
/// with its own term is not).
#[must_use]
pub fn voice_terms_in(layer: &str) -> bool {
    !tesir::owns_term(layer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry() {
        assert_eq!(OPENS[0].name, DEFAULT_OPEN);
        assert!(is_open("tesir"));
        assert!(!is_open("nope"));
        assert_eq!(by_name(Some("nope")).name, DEFAULT_OPEN);
        assert_eq!(height_in(&crate::look::R4, None), None);
        assert!(height_in(&crate::look::R4, Some("tesir")).is_some());
        for n in names() {
            assert!(n
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_'));
        }
    }
}
