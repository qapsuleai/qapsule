//! Scoring and the selection rules (SPEC §2–§4), deterministic from the raw records:
//! `readers/*/{meta.json, answer.json}`, the hidden keys `keys/<code>.json`, the critics'
//! `critics/c<k>/{answer.json, key.json}` and the computed measures. Free text is recorded,
//! never scored. The reference (`r3`) is ranked for comparison, never selected.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Serialize;
use serde_json::{json, Value};

use crate::io;
use crate::{m10, m8, m9, prompts, s8};

/// SPEC: below this difference of the primary measure the tiebreaks decide.
pub const TIE: f64 = 0.05;

/// Normalized answer text: Turkish-aware lower case, quotes and end punctuation
/// stripped, spaces collapsed.
#[must_use]
pub fn norm(s: &str) -> String {
    let low: String = s
        .chars()
        .flat_map(|c| match c {
            'İ' => vec!['i'],
            'I' => vec!['ı'],
            c => c.to_lowercase().collect(),
        })
        .collect();
    let t = low.trim().trim_matches(|c: char| {
        c == '"' || c == '\'' || c == '“' || c == '”' || c == '.' || c == ' '
    });
    t.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn eq(a: &Value, key: &str) -> bool {
    a.as_str().is_some_and(|s| norm(s) == norm(key))
        // "I" folds to "ı"; accept the plain-ASCII lower case of the key too.
        || a.as_str().is_some_and(|s| s.trim().to_lowercase() == key.to_lowercase())
}

/// A reader's parsed answer and meta.
struct Read {
    meta: Value,
    answer: Option<Value>,
    dir: String,
}

fn readers(root: &Path) -> Vec<Read> {
    io::subdirs(&root.join("raw").join("readers"))
        .into_iter()
        .filter_map(|d| {
            let meta: Value = serde_json::from_str(&io::read(&d.join("meta.json")).ok()?).ok()?;
            let raw = io::read(&d.join("answer.json"))
                .ok()
                .filter(|s| !s.trim().is_empty())?;
            Some(Read {
                meta,
                answer: io::first_json_object(&io::answer_text(&raw)),
                dir: io::file_name(&d),
            })
        })
        .collect()
}

fn persona_of(m: &Value) -> String {
    m["persona"].as_str().unwrap_or("").to_owned()
}

/// `(2·F + 1·H) / 3`; one persona alone if the other has no reader.
#[must_use]
pub fn weighted(f: Option<f64>, h: Option<f64>) -> Option<f64> {
    match (f, h) {
        (Some(f), Some(h)) => Some((2.0 * f + h) / 3.0),
        (Some(x), None) | (None, Some(x)) => Some(x),
        (None, None) => None,
    }
}

/// One design's line of a selection.
#[derive(Debug, Clone, Serialize)]
pub struct Entry {
    /// Design.
    pub design: String,
    /// Primary measure.
    pub primary: Option<f64>,
    /// Tiebreak values in order (lower or higher better as the rule says).
    pub ties: Vec<Option<f64>>,
    /// Eligible (hard thresholds etc.).
    pub eligible: bool,
}

/// The selection: ranked designs and the steps taken.
#[derive(Debug, Clone, Serialize)]
pub struct Selection {
    /// Ranked (eligible alternatives first by the rule, then the rest).
    pub ranking: Vec<String>,
    /// The selected design (`None`: no eligible alternative with readers).
    pub selected: Option<String>,
    /// The first two (M8: both go on to the human test).
    pub top2: Vec<String>,
    /// What the rule did, in words.
    pub steps: Vec<String>,
}

/// Applies a rule: rank eligible non-reference entries by `primary` (desc); if the top two
/// differ by less than [`TIE`], the tiebreaks decide in order (`higher[i]`: higher wins).
#[must_use]
pub fn select(entries: &[Entry], higher: &[bool], names: &[&str]) -> Selection {
    let mut steps = Vec::new();
    let alts: Vec<&Entry> = entries
        .iter()
        .filter(|e| e.design != crate::REFERENCE && e.primary.is_some())
        .collect();
    let mut elig: Vec<&Entry> = alts.iter().copied().filter(|e| e.eligible).collect();
    if elig.is_empty() && !alts.is_empty() {
        steps.push("no alternative passes the eligibility rule: rule 1 not applied".into());
        elig.clone_from(&alts);
    }
    for e in &alts {
        if !elig.iter().any(|x| x.design == e.design) {
            steps.push(format!("`{}` eliminated (eligibility)", e.design));
        }
    }
    elig.sort_by(|a, b| {
        b.primary
            .unwrap_or(0.0)
            .total_cmp(&a.primary.unwrap_or(0.0))
            .then(a.design.cmp(&b.design))
    });
    if elig.len() >= 2 {
        let (a, b) = (elig[0], elig[1]);
        let d = a.primary.unwrap_or(0.0) - b.primary.unwrap_or(0.0);
        if d.abs() < TIE {
            steps.push(format!(
                "primary: `{}` {:.4} vs `{}` {:.4} (difference {:.4} < {TIE}): tiebreaks",
                a.design,
                a.primary.unwrap_or(0.0),
                b.design,
                b.primary.unwrap_or(0.0),
                d
            ));
            let mut swapped = false;
            for (i, hi) in higher.iter().enumerate() {
                let (x, y) = (
                    a.ties.get(i).copied().flatten(),
                    b.ties.get(i).copied().flatten(),
                );
                let name = names.get(i).copied().unwrap_or("tiebreak");
                match (x, y) {
                    (Some(x), Some(y)) if (x - y).abs() > 1e-12 => {
                        let b_wins = if *hi { y > x } else { y < x };
                        steps.push(format!(
                            "{name}: `{}` {x:.4}, `{}` {y:.4}",
                            a.design, b.design
                        ));
                        swapped = b_wins;
                        break;
                    }
                    _ => steps.push(format!("{name}: equal or missing")),
                }
            }
            if swapped {
                elig.swap(0, 1);
            }
        } else {
            steps.push(format!(
                "primary decides: `{}` {:.4} vs `{}` {:.4}",
                a.design,
                a.primary.unwrap_or(0.0),
                b.design,
                b.primary.unwrap_or(0.0)
            ));
        }
    }
    let mut ranking: Vec<String> = elig.iter().map(|e| e.design.clone()).collect();
    for e in entries {
        if !ranking.contains(&e.design) {
            ranking.push(e.design.clone());
        }
    }
    Selection {
        selected: elig.first().map(|e| e.design.clone()),
        top2: elig.iter().take(2).map(|e| e.design.clone()).collect(),
        ranking,
        steps,
    }
}

fn mean(v: &[f64]) -> Option<f64> {
    #[allow(clippy::cast_precision_loss)]
    (!v.is_empty()).then(|| v.iter().sum::<f64>() / v.len() as f64)
}

fn r4(x: Option<f64>) -> Option<f64> {
    x.map(|v| io::round(v, 4))
}

const QS: [&str; 6] = ["S1", "S1b", "S2", "S3", "S4", "S5"];

fn m8_key(k: &s8::Key, q: &str) -> String {
    match q {
        "S1" => k.s1.clone(),
        "S1b" => k.s1b.clone(),
        "S2" => k.s2.clone(),
        "S3" => k.s3.clone(),
        "S4" => k.s4.clone(),
        _ => k.s5.clone(),
    }
}

/// M8 scores and selection.
///
/// # Errors
/// Unreadable keys.
#[allow(clippy::too_many_lines, clippy::cast_precision_loss)]
pub fn m8(exp: &Path) -> Result<Value, String> {
    let root = m8::root(exp);
    let raw = root.join("raw");
    // design → persona → (question → [correct…], full → [..])
    let mut acc: BTreeMap<String, BTreeMap<String, BTreeMap<String, Vec<f64>>>> = BTreeMap::new();
    let mut per_reader = Vec::new();
    for r in readers(&root) {
        let code = r.meta["code"].as_str().unwrap_or("");
        let pk: m8::PageKey =
            serde_json::from_str(&io::read(&raw.join("keys").join(format!("{code}.json")))?)
                .map_err(|e| e.to_string())?;
        let p = persona_of(&r.meta);
        let ans = r.answer.clone().unwrap_or(Value::Null);
        let mut full_n = 0;
        for (label, lk) in &pk.labels {
            let a = &ans[label.as_str()];
            let mut all = true;
            for q in QS {
                let ok = eq(&a[q], &m8_key(&lk.key, q));
                all &= ok;
                acc.entry(pk.face.clone())
                    .or_default()
                    .entry(p.clone())
                    .or_default()
                    .entry(q.to_owned())
                    .or_default()
                    .push(if ok { 1.0 } else { 0.0 });
            }
            if all {
                full_n += 1;
            }
            acc.entry(pk.face.clone())
                .or_default()
                .entry(p.clone())
                .or_default()
                .entry("full".to_owned())
                .or_default()
                .push(if all { 1.0 } else { 0.0 });
        }
        per_reader.push(
            json!({"reader": r.dir, "design": pk.face, "persona": p, "parsed": r.answer.is_some(),
                               "full_correct": full_n, "faces": pk.labels.len()}),
        );
    }
    // Critics.
    let mut ranks: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    let mut crit: BTreeMap<String, BTreeMap<String, Vec<f64>>> = BTreeMap::new();
    for d in io::subdirs(&raw.join("critics")) {
        let Ok(a) = io::read(&d.join("answer.json")) else {
            continue;
        };
        let Some(v) = io::first_json_object(&io::answer_text(&a)) else {
            continue;
        };
        let key: BTreeMap<String, String> =
            serde_json::from_str(&io::read(&d.join("key.json"))?).map_err(|e| e.to_string())?;
        if let Some(order) = v["sira"].as_array() {
            for (i, l) in order.iter().enumerate() {
                if let Some(des) = l.as_str().and_then(|l| key.get(l.trim())) {
                    ranks.entry(des.clone()).or_default().push((i + 1) as f64);
                }
            }
        }
        if let Some(ps) = v["puanlar"].as_object() {
            for (l, sc) in ps {
                if let Some(des) = key.get(l.trim()) {
                    for c in 1..=8 {
                        if let Some(x) = sc[c.to_string()].as_f64() {
                            crit.entry(des.clone())
                                .or_default()
                                .entry(c.to_string())
                                .or_default()
                                .push(x);
                        }
                    }
                }
            }
        }
    }
    let mut designs = BTreeMap::new();
    let mut entries = Vec::new();
    let names: Vec<String> = io::subdirs(&raw)
        .into_iter()
        .filter(|d| d.join("computed.json").exists())
        .map(|d| io::file_name(&d))
        .chain(acc.keys().cloned())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    for d in names {
        let comp: Option<m8::Computed> = io::read(&raw.join(&d).join("computed.json"))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok());
        let pf = |p: &str, q: &str| {
            acc.get(&d)
                .and_then(|m| m.get(p))
                .and_then(|m| m.get(q))
                .and_then(|v| mean(v))
        };
        let full_f = pf("F", "full");
        let full_h = pf("H", "full");
        let primary = weighted(full_f, full_h);
        let rank = ranks.get(&d).and_then(|v| mean(v));
        let head = comp.as_ref().map(|c| {
            (c.headline_chars.get("tr").map_or(0.0, |v| v[1])
                + c.headline_chars.get("en").map_or(0.0, |v| v[1]))
                / 2.0
        });
        let mut q = BTreeMap::new();
        for p in prompts::PERSONAS {
            let mut m = BTreeMap::new();
            for k in QS.iter().copied().chain(["full"]) {
                m.insert(k.to_owned(), r4(pf(p, k)));
            }
            q.insert(p.to_owned(), m);
        }
        designs.insert(d.clone(), json!({
            "persona": q, "weighted_full": r4(primary), "critic_mean_rank": r4(rank),
            "critic_criteria": crit.get(&d).map(|m| m.iter().map(|(k, v)| (k.clone(), r4(mean(v)))).collect::<BTreeMap<_, _>>()),
            "hard_thresholds_pass": comp.as_ref().map(|c| c.pass), "headline_mean_chars_tr_en": head.map(|h| io::round(h, 2)),
        }));
        entries.push(Entry {
            design: d.clone(),
            primary,
            ties: vec![rank, head],
            eligible: comp.as_ref().is_some_and(|c| c.pass),
        });
    }
    let sel = select(
        &entries,
        &[false, false],
        &[
            "critic mean rank (lower wins)",
            "mean headline TR+EN (shorter wins)",
        ],
    );
    let chance: BTreeMap<&str, f64> = [
        ("S1", 1.0 / s8::S1.len() as f64),
        ("S1b", 1.0 / s8::S1B.len() as f64),
        ("S2", 1.0 / s8::S2.len() as f64),
        ("S3", 1.0 / s8::S3.len() as f64),
        ("S4", 1.0 / s8::S4.len() as f64),
        ("S5", 1.0 / s8::S5.len() as f64),
    ]
    .into_iter()
    .map(|(k, v)| (k, io::round(v, 4)))
    .collect();
    let full_chance: f64 = chance.values().product();
    Ok(
        json!({"experiment": "m8", "harness": crate::HARNESS_VERSION, "chance": chance,
              "chance_full": io::round(full_chance, 6), "designs": designs, "readers": per_reader,
              "selection": sel,
              "limit": "Persona ajanı insan değildir; bu ölçüm ön elemedir, son söz insan testi v2'dedir."}),
    )
}

