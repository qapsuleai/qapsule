//! M2 registry: every realizer the harness can measure.
//!
//! A rule-based alternative lives in its own crate under `rounds/R3/experiments/M2/<alt>/`
//! and exports a type implementing `qp_capsule::Realizer`. To register it:
//!
//! 1. in `Cargo.toml`, under the "M2 alternatives" comment:
//!    `m2-<alt> = { path = "../M2/<alt>" }`
//! 2. in [`all`] below, one line: `Box::new(m2_<alt>::Realizer),`
//!
//! A language-model alternative needs no crate: it writes its sentences to
//! `rounds/R3/experiments/M2/<alt>/sentences.json` (format in `file_realizer`) and adds one
//! line: `Box::new(FileRealizer::new("<alt>", alt_file("<alt>"))),`
//!
//! The `--alt` name is the realizer's `name()`.

use std::path::PathBuf;

use qp_capsule::Realizer;

#[allow(unused_imports)]
use super::file_realizer::FileRealizer;

/// `rounds/R3/experiments/M2/<alt>/sentences.json`.
#[must_use]
pub fn alt_file(alt: &str) -> PathBuf {
    crate::io::default_root()
        .join("M2")
        .join(alt)
        .join("sentences.json")
}

/// Every registered realizer.
#[must_use]
pub fn all() -> Vec<Box<dyn Realizer>> {
    vec![
        // Self-test only, not a candidate.
        Box::new(super::control::Control),
        // --- M2 alternatives: one line each, e.g.
        // Box::new(m2_foo::Realizer),
        // Box::new(FileRealizer::new("llm_foo", alt_file("llm_foo"))),
        Box::new(FileRealizer::new("llmslot", alt_file("llmslot"))),
        // Second independent generation of `llmslot` (same prompt), for run-to-run checks only.
        Box::new(FileRealizer::new(
            "llmslot_run2",
            crate::io::default_root()
                .join("M2")
                .join("llmslot")
                .join("sentences.run2.json"),
        )),
    ]
}

/// The realizer named `name`.
///
/// # Errors
/// Unknown name (the message lists the known ones).
pub fn by_name(name: &str) -> Result<Box<dyn Realizer>, String> {
    let mut v = all();
    let names: Vec<&'static str> = v.iter().map(|r| r.name()).collect();
    match names.iter().position(|n| *n == name) {
        Some(i) => Ok(v.swap_remove(i)),
        None => Err(format!(
            "unknown M2 realizer `{name}` (registered: {})",
            names.join(", ")
        )),
    }
}
