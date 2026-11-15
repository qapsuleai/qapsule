//! RP2/T7 consistency scenario harness.
//!
//! A deterministic, single-threaded simulation of one authority and one replica, each
//! with its own market-data feed, joined by a protocol channel. Approaches to the
//! data-plane / meaning-plane consistency problem plug in as two adapters
//! ([`adapter::AuthorityAdapter`], [`adapter::ReplicaAdapter`]); the oracle measures
//! M1–M13 from the replica's display, its visible data and the channel bytes.
//! See `rounds/RP2/experiments/HARNESS.md`.

#![forbid(unsafe_code)]

pub mod adapter;
pub mod approaches;
pub mod channel;
pub mod detector;
pub mod display;
pub mod engine;
pub mod feed;
pub mod metrics;
pub mod oracle;
pub mod rng;
pub mod scenarios;
pub mod series;
pub mod sim;
pub mod spec;
pub mod world;

use adapter::Approach;
use metrics::{summarize, Metrics, Summary};
use spec::ScenarioSpec;

/// Runs `seeds` seeds (`seed_base..seed_base + seeds`) of a scenario on `threads`
/// threads. Each seed is an independent deterministic run; results are in seed order.
#[must_use]
pub fn run_scenario(
    spec: &ScenarioSpec,
    approach: &Approach,
    seed_base: u64,
    seeds: u64,
    threads: usize,
) -> (Vec<Metrics>, Summary) {
    let t0 = std::time::Instant::now();
    let threads = threads.max(1);
    let all: Vec<u64> = (seed_base..seed_base + seeds).collect();
    let mut out: Vec<Metrics> = Vec::with_capacity(all.len());
    std::thread::scope(|sc| {
        let chunks: Vec<Vec<u64>> = (0..threads)
            .map(|i| all.iter().copied().skip(i).step_by(threads).collect())
            .collect();
        let handles: Vec<_> = chunks
            .into_iter()
            .map(|c| {
                sc.spawn(move || {
                    c.into_iter()
                        .map(|s| sim::run_one(spec, approach, s).metrics)
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        for h in handles {
            out.extend(h.join().expect("worker"));
        }
    });
    out.sort_by_key(|m| m.seed);
    let s = summarize(approach.name, &spec.name, &out, t0.elapsed().as_secs_f64());
    (out, s)
}
