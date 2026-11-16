//! The alphabets every outcome style must work with (SPEC §5): `siluet` (the product
//! alphabet, `qp_grammar::glyph::siluet`) and `rozet` (the R3/M1 finalist, drawn from its
//! experiment crate `rounds/R3/experiments/M1/rozet/`, unchanged).

use qp_grammar::glyph::siluet::Siluet;
use qp_grammar::glyph::Alphabet;

/// Alphabet names, report order. The persona sheets use the first (the product alphabet)
/// unless asked otherwise.
pub const ALPHABETS: [&str; 2] = ["siluet", "rozet"];

static SILUET: Siluet = Siluet;
static ROZET: m1_rozet::Alphabet = m1_rozet::Alphabet;

/// The alphabet named `name`.
///
/// # Errors
/// Unknown name.
pub fn by_name(name: &str) -> Result<&'static dyn Alphabet, String> {
    match name {
        "siluet" => Ok(&SILUET),
        "rozet" => Ok(&ROZET),
        _ => Err(format!(
            "unknown alphabet `{name}` (known: {})",
            ALPHABETS.join(", ")
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_match() {
        for n in ALPHABETS {
            assert_eq!(by_name(n).unwrap().name(), n);
        }
        assert!(by_name("iz").is_err());
    }
}
