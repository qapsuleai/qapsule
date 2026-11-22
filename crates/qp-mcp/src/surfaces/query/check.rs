//! The `query` surface's vocabulary tables and argument checks (allowed keys,
//! `did_you_mean`).

use super::*;

// ---------------------------------------------------------------------------------------
// Vocabulary tables.

pub(super) const ASK_KEYS: [&str; 6] = ["find", "where", "select", "expand", "limit", "format"];

pub(super) const FINDS: [&str; 11] = [
    "capsules",
    "stories",
    "facts",
    "instrument",
    "ledger",
    "market",
    "glossary",
    "changes",
    "comments",
    "watches",
    "proposals",
];

/// Accepted spellings of a noun → the canonical one.
pub(super) const ALIASES: [(&str, &str); 16] = [
    ("capsule", "capsules"),
    ("story", "stories"),
    ("fact", "facts"),
    ("instruments", "instrument"),
    ("symbol", "instrument"),
    ("ledgers", "ledger"),
    ("scorecard", "ledger"),
    ("terms", "glossary"),
    ("change", "changes"),
    ("log", "changes"),
    ("comment", "comments"),
    ("watch", "watches"),
    ("intents", "watches"),
    ("proposal", "proposals"),
    ("market_status", "market"),
    ("findings", "capsules"),
];

pub(super) fn where_keys(find: Find) -> &'static [&'static str] {
    match find {
        Find::Capsules => &[
            "id",
            "capsule",
            "instrument",
            "meaning",
            "direction",
            "stage",
            "open",
            "holdings",
            "latest",
            "audience",
            "since_bar",
            "since_t",
        ],
        Find::Stories => &[
            "instrument",
            "meaning",
            "direction",
            "stage",
            "open",
            "range",
            "after",
        ],
        Find::Facts => &[
            "id",
            "capsule",
            "instrument",
            "meaning",
            "direction",
            "kind",
            "since_bar",
            "since_t",
        ],
        Find::Instrument | Find::Market => &["instrument"],
        Find::Ledger => &["key", "meaning", "detector", "direction", "by_kind"],
        Find::Glossary => &["instrument", "capsule", "id"],
        Find::Changes => &["after_seq", "since_t", "within_ms", "author", "since_bar"],
        Find::Comments => &["id", "capsule", "instrument"],
        Find::Watches => &["id", "state", "instrument"],
        Find::Proposals => &["id", "state"],
    }
}

pub(super) const ACT_DOS: [&str; 5] = ["comment", "watch", "unwatch", "propose", "preview"];

pub(super) fn act_keys(d: Do) -> &'static [&'static str] {
    match d {
        Do::Comment => &[
            "do",
            "capsule",
            "instrument",
            "text",
            "lang",
            "cites",
            "external",
        ],
        Do::Watch => &["do", "for", "text", "match", "expires_t", "max_matches"],
        Do::Unwatch => &["do", "watch"],
        Do::Propose => &["do", "ops", "reason"],
        Do::Preview => &["do", "proposal", "width", "height"],
    }
}

/// Operations only a person may send (§25.2): never through this surface.
pub(super) const HUMAN_ONLY: [&str; 4] = [
    "accept_proposal",
    "reject_proposal",
    "approve_finding",
    "reject_finding",
];

/// Stages after which a story is closed (§17.1 `open`).
pub(super) const CLOSED: [&str; 4] = ["realized", "invalidated", "expired", "retracted"];

/// Edit distance (for `did_you_mean`).
pub(super) fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut cur = vec![i + 1; b.len() + 1];
        for (j, cb) in b.iter().enumerate() {
            let sub = prev[j] + usize::from(ca != *cb);
            cur[j + 1] = sub.min(prev[j + 1] + 1).min(cur[j] + 1);
        }
        prev = cur;
    }
    prev[b.len()]
}

/// The closest candidates to `word` (distance ≤ 2 or a shared prefix of 3).
pub(super) fn did_you_mean<'a>(
    word: &str,
    candidates: impl IntoIterator<Item = &'a str>,
) -> Vec<String> {
    let w = word.to_ascii_lowercase();
    let mut scored: Vec<(usize, &str)> = candidates
        .into_iter()
        .filter_map(|c| {
            let d = distance(&w, &c.to_ascii_lowercase());
            let prefix = w.len() >= 3 && c.to_ascii_lowercase().starts_with(&w[..3]);
            (d <= 2 || prefix).then_some((d, c))
        })
        .collect();
    scored.sort_unstable();
    scored
        .into_iter()
        .take(3)
        .map(|(_, c)| c.to_owned())
        .collect()
}

/// Checks the keys of object `v` at `path` against `allowed`.
pub(super) fn check_keys(v: &Value, path: &str, allowed: &[&str], what: &str) -> Result<(), Fail> {
    let Some(o) = v.as_object() else {
        return Err(Fail::schema(
            path,
            format!("{what} must be an object"),
            allowed,
            vec![],
        ));
    };
    for k in o.keys() {
        if !allowed.contains(&k.as_str()) {
            return Err(Fail::schema(
                &format!("{path}/{k}"),
                format!("`{k}` is not a key of {what}"),
                allowed,
                did_you_mean(k, allowed.iter().copied()),
            ));
        }
    }
    Ok(())
}

pub(super) fn serde_fail(e: &serde_json::Error, what: &str) -> Fail {
    Fail(
        json!({"error": {"code": "schema", "message": format!("{what}: {e}"),
        "hint": "Check the value types against the tool's input schema."}}),
    )
}
