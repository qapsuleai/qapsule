//! R3/S-A engine cost (GOAL criterion 17, **engine side only**; the session-level
//! measurement comes later).
//!
//! ```text
//! cargo run --release -p qp-capsule --example engine_cost [-- <ticks per bar> <runs>]
//! ```
//!
//! 30 instruments (`synthetic_market(30, 500, 1)` with synthetic flow) × the seven
//! default detectors (six meaning kinds), streaming, with the chosen engine
//! (`EngineConfig::r3()`):
//!
//! - **Tick** (live revision of the open bar): per instrument, every detector
//!   `update_last` (the first tick of a bar is a `push`, which commits the previous bar),
//!   the detections become facts (`to_fact`), and `Engine::candidates` runs on the open
//!   bar. A *market tick* = one tick on each of the 30 instruments.
//! - **Bar close**: per instrument `Engine::on_bar` with the final facts (the last tick's
//!   detections; the last tick equals the final bar).
//!
//! Ticks are synthetic revisions of each bar: tick `k` of `K` moves the close from the
//! open towards the final close with a small deterministic wiggle; high / low / volume
//! grow monotonically; tick `K` is the final bar. Time is measured per market tick and per
//! market bar close (one `Instant` pair each, around 30 instruments), from bar 60 on (the
//! first 60 bars warm the detectors and the ledger). Reported: mean per run, median of
//! `runs` runs; ns per instrument-tick = market tick / 30.

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

use qp_capsule::engine::{Engine, EngineConfig};
use qp_capsule::pipeline::ENGINE_VERSION;
use qp_capsule::InstrumentInput;
use qp_facts::{default_detectors, to_fact, Detector};
use qp_schema::{Bar, Fact};

const WARM: usize = 60;

/// Tick `k` of `n` (1-based) of bar `b`; tick `n` is `b` itself.
fn tick(b: &Bar, k: usize, n: usize) -> Bar {
    if k == n {
        return b.clone();
    }
    #[allow(clippy::cast_precision_loss)]
    let f = k as f64 / n as f64;
    #[allow(clippy::cast_precision_loss)]
    let wiggle = if k.is_multiple_of(2) { 0.25 } else { -0.25 } * (b.h - b.l) * (1.0 - f);
    let c = (b.o + (b.c - b.o) * f + wiggle).clamp(b.l, b.h);
    Bar {
        t: b.t,
        o: b.o,
        h: b.o.max(c) + (b.h - b.o.max(b.c)).max(0.0) * f,
        l: b.o.min(c) - (b.o.min(b.c) - b.l).max(0.0) * f,
        c,
        v: b.v * f,
    }
}

struct Inst<'a> {
    inp: &'a InstrumentInput,
    dets: Vec<Box<dyn Detector>>,
    hist: Vec<Bar>,
    facts: Vec<Fact>,
}

#[derive(Default, Clone, Copy)]
struct RunStats {
    /// ns per market tick (detectors + facts + candidates).
    tick: f64,
    /// ns per market tick, detectors + facts only.
    tick_det: f64,
    /// ns per market tick, candidates only.
    tick_cand: f64,
    /// ns per market bar close.
    close: f64,
    /// ns per market tick, bars 60..200 (small ledger).
    tick_early: f64,
    /// ns per market tick, bars 360.. (large ledger).
    tick_late: f64,
    /// ledger rows at the end.
    ledger: usize,
    /// ns per `Engine::ledger_stat` call over the final ledger (one key scan).
    ledger_scan: f64,
    /// p99 of market tick (ns).
    tick_p99: f64,
    /// candidates returned per market tick (mean).
    cands: f64,
    /// open stories at the end.
    open: usize,
    /// events at the end.
    events: usize,
}

