//! Names of the language and `did_you_mean` (PR §11.1): the node operations, the output
//! kinds, the fixed Turkish → English suggestion table, and the nearest-name rule of
//! `PROTOCOL.md` §13 (edit distance ≤ 3 or substring; at most 5; by distance, then name).
//!
//! The rule is the same as `qp_plane::suggest`; it is restated here because `qp-lang` sits
//! below the session layer and may not depend on `qp-plane` (`docs/ENGINEERING.md` §2).

/// Every node `op` of `qp-ir/0`, in PR §4 order.
pub const OP_NAMES: [&str; 43] = [
    "input",
    "const",
    "bar_index",
    "time",
    "sma",
    "ema",
    "rsi",
    "macd",
    "bollinger",
    "highest",
    "lowest",
    "atr",
    "stdev",
    "percentrank",
    "add",
    "sub",
    "mul",
    "div",
    "min",
    "max",
    "neg",
    "abs",
    "lag",
    "gt",
    "ge",
    "lt",
    "le",
    "eq",
    "ne",
    "and",
    "or",
    "not",
    "cross_above",
    "cross_below",
    "streak",
    "bars_since",
    "once",
    "rearm",
    "cooldown",
    "defined",
    "align",
    "line",
    "bucket",
];

/// Output kinds (PR §5).
pub const OUTPUT_KINDS: [&str; 4] = ["series", "event", "signal", "state"];

/// Bar fields (PR §4.1).
pub const BAR_FIELDS: [&str; 5] = ["open", "high", "low", "close", "volume"];

/// Fixed suggestion table: a Turkish (or RX4) name → the English name. Never accepted as
/// a name; only offered in `did_you_mean` (PR §11.1, §12.2).
pub const TURKISH_NAMES: [(&str, &str); 40] = [
    ("ortalama", "sma"),
    ("basit_ortalama", "sma"),
    ("hareketli_ortalama", "sma"),
    ("ussel_ortalama", "ema"),
    ("ustel_ortalama", "ema"),
    ("gorece_guc", "rsi"),
    ("goreceli_guc", "rsi"),
    ("en_yuksek", "highest"),
    ("en_dusuk", "lowest"),
    ("standart_sapma", "stdev"),
    ("sapma", "stdev"),
    ("yuzdelik", "percentrank"),
    ("yuzdelik_sira", "percentrank"),
    ("ortalama_gercek_aralik", "atr"),
    ("bollinger_bandi", "bollinger"),
    ("topla", "add"),
    ("cikar", "sub"),
    ("carp", "mul"),
    ("bol", "div"),
    ("en_kucuk", "min"),
    ("en_buyuk", "max"),
    ("mutlak", "abs"),
    ("gecikme", "lag"),
    ("onceki", "lag"),
    ("ve", "and"),
    ("veya", "or"),
    ("degil", "not"),
    ("yukari_kesisim", "cross_above"),
    ("asagi_kesisim", "cross_below"),
    ("ust_uste", "streak"),
    ("bir_kez", "once"),
    ("yeniden_kur", "rearm"),
    ("bekleme", "cooldown"),
    ("tanimli", "defined"),
    ("hizala", "align"),
    ("cizgi", "line"),
    ("kutu", "bucket"),
    ("zaman", "time"),
    ("sabit", "const"),
    ("fingerprint", "state"),
];

/// Levenshtein distance over Unicode scalar values.
#[must_use]
pub fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0; b.len() + 1];
    for (i, ca) in a.iter().enumerate() {
        cur[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let sub = prev[j] + usize::from(ca != cb);
            cur[j + 1] = sub.min(prev[j + 1] + 1).min(cur[j] + 1);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

/// Up to five candidates close to `given` (exact matches excluded, duplicates removed),
/// by distance, then name.
#[must_use]
pub fn did_you_mean<'a, I>(given: &str, candidates: I) -> Vec<String>
where
    I: IntoIterator<Item = &'a str>,
{
    let mut scored: Vec<(usize, &str)> = Vec::new();
    for c in candidates {
        if c == given || scored.iter().any(|(_, s)| *s == c) {
            continue;
        }
        let d = edit_distance(given, c);
        let substring = !given.is_empty() && (c.contains(given) || given.contains(c));
        if d <= 3 || substring {
            scored.push((d, c));
        }
    }
    scored.sort_unstable();
    scored
        .into_iter()
        .take(5)
        .map(|(_, s)| s.to_owned())
        .collect()
}

/// Suggestions for a name in a fixed English vocabulary (`OP_NAMES`, `OUTPUT_KINDS` ...):
/// the Turkish table's English name first when it is in the vocabulary, then the
/// nearest names.
#[must_use]
pub fn suggest(given: &str, vocabulary: &[&str]) -> Vec<String> {
    let mut out: Vec<String> = TURKISH_NAMES
        .iter()
        .filter(|(tr, en)| *tr == given && vocabulary.contains(en))
        .map(|(_, en)| (*en).to_owned())
        .collect();
    for s in did_you_mean(given, vocabulary.iter().copied()) {
        if out.len() < 5 && !out.contains(&s) {
            out.push(s);
        }
    }
    out
}

/// Whether `s` is a node id: `^[A-Za-z_][A-Za-z0-9_]{0,63}$` (PR §2).
#[must_use]
pub fn is_node_id(s: &str) -> bool {
    let mut it = s.chars();
    matches!(it.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && s.len() <= 64
        && it.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Whether `s` is an output, dimension or parameter name: `^[a-z][a-z0-9_]{0,31}$`.
#[must_use]
pub fn is_lower_name(s: &str) -> bool {
    let mut it = s.chars();
    matches!(it.next(), Some(c) if c.is_ascii_lowercase())
        && s.len() <= 32
        && it.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_and_suggestions() {
        assert_eq!(edit_distance("kitten", "sitting"), 3);
        assert_eq!(did_you_mean("rsii", OP_NAMES.iter().copied())[0], "rsi");
        assert_eq!(suggest("ortalama", &OP_NAMES)[0], "sma");
        assert_eq!(suggest("ussel_ortalama", &OP_NAMES)[0], "ema");
        assert_eq!(suggest("fingerprint", &OUTPUT_KINDS), vec!["state"]);
        let many = ["a1", "a2", "a3", "a4", "a5", "a6"];
        assert_eq!(did_you_mean("a", many.iter().copied()).len(), 5);
    }

    #[test]
    fn name_patterns() {
        assert!(is_node_id("_1") && is_node_id("Ab_9") && !is_node_id("1a"));
        assert!(!is_node_id("a.b") && !is_node_id(&"a".repeat(65)));
        assert!(is_lower_name("kirilim") && !is_lower_name("Kirilim"));
        assert!(!is_lower_name("_x") && !is_lower_name(&"a".repeat(33)));
    }

    #[test]
    fn turkish_targets_exist() {
        for (_, en) in TURKISH_NAMES {
            assert!(OP_NAMES.contains(&en) || OUTPUT_KINDS.contains(&en), "{en}");
        }
    }
}
