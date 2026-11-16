//! Reader scoring and the M11 selection rule (`m11 score`; SPEC §5, "Seçim kuralı (M11)").
//!
//! Reads only raw files: every `raw/readers/<run>/meta.json` + `answer.json`, the hidden
//! key `raw/keys/<code>.json` named by the meta (the style comes from the key, never from
//! the reader record), and each style's `raw/<style>/distinct.json`. Writes
//! `raw/scores.json` and `raw/scores.md`; re-scoring the same raw files gives the same
//! bytes.
//!
//! - Per reader: 16 items; an item is right when the answer equals the truth
//!   (`gerçekleşti` = realized, `tutmadı` = failed, `açık` = open). `bilmiyorum`, a
//!   missing, unknown or unparsable answer is wrong. Chance 1/3 (three real options);
//!   majority baseline 6/16 (the sheet mix).
//! - Per style × persona × condition: accuracy over all items of its readers.
//! - Weighted accuracy per style × condition: `(2·F + 1·H) / 3` (only personas with
//!   readers count; then the weights of the present personas).
//! - **Selection rule (SPEC §5, written before any result):**
//!   1. a style whose worst-condition ΔE (main metric of `distinct.json`) is below the
//!      reference `r3`'s is eliminated — checked **per alphabet** (`siluet` and `rozet`;
//!      SPEC: "hem siluet hem rozet ile çalışmalıdır");
//!   2. primary: weighted `nolegend` accuracy;
//!   3. if the best two remaining differ by < 0.05 in the primary: the higher
//!      worst-condition ΔE (over both alphabets) wins.
//!
//!   The reference itself is ranked for comparison and is never "selected"; a style
//!   without `distinct.json` or without `nolegend` readers is not eligible.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::distinct::Report;
use crate::io::{
    answer_text, file_name, first_json_object, read, round, subdirs, write, write_json,
};
use crate::sheets::{Key, Outcome, MIX};

/// Persona weights (SPEC §5: `F` 2, `H` 1).
pub const WEIGHTS: [(&str, f64); 2] = [("F", 2.0), ("H", 1.0)];
/// Primary-metric tie band.
pub const TIE: f64 = 0.05;
/// Chance level (three real options).
pub const CHANCE: f64 = 1.0 / 3.0;

/// `meta.json` written by `run_readers.sh` (no style name: it is resolved via the key).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Meta {
    /// Experiment (`m11`).
    pub experiment: String,
    /// Sheet code.
    pub code: String,
    /// `F` / `H`.
    pub persona: String,
    /// `nolegend` / `legend`.
    pub condition: String,
    /// Reader id (`r1`, …).
    pub reader: String,
    /// Model alias.
    pub model: String,
}

/// A reader's answer to one item, normalised.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Answer {
    /// gerçekleşti.
    Realized,
    /// tutmadı.
    Failed,
    /// açık.
    Open,
    /// bilmiyorum.
    Unknown,
    /// Missing or not one of the options.
    Invalid,
}

fn fold_tr(s: &str) -> String {
    s.trim()
        .chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'ç' => 'c',
            'ğ' => 'g',
            'ı' => 'i',
            'â' => 'a',
            'î' => 'i',
            'û' => 'u',
            'ö' => 'o',
            'ş' => 's',
            'ü' => 'u',
            other => other,
        })
        .filter(|c| *c != '\u{307}')
        .collect::<String>()
        .trim_matches(|c: char| !c.is_alphanumeric())
        .to_owned()
}

/// Normalises one answer value (a string, or an object with `cevap`/`answer`/`outcome`).
#[must_use]
pub fn normalise(v: Option<&serde_json::Value>) -> Answer {
    let s = match v {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(serde_json::Value::Object(o)) => ["cevap", "answer", "outcome"]
            .iter()
            .find_map(|k| o.get(*k).and_then(serde_json::Value::as_str))
            .unwrap_or_default()
            .to_owned(),
        _ => String::new(),
    };
    match fold_tr(&s).as_str() {
        "gerceklesti" | "realized" => Answer::Realized,
        "tutmadi" | "failed" => Answer::Failed,
        "acik" | "hala acik" | "open" => Answer::Open,
        "bilmiyorum" | "unknown" => Answer::Unknown,
        _ => Answer::Invalid,
    }
}

fn right(a: Answer, t: Outcome) -> bool {
    matches!(
        (a, t),
        (Answer::Realized, Outcome::Realized)
            | (Answer::Failed, Outcome::Failed)
            | (Answer::Open, Outcome::Open)
    )
}

