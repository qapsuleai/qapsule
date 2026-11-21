//! M25 — outcome rule measures (SPEC §3), alternative paired with the reference `r3`.

use std::collections::BTreeMap;
use std::path::Path;

use qp_schema::capsule::Direction;
use serde::{Deserialize, Serialize};

use crate::boot::{paired, ratio_w, Boot, Est};
use crate::common::{day_str, par_map, percentile, to_json, write_file, xist, REPS, SEED};
use crate::data::{load_for, ranges, Period, Split, SymbolSeries, Window};
use crate::registry::{self, RuleEntry};
use crate::sim::{day_positions, facts_for, run_symbol, Ledger, StoryRec, EXP, INV, REAL};

/// Minimum ledger size for a ledger rate to count (Brier, calibration gap).
pub const MIN_LEDGER_N: u32 = 20;

/// Row names: directional (`up`/`down`), neutral.
pub const ROWS: [&str; 2] = ["directional", "neutral"];

/// Metric names, in output order.
pub const METRICS: [&str; 12] = [
    "p_real",
    "p_inv",
    "p_exp",
    "ecr_own",
    "ecr_ref",
    "rev",
    "brier",
    "brier_base",
    "calib_gap",
    "sens",
    "decoy",
    "real_minus_decoy",
];

/// One metric for the alternative, the reference and their difference.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Triple {
    /// Alternative.
    pub alt: Est,
    /// Reference `r3`.
    pub reference: Est,
    /// Alternative − reference (paired).
    pub diff: Est,
}

/// Counts of one rule in one row.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Counts {
    /// Counted (closed, born in the window).
    pub n: u64,
    /// Realized.
    pub realized: u64,
    /// Invalidated.
    pub invalidated: u64,
    /// Expired.
    pub expired: u64,
    /// Born in the window but still open at the end of the data (not counted).
    pub open_at_end: u64,
    /// Stories with a Brier term (ledger at birth `n ≥ 20`).
    pub brier_n: u64,
    /// Invalidated stories without levels (left out of `ECR_own`).
    pub ecr_skipped: u64,
    /// Stories in the calibration-gap set (eligible keys).
    pub gap_n: u64,
    /// Life in bars (birth → close): median, p90.
    pub life_median: Option<f64>,
    /// p90.
    pub life_p90: Option<f64>,
    /// `SENS`: stories whose unperturbed re-run did not reproduce the engine (self-check; 0
    /// for a pure rule).
    pub sens_replay_mismatch: u64,
}

/// One row (directional / neutral).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Row {
    /// Row name.
    pub row: String,
    /// Counts of the alternative.
    pub alt: Counts,
    /// Counts of the reference.
    pub reference: Counts,
    /// Metrics.
    pub metrics: BTreeMap<String, Triple>,
}

/// M25 result (aggregate numbers only: no symbol, no price).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct M25Result {
    /// Rule measured.
    pub rule: String,
    /// Reference.
    pub reference: String,
    /// Constants of the rule (simplicity).
    pub constants: u32,
    /// Period.
    pub period: String,
    /// Split.
    pub split: String,
    /// Counting window, first and last local day.
    pub window: [String; 2],
    /// Symbols with bars in the window.
    pub symbols: u64,
    /// Trading days with bars in the window.
    pub days: u64,
    /// Bars read (all, incl. warm-up).
    pub bars_read: u64,
    /// Bars in the window.
    pub bars_window: u64,
    /// Calibration-gap reference time: ledger as of this local day (exclusive).
    pub gap_ref_day: String,
    /// Brier base rate `q` per row (calibration range).
    pub base_rate: BTreeMap<String, Option<f64>>,
    /// Bootstrap repetitions and seed.
    pub bootstrap: (usize, u64),
    /// Rows.
    pub rows: Vec<Row>,
}

/// Per-symbol sums of one rule in one row.
#[derive(Debug, Clone, Default)]
struct Agg {
    n: f64,
    real: f64,
    inv: f64,
    exp: f64,
    ecr_own: (f64, f64),
    ecr_ref: (f64, f64),
    rev: (f64, f64),
    sens: (f64, f64),
    brier_sq: f64,
    brier_cnt: f64,
    brier_y: f64,
    decoy: (f64, f64),
    gap: BTreeMap<String, (f64, f64)>,
}

fn row_of(d: Direction) -> usize {
    usize::from(d == Direction::Neutral)
}

/// Everything one symbol contributes.
struct SymOut {
    alt: Vec<StoryRec>,
    reference: Vec<StoryRec>,
    decoy_alt: Vec<StoryRec>,
    decoy_ref: Vec<StoryRec>,
    days: Vec<i64>,
    bars_window: u64,
}

