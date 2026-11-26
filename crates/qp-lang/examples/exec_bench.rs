//! Per-bar cost of the executor on the PR §12.5 examples (and the rest of the digest
//! suite): ns per closed bar (`push_closed`), ns per live tick (`update_last`, a pure
//! evaluation), on 100 000 hourly UTC bars. Release build:
//!
//! ```sh
//! cargo run --release -p qp-lang --example exec_bench
//! ```

#![allow(clippy::cast_precision_loss, clippy::unwrap_used, clippy::expect_used)]

use std::time::Instant;

use qp_lang::{check, Binding, CheckCtx, ExecOptions, Executor, Limit, Limits, DIGEST_SUITE};
use qp_schema::Bar;

const NAMES: [&str; 9] = [
    "12.5 indicator",
    "12.5 condition",
    "12.5 signal",
    "12.5 higher tf",
    "14 sma",
    "14 ema+rsi",
    "14 macd+bollinger",
    "every node",
    "arithmetic",
];

fn bars(n: usize) -> Vec<Bar> {
    let mut s: u64 = 0x2545_F491_4F6C_DD1D;
    let mut c = 100.0;
    (0..n)
        .map(|i| {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            let u = (s >> 11) as f64 / (1u64 << 53) as f64;
            let o = c;
            c *= 1.0 + (u - 0.5) * 0.04;
            Bar {
                t: 1_704_067_200_000 + i as i64 * 3_600_000,
                o,
                h: o.max(c) * 1.004,
                l: o.min(c) * 0.996,
                c,
                v: 100.0,
            }
        })
        .collect()
}

fn main() {
    let n = 100_000;
    let b = bars(n);
    let limits = Limits::default().with(Limit::MaxCostPerBar, 10_000);
    println!("| program | per_bar | closed ns/bar | live ns/tick |");
    println!("|---|---|---|---|");
    for (name, p) in NAMES.iter().zip(DIGEST_SUITE) {
        let v: serde_json::Value = serde_json::from_str(p).unwrap();
        let ctx = CheckCtx {
            limits,
            binding: Some(Binding {
                interval: Some("1h".to_owned()),
                bars: None,
            }),
            ..CheckCtx::default()
        };
        let c = check(&v, &ctx).checked.expect("checks");
        let mut best = f64::INFINITY;
        for _ in 0..3 {
            let t = Instant::now();
            let x = Executor::run_closed(&c, &b, ExecOptions::default()).unwrap();
            best = best.min(t.elapsed().as_nanos() as f64 / n as f64);
            std::hint::black_box(x.closed_len());
        }
        let mut x = Executor::run_closed(&c, &b[..n - 1], ExecOptions::default()).unwrap();
        x.push(b[n - 1].clone()).unwrap();
        let ticks = 100_000;
        let t = Instant::now();
        for k in 0..ticks {
            let mut bar = b[n - 1].clone();
            bar.c *= 1.0 + (k % 7) as f64 * 1e-5;
            x.update_last(bar).unwrap();
            std::hint::black_box(x.value_at(0, n - 1));
        }
        let tick = t.elapsed().as_nanos() as f64 / f64::from(ticks);
        println!("| {name} | {} | {best:.0} | {tick:.0} |", c.cost.per_bar);
    }
    session(limits);
}

/// The higher-timeframe example on XIST session bars (calendar alignment).
fn session(limits: Limits) {
    let cal = qp_calendar::ExchangeCalendar::new(&qp_calendar::builtin("XIST").unwrap()).unwrap();
    let ts: Vec<i64> = cal
        .bars_between(
            qp_calendar::Interval::Minutes(60),
            1_514_764_800_000,
            1_798_761_600_000,
        )
        .iter()
        .map(|s| s.open)
        .collect();
    let mut b = bars(ts.len());
    for (bar, t) in b.iter_mut().zip(&ts) {
        bar.t = *t;
    }
    let v: serde_json::Value = serde_json::from_str(DIGEST_SUITE[3]).unwrap();
    let ctx = CheckCtx {
        limits,
        binding: Some(Binding {
            interval: Some("1h".to_owned()),
            bars: None,
        }),
        ..CheckCtx::default()
    };
    let c = check(&v, &ctx).checked.expect("checks");
    let opts = ExecOptions {
        align: qp_lang::Align::for_calendar(Some("XIST")),
        ..ExecOptions::default()
    };
    let t = Instant::now();
    let x = Executor::run_closed(&c, &b, opts).unwrap();
    let ns = t.elapsed().as_nanos() as f64 / b.len() as f64;
    println!(
        "| 12.5 higher tf, XIST session ({} bars) | {} | {ns:.0} | — |",
        x.closed_len(),
        c.cost.per_bar
    );
}
