//! M5 — outcome definition (SPEC M5).
//!
//! Run: the measured outcome rule (inside [`sim::GuardedOutcome`]) with the reference
//! theme and ranker. Per dataset the raw holds:
//! - the base ledger in close order `[key, stage, day, story direction]`;
//! - per perturbation (the alternative's `perturbations()`): ledger per key and the final
//!   verdict of every base-closed capsule in the perturbed run (`0` = absent / open);
//! - look-ahead: guard violations; for three cuts T (25/50/75 %), bars after T shuffled
//!   (seeded permutation, time stamps kept): mismatches among events with `t ≤ T`, among
//!   verdict events with `t ≤ T`, and ledger statistics at T (closes `≤ T`) per key;
//! - decoy: the same data with every detection's direction flipped (up ↔ down) →
//!   ledger per key, directional stories only.
//!
//! Score: stability (p(n) at n = 25, 50, 100, 200, final per key; |p(n) − p(final)|;
//! first vs second half of the days), sensitivity (rate change per key; verdict
//! agreement), look-ahead (all zero = pass), meaningfulness (real − decoy rate on
//! directional stories; expired share).

use std::cell::Cell;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::rc::Rc;

use qp_capsule::engine::Engine;
use qp_capsule::{pipeline::simulate_engine, EngineConfig, InstrumentInput, OutcomeRule};
use qp_schema::capsule::Direction;
use serde::{Deserialize, Serialize};

use crate::common::{
    self, cut_times, dir_code, f, ratio, shuffle_after, stage_code, to_json, write_file, DataKind,
    Dataset, EXPIRED, INVALIDATED, REALIZED,
};
use crate::m3::{raw_location, read_raws, write_summary};
use crate::registry::{self, OutcomeFactory};
use crate::sim;

/// Per-key ledger `[key, n, realized]` (key = name).
pub type KeyStats = Vec<(String, u64, u64)>;

/// One perturbation's result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PertRaw {
    /// Name.
    pub name: String,
    /// Ledger per key.
    pub keys: KeyStats,
    /// Final verdict (stage code, 0 = absent or still open) of each base-closed capsule,
    /// aligned with `M5Raw::base_verdicts`.
    pub verdicts: Vec<u8>,
}

/// One look-ahead cut.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CutRaw {
    /// T.
    pub cut_t: i64,
    /// Events with `t ≤ T` (base).
    pub compared: u64,
    /// Mismatched events (JSON) with `t ≤ T`.
    pub mismatched: u64,
    /// Mismatched verdict events (`realized`/`invalidated`/`expired`) with `t ≤ T`.
    pub verdict_mismatched: u64,
    /// Keys whose ledger statistic at T (closes `≤ T`) differs.
    pub ledger_keys_differ: u64,
    /// Events after T that differ (the future really changed).
    pub future_changed: u64,
}

/// Raw record of one dataset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct M5Raw {
    /// Dataset.
    pub dataset: String,
    /// Alternative.
    pub alt: String,
    /// Rules (theme, outcome, ranker).
    pub rules: Vec<String>,
    /// Trading days.
    pub n_days: u32,
    /// Ledger keys.
    pub keys: Vec<String>,
    /// `[key index, stage, day, direction]` per ledger row, close order.
    pub ledger: Vec<[u32; 4]>,
    /// Final verdict of every closed capsule (close order).
    pub base_verdicts: Vec<u8>,
    /// Stories still open at the end.
    pub open_at_end: u64,
    /// Perturbations.
    pub perturbations: Vec<PertRaw>,
    /// Guard violations (slices longer than `now + 1`).
    pub guard_violations: u64,
    /// Look-ahead cuts.
    pub lookahead: Vec<CutRaw>,
    /// Decoy ledger per key, directional stories only.
    pub decoy: KeyStats,
}

/// Outcome factory that may see the data (only test cheaters use it).
pub type DataOutcome<'a> = &'a (dyn Fn(&[InstrumentInput]) -> Box<dyn OutcomeRule> + Sync);

