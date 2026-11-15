//! Authority ↔ replica (RP2/T8, `docs/PROTOCOL.md` §19; VERIFY 12, GOAL criterion 8):
//! the same tick stream gives the same document and view digests after every step, and a
//! replica whose data lags never shows a wrong or early claim (checked by an independent
//! oracle over its own visible bars, `qp_session::tape::run`).

use qp_session::tape::run;

#[test]
fn in_step_replica_has_the_authoritys_digests_after_every_step() {
    let o = run(0).unwrap();
    let steps = o.tape.steps.len();
    assert!(steps > 100);
    assert_eq!(o.differing_steps, 0, "steps with different digests");
    assert!(o.violations.is_empty(), "{:?}", o.violations);
    assert!(o.final_claims > 0, "the scenario makes final claims");
    assert_eq!(
        o.shown_final, o.final_claims,
        "every final claim is shown final"
    );
    println!(
        "lag 0: {steps} steps, {} final claims, all shown; display counts {:?}",
        o.final_claims, o.shown_counts
    );
}

#[test]
fn lagging_replica_shows_no_wrong_or_early_claim() {
    for lag in [1, 3, 7, 25, 60] {
        let o = run(lag).unwrap();
        assert!(o.violations.is_empty(), "lag {lag}: {:?}", o.violations);
        // Once its data has caught up, it shows exactly the authority's final claims.
        let last = o.tape.steps.last().unwrap();
        assert_eq!(last.expect.doc, last.expect.authority_doc, "lag {lag}");
        assert_eq!(last.expect.view, last.expect.authority_view, "lag {lag}");
        assert_eq!(o.shown_final, o.final_claims, "lag {lag}");
        assert!(
            o.differing_steps > 0,
            "lag {lag}: the lag shows in the view digest"
        );
        println!(
            "lag {lag}: {} steps differ while lagging; {} final claims, all shown at the end; display counts {:?}",
            o.differing_steps, o.final_claims, o.shown_counts
        );
    }
}

#[test]
fn tape_is_deterministic() {
    let a = serde_json::to_string(&run(3).unwrap().tape).unwrap();
    let b = serde_json::to_string(&run(3).unwrap().tape).unwrap();
    assert_eq!(a, b);
}

/// A replica whose own provider does not entitle its principal cannot verify anything:
/// every claim stays pending with reason `no_entitlement` (§19.7), nothing is shown.
#[test]
fn unentitled_replica_shows_only_pending() {
    let p = serde_json::json!({"feeds":[{"feed":{"symbol":"THYAO","interval":"1d"},
        "synthetic":{"n":qp_session::tape::START,"seed":qp_session::tape::SEED}}],
        "entitlements":{}});
    let o = qp_session::tape::run_with(0, &p).unwrap();
    assert!(o.violations.is_empty(), "{:?}", o.violations);
    assert_eq!(o.shown_counts.0 + o.shown_counts.1 + o.shown_counts.3, 0);
    assert!(o.shown_counts.2 > 0, "claims arrive and stay pending");
    assert_eq!(o.shown_final, 0);
}