fn str_list(v: &Value) -> Vec<String> {
    v.as_array()
        .map(|a| a.iter().filter_map(Value::as_str).map(norm).collect())
        .unwrap_or_default()
}

fn screens_used(v: &Value) -> Option<f64> {
    v["ekran"].as_array().and_then(|a| {
        a.iter()
            .filter_map(Value::as_f64)
            .fold(None, |m, x| Some(m.map_or(x, |m: f64| m.max(x))))
    })
}

/// M9 scores and selection.
///
/// # Errors
/// Unreadable keys.
#[allow(clippy::too_many_lines, clippy::cast_precision_loss)]
pub fn m9(exp: &Path) -> Result<Value, String> {
    let root = m9::root(exp);
    let raw = root.join("raw");
    let mut acc: BTreeMap<String, BTreeMap<String, BTreeMap<String, Vec<f64>>>> = BTreeMap::new();
    let mut rules: BTreeMap<String, u32> = BTreeMap::new();
    let mut minsc: BTreeMap<String, Value> = BTreeMap::new();
    let mut per_reader = Vec::new();
    for r in readers(&root) {
        let code = r.meta["code"].as_str().unwrap_or("");
        let k: m9::Key =
            serde_json::from_str(&io::read(&raw.join("keys").join(format!("{code}.json")))?)
                .map_err(|e| e.to_string())?;
        if let Ok(p) = io::read(&raw.join("keys").join(format!("{code}.positions.json"))) {
            if let Ok(pv) = serde_json::from_str::<Value>(&p) {
                minsc.insert(k.design.clone(), json!(m9::min_scroll(&k, &pv)));
            }
        }
        rules.insert(k.design.clone(), k.rules);
        let p = persona_of(&r.meta);
        let a = r.answer.clone().unwrap_or(Value::Null);
        let ans: Vec<String> = str_list(&a["G1"]["cevap"]);
        let keyset: Vec<String> = k.g1.iter().map(|s| norm(s)).collect();
        let hit = ans.iter().filter(|x| keyset.contains(x)).count() as f64;
        let prec = if ans.is_empty() {
            if keyset.is_empty() {
                1.0
            } else {
                0.0
            }
        } else {
            hit / ans.len() as f64
        };
        let rec = if keyset.is_empty() {
            if ans.is_empty() {
                1.0
            } else {
                0.0
            }
        } else {
            hit / keyset.len() as f64
        };
        let f1 = if prec + rec > 0.0 {
            2.0 * prec * rec / (prec + rec)
        } else {
            0.0
        };
        let g2 = eq(&a["G2"]["hangisi"], &k.g2_which) && eq(&a["G2"]["ne"], &k.g2_what);
        let g3 = a["G3"]["cevap"]
            .as_str()
            .is_some_and(|l| k.g3.iter().any(|x| x.eq_ignore_ascii_case(l.trim())));
        let accv = (f1 + f64::from(u8::from(g2)) + f64::from(u8::from(g3))) / 3.0;
        let e = acc
            .entry(k.design.clone())
            .or_default()
            .entry(p.clone())
            .or_default();
        e.entry("g1_f1".into()).or_default().push(f1);
        e.entry("g1_precision".into()).or_default().push(prec);
        e.entry("g1_recall".into()).or_default().push(rec);
        e.entry("g2".into())
            .or_default()
            .push(f64::from(u8::from(g2)));
        e.entry("g3".into())
            .or_default()
            .push(f64::from(u8::from(g3)));
        e.entry("accuracy".into()).or_default().push(accv);
        for (task, ok) in [("G1", f1 >= 1.0), ("G2", g2), ("G3", g3)] {
            let t = &a[task];
            let taps = t["acmam_gerekirdi"].as_array().map_or(0, Vec::len) as f64;
            e.entry("taps".into()).or_default().push(taps);
            if ok {
                if let Some(s) = screens_used(t) {
                    e.entry("scroll".into())
                        .or_default()
                        .push((s - 1.0).max(0.0));
                }
            }
        }
        per_reader.push(
            json!({"reader": r.dir, "design": k.design, "persona": p, "parsed": r.answer.is_some(),
                               "g1_f1": io::round(f1, 4), "g2": g2, "g3": g3}),
        );
    }
    let mut designs = BTreeMap::new();
    let mut entries = Vec::new();
    for (d, per) in &acc {
        let pm = |p: &str, k: &str| per.get(p).and_then(|m| m.get(k)).and_then(|v| mean(v));
        let primary = weighted(pm("F", "accuracy"), pm("H", "accuracy"));
        let cost = weighted(
            pm("F", "scroll").map(|s| s + pm("F", "taps").unwrap_or(0.0)),
            pm("H", "scroll").map(|s| s + pm("H", "taps").unwrap_or(0.0)),
        );
        let mut q = BTreeMap::new();
        for p in prompts::PERSONAS {
            let m: BTreeMap<&str, Option<f64>> = [
                "g1_f1",
                "g1_precision",
                "g1_recall",
                "g2",
                "g3",
                "accuracy",
                "scroll",
                "taps",
            ]
            .into_iter()
            .map(|k| (k, r4(pm(p, k))))
            .collect();
            q.insert(p.to_owned(), m);
        }
        designs.insert(d.clone(), json!({"persona": q, "weighted_accuracy": r4(primary),
            "weighted_scroll_plus_taps": r4(cost), "rules": rules.get(d), "min_scroll_from_key": minsc.get(d)}));
        entries.push(Entry {
            design: d.clone(),
            primary,
            ties: vec![cost, rules.get(d).map(|r| f64::from(*r))],
            eligible: true,
        });
    }
    let sel = select(
        &entries,
        &[false, false],
        &["mean scroll + taps (fewer wins)", "rules (fewer wins)"],
    );
    Ok(
        json!({"experiment": "m9", "harness": crate::HARNESS_VERSION, "designs": designs,
              "readers": per_reader, "selection": sel,
              "limit": "Persona ajanı insan değildir; bu ölçüm ön elemedir, son söz insan testi v2'dedir."}),
    )
}

