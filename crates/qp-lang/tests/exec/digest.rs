//! The determinism digest (VERIFY 9): every suite program checks and runs, and the host's
//! digest equals `fixtures/compute/digest-program.txt` (verify.sh also compares x86_64 and
//! wasm32).

use qp_lang::{program_digest, Executor, DIGEST_SUITE};

use crate::support::*;

#[test]
fn suite_programs_check_and_run() {
    let b = bars(300, 1);
    for p in DIGEST_SUITE {
        let v: serde_json::Value = serde_json::from_str(p).unwrap();
        // The digest's own limits: defaults with a raised cost limit.
        let limits = qp_lang::Limits::default().with(qp_lang::Limit::MaxCostPerBar, 10_000);
        let c = checked_with(&v, "1h", None, limits);
        assert!(Executor::run_closed(&c, &b, qp_lang::ExecOptions::default()).is_ok());
    }
}

#[test]
fn digest_matches_the_fixture() {
    let want = include_str!("../../../../fixtures/compute/digest-program.txt").trim();
    assert_eq!(program_digest(), want);
}
