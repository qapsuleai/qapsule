//! What a principal may receive of the capsule items (K33, K34, K36): the projection the
//! authority computes per reader (`qp-session` `capsule` module docs) and the reader
//! indices keep ([`crate::capidx::ReaderProj`]).

use std::collections::{BTreeMap, BTreeSet};

use qp_schema::capsule::Story;

/// What a principal may receive (see the module docs).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Proj {
    /// The principal it is for: only its own capsule preferences (audience) go to it
    /// (empty: the host, which receives every audience).
    pub principal: String,
    /// Fact, finding and capsule ids that never go.
    pub hidden: BTreeSet<String>,
    /// Fact ids sent without evidence values and `evidence_ref`.
    pub redact: BTreeSet<String>,
    /// Capsule id → its story as the principal sees it (licensed joins removed), when
    /// that differs from the stored one.
    pub stories: BTreeMap<String, Story>,
    /// Capsules whose sentence must be written for this principal (redacted or with
    /// hidden facts).
    pub rewrite: BTreeSet<String>,
}

impl Proj {
    /// Whether the JSON of `v` holds a hidden id as a string (`"id"`).
    pub fn mentions_hidden(&self, v: &impl serde::Serialize) -> bool {
        if self.hidden.is_empty() {
            return false;
        }
        let s = serde_json::to_string(v).unwrap_or_default();
        // An id without a quote occurs as `"id"` exactly when it is the text between two
        // consecutive quotes of the JSON: look those up instead of searching every id.
        let mut quotes = s.match_indices('"').map(|(i, _)| i);
        if let Some(mut a) = quotes.next() {
            for b in quotes {
                if self.hidden.contains(&s[a + 1..b]) {
                    return true;
                }
                a = b;
            }
        }
        self.hidden
            .iter()
            .filter(|h| h.contains('"'))
            .any(|h| s.contains(&format!("\"{h}\"")))
    }
}
