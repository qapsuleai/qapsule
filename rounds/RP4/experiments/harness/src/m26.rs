//! M26 — detector thresholds intraday (SPEC §5): flow load, hit rate against the K53
//! coin-flip threshold, opening / closing births, paired with the reference set `r3`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::boot::{paired, ratio_w, Boot};
use crate::common::{day_str, par_map, to_json, write_file, xist, REPS, SEED};
use crate::data::{load_for, Period, Split, SymbolSeries, Window};
use crate::m25::Triple;
use crate::registry::{self, DetectorEntry, RuleEntry};
use crate::sim::{day_positions, facts_for, run_symbol, StoryRec, POS_CLOSE, POS_OPEN, REAL};

/// K53 coin-flip threshold.
pub const COIN_FLIP: f64 = 0.55;

/// Pooled row name.
pub const ALL: &str = "(hepsi)";

/// Metric names.
pub const METRICS: [&str; 10] = [
    "load",
    "hit",
    "share_open",
    "share_close",
    "hit_open",
    "hit_close",
    "hit_rest",
    "d_open",
    "d_close",
    "ft",
];

// Per-symbol sums, per detector.
const BIRTHS: usize = 0;
const N: usize = 1;
const R: usize = 2;
const B_OPEN: usize = 3;
const N_OPEN: usize = 4;
const R_OPEN: usize = 5;
const B_CLOSE: usize = 6;
const N_CLOSE: usize = 7;
const R_CLOSE: usize = 8;
const N_REST: usize = 9;
const R_REST: usize = 10;
const N_OC: usize = 11;
const R_OC: usize = 12;
const SLOTS: usize = 13;

type Sums = BTreeMap<String, [f64; SLOTS]>;

/// Counts of one detector.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DetCounts {
    /// Births in the window.
    pub births: u64,
    /// Closed (counted).
    pub n: u64,
    /// Opening births (first two bars of the day).
    pub births_open: u64,
    /// Closing births (last bar of the day).
    pub births_close: u64,
}

/// One detector row.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DetRow {
    /// Detector id (or [`ALL`]).
    pub detector: String,
    /// Alternative counts.
    pub alt: DetCounts,
    /// Reference counts.
    pub reference: DetCounts,
    /// Metrics (alt, reference, alt − reference).
    pub metrics: BTreeMap<String, Triple>,
    /// Load ratio alternative / reference (point).
    pub load_ratio: Option<f64>,
    /// Alternative hit interval vs 0.55: `above`, `contains`, `below`.
    pub vs_coin_flip: String,
    /// "Gün içinde anlamsız" (SPEC §5) for the alternative.
    pub meaningless: bool,
}

/// M26 result (aggregate only).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct M26Result {
    /// Detector set measured.
    pub detectors: String,
    /// Reference set.
    pub reference: String,
    /// Outcome rule.
    pub rule: String,
    /// Period.
    pub period: String,
    /// Split.
    pub split: String,
    /// Window (first, last local day).
    pub window: [String; 2],
    /// Symbols with bars in the window.
    pub symbols: u64,
    /// Trading days.
    pub days: u64,
    /// Symbol·days with bars in the window.
    pub symbol_days: u64,
    /// Bootstrap.
    pub bootstrap: (usize, u64),
    /// Rows: each detector, then [`ALL`].
    pub rows: Vec<DetRow>,
}

struct SymOut {
    alt: Vec<StoryRec>,
    reference: Vec<StoryRec>,
    symdays: u64,
    days: Vec<i64>,
}

fn run_one(
    s: &SymbolSeries,
    period: Period,
    alt: &DetectorEntry,
    reference: &DetectorEntry,
    rule: &RuleEntry,
    window: Window,
) -> SymOut {
    let cal = xist();
    let series = period.name();
    let pos = if period.intraday() {
        day_positions(&cal, &s.bars)
    } else {
        vec![0; s.bars.len()]
    };
    let fa = facts_for(&s.symbol, series, &s.bars, alt.make, false);
    let a = run_symbol(&s.symbol, series, &s.bars, &fa, rule, window, &pos, false);
    drop(fa);
    let r = if alt.name == reference.name {
        a.clone()
    } else {
        let fr = facts_for(&s.symbol, series, &s.bars, reference.make, false);
        run_symbol(&s.symbol, series, &s.bars, &fr, rule, window, &pos, false)
    };
    let mut days: Vec<i64> = s
        .bars
        .iter()
        .filter(|b| window.contains(b.t))
        .map(|b| cal.local_day(b.t))
        .collect();
    days.dedup();
    SymOut {
        alt: a,
        reference: r,
        symdays: days.len() as u64,
        days,
    }
}

