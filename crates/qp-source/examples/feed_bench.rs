//! Quick native measurement of the feed store (RP2/T3, `rounds/RP2/experiments/T3.md`):
//! event apply cost and digest update cost (append / live update / correction) on
//! 10 000 bars. Run: `cargo run --release -p qp-source --example feed_bench`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::cognitive_complexity
)]

use std::hint::black_box;
use std::time::Instant;

use qp_schema::data::{data_digest_u64, BarPatch, DataChange, DataEvent, FeedKey};
use qp_source::{synthetic_bars, FeedStore, SyntheticEvents, SyntheticEventsConfig};

fn key() -> FeedKey {
    FeedKey {
        symbol: "BENCH".into(),
        interval: "1d".into(),
        feed: None,
    }
}

fn per_op<F: FnMut(u64)>(label: &str, reps: u64, mut f: F) -> f64 {
    let start = Instant::now();
    for i in 0..reps {
        f(i);
    }
    let ns = start.elapsed().as_nanos() as f64 / reps as f64;
    println!("{label:<58} {ns:>10.1} ns");
    ns
}

fn main() {
    // 1. Event apply: 10 000 bars x 10 ticks, events pre-generated.
    let events: Vec<DataEvent> =
        SyntheticEvents::new(key(), SyntheticEventsConfig::new(10_000, 1, 10)).collect();
    let n_ev = events.len() as u64;
    let mut best = f64::MAX;
    for _ in 0..5 {
        let mut s = FeedStore::new(key());
        let start = Instant::now();
        for e in &events {
            black_box(s.apply(e).unwrap());
        }
        best = best.min(start.elapsed().as_nanos() as f64 / n_ev as f64);
    }
    println!("apply, field-level stream ({n_ev} events, best of 5) {best:>10.1} ns/event");

    // 2. Digest on 10 000 bars.
    let bars = synthetic_bars(10_000, 2);
    per_op("data_digest from scratch, 10k bars", 200, |_| {
        black_box(data_digest_u64(black_box(&bars)));
    });

    let mut s = FeedStore::with_bars(key(), bars.clone(), 0).unwrap();
    s.position();
    let mut seq = 0;
    let mut t = bars.last().unwrap().t;
    per_op("append 1 bar + position()", 10_000, |_| {
        seq += 1;
        t += 86_400_000;
        let b = qp_schema::Bar {
            t,
            o: 1.0,
            h: 2.0,
            l: 0.5,
            c: 1.5,
            v: 1.0,
        };
        s.apply(&DataEvent {
            feed: key(),
            seq,
            change: DataChange::Bar { bar: b },
        })
        .unwrap();
        black_box(s.position());
    });
    let mut s = FeedStore::with_bars(key(), bars.clone(), 0).unwrap();
    s.position();
    let mut seq = 0;
    let last_t = bars.last().unwrap().t;
    let last = bars.last().unwrap().clone();
    per_op("live update (last bar) + position()", 100_000, |i| {
        seq += 1;
        let c = if i % 2 == 0 { last.l } else { last.h };
        s.apply(&DataEvent {
            feed: key(),
            seq,
            change: DataChange::Patch {
                t: last_t,
                fields: BarPatch {
                    c: Some(c),
                    ..BarPatch::default()
                },
            },
        })
        .unwrap();
        black_box(s.position());
    });
    for (label, idx) in [
        ("correction at bar 0 + position()", 0usize),
        ("correction at bar 5000 + position()", 5_000),
        ("correction at bar 9990 + position()", 9_990),
    ] {
        let mut s = FeedStore::with_bars(key(), bars.clone(), 0).unwrap();
        s.position();
        let mut seq = 0;
        let b = bars[idx].clone();
        per_op(label, 2_000, |i| {
            seq += 1;
            let c = if i % 2 == 0 { b.l } else { b.h };
            s.apply(&DataEvent {
                feed: key(),
                seq,
                change: DataChange::Patch {
                    t: b.t,
                    fields: BarPatch {
                        c: Some(c),
                        ..BarPatch::default()
                    },
                },
            })
            .unwrap();
            black_box(s.position());
        });
    }
    let mut s = FeedStore::with_bars(key(), bars.clone(), 0).unwrap();
    s.position();
    let mut seq = 0;
    per_op("live update, no position() (apply only)", 100_000, |i| {
        seq += 1;
        let c = if i % 2 == 0 { last.l } else { last.h };
        black_box(
            s.apply(&DataEvent {
                feed: key(),
                seq,
                change: DataChange::Patch {
                    t: last_t,
                    fields: BarPatch {
                        c: Some(c),
                        ..BarPatch::default()
                    },
                },
            })
            .unwrap(),
        );
    });
    per_op("index_of (binary search), 10k bars", 1_000_000, |i| {
        black_box(s.index_of(bars[(i as usize * 7919) % 10_000].t));
    });
}
