//! `harness` command line (R3/H, M1 and M2).
//!
//! ```text
//! harness m1 list
//! harness m1 measure --alt <name>
//! harness m1 sheets  --alt <name> --seed <s> [--cond nolegend|legend|learn]
//! harness m1 score   --alt <name>
//! harness m2 list
//! harness m2 measure --alt <name> [--passes <n>]
//! harness m2 sheets  --alt <name> --lang tr|en --seed <s>
//! harness m2 score   --alt <name>
//! harness m2 inputs  --out <file.json>
//! ```
//!
//! `--root <dir>` (any command) replaces the experiments root `rounds/R3/experiments/`.

use std::path::PathBuf;
use std::process::ExitCode;

use harness::{io, m1, m2};

#[derive(Default)]
struct Args {
    alt: Option<String>,
    seed: Option<u64>,
    cond: Option<String>,
    lang: Option<String>,
    out: Option<String>,
    root: Option<String>,
    passes: Option<usize>,
}

fn parse(rest: &[String]) -> Result<Args, String> {
    let mut a = Args::default();
    let mut it = rest.iter();
    while let Some(k) = it.next() {
        let mut v = || it.next().cloned().ok_or(format!("{k} needs a value"));
        match k.as_str() {
            "--alt" => a.alt = Some(v()?),
            "--seed" => a.seed = Some(v()?.parse().map_err(|e| format!("--seed: {e}"))?),
            "--cond" => a.cond = Some(v()?),
            "--lang" => a.lang = Some(v()?),
            "--out" => a.out = Some(v()?),
            "--root" => a.root = Some(v()?),
            "--passes" => a.passes = Some(v()?.parse().map_err(|e| format!("--passes: {e}"))?),
            _ => return Err(format!("unknown argument `{k}`")),
        }
    }
    Ok(a)
}

fn need<T: Clone>(v: &Option<T>, name: &str) -> Result<T, String> {
    v.clone().ok_or(format!("missing {name}"))
}

fn run() -> Result<(), String> {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    if argv.len() < 2 {
        return Err("usage: harness m1|m2 <command> [options] (see src/main.rs)".into());
    }
    let a = parse(&argv[2..])?;
    let root = a.root.clone().map_or_else(io::default_root, PathBuf::from);
    match (argv[0].as_str(), argv[1].as_str()) {
        ("m1", "list") => {
            for e in m1::registry::all() {
                println!("{}", e.alphabet.name());
            }
        }
        ("m1", "measure") => {
            let e = m1::registry::by_name(&need(&a.alt, "--alt")?)?;
            io::check_name(e.alphabet.name())?;
            let r = m1::distinct::run(&e, &root)?;
            println!(
                "main (weakest K pair, worst condition): {:.2} ({} vs {} at {})",
                r.main.pair.d, r.main.pair.a, r.main.pair.b, r.main.condition
            );
            for (ch, w) in &r.channel_worst {
                println!("  {ch}: {:.2} at {}", w.pair.d, w.condition);
            }
            println!(
                "wrote {}",
                m1::raw_dir(&root, e.alphabet.name())
                    .join("distinct.json")
                    .display()
            );
        }
        ("m1", "sheets") => {
            let e = m1::registry::by_name(&need(&a.alt, "--alt")?)?;
            let seed = need(&a.seed, "--seed")?;
            let conds: Vec<String> = a.cond.clone().map_or_else(
                || {
                    m1::sheets::CONDITIONS
                        .iter()
                        .map(|s| (*s).to_owned())
                        .collect()
                },
                |c| vec![c],
            );
            for c in conds {
                let d = m1::sheets::write_condition(&e, &root, &c, seed)?;
                println!("{}", d.display());
            }
        }
        ("m1", "score") => {
            let alt = need(&a.alt, "--alt")?;
            io::check_name(&alt)?;
            let s = m1::score::run(&root, &alt)?;
            print!("{}", m1::score::markdown(&s));
        }
        ("m2", "list") => {
            for r in m2::registry::all() {
                println!("{}", r.name());
            }
        }
        ("m2", "measure") => {
            let r = m2::registry::by_name(&need(&a.alt, "--alt")?)?;
            let (auto, cost) = m2::metrics::run(r.as_ref(), &root, a.passes.unwrap_or(20))?;
            print!("{}", m2::metrics::markdown(&auto, Some(&cost)));
        }
        ("m2", "sheets") => {
            let r = m2::registry::by_name(&need(&a.alt, "--alt")?)?;
            let lang = m2::parse_lang(&need(&a.lang, "--lang")?)?;
            let p = m2::sheets::write_sheet(r.as_ref(), &root, lang, need(&a.seed, "--seed")?)?;
            println!("{}", p.display());
        }
        ("m2", "score") => {
            let alt = need(&a.alt, "--alt")?;
            io::check_name(&alt)?;
            let s = m2::score::run(&root, &alt)?;
            print!("{}", m2::score::markdown(&s));
        }
        ("m2", "inputs") => {
            let sits = m2::situations::all();
            let v: Vec<m2::situations::InputView<'_>> = sits.iter().map(Into::into).collect();
            io::write_json(&PathBuf::from(need(&a.out, "--out")?), &v)?;
        }
        (e, c) => return Err(format!("unknown command `{e} {c}`")),
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("harness: {e}");
            ExitCode::FAILURE
        }
    }
}
