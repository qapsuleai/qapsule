//! Ledger lookup cost and output digest (R3/S-C, `rounds/R3/experiments/S/S-C.md`).
//!
//! Runs `simulate` with `EngineConfig::r3()` on `synthetic_market(30, 500, 1)` + the
//! synthetic licensed flow (the S-A construction) and prints:
//!
//! - FNV-1a 64 digests of the events, ledger rows and facts JSON (byte equality of the
//!   engine output before/after the ledger index);
//! - wall time of the simulation;
//! - the mean time of one `Engine::ledger_stat` call on the final ledger (every key, at
//!   100 times spread over the run).
//!
//! ```text
//! cargo run --release -p qp-capsule --example ledger_index
//! ```

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::cognitive_complexity
)]

#[path = "support/mod.rs"]
mod support;

use std::time::Instant;

use qp_capsule::pipeline::simulate_engine;
use qp_capsule::EngineConfig;

fn fnv(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

fn main() {
    let inputs = support::synthetic(1, 30, 500);
    let t0 = Instant::now();
    let (engine, facts) =
        simulate_engine(&inputs, &qp_facts::default_detectors, EngineConfig::r3());
    let sim = t0.elapsed();
    let events = serde_json::to_vec(engine.events()).expect("events json");
    let ledger = serde_json::to_vec(engine.ledger_records()).expect("ledger json");
    let facts_json = serde_json::to_vec(&facts).expect("facts json");
    println!(
        "events {} ({} B) digest {:016x}",
        engine.events().len(),
        events.len(),
        fnv(&events)
    );
    println!(
        "ledger {} digest {:016x}",
        engine.ledger_records().len(),
        fnv(&ledger)
    );
    println!("facts {} digest {:016x}", facts.len(), fnv(&facts_json));

    let mut keys: Vec<String> = engine
        .ledger_records()
        .iter()
        .filter_map(|r| r.entry.key.clone())
        .collect();
    keys.sort();
    keys.dedup();
    let ts: Vec<i64> = inputs[0].bars.iter().map(|b| b.t).collect();
    let step = (ts.len() / 100).max(1);
    let times: Vec<i64> = ts.iter().step_by(step).copied().collect();
    let mut calls = 0_u64;
    let mut acc = 0_u64;
    let t1 = Instant::now();
    for _ in 0..20 {
        for k in &keys {
            for t in &times {
                let s = engine.ledger_stat(k, *t);
                acc = acc.wrapping_add(u64::from(s.n) + u64::from(s.realized));
                calls += 1;
            }
        }
    }
    let per = t1.elapsed().as_nanos() / u128::from(calls.max(1));
    println!("simulate {:.3} s", sim.as_secs_f64());
    println!(
        "ledger_stat: {calls} calls over {} keys, {per} ns/call (checksum {acc})",
        keys.len()
    );
}