fn run_one(
    s: &SymbolSeries,
    period: Period,
    alt: &RuleEntry,
    reference: &RuleEntry,
    window: Window,
) -> SymOut {
    let cal = xist();
    let series = period.name();
    let det = registry::reference_detectors();
    let facts = facts_for(&s.symbol, series, &s.bars, det.make, false);
    let pos = day_positions(&cal, &s.bars);
    let a = run_symbol(&s.symbol, series, &s.bars, &facts, alt, window, &pos, true);
    let r = if alt.name == reference.name {
        a.clone()
    } else {
        run_symbol(
            &s.symbol, series, &s.bars, &facts, reference, window, &pos, true,
        )
    };
    drop(facts);
    let flipped = facts_for(&s.symbol, series, &s.bars, det.make, true);
    let da = run_symbol(
        &s.symbol, series, &s.bars, &flipped, alt, window, &pos, false,
    );
    let dr = if alt.name == reference.name {
        da.clone()
    } else {
        run_symbol(
            &s.symbol, series, &s.bars, &flipped, reference, window, &pos, false,
        )
    };
    let mut days: Vec<i64> = s
        .bars
        .iter()
        .filter(|b| window.contains(b.t))
        .map(|b| cal.local_day(b.t))
        .collect();
    days.dedup();
    let bars_window = s.bars.iter().filter(|b| window.contains(b.t)).count() as u64;
    SymOut {
        alt: a,
        reference: r,
        decoy_alt: da,
        decoy_ref: dr,
        days,
        bars_window,
    }
}

/// The gap reference: test → the cut; calibration → the start of the middle trading day
/// of the window (see README "Yorumlar").
fn gap_ref(split: Split, window: Window, days: &[i64]) -> (i64, Option<i64>) {
    let cal = xist();
    match split {
        Split::Test => (window.from, Some(cal.local_day(window.from))),
        Split::Calib => {
            if days.is_empty() {
                return (window.from, None);
            }
            let mid = days[days.len() / 2];
            (cal.day_span(mid).map_or(window.from, |s| s.open), Some(mid))
        }
    }
}

/// Base rate `q` per row: realized share of the calibration-range stories (born in the
/// calibration window and closed before the cut).
fn base_rates(all: &[&[StoryRec]], period: Period) -> [Option<f64>; 2] {
    let r = ranges(period);
    let cw = Window {
        from: r.calib_start,
        to: r.cut,
    };
    let mut n = [0f64; 2];
    let mut k = [0f64; 2];
    for recs in all {
        for s in recs.iter() {
            if s.closed() && cw.contains(s.born_t) && s.close_t < r.cut {
                let i = row_of(s.direction);
                n[i] += 1.0;
                k[i] += f64::from(u8::from(s.stage == REAL));
            }
        }
    }
    [
        (n[0] > 0.0).then(|| k[0] / n[0]),
        (n[1] > 0.0).then(|| k[1] / n[1]),
    ]
}

struct RuleSide {
    aggs: Vec<[Agg; 2]>,
    counts: [Counts; 2],
    gap_l: BTreeMap<String, f64>,
    q: [Option<f64>; 2],
}