fn run_with(
    inputs: &[InstrumentInput],
    rule: Box<dyn OutcomeRule>,
    flipped: bool,
) -> (Engine, Rc<Cell<u64>>) {
    let violations = Rc::new(Cell::new(0));
    let guarded = Box::new(sim::GuardedOutcome {
        inner: rule,
        violations: Rc::clone(&violations),
    });
    let cfg = EngineConfig {
        theme: registry::reference_theme(),
        outcome: guarded,
        ranker: registry::reference_ranker(),
    };
    let (e, _) = if flipped {
        simulate_engine(inputs, &sim::flipped_detectors, cfg)
    } else {
        simulate_engine(inputs, &sim::detectors, cfg)
    };
    (e, violations)
}

fn key_stats(e: &Engine, directional_only: bool) -> KeyStats {
    let dir: BTreeMap<&str, Direction> = sim::all_stories(e)
        .iter()
        .map(|s| (s.id.as_str(), s.story.direction))
        .collect();
    let mut m: BTreeMap<String, (u64, u64)> = BTreeMap::new();
    for r in e.ledger_records() {
        let cap = r.entry.capsule.as_deref().unwrap_or("");
        if directional_only && dir.get(cap).is_none_or(|d| *d == Direction::Neutral) {
            continue;
        }
        let k = m
            .entry(r.entry.key.clone().unwrap_or_default())
            .or_default();
        k.0 += 1;
        if r.entry.outcome == qp_schema::Outcome::Confirmed {
            k.1 += 1;
        }
    }
    m.into_iter().map(|(k, (n, r))| (k, n, r)).collect()
}

fn final_verdicts(e: &Engine) -> BTreeMap<String, u8> {
    e.closed_stories()
        .iter()
        .map(|s| (s.id.clone(), stage_code(s.story.stage)))
        .collect()
}

