//! M2 comprehension scoring (`harness m2 score --alt <name>`).
//!
//! Reads `readers/<run>/meta.json` + `answer.json` and the hidden key
//! `keys/<lang>_s<seed>.json`; writes `scores.json` / `scores.md`. Pure function of the raw
//! files (re-scoring gives the same bytes).
//!
//! Fields: `kind`, `direction`, `what` (both), `when` (category: past/now/conditional; a
//! true "past, 0 bars ago" also accepts "now"), `when_exact` (category and, for past, the
//! same `bars_ago`), `still_valid`, `full` (what + when_exact + still_valid). Chance: kind
//! 1/6, direction 1/3, what 1/18, when 1/3, still_valid 1/3. Majority baseline from the keys.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::sheets::{Item, Key};
use crate::io::{
    answer_text, field, file_name, first_json_object, read, round, subdirs, write, write_json,
};

/// Scored fields, report order.
pub const FIELDS: [&str; 7] = [
    "kind",
    "direction",
    "what",
    "when",
    "when_exact",
    "still_valid",
    "full",
];

/// `meta.json` written by `run_readers.sh`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Meta {
    /// `m2`.
    pub experiment: String,
    /// Alternative.
    pub alt: String,
    /// `tr` / `en`.
    pub lang: String,
    /// Seed.
    pub seed: u64,
    /// Reader id.
    pub reader: String,
    /// Model alias.
    pub model: String,
}

/// One reader.
#[derive(Debug, Clone, Serialize)]
pub struct ReaderScore {
    /// Run directory.
    pub run: String,
    /// Reader id.
    pub reader: String,
    /// Seed.
    pub seed: u64,
    /// JSON parsed.
    pub parsed: bool,
    /// Items.
    pub items: usize,
    /// Unanswered items.
    pub missing: usize,
    /// Correct counts per field.
    pub correct: BTreeMap<String, usize>,
    /// K26 item: fields right.
    pub k26: BTreeMap<String, bool>,
    /// Per item (label): wrong fields.
    pub wrong: BTreeMap<String, Vec<String>>,
}

/// One language.
#[derive(Debug, Clone, Serialize)]
pub struct LangScore {
    /// Readers.
    pub readers: usize,
    /// Items.
    pub items: usize,
    /// Accuracy per field.
    pub accuracy: BTreeMap<String, f64>,
    /// Chance per field.
    pub chance: BTreeMap<String, f64>,
    /// Majority baseline per field.
    pub majority: BTreeMap<String, f64>,
    /// K26 fully right / readers.
    pub k26_full: String,
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
    /// Per language.
    pub langs: BTreeMap<String, LangScore>,
}

fn bars(v: &serde_json::Value) -> Option<u64> {
    match v.get("bars_ago") {
        Some(serde_json::Value::Number(n)) => n.as_u64().or_else(|| {
            n.as_f64()
                .filter(|f| *f >= 0.0 && f.fract() == 0.0)
                .map(|f| f as u64)
        }),
        Some(serde_json::Value::String(s)) => s.trim().parse().ok(),
        _ => None,
    }
}

/// Field results for one item.
#[must_use]
pub fn judge(it: &Item, a: Option<&serde_json::Value>) -> BTreeMap<String, bool> {
    let t = &it.truth;
    let g = |k: &str| a.map(|v| field(v, k)).unwrap_or_default();
    let kind = g("kind") == t.kind;
    let direction = g("direction") == t.direction;
    let when_ans = g("when");
    let zero_past = t.when == "past" && t.bars_ago == Some(0);
    let when = when_ans == t.when || (zero_past && when_ans == "now");
    let when_exact = if t.when == "past" {
        (when_ans == "past" && a.and_then(bars) == t.bars_ago) || (zero_past && when_ans == "now")
    } else {
        when
    };
    let still = g("still_valid") == t.still_valid;
    let what = kind && direction;
    [
        ("kind", kind),
        ("direction", direction),
        ("what", what),
        ("when", when),
        ("when_exact", when_exact),
        ("still_valid", still),
        ("full", what && when_exact && still),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), v))
    .collect()
}

fn majority(keys: &[Key]) -> BTreeMap<String, f64> {
    let items: Vec<&Item> = keys.iter().flat_map(|k| k.items.iter()).collect();
    let n = items.len().max(1);
    let share = |f: &dyn Fn(&Item) -> String| {
        let mut c: BTreeMap<String, usize> = BTreeMap::new();
        for i in &items {
            *c.entry(f(i)).or_default() += 1;
        }
        #[allow(clippy::cast_precision_loss)]
        round(c.values().copied().max().unwrap_or(0) as f64 / n as f64, 4)
    };
    let mut m = BTreeMap::new();
    m.insert("kind".to_owned(), share(&|i| i.truth.kind.clone()));
    m.insert(
        "direction".to_owned(),
        share(&|i| i.truth.direction.clone()),
    );
    m.insert(
        "what".to_owned(),
        share(&|i| format!("{}|{}", i.truth.kind, i.truth.direction)),
    );
    m.insert("when".to_owned(), share(&|i| i.truth.when.clone()));
    m.insert(
        "when_exact".to_owned(),
        share(&|i| format!("{}|{:?}", i.truth.when, i.truth.bars_ago)),
    );
    m.insert(
        "still_valid".to_owned(),
        share(&|i| i.truth.still_valid.clone()),
    );
    m.insert(
        "full".to_owned(),
        share(&|i| {
            format!(
                "{}|{}|{}|{:?}|{}",
                i.truth.kind,
                i.truth.direction,
                i.truth.when,
                i.truth.bars_ago,
                i.truth.still_valid
            )
        }),
    );
    m
}

