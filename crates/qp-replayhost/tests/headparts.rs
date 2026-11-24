//! Tests of the headline pieces (`qp_plane::headparts`) through the session (R4/H-FP),
//! over the demo replay (moved from `qp-authority` `capsule/tests/headparts.rs` in RF1
//! S3.3: the replay host left the authority).

use std::collections::BTreeMap;

use qp_grammar::HeadlineParts;
use qp_schema::capsule::Validity;
use qp_schema::protocol::{Language, ReplayInit, ReplaySourceKind};

use qp_replayhost::{demo_init, ReplayHost};

/// An example headline template (tests only; no face design in the registry).
fn example(hp: &HeadlineParts) -> String {
    let (sep, few) = match hp.lang {
        Language::Tr => (" · ", "az örnek"),
        Language::En => (" · ", "few cases"),
    };
    let tail = if hp.validity == Validity::Valid {
        hp.when.clone()
    } else {
        hp.validity_phrase.clone()
    };
    let mut s = format!("{} {}{sep}{tail}", hp.instrument, hp.event);
    if hp.few_cases {
        s.push_str(sep);
        s.push_str(few);
    }
    s
}

/// The demo world (synthetic, as `qp demo`), small; `visitor` reads without data.
fn host() -> ReplayHost {
    let mut ent = BTreeMap::new();
    for p in ["host", "arkin"] {
        ent.insert(p.to_owned(), vec!["*".to_owned()]);
    }
    ent.insert("visitor".to_owned(), Vec::new());
    let mut init = demo_init(ReplayInit {
        source: ReplaySourceKind::Synthetic,
        instruments: Some(8),
        days: Some(260),
        history: Some(120),
        fine: Some("1h".to_owned()),
        entitlements: ent,
        ..ReplayInit::default()
    });
    init.principals.push(
        serde_json::from_value(serde_json::json!(
            {"id": "visitor", "author": {"kind": "human", "name": "visitor"}, "role": "owner"}))
        .expect("visitor"),
    );
    let none = std::env::temp_dir().join("qp-hfp-no-local-data");
    ReplayHost::with_local_dir(
        init,
        std::path::Path::new("."),
        &none,
        qp_authority::Subsystems::NONE,
    )
    .expect("demo host")
}

/// Every capsule's pieces for a reader: `(finding, TR, EN)`.
fn all_parts(h: &mut ReplayHost, reader: &str) -> Vec<(String, HeadlineParts, HeadlineParts)> {
    let s = h.session_mut();
    let mut out = Vec::new();
    let findings: Vec<String> = s
        .document()
        .capsules
        .iter()
        .filter(|c| c.story.is_some())
        .map(|c| c.finding.clone())
        .collect();
    for f in findings {
        let mut both = Vec::new();
        for lang in [Language::Tr, Language::En] {
            both.extend(s.reader_parts_of(reader, lang, &f));
        }
        if let [tr, en] = both.as_slice() {
            out.push((f, tr.clone(), en.clone()));
        }
    }
    out
}

/// (R4/H-FP) The authority fills the headline pieces by the sentence's language and
/// audience rule: TR and EN texts, an example template within 80 characters, no
/// internal term, no digit for a reader without data entitlement; deterministic.
#[test]
fn reader_parts_by_language_and_audience() {
    let mut h = host();
    let mut seen = 0;
    let mut redacted = 0;
    let mut kinds = std::collections::BTreeSet::new();
    for step in 0..60 {
        if step % 20 == 0 {
            let a = all_parts(&mut h, "arkin");
            let v = all_parts(&mut h, "visitor");
            for (f, tr, en) in &a {
                seen += 1;
                assert_eq!((tr.lang, en.lang), (Language::Tr, Language::En));
                assert!(!tr.redacted && tr.ledger_n.is_some(), "{f}");
                assert_eq!(tr.few_cases, tr.ledger_n.unwrap() < 20);
                kinds.insert(format!("{:?}/{:?}", tr.stage, tr.validity));
                for hp in [tr, en] {
                    let line = example(hp);
                    assert!(line.chars().count() <= 80, "{line}");
                    let low = line.to_lowercase();
                    for bad in ["bar", "aile", "teyit", "family"] {
                        assert!(!low.contains(bad), "{line}");
                    }
                }
                assert_ne!(tr.event, en.event);
            }
            for (f, tr, en) in &v {
                if tr.redacted {
                    redacted += 1;
                    assert_eq!(tr.ledger_n, None, "{f}");
                    for hp in [tr, en] {
                        let line = example(hp);
                        for t in [
                            &line,
                            &hp.when,
                            &hp.moved,
                            &hp.ledger_short,
                            &hp.validity_phrase,
                        ] {
                            let digits = t.chars().filter(char::is_ascii_digit).count();
                            // The instrument label (`SYN03`) is the only digit source.
                            let label = hp.instrument.chars().filter(char::is_ascii_digit).count();
                            let allowed = if std::ptr::eq(t, &line) { label } else { 0 };
                            assert_eq!(digits, allowed, "{f}: {t}");
                        }
                    }
                }
            }
            // Deterministic.
            assert_eq!(a, all_parts(&mut h, "arkin"));
        }
        assert!(h.take_step().is_some());
    }
    assert!(seen > 0, "no capsule with a story");
    assert!(redacted > 0, "no redacted reader's pieces");
    assert!(kinds.len() > 1, "{kinds:?}");
}