/// Run one dataset.
#[must_use]
pub fn run_dataset(
    ds: &Dataset,
    alt: &str,
    make: DataOutcome<'_>,
    perts: &[String],
    make_pert: &(dyn Fn(&str) -> Box<dyn OutcomeRule> + Sync),
) -> M5Raw {
    let (engine, viol) = run_with(&ds.inputs, make(&ds.inputs), false);
    let (tn, on, rn) = engine.rule_names();
    let stories = sim::all_stories(&engine);
    let story_dir: BTreeMap<&str, Direction> = stories
        .iter()
        .map(|s| (s.id.as_str(), s.story.direction))
        .collect();
    let inst_idx: BTreeMap<&str, usize> = ds
        .inputs
        .iter()
        .enumerate()
        .map(|(i, x)| (x.symbol.as_str(), i))
        .collect();
    let mut keys: Vec<String> = Vec::new();
    let mut ledger = Vec::new();
    for r in engine.ledger_records() {
        let key = r.entry.key.clone().unwrap_or_default();
        let ki = match keys.iter().position(|k| *k == key) {
            Some(p) => p,
            None => {
                keys.push(key);
                keys.len() - 1
            }
        };
        let cap = r.entry.capsule.as_deref().unwrap_or("");
        let inst = cap
            .split('.')
            .nth(1)
            .and_then(|s| inst_idx.get(s))
            .copied()
            .unwrap_or(0);
        let bar = usize::try_from(r.entry.bar.unwrap_or(0)).unwrap_or(0);
        let day = ds.day_of_bar[inst].get(bar).copied().unwrap_or(0);
        ledger.push([
            u32::try_from(ki).unwrap_or(0),
            u32::from(r.entry.stage.map_or(0, stage_code)),
            day,
            u32::from(dir_code(
                story_dir.get(cap).copied().unwrap_or(Direction::Neutral),
            )),
        ]);
    }
    let closed: Vec<(String, u8)> = engine
        .closed_stories()
        .iter()
        .map(|s| (s.id.clone(), stage_code(s.story.stage)))
        .collect();
    let base_verdicts: Vec<u8> = closed.iter().map(|x| x.1).collect();

    let mut perturbations = Vec::new();
    for name in perts {
        let (e2, _) = run_with(&ds.inputs, make_pert(name), false);
        let fv = final_verdicts(&e2);
        perturbations.push(PertRaw {
            name: name.clone(),
            keys: key_stats(&e2, false),
            verdicts: closed
                .iter()
                .map(|(id, _)| fv.get(id).copied().unwrap_or(0))
                .collect(),
        });
    }

    // Look-ahead.
    let base_ev: Vec<(i64, String, u8)> = engine
        .events()
        .iter()
        .map(|ev| {
            (
                ev.transition.t,
                serde_json::to_string(ev).expect("json"),
                stage_code(ev.transition.stage),
            )
        })
        .collect();
    let mut lookahead = Vec::new();
    let mut guard_violations = viol.get();
    for (ci, cut) in cut_times(&ds.inputs).into_iter().enumerate() {
        let sh = shuffle_after(&ds.inputs, cut, 0x5EED + ci as u64);
        let (e2, v2) = run_with(&sh, make(&sh), false);
        guard_violations += v2.get();
        let other: Vec<(i64, String, u8)> = e2
            .events()
            .iter()
            .map(|ev| {
                (
                    ev.transition.t,
                    serde_json::to_string(ev).expect("json"),
                    stage_code(ev.transition.stage),
                )
            })
            .collect();
        let pa: Vec<_> = base_ev.iter().filter(|x| x.0 <= cut).collect();
        let pb: Vec<_> = other.iter().filter(|x| x.0 <= cut).collect();
        let mut mism = pa.len().abs_diff(pb.len()) as u64;
        for (a, b) in pa.iter().zip(pb.iter()) {
            if a.1 != b.1 {
                mism += 1;
            }
        }
        let va: Vec<_> = pa
            .iter()
            .filter(|x| (REALIZED..=EXPIRED).contains(&x.2))
            .collect();
        let vb: Vec<_> = pb
            .iter()
            .filter(|x| (REALIZED..=EXPIRED).contains(&x.2))
            .collect();
        let mut vm = va.len().abs_diff(vb.len()) as u64;
        for (a, b) in va.iter().zip(vb.iter()) {
            if a.1 != b.1 {
                vm += 1;
            }
        }
        let mut all_keys: Vec<String> = engine
            .ledger_records()
            .iter()
            .chain(e2.ledger_records().iter())
            .filter_map(|r| r.entry.key.clone())
            .collect();
        all_keys.sort();
        all_keys.dedup();
        let lk = all_keys
            .iter()
            .filter(|k| engine.ledger_stat(k, cut + 1) != e2.ledger_stat(k, cut + 1))
            .count() as u64;
        let fa: Vec<_> = base_ev.iter().filter(|x| x.0 > cut).collect();
        let fb: Vec<_> = other.iter().filter(|x| x.0 > cut).collect();
        let mut fut = fa.len().abs_diff(fb.len()) as u64;
        for (a, b) in fa.iter().zip(fb.iter()) {
            if a.1 != b.1 {
                fut += 1;
            }
        }
        lookahead.push(CutRaw {
            cut_t: cut,
            compared: pa.len() as u64,
            mismatched: mism,
            verdict_mismatched: vm,
            ledger_keys_differ: lk,
            future_changed: fut,
        });
    }

    let (ed, _) = run_with(&ds.inputs, make(&ds.inputs), true);
    let decoy = key_stats(&ed, true);

    M5Raw {
        dataset: ds.name.clone(),
        alt: alt.to_owned(),
        rules: vec![tn.to_owned(), on.to_owned(), rn.to_owned()],
        n_days: ds.n_days,
        keys,
        ledger,
        base_verdicts,
        open_at_end: engine.open_stories().len() as u64,
        perturbations,
        guard_violations,
        lookahead,
        decoy,
    }
}

// ---------------------------------------------------------------------------
// Score
// ---------------------------------------------------------------------------

/// Sample sizes of the stability trace (`0` = final).
pub const TRACE_N: [u64; 5] = [25, 50, 100, 200, 0];

/// Stability of one key.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct KeyStability {
    /// Key.
    pub key: String,
    /// Closes.
    pub n: u64,
    /// p(25), p(50), p(100), p(200), p(final).
    pub p: Vec<Option<f64>>,
    /// |p(n) − p(final)| for n = 25, 50, 100, 200.
    pub dev: Vec<Option<f64>>,
    /// Rate in the first / second half of the days, and the difference.
    pub halves: [Option<f64>; 3],
}

/// `(key, base rate, perturbed rate, change)`.
pub type KeyChange = (String, Option<f64>, Option<f64>, Option<f64>);

