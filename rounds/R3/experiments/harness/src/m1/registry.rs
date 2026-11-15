//! M1 registry: every alphabet the harness can measure, with its legend.
//!
//! An alternative lives in its own crate under `rounds/R3/experiments/M1/<alt>/` and
//! exports a type implementing `qp_grammar::glyph::Alphabet` plus a `LEGEND` string (at
//! most 120 words; read only in the `legend` reading condition). To register it:
//!
//! 1. in `Cargo.toml`, under the "M1 alternatives" comment:
//!    `m1-<alt> = { path = "../M1/<alt>" }`
//! 2. in [`all`] below, one line:
//!    `entry(Box::new(m1_<alt>::Alphabet), m1_<alt>::LEGEND),`
//!
//! The `--alt` name is the alphabet's `name()`. Nothing else in the harness changes.

use qp_grammar::glyph::Alphabet;

/// An alphabet with its legend (the alternative's own ≤ 120-word explanation).
pub struct M1Entry {
    /// The alphabet.
    pub alphabet: Box<dyn Alphabet>,
    /// Its legend text (shown to readers only in the `legend` condition).
    pub legend: &'static str,
}

/// Pairs an alphabet with its legend.
#[must_use]
pub fn entry(alphabet: Box<dyn Alphabet>, legend: &'static str) -> M1Entry {
    M1Entry { alphabet, legend }
}

/// Every registered alphabet.
#[must_use]
pub fn all() -> Vec<M1Entry> {
    vec![
        // Self-test only, not a candidate.
        entry(Box::new(super::control::Control), super::control::LEGEND),
        // --- M1 alternatives: one line each, e.g.
        // entry(Box::new(m1_foo::Alphabet), m1_foo::LEGEND),
        entry(Box::new(m1_siluet::Alphabet), m1_siluet::LEGEND),
    ]
}

/// The alphabet named `name`.
///
/// # Errors
/// Unknown name (the message lists the known ones).
pub fn by_name(name: &str) -> Result<M1Entry, String> {
    let mut v = all();
    let names: Vec<&'static str> = v.iter().map(|e| e.alphabet.name()).collect();
    match names.iter().position(|n| *n == name) {
        Some(i) => Ok(v.swap_remove(i)),
        None => Err(format!(
            "unknown M1 alphabet `{name}` (registered: {})",
            names.join(", ")
        )),
    }
}
