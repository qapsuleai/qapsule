//! `report`: M25 / M26 result files → Markdown and CSV tables (aggregate numbers only),
//! with the SPEC §4 and §5 candidate checks evaluated mechanically.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use crate::boot::Est;
use crate::common::{f, write_file};
use crate::m25::{M25Result, Triple};
use crate::m26::{M26Result, ALL};

fn read_dir_json<T: serde::de::DeserializeOwned>(dir: &Path) -> Result<Vec<T>, String> {
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    files.sort();
    files
        .iter()
        .map(|p| {
            let t = std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))?;
            serde_json::from_str(&t).map_err(|e| format!("{}: {e}", p.display()))
        })
        .collect()
}

fn ci(e: &Est) -> String {
    match (e.pt, e.lo, e.hi) {
        (Some(p), Some(l), Some(h)) => format!("{p:.3} [{l:.3}, {h:.3}]"),
        (p, _, _) => f(p, 3),
    }
}

fn period_rank(p: &str) -> u8 {
    match p {
        "5m" => 0,
        "15m" => 1,
        "1h" => 2,
        _ => 3,
    }
}

fn csv_est(e: &Est) -> String {
    let g = |x: Option<f64>| x.map_or_else(String::new, |v| format!("{v:.6}"));
    format!("{},{},{}", g(e.pt), g(e.lo), g(e.hi))
}

/// Verdict of one criterion.
fn mark(x: Option<bool>) -> &'static str {
    match x {
        Some(true) => "geçti",
        Some(false) => "kaldı",
        None => "eksik",
    }
}

/// SPEC §4 checks for one rule (calibration files, directional row).
fn criteria(res: &[M25Result], rule: &str) -> Vec<(&'static str, Option<bool>)> {
    let get = |period: &str| {
        res.iter()
            .find(|r| r.rule == rule && r.period == period && r.split == "calib")
    };
    let m = |r: Option<&M25Result>, name: &str| -> Option<Triple> {
        r.and_then(|r| r.rows.first())
            .and_then(|row| row.metrics.get(name).copied())
    };
    let r5 = get("5m");
    let r1h = get("1h");
    let c1 = m(r5, "ecr_own").and_then(|t| {
        let (a, b, hi) = (t.alt.pt?, t.reference.pt?, t.diff.hi?);
        Some(a <= 0.5 * b && hi < 0.0)
    });
    let c2 = m(r5, "ecr_ref").and_then(|t| Some(t.diff.lo? <= 0.0));
    let brier_ok = |r: Option<&M25Result>| m(r, "brier").and_then(|t| Some(t.diff.lo? <= 0.0));
    let gap_ok = m(r5, "calib_gap").and_then(|t| Some(t.diff.pt? <= 0.03));
    let c3 = match (brier_ok(r5), brier_ok(r1h), gap_ok) {
        (Some(a), Some(b), Some(c)) => Some(a && b && c),
        (Some(false), _, _) | (_, Some(false), _) | (_, _, Some(false)) => Some(false),
        _ => None,
    };
    let c4 = m(r5, "p_exp").and_then(|t| Some(t.alt.pt? <= 0.40));
    let c5 = m(r5, "sens").and_then(|t| Some(t.diff.lo? <= 0.0));
    vec![
        ("1 ECR_own ≤ ½ başvuru, üst uç < 0", c1),
        ("2 ECR_ref büyük değil", c2),
        ("3 Brier (5dk, 1sa) ve kalibrasyon farkı", c3),
        ("4 süresi dolan ≤ 0,40", c4),
        ("5 SENS büyük değil", c5),
    ]
}

