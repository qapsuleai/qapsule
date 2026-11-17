//! The dump (every surface of every situation in TR and EN) and its Markdown tables
//! (SPEC §1.3): the detector × stage × language table for `docs/LANGUAGE.md`, the
//! coverage matrix, and the scan summary.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use serde::{Deserialize, Serialize};

use crate::key::{key, Key};
use crate::lang::{Lang, Language, Surfaces};
use crate::scan::{scan, Finding, Item, SCANS};
use crate::situ::{cell, direction_name, stage_name, validity_name, Situation};

/// One dumped situation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Row {
    /// Situation id.
    pub id: String,
    /// Cell key.
    pub cell: String,
    /// Answer key.
    pub key: Key,
    /// Turkish surfaces.
    pub tr: Surfaces,
    /// English surfaces.
    pub en: Surfaces,
    /// Second rendering equal to the first (determinism).
    pub deterministic: bool,
}

/// The dump of one language.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dump {
    /// Language name.
    pub language: String,
    /// Rows.
    pub rows: Vec<Row>,
}

/// Dumps a language over the situations (each rendered twice for the determinism scan).
#[must_use]
pub fn dump(l: &dyn Language, sits: &[Situation]) -> Dump {
    let rows = sits
        .iter()
        .map(|s| {
            let tr = l.surfaces(s, Lang::Tr);
            let en = l.surfaces(s, Lang::En);
            let det = tr == l.surfaces(s, Lang::Tr) && en == l.surfaces(s, Lang::En);
            Row {
                id: s.id.clone(),
                cell: cell(s),
                key: key(s),
                tr,
                en,
                deterministic: det,
            }
        })
        .collect();
    Dump {
        language: l.name().to_owned(),
        rows,
    }
}

/// Every finding of a dump.
#[must_use]
pub fn findings(d: &Dump) -> Vec<Finding> {
    let mut out = Vec::new();
    for r in &d.rows {
        let mut by_lang = BTreeMap::new();
        by_lang.insert(Lang::Tr, &r.tr);
        by_lang.insert(Lang::En, &r.en);
        let it = Item {
            key: &r.key,
            by_lang,
            again: BTreeMap::new(),
        };
        out.extend(scan(&it));
        if !r.deterministic {
            out.push(Finding {
                scan: "determinism".into(),
                situation: r.id.clone(),
                lang: "tr+en".into(),
                surface: "screen".into(),
                detail: "iki üretim farklı".into(),
            });
        }
    }
    out
}

fn esc(s: &str) -> String {
    s.replace('|', "\\|").replace('\n', " ")
}

const STAGES: [&str; 8] = [
    "born",
    "strengthened",
    "confirmed",
    "weakened",
    "realized",
    "invalidated",
    "expired",
    "retracted",
];