/// Sensitivity of one perturbation.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PertScore {
    /// Name.
    pub name: String,
    /// Per key: (key, base rate, perturbed rate, change).
    pub keys: Vec<KeyChange>,
    /// Overall rate change.
    pub overall_change: Option<f64>,
    /// Base-closed capsules also closed in the perturbed run.
    pub common: u64,
    /// Of those, same verdict.
    pub agree: u64,
    /// agree / common.
    pub agreement: Option<f64>,
    /// common / base-closed.
    pub coverage: Option<f64>,
}

/// Dataset score.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DatasetScore {
    /// Name.
    pub dataset: String,
    /// Closes, realized, invalidated, expired.
    pub closes: [u64; 4],
    /// Hit rate (realized / closes).
    pub rate: Option<f64>,
    /// Expired share.
    pub expired_share: Option<f64>,
    /// Per key stability.
    pub keys: Vec<KeyStability>,
    /// Max over keys (with n ≥ that n) of |p(n) − p(final)|, for n = 25, 50, 100, 200.
    pub max_dev: Vec<Option<f64>>,
    /// Perturbations.
    pub perturbations: Vec<PertScore>,
    /// Look-ahead (guard violations + all cuts zero).
    pub lookahead_ok: bool,
    /// Guard violations.
    pub guard_violations: u64,
    /// Cuts.
    pub cuts: Vec<CutRaw>,
    /// Real directional rate, decoy directional rate, difference.
    pub decoy: [Option<f64>; 3],
}

/// Summary.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct M5Summary {
    /// Alternative.
    pub alt: String,
    /// Data.
    pub data: String,
    /// Rules.
    pub rules: Vec<String>,
    /// Per dataset.
    pub datasets: Vec<DatasetScore>,
}

fn rate_of(ks: &KeyStats) -> Option<f64> {
    let n: u64 = ks.iter().map(|k| k.1).sum();
    let r: u64 = ks.iter().map(|k| k.2).sum();
    ratio(r, n)
}

/// Score one dataset.
#[must_use]
pub fn score_one(r: &M5Raw) -> DatasetScore {
    let mut closes = [0u64; 4];
    for row in &r.ledger {
        closes[0] += 1;
        match u8::try_from(row[1]).unwrap_or(0) {
            REALIZED => closes[1] += 1,
            INVALIDATED => closes[2] += 1,
            EXPIRED => closes[3] += 1,
            _ => {}
        }
    }
    let half = r.n_days / 2;
    let mut keys = Vec::new();
    let mut base_ks: KeyStats = Vec::new();
    for (ki, key) in r.keys.iter().enumerate() {
        let rows: Vec<&[u32; 4]> = r.ledger.iter().filter(|x| x[0] as usize == ki).collect();
        let n = rows.len() as u64;
        let mut cum = Vec::with_capacity(rows.len());
        let mut hit = 0u64;
        for x in &rows {
            if x[1] == u32::from(REALIZED) {
                hit += 1;
            }
            cum.push(hit);
        }
        let p_at = |m: u64| -> Option<f64> {
            if m == 0 {
                return ratio(hit, n);
            }
            (m <= n)
                .then(|| ratio(cum[usize::try_from(m).unwrap_or(1) - 1], m))
                .flatten()
        };
        let p: Vec<Option<f64>> = TRACE_N.iter().map(|&m| p_at(m)).collect();
        let fin = p[4];
        let dev = p[..4]
            .iter()
            .map(|x| match (x, fin) {
                (Some(a), Some(b)) => Some((a - b).abs()),
                _ => None,
            })
            .collect();
        let (mut n1, mut h1, mut n2, mut h2) = (0u64, 0u64, 0u64, 0u64);
        for x in &rows {
            let hitb = u64::from(x[1] == u32::from(REALIZED));
            if x[2] < half {
                n1 += 1;
                h1 += hitb;
            } else {
                n2 += 1;
                h2 += hitb;
            }
        }
        let (a, b) = (ratio(h1, n1), ratio(h2, n2));
        keys.push(KeyStability {
            key: key.clone(),
            n,
            p,
            dev,
            halves: [a, b, a.zip(b).map(|(a, b)| a - b)],
        });
        base_ks.push((key.clone(), n, hit));
    }
    let max_dev = (0..4)
        .map(|i| keys.iter().filter_map(|k| k.dev[i]).reduce(f64::max))
        .collect();
    let base_rate = |k: &str| {
        base_ks
            .iter()
            .find(|x| x.0 == k)
            .and_then(|x| ratio(x.2, x.1))
    };
    let perturbations = r
        .perturbations
        .iter()
        .map(|p| {
            let mut names: Vec<String> = r.keys.clone();
            for k in &p.keys {
                if !names.contains(&k.0) {
                    names.push(k.0.clone());
                }
            }
            let rows = names
                .iter()
                .map(|k| {
                    let a = base_rate(k);
                    let b = p
                        .keys
                        .iter()
                        .find(|x| &x.0 == k)
                        .and_then(|x| ratio(x.2, x.1));
                    (k.clone(), a, b, a.zip(b).map(|(a, b)| b - a))
                })
                .collect();
            let mut common = 0u64;
            let mut agree = 0u64;
            for (a, b) in r.base_verdicts.iter().zip(p.verdicts.iter()) {
                if *b != 0 {
                    common += 1;
                    if a == b {
                        agree += 1;
                    }
                }
            }
            PertScore {
                name: p.name.clone(),
                keys: rows,
                overall_change: rate_of(&p.keys).zip(rate_of(&base_ks)).map(|(b, a)| b - a),
                common,
                agree,
                agreement: ratio(agree, common),
                coverage: ratio(common, r.base_verdicts.len() as u64),
            }
        })
        .collect();
    let ok = r.guard_violations == 0
        && r.lookahead
            .iter()
            .all(|c| c.mismatched == 0 && c.verdict_mismatched == 0 && c.ledger_keys_differ == 0);
    let mut dn = 0u64;
    let mut dh = 0u64;
    for row in &r.ledger {
        if row[3] != u32::from(dir_code(Direction::Neutral)) {
            dn += 1;
            dh += u64::from(row[1] == u32::from(REALIZED));
        }
    }
    let real_dir = ratio(dh, dn);
    let decoy = rate_of(&r.decoy);
    DatasetScore {
        dataset: r.dataset.clone(),
        closes,
        rate: ratio(closes[1], closes[0]),
        expired_share: ratio(closes[3], closes[0]),
        keys,
        max_dev,
        perturbations,
        lookahead_ok: ok,
        guard_violations: r.guard_violations,
        cuts: r.lookahead.clone(),
        decoy: [real_dir, decoy, real_dir.zip(decoy).map(|(a, b)| a - b)],
    }
}

