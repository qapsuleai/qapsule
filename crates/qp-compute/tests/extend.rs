//! RP1/E5: `Study::extend` (commit-only attach) leaves exactly the state of pushing the same
//! bars, for every study and for chains, mixed randomly with `push` and `update_last`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::cognitive_complexity
)]

use std::collections::BTreeMap;

use qp_compute::sweep::{bench_closes, bench_grid};
use qp_compute::{create, Chain, Field, Study, STUDY_NAMES};

fn studies() -> Vec<Box<dyn Study>> {
    let mut v: Vec<Box<dyn Study>> = Vec::new();
    for name in STUDY_NAMES {
        for p in bench_grid(name).into_iter().step_by(97) {
            v.push(create(name, &p, Field::Close).unwrap());
        }
    }
    let rsi = create("rsi", &BTreeMap::new(), Field::Close).unwrap();
    let ema = create(
        "ema",
        &BTreeMap::from([("period".to_owned(), 5.0)]),
        Field::Close,
    )
    .unwrap();
    v.push(Box::new(Chain::new(rsi, 0, ema)));
    v
}

fn bits(o: &[Option<f64>]) -> Vec<Option<u64>> {
    o.iter().map(|v| v.map(f64::to_bits)).collect()
}

#[test]
fn extend_equals_pushes_in_any_mix() {
    let x = bench_closes(3000, 12);
    let mut s: u64 = 5;
    let mut next = |m: u64| {
        s = s
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (s >> 33) % m
    };
    for round in 0..20 {
        let mut a = studies();
        let mut b = studies();
        let mut i = 0;
        while i < x.len() {
            let k = (1 + next(400)) as usize;
            let j = (i + k).min(x.len());
            match next(3) {
                0 => {
                    for st in &mut a {
                        st.extend(&[&x[i..j]]);
                    }
                }
                _ => {
                    for st in &mut a {
                        let mut o = vec![None; st.outputs().len()];
                        for &v in &x[i..j] {
                            st.push(&[v], &mut o);
                        }
                    }
                }
            }
            for st in &mut b {
                let mut o = vec![None; st.outputs().len()];
                for &v in &x[i..j] {
                    st.push(&[v], &mut o);
                }
            }
            // Revise the last bar on both and compare outputs.
            let rev = x[j - 1] * (1.0 + 0.001 * next(10) as f64);
            for (sa, sb) in a.iter_mut().zip(b.iter_mut()) {
                let mut oa = vec![None; sa.outputs().len()];
                let mut ob = vec![None; sb.outputs().len()];
                sa.update_last(&[rev], &mut oa);
                sb.update_last(&[rev], &mut ob);
                assert_eq!(bits(&oa), bits(&ob), "round {round} bar {j} {}", sa.name());
                sa.update_last(&[x[j - 1]], &mut oa);
                sb.update_last(&[x[j - 1]], &mut ob);
            }
            i = j;
        }
    }
}
