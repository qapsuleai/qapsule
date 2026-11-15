//! Meaning glyphs (R3): the contract an alphabet implements. An alphabet draws one
//! [`GlyphSpec`] into a square box as scene primitives; the `glyph` mark
//! (`docs/GRAMMAR.md` §18) and the M1 measurement harness both go through
//! [`Alphabet::draw`], so every alternative is rendered and measured by the same code path.
//!
//! Rules: deterministic; IEEE basic operations only (`crate::detmath` for anything else);
//! all colours from the [`Theme`] (no hard-coded colours, so light/dark and colour-blind
//! simulation apply uniformly); every primitive inside the box.
//!
//! The product uses one alphabet, [`alphabet`]: `siluet` (R3/M1). The human test compares
//! `siluet` with `rozet`; switching is the one line in [`alphabet`].

pub mod siluet;

use qp_schema::capsule::GlyphSpec;
use qp_schema::scene::Prim;

use crate::theme::Theme;

/// A square box in pixels: top-left corner and side length.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlyphBox {
    /// Left.
    pub x: f32,
    /// Top.
    pub y: f32,
    /// Side length (16, 24, 32, … px).
    pub size: f32,
}

/// A meaning-glyph alphabet (M1).
pub trait Alphabet {
    /// Name used in reports.
    fn name(&self) -> &'static str;
    /// Draw `spec` into `bx` with `theme`'s colours.
    fn draw(&self, spec: &GlyphSpec, bx: GlyphBox, theme: &Theme) -> Vec<Prim>;
}

/// The product alphabet: the one the `glyph` mark draws with. **Changing the alphabet is
/// this one line** (R3/M1 chose `siluet`; the human test may bring `rozet`).
#[must_use]
pub fn alphabet() -> &'static dyn Alphabet {
    &siluet::Siluet
}
