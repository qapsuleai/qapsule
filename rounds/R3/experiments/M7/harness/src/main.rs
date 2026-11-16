//! `harness` command line.
//!
//! ```text
//! harness list
//! harness spec <S#>                         # preset as JSON (edit it for a custom scenario)
//! harness run --approach <name> [--scenario <S#|all>]... [--spec file.json]...
//!             [--seeds N] [--seed-base B] [--threads T] [--out results.json]
//! harness log --approach <name> (--scenario <S#> | --spec f.json) --seed S
//! harness determinism --approach <name> (--scenario <S#> | --spec f.json) [--seeds N]
//! ```

use std::process::ExitCode;

use harness::approaches;
use harness::metrics::Summary;
use harness::scenarios;
use harness::spec::ScenarioSpec;

struct Args {
    cmd: String,
    approach: String,
    scenarios: Vec<String>,
    specs: Vec<String>,
    seeds: Option<u64>,
    seed_base: u64,
    seed: u64,
    threads: usize,
    out: Option<String>,
}

fn parse() -> Result<Args, String> {
    let mut it = std::env::args().skip(1);
    let cmd = it
        .next()
        .ok_or("missing command (list, spec, run, log, determinism)")?;
    let mut a = Args {
        cmd,
        approach: "n".into(),
        scenarios: Vec::new(),
        specs: Vec::new(),
        seeds: None,
        seed_base: 0,
        seed: 0,
        threads: std::thread::available_parallelism().map_or(4, std::num::NonZero::get),
        out: None,
    };
    let mut positional = Vec::new();
    while let Some(x) = it.next() {
        let mut val = || it.next().ok_or(format!("{x} needs a value"));
        match x.as_str() {
            "--approach" => a.approach = val()?,
            "--scenario" => a.scenarios.push(val()?),
            "--spec" => a.specs.push(val()?),
            "--seeds" => a.seeds = Some(val()?.parse().map_err(|e| format!("--seeds: {e}"))?),
            "--seed-base" => {
                a.seed_base = val()?.parse().map_err(|e| format!("--seed-base: {e}"))?
            }
            "--seed" => a.seed = val()?.parse().map_err(|e| format!("--seed: {e}"))?,
            "--threads" => a.threads = val()?.parse().map_err(|e| format!("--threads: {e}"))?,
            "--out" => a.out = Some(val()?),
            _ if x.starts_with("--") => return Err(format!("unknown option {x}")),
            _ => positional.push(x),
        }
    }
    a.scenarios.extend(positional);
    Ok(a)
}

fn load_specs(a: &Args) -> Result<Vec<ScenarioSpec>, String> {
    let mut v = Vec::new();
    for s in &a.scenarios {
        if s.eq_ignore_ascii_case("all") {
            v.extend(scenarios::presets());
        } else {
            v.push(scenarios::find(s).ok_or(format!("unknown scenario {s}"))?);
        }
    }
    for p in &a.specs {
        let text = std::fs::read_to_string(p).map_err(|e| format!("{p}: {e}"))?;
        let spec: ScenarioSpec = serde_json::from_str(&text).map_err(|e| format!("{p}: {e}"))?;
        v.push(spec);
    }
    for s in &v {
        s.validate().map_err(|e| format!("{}: {e}", s.name))?;
    }
    if v.is_empty() {
        return Err("no scenario (use --scenario S# | all, or --spec file.json)".into());
    }
    Ok(v)
}

fn header() -> String {
    format!(
        "{:<5} {:>5} {:>5} {:>6} {:>5} | {:>5} {:>6} {:>9} {:>5} {:>5} | {:>5} {:>9} | {:>17} {:>17} | {:>9} {:>4} | {:>5} | {:>7} {:>8} | {:>6} | {:>13} | {:>5} {:>4} | {:>9}",
        "scen", "seeds", "fail", "live", "failL",
        "M1", "M2", "M2ms", "M3f", "M3p",
        "M4", "M5",
        "M6p p50/95/99", "M6f p50/95/99",
        "M7ms", "M7>",
        "M8",
        "M9B/cl", "M9B/s",
        "M10w/e",
        "M11 p50/un/x",
        "M12", "max",
        "M13ms"
    )
}