/// M25 tables.
#[must_use]
pub fn m25_tables(mut res: Vec<M25Result>) -> (String, String) {
    res.sort_by(|a, b| {
        (a.split.as_str(), period_rank(&a.period), a.rule.as_str()).cmp(&(
            b.split.as_str(),
            period_rank(&b.period),
            b.rule.as_str(),
        ))
    });
    let mut md = String::new();
    let mut csv = String::from("period,split,rule,row,metric,alt_pt,alt_lo,alt_hi,ref_pt,ref_lo,ref_hi,diff_pt,diff_lo,diff_hi\n");
    let _ = writeln!(md, "# M25 — çıkış kuralı ölçüleri\n");
    let _ = writeln!(md, "Düzenek: `rounds/RP4/experiments/harness` (SPEC.md §3–§4). Yalnız toplu sayılar. Aralıklar sembol kümeli bootstrap %95 (1.000 tekrar, tohum 20261121); fark = alternatif − `r3` (eşli).\n");
    let mut groups: BTreeMap<(String, u8, String), Vec<&M25Result>> = BTreeMap::new();
    for r in &res {
        groups
            .entry((r.split.clone(), period_rank(&r.period), r.period.clone()))
            .or_default()
            .push(r);
    }
    for ((split, _, period), rs) in &groups {
        let _ = writeln!(md, "## {period} — {split}\n");
        if let Some(r) = rs.first() {
            let _ = writeln!(
                md,
                "Pencere {} → {}; {} sembol, {} gün, {} bar.\n",
                r.window[0], r.window[1], r.symbols, r.days, r.bars_window
            );
        }
        for (ri, row) in ["directional", "neutral"].iter().enumerate() {
            let _ = writeln!(md, "### {}\n", if ri == 0 { "Yönlü" } else { "Yönsüz" });
            let _ = writeln!(md, "| Kural | n | açık kalan | p_real | p_inv | p_exp | ECR_own | ECR_ref | REV | ömür ortanca/p90 | Brier (n) | taban | kal. farkı | SENS | gerçek − yalancı |");
            let _ = writeln!(
                md,
                "|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|"
            );
            for r in rs {
                let Some(x) = r.rows.get(ri) else { continue };
                let g = |k: &str| x.metrics.get(k).map(|t| t.alt).unwrap_or_default();
                let _ = writeln!(
                    md,
                    "| `{}` | {} | {} | {} | {} | {} | {} | {} | {} | {}/{} | {} ({}) | {} | {} | {} | {} |",
                    r.rule,
                    x.alt.n,
                    x.alt.open_at_end,
                    ci(&g("p_real")),
                    f(g("p_inv").pt, 3),
                    f(g("p_exp").pt, 3),
                    ci(&g("ecr_own")),
                    ci(&g("ecr_ref")),
                    ci(&g("rev")),
                    f(x.alt.life_median, 0),
                    f(x.alt.life_p90, 0),
                    ci(&g("brier")),
                    x.alt.brier_n,
                    f(g("brier_base").pt, 3),
                    ci(&g("calib_gap")),
                    ci(&g("sens")),
                    ci(&g("real_minus_decoy")),
                );
            }
            let _ = writeln!(md, "\nFark (alternatif − `r3`):\n");
            let _ = writeln!(md, "| Kural | ECR_own | ECR_ref | REV | Brier | kal. farkı | SENS | p_exp | gerçek − yalancı |");
            let _ = writeln!(md, "|---|---|---|---|---|---|---|---|---|");
            for r in rs.iter().filter(|r| r.rule != r.reference) {
                let Some(x) = r.rows.get(ri) else { continue };
                let d = |k: &str| x.metrics.get(k).map(|t| t.diff).unwrap_or_default();
                let _ = writeln!(
                    md,
                    "| `{}` | {} | {} | {} | {} | {} | {} | {} | {} |",
                    r.rule,
                    ci(&d("ecr_own")),
                    ci(&d("ecr_ref")),
                    ci(&d("rev")),
                    ci(&d("brier")),
                    ci(&d("calib_gap")),
                    ci(&d("sens")),
                    ci(&d("p_exp")),
                    ci(&d("real_minus_decoy")),
                );
            }
            let _ = writeln!(md);
            for r in rs {
                let Some(x) = r.rows.get(ri) else { continue };
                for (k, t) in &x.metrics {
                    let _ = writeln!(
                        csv,
                        "{},{},{},{},{},{},{},{}",
                        r.period,
                        r.split,
                        r.rule,
                        row,
                        k,
                        csv_est(&t.alt),
                        csv_est(&t.reference),
                        csv_est(&t.diff)
                    );
                }
                for (k, a, b) in [
                    ("n", x.alt.n, x.reference.n),
                    ("open_at_end", x.alt.open_at_end, x.reference.open_at_end),
                    ("brier_n", x.alt.brier_n, x.reference.brier_n),
                    (
                        "sens_replay_mismatch",
                        x.alt.sens_replay_mismatch,
                        x.reference.sens_replay_mismatch,
                    ),
                ] {
                    let _ = writeln!(
                        csv,
                        "{},{},{},{},{},{a},,,{b},,,,,",
                        r.period, r.split, r.rule, row, k
                    );
                }
            }
        }
    }
    let mut rules: Vec<&str> = res
        .iter()
        .filter(|r| r.split == "calib" && r.rule != r.reference)
        .map(|r| r.rule.as_str())
        .collect();
    rules.sort_unstable();
    rules.dedup();
    let _ = writeln!(md, "## Puanlama (SPEC §4, kalibrasyon, yönlü satır)\n");
    if rules.is_empty() {
        let _ = writeln!(md, "Alternatif sonucu yok.\n");
    } else {
        let names: Vec<&str> = criteria(&res, rules[0]).iter().map(|c| c.0).collect();
        let _ = writeln!(md, "| Kural | sabit | {} | aday |", names.join(" | "));
        let _ = writeln!(md, "|---|---|{}---|", "---|".repeat(names.len()));
        for rule in rules {
            let cs = criteria(&res, rule);
            let cand = if cs.iter().all(|c| c.1 == Some(true)) {
                "evet"
            } else if cs.iter().any(|c| c.1 == Some(false)) {
                "hayır"
            } else {
                "eksik"
            };
            let k = res
                .iter()
                .find(|r| r.rule == rule)
                .map_or(0, |r| r.constants);
            let _ = writeln!(
                md,
                "| `{rule}` | {k} | {} | {cand} |",
                cs.iter().map(|c| mark(c.1)).collect::<Vec<_>>().join(" | ")
            );
        }
        let _ = writeln!(
            md,
            "\nSıralama (SPEC §4 a–d) ve \"ayırt edilemedi\" kararı tablolardan elle verilir.\n"
        );
    }
    (md, csv)
}

