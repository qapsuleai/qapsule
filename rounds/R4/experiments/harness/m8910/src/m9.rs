//! M9 — the feed's layout (SPEC §3).
//!
//! - [`scenario`]: the synthetic scenario — a "last look" moment `t0` and a "now" `t1`
//!   (one trading session later), a watchlist of 8 instruments and 2 positions, one of which
//!   has a negative transition between `t0` and `t1`, sensitivity `balanced` — found in the
//!   demo's replay and checked through the web shell's core (the negative transition must be
//!   in the reader's feed at `t1`);
//! - [`key`]: the answer key from the engine (the `capsules` answer at `t1`), not from a
//!   layout: G1 the instruments whose capsules moved after `t0` (with the capsule's
//!   direction), G2 the held instrument with the negative transition and what it was, G3
//!   the capsules in the highest reliability bucket with `n ≥ 5`;
//! - [`screens`]: the live demo in headless Chromium with the layout by name, the
//!   audience set on the settings page, the last look marked at `t0`, the feed at `t1`
//!   scrolled screen by screen (at most 6), labels as layout elements.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use qp_schema::capsule::{Direction, Stage};
use qp_schema::protocol::CapsuleItem;

use crate::blind;
use crate::io;
use crate::prompts;
use crate::rng::Rng;
use crate::shots;
use crate::world::{Core, Raw, World};

/// Steps between the last look and now: one trading session of the demo's hourly steps
/// (daily capsules move once a session, at its first step).
pub const WINDOW: u64 = 9;
/// First candidate last-look step (the ledgers need history).
pub const FIRST_T0: u64 = 600;
/// Watchlist size.
pub const WATCH: usize = 8;
/// Most screens.
pub const MAX_SCREENS: usize = 6;
/// Port of the demo the screenshots use.
pub const PORT: u16 = 8471;

/// The scenario.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Scenario {
    /// Harness version.
    pub harness: String,
    /// Last-look step.
    pub t0_step: u64,
    /// Now step.
    pub t1_step: u64,
    /// Last-look time (ms).
    pub t0: i64,
    /// Now (ms).
    pub t1: i64,
    /// The audience choice (settings page): group, sensitivity, watch, long, short.
    pub choice: Value,
    /// The two held instruments as the prompt names them (X, Y).
    pub held: [String; 2],
    /// The held instrument with the negative transition.
    pub negative: String,
}

fn negative_word(s: Stage) -> Option<&'static str> {
    match s {
        Stage::Weakened => Some(prompts::G2_WHAT[0]),
        Stage::Invalidated => Some(prompts::G2_WHAT[1]),
        Stage::Retracted => Some(prompts::G2_WHAT[2]),
        _ => None,
    }
}

/// The reader's feed at `t1` through the shell core with `choice` set at `t0` and the
/// last look marked there.
///
/// # Errors
/// The world does not start.
pub fn feed_at(
    sc: &Scenario,
    feed: Option<&str>,
) -> Result<(Vec<CapsuleItem>, Value, Vec<bool>), String> {
    let mut w = World::demo();
    let mut c = Core::connect(&mut w, "m9", crate::READER, "tr")?;
    if let Some(f) = feed {
        c.act(&mut w, |k| k.set_look(&json!({"feed": f})));
    }
    c.seek(&mut w, sc.t0_step);
    let choice = sc.choice.clone();
    c.act(&mut w, |k| k.set_audience(&choice));
    c.act(&mut w, qp_wasm::shell::ShellCore::mark_seen);
    c.seek(&mut w, sc.t1_step);
    c.pump(&mut w);
    let rows = c.core.feed_json();
    let items = c.core.feed().to_vec();
    let moved = items.iter().map(|i| c.core.known_at(i) > sc.t0).collect();
    Ok((items, rows, moved))
}

