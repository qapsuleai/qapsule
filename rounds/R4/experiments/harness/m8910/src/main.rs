//! `m8910` command line (R4/H-M8910). See README.md.
//!
//! ```text
//! m8910 list                                  registered designs (faces, feeds, transitions)
//! m8910 s8 [--max-steps N]                    find S8 → harness/s8.json
//! m8910 m8 --face <name> [--no-shoot]         M8 pages, keys, prompts, computed measures
//! m8910 m8-critic --k <n>                     critic n's directory (all designs' pages)
//! m8910 m9-scenario                           find the M9 scenario → M9/raw/scenario.json
//! m8910 m9 --feed <name> [--no-shoot]         M9 screens, key, prompts
//! m8910 m10 --transition <name> [--no-shoot]  M10 plan, computed, screens, key
//! m8910 reader-setup --exp m8|m9|m10 --design <name> --persona F|H [--page 1|2]
//!                    --reader <k> [--seed <s>] [--model <m>]
//!                                             one reader's record directory (prints it)
//! m8910 check-prompt <file>                   forbidden-word check (exit 1 on a hit)
//! m8910 score --exp m8|m9|m10                 raw/scores.{json,md} + selection rule
//! ```
//!
//! `--root <dir>` replaces the experiments root `rounds/R4/experiments/`; `--s8 <file>`
//! the S8 file (`<root>/harness/s8.json`); `--salt <n>` the code salt.

use std::path::PathBuf;
use std::process::ExitCode;

use m8910::{blind, io, m10, m8, m9, readers, s8, score};

#[derive(Default)]
struct Args {
    kv: std::collections::BTreeMap<String, String>,
    flags: Vec<String>,
    file: Option<String>,
}

impl Args {
    fn get(&self, k: &str) -> Option<&str> {
        self.kv.get(k).map(String::as_str)
    }
    fn need(&self, k: &str) -> Result<&str, String> {
        self.get(k).ok_or(format!("missing --{k}"))
    }
    fn num(&self, k: &str, d: u64) -> Result<u64, String> {
        self.get(k)
            .map_or(Ok(d), |v| v.parse().map_err(|e| format!("--{k}: {e}")))
    }
    fn flag(&self, k: &str) -> bool {
        self.flags.iter().any(|f| f == k)
    }
}

fn parse(rest: &[String]) -> Args {
    let mut a = Args::default();
    let mut i = 0;
    while i < rest.len() {
        let k = &rest[i];
        if let Some(name) = k.strip_prefix("--") {
            if name.starts_with("no-") {
                a.flags.push(name.to_owned());
            } else if i + 1 < rest.len() {
                a.kv.insert(name.to_owned(), rest[i + 1].clone());
                i += 1;
            }
        } else if a.file.is_none() {
            a.file = Some(k.clone());
        }
        i += 1;
    }
    a
}

fn run() -> Result<(), String> {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let Some(cmd) = argv.first() else {
        return Err("usage: m8910 <command> (see src/main.rs or README.md)".into());
    };
    let a = parse(&argv[1..]);
    let exp = a.get("root").map_or_else(io::default_root, PathBuf::from);
    let s8p = a
        .get("s8")
        .map_or_else(|| exp.join("harness").join("s8.json"), PathBuf::from);
    let salt = a.num("salt", blind::DEFAULT_SALT)?;
    let shoot = !a.flag("no-shoot");
    match cmd.as_str() {
        "list" => {
            println!("faces: {}", qp_grammar::faces::names().join(", "));
            println!("feeds: {}", qp_grammar::feeds::names().join(", "));
            println!(
                "transitions: {}",
                qp_grammar::transitions::names().join(", ")
            );
        }
        "s8" => {
            let s = s8::find(a.num("max-steps", u64::MAX)?)?;
            io::write_json(&s8p, &s)?;
            for st in &s.states {
                println!(
                    "#{:>2} {} {}",
                    st.state,
                    if st.found {
                        format!("step {} {}", st.step, st.capsule)
                    } else {
                        "NOT FOUND".into()
                    },
                    st.note
                );
            }
        }
        "m8" => {
            let s = m8::read_s8(&s8p)?;
            let o = m8::sheets(a.need("face")?, &s, &exp, salt, shoot)?;
            for (p, c) in o.codes {
                println!("page {p}: {c}");
            }
        }
        "m8-critic" => {
            let d = m8::critic(&exp, a.num("k", 1)?)?;
            println!("{}", d.display());
        }
        "m9-scenario" => {
            let sc = m9::scenario()?;
            io::write_json(&m9::root(&exp).join("raw").join("scenario.json"), &sc)?;
            println!(
                "t0 step {}, t1 step {}, negative {}",
                sc.t0_step, sc.t1_step, sc.negative
            );
        }
        "m9" => {
            let sc: m9::Scenario = serde_json::from_str(&io::read(
                &m9::root(&exp).join("raw").join("scenario.json"),
            )?)
            .map_err(|e| e.to_string())?;
            println!("{}", m9::screens(a.need("feed")?, &sc, &exp, salt, shoot)?);
        }
        "m10" => {
            let s = m8::read_s8(&s8p)?;
            let p = m10::screens(a.need("transition")?, &s, &exp, salt, shoot)?;
            println!("{} ({} states)", p.code, p.states.len());
        }
        "reader-setup" => {
            let r = readers::Reader {
                experiment: a.need("exp")?.to_owned(),
                design: a.need("design")?.to_owned(),
                persona: a.need("persona")?.to_owned(),
                page: usize::try_from(a.num("page", 1)?).unwrap_or(1),
                k: a.num("reader", 1)?,
                seed: a.num("seed", 1)?,
                model: a.get("model").unwrap_or("sonnet").to_owned(),
            };
            println!("{}", readers::setup(&exp, &r)?.display());
        }
        "check-prompt" => {
            let f = a.file.clone().ok_or("missing <file>")?;
            blind::check(&f, &io::read(&PathBuf::from(&f))?)?;
        }
        "score" => print!("{}", score::run(&exp, a.need("exp")?)?),
        c => return Err(format!("unknown command `{c}`")),
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("m8910: {e}");
            ExitCode::FAILURE
        }
    }
}