/// Scores every reader run of `alt`; writes `scores.json` / `scores.md`.
///
/// # Errors
/// I/O failure or a run without its key.
pub fn run(root: &Path, alt: &str) -> Result<Scores, String> {
    let raw = super::raw_dir(root, alt);
    let mut by_lang: BTreeMap<String, (Vec<ReaderScore>, Vec<Key>)> = BTreeMap::new();
    for d in subdirs(&raw.join("readers")) {
        let mp = d.join("meta.json");
        if !mp.exists() {
            continue;
        }
        let meta: Meta =
            serde_json::from_str(&read(&mp)?).map_err(|e| format!("{}: {e}", mp.display()))?;
        let kp = raw
            .join("keys")
            .join(format!("{}_s{}.json", meta.lang, meta.seed));
        let key: Key =
            serde_json::from_str(&read(&kp)?).map_err(|e| format!("{}: {e}", kp.display()))?;
        let ans = read(&d.join("answer.json"))
            .ok()
            .and_then(|raw| first_json_object(&answer_text(&raw)));
        let mut correct: BTreeMap<String, usize> =
            FIELDS.iter().map(|f| ((*f).to_owned(), 0)).collect();
        let mut wrong = BTreeMap::new();
        let mut k26 = BTreeMap::new();
        let mut missing = 0;
        for it in &key.items {
            let a = ans.as_ref().and_then(|v| v.get(&it.label));
            if a.is_none() {
                missing += 1;
            }
            let j = judge(it, a);
            for (f, ok) in &j {
                if *ok {
                    *correct.entry(f.clone()).or_default() += 1;
                }
            }
            wrong.insert(
                it.label.clone(),
                j.iter()
                    .filter(|(f, ok)| !**ok && FIELDS[..6].contains(&f.as_str()))
                    .map(|(f, _)| f.clone())
                    .collect(),
            );
            if it.situation == "k26" {
                k26 = j;
            }
        }
        let e = by_lang.entry(meta.lang.clone()).or_default();
        e.0.push(ReaderScore {
            run: file_name(&d),
            reader: meta.reader.clone(),
            seed: meta.seed,
            parsed: ans.is_some(),
            items: key.items.len(),
            missing,
            correct,
            k26,
            wrong,
        });
        e.1.push(key);
    }
    let mut langs = BTreeMap::new();
    for (lang, (per_reader, keys)) in by_lang {
        let items: usize = per_reader.iter().map(|r| r.items).sum();
        let accuracy = FIELDS
            .iter()
            .map(|f| {
                let c: usize = per_reader.iter().map(|r| r.correct[*f]).sum();
                #[allow(clippy::cast_precision_loss)]
                ((*f).to_owned(), round(c as f64 / items.max(1) as f64, 4))
            })
            .collect();
        let chance = [
            ("kind", 1.0 / 6.0),
            ("direction", 1.0 / 3.0),
            ("what", 1.0 / 18.0),
            ("when", 1.0 / 3.0),
            ("still_valid", 1.0 / 3.0),
        ]
        .iter()
        .map(|(k, v)| ((*k).to_owned(), round(*v, 4)))
        .collect();
        let k26_full = format!(
            "{}/{}",
            per_reader
                .iter()
                .filter(|r| r.k26.get("full").copied().unwrap_or(false))
                .count(),
            per_reader.len()
        );
        langs.insert(
            lang,
            LangScore {
                readers: per_reader.len(),
                items,
                accuracy,
                chance,
                majority: majority(&keys),
                k26_full,
                per_reader,
            },
        );
    }
    let s = Scores {
        harness: crate::HARNESS_VERSION.to_owned(),
        alt: alt.to_owned(),
        langs,
    };
    write_json(&raw.join("scores.json"), &s)?;
    write(&raw.join("scores.md"), markdown(&s).as_bytes())?;
    Ok(s)
}

/// Markdown summary.
#[must_use]
pub fn markdown(s: &Scores) -> String {
    let mut m = format!(
        "# M2 comprehension scores — `{}`\n\n| lang | readers | items |",
        s.alt
    );
    for f in FIELDS {
        m.push_str(&format!(" {f} |"));
    }
    m.push_str(" K26 full |\n|---|---:|---:|");
    for _ in FIELDS {
        m.push_str("---:|");
    }
    m.push_str("---:|\n");
    for (l, ls) in &s.langs {
        m.push_str(&format!("| {l} | {} | {} |", ls.readers, ls.items));
        for f in FIELDS {
            m.push_str(&format!(
                " {:.2} (maj {:.2}) |",
                ls.accuracy[f], ls.majority[f]
            ));
        }
        m.push_str(&format!(" {} |\n", ls.k26_full));
    }
    m.push_str("\nChance: kind 0.17, direction 0.33, what 0.06, when 0.33, still_valid 0.33.\n");
    m
}