/// One reader's result.
#[derive(Debug, Clone, Serialize)]
pub struct ReaderScore {
    /// Raw run directory name.
    pub run: String,
    /// Reader id.
    pub reader: String,
    /// Sheet code.
    pub code: String,
    /// Alphabet of the sheet.
    pub alphabet: String,
    /// Whether a JSON object could be parsed from the answer.
    pub parsed: bool,
    /// Items asked.
    pub items: usize,
    /// Items right.
    pub correct: usize,
    /// Truth → answer → count.
    pub confusion: BTreeMap<String, BTreeMap<String, usize>>,
}

/// Aggregate of one style × persona × condition.
#[derive(Debug, Clone, Serialize)]
pub struct Cell {
    /// Readers.
    pub readers: usize,
    /// Items over all readers.
    pub items: usize,
    /// Accuracy.
    pub accuracy: f64,
    /// Truth → answer → count over readers.
    pub confusion: BTreeMap<String, BTreeMap<String, usize>>,
    /// Per reader.
    pub per_reader: Vec<ReaderScore>,
}

/// One style's summary.
#[derive(Debug, Clone, Serialize)]
pub struct StyleScore {
    /// `persona/condition` → cell.
    pub cells: BTreeMap<String, Cell>,
    /// Condition → weighted accuracy (`F` 2, `H` 1).
    pub weighted: BTreeMap<String, f64>,
    /// Alphabet → worst-condition ΔE (from `distinct.json`), if measured.
    pub delta_e: BTreeMap<String, f64>,
    /// Worst-condition ΔE over both alphabets, if measured.
    pub delta_e_main: Option<f64>,
    /// Eliminated by rule 1 (or not eligible), with the reason.
    pub excluded: Option<String>,
}

/// `raw/scores.json`.
#[derive(Debug, Clone, Serialize)]
pub struct Scores {
    /// Harness version.
    pub harness: String,
    /// Chance level per item.
    pub chance: f64,
    /// Majority baseline per item.
    pub majority: f64,
    /// Per style (style names come from the hidden keys).
    pub styles: BTreeMap<String, StyleScore>,
    /// Eligible non-reference styles, best first.
    pub ranking: Vec<String>,
    /// The selected style (`None` if none eligible).
    pub selected: Option<String>,
    /// How the rule decided.
    pub rule: Vec<String>,
}

fn add_conf(into: &mut BTreeMap<String, BTreeMap<String, usize>>, t: Outcome, a: Answer) {
    let an = serde_json::to_value(a)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default();
    *into
        .entry(t.name().to_owned())
        .or_default()
        .entry(an)
        .or_default() += 1;
}

/// Scores one answer object against a key.
#[must_use]
pub fn score_answer(
    run: &str,
    meta: &Meta,
    key: &Key,
    ans: Option<&serde_json::Value>,
) -> ReaderScore {
    let mut correct = 0;
    let mut confusion = BTreeMap::new();
    for it in &key.items {
        let a = normalise(
            ans.and_then(|v| v.get(&it.label).or_else(|| v.get(it.label.to_lowercase()))),
        );
        if right(a, it.outcome) {
            correct += 1;
        }
        add_conf(&mut confusion, it.outcome, a);
    }
    ReaderScore {
        run: run.to_owned(),
        reader: meta.reader.clone(),
        code: meta.code.clone(),
        alphabet: key.alphabet.clone(),
        parsed: ans.is_some(),
        items: key.items.len(),
        correct,
        confusion,
    }
}

#[allow(clippy::cast_precision_loss)]
fn ratio(a: usize, b: usize) -> f64 {
    round(a as f64 / b.max(1) as f64, 4)
}

