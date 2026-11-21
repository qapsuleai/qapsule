//! `harness` — RP4 measurement harness (see README.md).
//!
//! ```text
//! harness list
//! harness prepare --rest <dir> --out <dir> [--universe <file>]
//! harness m25 --data <dir> --rule <ad> [--reference <ad>] --period 5m|15m|1h|1d [--split calib|test] [--out <dir>]
//! harness m26 --data <dir> --detectors <ad> --period … [--rule <ad>] [--split …] [--out <dir>]
//! harness report [--results <dir>] --out <dir>
//! harness ticks --raw <dir> [--universe <file>]
//! harness synth --out <dir> [--symbols N]
//! ```

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

use rp4_harness::common::{default_local_root, to_json};
use rp4_harness::data::{Period, Split};
use rp4_harness::{data, m25, m26, registry, report, synth, tick};

const USAGE: &str = "kullanım:
  harness list
  harness prepare --rest <dir> --out <dir> [--universe <file>]
  harness m25 --data <dir> --rule <ad> [--reference <ad>] --period 5m|15m|1h|1d [--split calib|test] [--out <dir>]
  harness m26 --data <dir> --detectors <ad> --period 5m|15m|1h|1d [--rule <ad>] [--split calib|test] [--out <dir>]
  harness cases --data <dir> --rule <ad> --period 5m --split test [--day YYYY-MM-DD] --out <dir>
  harness report [--results <dir>] --out <dir>
  harness ticks --raw <dir> [--universe <file>]
  harness synth --out <dir> [--symbols N]";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(cmd) = args.first().cloned() else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let mut opt: BTreeMap<String, String> = BTreeMap::new();
    let mut i = 1;
    while i < args.len() {
        let (Some(k), Some(v)) = (args[i].strip_prefix("--"), args.get(i + 1)) else {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        };
        opt.insert(k.to_owned(), v.clone());
        i += 2;
    }
    let path = |k: &str| opt.get(k).map(PathBuf::from);
    let results = || path("out").unwrap_or_else(|| default_local_root().join("results"));
    let period = opt.get("period").and_then(|p| Period::parse(p));
    let split = match opt.get("split") {
        None => Some(Split::Calib),
        Some(s) => Split::parse(s),
    };
    let t0 = Instant::now();
    let res: Result<String, String> = match cmd.as_str() {
        "list" => Ok(format!(
            "kurallar: {}\ndedektör setleri: {}",
            registry::rules()
                .iter()
                .map(|r| r.name)
                .collect::<Vec<_>>()
                .join(", "),
            registry::detector_sets()
                .iter()
                .map(|r| r.name)
                .collect::<Vec<_>>()
                .join(", ")
        )),
        "prepare" => match (path("rest"), path("out")) {
            (Some(rest), Some(out)) => {
                data::prepare(&rest, &out, path("universe").as_deref()).map(|m| to_json(&m))
            }
            _ => Err(USAGE.to_owned()),
        },
        "m25" => match (path("data"), opt.get("rule"), period, split) {
            (Some(d), Some(rule), Some(p), Some(s)) => {
                let reference = opt.get("reference").map_or("r3", String::as_str);
                m25::cmd(&d, rule, reference, p, s, &results())
            }
            _ => Err(USAGE.to_owned()),
        },
        "cases" => match (path("data"), opt.get("rule"), period, split, path("out")) {
            (Some(d), Some(rule), Some(p), Some(s), Some(o)) => {
                rp4_harness::cases::cmd(&d, rule, p, s, opt.get("day").map(String::as_str), &o)
            }
            _ => Err(USAGE.to_owned()),
        },
        "m26" => match (path("data"), opt.get("detectors"), period, split) {
            (Some(d), Some(det), Some(p), Some(s)) => {
                let rule = opt.get("rule").map_or("r3", String::as_str);
                m26::cmd(&d, det, rule, p, s, &results())
            }
            _ => Err(USAGE.to_owned()),
        },
        "report" => match path("out") {
            Some(out) => {
                let r = path("results").unwrap_or_else(|| default_local_root().join("results"));
                report::cmd(&r, &out)
            }
            None => Err(USAGE.to_owned()),
        },
        "ticks" => match path("raw") {
            Some(raw) => {
                let uni: Option<Vec<String>> = path("universe").map(|u| {
                    std::fs::read_to_string(u)
                        .unwrap_or_default()
                        .lines()
                        .map(|l| l.trim().to_owned())
                        .filter(|l| !l.is_empty())
                        .collect()
                });
                tick::check_raw(&raw, uni.as_deref()).map(|c| to_json(&c))
            }
            None => Err(USAGE.to_owned()),
        },
        "synth" => match path("out") {
            Some(out) => {
                let n = opt.get("symbols").and_then(|s| s.parse().ok()).unwrap_or(6);
                synth::write_rest_dir(&out, n, "2025-11-03", "2023-01-02", "2026-11-20", 1)
                    .map(|v| format!("{} sembol → {}", v.len(), out.display()))
                    .map_err(|e| e.to_string())
            }
            None => Err(USAGE.to_owned()),
        },
        _ => Err(USAGE.to_owned()),
    };
    match res {
        Ok(s) => {
            println!("{s}");
            // Wall time goes to stderr only; never part of a result file.
            eprintln!("süre: {:.1} s", t0.elapsed().as_secs_f64());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("hata: {e}");
            ExitCode::FAILURE
        }
    }
}