fn row(s: &Summary) -> String {
    let p = |x: &harness::metrics::Pct| {
        if x.n == 0 {
            "-".to_owned()
        } else {
            format!("{:.0}/{:.0}/{:.0}", x.p50, x.p95, x.p99)
        }
    };
    format!(
        "{:<5} {:>5} {:>5} {:>6} {:>5} | {:>5} {:>6} {:>9.0} {:>5} {:>5} | {:>5} {:>9} | {:>17} {:>17} | {:>9.0} {:>4} | {:>5} | {:>7.0} {:>8.1} | {:>6.1} | {:>13} | {:>5} {:>4} | {:>9.0}",
        s.scenario,
        s.seeds,
        s.failing_seeds,
        s.live_errors,
        s.failing_seeds_live,
        s.m1_early,
        s.m2_contradictory,
        s.m2_contradictory_ms,
        s.m3_wrong_final,
        s.m3_wrong_provisional,
        s.m4_retracted_provisional,
        format!("{}/{}", s.m5_missed, s.m5_eligible),
        p(&s.m6_provisional),
        p(&s.m6_final),
        s.m7_stale_ms,
        s.m7_stale_over,
        s.m8_blocked,
        s.m9_a2r_bytes_per_claim,
        s.m9_a2r_bytes_per_sec,
        s.m10_work_per_event,
        format!(
            "{}/{}/{}",
            if s.m11_latency.n == 0 {
                "-".to_owned()
            } else {
                format!("{:.0}", s.m11_latency.p50)
            },
            s.m11_unresolved,
            s.m11_unneeded
        ),
        s.m12_flips,
        s.m12_max_flips,
        s.m13_dispute_ms
    )
}

fn real_main() -> Result<(), String> {
    let a = parse()?;
    match a.cmd.as_str() {
        "list" => {
            println!("approaches:");
            for x in approaches::registry() {
                println!("  {:<6} {}", x.name, x.summary);
            }
            println!("scenarios:");
            for s in scenarios::presets() {
                println!(
                    "  {:<5} [{} seeds] {}",
                    s.name, s.default_seeds, s.description
                );
            }
            Ok(())
        }
        "spec" => {
            for s in load_specs(&a)? {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&s).map_err(|e| e.to_string())?
                );
            }
            Ok(())
        }
        "run" => {
            let ap =
                approaches::find(&a.approach).ok_or(format!("unknown approach {}", a.approach))?;
            let specs = load_specs(&a)?;
            println!("approach {} — {}", ap.name, ap.summary);
            println!("{}", header());
            let mut results = Vec::new();
            for spec in &specs {
                // A preset with fewer than 100 default seeds (S17, load) caps --seeds.
                let def = u64::from(spec.default_seeds);
                let seeds = match a.seeds {
                    Some(n) if def < 100 => n.min(def),
                    Some(n) => n,
                    None => def,
                };
                let (runs, sum) = harness::run_scenario(spec, &ap, a.seed_base, seeds, a.threads);
                println!("{}", row(&sum));
                eprintln!(
                    "  {} done in {:.1}s; worst seed {:?}",
                    spec.name, sum.wall_secs, sum.worst_seed
                );
                results.push(serde_json::json!({
                    "spec": spec,
                    "summary": sum,
                    "runs": runs,
                }));
            }
            let path = a
                .out
                .clone()
                .unwrap_or_else(|| format!("harness-results-{}.json", ap.name));
            let doc = serde_json::json!({ "approach": ap.name, "results": results });
            std::fs::write(
                &path,
                serde_json::to_vec_pretty(&doc).map_err(|e| e.to_string())?,
            )
            .map_err(|e| format!("{path}: {e}"))?;
            eprintln!("results written to {path}");
            Ok(())
        }
        "log" => {
            let ap =
                approaches::find(&a.approach).ok_or(format!("unknown approach {}", a.approach))?;
            let spec = load_specs(&a)?.remove(0);
            let out = harness::sim::run_one(&spec, &ap, a.seed);
            print!("{}", String::from_utf8_lossy(&out.log));
            for n in &out.notes {
                eprintln!("note: {n}");
            }
            for (side, b, st) in &out.feed_stats {
                eprintln!("feed {side:?} b{b}: {st:?}");
            }
            eprintln!(
                "{}",
                serde_json::to_string_pretty(&out.metrics).map_err(|e| e.to_string())?
            );
            Ok(())
        }
        "determinism" => {
            let ap =
                approaches::find(&a.approach).ok_or(format!("unknown approach {}", a.approach))?;
            let mut bad = 0;
            for spec in load_specs(&a)? {
                let n = a.seeds.unwrap_or(5);
                for seed in a.seed_base..a.seed_base + n {
                    let x = harness::sim::run_one(&spec, &ap, seed);
                    let y = harness::sim::run_one(&spec, &ap, seed);
                    let same = x.log == y.log;
                    println!(
                        "{} seed {seed}: {} lines, hash {} / {} -> {}",
                        spec.name,
                        x.metrics.log_lines,
                        x.metrics.log_hash,
                        y.metrics.log_hash,
                        if same { "identical" } else { "DIFFERENT" }
                    );
                    if !same {
                        bad += 1;
                    }
                }
            }
            if bad > 0 {
                return Err(format!("{bad} runs not byte-identical"));
            }
            Ok(())
        }
        other => Err(format!("unknown command {other}")),
    }
}

fn main() -> ExitCode {
    match real_main() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
