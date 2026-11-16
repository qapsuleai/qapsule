//! Detection counts and cost per bar of the default detectors (R3/T3).
//!
//! ```sh
//! cargo run --release -p qp-facts --example facts_bench
//! ```
//!
//! Counts: `synthetic_bars(2_000, seed)` for seeds 1, 7, 42; "plain" as generated,
//! "+spikes" with every bar whose LCG draw is `0 mod 53` given 5x volume (the generator's
//! volume never reaches 2.5x its own average). The flow series is synthetic (5% missing
//! points, 2% points without `net`, 7% 8x spikes). Cost: 30 instruments x 5 000 bars,
//! `run_batch` per detector, best of 5 runs, ns per bar.

use std::collections::BTreeMap;
use std::time::Instant;

use qp_facts::{default_detectors, run_batch, FlowPoint};
use qp_schema::Bar;

struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }
    #[allow(clippy::cast_precision_loss)]
    fn unit(&mut self) -> f64 {
        self.next() as f64 / 8_589_934_592.0
    }
}

fn spiked(mut bars: Vec<Bar>, seed: u64) -> Vec<Bar> {
    let mut r = Lcg(seed ^ 0x5eed);
    for b in &mut bars {
        if r.next().is_multiple_of(53) {
            b.v *= 5.0;
        }
    }
    bars
}

fn flow(bars: &[Bar], seed: u64) -> Vec<FlowPoint> {
    let mut r = Lcg(seed ^ 0xf10e);
    bars.iter()
        .map(|b| {
            let k = r.next() % 100;
            let mut values = BTreeMap::new();
            if k >= 7 {
                let mut net = (r.unit() - 0.5) * 2.0e6;
                if k >= 93 {
                    net *= 8.0;
                }
                values.insert("net".to_owned(), net.round());
            }
            FlowPoint { t: b.t, values }
        })
        .collect()
}

fn main() {
    let seeds = [1u64, 7, 42];
    println!("| detector | plain s1 / s7 / s42 | +spikes s1 / s7 / s42 | ns/bar |");
    println!("|---|---|---|---|");
    let instruments: Vec<(Vec<Bar>, Vec<FlowPoint>)> = (0..30u64)
        .map(|s| {
            let b = spiked(qp_source::synthetic_bars(5_000, 1000 + s), 1000 + s);
            let f = flow(&b, 1000 + s);
            (b, f)
        })
        .collect();
    let total_bars: usize = instruments.iter().map(|(b, _)| b.len()).sum();
    for k in 0..default_detectors().len() {
        let id = default_detectors()[k].id();
        let mut plain = Vec::new();
        let mut spikes = Vec::new();
        for s in seeds {
            let b = qp_source::synthetic_bars(2_000, s);
            let f = flow(&b, s);
            plain.push(run_batch(default_detectors()[k].as_mut(), &b, Some(&f)).len());
            let b = spiked(b, s);
            spikes.push(run_batch(default_detectors()[k].as_mut(), &b, Some(&f)).len());
        }
        let mut best = f64::INFINITY;
        let mut sink = 0usize;
        for _ in 0..5 {
            let t0 = Instant::now();
            for (b, f) in &instruments {
                let mut det = default_detectors().swap_remove(k);
                sink += run_batch(det.as_mut(), b, Some(f)).len();
            }
            #[allow(clippy::cast_precision_loss)]
            let ns = t0.elapsed().as_nanos() as f64 / total_bars as f64;
            if ns < best {
                best = ns;
            }
        }
        std::hint::black_box(sink);
        let j = |v: &[usize]| {
            v.iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" / ")
        };
        println!("| `{id}` | {} | {} | {best:.1} |", j(&plain), j(&spikes));
    }
}