/// M26 tables.
#[must_use]
pub fn m26_tables(mut res: Vec<M26Result>) -> (String, String) {
    res.sort_by(|a, b| {
        (
            a.split.as_str(),
            period_rank(&a.period),
            a.detectors.as_str(),
            a.rule.as_str(),
        )
            .cmp(&(
                b.split.as_str(),
                period_rank(&b.period),
                b.detectors.as_str(),
                b.rule.as_str(),
            ))
    });
    let daily_hit = |set: &str, rule: &str, split: &str, det: &str| -> Option<f64> {
        res.iter()
            .find(|r| r.detectors == set && r.rule == rule && r.split == split && r.period == "1d")
            .and_then(|r| r.rows.iter().find(|x| x.detector == det))
            .and_then(|x| x.metrics.get("hit"))
            .and_then(|t| t.alt.pt)
    };
    let mut md = String::from("# M26 — dedektör eşikleri gün içinde\n\n");
    let _ = writeln!(md, "Düzenek: `rounds/RP4/experiments/harness` (SPEC.md §5). Yalnız toplu sayılar; aralıklar sembol kümeli bootstrap %95. Açılış = günün ilk iki barı, kapanış = günün son barı (kapanış seansı birleşik). `ft` = isabet(geri kalan) − isabet(açılış ∪ kapanış).\n");
    let mut csv = String::from("period,split,detectors,rule,detector,metric,alt_pt,alt_lo,alt_hi,ref_pt,ref_lo,ref_hi,diff_pt,diff_lo,diff_hi\n");
    for r in &res {
        let _ = writeln!(
            md,
            "## `{}` / kural `{}` — {} — {}\n\nPencere {} → {}; {} sembol, {} gün, {} sembol·gün.\n",
            r.detectors, r.rule, r.period, r.split, r.window[0], r.window[1], r.symbols, r.days, r.symbol_days
        );
        let _ = writeln!(md, "| Dedektör | doğum | yük (sembol·gün) | yük oranı | isabet | 0,55'e göre | anlamsız | açılış payı | açılış − kalan | kapanış payı | kapanış − kalan | ft | isabet − günlük |");
        let _ = writeln!(md, "|---|---|---|---|---|---|---|---|---|---|---|---|---|");
        for x in &r.rows {
            let g = |k: &str| x.metrics.get(k).map(|t| t.alt).unwrap_or_default();
            let dd = daily_hit(&r.detectors, &r.rule, &r.split, &x.detector)
                .zip(g("hit").pt)
                .filter(|_| r.period != "1d")
                .map(|(d, h)| h - d);
            let _ = writeln!(
                md,
                "| `{}` | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
                x.detector,
                x.alt.births,
                f(g("load").pt, 4),
                f(x.load_ratio, 2),
                ci(&g("hit")),
                x.vs_coin_flip,
                if x.meaningless { "evet" } else { "—" },
                f(g("share_open").pt, 3),
                ci(&g("d_open")),
                f(g("share_close").pt, 3),
                ci(&g("d_close")),
                ci(&g("ft")),
                f(dd, 3),
            );
            for (k, t) in &x.metrics {
                let _ = writeln!(
                    csv,
                    "{},{},{},{},{},{},{},{},{}",
                    r.period,
                    r.split,
                    r.detectors,
                    r.rule,
                    x.detector,
                    k,
                    csv_est(&t.alt),
                    csv_est(&t.reference),
                    csv_est(&t.diff)
                );
            }
        }
        if r.detectors != r.reference && r.split == "calib" && r.period != "1d" {
            let c1 = r
                .rows
                .iter()
                .filter(|x| x.detector != ALL)
                .map(|x| {
                    x.metrics
                        .get("hit")
                        .and_then(|t| t.diff.hi)
                        .map(|hi| hi >= 0.0)
                })
                .try_fold(true, |acc, v| v.map(|v| acc && v));
            let all = r.rows.iter().find(|x| x.detector == ALL);
            let c2 = all.and_then(|x| x.load_ratio).map(|q| q <= 1.5);
            let c3 = all
                .and_then(|x| x.metrics.get("ft"))
                .and_then(|t| t.diff.hi)
                .map(|hi| hi < 0.0);
            let cand = match (c1, c2, c3) {
                (Some(true), Some(true), Some(true)) => "evet",
                (Some(false), _, _) | (_, Some(false), _) | (_, _, Some(false)) => "hayır",
                _ => "eksik",
            };
            let _ = writeln!(
                md,
                "\nPuanlama (SPEC §5): (1) isabet düşmedi {} · (2) yük ≤ 1,5× {} · (3) yanlış tetikleme azaldı {} → aday: {cand}\n",
                mark(c1),
                mark(c2),
                mark(c3)
            );
        }
        let _ = writeln!(md);
    }
    (md, csv)
}

