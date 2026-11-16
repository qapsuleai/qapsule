//! M1 reader scoring (`harness m1 score --alt <name>`).
//!
//! Reads only raw files: every `readers/<run>/meta.json` + `answer.json` under
//! `M1/raw/<alt>/` and the matching hidden key `keys/<condition>_s<seed>.json`; writes
//! `scores.json` and `scores.md`. Re-scoring the same raw files gives the same bytes.
//!
//! Accuracy is per field (kind, direction, strength, phase) and "full" (all four right).
//! A missing or unparsable answer counts as wrong. Chance level = 1 / number of options
//! (kind 1/6, direction 1/3, strength 1/2, phase 1/5, full 1/180); the majority baseline
//! (always answering the most frequent true value of the key) is also given.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::sheets::{Key, Truth};
use crate::io::{
    answer_text, field, file_name, first_json_object, read, round, subdirs, write, write_json,
};

/// Answer fields.
pub const FIELDS: [&str; 4] = ["kind", "direction", "strength", "phase"];
/// Chance level per field and for "full".
pub const CHANCE: [(&str, f64); 5] = [
    ("kind", 1.0 / 6.0),
    ("direction", 1.0 / 3.0),
    ("strength", 1.0 / 2.0),
    ("phase", 1.0 / 5.0),
    ("full", 1.0 / 180.0),
];

/// `meta.json` written by `run_readers.sh` next to each raw answer.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Meta {
    /// Experiment (`m1`).
    pub experiment: String,
    /// Alternative.
    pub alt: String,
    /// Condition.
    pub condition: String,
    /// Sheet seed.
    pub seed: u64,
    /// Reader id (`r1`, …).
    pub reader: String,
    /// Model alias.
    pub model: String,
}

/// One reader's result.
#[derive(Debug, Clone, Serialize)]
pub struct ReaderScore {
    /// Raw run directory name.
    pub run: String,
    /// Reader id.
    pub reader: String,
    /// Seed.
    pub seed: u64,
    /// Whether a JSON object could be parsed from the answer.
    pub parsed: bool,
    /// Items asked.
    pub items: usize,
    /// Items with no answer.
    pub missing: usize,
    /// Correct counts per field and `full`.
    pub correct: BTreeMap<String, usize>,
    /// Per item: label → wrong fields (empty = all right).
    pub wrong: BTreeMap<String, Vec<String>>,
}

/// Aggregate of one condition.
#[derive(Debug, Clone, Serialize)]
pub struct ConditionScore {
    /// Readers scored.
    pub readers: usize,
    /// Items over all readers.
    pub items: usize,
    /// Accuracy per field and `full`.
    pub accuracy: BTreeMap<String, f64>,
    /// Chance level per field and `full`.
    pub chance: BTreeMap<String, f64>,
    /// Majority baseline per field and `full` (from the keys).
    pub majority: BTreeMap<String, f64>,
    /// Per reader.
    pub per_reader: Vec<ReaderScore>,
}

/// `scores.json`.
#[derive(Debug, Clone, Serialize)]
pub struct Scores {
    /// Harness version.
    pub harness: String,
    /// Alternative.
    pub alt: String,
    /// Per condition.
    pub conditions: BTreeMap<String, ConditionScore>,
}

fn norm(s: &str) -> String {
    s.trim().to_lowercase()
}

/// Scores one answer object against a key's items.
#[must_use]
pub fn score_answer(
    run: &str,
    meta: &Meta,
    items: &[Truth],
    ans: Option<&serde_json::Value>,
) -> ReaderScore {
    let mut correct: BTreeMap<String, usize> =
        FIELDS.iter().map(|f| ((*f).to_owned(), 0)).collect();
    correct.insert("full".to_owned(), 0);
    let mut wrong = BTreeMap::new();
    let mut missing = 0;
    for t in items {
        let a = ans.and_then(|v| v.get(&t.label).or_else(|| v.get(t.label.to_lowercase())));
        if a.is_none() {
            missing += 1;
        }
        let truth = [&t.kind, &t.direction, &t.strength, &t.phase];
        let mut w = Vec::new();
        for (f, tv) in FIELDS.iter().zip(truth) {
            let got = a.map(|v| field(v, f)).unwrap_or_default();
            if norm(&got) == *tv {
                *correct.entry((*f).to_owned()).or_default() += 1;
            } else {
                w.push((*f).to_owned());
            }
        }
        if w.is_empty() {
            *correct.entry("full".to_owned()).or_default() += 1;
        }
        wrong.insert(t.label.clone(), w);
    }
    ReaderScore {
        run: run.to_owned(),
        reader: meta.reader.clone(),
        seed: meta.seed,
        parsed: ans.is_some(),
        items: items.len(),
        missing,
        correct,
        wrong,
    }
}