#[allow(clippy::too_many_lines)]
fn side(
    recs: &[&[StoryRec]],
    decoys: &[&[StoryRec]],
    window: Window,
    gap_t: i64,
    period: Period,
) -> RuleSide {
    let ledger = Ledger::build(recs.iter().flat_map(|r| r.iter()));
    let q = base_rates(recs, period);
    let gap_l: BTreeMap<String, f64> = ledger
        .keys()
        .filter_map(|k| {
            let (n, r) = ledger.stat(k, gap_t);
            (n >= MIN_LEDGER_N).then(|| (k.clone(), f64::from(r) / f64::from(n)))
        })
        .collect();
    let mut counts: [Counts; 2] = Default::default();
    let mut lives: [Vec<f64>; 2] = Default::default();
    let mut aggs = Vec::with_capacity(recs.len());
    for (si, rs) in recs.iter().enumerate() {
        let mut a: [Agg; 2] = Default::default();
        for s in rs.iter() {
            if !window.contains(s.born_t) {
                continue;
            }
            let i = row_of(s.direction);
            if !s.closed() {
                counts[i].open_at_end += 1;
                continue;
            }
            let g = &mut a[i];
            let c = &mut counts[i];
            let y = f64::from(u8::from(s.stage == REAL));
            g.n += 1.0;
            c.n += 1;
            match s.stage {
                REAL => {
                    g.real += 1.0;
                    c.realized += 1;
                }
                INV => {
                    g.inv += 1.0;
                    c.invalidated += 1;
                }
                EXP => {
                    g.exp += 1.0;
                    c.expired += 1;
                }
                _ => {}
            }
            #[allow(clippy::cast_precision_loss)]
            lives[i].push((s.close_bar - s.born_bar) as f64);
            if s.stage == INV {
                match s.ecr_own {
                    Some(x) => {
                        g.ecr_own.0 += f64::from(u8::from(x));
                        g.ecr_own.1 += 1.0;
                    }
                    None => c.ecr_skipped += 1,
                }
                if let Some(x) = s.ecr_ref {
                    g.ecr_ref.0 += f64::from(u8::from(x));
                    g.ecr_ref.1 += 1.0;
                }
            }
            if let Some(x) = s.rev {
                g.rev.0 += f64::from(u8::from(x));
                g.rev.1 += 1.0;
            }
            if let Some(x) = s.sens {
                g.sens.0 += f64::from(u8::from(x));
                g.sens.1 += 1.0;
            }
            if s.sens_replay_ok == Some(false) {
                c.sens_replay_mismatch += 1;
            }
            let (ln, lr) = ledger.stat(&s.key, s.born_t);
            if ln >= MIN_LEDGER_N {
                let p = f64::from(lr) / f64::from(ln);
                g.brier_sq += (p - y) * (p - y);
                g.brier_cnt += 1.0;
                g.brier_y += y;
                c.brier_n += 1;
            }
            if s.born_t >= gap_t && gap_l.contains_key(&s.key) {
                let e = g.gap.entry(s.key.clone()).or_insert((0.0, 0.0));
                e.0 += 1.0;
                e.1 += y;
                c.gap_n += 1;
            }
        }
        for s in decoys[si].iter() {
            if s.closed() && window.contains(s.born_t) && s.direction != Direction::Neutral {
                a[0].decoy.0 += f64::from(u8::from(s.stage == REAL));
                a[0].decoy.1 += 1.0;
            }
        }
        aggs.push(a);
    }
    for i in 0..2 {
        counts[i].life_median = percentile(&lives[i], 0.5);
        counts[i].life_p90 = percentile(&lives[i], 0.9);
    }
    RuleSide {
        aggs,
        counts,
        gap_l,
        q,
    }
}

/// Value of metric `m` for row `row` of one side under weights `w`.
fn metric(side: &RuleSide, row: usize, m: &str, w: &[u32]) -> Option<f64> {
    let col =
        |f: &dyn Fn(&Agg) -> f64| -> Vec<f64> { side.aggs.iter().map(|a| f(&a[row])).collect() };
    let rw =
        |num: &dyn Fn(&Agg) -> f64, den: &dyn Fn(&Agg) -> f64| ratio_w(w, &col(num), &col(den));
    match m {
        "p_real" => rw(&|a| a.real, &|a| a.n),
        "p_inv" => rw(&|a| a.inv, &|a| a.n),
        "p_exp" => rw(&|a| a.exp, &|a| a.n),
        "ecr_own" => rw(&|a| a.ecr_own.0, &|a| a.ecr_own.1),
        "ecr_ref" => rw(&|a| a.ecr_ref.0, &|a| a.ecr_ref.1),
        "rev" => rw(&|a| a.rev.0, &|a| a.rev.1),
        "sens" => rw(&|a| a.sens.0, &|a| a.sens.1),
        "brier" => rw(&|a| a.brier_sq, &|a| a.brier_cnt),
        "brier_base" => {
            let q = side.q[row]?;
            let num =
                |a: &Agg| a.brier_y * (1.0 - q) * (1.0 - q) + (a.brier_cnt - a.brier_y) * q * q;
            rw(&num, &|a| a.brier_cnt)
        }
        "calib_gap" => {
            let mut num = 0.0;
            let mut den = 0.0;
            for (k, l) in &side.gap_l {
                let mut n = 0.0;
                let mut r = 0.0;
                for (i, a) in side.aggs.iter().enumerate() {
                    if let Some((kn, kr)) = a[row].gap.get(k) {
                        n += f64::from(w[i]) * kn;
                        r += f64::from(w[i]) * kr;
                    }
                }
                if n > 0.0 {
                    num += n * (r / n - l).abs();
                    den += n;
                }
            }
            (den > 0.0).then(|| num / den)
        }
        "decoy" => (row == 0)
            .then(|| rw(&|a| a.decoy.0, &|a| a.decoy.1))
            .flatten(),
        "real_minus_decoy" => {
            if row != 0 {
                return None;
            }
            let r = rw(&|a| a.real, &|a| a.n)?;
            let d = rw(&|a| a.decoy.0, &|a| a.decoy.1)?;
            Some(r - d)
        }
        _ => None,
    }
}

