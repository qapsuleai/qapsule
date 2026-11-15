//! M1 — meaning-glyph alphabet (SPEC "M1").
//!
//! - [`glyphset`]: the core set `K` (13 kind × direction) and the channel variation pairs;
//! - [`render`]: draws one glyph through `Alphabet::draw` and the project CPU rasterizer
//!   (`qp_render`, the golden-image path);
//! - [`color`]: sRGB ↔ linear, colour-vision simulation (Machado 2009, severity 1.0, in
//!   linear RGB), CIELAB (D65), ΔE76;
//! - [`distinct`]: computed distinctness (`harness m1 measure`);
//! - [`sheets`]: reading-test sheets and hidden answer keys (`harness m1 sheets`);
//! - [`score`]: reader scoring (`harness m1 score`);
//! - [`registry`]: the alphabets the harness knows (alternatives add one line);
//! - [`control`]: the self-test alphabet, **not a candidate**.

pub mod color;
pub mod control;
pub mod distinct;
pub mod glyphset;
pub mod registry;
pub mod render;
pub mod score;
pub mod sheets;

use std::path::{Path, PathBuf};

/// `<root>/M1/raw/<alt>/`.
#[must_use]
pub fn raw_dir(root: &Path, alt: &str) -> PathBuf {
    root.join("M1").join("raw").join(alt)
}
