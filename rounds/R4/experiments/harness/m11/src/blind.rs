//! Blinding (SPEC §0.2, §0.3): sheet images are named by a seeded code instead of the
//! style name (the mapping is in `raw/key.json`, never copied to a reader), and every
//! prompt is checked against a forbidden-word list before it is sent.

use qp_grammar::glyph::outcome;

use crate::rng::Rng;

/// Default salt of the codes.
pub const DEFAULT_SALT: u64 = 11;

/// The code of one sheet: 8 hex digits from a seeded hash of (style, alphabet, seed).
/// Hex digits contain no letter beyond `f`, so a code never spells a forbidden word.
#[must_use]
pub fn code(style: &str, alphabet: &str, seed: u64, salt: u64) -> String {
    let mut r = Rng::derive(salt, &format!("m11|{style}|{alphabet}|{seed}"));
    format!("{:08x}", r.next_u64() & 0xffff_ffff)
}

/// Whole words that must not appear in a prompt (case-insensitive), besides every
/// registered style name: experiment, branch and agent vocabulary.
pub const FORBIDDEN_WORDS: [&str; 16] = [
    "variant",
    "varyant",
    "alternatif",
    "alternatifi",
    "alternative",
    "alternatives",
    "harness",
    "düzenek",
    "m11",
    "m11a",
    "m11b",
    "claude",
    "r3",
    "r4",
    "style",
    "stil",
];

/// Substrings that must not appear anywhere (branch names `r4/m11a-…`, identifiers
/// `alt-…`, `alt_…`, the code word `outcome`). The bare Turkish word "alt" ("bottom",
/// as in "sağ alt köşe") is ordinary Turkish and allowed (README "Sapmalar").
pub const FORBIDDEN_SUBSTRINGS: [&str; 8] = [
    "r4/", "r3/", "m11", "alt-", "alt_", "/alt", "outcome", "variant",
];

/// Lower-cased words of `text` (split on anything that is not alphanumeric, `_` or `-`).
fn words(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '-'))
        .filter(|w| !w.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Forbidden words and substrings found in `text` (empty = clean). Style names are every
/// name in `qp_grammar::glyph::outcome::STYLES` plus `extra`.
#[must_use]
pub fn forbidden_hits(text: &str, extra: &[&str]) -> Vec<String> {
    let mut banned: Vec<String> = FORBIDDEN_WORDS.iter().map(|s| (*s).to_owned()).collect();
    banned.extend(outcome::names().iter().map(|s| (*s).to_lowercase()));
    banned.extend(extra.iter().map(|s| s.to_lowercase()));
    let low = text.to_lowercase();
    let mut hits: Vec<String> = Vec::new();
    for w in words(text) {
        // A word with `-`/`_` is also checked part by part.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_seeded_and_opaque() {
        let a = code("r3", "siluet", 1, DEFAULT_SALT);
        assert_eq!(a, code("r3", "siluet", 1, DEFAULT_SALT));
        assert_ne!(a, code("r3", "siluet", 2, DEFAULT_SALT));
        assert_ne!(a, code("r3", "rozet", 1, DEFAULT_SALT));
        assert_ne!(a, code("r3", "siluet", 1, DEFAULT_SALT + 1));
        assert_eq!(a.len(), 8);
        assert!(forbidden_hits(&a, &[]).is_empty());
    }

    #[test]
    fn finds_forbidden_words() {
        assert!(forbidden_hits("Sağ alt köşede onay işareti.", &[]).is_empty());
        assert_eq!(forbidden_hits("the R3 sign", &[]), vec!["r3"]);
        assert!(forbidden_hits("dal r4/m11a-kalkan", &[]).contains(&"m11".to_owned()));
        assert_eq!(forbidden_hits("Kalkan biçimi", &["kalkan"]), vec!["kalkan"]);
        assert_eq!(forbidden_hits("alt_b", &[]), vec!["alt_"]);
        assert!(forbidden_hits("başka bir varyant", &[]).contains(&"varyant".to_owned()));
    }
}