/// Run M25 for `rule` on `data` and return the result.
///
/// # Errors
/// Unknown rule, data errors.
pub fn run(data: &Path, rule: &str, period: Period, split: Split) -> Result<M25Result, String> {
    let alt = registry::rule(rule).ok_or(format!("bilinmeyen kural: {rule}"))?;
    let reference = registry::reference_rule();
    let series = load_for(data, period, split)?;
    if series.is_empty() {
        return Err("veri yok".to_owned());
    }
    Ok(run_series(&series, &alt, &reference, period, split))
}

/// Run M25 on loaded series (tests call this directly).
#[must_use]
pub fn run_series(
    series: &[SymbolSeries],
    alt: &RuleEntry,
    reference: &RuleEntry,
    period: Period,
    split: Split,
) -> M25Result {
    let window = Window::of(period, split);
    let outs = par_map(series, |s| run_one(s, period, alt, reference, window));
    let mut all_days: Vec<i64> = outs.iter().flat_map(|o| o.days.iter().copied()).collect();
    all_days.sort_unstable();
    all_days.dedup();
    let (gap_t, gap_day) = gap_ref(split, window, &all_days);
    let ra: Vec<&[StoryRec]> = outs.iter().map(|o| o.alt.as_slice()).collect();
    let rr: Vec<&[StoryRec]> = outs.iter().map(|o| o.reference.as_slice()).collect();
    let da: Vec<&[StoryRec]> = outs.iter().map(|o| o.decoy_alt.as_slice()).collect();
    let dr: Vec<&[StoryRec]> = outs.iter().map(|o| o.decoy_ref.as_slice()).collect();
    let sa = side(&ra, &da, window, gap_t, period);
    let sr = side(&rr, &dr, window, gap_t, period);
    let boot = Boot::new(series.len());
    let mut rows = Vec::new();
    for (ri, name) in ROWS.iter().enumerate() {
        let mut metrics = BTreeMap::new();
        for m in METRICS {
            let (a, r, d) = paired(&boot, |w| metric(&sa, ri, m, w), |w| metric(&sr, ri, m, w));
            metrics.insert(
                m.to_owned(),
                Triple {
                    alt: a,
                    reference: r,
                    diff: d,
                },
            );
        }
        rows.push(Row {
            row: (*name).to_owned(),
            alt: sa.counts[ri].clone(),
            reference: sr.counts[ri].clone(),
            metrics,
        });
    }
    let cal = xist();
    M25Result {
        rule: alt.name.to_owned(),
        reference: reference.name.to_owned(),
        constants: alt.constants,
        period: period.name().to_owned(),
        split: split.name().to_owned(),
        window: [
            day_str(cal.local_day(window.from)),
            day_str(cal.local_day(window.to - 1)),
        ],
        symbols: outs.iter().filter(|o| o.bars_window > 0).count() as u64,
        days: all_days.len() as u64,
        bars_read: series.iter().map(|s| s.bars.len() as u64).sum(),
        bars_window: outs.iter().map(|o| o.bars_window).sum(),
        gap_ref_day: gap_day.map(day_str).unwrap_or_default(),
        base_rate: ROWS
            .iter()
            .enumerate()
            .map(|(i, r)| ((*r).to_owned(), sa.q[i]))
            .collect(),
        bootstrap: (REPS, SEED),
        rows,
    }
}

/// File name of a result.
#[must_use]
pub fn file_name(rule: &str, period: &str, split: &str) -> String {
    format!("{rule}.{period}.{split}.json")
}

/// `m25` command: run and write `<out>/M25/<rule>.<period>.<split>.json`.
///
/// # Errors
/// As [`run`]; I/O.
pub fn cmd(
    data: &Path,
    rule: &str,
    period: Period,
    split: Split,
    out: &Path,
) -> Result<String, String> {
    let r = run(data, rule, period, split)?;
    let path = out
        .join("M25")
        .join(file_name(&r.rule, &r.period, &r.split));
    write_file(&path, &to_json(&r)).map_err(|e| e.to_string())?;
    Ok(format!(
        "{}: {} sembol, {} gün, {} bar (pencere), yönlü n = {} / yönsüz n = {}",
        path.display(),
        r.symbols,
        r.days,
        r.bars_window,
        r.rows[0].alt.n,
        r.rows[1].alt.n
    ))
}
