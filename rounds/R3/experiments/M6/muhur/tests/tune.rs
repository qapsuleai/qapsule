//! Parameter study (ignored by default): utility and message rate on synthetic flow.
//! `cargo test --release --test tune -- --ignored --nocapture`

use m6_muhur::{MuhurPath, Params};
use qp_capsule::{FlowObservation, TrustPath};
use qp_facts::{Detector, FlowPoint, InstitutionalFlow};

#[test]
#[ignore = "parameter study"]
fn study() {
    let mut grid = Vec::new();
    for window in [5usize, 10, 20] {
        for low in [1, 2, 3] {
            for loud in [1, 2] {
                for epoch in [1u64, 5] {
                    grid.push(Params {
                        window,
                        min_obs: 5,
                        low,
                        loud,
                        epoch,
                    });
                }
            }
        }
    }
    for p in grid {
        let (mut facts, mut hit, mut msgs, mut bars) = (0u64, 0u64, 0u64, 0u64);
        for seed in [11u64, 12, 13] {
            for inst in qp_source::synthetic_market(30, 500, seed) {
                let flow = qp_source::synthetic_flow(&inst.bars, seed * 1000 + 7);
                let mut det = InstitutionalFlow::default().detector();
                let mut path = MuhurPath::new(p);
                for (i, (b, f)) in inst.bars.iter().zip(&flow).enumerate() {
                    let fp = FlowPoint {
                        t: f.t,
                        values: f.values.clone(),
                    };
                    let fact = !det.push(b, Some(&fp)).is_empty();
                    let obs = FlowObservation {
                        t: b.t,
                        bar: i as u64,
                        instrument: inst.symbol.clone(),
                        values: f.values.clone(),
                    };
                    let said = !path.observe(&obs, &[]).is_empty();
                    bars += 1;
                    facts += u64::from(fact);
                    hit += u64::from(fact && said);
                    msgs += u64::from(said);
                }
            }
        }
        #[allow(clippy::cast_precision_loss)]
        let r = |a: u64, b: u64| a as f64 / b as f64;
        println!(
            "{p:?}: utility {:.3}  msgs/bar {:.3}  precision(msg has fact) {:.3}",
            r(hit, facts),
            r(msgs, bars),
            r(hit, msgs)
        );
    }
}
