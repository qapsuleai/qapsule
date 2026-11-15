//! The ledger index (R3/S-C) answers exactly what the row scan answers, on the real
//! detectors with the chosen configuration, with retractions before and after closes.

#[path = "../examples/support/mod.rs"]
mod support;

use qp_capsule::pipeline::simulate_engine;
use qp_capsule::EngineConfig;

#[test]
fn indexed_ledger_equals_row_scan() {
    let inputs = support::synthetic(7, 8, 260);
    let (mut engine, _) =
        simulate_engine(&inputs, &qp_facts::default_detectors, EngineConfig::r3());
    assert!(engine.ledger_records().len() > 50, "enough rows");
    // Retract a few closed stories (rows stop counting after the retraction time) and an
    // open one.
    let last_t = inputs[0].bars.last().unwrap().t;
    let closed: Vec<(String, i64)> = engine
        .closed_stories()
        .iter()
        .step_by(7)
        .take(6)
        .map(|s| (s.id.clone(), s.story.history.last().unwrap().t))
        .collect();
    for (i, (id, t)) in closed.iter().enumerate() {
        let at = if i % 2 == 0 { *t } else { last_t };
        assert!(engine.retract(id, 0, at, "data_corrected").is_some());
    }
    if let Some(open) = engine.open_stories().first().map(|s| s.id.clone()) {
        assert!(engine.retract(&open, 0, last_t, "dispute_upheld").is_some());
    }
    let mut keys: Vec<Option<String>> = engine
        .ledger_records()
        .iter()
        .map(|r| r.entry.key.clone())
        .collect();
    keys.sort();
    keys.dedup();
    keys.push(Some("no/such@1".to_owned()));
    let mut times: Vec<i64> = inputs[0].bars.iter().map(|b| b.t).collect();
    times.extend(closed.iter().map(|(_, t)| *t));
    times.extend(closed.iter().map(|(_, t)| *t + 1));
    times.push(i64::MIN);
    times.push(i64::MAX);
    for t in &times {
        assert_eq!(
            engine.ledger_stat_all(*t),
            engine.ledger_stat_scan(None, *t)
        );
        for k in keys.iter().flatten() {
            assert_eq!(
                engine.ledger_stat(k, *t),
                engine.ledger_stat_scan(Some(k), *t),
                "key {k} at {t}"
            );
        }
    }
}
