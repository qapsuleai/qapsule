//! `m11` command line (R4/H-M11).
//!
//! ```text
//! m11 list                                   registered outcome styles
//! m11 measure --style <name>                 raw/<style>/distinct.{json,md}
//! m11 sheets  --style <name> --seed <s> [--alphabet siluet|rozet] [--persona F|H]
//!             [--cond nolegend|legend] [--salt <n>]
//!                                            raw/screens/<code>.png, raw/keys/<code>.json,
//!                                            raw/key.json, raw/prompts/…; prints the code
//! m11 check-prompt <file>                    forbidden-word check (exit 1 on a hit)
//! m11 score                                  raw/scores.{json,md} + selection rule
//! ```
//!
//! `--root <dir>` (any command) replaces the M11 root `rounds/R4/experiments/M11/`.

use std::path::PathBuf;
use std::process::ExitCode;

use m11::{blind, distinct, io, score, sheets};
use qp_grammar::glyph::outcome;

#[derive(Default)]
struct Args {
    style: Option<String>,
    seed: Option<u64>,
    alphabet: Option<String>,
    persona: Option<String>,
    cond: Option<String>,
    salt: Option<u64>,
    root: Option<String>,
    file: Option<String>,
}

fn parse(rest: &[String]) -> Result<Args, String> {
    let mut a = Args::default();
    let mut it = rest.iter();
    while let Some(k) = it.next() {
        let mut v = || it.next().cloned().ok_or(format!("{k} needs a value"));
        match k.as_str() {
            "--style" => a.style = Some(v()?),
            "--seed" => a.seed = Some(v()?.parse().map_err(|e| format!("--seed: {e}"))?),
            "--alphabet" => a.alphabet = Some(v()?),
            "--persona" => a.persona = Some(v()?),
            "--cond" => a.cond = Some(v()?),
            "--salt" => a.salt = Some(v()?.parse().map_err(|e| format!("--salt: {e}"))?),
            "--root" => a.root = Some(v()?),
            s if !s.starts_with("--") && a.file.is_none() => a.file = Some(s.to_owned()),
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
    let Some(cmd) = argv.first() else {
        return Err(
            "usage: m11 list|measure|sheets|check-prompt|score [options] (see src/main.rs)".into(),
        );
    };
    let a = parse(&argv[1..])?;
    let root = a.root.clone().map_or_else(io::default_root, PathBuf::from);
    match cmd.as_str() {
        "list" => {
            for n in outcome::names() {
                println!("{n}");
            }
        }
        "measure" => {
            let st = outcome::by_name(&need(&a.style, "--style")?).map_err(|e| e.reason)?;
            let r = distinct::run(st, &root)?;
            println!(
                "main (weakest realized/failed pair, worst condition): {:.2} ({} at {})",
                r.main.pair.d, r.main.pair.glyph, r.main.condition
            );
            for (al, ar) in &r.alphabets {
                println!(
                    "  {al}: {:.2} ({} at {}); out of box {}",
                    ar.main.pair.d, ar.main.pair.glyph, ar.main.condition, ar.out_of_box_prims
                );
            }
            println!(
                "wrote {}",
                root.join("raw")
                    .join(st.name())
                    .join("distinct.json")
                    .display()
            );
        }
        "sheets" => {
            let st = outcome::by_name(&need(&a.style, "--style")?).map_err(|e| e.reason)?;
            let alphabet = a.alphabet.clone().unwrap_or_else(|| "siluet".into());
            let personas: Vec<&str> = match a.persona.as_deref() {
                Some(p) => vec![p],
                None => sheets::PERSONAS.to_vec(),
            };
            let conds: Vec<&str> = match a.cond.as_deref() {
                Some(c) => vec![c],
                None => sheets::CONDITIONS.to_vec(),
            };
            let w = sheets::write_sheet(
                st,
                &alphabet,
                need(&a.seed, "--seed")?,
                a.salt.unwrap_or(blind::DEFAULT_SALT),
                &personas,
                &conds,
                &root,
            )?;
            println!("{}", w.code);
        }
        "check-prompt" => {
            let f = need(&a.file, "<file>")?;
            let hits = blind::forbidden_hits(&io::read(&PathBuf::from(&f))?, &[]);
            if !hits.is_empty() {
                return Err(format!("{f}: forbidden words: {}", hits.join(", ")));
            }
        }
        "score" => {
            let s = score::run(&root)?;
            print!("{}", score::markdown(&s));
        }
        c => return Err(format!("unknown command `{c}`")),
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("m11: {e}");
            ExitCode::FAILURE
        }
    }
}
