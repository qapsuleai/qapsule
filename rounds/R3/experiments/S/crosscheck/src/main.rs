//! `crosscheck` command line (R3/S-A).
//!
//! ```text
//! crosscheck fixture          # (re)write crates/qp-capsule/tests/fixtures/m2_sentences.json
//! crosscheck diff             # changed situations: measured vs product, TR and EN
//! crosscheck sim [seeds…]     # simulate product vs experiment config (default 1 2 3)
//! ```

use std::process::ExitCode;

use crosscheck::{fixtures_dir, pretty, sentence_fixture, sim_check, support, CHANGED};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("fixture") => {
            let f = sentence_fixture();
            let p = fixtures_dir().join("m2_sentences.json");
            std::fs::write(&p, pretty(&f)).expect("write fixture");
            println!(
                "wrote {} ({} situations, {} changed)",
                p.display(),
                f.measured.len(),
                f.product_changes.len()
            );
        }
        Some("diff") => {
            let f = sentence_fixture();
            for (id, p) in &f.product_changes {
                let m = &f.measured[id];
                println!("### {id}\n");
                if m.tr != p.tr {
                    println!("- TR önce: {}\n- TR sonra: {}", m.tr, p.tr);
                }
                if m.en != p.en {
                    println!("- EN before: {}\n- EN after: {}", m.en, p.en);
                }
                println!();
            }
            let ids: Vec<&str> = f.product_changes.keys().map(String::as_str).collect();
            println!("changed: {} ({ids:?}); expected {CHANGED:?}", ids.len());
        }
        Some("sim") => {
            let seeds: Vec<u64> = if args.len() > 1 {
                args[1..].iter().map(|s| s.parse().expect("seed")).collect()
            } else {
                vec![1, 2, 3]
            };
            let mut ok = true;
            for s in seeds {
                let inputs = support::synthetic(s, 30, 500);
                let c = sim_check(&format!("synthetic-30x500-s{s}"), &inputs);
                ok &= c.events_equal && c.ledger_equal && c.facts_equal;
                println!("{}", crosscheck::compact(&c));
            }
            if !ok {
                return ExitCode::FAILURE;
            }
        }
        _ => {
            eprintln!("usage: crosscheck fixture | diff | sim [seeds…]");
            return ExitCode::FAILURE;
        }
    }
    ExitCode::SUCCESS
}