/// Finds the scenario (module docs): the first window `(t0, t0 + WINDOW]` from
/// [`FIRST_T0`] on with a negative transition that reaches the reader's feed.
///
/// # Errors
/// None found, or the world does not start.
pub fn scenario() -> Result<Scenario, String> {
    let mut w = World::demo();
    let mut a = Raw::connect(&mut w, "m9s", crate::READER, "tr");
    let steps = w.steps();
    let mut t0_step = FIRST_T0;
    while t0_step + WINDOW < steps {
        a.replay(&mut w, json!({"kind":"seek","step":t0_step}));
        let t0 = w.now();
        a.replay(&mut w, json!({"kind":"step","n":WINDOW}));
        let t1_step = w.step();
        // What became known after t0 (the core's rule, `ShellCore::known_at`).
        let all = a.capsules(&mut w, json!({}));
        let moved: Vec<CapsuleItem> = all
            .into_iter()
            .filter(|i| known_at(&mut w, i) > t0)
            .collect();
        let neg = moved.iter().find(|i| negative_word(i.stage).is_some());
        if let Some(n) = neg {
            let p = n.instrument.clone();
            let mut others: Vec<String> = moved
                .iter()
                .filter(|i| i.instrument != p)
                .map(|i| i.instrument.clone())
                .collect();
            others.sort();
            others.dedup();
            let neg_insts: Vec<String> = moved
                .iter()
                .filter(|i| negative_word(i.stage).is_some())
                .map(|i| i.instrument.clone())
                .collect();
            let q = others
                .iter()
                .find(|s| !neg_insts.contains(s))
                .cloned()
                .or_else(|| {
                    (1..=30)
                        .map(|k| format!("SYN{k:02}"))
                        .find(|s| *s != p && !neg_insts.contains(s))
                });
            let Some(q) = q else {
                t0_step = t1_step + 1;
                continue;
            };
            let mut watch: Vec<String> = others
                .iter()
                .filter(|s| **s != q && !neg_insts.contains(s))
                .take(WATCH)
                .cloned()
                .collect();
            for k in 1..=30 {
                if watch.len() >= WATCH {
                    break;
                }
                let s = format!("SYN{k:02}");
                if s != p && s != q && !watch.contains(&s) && !neg_insts.contains(&s) {
                    watch.push(s);
                }
            }
            watch.sort();
            let mut held = [p.clone(), q.clone()];
            held.sort();
            let sc = Scenario {
                harness: crate::HARNESS_VERSION.to_owned(),
                t0_step,
                t1_step,
                t0,
                t1: w.now(),
                choice: json!({"group":"habituated","sensitivity":"balanced","watch":watch,
                               "long":[p, q],"short":[]}),
                held,
                negative: n.instrument.clone(),
            };
            // The negative transition must be in the reader's feed at t1.
            let (items, _, fresh) = feed_at(&sc, None)?;
            if items
                .iter()
                .zip(&fresh)
                .any(|(i, f)| *f && i.instrument == sc.negative && negative_word(i.stage).is_some())
            {
                return Ok(sc);
            }
        }
        t0_step = t1_step + 1;
    }
    Err("no window with a negative transition in the reader's feed".into())
}

/// When the reader could first know `i`'s latest transition (as `ShellCore::known_at`,
/// over the authority's bars).
fn known_at(w: &mut World, i: &CapsuleItem) -> i64 {
    let b = w.bars(&i.series);
    usize::try_from(i.last_bar)
        .ok()
        .and_then(|x| b.get(x + 1))
        .map_or(i64::MAX, |x| x.t)
}

/// The hidden key of one design's page set.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Key {
    /// Experiment.
    pub experiment: String,
    /// Feed layout.
    pub design: String,
    /// G1: `"<instrument> <direction>"` of every capsule moved after `t0`.
    pub g1: Vec<String>,
    /// G1 options' instruments.
    pub instruments: Vec<String>,
    /// G2: which held instrument (or "ikisinde de") and what.
    pub g2_which: String,
    /// G2 what.
    pub g2_what: String,
    /// G3: labels of the capsules in the highest reliability bucket (n ≥ 5).
    pub g3: Vec<String>,
    /// Capsule id → label.
    pub labels: BTreeMap<String, String>,
    /// Labels needed per task (G1 every moved capsule, G2 the negative one, G3 any of).
    pub needed: BTreeMap<String, Vec<String>>,
    /// Layout's own rule count (M9 tiebreak).
    pub rules: u32,
}

