//! `qp_capsule::trust::Muhur` behaves exactly as the measured M6 crate
//! (`rounds/R3/experiments/M6/muhur/`, chosen in `M6/RESULTS.md`): on the synthetic
//! licensed flow of several instruments, every message (codes, texts, trust, shown bar,
//! attachments incl. receipts) and every audit-log entry is equal. The experiment's
//! source is compiled here unchanged (`#[path]`; it uses `qp_capsule`'s trait types).

#[allow(dead_code, clippy::all, clippy::pedantic)]
#[path = "../../../rounds/R3/experiments/M6/muhur/src/lib.rs"]
mod experiment;

use qp_capsule::trust::{self, Muhur, Params};
use qp_capsule::{FlowObservation, TrustPath};

fn observations(seed: u64, n: usize) -> Vec<Vec<FlowObservation>> {
    qp_source::synthetic_market(6, n, seed)
        .into_iter()
        .enumerate()
        .map(|(i, inst)| {
            qp_source::synthetic_flow(&inst.bars, seed ^ ((i as u64) << 8))
                .into_iter()
                .enumerate()
                .map(|(b, r)| FlowObservation {
                    t: r.t,
                    bar: b as u64,
                    instrument: inst.symbol.clone(),
                    values: r.values,
                })
                .collect()
        })
        .collect()
}

fn log(entries: &[impl AuditLike]) -> Vec<(u64, String, String)> {
    entries.iter().map(AuditLike::row).collect()
}

trait AuditLike {
    fn row(&self) -> (u64, String, String);
}

impl AuditLike for trust::AuditEntry {
    fn row(&self) -> (u64, String, String) {
        (
            self.bar,
            self.receipt.clone(),
            self.evidence_commitment.clone(),
        )
    }
}

impl AuditLike for experiment::AuditEntry {
    fn row(&self) -> (u64, String, String) {
        (
            self.bar,
            self.receipt.clone(),
            self.evidence_commitment.clone(),
        )
    }
}

#[test]
fn product_muhur_equals_measured_crate() {
    let mut said = 0;
    for seed in [1_u64, 2, 3] {
        for obs in observations(seed, 400) {
            let mut a = Muhur::demo(Params::default());
            let mut b = experiment::MuhurPath::new(experiment::Params::default());
            for o in &obs {
                let ma = a.observe(o, &[]);
                let mb = b.observe(o, &[]);
                assert_eq!(ma, mb, "messages at {} bar {}", o.instrument, o.bar);
                said += ma.len();
            }
            assert_eq!(log(a.audit_log()), log(b.audit_log()));
            let (ra, la) = trust::replay(&obs, Params::default());
            let (rb, lb) = experiment::replay(&obs, experiment::Params::default());
            assert_eq!(ra, rb);
            assert_eq!(log(&la), log(&lb));
            assert_eq!(trust::verify_receipts(&ra, trust::DEMO_RECEIPT_KEY), None);
        }
    }
    assert!(said > 20, "the path spoke ({said} messages)");
}

/// Host keys (security review): a path built with host-supplied keys says the same thing
/// as the demo path but its receipts differ, so the demo keys never stand in for a real
/// host's secret once it calls `with_keys` (`Session::set_trust_keys`).
#[test]
fn host_keys_change_receipts_not_statements() {
    use qp_capsule::trust::{Muhur, Params};
    let mut demo = Muhur::demo(Params::default());
    let mut host = Muhur::with_keys(
        Params::default(),
        b"host-receipt-secret",
        b"host-audit-secret",
    );
    let (mut said, mut differ) = (0, 0);
    for inst in observations(7, 300) {
        for o in &inst {
            let a = demo.observe(o, &[]);
            let b = host.observe(o, &[]);
            assert_eq!(a.len(), b.len());
            for (x, y) in a.iter().zip(&b) {
                assert_eq!(
                    (&x.codes, &x.tr, &x.en, x.shown_bar),
                    (&y.codes, &y.tr, &y.en, y.shown_bar)
                );
                said += 1;
                if x.attachments.get("receipt") != y.attachments.get("receipt") {
                    differ += 1;
                }
            }
        }
    }
    assert!(said > 0);
    assert_eq!(said, differ, "every receipt depends on the key");
}
