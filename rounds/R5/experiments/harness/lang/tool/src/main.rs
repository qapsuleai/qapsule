//! `langh` — R5/T2 M17 harness CLI (see `../README.md`).
//!
//! ```text
//! langh situations --out out/situations.json
//! langh dump   --lang <name> --out out/<name>       # dump.json, TABLE.md, SCAN.md, scan.json
//! langh plan   --run <dir> --langs a,b,... [--personas F,H] [--readers 3] [--reps 3] [--seed 1] [--model haiku]
//! langh check-prompt <prompt.txt> [--langs a,b]     # exit 1 when a forbidden word is found
//! langh score  --run <dir> [--seed 1]               # RESULTS.md, scores.json
//! ```

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use langh::lang::{by_name, registry, Language};
use langh::readers::{check_prompt, forbidden_words, overlap, plan, score, CHANCE};
use langh::situ::{generate, Situation};
use langh::table::{counts, dump, findings, language_table, scan_summary};

fn arg(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn write(p: &Path, s: &str) {
    if let Some(d) = p.parent() {
        std::fs::create_dir_all(d).expect("mkdir");
    }
    std::fs::write(p, s).expect("write");
}

fn langs_of(spec: &str) -> Vec<Box<dyn Language>> {
    spec.split(',')
        .filter(|s| !s.is_empty())
        .map(|n| by_name(n).unwrap_or_else(|| panic!("unknown language `{n}`")))
        .collect()
}

#[allow(clippy::too_many_lines)]
fn results_md(run: &Path, sits: &[Situation], seed: u64) -> String {
    let sc = score(run, sits, seed);
    let key: BTreeMap<String, String> = std::fs::read_to_string(run.join("key.json"))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let name = |c: &str| key.get(c).cloned().unwrap_or_else(|| c.to_owned());
    let mut t = String::new();
    let _ = writeln!(
        t,
        "# Okuyucu ve eleştirmen sonuçları — `{}`\n",
        run.display()
    );
    let _ = writeln!(t, "Ajan insan değildir; bu ölçüm ön elemedir (SPEC §0). Okuyucu koşusu {}, eleştirmen koşusu {}, çözümlenemeyen {}; maliyet {:.4} $. Önyükleme tohumu {}.\n", sc.reader_runs, sc.critic_runs, sc.unparsed.len(), sc.cost_usd, sc.bootstrap_seed);
    let _ = writeln!(t, "## Anlama doğruluğu (birincil; F 2, H 1; Wilson %95)\n");
    let _ = writeln!(t, "| dil | kod | birincil | alt | üst | tavsiye okuması |");
    let _ = writeln!(t, "|---|---|---:|---:|---:|---:|");
    let mut prim: Vec<(&String, &(f64, f64, f64))> = sc.primary.iter().collect();
    prim.sort_by(|a, b| b.1 .0.total_cmp(&a.1 .0));
    for (c, (m, l, h)) in &prim {
        let a = sc
            .advice_reads
            .get(*c)
            .map_or(0.0, |x| if x.n > 0.0 { x.k / x.n } else { 0.0 });
        let _ = writeln!(
            t,
            "| `{}` | {c} | {m:.3} | {l:.3} | {h:.3} | {a:.3} |",
            name(c)
        );
    }
    let _ = writeln!(t, "\n### Soru × persona × koşul\n");
    let qs = ["S1", "S2", "S3", "S4", "S5", "S6"];
    let _ = writeln!(t, "| dil | persona | koşul | {} |", qs.join(" | "));
    let _ = writeln!(t, "|---|---|---|{}", "---:|".repeat(qs.len()));
    for (c, per) in &sc.readers {
        for (p, conds) in per {
            for (cond, q) in conds {
                let cells: Vec<String> = qs
                    .iter()
                    .map(|x| {
                        q.get(*x).map_or("—".to_owned(), |t| {
                            format!(
                                "{:.2} ({}/{})",
                                if t.n > 0.0 { t.k / t.n } else { 0.0 },
                                t.k,
                                t.n
                            )
                        })
                    })
                    .collect();
                let _ = writeln!(
                    t,
                    "| `{}` | {p} | {cond} | {} |",
                    name(c),
                    cells.join(" | ")
                );
            }
        }
    }
    let _ = writeln!(
        t,
        "\nŞans düzeyi: {}.\n",
        CHANCE
            .iter()
            .map(|(q, c)| format!("{q} {c:.2}"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let _ = writeln!(
        t,
        "## Eleştirmen ikili karşılaştırma (Bradley–Terry, önyükleme %95)\n"
    );
    let _ = writeln!(
        t,
        "| dil | kod | log güç | alt | üst |\n|---|---|---:|---:|---:|"
    );
    let mut bt: Vec<(&String, &(f64, f64, f64))> = sc.bt.iter().collect();
    bt.sort_by(|a, b| b.1 .0.total_cmp(&a.1 .0));
    for (c, (s, l, h)) in &bt {
        let _ = writeln!(t, "| `{}` | {c} | {s:.3} | {l:.3} | {h:.3} |", name(c));
    }
    let _ = writeln!(t, "\nÇelişki oranı (iki sırada farklı kazanan; > %30 ise çift \"ayırt edilemedi\", SPEC §4.2):\n");
    for (pair, (k, n)) in &sc.conflicts {
        let (a, b) = pair.split_once('|').unwrap_or((pair, ""));
        let r = if *n > 0 { *k as f64 / *n as f64 } else { 0.0 };
        let _ = writeln!(
            t,
            "- `{}` – `{}`: {k}/{n} = {r:.2}{}",
            name(a),
            name(b),
            if r > 0.30 {
                " → ayırt edilemedi"
            } else {
                ""
            }
        );
    }
    let _ = writeln!(t, "\nUzunluk yanlılığı (U2): kararlı birimlerde uzun metin {}/{} kez kazandı; korelasyon {:.3}.\n", sc.longer_won.0, sc.longer_won.1, sc.length_corr);
    // Distinguishability of the top two (SPEC §5).
    let _ = writeln!(t, "## Seçim kuralı girdileri (SPEC §5)\n");
    if prim.len() >= 2 {
        let (a, b) = (prim[0], prim[1]);
        let same = overlap((a.1 .1, a.1 .2), (b.1 .1, b.1 .2));
        let _ = writeln!(
            t,
            "- Birincil: `{}` {:.3} [{:.3}, {:.3}] ↔ `{}` {:.3} [{:.3}, {:.3}] → {}",
            name(a.0),
            a.1 .0,
            a.1 .1,
            a.1 .2,
            name(b.0),
            b.1 .0,
            b.1 .1,
            b.1 .2,
            if same {
                "**ayırt edilemedi**"
            } else {
                "ayrıldı"
            }
        );
    }
    if bt.len() >= 2 {
        let (a, b) = (bt[0], bt[1]);
        let same = overlap((a.1 .1, a.1 .2), (b.1 .1, b.1 .2));
        let _ = writeln!(
            t,
            "- BT: `{}` ↔ `{}` → {}",
            name(a.0),
            name(b.0),
            if same {
                "**ayırt edilemedi**"
            } else {
                "ayrıldı"
            }
        );
    }
    if let Some(bz) = key
        .iter()
        .find(|(_, n)| n.as_str() == "bozuk")
        .map(|(c, _)| c.clone())
    {
        let last_prim = prim.last().is_some_and(|x| *x.0 == bz);
        let last_bt = bt.is_empty() || bt.last().is_some_and(|x| *x.0 == bz);
        let sep_prim = prim.len() >= 2 && {
            let others: Vec<_> = prim.iter().filter(|x| *x.0 != bz).collect();
            let me = sc.primary[&bz];
            others
                .iter()
                .all(|o| !overlap((me.1, me.2), (o.1 .1, o.1 .2)))
        };
        let _ = writeln!(t, "- **Bozuk dil sınaması:** birincilde son: {}; aralığı ötekilerden ayrık: {}; BT'de son: {} → düzenek {}.", yn(last_prim), yn(sep_prim), yn(last_bt), if last_prim && last_bt { "**geçerli**" } else { "**GEÇERSİZ**" });
    }
    if !sc.unparsed.is_empty() {
        let _ = writeln!(t, "\nÇözümlenemeyen cevaplar: {:?}", sc.unparsed);
    }
    write(
        &run.join("scores.json"),
        &serde_json::to_string_pretty(&sc).unwrap_or_default(),
    );
    t
}

fn yn(b: bool) -> &'static str {
    if b {
        "evet"
    } else {
        "hayır"
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map_or("", String::as_str);
    match cmd {
        "situations" => {
            let out =
                PathBuf::from(arg(&args, "--out").unwrap_or_else(|| "out/situations.json".into()));
            let s = generate();
            write(&out, &serde_json::to_string(&s).expect("json"));
            println!("{} situations → {}", s.len(), out.display());
        }
        "dump" => {
            let name = arg(&args, "--lang").unwrap_or_else(|| "r4".into());
            let out = PathBuf::from(arg(&args, "--out").unwrap_or_else(|| format!("out/{name}")));
            let l = by_name(&name).unwrap_or_else(|| panic!("unknown language `{name}`"));
            let sits = generate();
            let d = dump(l.as_ref(), &sits);
            let fs = findings(&d);
            write(
                &out.join("dump.json"),
                &serde_json::to_string_pretty(&d).expect("json"),
            );
            write(&out.join("TABLE.md"), &language_table(&d, &sits));
            write(&out.join("SCAN.md"), &scan_summary(&d, &fs));
            write(
                &out.join("findings.json"),
                &serde_json::to_string_pretty(&fs).expect("json"),
            );
            write(
                &out.join("scan.json"),
                &serde_json::to_string_pretty(&counts(&fs)).expect("json"),
            );
            println!(
                "{name}: {} situations, {} findings → {}",
                d.rows.len(),
                fs.len(),
                out.display()
            );
            for (k, v) in counts(&fs) {
                println!("  {k:15} {v}");
            }
        }
        "plan" => {
            let run = PathBuf::from(arg(&args, "--run").expect("--run"));
            let langs = langs_of(&arg(&args, "--langs").unwrap_or_else(|| "r4,bozuk".into()));
            let personas_s = arg(&args, "--personas").unwrap_or_else(|| "F,H".into());
            let personas: Vec<&str> = personas_s.split(',').collect();
            let readers: usize = arg(&args, "--readers")
                .and_then(|x| x.parse().ok())
                .unwrap_or(3);
            let reps: usize = arg(&args, "--reps")
                .and_then(|x| x.parse().ok())
                .unwrap_or(3);
            let seed: u64 = arg(&args, "--seed")
                .and_then(|x| x.parse().ok())
                .unwrap_or(1);
            let model = arg(&args, "--model").unwrap_or_else(|| "haiku".into());
            let sits = generate();
            let n =
                plan(&run, &sits, &langs, &personas, readers, reps, seed, &model).expect("plan");
            println!("{n} runs planned in {}", run.display());
        }
        "check-prompt" => {
            let p = args.get(2).expect("prompt path");
            let langs = arg(&args, "--langs").map_or_else(registry, |s| langs_of(&s));
            let text = std::fs::read_to_string(p).expect("read prompt");
            let bad = check_prompt(&text, &forbidden_words(&langs));
            if !bad.is_empty() {
                eprintln!("forbidden words in {p}: {bad:?}");
                return ExitCode::from(1);
            }
        }
        "score" => {
            let run = PathBuf::from(arg(&args, "--run").expect("--run"));
            let seed: u64 = arg(&args, "--seed")
                .and_then(|x| x.parse().ok())
                .unwrap_or(1);
            let sits = generate();
            let md = results_md(&run, &sits, seed);
            write(&run.join("RESULTS.md"), &md);
            println!("{md}");
        }
        _ => {
            eprintln!("usage: langh situations|dump|plan|check-prompt|score (see src/main.rs)");
            return ExitCode::from(2);
        }
    }
    ExitCode::SUCCESS
}