fn dir_word(d: Direction) -> &'static str {
    match d {
        Direction::Up => prompts::G1_DIRS[0],
        Direction::Down => prompts::G1_DIRS[1],
        Direction::Neutral => prompts::G1_DIRS[2],
    }
}

/// Labels: two consonants (no hex letter), seeded per page set.
fn labels(ids: &[String], code: &str) -> BTreeMap<String, String> {
    const C: [char; 10] = ['K', 'L', 'M', 'N', 'P', 'R', 'S', 'T', 'V', 'Z'];
    let mut pool: Vec<String> = C
        .iter()
        .flat_map(|a| C.iter().map(move |b| format!("{a}{b}")))
        .filter(|s| s.chars().next() != s.chars().nth(1))
        .collect();
    Rng::derive(u64::from_str_radix(code, 16).unwrap_or(0), "m9-labels").shuffle(&mut pool);
    ids.iter().cloned().zip(pool).collect()
}

fn bucket(i: &CapsuleItem) -> Option<(u8, f64)> {
    let l = &i.ledger_at_birth;
    if l.n < 5 {
        return None;
    }
    let r = f64::from(l.realized) / f64::from(l.n);
    Some((
        if r >= 0.6 {
            3
        } else if r >= 0.4 {
            2
        } else {
            1
        },
        r,
    ))
}

/// The key of design `feed` (module docs).
#[must_use]
pub fn key(sc: &Scenario, feed: &str, items: &[CapsuleItem], fresh: &[bool], code: &str) -> Key {
    let ids: Vec<String> = items.iter().map(|i| i.id.clone()).collect();
    let lab = labels(&ids, code);
    let moved: Vec<&CapsuleItem> = items
        .iter()
        .zip(fresh)
        .filter(|(_, f)| **f)
        .map(|(i, _)| i)
        .collect();
    let mut g1: Vec<String> = moved
        .iter()
        .map(|i| format!("{} {}", i.instrument, dir_word(i.direction)))
        .collect();
    g1.sort();
    g1.dedup();
    let mut instruments: Vec<String> = items.iter().map(|i| i.instrument.clone()).collect();
    for k in ["watch", "long", "short"] {
        if let Some(a) = sc.choice[k].as_array() {
            instruments.extend(a.iter().filter_map(|x| x.as_str().map(str::to_owned)));
        }
    }
    instruments.sort();
    instruments.dedup();
    let negs: Vec<&CapsuleItem> = moved
        .iter()
        .copied()
        .filter(|i| sc.held.contains(&i.instrument) && negative_word(i.stage).is_some())
        .collect();
    let mut held_neg: Vec<&str> = negs.iter().map(|i| i.instrument.as_str()).collect();
    held_neg.sort_unstable();
    held_neg.dedup();
    let g2_which = match held_neg.len() {
        0 => prompts::G2_WHICH[3].to_owned(),
        1 => held_neg[0].to_owned(),
        _ => prompts::G2_WHICH[2].to_owned(),
    };
    let g2_what = negs
        .first()
        .and_then(|i| negative_word(i.stage))
        .unwrap_or(prompts::G2_WHAT[3])
        .to_owned();
    let best = items.iter().filter_map(bucket).map(|b| b.0).max();
    let g3: Vec<String> = items
        .iter()
        .filter(|i| best.is_some() && bucket(i).map(|b| b.0) == best)
        .filter_map(|i| lab.get(&i.id).cloned())
        .collect();
    let mut needed = BTreeMap::new();
    needed.insert(
        "G1".to_owned(),
        moved
            .iter()
            .filter_map(|i| lab.get(&i.id).cloned())
            .collect(),
    );
    needed.insert(
        "G2".to_owned(),
        negs.iter()
            .filter_map(|i| lab.get(&i.id).cloned())
            .collect(),
    );
    needed.insert("G3".to_owned(), g3.clone());
    Key {
        experiment: "m9".into(),
        design: feed.to_owned(),
        g1,
        instruments,
        g2_which,
        g2_what,
        g3,
        labels: lab,
        needed,
        rules: qp_grammar::feeds::by_name(Some(feed)).rules,
    }
}

/// Root of M9's records.
#[must_use]
pub fn root(exp: &Path) -> PathBuf {
    exp.join("M9")
}