fn sums(recs: &[StoryRec], window: Window) -> Sums {
    let mut m: Sums = BTreeMap::new();
    for s in recs.iter().filter(|s| window.contains(s.born_t)) {
        for key in [s.detector.as_str(), ALL] {
            let e = m.entry(key.to_owned()).or_insert([0.0; SLOTS]);
            let open = s.day_pos & POS_OPEN != 0;
            let close = s.day_pos & POS_CLOSE != 0;
            e[BIRTHS] += 1.0;
            if open {
                e[B_OPEN] += 1.0;
            }
            if close {
                e[B_CLOSE] += 1.0;
            }
            if !s.closed() {
                continue;
            }
            let y = f64::from(u8::from(s.stage == REAL));
            e[N] += 1.0;
            e[R] += y;
            if open {
                e[N_OPEN] += 1.0;
                e[R_OPEN] += y;
            }
            if close {
                e[N_CLOSE] += 1.0;
                e[R_CLOSE] += y;
            }
            if open || close {
                e[N_OC] += 1.0;
                e[R_OC] += y;
            } else {
                e[N_REST] += 1.0;
                e[R_REST] += y;
            }
        }
    }
    m
}

fn metric(
    sums: &[Sums],
    symdays: &[f64],
    det: &str,
    m: &str,
    intraday: bool,
    w: &[u32],
) -> Option<f64> {
    let col = |k: usize| -> Vec<f64> {
        sums.iter()
            .map(|s| s.get(det).map_or(0.0, |x| x[k]))
            .collect()
    };
    let r = |a: usize, b: usize| ratio_w(w, &col(a), &col(b));
    if !intraday && m != "load" && m != "hit" {
        return None;
    }
    match m {
        "load" => ratio_w(w, &col(BIRTHS), symdays),
        "hit" => r(R, N),
        "share_open" => r(B_OPEN, BIRTHS),
        "share_close" => r(B_CLOSE, BIRTHS),
        "hit_open" => r(R_OPEN, N_OPEN),
        "hit_close" => r(R_CLOSE, N_CLOSE),
        "hit_rest" => r(R_REST, N_REST),
        "d_open" => Some(r(R_OPEN, N_OPEN)? - r(R_REST, N_REST)?),
        "d_close" => Some(r(R_CLOSE, N_CLOSE)? - r(R_REST, N_REST)?),
        "ft" => Some(r(R_REST, N_REST)? - r(R_OC, N_OC)?),
        _ => None,
    }
}

fn counts(sums: &[Sums], det: &str) -> DetCounts {
    let tot = |k: usize| -> u64 {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let v = sums
            .iter()
            .map(|s| s.get(det).map_or(0.0, |x| x[k]))
            .sum::<f64>() as u64;
        v
    };
    DetCounts {
        births: tot(BIRTHS),
        n: tot(N),
        births_open: tot(B_OPEN),
        births_close: tot(B_CLOSE),
    }
}

/// Run M26.
///
/// # Errors
/// Unknown names, data errors.
pub fn run(
    data: &Path,
    detectors: &str,
    rule: &str,
    period: Period,
    split: Split,
) -> Result<M26Result, String> {
    let alt = registry::detector_set(detectors)
        .ok_or(format!("bilinmeyen dedektör seti: {detectors}"))?;
    let rule = registry::rule(rule).ok_or(format!("bilinmeyen kural: {rule}"))?;
    let series = load_for(data, period, split)?;
    if series.is_empty() {
        return Err("veri yok".to_owned());
    }
    Ok(run_series(
        &series,
        &alt,
        &registry::reference_detectors(),
        &rule,
        period,
        split,
    ))
}

