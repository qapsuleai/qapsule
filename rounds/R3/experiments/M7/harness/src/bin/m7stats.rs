//! R3/M7: extra counters of approach `bs2` (disputes sent/upheld/rejected, withdrawals
//! with a reason, correction latency, settled fetches), per scenario over seeds. The
//! harness's metrics are not touched: this binary only reads the approach's own
//! per-thread counters around `sim::run_one`.
//!
//! usage: m7stats [--scenario S]… [--spec file.json]… [--seeds N] [--threads T]

use std::collections::BTreeMap;

use harness::approaches::bs2::{take_stats, Stats};
use harness::spec::ScenarioSpec;
use harness::{approaches, scenarios, sim};

fn pct(v: &[i64], p: f64) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    let i = ((v.len() as f64 - 1.0) * p).round() as usize;
    v[i] as f64 / 1000.0
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut specs: Vec<ScenarioSpec> = Vec::new();
    let mut seeds: u64 = 100;
    let mut threads: usize = 8;
    let mut i = 0;
    while i < args.len() {
        let v = args.get(i + 1).cloned().unwrap_or_default();
        match args[i].as_str() {
            "--scenario" if v == "all" => specs.extend(scenarios::presets()),
            "--scenario" => specs.push(scenarios::find(&v).expect("scenario")),
            "--spec" => specs.push(
                serde_json::from_str(&std::fs::read_to_string(&v).expect("spec")).expect("spec"),
            ),
            "--seeds" => seeds = v.parse().expect("seeds"),
            "--threads" => threads = v.parse().expect("threads"),
            a => panic!("unknown argument {a}"),
        }
        i += 2;
    }
    let ap = approaches::find("bs2").expect("bs2");
    let mut out = BTreeMap::new();
    for spec in &specs {
        let n = seeds.min(u64::from(spec.default_seeds));
        let all: Vec<u64> = (0..n).collect();
        let mut per: Vec<(u64, Stats)> = Vec::new();
        std::thread::scope(|sc| {
            let hs: Vec<_> = (0..threads)
                .map(|t| {
                    let chunk: Vec<u64> = all.iter().copied().skip(t).step_by(threads).collect();
                    let spec = spec.clone();
                    sc.spawn(move || {
                        chunk
                            .into_iter()
                            .map(|s| {
                                let _ = take_stats();
                                let _ = sim::run_one(&spec, &ap, s);
                                (s, take_stats())
                            })
                            .collect::<Vec<_>>()
                    })
                })
                .collect();
            for h in hs {
                per.extend(h.join().expect("worker"));
            }
        });
        per.sort_by_key(|(s, _)| *s);
        let mut tot = Stats::default();
        for (_, s) in &per {
            tot.disputes_sent += s.disputes_sent;
            tot.disputes_in += s.disputes_in;
            tot.fetches += s.fetches;
            for (k, v) in &s.verdicts {
                *tot.verdicts.entry(k.clone()).or_default() += v;
            }
            for (k, v) in &s.withdrawn {
                *tot.withdrawn.entry(k.clone()).or_default() += v;
            }
            for (k, v) in &s.asserted {
                *tot.asserted.entry(k.clone()).or_default() += v;
            }
            tot.correction_us.extend(&s.correction_us);
        }
        let mut lat = tot.correction_us.clone();
        lat.sort_unstable();
        let row = serde_json::json!({
            "seeds": n,
            "disputes_sent": tot.disputes_sent,
            "disputes_received": tot.disputes_in,
            "verdicts": tot.verdicts,
            "withdrawn_with_reason": tot.withdrawn,
            "asserted_with_reason": tot.asserted,
            "settled_fetches": tot.fetches,
            "correction_ms": {"n": lat.len(), "p50": pct(&lat, 0.5), "p95": pct(&lat, 0.95),
                              "max": pct(&lat, 1.0)},
        });
        println!("{} {}", spec.name, row);
        out.insert(spec.name.clone(), row);
    }
    eprintln!("{}", serde_json::to_string(&out).expect("json"));
}
