//! `harness-sim` — R3 measurement harness M3–M6 (see README.md).
//!
//! ```text
//! harness-sim list
//! harness-sim <m3|m4|m5|m6> run   --alt <ad> --data synthetic|real [--seeds 1,2,3]
//! harness-sim <m3|m4|m5|m6> score --alt <ad> --data synthetic|real
//! ```

use std::process::ExitCode;
use std::time::Instant;

use harness_sim::common::{DataKind, SEEDS};
use harness_sim::{m3, m4, m5, m6, registry};

fn usage() -> ExitCode {
    eprintln!(
        "kullanım:\n  harness-sim list\n  harness-sim <m3|m4|m5|m6> run --alt <ad> --data synthetic|real [--seeds 1,2,3]\n  harness-sim <m3|m4|m5|m6> score --alt <ad> --data synthetic|real"
    );
    ExitCode::from(2)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("list") {
        let names = |v: Vec<&'static str>| v.join(", ");
        println!(
            "m3: {}",
            names(registry::rankers().into_iter().map(|x| x.0).collect())
        );
        println!(
            "m4: {}",
            names(registry::themes().into_iter().map(|x| x.0).collect())
        );
        println!(
            "m5: {}",
            names(registry::outcomes().into_iter().map(|x| x.0).collect())
        );
        println!(
            "m6: {}",
            names(registry::trust_paths().into_iter().map(|x| x.0).collect())
        );
        return ExitCode::SUCCESS;
    }
    if args.len() < 2 {
        return usage();
    }
    let exp = args[0].as_str();
    let cmd = args[1].as_str();
    let mut alt = None;
    let mut data = None;
    let mut seeds: Vec<u64> = SEEDS.to_vec();
    let mut i = 2;
    while i < args.len() {
        match (args[i].as_str(), args.get(i + 1)) {
            ("--alt", Some(v)) => alt = Some(v.clone()),
            ("--data", Some(v)) => data = DataKind::parse(v),
            ("--seeds", Some(v)) => {
                seeds = v.split(',').filter_map(|s| s.trim().parse().ok()).collect();
            }
            _ => return usage(),
        }
        i += 2;
    }
    let (Some(alt), Some(data)) = (alt, data) else {
        return usage();
    };
    let t0 = Instant::now();
    let res = match (exp, cmd) {
        ("m3", "run") => m3::cmd_run(&alt, data, &seeds),
        ("m3", "score") => m3::cmd_score(&alt, data),
        ("m4", "run") => m4::cmd_run(&alt, data, &seeds),
        ("m4", "score") => m4::cmd_score(&alt, data),
        ("m5", "run") => m5::cmd_run(&alt, data, &seeds),
        ("m5", "score") => m5::cmd_score(&alt, data),
        ("m6", "run") => m6::cmd_run(&alt, data, &seeds),
        ("m6", "score") => m6::cmd_score(&alt, data),
        _ => return usage(),
    };
    match res {
        Ok(md) => {
            println!("{md}");
            // Wall time goes to stderr only: it is never part of a raw record.
            eprintln!("süre: {:.1} s", t0.elapsed().as_secs_f64());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("hata: {e}");
            ExitCode::FAILURE
        }
    }
}