/// M10 scores and selection.
///
/// # Errors
/// Unreadable keys.
#[allow(clippy::cast_precision_loss)]
pub fn m10(exp: &Path) -> Result<Value, String> {
    let root = m10::root(exp);
    let raw = root.join("raw");
    let mut acc: BTreeMap<String, BTreeMap<String, BTreeMap<String, Vec<f64>>>> = BTreeMap::new();
    let mut per_reader = Vec::new();
    for r in readers(&root) {
        let code = r.meta["code"].as_str().unwrap_or("");
        let k: m10::Key =
            serde_json::from_str(&io::read(&raw.join("keys").join(format!("{code}.json")))?)
                .map_err(|e| e.to_string())?;
        let p = persona_of(&r.meta);
        let a = r.answer.clone().unwrap_or(Value::Null);
        let order: Vec<String> = r.meta["order"]
            .as_array()
            .map(|v| {
                v.iter()
                    .filter_map(|x| x.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default();
        let e = acc
            .entry(k.design.clone())
            .or_default()
            .entry(p.clone())
            .or_default();
        for (pos, idx) in order.iter().enumerate() {
            let Some((_, g1, g2, g3)) = k.seqs.get(idx) else {
                continue;
            };
            let s = &a[(pos + 1).to_string().as_str()];
            let b = |x: bool| f64::from(u8::from(x));
            e.entry("g1".into()).or_default().push(b(s["G1"]
                .as_str()
                .is_some_and(|x| x.trim().eq_ignore_ascii_case(g1))));
            e.entry("g2".into()).or_default().push(b(eq(&s["G2"], g2)));
            e.entry("g3".into()).or_default().push(b(eq(&s["G3"], g3)));
        }
        per_reader.push(json!({"reader": r.dir, "design": k.design, "persona": p, "parsed": r.answer.is_some()}));
    }
    let mut designs = BTreeMap::new();
    let mut entries = Vec::new();
    for (d, per) in &acc {
        let pm = |p: &str, k: &str| per.get(p).and_then(|m| m.get(k)).and_then(|v| mean(v));
        let primary = weighted(pm("F", "g1"), pm("H", "g1"));
        let g23 = weighted(
            pm("F", "g2").zip(pm("F", "g3")).map(|(a, b)| (a + b) / 2.0),
            pm("H", "g2").zip(pm("H", "g3")).map(|(a, b)| (a + b) / 2.0),
        );
        let plan: Option<m10::Plan> = io::read(&raw.join(d).join("computed.json"))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok());
        let share = plan
            .as_ref()
            .and_then(|p| p.mean.get("area_share_after").copied());
        let taps = plan.as_ref().and_then(|p| p.mean.get("taps").copied());
        let mut q = BTreeMap::new();
        for p in prompts::PERSONAS {
            let m: BTreeMap<&str, Option<f64>> = ["g1", "g2", "g3"]
                .into_iter()
                .map(|k| (k, r4(pm(p, k))))
                .collect();
            q.insert(p.to_owned(), m);
        }
        designs.insert(
            d.clone(),
            json!({"persona": q, "weighted_g1": r4(primary), "weighted_g2_g3": r4(g23),
            "area_share_after": share, "taps": taps}),
        );
        entries.push(Entry {
            design: d.clone(),
            primary,
            ties: vec![g23, share, taps],
            eligible: true,
        });
    }
    let sel = select(
        &entries,
        &[true, true, false],
        &[
            "G2+G3 (higher wins)",
            "shared area share (higher wins)",
            "taps (fewer wins)",
        ],
    );
    Ok(
        json!({"experiment": "m10", "harness": crate::HARNESS_VERSION, "designs": designs,
              "readers": per_reader, "selection": sel,
              "limit": "Persona ajanı insan değildir; bu ölçüm ön elemedir, son söz insan testi v2'dedir."}),
    )
}

/// Scores `experiment` and writes `raw/scores.json` and `raw/scores.md`; returns the
/// markdown.
///
/// # Errors
/// Unknown experiment, unreadable records.
pub fn run(exp: &Path, experiment: &str) -> Result<String, String> {
    let (root, v) = match experiment {
        "m8" => (m8::root(exp), m8(exp)?),
        "m9" => (m9::root(exp), m9(exp)?),
        "m10" => (m10::root(exp), m10(exp)?),
        e => return Err(format!("unknown experiment `{e}`")),
    };
    io::write_json(&root.join("raw").join("scores.json"), &v)?;
    let md = markdown(&v);
    io::write(&root.join("raw").join("scores.md"), md.as_bytes())?;
    Ok(md)
}

/// A short markdown view of a scores record.
#[must_use]
pub fn markdown(v: &Value) -> String {
    let mut s = format!(
        "# {} scores ({})\n\n",
        v["experiment"].as_str().unwrap_or("").to_uppercase(),
        v["harness"].as_str().unwrap_or("")
    );
    if let Some(d) = v["designs"].as_object() {
        s.push_str("| design | measures |\n|---|---|\n");
        for (k, x) in d {
            let mut m = x.clone();
            if let Some(o) = m.as_object_mut() {
                o.remove("persona");
            }
            s.push_str(&format!("| `{k}` | `{m}` |\n"));
        }
    }
    s.push_str(&format!("\nSelection: {}\n\n", v["selection"]));
    s.push_str(&format!("{}\n", v["limit"].as_str().unwrap_or("")));
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_answers() {
        assert_eq!(norm(" \"Yukarı\". "), "yukarı");
        assert_eq!(norm("Kapandı:  gerçekleşti"), "kapandı: gerçekleşti");
        assert!(eq(&json!("YUKARI"), "yukarı"));
    }

    #[test]
    fn rule_with_tiebreak() {
        let e = |d: &str, p: f64, t: f64| Entry {
            design: d.into(),
            primary: Some(p),
            ties: vec![Some(t)],
            eligible: true,
        };
        let s = select(
            &[e("a", 0.80, 2.0), e("b", 0.78, 1.0), e("r3", 0.99, 0.0)],
            &[false],
            &["rank"],
        );
        assert_eq!(s.selected.as_deref(), Some("b"));
        assert_eq!(s.ranking[2], "r3");
        let s = select(&[e("a", 0.90, 2.0), e("b", 0.70, 1.0)], &[false], &["rank"]);
        assert_eq!(s.selected.as_deref(), Some("a"));
        let mut x = e("a", 0.9, 0.0);
        x.eligible = false;
        let s = select(&[x, e("b", 0.1, 0.0)], &[false], &["rank"]);
        assert_eq!(s.selected.as_deref(), Some("b"));
    }
}
