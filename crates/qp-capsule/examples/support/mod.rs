//! Shared by the examples and tests of `qp-capsule` (included with `#[path]`): engine
//! inputs from the regime synthetic market with a synthetic licensed flow (the same
//! construction as `rounds/R3/experiments/harness-sim/src/common.rs`), and the local real
//! bars when present (never committed).

#![allow(dead_code)]

use std::path::{Path, PathBuf};

use qp_capsule::InstrumentInput;
use qp_facts::FlowPoint;

/// SplitMix64 finaliser (as in harness-sim).
#[must_use]
pub fn mix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Flow seed of instrument `i` in the synthetic market of `seed` (as in harness-sim).
#[must_use]
pub fn flow_seed(seed: u64, i: usize) -> u64 {
    mix(seed
        .wrapping_mul(1_000_003)
        .wrapping_add(u64::try_from(i).unwrap_or(0).wrapping_mul(7919))
        .wrapping_add(0xF1))
}

/// `qp_source::synthetic_market(n, days, seed)` with a synthetic licensed flow per
/// instrument; series `<symbol>.1day`.
#[must_use]
pub fn synthetic(seed: u64, n_instruments: usize, n_days: usize) -> Vec<InstrumentInput> {
    qp_source::synthetic_market(n_instruments, n_days, seed)
        .into_iter()
        .enumerate()
        .map(|(i, inst)| {
            let flow = qp_source::synthetic_flow(&inst.bars, flow_seed(seed, i))
                .into_iter()
                .map(|r| FlowPoint {
                    t: r.t,
                    values: r.values,
                })
                .collect();
            InstrumentInput {
                series: format!("{}.1day", inst.symbol),
                symbol: inst.symbol,
                bars: inst.bars,
                flow: Some(flow),
            }
        })
        .collect()
}

/// The local real-bars directory: `$QAPSULE_LOCAL_BARS`, else the first
/// `fixtures/local/bars` walking up from this crate (works from a git worktree).
#[must_use]
pub fn real_bars_dir() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os(qp_source::LOCAL_BARS_ENV) {
        let p = PathBuf::from(p);
        return p.is_dir().then_some(p);
    }
    let mut d = Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();
    loop {
        let c = d.join("fixtures/local/bars");
        if c.is_dir() {
            return Some(c);
        }
        if !d.pop() {
            return None;
        }
    }
}

/// Local real bars of `period` (`1day`, `1hour`) as engine inputs (no flow); `None` when
/// absent.
#[must_use]
pub fn real(period: &str) -> Option<Vec<InstrumentInput>> {
    let dir = real_bars_dir()?;
    let all = qp_source::load_local_bars(&dir, period);
    if all.is_empty() {
        return None;
    }
    Some(
        all.into_iter()
            .map(|(sym, bars)| InstrumentInput {
                series: format!("{sym}.{period}"),
                symbol: sym,
                bars,
                flow: None,
            })
            .collect(),
    )
}