/// Score.
#[must_use]
pub fn score(alt: &str, data: &str, raws: &[M5Raw]) -> M5Summary {
    M5Summary {
        alt: alt.to_owned(),
        data: data.to_owned(),
        rules: raws.first().map(|r| r.rules.clone()).unwrap_or_default(),
        datasets: raws.iter().map(score_one).collect(),
    }
}

/// Markdown.
#[must_use]
pub fn markdown(s: &M5Summary) -> String {
    let mut o = String::new();
    let _ = writeln!(o, "# M5 — `{}` ({})\n", s.alt, s.data);
    let _ = writeln!(
        o,
        "Kurallar: tema `{}`, sonuç `{}`, sıralayıcı `{}`.\n",
        s.rules.first().map_or("", String::as_str),
        s.rules.get(1).map_or("", String::as_str),
        s.rules.get(2).map_or("", String::as_str)
    );
    let _ = writeln!(o, "## Özet\n");
    let _ = writeln!(o, "| Veri | Kapanan | Gerçekleşen | Geçersiz | Süresi dolan | İsabet | Süresi dolan payı | maks \\|p(25)−p(son)\\| | maks \\|p(50)−p(son)\\| | maks \\|p(100)−p(son)\\| | maks \\|p(200)−p(son)\\| | Yön (gerçek) | Yalancı eş | Fark | İleriye bakma |");
    let _ = writeln!(
        o,
        "|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|"
    );
    for d in &s.datasets {
        let _ = writeln!(
            o,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            d.dataset,
            d.closes[0],
            d.closes[1],
            d.closes[2],
            d.closes[3],
            f(d.rate, 3),
            f(d.expired_share, 3),
            f(d.max_dev[0], 3),
            f(d.max_dev[1], 3),
            f(d.max_dev[2], 3),
            f(d.max_dev[3], 3),
            f(d.decoy[0], 3),
            f(d.decoy[1], 3),
            f(d.decoy[2], 3),
            if d.lookahead_ok { "geçti" } else { "KALDI" }
        );
    }
    let _ = writeln!(o, "\n## 1. Kararlılık (anahtar başına)\n");
    let _ = writeln!(o, "| Veri | Anahtar | n | p(25) | p(50) | p(100) | p(200) | p(son) | ilk yarı | ikinci yarı | fark |");
    let _ = writeln!(o, "|---|---|---|---|---|---|---|---|---|---|---|");
    for d in &s.datasets {
        for k in &d.keys {
            let _ = writeln!(
                o,
                "| {} | `{}` | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
                d.dataset,
                k.key,
                k.n,
                f(k.p[0], 3),
                f(k.p[1], 3),
                f(k.p[2], 3),
                f(k.p[3], 3),
                f(k.p[4], 3),
                f(k.halves[0], 3),
                f(k.halves[1], 3),
                f(k.halves[2], 3)
            );
        }
    }
    let _ = writeln!(o, "\n## 2. Duyarlılık\n");
    let _ = writeln!(o, "| Veri | Çeşit | Genel oran değişimi | Ortak kapsül | Hüküm uyuşması | Kapsama | Anahtar başına en büyük \\|değişim\\| |");
    let _ = writeln!(o, "|---|---|---|---|---|---|---|");
    for d in &s.datasets {
        for p in &d.perturbations {
            let mx = p
                .keys
                .iter()
                .filter_map(|k| k.3.map(f64::abs))
                .reduce(f64::max);
            let _ = writeln!(
                o,
                "| {} | {} | {} | {} | {} | {} | {} |",
                d.dataset,
                p.name,
                f(p.overall_change, 3),
                p.common,
                f(p.agreement, 3),
                f(p.coverage, 3),
                f(mx, 3)
            );
        }
    }
    let _ = writeln!(o, "\n## 3. İleriye bakma\n");
    let _ = writeln!(o, "| Veri | Kesim t | Karşılaştırılan (t ≤ T) | Farklı olay | Farklı hüküm | Karnesi farklı anahtar | Gelecekte değişen | Koruma ihlali |");
    let _ = writeln!(o, "|---|---|---|---|---|---|---|---|");
    for d in &s.datasets {
        for c in &d.cuts {
            let _ = writeln!(
                o,
                "| {} | {} | {} | {} | {} | {} | {} | {} |",
                d.dataset,
                c.cut_t,
                c.compared,
                c.mismatched,
                c.verdict_mismatched,
                c.ledger_keys_differ,
                c.future_changed,
                d.guard_violations
            );
        }
    }
    o
}

