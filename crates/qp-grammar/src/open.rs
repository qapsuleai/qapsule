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
//! (R5/S) A design may also read the reader's context ([`OpenCtx`]: an opened question,
//! the language, the device's check, the frame; [`OpenDesign::view_ctx`]) and have its
//! own tap rule at the capsule depth ([`OpenDesign::tap`]; `nobra`'s question cards).
//!
//! **Adding a design (M20):** `mod <name>;` with `src/open/<name>.rs` and one line in
//! [`OPENS`]. Every colour, type size and spacing comes from the look.

use qp_schema::capsule::{LedgerStat, Trust, Validity};
use qp_schema::grammar::View;
use qp_schema::protocol::{CapsuleVoice, HitInfo, Language, OpenDepth};
use qp_schema::Fact;

use crate::error::GrammarError;
use crate::look::Look;
use crate::provider::DataProvider;

pub mod nobra;
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

/// (R5/M20 `nobra`) The device's check as a design with questions reads it: `None` is
/// the authority's claim ("verified by the server", [`DeviceCheck::Server`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Check {
    /// Its own data does not cover it yet ("checking on this device").
    Checking,
    /// Checked on its own data ("checked on this device").
    Device,
    /// Its own data contradicts it ("sources differ").
    Differs,
}

impl Check {
    /// Every state, for layouts that must not depend on which one is shown.
    pub const ALL: [Option<Check>; 4] = [
        None,
        Some(Check::Checking),
        Some(Check::Device),
        Some(Check::Differs),
    ];

    /// Its name in a text field (`server` for `None`).
    #[must_use]
    pub fn name(c: Option<Check>) -> &'static str {
        match c {
            None => "server",
            Some(Check::Checking) => "checking",
            Some(Check::Device) => "device",
            Some(Check::Differs) => "differs",
        }
    }

    /// The state a text field names (`server` → `None`).
    #[must_use]
    pub fn parse(s: &str) -> Option<Option<Check>> {
        Some(match s {
            "server" => None,
            "checking" => Some(Check::Checking),
            "device" => Some(Check::Device),
            "differs" => Some(Check::Differs),
            _ => return None,
        })
    }

    /// (R5/S) The same state from a [`DeviceCheck`] (one concept, two forms).
    #[must_use]
    pub fn of(d: DeviceCheck) -> Option<Check> {
        match d {
            DeviceCheck::Server => None,
            DeviceCheck::Checking => Some(Check::Checking),
            DeviceCheck::Device => Some(Check::Device),
            DeviceCheck::Differs => Some(Check::Differs),
        }
    }
}

/// (R5/M20 `nobra`) What a design reads besides the data: the opened question, the
/// reader's language, the device's check and the frame the scene is compiled at (CSS px).
#[derive(Debug, Clone, PartialEq)]
pub struct OpenCtx<'a> {
    /// The opened question (`ViewState::ask`), if any.
    pub ask: Option<&'a str>,
    /// The reader's language.
    pub lang: Language,
    /// The device's check (`None`: the authority's claim).
    pub check: Option<Check>,
    /// Frame `[width, height]`.
    pub frame: [u32; 2],
}

/// (R5/M20 `nobra`) What a tap on the capsule depth does under a design.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenTap {
    /// Open a depth (the chart).
    Open(OpenDepth),
    /// Set the opened question (`None`: close it); the depth stays.
    Ask(Option<String>),
    /// Nothing opens (the tap may still focus a bar, as today's chart tap).
    Nothing,
}