fn majority(keys: &[Key]) -> BTreeMap<String, f64> {
    let mut out = BTreeMap::new();
    let all: Vec<&Truth> = keys.iter().flat_map(|k| k.items.iter()).collect();
    let n = all.len().max(1);
    let mut full_mode: BTreeMap<String, usize> = BTreeMap::new();
    for f in FIELDS {
        let mut c: BTreeMap<&str, usize> = BTreeMap::new();
        for t in &all {
            let v = match f {
                "kind" => &t.kind,
                "direction" => &t.direction,
                "strength" => &t.strength,
                _ => &t.phase,
            };
            *c.entry(v.as_str()).or_default() += 1;
        }
        let m = c.values().copied().max().unwrap_or(0);
        #[allow(clippy::cast_precision_loss)]
        out.insert(f.to_owned(), round(m as f64 / n as f64, 4));
    }
    for t in &all {
        *full_mode
            .entry(format!(
                "{}|{}|{}|{}",
                t.kind, t.direction, t.strength, t.phase
            ))
            .or_default() += 1;
    }
    let m = full_mode.values().copied().max().unwrap_or(0);
    #[allow(clippy::cast_precision_loss)]
    out.insert("full".to_owned(), round(m as f64 / n as f64, 4));
    out
}

/// Scores every reader run of `alt` and writes `scores.json` / `scores.md`.
///
/// # Errors
/// I/O failure or a run without its key.
pub fn run(root: &Path, alt: &str) -> Result<Scores, String> {
    let raw = super::raw_dir(root, alt);
    let mut by_cond: BTreeMap<String, (Vec<ReaderScore>, Vec<Key>)> = BTreeMap::new();
    for d in subdirs(&raw.join("readers")) {
        let meta_path = d.join("meta.json");
        if !meta_path.exists() {
            continue;
        }
        let meta: Meta = serde_json::from_str(&read(&meta_path)?)
            .map_err(|e| format!("{}: {e}", meta_path.display()))?;
        let key_path = raw
            .join("keys")
            .join(format!("{}_s{}.json", meta.condition, meta.seed));
        let key: Key = serde_json::from_str(&read(&key_path)?)
            .map_err(|e| format!("{}: {e}", key_path.display()))?;
        let ans = read(&d.join("answer.json"))
            .ok()
            .and_then(|raw| first_json_object(&answer_text(&raw)));
        let s = score_answer(&file_name(&d), &meta, &key.items, ans.as_ref());
        let e = by_cond.entry(meta.condition.clone()).or_default();
        e.0.push(s);
        e.1.push(key);
    }
    let mut conditions = BTreeMap::new();
    for (cond, (per_reader, keys)) in by_cond {
        let items: usize = per_reader.iter().map(|r| r.items).sum();
        let mut accuracy = BTreeMap::new();
        for f in FIELDS.iter().copied().chain(["full"]) {
            let c: usize = per_reader
                .iter()
                .map(|r| r.correct.get(f).copied().unwrap_or(0))
                .sum();
            #[allow(clippy::cast_precision_loss)]
            accuracy.insert(f.to_owned(), round(c as f64 / items.max(1) as f64, 4));
        }
        conditions.insert(
            cond,
            ConditionScore {
                readers: per_reader.len(),
                items,
                accuracy,
                chance: CHANCE
                    .iter()
                    .map(|(f, c)| ((*f).to_owned(), round(*c, 4)))
                    .collect(),
                majority: majority(&keys),
                per_reader,
            },
        );
    }
    let s = Scores {
        harness: crate::HARNESS_VERSION.to_owned(),
        alt: alt.to_owned(),
        conditions,
    };
    write_json(&raw.join("scores.json"), &s)?;
    write(&raw.join("scores.md"), markdown(&s).as_bytes())?;
    Ok(s)
}

/// Markdown summary.
#[must_use]
pub fn markdown(s: &Scores) -> String {
    let mut m = format!(
        "# M1 reading scores — `{}`\n\n| condition | readers | items | kind | direction | strength | phase | full |\n|---|---:|---:|---:|---:|---:|---:|---:|\n",
        s.alt
    );
    for (c, cs) in &s.conditions {
        m.push_str(&format!("| {c} | {} | {} |", cs.readers, cs.items));
        for f in FIELDS.iter().copied().chain(["full"]) {
            m.push_str(&format!(
                " {:.2} (chance {:.2}, majority {:.2}) |",
                cs.accuracy[f], cs.chance[f], cs.majority[f]
            ));
        }
        m.push('\n');
    }
    m
}
