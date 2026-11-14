//! RD1/T6a experiment driver. See `rounds/RD1/experiments/T6a-concurrency.md`.

mod am_model;
mod cases;
mod common;
mod log_model;
mod loro_model;
mod sim;

use std::path::PathBuf;

use am_model::AmModel;
use log_model::LogModel;
use loro_model::LoroModel;
use sim::{run, Model, RunResult};

type ABase = LogModel<true, true>;
type AStrict = LogModel<true, false>;
type ANoBase = LogModel<false, false>;

fn out_dir() -> PathBuf {
    let p = PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../rounds/RD1/experiments/raw/t6a"
    ));
    std::fs::create_dir_all(&p).expect("mkdir");
    p
}

fn timed<M: Model>(w: &common::Workload, doc: &qp_schema::Document, reps: usize) -> (RunResult, Vec<f64>) {
    let mut times = Vec::new();
    let mut first: Option<RunResult> = None;
    for _ in 0..reps {
        let r = run::<M>(w, doc);
        times.push(r.replicate_time_us);
        if let Some(f) = &first {
            assert_eq!(f.history_sha, r.history_sha, "{}: nondeterministic history", M::NAME);
            assert_eq!(f.final_snap_sha, r.final_snap_sha, "{}: nondeterministic state", M::NAME);
        } else {
            first = Some(r);
        }
    }
    times.sort_by(|a, b| a.partial_cmp(b).expect("cmp"));
    let mut r = first.expect("one run");
    r.replicate_time_us = times[times.len() / 2];
    (r, times)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map(String::as_str).unwrap_or("all");
    let doc = common::start_document();
    let dir = out_dir();
    if mode == "cases" || mode == "all" {
        let res = cases::run_all(&doc);
        std::fs::write(dir.join("cases.json"), serde_json::to_string_pretty(&res).expect("j"))
            .expect("write");
        cases::print(&res);
    }
    if mode == "runs" || mode == "all" {
        let reps: usize = std::env::var("REPS").ok().and_then(|s| s.parse().ok()).unwrap_or(5);
        let seed = 0x5EED_0001_u64;
        let max: usize = std::env::var("MAXOPS").ok().and_then(|s| s.parse().ok()).unwrap_or(10_000);
        let configs: Vec<(usize, usize)> = vec![
            (100, 10),
            (1_000, 10),
            (1_000, 100),
            (10_000, 10),
            (10_000, 100),
            (10_000, 1_000),
        ]
        .into_iter()
        .filter(|(o, _)| *o <= max)
        .collect();
        let mut all = Vec::new();
        for (ops, win) in configs {
            let w = common::workload(seed, ops, win);
            let reps = if ops > 1_000 {
                std::env::var("REPS_BIG").ok().and_then(|s| s.parse().ok()).unwrap_or(3)
            } else {
                reps
            };
            // ONLY=<substring> restricts the run to matching models (side measurements).
            let only = std::env::var("ONLY").unwrap_or_default();
            let want = |n: &str| only.is_empty() || n.contains(&only);
            let mut rs = Vec::new();
            if want(ABase::NAME) {
                rs.push(timed::<ABase>(&w, &doc, reps));
            }
            if want(AStrict::NAME) {
                rs.push(timed::<AStrict>(&w, &doc, reps));
            }
            if want(ANoBase::NAME) {
                rs.push(timed::<ANoBase>(&w, &doc, reps));
            }
            if want(AmModel::NAME) {
                rs.push(timed::<AmModel>(&w, &doc, reps));
            }
            if want(LoroModel::NAME) {
                rs.push(timed::<LoroModel>(&w, &doc, reps));
            }
            for (r, times) in rs {
                println!(
                    "{:>6} ops win {:>5} {:<22} conv={} anom={:>4} applied={:>5} retried={:>4} refused={:>4} up_med={:>5}B down_med={:>5}B hist={:>9}B gz={:>8}B rc_up={:>7}B rc_down={:>8}B t={:>10.0}us",
                    r.ops, r.window, r.model, r.converged, r.anomaly_total, r.applied,
                    r.applied_after_retry, r.refused, r.wire_up_median, r.wire_down_median,
                    r.history_bytes, r.history_gzip, r.reconnect.bytes_up, r.reconnect.bytes_down,
                    r.replicate_time_us
                );
                all.push(serde_json::json!({"result": r, "times_us": times}));
            }
        }
        let name = std::env::var("RUNS_OUT").unwrap_or_else(|_| "runs.json".into());
        std::fs::write(dir.join(name), serde_json::to_string_pretty(&all).expect("j"))
            .expect("write");
    }
}
