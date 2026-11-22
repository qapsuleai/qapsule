//! Parallel sweep with per-variant reduction (RP1/E3).
//!
//! Variants are split into contiguous blocks, one per thread; each thread runs the
//! ordinary sweep on its block and reduces every variant's outputs to an `R` right away
//! (no columns are kept). Results come back in variant order. A variant's result depends
//! only on its own outputs, which are bit-identical to a single run, so the result is the
//! same for every thread count. On wasm32 (no threads) everything runs on the caller.

use std::collections::BTreeMap;

use crate::sweep::{sweep_each, SweepColumns};
use crate::ComputeError;

#[allow(clippy::expect_used)]
/// Runs `name` for every parameter set over `x` on up to `threads` threads and returns
/// `reduce(outputs)` per variant, in parameter-set order.
///
/// # Errors
/// Unknown study or an invalid parameter set (checked before any work).
pub fn sweep_reduce_par<R: Send>(
    name: &str,
    param_sets: &[BTreeMap<String, f64>],
    x: &[f64],
    threads: usize,
    reduce: impl Fn(&SweepColumns) -> R + Sync,
) -> Result<Vec<R>, ComputeError> {
    // Validate everything on the caller first (no work on invalid input).
    sweep_each(name, &[], x, |_, _| {})?;
    for p in param_sets {
        crate::create(name, p, crate::Field::Close)?;
    }
    let threads = if cfg!(target_arch = "wasm32") {
        1
    } else {
        threads.clamp(1, param_sets.len().max(1))
    };
    let run = |block: &[BTreeMap<String, f64>]| -> Result<Vec<R>, ComputeError> {
        let mut out = Vec::with_capacity(block.len());
        sweep_each(name, block, x, |_, c| out.push(reduce(c)))?;
        Ok(out)
    };
    if threads == 1 {
        return run(param_sets);
    }
    let per = param_sets.len().div_ceil(threads);
    std::thread::scope(|s| {
        let handles: Vec<_> = param_sets
            .chunks(per)
            .map(|block| s.spawn(move || run(block)))
            .collect();
        let mut all = Vec::with_capacity(param_sets.len());
        for h in handles {
            all.extend(h.join().expect("sweep thread panicked")?);
        }
        Ok(all)
    })
}
