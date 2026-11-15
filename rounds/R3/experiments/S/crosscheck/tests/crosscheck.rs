//! R3/S-A: the productized strategies equal the measured experiment crates.

use crosscheck::{
    fixtures_dir, pretty, sentence_fixture, sentences, sim_check, situations, support, CHANGED,
};

#[test]
fn sentences_equal_the_measured_realizer_except_the_documented_fixes() {
    let sits = situations();
    assert_eq!(sits.len(), 73);
    let measured = sentences(&m2_template::Realizer, &sits);
    let product = sentences(&qp_capsule::realize::Template, &sits);
    let mut changed = Vec::new();
    for (id, m) in &measured {
        let p = &product[id];
        if p != m {
            changed.push(id.as_str());
        }
    }
    assert_eq!(
        changed, CHANGED,
        "only the documented situations may change"
    );
}

#[test]
fn fixture_in_qp_capsule_is_current() {
    let p = fixtures_dir().join("m2_sentences.json");
    let on_disk = std::fs::read_to_string(&p).expect("fixture");
    assert_eq!(
        on_disk,
        pretty(&sentence_fixture()),
        "regenerate: crosscheck fixture"
    );
}

#[test]
fn simulate_product_config_equals_experiment_config() {
    for seed in [1u64, 2, 3] {
        let inputs = support::synthetic(seed, 30, 500);
        let c = sim_check(&format!("s{seed}"), &inputs);
        assert!(c.events > 1000, "{c:?}");
        assert!(c.ledger > 100, "{c:?}");
        assert!(c.events_equal && c.ledger_equal && c.facts_equal, "{c:?}");
    }
}

#[test]
fn rule_names_match() {
    let a = qp_capsule::Engine::new(qp_capsule::EngineConfig::r3());
    let b = qp_capsule::Engine::new(crosscheck::experiment_config());
    assert_eq!(a.rule_names(), b.rule_names());
    assert_eq!(a.rule_names(), ("kinddir", "barrier", "confirm"));
}
