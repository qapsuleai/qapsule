//! Blinding (SPEC §0.2, §0.3): screenshots are named by a seeded code instead of the
//! design name (the mapping is in `raw/key.json`, never copied to a reader), and every
//! prompt is checked against a forbidden-word list before it is written and again before
//! it is sent.

use crate::rng::Rng;

/// Default salt of the codes.
pub const DEFAULT_SALT: u64 = 8910;

/// A seeded code: 8 hex digits from a hash of `parts` (hex digits never spell a word).
#[must_use]
pub fn code(salt: u64, parts: &str) -> String {
    let mut r = Rng::derive(salt, &format!("m8910|{parts}"));
    format!("{:08x}", r.next_u64() & 0xffff_ffff)
}

/// Whole words that must not appear in a prompt (case-insensitive), besides every
/// registered design name: experiment, branch and agent vocabulary.
pub const FORBIDDEN_WORDS: [&str; 25] = [
    "variant",
    "varyant",
    "alternatif",
    "alternatifi",
    "alternative",
    "alternatives",
    "harness",
    "düzenek",
    "m8",
    "m9",
    "m10",
    "m11",
    "m8910",
    "claude",
    "r3",
    "r4",
    "style",
    "stil",
    "face",
    "feed",
    "transition",
    "outcome",
    "card",
    "s8",
    "key",
];

/// Substrings that must not appear anywhere (branch names `r4/m8a-…`, identifiers
/// `alt-…`, `alt_…`). The bare Turkish word "alt" ("bottom") is ordinary and allowed
/// (as in the M11 harness, README "Sapmalar").
pub const FORBIDDEN_SUBSTRINGS: [&str; 9] = [
    "r4/", "r3/", "m8a", "m9a", "m10a", "alt-", "alt_", "/alt", "variant",
];

/// Every registered design name of the product's registries (M8 faces, M9 layouts, M10
/// transitions, M11 outcome styles) and the alphabets' names are not forbidden (the
/// alphabet is not a design under test).
#[must_use]
pub fn design_names() -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    for n in qp_grammar::faces::names()
        .into_iter()
        .chain(qp_grammar::feeds::names())
        .chain(qp_grammar::transitions::names())
        .chain(qp_grammar::glyph::outcome::names())
    {
        let n = n.to_lowercase();
        if !v.contains(&n) {
            v.push(n);
        }
    }
    for (a, _) in qp_grammar::faces::ALIASES {
        v.push(a.to_owned());
    }
    v
}

/// Lower-cased words of `text` (split on anything that is not alphanumeric, `_` or `-`).
fn words(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '-'))
        .filter(|w| !w.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Forbidden words and substrings found in `text` (empty = clean).
#[must_use]
pub fn forbidden_hits(text: &str, extra: &[&str]) -> Vec<String> {
    let mut banned: Vec<String> = FORBIDDEN_WORDS.iter().map(|s| (*s).to_owned()).collect();
    banned.extend(design_names());
    banned.extend(extra.iter().map(|s| s.to_lowercase()));
    let low = text.to_lowercase();
    let mut hits: Vec<String> = Vec::new();
    for w in words(text) {
        let parts = std::iter::once(w.clone())
            .chain(w.split(['-', '_']).map(str::to_owned))
            .collect::<Vec<_>>();
        for p in parts {
            if banned.contains(&p) && !hits.contains(&p) {
                hits.push(p);
            }
        }
    }
    for s in FORBIDDEN_SUBSTRINGS {
        if low.contains(s) && !hits.iter().any(|h| h == s) {
            hits.push(s.to_owned());
        }
    }
    hits
}

/// Fails with the hits if `text` is not clean.
///
/// # Errors
/// A forbidden word or substring.
pub fn check(what: &str, text: &str) -> Result<(), String> {
    let h = forbidden_hits(text, &[]);
    if h.is_empty() {
        Ok(())
    } else {
        Err(format!("{what}: forbidden words: {}", h.join(", ")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_seeded_and_opaque() {
        let a = code(DEFAULT_SALT, "m8|r3|1");
        assert_eq!(a, code(DEFAULT_SALT, "m8|r3|1"));
        assert_ne!(a, code(DEFAULT_SALT, "m8|r3|2"));
        assert_ne!(a, code(DEFAULT_SALT + 1, "m8|r3|1"));
        assert_eq!(a.len(), 8);
        assert!(forbidden_hits(&a, &[]).is_empty());
    }

    #[test]
    fn finds_forbidden_words() {
        assert!(forbidden_hits("Sağ alt köşede bir işaret.", &[]).is_empty());
        assert_eq!(forbidden_hits("the R3 page", &[]), vec!["r3"]);
        assert!(forbidden_hits("dal r4/m8a-sade", &[]).contains(&"r4/".to_owned()));
        assert_eq!(forbidden_hits("Sade düzen", &["sade"]), vec!["sade"]);
        assert!(forbidden_hits("card", &[]).contains(&"card".to_owned()));
    }
}