/// `report`: read `<results>/M25`, `<results>/M26`; write `<out>/M25.{md,csv}` and
/// `<out>/M26.{md,csv}` (only for the experiments that have results).
///
/// # Errors
/// I/O, malformed result files.
pub fn cmd(results: &Path, out: &Path) -> Result<String, String> {
    let m25: Vec<M25Result> = read_dir_json(&results.join("M25"))?;
    let m26: Vec<M26Result> = read_dir_json(&results.join("M26"))?;
    let mut msg = Vec::new();
    if !m25.is_empty() {
        let n = m25.len();
        let (md, csv) = m25_tables(m25);
        write_file(&out.join("M25.md"), &md).map_err(|e| e.to_string())?;
        write_file(&out.join("M25.csv"), &csv).map_err(|e| e.to_string())?;
        msg.push(format!("M25: {n} sonuç → {}", out.join("M25.md").display()));
    }
    if !m26.is_empty() {
        let n = m26.len();
        let (md, csv) = m26_tables(m26);
        write_file(&out.join("M26.md"), &md).map_err(|e| e.to_string())?;
        write_file(&out.join("M26.csv"), &csv).map_err(|e| e.to_string())?;
        msg.push(format!("M26: {n} sonuç → {}", out.join("M26.md").display()));
    }
    if msg.is_empty() {
        return Err(format!("{}: sonuç yok", results.display()));
    }
    Ok(msg.join("\n"))
}