/// (R5/S) A capsule-depth view that reads the reader's context ([`OpenCtx`]).
pub type ViewCtxFn =
    fn(&Look, &str, u32, &dyn DataProvider, &str, &OpenCtx<'_>) -> Result<View, GrammarError>;

/// (R5/S) A design's tap rule at the capsule depth.
pub type TapFn = fn(&View, &HitInfo, Option<&str>) -> OpenTap;

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
#[derive(Clone, Copy)]
pub struct OpenDesign {
    /// Setting name (`[a-z][a-z0-9_-]*`, not an ordinary word).
    pub name: &'static str,
    /// One line: the idea.
    pub note: &'static str,
    /// The capsule depth's view.
    pub view: ViewFn,
    /// (R5/S) The capsule depth's view with the reader's context; when set, it is the
    /// design's view ([`capsule_view`]) and `view` is not used.
    pub view_ctx: Option<ViewCtxFn>,
    /// (R5/S) What a tap at the capsule depth does; `None`: the reference rule
    /// ([`crate::depth::tap_opens`]).
    pub tap: Option<TapFn>,
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

impl std::fmt::Debug for OpenDesign {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenDesign")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

/// The reference: today's open capsule.
const R4_OPEN: OpenDesign = OpenDesign {
    name: "r4",
    note: "R4 reference: the capsule card (glyph, sentence, track record, mini price line); any tap opens the chart",
    view: crate::depth::capsule_view_in,
    view_ctx: None,
    tap: None,
    height: None,
    text: None,
    scene_text: None,
};

/// Designs a connection may choose; the first is the default. (R5/S) M20's first two:
/// `tesir` (chosen), `nobra`.
pub static OPENS: &[OpenDesign] = &[R4_OPEN, tesir::DESIGN, nobra::DESIGN];

/// The default design (`ViewState.open` absent).
pub const DEFAULT_OPEN: &str = "r4";

/// (R5/S) The open capsule design the presentation shell asks for (M20's pick, `tesir`;
/// `rounds/R5/experiments/M20/RESULTS.md`); the protocol default stays [`DEFAULT_OPEN`].
pub const PRESENTED_OPEN: &str = "tesir";

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

/// Whether a design other than the reference builds the capsule depth.
#[must_use]
pub fn is_styled(name: Option<&str>) -> bool {
    by_name(name).name != DEFAULT_OPEN
}

/// The capsule-depth view by design `name` (`None` or `r4`: the reference card), with
/// the reader's context for a design that reads it.
///
/// # Errors
/// The design's own errors.
pub fn capsule_view(
    name: Option<&str>,
    look: &Look,
    finding: &str,
    max_points: u32,
    p: &dyn DataProvider,
    id: &str,
    ctx: &OpenCtx<'_>,
) -> Result<View, GrammarError> {
    let d = by_name(name);
    match d.view_ctx {
        Some(f) => f(look, finding, max_points, p, id, ctx),
        None => (d.view)(look, finding, max_points, p, id),
    }
}

/// What a tap at open depth `d` does by design `name`: at the capsule depth a design's own
/// rule, elsewhere (and for a design without one) [`crate::depth::tap_opens`].
#[must_use]
pub fn tap(
    name: Option<&str>,
    d: OpenDepth,
    view: Option<&View>,
    hit: Option<&HitInfo>,
    ask: Option<&str>,
) -> OpenTap {
    match (d, by_name(name).tap, view, hit) {
        (OpenDepth::Capsule, Some(f), Some(v), Some(h)) => f(v, h, ask),
        (OpenDepth::Capsule, Some(_), _, _) => OpenTap::Nothing,
        _ => crate::depth::tap_opens(d, hit.and_then(|h| h.item.as_ref()))
            .map_or(OpenTap::Nothing, OpenTap::Open),
    }
}

/// The design's hint line for open depth `d` (the page shows it under the open capsule;
/// `None`: the page's own hint).
#[must_use]
pub fn hint(name: Option<&str>, d: OpenDepth, lang: Language) -> Option<&'static str> {
    if d != OpenDepth::Capsule {
        return None;
    }
    match by_name(name).name {
        "nobra" => Some(nobra::hint(lang)),
        _ => None,
    }
}

/// The design's question cards as text for a reader **without a scene** at open depth `d`
/// (no data of the instrument, A32; the page lists them under the sentence); empty for a
/// design without them or another depth.
#[must_use]
pub fn text_rows(
    name: Option<&str>,
    d: OpenDepth,
    item: &qp_schema::protocol::CapsuleItem,
    lang: Language,
) -> Vec<nobra::AskRow> {
    match (d, by_name(name).name) {
        (OpenDepth::Capsule, "nobra") => nobra::text_rows(item, lang, None),
        _ => Vec::new(),
    }
}

/// The text of `nobra`'s text field (`ask/…`, written into a view's `text` channel and
/// resolved at compile time from the provider, so a view carries no text of its own), or
/// `None` when the field is not one.
#[must_use]
pub fn field_text(field: &str, finding: &str, p: &dyn DataProvider) -> Option<String> {
    let rest = field.strip_prefix("ask/")?;
    nobra::field_text(rest, finding, p)
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
        assert!(is_open("tesir") && is_open("nobra"));
        assert!(!is_styled(None) && !is_styled(Some("r4")) && is_styled(Some("nobra")));
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

    #[test]
    fn check_names_round_trip() {
        for c in Check::ALL {
            assert_eq!(Check::parse(Check::name(c)), Some(c));
        }
        assert_eq!(Check::parse("x"), None);
    }
}