#[allow(
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    clippy::needless_range_loop
)]
fn run(inputs: &[InstrumentInput], k_ticks: usize) -> RunStats {
    let mut engine = Engine::new(EngineConfig::r3());
    let mut insts: Vec<Inst<'_>> = inputs
        .iter()
        .map(|inp| Inst {
            inp,
            dets: default_detectors(),
            hist: Vec::with_capacity(inp.bars.len()),
            facts: Vec::new(),
        })
        .collect();
    let n_bars = inputs[0].bars.len();
    let (mut t_det, mut t_cand, mut t_close) = (0u128, 0u128, 0u128);
    let mut tick_lat: Vec<u128> = Vec::new();
    let (mut t_early, mut n_early, mut t_late, mut n_late) = (0u128, 0u64, 0u128, 0u64);
    let (mut n_ticks, mut n_closes, mut n_cands) = (0u64, 0u64, 0u64);
    // Pre-built ticks so their construction is not timed.
    let ticks: Vec<Vec<Vec<Bar>>> = inputs
        .iter()
        .map(|inp| {
            inp.bars
                .iter()
                .map(|b| (1..=k_ticks).map(|k| tick(b, k, k_ticks)).collect())
                .collect()
        })
        .collect();
    for i in 0..n_bars {
        let timed = i >= WARM;
        let idx = i as u64;
        for k in 0..k_ticks {
            // Phase 1: detectors (push on the first tick, update_last after) + facts.
            let t0 = Instant::now();
            for (j, s) in insts.iter_mut().enumerate() {
                let b = &ticks[j][i][k];
                if k == 0 {
                    s.hist.push(b.clone());
                } else {
                    s.hist[i] = b.clone();
                }
                let flow = s.inp.flow.as_ref().and_then(|f| f.get(i));
                s.facts.clear();
                for d in &mut s.dets {
                    let out = if k == 0 {
                        d.push(b, flow)
                    } else {
                        d.update_last(b, flow)
                    };
                    for x in &out {
                        s.facts.push(to_fact(
                            d.as_ref(),
                            x,
                            &s.inp.symbol,
                            &s.inp.series,
                            idx,
                            b.t,
                            ENGINE_VERSION,
                        ));
                    }
                }
            }
            let t1 = Instant::now();
            // Phase 2: candidates on the open bar.
            let mut c = 0usize;
            for s in &insts {
                c += engine
                    .candidates(&s.inp.symbol, &s.inp.series, idx, &s.hist, &s.facts)
                    .len();
            }
            let t2 = Instant::now();
            if timed {
                t_det += (t1 - t0).as_nanos();
                t_cand += (t2 - t1).as_nanos();
                tick_lat.push((t2 - t0).as_nanos());
                if i < 200 {
                    t_early += (t2 - t0).as_nanos();
                    n_early += 1;
                } else if i >= 360 {
                    t_late += (t2 - t0).as_nanos();
                    n_late += 1;
                }
                n_ticks += 1;
                n_cands += c as u64;
            }
        }
        // Bar close: the last tick is the final bar; its detections are the final facts.
        let t0 = Instant::now();
        for s in &insts {
            engine.on_bar(&s.inp.symbol, &s.inp.series, idx, &s.hist, &s.facts);
        }
        let t1 = Instant::now();
        if timed {
            t_close += (t1 - t0).as_nanos();
            n_closes += 1;
        }
    }
    // Cost of one ledger question over the final ledger (the rank step asks one per
    // touched story plus one over all keys).
    let keys: Vec<String> = engine
        .ledger_records()
        .iter()
        .filter_map(|r| r.entry.key.clone())
        .take(8)
        .collect();
    let t_end = inputs[0].bars[n_bars - 1].t + 1;
    let reps = 2000usize;
    let t0 = Instant::now();
    let mut acc = 0u64;
    for r in 0..reps {
        acc += u64::from(engine.ledger_stat(&keys[r % keys.len()], t_end).n);
    }
    let ledger_scan = (t0.elapsed().as_nanos() as f64) / reps as f64;
    assert!(acc > 0);
    tick_lat.sort_unstable();
    let p99 = tick_lat
        .get((tick_lat.len() * 99 / 100).min(tick_lat.len().saturating_sub(1)))
        .copied()
        .unwrap_or(0);
    let nt = n_ticks as f64;
    RunStats {
        tick: (t_det + t_cand) as f64 / nt,
        tick_det: t_det as f64 / nt,
        tick_cand: t_cand as f64 / nt,
        close: t_close as f64 / n_closes as f64,
        tick_early: t_early as f64 / n_early.max(1) as f64,
        tick_late: t_late as f64 / n_late.max(1) as f64,
        ledger: engine.ledger_records().len(),
        ledger_scan,
        tick_p99: p99 as f64,
        cands: n_cands as f64 / nt,
        open: engine.open_stories().len(),
        events: engine.events().len(),
    }
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let k_ticks: usize = args
        .first()
        .map_or(20, |s| s.parse().expect("ticks per bar"));
    let runs: usize = args.get(1).map_or(5, |s| s.parse().expect("runs"));
    let inputs = support::synthetic(1, 30, 500);
    let n = inputs.len();
    // One untimed warm-up run (caches, allocator).
    let _ = run(&inputs, k_ticks);
    let stats: Vec<RunStats> = (0..runs).map(|_| run(&inputs, k_ticks)).collect();
    let m = |f: fn(&RunStats) -> f64| median(stats.iter().map(f).collect());
    #[allow(clippy::cast_precision_loss)]
    let per = |x: f64| x / n as f64;
    println!(
        "engine_cost: {n} instruments x 7 detectors, {} bars, {k_ticks} ticks/bar, {runs} runs (median), bars >= {WARM} timed",
        inputs[0].bars.len()
    );
    println!("| Quantity | Market (30 instr.) | Per instrument |");
    println!("|---|---:|---:|");
    println!(
        "| Tick: detectors + facts + candidates (ns) | {:.0} | {:.0} |",
        m(|s| s.tick),
        per(m(|s| s.tick))
    );
    println!(
        "| — detectors (7 × update_last) + to_fact (ns) | {:.0} | {:.0} |",
        m(|s| s.tick_det),
        per(m(|s| s.tick_det))
    );
    println!(
        "| — Engine::candidates on the open bar (ns) | {:.0} | {:.0} |",
        m(|s| s.tick_cand),
        per(m(|s| s.tick_cand))
    );
    println!(
        "| Tick, bars 60–199 (ns) | {:.0} | {:.0} |",
        m(|s| s.tick_early),
        per(m(|s| s.tick_early))
    );
    println!(
        "| Tick, bars 360–499 (ns) | {:.0} | {:.0} |",
        m(|s| s.tick_late),
        per(m(|s| s.tick_late))
    );
    println!(
        "| Tick p99 (ns) | {:.0} | {:.0} |",
        m(|s| s.tick_p99),
        per(m(|s| s.tick_p99))
    );
    println!(
        "| One ledger_stat over the final ledger (ns) | {:.0} | — |",
        m(|s| s.ledger_scan)
    );
    println!(
        "| Bar close: Engine::on_bar (ns) | {:.0} | {:.0} |",
        m(|s| s.close),
        per(m(|s| s.close))
    );
    println!(
        "candidates per market tick (mean): {:.2}; open stories at end: {}; events: {}; ledger rows at end: {}",
        m(|s| s.cands),
        stats[0].open,
        stats[0].events,
        stats[0].ledger
    );
    for (r, s) in stats.iter().enumerate() {
        println!(
            "run {r}: tick {:.0} ns (det {:.0}, cand {:.0}), close {:.0} ns",
            s.tick, s.tick_det, s.tick_cand, s.close
        );
    }
}
