//! (RF1/G2b) The design registry ([`Designs`]; `docs/CONCEPTS.md` "tasarım kaydı"): the
//! table of functions through which the compiler reaches the named designs, and the types
//! those call points need. The compiler does not know the design side: a caller hands it a
//! table in [`crate::CompileOptions`] (the product's is `qp_designs::DESIGNS`, through
//! `qp_designs::options`). The table covers exactly the five places the compiler asked a
//! design before RF1 (`rounds/RF1/PLAN.md` §1.2, §3.1): a face's headline template
//! (`compile::scene`), an open design's text field and own field (`layer::prepare`), a
//! design layer's own glossary entry and whether the voice's terms are searched in a layer
//! (`terms::mark`). A function-pointer table, not a trait object: options stay `Clone`,
//! and equality and `Debug` go by the table's name.

use std::fmt;

use qp_look::error::{ErrorKind, GrammarError};
use qp_schema::protocol::GlossEntry;

use crate::provider::{DataProvider, HeadlineParts};

/// A face design's headline template (R4/H-FP): the headline sentence from the reader's
/// headline pieces, in the pieces' language ([`HeadlineParts::lang`]). Deterministic;
/// at most 80 characters (SPEC §2(b)); for a redacted reader ([`HeadlineParts::redacted`])
/// it must not add digits (the pieces have none).
pub type HeadlineFn = fn(&HeadlineParts) -> String;

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

/// The headline template of the face design named in a face scene
/// ([`crate::CompileOptions::face`]); `None` (not a face scene): no template.
///
/// # Errors
/// An unknown face name (`invalid`, `/face`).
pub type HeadlineTemplateFn = fn(Option<&str>) -> Result<Option<HeadlineFn>, GrammarError>;

/// A text field of a finding row read from a design: `(field, finding, provider)`; `None`
/// when the field is not the design's.
pub type FieldTextFn = fn(&str, &str, &dyn DataProvider) -> Option<String>;

/// (K49) The glossary entry a design attaches to one of its text layers as a whole:
/// `(layer, finding, provider)`; `None` for any other layer.
pub type LayerTermFn = fn(&str, &str, &dyn DataProvider) -> Option<GlossEntry>;

/// Whether the voice's glossary terms are searched in a layer (a design's layer with its
/// own term is not).
pub type VoiceTermsFn = fn(&str) -> bool;

/// The design registry: the compiler's five call points into the named designs.
#[derive(Clone, Copy)]
pub struct Designs {
    /// The table's name: its identity for `PartialEq` and `Debug`.
    pub name: &'static str,
    /// A face's headline template (`compile::scene`).
    pub headline_template: HeadlineTemplateFn,
    /// An open design's text field (`ask/…`; `layer::prepare`).
    pub field_text: FieldTextFn,
    /// An open design's own field (`open.*`; `layer::prepare`).
    pub open_text: FieldTextFn,
    /// A design layer's own glossary entry (`terms::mark`).
    pub layer_term: LayerTermFn,
    /// Whether the voice's terms are searched in a layer (`terms::mark`).
    pub voice_terms_in: VoiceTermsFn,
}

impl fmt::Debug for Designs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Designs").field(&self.name).finish()
    }
}

impl PartialEq for Designs {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

/// No named designs: no face is known, no design writes a text field or owns a term. For
/// the compiler's own tests and benches; the product compiles with `qp_designs::DESIGNS`.
pub static NONE: Designs = Designs {
    name: "none",
    headline_template: no_headline_template,
    field_text: no_text,
    open_text: no_text,
    layer_term: no_term,
    voice_terms_in: all_layers,
};

fn no_headline_template(name: Option<&str>) -> Result<Option<HeadlineFn>, GrammarError> {
    match name {
        None => Ok(None),
        Some(n) => Err(GrammarError::new(
            ErrorKind::Invalid,
            "/face",
            format!("unknown face design `{n}` (no designs registered)"),
        )),
    }
}

fn no_text(_field: &str, _finding: &str, _p: &dyn DataProvider) -> Option<String> {
    None
}

fn no_term(_layer: &str, _finding: &str, _p: &dyn DataProvider) -> Option<GlossEntry> {
    None
}

fn all_layers(_layer: &str) -> bool {
    true
}
