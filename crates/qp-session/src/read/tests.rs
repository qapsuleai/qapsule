//! Tests of the summary's indexed hints (`qp_ops::summary`) against a session's document.

use qp_ops::select::Resolver;
use qp_ops::summary::FastHints;

/// The summary's indexed hints are the resolver's, for every fact and finding of a
/// document with an engine history (many facts per kind, findings sharing facts).
#[test]
fn fast_hints_equal_the_resolvers() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let t: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(
            root.join("fixtures/protocol/conformance/38-capsule-lifecycle.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let mut init = t["init"].clone();
    let bars = qp_source::synthetic_market(1, 260, 1)[0].bars.clone();
    init["data"]["feeds"][0]["bars"] = serde_json::json!(bars);
    let s = crate::Session::new(serde_json::from_value(init).unwrap(), &root).unwrap();
    let doc = s.document();
    assert!(
        doc.facts.len() > 50 && doc.findings.len() > 20,
        "a real history"
    );
    let fast = FastHints::new(doc);
    let r = Resolver { doc, root: None };
    for f in &doc.facts {
        assert_eq!(fast.fact_hint(f), r.fact_hint(f), "fact {}", f.id);
    }
    for f in &doc.findings {
        assert_eq!(fast.finding_hint(f), r.finding_hint(f), "finding {}", f.id);
    }
}