/// The code of design `feed`'s page set.
#[must_use]
pub fn code(feed: &str, salt: u64) -> String {
    blind::code(salt, &format!("m9|{feed}"))
}

/// Builds design `feed`'s screens (`raw/screens/<code>-<k>.png`), key, positions and
/// persona prompts. `shoot: false` skips Chromium (prompts then list no screen unless
/// `raw/keys/<code>.positions.json` exists).
///
/// # Errors
/// Unknown layout, unreadable scenario, Chromium failure.
pub fn screens(
    feed: &str,
    sc: &Scenario,
    exp: &Path,
    salt: u64,
    shoot: bool,
) -> Result<String, String> {
    io::check_name(feed)?;
    if !qp_grammar::feeds::is_feed(feed) {
        return Err(format!(
            "unknown feed layout `{feed}` (known: {})",
            qp_grammar::feeds::names().join(", ")
        ));
    }
    let raw = root(exp).join("raw");
    let code = code(feed, salt);
    let (items, rows, fresh) = feed_at(sc, Some(feed))?;
    let k = key(sc, feed, &items, &fresh, &code);
    io::write_json(&raw.join("keys").join(format!("{code}.json")), &k)?;
    io::write_json(&raw.join("keys").join(format!("{code}.rows.json")), &rows)?;
    let mut m: BTreeMap<String, Value> = if raw.join("key.json").exists() {
        serde_json::from_str(&io::read(&raw.join("key.json"))?).map_err(|e| e.to_string())?
    } else {
        BTreeMap::new()
    };
    m.insert(code.clone(), json!({"experiment":"m9","design":feed}));
    io::write_json(&raw.join("key.json"), &m)?;
    let pos = raw.join("keys").join(format!("{code}.positions.json"));
    if shoot {
        let plan = json!({
            "kind": "m9", "qp": shots::qp_bin().display().to_string(), "port": PORT,
            "settings": {"feed": feed, "lang": "tr", "theme": "light"},
            "t0_step": sc.t0_step, "t1_step": sc.t1_step, "choice": sc.choice,
            "labels": k.labels, "code": code, "max_screens": MAX_SCREENS,
            "out": raw.join("screens").display().to_string(), "positions": pos.display().to_string(),
        });
        let pf = io::scratch().join("m9").join(format!("{code}.plan.json"));
        io::write_json(&pf, &plan)?;
        shots::node("demo_shots.mjs", &[&pf.display().to_string()])?;
    }
    let n = if pos.exists() {
        let v: Value = serde_json::from_str(&io::read(&pos)?).map_err(|e| e.to_string())?;
        usize::try_from(v["screens"].as_u64().unwrap_or(0)).unwrap_or(0)
    } else {
        0
    };
    let names: Vec<String> = (1..=n).map(|i| format!("{code}-{i}.png")).collect();
    let held = [sc.held[0].as_str(), sc.held[1].as_str()];
    for p in prompts::PERSONAS {
        let t = prompts::m9_persona(p, &names, held, &k.instruments);
        blind::check("m9 prompt", &t)?;
        io::write(
            &raw.join("prompts").join(format!("{p}_{code}.txt")),
            t.as_bytes(),
        )?;
    }
    Ok(code)
}

/// The minimum scroll the key needs, from the positions (`label → [screen…]`, 1-based):
/// per task, the latest first-visible screen of the needed labels (G3: the earliest of
/// any), minus one.
#[must_use]
pub fn min_scroll(k: &Key, positions: &Value) -> BTreeMap<String, Option<u64>> {
    let first = |l: &str| -> Option<u64> {
        positions["labels"][l]
            .as_array()
            .and_then(|a| a.iter().filter_map(Value::as_u64).min())
    };
    let mut out = BTreeMap::new();
    for (task, labs) in &k.needed {
        let v: Vec<Option<u64>> = labs.iter().map(|l| first(l)).collect();
        let s = if labs.is_empty() {
            Some(1)
        } else if task == "G3" {
            v.iter().flatten().min().copied()
        } else if v.iter().all(Option::is_some) {
            v.iter().flatten().max().copied()
        } else {
            None
        };
        out.insert(task.clone(), s.map(|x| x.saturating_sub(1)));
    }
    out
}
