//! M2 — sentence realizer (SPEC "M2").
//!
//! - [`situations`]: the 72 seeded situations + K26 with hidden truth;
//! - [`lexicon`]: TR/EN lexicons and the time-correctness classifier;
//! - [`metrics`]: automatic metrics (`harness m2 measure`): time correctness, determinism,
//!   leakage, invented numbers, cost;
//! - [`sheets`] / [`score`]: comprehension test (`harness m2 sheets`, `harness m2 score`);
//! - [`file_realizer`]: reads pre-generated sentences (LLM alternatives) through the same
//!   `Realizer` interface;
//! - [`registry`]: the realizers the harness knows (alternatives add one line);
//! - [`control`]: the self-test realizer, **not a candidate**.

pub mod control;
pub mod file_realizer;
pub mod lexicon;
pub mod metrics;
pub mod registry;
pub mod score;
pub mod sheets;
pub mod situations;

use std::path::{Path, PathBuf};

use qp_capsule::Lang;

/// Both languages, report order.
pub const LANGS: [Lang; 2] = [Lang::Tr, Lang::En];

/// `tr` / `en`.
#[must_use]
pub fn lang_name(l: Lang) -> &'static str {
    match l {
        Lang::Tr => "tr",
        Lang::En => "en",
    }
}

/// Parses `tr` / `en`.
///
/// # Errors
/// Anything else.
pub fn parse_lang(s: &str) -> Result<Lang, String> {
    match s {
        "tr" => Ok(Lang::Tr),
        "en" => Ok(Lang::En),
        _ => Err(format!("unknown language `{s}` (tr, en)")),
    }
}

/// `<root>/M2/raw/<alt>/`.
#[must_use]
pub fn raw_dir(root: &Path, alt: &str) -> PathBuf {
    root.join("M2").join("raw").join(alt)
}