/// Scores every reader run under `<root>/raw/` and applies the selection rule; writes
/// `raw/scores.json` and `raw/scores.md`.
///
/// # Errors
/// I/O failure, or a run whose key is missing.
#[allow(clippy::too_many_lines)]
pub fn run(root: &Path) -> Result<Scores, String> {
    let raw = root.join("raw");
    let mut by: BTreeMap<String, BTreeMap<String, Vec<ReaderScore>>> = BTreeMap::new();
    for d in subdirs(&raw.join("readers")) {
        let meta_path = d.join("meta.json");
        let ans_path = d.join("answer.json");
        if !meta_path.exists() || !ans_path.exists() {
            continue;
        }
        let meta: Meta = serde_json::from_str(&read(&meta_path)?)
            .map_err(|e| format!("{}: {e}", meta_path.display()))?;
        let key_path = raw.join("keys").join(format!("{}.json", meta.code));
        let key: Key = serde_json::from_str(&read(&key_path)?)
            .map_err(|e| format!("{}: {e}", key_path.display()))?;
        let ans = read(&ans_path)
            .ok()
            .and_then(|r| first_json_object(&answer_text(&r)));
        let s = score_answer(&file_name(&d), &meta, &key, ans.as_ref());
        by.entry(key.style.clone())
            .or_default()
            .entry(format!("{}/{}", meta.persona, meta.condition))
            .or_default()
            .push(s);
    }
    // Styles with a distinctness report but no readers still appear.
    for d in subdirs(&raw) {
        let n = file_name(&d);
        if d.join("distinct.json").exists() {
            by.entry(n).or_default();
        }
    }
    let mut styles = BTreeMap::new();
    for (style, cells_in) in by {
        let mut cells = BTreeMap::new();
        for (k, per_reader) in cells_in {
            let items: usize = per_reader.iter().map(|r| r.items).sum();
            let correct: usize = per_reader.iter().map(|r| r.correct).sum();
            let mut confusion: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
            for r in &per_reader {
                for (t, m) in &r.confusion {
                    for (a, n) in m {
                        *confusion
                            .entry(t.clone())
                            .or_default()
                            .entry(a.clone())
                            .or_default() += n;
                    }
                }
            }
            cells.insert(
                k,
                Cell {
                    readers: per_reader.len(),
                    items,
                    accuracy: ratio(correct, items),
                    confusion,
                    per_reader,
                },
            );
        }
        let mut weighted = BTreeMap::new();
        for cond in crate::sheets::CONDITIONS {
            let (mut num, mut den) = (0.0, 0.0);
            for (p, w) in WEIGHTS {
                if let Some(c) = cells.get(&format!("{p}/{cond}")) {
                    num += w * c.accuracy;
                    den += w;
                }
            }
            if den > 0.0 {
                weighted.insert(cond.to_owned(), round(num / den, 4));
            }
        }
        let rep: Option<Report> = read(&raw.join(&style).join("distinct.json"))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok());
        let delta_e = rep
            .as_ref()
            .map(|r| {
                r.alphabets
                    .iter()
                    .map(|(a, ar)| (a.clone(), ar.main.pair.d))
                    .collect()
            })
            .unwrap_or_default();
        styles.insert(
            style,
            StyleScore {
                cells,
                weighted,
                delta_e,
                delta_e_main: rep.map(|r| r.main.pair.d),
                excluded: None,
            },
        );
    }
    let mut rule = Vec::new();
    let reference = crate::REFERENCE;
    let ref_de = styles
        .get(reference)
        .map(|s| s.delta_e.clone())
        .unwrap_or_default();
    if ref_de.is_empty() {
        rule.push(format!(
            "reference `{reference}` has no distinct.json: rule 1 not applied"
        ));
    }
    for (name, s) in &mut styles {
        if name == reference {
            continue;
        }
        if s.delta_e_main.is_none() {
            s.excluded = Some("no distinct.json (run `m11 measure`)".into());
        } else if !s.weighted.contains_key("nolegend") {
            s.excluded = Some("no nolegend readers".into());
        } else {
            for (a, r) in &ref_de {
                match s.delta_e.get(a) {
                    Some(d) if d < r => {
                        s.excluded = Some(format!(
                            "rule 1: worst-condition ΔE on `{a}` {d:.4} < reference {r:.4}"
                        ));
                        break;
                    }
                    None => {
                        s.excluded = Some(format!("rule 1: `{a}` not measured"));
                        break;
                    }
                    _ => {}
                }
            }
        }
        if let Some(why) = &s.excluded {
            rule.push(format!("`{name}` excluded: {why}"));
        }
    }
    let mut ranking: Vec<String> = styles
        .iter()
        .filter(|(n, s)| n.as_str() != reference && s.excluded.is_none())
        .map(|(n, _)| n.clone())
        .collect();
    let prim = |n: &str| styles[n].weighted.get("nolegend").copied().unwrap_or(0.0);
    let de = |n: &str| styles[n].delta_e_main.unwrap_or(0.0);
    // Primary descending, then ΔE descending, then name (stable, for a total order only).
    ranking.sort_by(|a, b| {
        prim(b)
            .total_cmp(&prim(a))
            .then(de(b).total_cmp(&de(a)))
            .then(a.cmp(b))
    });
    let mut swap = false;
    let selected = match ranking.as_slice() {
        [] => {
            rule.push("no eligible style".into());
            None
        }
        [only] => {
            rule.push(format!("`{only}` is the only eligible style"));
            Some(only.clone())
        }
        [a, b, ..] => {
            let gap = round(prim(a) - prim(b), 4);
            if gap < TIE {
                let win = if de(b) > de(a) { b } else { a };
                rule.push(format!(
                    "rule 3: primary gap `{a}` {:.4} vs `{b}` {:.4} = {gap:.4} < {TIE}; worst-condition ΔE {:.4} vs {:.4} → `{win}`",
                    prim(a),
                    prim(b),
                    de(a),
                    de(b)
                ));
                swap = win == b;
                Some(win.clone())
            } else {
                rule.push(format!(
                    "rule 2: primary `{a}` {:.4} − `{b}` {:.4} = {gap:.4} ≥ {TIE} → `{a}`",
                    prim(a),
                    prim(b)
                ));
                Some(a.clone())
            }
        }
    };
    if swap {
        ranking.swap(0, 1);
    }
    if let (Some(sel), Some(r)) = (&selected, styles.get(reference)) {
        if let Some(rp) = r.weighted.get("nolegend") {
            rule.push(format!(
                "for comparison: reference `{reference}` primary {rp:.4}, selected `{sel}` {:.4}",
                prim(sel)
            ));
        }
    }
    let majority = ratio(
        MIX.iter().map(|(_, n)| *n).max().unwrap_or(0),
        MIX.iter().map(|(_, n)| *n).sum(),
    );
    let s = Scores {
        harness: crate::HARNESS_VERSION.to_owned(),
        chance: round(CHANCE, 4),
        majority,
        styles,
        ranking,
        selected,
        rule,
    };
    write_json(&raw.join("scores.json"), &s)?;
    write(&raw.join("scores.md"), markdown(&s).as_bytes())?;
    Ok(s)
}