/// `m5 run`.
///
/// # Errors
/// Unknown alternative, missing data, I/O.
pub fn cmd_run(alt: &str, data: DataKind, seeds: &[u64]) -> Result<String, String> {
    let fac: OutcomeFactory = registry::find(&registry::outcomes(), alt)
        .ok_or(format!("bilinmeyen M5 alternatifi: {alt}"))?;
    let ds = common::datasets(data, seeds);
    if ds.is_empty() {
        return Err("veri yok".to_owned());
    }
    let pert_names: Vec<String> = fac().1.into_iter().map(|(n, _)| n).collect();
    let make = move |_: &[InstrumentInput]| fac().0;
    let make_pert = move |name: &str| -> Box<dyn OutcomeRule> {
        fac()
            .1
            .into_iter()
            .find(|(n, _)| n == name)
            .map(|(_, r)| r)
            .expect("perturbation exists")
    };
    let raws = common::par_map(&ds, |d| run_dataset(d, alt, &make, &pert_names, &make_pert));
    let dir = raw_location("M5", alt, data)?;
    for r in &raws {
        write_file(
            &dir.join(common::raw_file_name(data, &r.dataset)),
            &to_json(r),
        )
        .map_err(|e| e.to_string())?;
    }
    cmd_score(alt, data)
}

/// `m5 score`.
///
/// # Errors
/// Missing raw, I/O.
pub fn cmd_score(alt: &str, data: DataKind) -> Result<String, String> {
    let dir = raw_location("M5", alt, data)?;
    let raws: Vec<M5Raw> = read_raws(&dir, data)?;
    let s = score(alt, data.name(), &raws);
    write_summary("M5", alt, data, &to_json(&s), &markdown(&s))
}
