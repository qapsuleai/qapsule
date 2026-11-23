//! (RF1/G2b) The product's design registry: the named designs behind the compiler's five
//! call points ([`DESIGNS`]) and the options that carry it ([`options`]).

use qp_look::error::GrammarError;

use crate::compile::CompileOptions;
use crate::designs::{Designs, HeadlineFn};
use crate::{faces, open};

/// The product's designs: the faces ([`faces::FACES`]) and the open capsule designs
/// ([`open::OPENS`]).
pub static DESIGNS: Designs = Designs {
    name: "qp-designs",
    headline_template,
    field_text: open::field_text,
    open_text: open::text,
    layer_term: open::layer_term,
    voice_terms_in: open::voice_terms_in,
};

/// Options for a `width` x `height` frame compiled with the product's designs, everything
/// else default (what `CompileOptions::new` was before RF1).
#[must_use]
pub fn options(width: u32, height: u32) -> CompileOptions {
    CompileOptions::with_designs(width, height, &DESIGNS)
}

/// The headline template of the face named in a face scene; no face, no template (the
/// compiler asked only when a face was named).
fn headline_template(name: Option<&str>) -> Result<Option<HeadlineFn>, GrammarError> {
    match name {
        Some(n) => faces::headline_template(Some(n)),
        None => Ok(None),
    }
}