/// Run M26 on loaded series.
#[must_use]
pub fn run_series(
    series: &[SymbolSeries],
    alt: &DetectorEntry,
    reference: &DetectorEntry,
    rule: &RuleEntry,
    period: Period,
    split: Split,
) -> M26Result {
    let window = Window::of(period, split);
    let outs = par_map(series, |s| run_one(s, period, alt, reference, rule, window));
    let sa: Vec<Sums> = outs.iter().map(|o| sums(&o.alt, window)).collect();
    let sr: Vec<Sums> = outs.iter().map(|o| sums(&o.reference, window)).collect();
    #[allow(clippy::cast_precision_loss)]
    let symdays: Vec<f64> = outs.iter().map(|o| o.symdays as f64).collect();
    let mut dets: BTreeSet<String> = BTreeSet::new();
    for s in sa.iter().chain(sr.iter()) {
        dets.extend(s.keys().filter(|k| *k != ALL).cloned());
    }
    let mut order: Vec<String> = dets.into_iter().collect();
    order.push(ALL.to_owned());
    let boot = Boot::new(series.len());
    let intraday = period.intraday();
    let mut rows = Vec::new();
    for d in &order {
        let mut metrics = BTreeMap::new();
        for m in METRICS {
            let (a, r, df) = paired(
                &boot,
                |w| metric(&sa, &symdays, d, m, intraday, w),
                |w| metric(&sr, &symdays, d, m, intraday, w),
            );
            metrics.insert(
                m.to_owned(),
                Triple {
                    alt: a,
                    reference: r,
                    diff: df,
                },
            );
        }
        let hit = metrics["hit"].alt;
        let vs = match (hit.lo, hit.hi) {
            (Some(lo), _) if lo > COIN_FLIP => "above",
            (_, Some(hi)) if hi < COIN_FLIP => "below",
            (Some(_), Some(_)) => "contains",
            _ => "—",
        };
        let meaningless = match (hit.pt, hit.lo, hit.hi) {
            (Some(pt), Some(lo), Some(hi)) => {
                hi < 0.5 || (pt < COIN_FLIP && lo <= 0.5 && 0.5 <= hi)
            }
            _ => false,
        };
        let load = &metrics["load"];
        let load_ratio = load
            .alt
            .pt
            .zip(load.reference.pt)
            .and_then(|(a, b)| (b > 0.0).then(|| a / b));
        rows.push(DetRow {
            detector: d.clone(),
            alt: counts(&sa, d),
            reference: counts(&sr, d),
            metrics,
            load_ratio,
            vs_coin_flip: vs.to_owned(),
            meaningless,
        });
    }
    let mut all_days: Vec<i64> = outs.iter().flat_map(|o| o.days.iter().copied()).collect();
    all_days.sort_unstable();
    all_days.dedup();
    let cal = xist();
    M26Result {
        detectors: alt.name.to_owned(),
        reference: reference.name.to_owned(),
        rule: rule.name.to_owned(),
        period: period.name().to_owned(),
        split: split.name().to_owned(),
        window: [
            day_str(cal.local_day(window.from)),
            day_str(cal.local_day(window.to - 1)),
        ],
        symbols: outs.iter().filter(|o| o.symdays > 0).count() as u64,
        days: all_days.len() as u64,
        symbol_days: outs.iter().map(|o| o.symdays).sum(),
        bootstrap: (REPS, SEED),
        rows,
    }
}

/// File name of a result.
#[must_use]
pub fn file_name(detectors: &str, rule: &str, period: &str, split: &str) -> String {
    format!("{detectors}.{rule}.{period}.{split}.json")
}

/// `m26` command: run and write `<out>/M26/<set>.<rule>.<period>.<split>.json`.
///
/// # Errors
/// As [`run`]; I/O.
pub fn cmd(
    data: &Path,
    detectors: &str,
    rule: &str,
    period: Period,
    split: Split,
    out: &Path,
) -> Result<String, String> {
    let r = run(data, detectors, rule, period, split)?;
    let path = out
        .join("M26")
        .join(file_name(&r.detectors, &r.rule, &r.period, &r.split));
    write_file(&path, &to_json(&r)).map_err(|e| e.to_string())?;
    let all = r.rows.last().map(|x| x.alt.births).unwrap_or_default();
    Ok(format!(
        "{}: {} sembol, {} gün, {} sembol·gün, {} doğum",
        path.display(),
        r.symbols,
        r.days,
        r.symbol_days,
        all
    ))
}
