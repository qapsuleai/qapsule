//! (R5/M18) The first fact of each story row's finding (a story chart's cluster names its
//! stories by these facts). RF1 S3.2: moved from the authority's viewer, used by the
//! viewer and by the reading subsystem (`qp-read`).

/// (R5/M18) The first fact of the finding of every row in `rows` (one pass over the
/// document's findings; a story chart's cluster names its stories by these facts).
#[must_use]
pub fn first_facts(
    doc: &qp_schema::Document,
    rows: &[qp_schema::protocol::StoryRow],
) -> std::collections::BTreeMap<String, String> {
    let want: std::collections::BTreeSet<&str> = rows.iter().map(|r| r.finding.as_str()).collect();
    doc.findings
        .iter()
        .filter(|f| want.contains(f.id.as_str()))
        .filter_map(|f| Some((f.id.clone(), f.facts.first()?.clone())))
        .collect()
}

/// (R5/m18-pavka-perf) FNV-1a as a [`std::hash::Hasher`]: a fixed (not random-seeded)
/// hash for [`FirstFacts`]' sets of ids; it only decides where an id sits, never an
/// answer.
#[derive(Default)]
struct FnvHasher(u64);

impl std::hash::Hasher for FnvHasher {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        let mut h = if self.0 == 0 {
            0xcbf2_9ce4_8422_2325
        } else {
            self.0
        };
        for b in bytes {
            h = (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3);
        }
        self.0 = h;
    }
}

type Fnv = std::hash::BuildHasherDefault<FnvHasher>;

/// (R5/m18-pavka-perf) [`first_facts`] looked up only when first asked, without copying
/// the document's ids and with hashed sets: [`FirstFacts::get`]`(f)` is exactly
/// `first_facts(doc, rows).get(f).cloned()` for every `f` (the same pass over the same
/// findings in the same order, the same keys, the last finding with a fact winning), but
/// a story chart asks only for its clustered stories, so a scene without a cluster never
/// makes the pass. Measured on the JIT-less phone setting (WebKit, `useJIT=false`), the
/// eager map over 1,600 findings and 533 rows took ~10 ms per scene build (string
/// comparisons in ordered sets); the hashed pass ~1.5 ms.
pub struct FirstFacts<'a> {
    doc: &'a qp_schema::Document,
    rows: &'a [qp_schema::protocol::StoryRow],
    map: std::cell::OnceCell<std::collections::HashMap<&'a str, &'a str, Fnv>>,
}

impl<'a> FirstFacts<'a> {
    /// The lookup over `doc` for the findings of `rows` (nothing is read yet).
    #[must_use]
    pub fn new(doc: &'a qp_schema::Document, rows: &'a [qp_schema::protocol::StoryRow]) -> Self {
        Self {
            doc,
            rows,
            map: std::cell::OnceCell::new(),
        }
    }

    /// The first fact of finding `f` (`None`: not a row's finding, unknown, or no fact).
    #[must_use]
    pub fn get(&self, f: &str) -> Option<String> {
        let m = self.map.get_or_init(|| {
            let want: std::collections::HashSet<&str, Fnv> =
                self.rows.iter().map(|r| r.finding.as_str()).collect();
            self.doc
                .findings
                .iter()
                .filter(|x| want.contains(x.id.as_str()))
                .filter_map(|x| Some((x.id.as_str(), x.facts.first()?.as_str())))
                .collect()
        });
        m.get(f).map(|s| (*s).to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::{first_facts, FirstFacts};
    use qp_schema::protocol::StoryRow;

    /// (R5/m18-pavka-perf) The lazy, hashed lookup answers what the eager map did, for
    /// every id: listed findings with facts, a listed finding without facts, duplicate
    /// ids (one later without facts, one later with other facts), a finding not listed,
    /// a listed finding the document does not have, and an id nobody has.
    #[test]
    fn first_facts_lookup_equals_the_eager_map() {
        let mut doc: qp_schema::Document = serde_json::from_str(include_str!(
            "../../../fixtures/schema/document.example.json"
        ))
        .unwrap();
        let base = doc.findings[0].clone();
        let mk = |id: &str, facts: &[&str]| {
            let mut f = base.clone();
            f.id = id.to_owned();
            f.facts = facts.iter().map(|s| (*s).to_owned()).collect();
            f
        };
        doc.findings = vec![
            mk("a", &["fa"]),
            mk("b", &[]),
            mk("c", &["fc1", "fc2"]),
            mk("d", &["fd"]),
            mk("d", &[]),
            mk("e", &["fe1"]),
            mk("e", &["fe2"]),
            mk("unlisted", &["fu"]),
        ];
        let row: StoryRow = serde_json::from_value(serde_json::json!({
            "id": "cap", "finding": "x", "series": "px", "kind": "breakout",
            "direction": "up", "tier": 1, "born_bar": 1, "born_t": 0, "last_bar": 1,
            "last_t": 0, "stage": "born", "validity": "valid", "in_feed": true,
            "coin_flip": false,
            "face": {"event": "e", "direction_word": "d", "when": "w", "moved": "m",
                     "validity_phrase": "v", "ledger_short": "l", "few_cases": false}
        }))
        .unwrap();
        let rows: Vec<StoryRow> = ["a", "b", "c", "d", "e", "missing"]
            .iter()
            .map(|f| StoryRow {
                finding: (*f).to_owned(),
                ..row.clone()
            })
            .collect();
        let eager = first_facts(&doc, &rows);
        let lazy = FirstFacts::new(&doc, &rows);
        for id in ["a", "b", "c", "d", "e", "unlisted", "missing", "nobody", ""] {
            assert_eq!(lazy.get(id), eager.get(id).cloned(), "{id}");
        }
        assert_eq!(lazy.get("e").as_deref(), Some("fe2"));
        assert_eq!(lazy.get("d").as_deref(), Some("fd"));
    }
}
