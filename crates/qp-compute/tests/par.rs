//! RP1/E3: the parallel reducing sweep gives the same results, in the same order, for every
//! thread count, equal to the sequential sweep; invalid input is rejected before work.

use std::collections::BTreeMap;

use qp_compute::par::sweep_reduce_par;
use qp_compute::sweep::{bench_closes, bench_grid, fnv1a_columns, sweep_each, FNV_OFFSET};
use qp_compute::STUDY_NAMES;

#[test]
fn parallel_results_equal_sequential_for_every_thread_count() {
    let x = bench_closes(1500, 3);
    for name in STUDY_NAMES {
        let grid: Vec<_> = bench_grid(name).into_iter().step_by(7).collect(); // odd count
        let mut want = Vec::new();
        sweep_each(name, &grid, &x, |_, c| {
            want.push(fnv1a_columns(FNV_OFFSET, c))
        })
        .unwrap();
        for threads in [0, 1, 2, 3, 7, 16, 1000] {
            let got = sweep_reduce_par(name, &grid, &x, threads, |c| fnv1a_columns(FNV_OFFSET, c))
                .unwrap();
            assert_eq!(got, want, "{name} threads {threads}");
        }
    }
}

#[test]
fn invalid_input_rejected_before_work() {
    let x = bench_closes(100, 1);
    let bad = vec![
        BTreeMap::from([("period".to_owned(), 5.0)]),
        BTreeMap::from([("period".to_owned(), 0.0)]),
    ];
    assert!(sweep_reduce_par("sma", &bad, &x, 4, |_| -> u8 { panic!("no work") }).is_err());
    assert!(sweep_reduce_par("nope", &[], &x, 4, |_| 0).is_err());
    assert!(sweep_reduce_par("sma", &[], &x, 4, |_| 0)
        .unwrap()
        .is_empty());
}