/// The language table (SPEC §1.3): one row per `(detector, direction, stage, validity)`
/// in the daily world for an entitled reader (first situation of that cell), plus the
/// redacted reader's rows and the hourly world's rows; then the coverage matrix.
#[must_use]
pub fn language_table(d: &Dump, sits: &[Situation]) -> String {
    let by_id: BTreeMap<&str, &Situation> = sits.iter().map(|s| (s.id.as_str(), s)).collect();
    let mut t = String::new();
    let _ = writeln!(t, "# Dil tablosu — `{}`\n", d.language);
    let _ = writeln!(t, "Üretildi: `langh dump --lang {}` (deterministik; sentetik veri). Satır: ilk durum; tam döküm `out/{}/dump.json`.\n", d.language, d.language);
    let mut seen = BTreeSet::new();
    let mut dets: Vec<&str> = d
        .rows
        .iter()
        .map(|r| by_id[r.id.as_str()].detector.as_str())
        .collect();
    dets.sort_unstable();
    dets.dedup();
    for (title, world, red) in [
        ("Günlük, yetkili okur", "syn-d", false),
        ("Saatlik, yetkili okur", "syn-h", false),
        ("Yetkisiz okur (lisanslı akış, K33)", "", true),
    ] {
        let _ = writeln!(t, "## {title}\n");
        for det in &dets {
            let rows: Vec<&crate::table::Row> = d
                .rows
                .iter()
                .filter(|r| {
                    let s = by_id[r.id.as_str()];
                    s.detector == *det
                        && s.redacted == red
                        && (world.is_empty() || s.world == world)
                })
                .filter(|r| {
                    let s = by_id[r.id.as_str()];
                    seen.insert((
                        title,
                        s.detector.clone(),
                        direction_name(s.story.direction),
                        stage_name(s.story.stage),
                        validity_name(s.validity),
                    ))
                })
                .collect();
            if rows.is_empty() {
                continue;
            }
            let _ = writeln!(t, "### `{det}`\n");
            let _ = writeln!(t, "| durum | yüzey | TR | EN |\n|---|---|---|---|");
            let mut rows = rows;
            rows.sort_by_key(|r| {
                let s = by_id[r.id.as_str()];
                (
                    direction_name(s.story.direction),
                    STAGES
                        .iter()
                        .position(|x| *x == stage_name(s.story.stage))
                        .unwrap_or(9),
                    validity_name(s.validity),
                )
            });
            for r in rows {
                let s = by_id[r.id.as_str()];
                let lab = format!(
                    "{} · {} · {} · {} · karne {} (`{}`{})",
                    s.id,
                    direction_name(s.story.direction),
                    stage_name(s.story.stage),
                    validity_name(s.validity),
                    s.ledger_bucket(),
                    s.code,
                    if s.origin == "constructed" {
                        ", kurgu"
                    } else {
                        ""
                    }
                );
                let _ = writeln!(
                    t,
                    "| {lab} | başlık | {} | {} |",
                    esc(&r.tr.headline),
                    esc(&r.en.headline)
                );
                let _ = writeln!(
                    t,
                    "| | ayrıntı | {} | {} |",
                    esc(&r.tr.detail),
                    esc(&r.en.detail)
                );
                let _ = writeln!(
                    t,
                    "| | karne | {} | {} |",
                    esc(&r.tr.ledger),
                    esc(&r.en.ledger)
                );
                let gl = |g: &[crate::lang::GlossEntry]| {
                    if g.is_empty() {
                        "—".to_owned()
                    } else {
                        g.iter()
                            .map(|e| format!("**{}**: {}", esc(&e.term), esc(&e.text)))
                            .collect::<Vec<_>>()
                            .join("<br>")
                    }
                };
                let _ = writeln!(
                    t,
                    "| | sözlük | {} | {} |",
                    gl(&r.tr.glossary),
                    gl(&r.en.glossary)
                );
            }
            let _ = writeln!(t);
        }
    }
    let _ = writeln!(
        t,
        "## Kapsam: dedektör × aşama (durum sayısı, bütün dünyalar ve okurlar)\n"
    );
    let _ = writeln!(t, "| dedektör | {} |", STAGES.join(" | "));
    let _ = writeln!(t, "|---|{}", "---:|".repeat(STAGES.len()));
    for det in &dets {
        let cells: Vec<String> = STAGES
            .iter()
            .map(|st| {
                let n = d
                    .rows
                    .iter()
                    .filter(|r| by_id[r.id.as_str()].detector == *det && r.key.stage == *st)
                    .count();
                if n == 0 {
                    "**0**".to_owned()
                } else {
                    n.to_string()
                }
            })
            .collect();
        let _ = writeln!(t, "| `{det}` | {} |", cells.join(" | "));
    }
    let _ = writeln!(t, "\n`candidate` (aday) aşaması yüzeye cümleyle girmez (yalnız izleyene, açık barda); döküme alınmadı. `retracted` sentetik oynatmada oluşmaz; kurgu durumdur (SPEC §1.2).");
    t
}

/// Scan summary (SPEC §2): counts per scan × surface × language and up to three examples.
#[must_use]
pub fn scan_summary(d: &Dump, fs: &[Finding]) -> String {
    let mut t = String::new();
    let _ = writeln!(t, "# Tarama — `{}`\n", d.language);
    let _ = writeln!(
        t,
        "{} durum × 2 dil. Bulgu sayısı (hedef: hepsi 0).\n",
        d.rows.len()
    );
    let _ = writeln!(
        t,
        "| tarama | toplam | durum | başlık | ayrıntı | karne | sözlük | ekran | TR | EN | TR+EN |"
    );
    let _ = writeln!(t, "|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|");
    for sname in SCANS {
        let x: Vec<&Finding> = fs.iter().filter(|f| f.scan == *sname).collect();
        let c = |p: &dyn Fn(&Finding) -> bool| x.iter().filter(|f| p(f)).count();
        let sits: BTreeSet<&str> = x.iter().map(|f| f.situation.as_str()).collect();
        let _ = writeln!(
            t,
            "| {sname} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            x.len(),
            sits.len(),
            c(&|f| f.surface == "headline"),
            c(&|f| f.surface == "detail"),
            c(&|f| f.surface == "ledger"),
            c(&|f| f.surface == "glossary"),
            c(&|f| f.surface == "screen"),
            c(&|f| f.lang == "tr"),
            c(&|f| f.lang == "en"),
            c(&|f| f.lang == "tr+en"),
        );
    }
    let _ = writeln!(t, "\n## Örnekler (tarama başına en çok 3 farklı ayrıntı)\n");
    for sname in SCANS {
        let mut shown = BTreeSet::new();
        let ex: Vec<&Finding> = fs
            .iter()
            .filter(|f| f.scan == *sname && shown.insert(f.detail.clone()))
            .take(3)
            .collect();
        if ex.is_empty() {
            continue;
        }
        let _ = writeln!(
            t,
            "- **{sname}**: {}",
            ex.iter()
                .map(|f| format!(
                    "{} {} {}: «{}»",
                    f.situation,
                    f.lang,
                    f.surface,
                    esc(&f.detail)
                ))
                .collect::<Vec<_>>()
                .join("; ")
        );
    }
    t
}

/// Finding counts per scan.
#[must_use]
pub fn counts(fs: &[Finding]) -> BTreeMap<String, usize> {
    let mut m: BTreeMap<String, usize> = SCANS.iter().map(|s| ((*s).to_owned(), 0)).collect();
    for f in fs {
        *m.entry(f.scan.clone()).or_insert(0) += 1;
    }
    m
}
