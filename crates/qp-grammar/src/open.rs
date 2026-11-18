//! R5/M20: open capsule designs (the `style` gesture's `open`; `?open=<name>` in the demo).
//!
//! An open capsule design decides what the **capsule depth** shows when a capsule is
//! opened from the feed: the order of the information, where the glossary and the track
//! record sit, how trust ("verified by the server" / "checked on this device", K48) and a
//! reader without the data entitlement (K33/K34, A32) are told, and how the reader goes on
//! to the chart. It builds a [`View`] from the provider's data (the same rule on the
//! authority, for `tap_at`/`hit`, and on the device, for painting: one scene description,
//! `ref` and `canvas` paint it alike) and says what a tap on that view does
//! ([`OpenDesign::tap`]).
//!
//! The reference `r4` is today's capsule card ([`crate::depth::capsule_view_in`]): every
//! tap opens the chart. It is the default; a design is chosen by name.
//!
//! **Adding an alternative (M20):** `mod <name>;` with `src/open/<name>.rs` and one line in
//! [`OPENS`]. Sizes, colours and spacing come from the look ([`Look`]).

use qp_schema::grammar::View;
use qp_schema::protocol::{HitInfo, Language, OpenDepth};

use crate::error::{ErrorKind, GrammarError};
use crate::look::Look;
use crate::provider::DataProvider;

pub mod nobra;

/// The device's own check of the open capsule (K48; the viewer's trust state beside the
/// authority's settled claim). `None` in [`OpenCtx::check`]: the authority's claim only
/// ("verified by the server"), which is also what the authority itself compiles.
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
}

/// What a design needs besides the data: the opened question, the reader's language,
/// the device's check and the frame the scene is compiled at (CSS px).
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

/// What a tap on the capsule depth does under a design.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenTap {
    /// Open a depth (the chart).
    Open(OpenDepth),
    /// Set the opened question (`None`: close it); the depth stays.
    Ask(Option<String>),
    /// Nothing opens (the tap may still focus a bar, as today's chart tap).
    Nothing,
}

/// One open capsule design.
#[derive(Clone, Copy)]
pub struct OpenDesign {
    /// Name (`[a-z][a-z0-9_-]*`).
    pub name: &'static str,
    /// One line: the idea.
    pub note: &'static str,
    /// The capsule-depth view; `None`: the reference card ([`crate::depth::capsule_view_in`]).
    #[allow(clippy::type_complexity)]
    pub view: Option<
        fn(&Look, &str, u32, &dyn DataProvider, &str, &OpenCtx<'_>) -> Result<View, GrammarError>,
    >,
    /// What a tap at the capsule depth does, from the view it was drawn from, the hit and
    /// the opened question; `None`: the reference rule ([`crate::depth::tap_opens`]).
    #[allow(clippy::type_complexity)]
    pub tap: Option<fn(&View, &HitInfo, Option<&str>) -> OpenTap>,
}

impl std::fmt::Debug for OpenDesign {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenDesign")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

/// The registered designs, the default first.
pub static OPENS: &[OpenDesign] = &[
    OpenDesign {
        name: "r4",
        note: "R4 reference: the capsule card (glyph, sentence, track record, mini price line); any tap opens the chart",
        view: None,
        tap: None,
    },
    // --- M20 alternatives below this line ------------------------------------------------
    OpenDesign {
        name: "nobra",
        note: "question cards: one short answer per reader question, details on touch; the evidence geometry inside \"what does it rest on?\"",
        view: Some(nobra::view),
        tap: Some(nobra::tap),
    },
];

/// The default design.
pub const DEFAULT_OPEN: &str = "r4";

/// The names, the default first.
#[must_use]
pub fn names() -> Vec<&'static str> {
    OPENS.iter().map(|d| d.name).collect()
}

/// Whether `name` is a registered design.
#[must_use]
pub fn is_open(name: &str) -> bool {
    OPENS.iter().any(|d| d.name == name)
}

/// The design named `name` (`None`: the default).
///
/// # Errors
/// An unknown name (`invalid`, `/open`).
pub fn by_name(name: Option<&str>) -> Result<&'static OpenDesign, GrammarError> {
    let n = name.unwrap_or(DEFAULT_OPEN);
    OPENS.iter().find(|d| d.name == n).ok_or_else(|| {
        GrammarError::new(
            ErrorKind::Invalid,
            "/open",
            format!("unknown open design `{n}` (known: {})", names().join(", ")),
        )
    })
}

/// Whether a design other than the reference builds the capsule depth.
#[must_use]
pub fn is_styled(name: Option<&str>) -> bool {
    by_name(name).is_ok_and(|d| d.view.is_some())
}

/// The capsule-depth view by design `name` (`None` or `r4`: the reference card).
///
/// # Errors
/// An unknown name, or the design's own errors.
pub fn capsule_view(
    name: Option<&str>,
    look: &Look,
    finding: &str,
    max_points: u32,
    p: &dyn DataProvider,
    id: &str,
    ctx: &OpenCtx<'_>,
) -> Result<View, GrammarError> {
    match by_name(name)?.view {
        Some(f) => f(look, finding, max_points, p, id, ctx),
        None => crate::depth::capsule_view_in(look, finding, max_points, p, id),
    }
}

/// What a tap at open depth `d` does by design `name`: at the capsule depth a design's own
/// rule, elsewhere (and for the reference) [`crate::depth::tap_opens`].
#[must_use]
pub fn tap(
    name: Option<&str>,
    d: OpenDepth,
    view: Option<&View>,
    hit: Option<&HitInfo>,
    ask: Option<&str>,
) -> OpenTap {
    let rule = by_name(name).ok().and_then(|x| x.tap);
    match (d, rule, view, hit) {
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
    if d != OpenDepth::Capsule || !is_styled(name) {
        return None;
    }
    match name {
        Some("nobra") => Some(nobra::hint(lang)),
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
    match (d, name) {
        (OpenDepth::Capsule, Some("nobra")) => nobra::text_rows(item, lang, None),
        _ => Vec::new(),
    }
}

/// The text of a design's text field (`ask/…`, written into a view's `text` channel and
/// resolved at compile time from the provider, so a view carries no text of its own), or
/// `None` when the field is not a design's.
#[must_use]
pub fn field_text(field: &str, finding: &str, p: &dyn DataProvider) -> Option<String> {
    let rest = field.strip_prefix("ask/")?;
    nobra::field_text(rest, finding, p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_is_well_formed() {
        assert_eq!(OPENS[0].name, DEFAULT_OPEN);
        assert!(OPENS[0].view.is_none() && OPENS[0].tap.is_none());
        let mut seen = std::collections::BTreeSet::new();
        for d in OPENS {
            assert!(seen.insert(d.name), "duplicate {}", d.name);
            assert!(d.name.chars().next().unwrap().is_ascii_lowercase());
            assert!(d
                .name
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_'));
        }
        assert!(is_open("nobra") && !is_open("zzz"));
        assert!(by_name(Some("zzz")).is_err());
        assert!(!is_styled(None) && !is_styled(Some("r4")) && is_styled(Some("nobra")));
    }

    #[test]
    fn check_names_round_trip() {
        for c in Check::ALL {
            assert_eq!(Check::parse(Check::name(c)), Some(c));
        }
        assert_eq!(Check::parse("x"), None);
    }
}