/// Markdown summary.
#[must_use]
pub fn markdown(s: &Scores) -> String {
    let mut m = format!(
        "# M11 persona scores\n\nChance per item {:.2}; majority baseline {:.2}. Persona \
         agents are not people: this is a pre-screen (SPEC §0.5).\n\n\
         | style | F nolegend | H nolegend | weighted nolegend | F legend | H legend | weighted legend | ΔE siluet | ΔE rozet | excluded |\n\
         |---|---:|---:|---:|---:|---:|---:|---:|---:|---|\n",
        s.chance, s.majority
    );
    let cell = |st: &StyleScore, k: &str| {
        st.cells.get(k).map_or("–".to_owned(), |c| {
            format!("{:.2} ({})", c.accuracy, c.readers)
        })
    };
    let opt = |v: Option<&f64>| v.map_or("–".to_owned(), |x| format!("{x:.2}"));
    for (n, st) in &s.styles {
        m.push_str(&format!(
            "| {n} | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
            cell(st, "F/nolegend"),
            cell(st, "H/nolegend"),
            opt(st.weighted.get("nolegend")),
            cell(st, "F/legend"),
            cell(st, "H/legend"),
            opt(st.weighted.get("legend")),
            opt(st.delta_e.get("siluet")),
            opt(st.delta_e.get("rozet")),
            st.excluded.clone().unwrap_or_default()
        ));
    }
    m.push_str("\nCells: accuracy (readers).\n\n## Selection\n\n");
    for r in &s.rule {
        m.push_str(&format!("- {r}\n"));
    }
    m.push_str(&format!(
        "\nRanking: {}. Selected: **{}**.\n",
        if s.ranking.is_empty() {
            "–".to_owned()
        } else {
            s.ranking.join(" > ")
        },
        s.selected.clone().unwrap_or_else(|| "none".into())
    ));
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalises_turkish_answers() {
        let v = |s: &str| serde_json::Value::String(s.into());
        assert_eq!(normalise(Some(&v("Gerçekleşti"))), Answer::Realized);
        assert_eq!(normalise(Some(&v("gerceklesti."))), Answer::Realized);
        assert_eq!(normalise(Some(&v("TUTMADI"))), Answer::Failed);
        assert_eq!(normalise(Some(&v("açık"))), Answer::Open);
        assert_eq!(normalise(Some(&v("hâlâ açık"))), Answer::Open);
        assert_eq!(normalise(Some(&v("AÇIK"))), Answer::Open);
        assert_eq!(normalise(Some(&v("GERÇEKLEŞTİ"))), Answer::Realized);
        assert_eq!(normalise(Some(&v("hala açık"))), Answer::Open);
        assert_eq!(normalise(Some(&v("bilmiyorum"))), Answer::Unknown);
        assert_eq!(normalise(None), Answer::Invalid);
        let o = serde_json::json!({"cevap": "tutmadı"});
        assert_eq!(normalise(Some(&o)), Answer::Failed);
    }
}
