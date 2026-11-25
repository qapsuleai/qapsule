//! [`rl1_studies_digest`]: the cross-target digest of the RL1 kernels (`stdev`,
//! `percentrank`), separate from the older digests so those stay byte-equal.

use super::{bench_closes, FNV_OFFSET};
use crate::core::Streaming;
use crate::studies::{PercentRank, Stdev};
use crate::{Field, Study};

/// Folds every output of `s` over `x` into `h`, every bar first pushed with a wrong value
/// (`x * 1.03`) and then revised (streaming + revise path); `None` = all ones.
fn feed(h: &mut u64, x: &[f64], s: &mut dyn Study) {
    let mut o = vec![None; s.outputs().len()];
    for &xi in x {
        s.push(&[xi * 1.03], &mut o);
        s.update_last(&[xi], &mut o);
        for v in &o {
            for b in v.map_or(u64::MAX, f64::to_bits).to_le_bytes() {
                *h ^= u64::from(b);
                *h = h.wrapping_mul(0x0100_0000_01b3);
            }
        }
    }
}

/// Digest of the RL1 kernels: FNV-1a over every output of `stdev(p)` and
/// `percentrank(p)` on `x = bench_closes(2_000, 7)` for `p` in `1, 2, 14, 20, 120`,
/// through the `Study` trait with every bar first pushed with a wrong value and then
/// revised. Printed by `examples/digest.rs rl1`; `verify.sh` compares it with
/// `fixtures/compute/digest-rl1.txt` on aarch64, x86_64 and wasm32.
#[must_use]
pub fn rl1_studies_digest() -> u64 {
    let x = bench_closes(2_000, 7);
    let mut h = FNV_OFFSET;
    for p in [1, 2, 14, 20, 120] {
        // Every period here is valid; a constructor error would show as a changed digest.
        if let Ok(k) = Stdev::new(p) {
            feed(&mut h, &x, &mut Streaming::new(k, Field::Close));
        }
        if let Ok(k) = PercentRank::new(p) {
            feed(&mut h, &x, &mut Streaming::new(k, Field::Close));
        }
    }
    h
}
