//! RP1 load profiles: the digests of (subsets of) every profile are pinned, so any
//! optimization of the paths they exercise must keep every output bit unchanged. The
//! full-size digests are printed by `examples/sweep_bench.rs` / `benches/profiles.rs` and
//! recorded in `rounds/RP1/BASELINE.md`.

use qp_compute::profiles as pr;
use qp_compute::sweep::{fnv1a_columns, FNV_OFFSET};
use qp_compute::STUDY_NAMES;

fn b_subset_digest() -> u64 {
    // First 10 symbols (50 series), every study.
    let series: Vec<Vec<f64>> = pr::b_series().into_iter().take(50).collect();
    let mut d = FNV_OFFSET;
    for name in STUDY_NAMES {
        pr::b_run(name, &series, |_, c| d = fnv1a_columns(d, c));
    }
    d
}

fn c_subset_digest() -> u64 {
    // Every chain family on 2,000 bars (all 1,000 variants each).
    let x = qp_compute::sweep::bench_closes(2_000, 77);
    let mut d = FNV_OFFSET;
    for fam in pr::C_FAMILIES {
        pr::c_run(fam, &x, |_, c| d = fnv1a_columns(d, c));
    }
    d
}

#[test]
fn profile_digests_are_pinned() {
    let got = [b_subset_digest(), c_subset_digest(), pr::d_digest()];
    assert_eq!(
        got,
        [
            0x6949_9539_b047_400d,
            0x3be6_2d94_873c_4668,
            0x4db8_6a72_46f7_33bf
        ]
    );
}
