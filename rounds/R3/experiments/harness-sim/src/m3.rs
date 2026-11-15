//! M3 — importance ranking (SPEC M3).
//!
//! Run: the measured ranker with the reference theme and outcome rules over every
//! dataset; raw = every story event as `[story seq, instrument, trading day, stage, tier]`
//! plus the look-ahead check. Score: daily counts per sensitivity level, same-instrument
//! repeats, realized share per tier, look-ahead result.
//!
//! Definitions (written in the README too):
//! - **Level** `important` = tier 1, `balanced` = tiers 1–2, `all` = every tier.
//! - **Daily capsule count** = stories **born** on that trading day whose tier **at
//!   birth** passes the level (primary). Secondary: every transition shown that day
//!   (any stage, tier of that event's snapshot passes).
//! - Days counted: from `first_day` (bar 50 = detector warm-up) to the end.
//! - **Regime ratio**: births per instrument-day, mean per regime of that instrument on
//!   that day; max / min over regimes (synthetic only).
//! - **Seed spread**: max − min of the per-dataset mean daily count (and / their mean).
//! - **Same instrument within 5 days**: share of passing births that follow a passing
//!   birth of the same instrument at most 5 trading days earlier (same day included);
//!   **longest run**: most consecutive trading days with ≥ 1 passing birth on one
//!   instrument.
//! - **Realized share per tier**: tier at birth; closed = realized + invalidated + expired
//!   (retracted excluded); share = realized / closed; base rate over all tiers.
//! - **Look-ahead**: at 25/50/75 % of the bars, every bar with `t >= T` (and its flow) is
//!   replaced by another seeded walk; every event with `t < T` must be byte-equal
//!   (JSON of the `StoryEvent`, which includes importance and tier).

use std::collections::BTreeMap;
use std::fmt::Write as _;

use qp_capsule::{InstrumentInput, Ranker};
use serde::{Deserialize, Serialize};

use crate::common::{
    self, cut_times, f, perturb_future, ratio, stage_code, to_json, write_file, DataKind, Dataset,
    Dist, EXPIRED, INVALIDATED, REALIZED,
};
use crate::registry;
use crate::sim;

/// Look-ahead check at one cut.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LookCut {
    /// Cut time.
    pub cut_t: i64,
    /// Events with `t < cut_t` in the base run.
    pub compared: u64,
    /// Of those, positions whose event JSON differs (plus any length difference).
    pub mismatched: u64,
    /// Of those, positions whose tier differs.
    pub tier_mismatched: u64,
    /// Events with `t >= cut_t` that differ (shows the future really changed).
    pub future_changed: u64,
}

/// Raw record of one dataset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct M3Raw {
    /// Dataset name.
    pub dataset: String,
    /// Alternative name.
    pub alt: String,
    /// Rule names: theme, outcome, ranker.
    pub rules: Vec<String>,
    /// Trading days.
    pub n_days: u32,
    /// First counted day.
    pub first_day: u32,
    /// Instruments.
    pub instruments: Vec<String>,
    /// Regime code per instrument per day (synthetic).
    pub regime: Option<Vec<Vec<u8>>>,
    /// `[story seq, instrument, day, stage, tier]` per event, in order.
    pub events: Vec<[u32; 5]>,
    /// Look-ahead checks.
    pub lookahead: Vec<LookCut>,
}

/// A ranker factory that may see the data (only test cheaters use it).
pub type DataRanker<'a> = &'a (dyn Fn(&[InstrumentInput]) -> Box<dyn Ranker> + Sync);

fn events_json(e: &qp_capsule::engine::Engine) -> Vec<(i64, String, u8)> {
    e.events()
        .iter()
        .map(|ev| {
            (
                ev.transition.t,
                serde_json::to_string(ev).expect("event json"),
                ev.story.tier,
            )
        })
        .collect()
}

/// Run one dataset.
#[must_use]
pub fn run_dataset(ds: &Dataset, alt: &str, make: DataRanker<'_>) -> M3Raw {
    let (engine, _facts) = sim::run(
        &ds.inputs,
        registry::reference_theme(),
        registry::reference_outcome(),
        make(&ds.inputs),
    );
    let (tn, on, rn) = engine.rule_names();
    let inst_idx: BTreeMap<&str, u32> = ds
        .inputs
        .iter()
        .enumerate()
        .map(|(i, x)| (x.symbol.as_str(), u32::try_from(i).unwrap_or(0)))
        .collect();
    let mut seq: BTreeMap<String, u32> = BTreeMap::new();
    let mut events = Vec::new();
    for ev in engine.events() {
        let n = u32::try_from(seq.len()).unwrap_or(0);
        let s = *seq.entry(ev.capsule.clone()).or_insert(n);
        let i = inst_idx
            .get(ev.story.instrument.as_str())
            .copied()
            .unwrap_or(0);
        let bar = usize::try_from(ev.transition.bar).unwrap_or(0);
        let day = ds.day_of_bar[i as usize].get(bar).copied().unwrap_or(0);
        events.push([
            s,
            i,
            day,
            u32::from(stage_code(ev.transition.stage)),
            u32::from(ev.story.tier),
        ]);
    }

    let base = events_json(&engine);
    let mut lookahead = Vec::new();
    for (ci, cut) in cut_times(&ds.inputs).into_iter().enumerate() {
        let pert = perturb_future(&ds.inputs, cut, 0xC0DE + ci as u64);
        let (e2, _) = sim::run(
            &pert,
            registry::reference_theme(),
            registry::reference_outcome(),
            make(&pert),
        );
        let other = events_json(&e2);
        let past_a: Vec<_> = base.iter().filter(|x| x.0 < cut).collect();
        let past_b: Vec<_> = other.iter().filter(|x| x.0 < cut).collect();
        let mut mism = past_a.len().abs_diff(past_b.len()) as u64;
        let mut tier = 0;
        for (a, b) in past_a.iter().zip(past_b.iter()) {
            if a.1 != b.1 {
                mism += 1;
            }
            if a.2 != b.2 {
                tier += 1;
            }
        }
        let fut_a: Vec<_> = base.iter().filter(|x| x.0 >= cut).collect();
        let fut_b: Vec<_> = other.iter().filter(|x| x.0 >= cut).collect();
        let mut fut = fut_a.len().abs_diff(fut_b.len()) as u64;
        for (a, b) in fut_a.iter().zip(fut_b.iter()) {
            if a.1 != b.1 {
                fut += 1;
            }
        }
        lookahead.push(LookCut {
            cut_t: cut,
            compared: past_a.len() as u64,
            mismatched: mism,
            tier_mismatched: tier,
            future_changed: fut,
        });
    }

    M3Raw {
        dataset: ds.name.clone(),
        alt: alt.to_owned(),
        rules: vec![tn.to_owned(), on.to_owned(), rn.to_owned()],
        n_days: ds.n_days,
        first_day: ds.first_day,
        instruments: ds.inputs.iter().map(|x| x.symbol.clone()).collect(),
        regime: ds.regime.clone(),
        events,
        lookahead,
    }
}

// ---------------------------------------------------------------------------
// Score
// ---------------------------------------------------------------------------

/// Per level metrics of one dataset (or pooled).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LevelStats {
    /// Level name.
    pub level: String,
    /// Daily births passing the level.
    pub births: Dist,
    /// Daily transitions shown.
    pub shown: Dist,
    /// Mean births per instrument-day by regime (`rising`, `falling`, `sideways`, `volatile`).
    pub regime_means: Vec<Option<f64>>,
    /// max / min of the regime means.
    pub regime_ratio: Option<f64>,
    /// Share of passing births within 5 trading days of a previous one, same instrument.
    pub repeat5: Option<f64>,
    /// Longest run of consecutive days with a passing birth on one instrument.
    pub longest_run: u32,
}

/// Realized share per tier.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TierOutcome {
    /// `[closed, realized]` for tiers 1, 2, 3.
    pub by_tier: Vec<[u64; 2]>,
    /// Shares per tier.
    pub share: Vec<Option<f64>>,
    /// Base rate (all tiers).
    pub base: Option<f64>,
    /// Tier 1 − tier 3.
    pub diff_1_3: Option<f64>,
}

/// Dataset-level summary.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DatasetScore {
    /// Name.
    pub dataset: String,
    /// Stories born.
    pub stories: u64,
    /// Levels.
    pub levels: Vec<LevelStats>,
    /// Outcome by tier.
    pub outcome: TierOutcome,
    /// Look-ahead: cuts, compared events, mismatched, tier mismatched, future changed.
    pub lookahead: Vec<LookCut>,
}

/// Summary of one alternative on one data kind.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct M3Summary {
    /// Alternative.
    pub alt: String,
    /// Data kind.
    pub data: String,
    /// Rule names (theme, outcome, ranker).
    pub rules: Vec<String>,
    /// Per dataset.
    pub datasets: Vec<DatasetScore>,
    /// Pooled per level (daily counts concatenated over datasets).
    pub pooled: Vec<LevelStats>,
    /// Pooled outcome by tier.
    pub pooled_outcome: TierOutcome,
    /// Seed spread per level: `[max − min of the dataset means, that / mean of means]`.
    pub seed_spread: Vec<[Option<f64>; 2]>,
    /// Look-ahead passed everywhere.
    pub lookahead_ok: bool,
}

/// Pooled daily counts, shown counts, regime sums/counts, unused, unused, longest run.
type Pool = (Vec<f64>, Vec<f64>, [f64; 4], [u64; 4], u64, u64, u32);

const LEVELS: [&str; 3] = ["important", "balanced", "all"];

struct Births {
    /// (seq, inst, day, tier at birth)
    list: Vec<(u32, u32, u32, u32)>,
    /// final stage per story seq (0 if open)
    final_stage: BTreeMap<u32, u32>,
}

fn births(r: &M3Raw) -> Births {
    let mut list = Vec::new();
    let mut final_stage = BTreeMap::new();
    for e in &r.events {
        if e[3] == 1 {
            list.push((e[0], e[1], e[2], e[4]));
        }
        if e[3] >= u32::from(REALIZED) && e[3] <= u32::from(EXPIRED) {
            final_stage.insert(e[0], e[3]);
        }
    }
    Births { list, final_stage }
}

struct LevelRaw {
    births: Vec<f64>,
    shown: Vec<f64>,
    regime_sum: [f64; 4],
    regime_cnt: [u64; 4],
}

fn level_raw(r: &M3Raw, b: &Births, lvl: u32) -> (LevelRaw, Option<f64>, u32) {
    let days = r.n_days as usize;
    let first = r.first_day as usize;
    let mut by_day = vec![0u64; days];
    let mut shown = vec![0u64; days];
    let n_inst = r.instruments.len();
    let mut inst_day: Vec<BTreeMap<u32, u64>> = vec![BTreeMap::new(); n_inst];
    for &(_, i, d, tier) in &b.list {
        if tier <= lvl && (d as usize) < days {
            by_day[d as usize] += 1;
            *inst_day[i as usize].entry(d).or_insert(0) += 1;
        }
    }
    for e in &r.events {
        if e[4] <= lvl && (e[2] as usize) < days {
            shown[e[2] as usize] += 1;
        }
    }
    #[allow(clippy::cast_precision_loss)]
    let births: Vec<f64> = by_day[first.min(days)..]
        .iter()
        .map(|&x| x as f64)
        .collect();
    #[allow(clippy::cast_precision_loss)]
    let shown_v: Vec<f64> = shown[first.min(days)..].iter().map(|&x| x as f64).collect();
    let mut regime_sum = [0.0; 4];
    let mut regime_cnt = [0u64; 4];
    if let Some(reg) = &r.regime {
        for (i, row) in reg.iter().enumerate() {
            for (d, &g) in row.iter().enumerate().skip(first) {
                let c = inst_day[i]
                    .get(&u32::try_from(d).unwrap_or(0))
                    .copied()
                    .unwrap_or(0);
                #[allow(clippy::cast_precision_loss)]
                {
                    regime_sum[g as usize] += c as f64;
                }
                regime_cnt[g as usize] += 1;
            }
        }
    }
    // Repeats and runs.
    let mut hits = 0u64;
    let mut total = 0u64;
    let mut longest = 0u32;
    for row in &inst_day {
        let mut prev: Option<u32> = None;
        let mut run = 0u32;
        let mut last_day: Option<u32> = None;
        for (&d, &c) in row {
            if (d as usize) < first {
                continue;
            }
            for k in 0..c {
                total += 1;
                let close = if k > 0 {
                    true
                } else {
                    prev.is_some_and(|p| d - p <= 5)
                };
                if close {
                    hits += 1;
                }
            }
            prev = Some(d);
            run = if last_day.is_some_and(|l| l + 1 == d) {
                run + 1
            } else {
                1
            };
            last_day = Some(d);
            longest = longest.max(run);
        }
    }
    (
        LevelRaw {
            births,
            shown: shown_v,
            regime_sum,
            regime_cnt,
        },
        ratio(hits, total),
        longest,
    )
}

fn regime_stats(sum: &[f64; 4], cnt: &[u64; 4], has: bool) -> (Vec<Option<f64>>, Option<f64>) {
    if !has {
        return (vec![None; 4], None);
    }
    #[allow(clippy::cast_precision_loss)]
    let means: Vec<Option<f64>> = (0..4)
        .map(|g| (cnt[g] > 0).then(|| sum[g] / cnt[g] as f64))
        .collect();
    let vals: Vec<f64> = means.iter().flatten().copied().collect();
    let mx = vals.iter().copied().reduce(f64::max);
    let mn = vals.iter().copied().reduce(f64::min);
    let r = match (mx, mn) {
        (Some(a), Some(b)) if b > 0.0 => Some(a / b),
        _ => None,
    };
    (means, r)
}

fn tier_outcome(counts: &[[u64; 2]]) -> TierOutcome {
    let share: Vec<Option<f64>> = counts.iter().map(|c| ratio(c[1], c[0])).collect();
    let closed: u64 = counts.iter().map(|c| c[0]).sum();
    let real: u64 = counts.iter().map(|c| c[1]).sum();
    let diff = match (
        share.first().copied().flatten(),
        share.get(2).copied().flatten(),
    ) {
        (Some(a), Some(b)) => Some(a - b),
        _ => None,
    };
    TierOutcome {
        by_tier: counts.to_vec(),
        share,
        base: ratio(real, closed),
        diff_1_3: diff,
    }
}

fn outcome_counts(b: &Births) -> Vec<[u64; 2]> {
    let mut c = vec![[0u64; 2]; 3];
    for &(s, _, _, tier) in &b.list {
        let Some(&st) = b.final_stage.get(&s) else {
            continue;
        };
        let t = (tier.clamp(1, 3) - 1) as usize;
        if st == u32::from(REALIZED) || st == u32::from(INVALIDATED) || st == u32::from(EXPIRED) {
            c[t][0] += 1;
            if st == u32::from(REALIZED) {
                c[t][1] += 1;
            }
        }
    }
    c
}

/// Score raw records (deterministic; pure function of the raw).
#[must_use]
pub fn score(alt: &str, data: &str, raws: &[M3Raw]) -> M3Summary {
    let mut datasets = Vec::new();
    let mut pooled_raw: Vec<Pool> = (0..3)
        .map(|_| (Vec::new(), Vec::new(), [0.0; 4], [0; 4], 0, 0, 0))
        .collect();
    let mut pooled_counts = vec![[0u64; 2]; 3];
    let mut means_per_level: Vec<Vec<f64>> = vec![Vec::new(); 3];
    let has_regime = raws.iter().all(|r| r.regime.is_some()) && !raws.is_empty();
    let mut ok = true;
    for r in raws {
        let b = births(r);
        let mut levels = Vec::new();
        for (li, name) in LEVELS.iter().enumerate() {
            let lvl = u32::try_from(li).unwrap_or(0) + 1;
            let (lr, rep, longest) = level_raw(r, &b, lvl);
            let (rm, rr) = regime_stats(&lr.regime_sum, &lr.regime_cnt, r.regime.is_some());
            let births_d = Dist::of(&lr.births);
            if let Some(m) = births_d.mean {
                means_per_level[li].push(m);
            }
            // Pool: repeats need hit/total, recompute from rep share × n is lossy, so
            // pool via the per-dataset numerators.
            let p = &mut pooled_raw[li];
            p.0.extend_from_slice(&lr.births);
            p.1.extend_from_slice(&lr.shown);
            for g in 0..4 {
                p.2[g] += lr.regime_sum[g];
                p.3[g] += lr.regime_cnt[g];
            }
            p.6 = p.6.max(longest);
            levels.push(LevelStats {
                level: (*name).to_owned(),
                births: births_d,
                shown: Dist::of(&lr.shown),
                regime_means: rm,
                regime_ratio: rr,
                repeat5: rep,
                longest_run: longest,
            });
        }
        let counts = outcome_counts(&b);
        for t in 0..3 {
            pooled_counts[t][0] += counts[t][0];
            pooled_counts[t][1] += counts[t][1];
        }
        ok &= r
            .lookahead
            .iter()
            .all(|c| c.mismatched == 0 && c.tier_mismatched == 0);
        datasets.push(DatasetScore {
            dataset: r.dataset.clone(),
            stories: b.list.len() as u64,
            levels,
            outcome: tier_outcome(&counts),
            lookahead: r.lookahead.clone(),
        });
    }
    // Pooled repeat share: weighted by births passing.
    let pooled: Vec<LevelStats> = (0..3)
        .map(|li| {
            let p = &pooled_raw[li];
            let (rm, rr) = regime_stats(&p.2, &p.3, has_regime);
            let mut num = 0.0;
            let mut den = 0.0;
            for d in &datasets {
                let l = &d.levels[li];
                if let (Some(s), Some(m)) = (l.repeat5, l.births.mean) {
                    #[allow(clippy::cast_precision_loss)]
                    let n = m * l.births.n as f64;
                    num += s * n;
                    den += n;
                }
            }
            LevelStats {
                level: LEVELS[li].to_owned(),
                births: Dist::of(&p.0),
                shown: Dist::of(&p.1),
                regime_means: rm,
                regime_ratio: rr,
                repeat5: (den > 0.0).then(|| num / den),
                longest_run: p.6,
            }
        })
        .collect();
    let seed_spread = means_per_level
        .iter()
        .map(|m| {
            if m.len() < 2 || data != "synthetic" {
                return [None, None];
            }
            let mx = m.iter().copied().fold(f64::MIN, f64::max);
            let mn = m.iter().copied().fold(f64::MAX, f64::min);
            let avg = common::mean(m).unwrap_or(0.0);
            [Some(mx - mn), (avg > 0.0).then(|| (mx - mn) / avg)]
        })
        .collect();
    M3Summary {
        alt: alt.to_owned(),
        data: data.to_owned(),
        rules: raws.first().map(|r| r.rules.clone()).unwrap_or_default(),
        datasets,
        pooled,
        pooled_outcome: tier_outcome(&pooled_counts),
        seed_spread,
        lookahead_ok: ok,
    }
}

fn level_table(out: &mut String, rows: &[(String, &LevelStats)]) {
    let _ = writeln!(
        out,
        "| Veri | Seviye | Ort. | Medyan | p90 | En çok | Sıfır gün | DK | Rejim oranı | 5 gün içinde tekrar | En uzun dizi | Gösterilen geçiş ort. |"
    );
    let _ = writeln!(out, "|---|---|---|---|---|---|---|---|---|---|---|---|");
    for (name, l) in rows {
        let _ = writeln!(
            out,
            "| {name} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            l.level,
            f(l.births.mean, 2),
            f(l.births.median, 0),
            f(l.births.p90, 0),
            f(l.births.max, 0),
            f(l.births.zero_share, 3),
            f(l.births.cv, 2),
            f(l.regime_ratio, 2),
            f(l.repeat5, 3),
            l.longest_run,
            f(l.shown.mean, 2),
        );
    }
}

/// Markdown of a summary.
#[must_use]
pub fn markdown(s: &M3Summary) -> String {
    let mut o = String::new();
    let _ = writeln!(o, "# M3 — `{}` ({})\n", s.alt, s.data);
    let _ = writeln!(
        o,
        "Kurallar: tema `{}`, sonuç `{}`, sıralayıcı `{}`. Düzenek: `harness-sim m3`. Sayılar yalnız ham kayıttan hesaplanır.\n",
        s.rules.first().map_or("", String::as_str),
        s.rules.get(1).map_or("", String::as_str),
        s.rules.get(2).map_or("", String::as_str)
    );
    let _ = writeln!(o, "## 1–2. Günlük kapsül sayısı (doğumda kademe süzgeci)\n");
    let mut rows = Vec::new();
    for l in &s.pooled {
        rows.push(("havuz".to_owned(), l));
    }
    for d in &s.datasets {
        for l in &d.levels {
            rows.push((d.dataset.clone(), l));
        }
    }
    level_table(&mut o, &rows);
    let _ = writeln!(o, "\nRejim ortalamaları (enstrüman-gün başına doğum; yükselen / düşen / yatay / oynak), havuz:\n");
    let _ = writeln!(o, "| Seviye | yükselen | düşen | yatay | oynak | oran |");
    let _ = writeln!(o, "|---|---|---|---|---|---|");
    for l in &s.pooled {
        let _ = writeln!(
            o,
            "| {} | {} | {} | {} | {} | {} |",
            l.level,
            f(l.regime_means[0], 4),
            f(l.regime_means[1], 4),
            f(l.regime_means[2], 4),
            f(l.regime_means[3], 4),
            f(l.regime_ratio, 2)
        );
    }
    let _ = writeln!(
        o,
        "\nTohumlar (veri kümeleri) arası fark, günlük ortalama:\n"
    );
    let _ = writeln!(o, "| Seviye | en çok − en az | göreli |");
    let _ = writeln!(o, "|---|---|---|");
    for (i, sp) in s.seed_spread.iter().enumerate() {
        let _ = writeln!(o, "| {} | {} | {} |", LEVELS[i], f(sp[0], 3), f(sp[1], 3));
    }
    let _ = writeln!(
        o,
        "\n## 3. Sonradan gerçekleşenlerin payı (doğumdaki kademe)\n"
    );
    let _ = writeln!(o, "| Veri | K1 kapanan | K1 pay | K2 kapanan | K2 pay | K3 kapanan | K3 pay | Taban | K1 − K3 |");
    let _ = writeln!(o, "|---|---|---|---|---|---|---|---|---|");
    let mut orow = |name: &str, t: &TierOutcome| {
        let _ = writeln!(
            o,
            "| {name} | {} | {} | {} | {} | {} | {} | {} | {} |",
            t.by_tier[0][0],
            f(t.share[0], 3),
            t.by_tier[1][0],
            f(t.share[1], 3),
            t.by_tier[2][0],
            f(t.share[2], 3),
            f(t.base, 3),
            f(t.diff_1_3, 3)
        );
    };
    orow("havuz", &s.pooled_outcome);
    for d in &s.datasets {
        orow(&d.dataset, &d.outcome);
    }
    let _ = writeln!(o, "\n## 4. İleriye bakma\n");
    let _ = writeln!(
        o,
        "Sonuç: **{}**.\n",
        if s.lookahead_ok { "geçti" } else { "KALDI" }
    );
    let _ = writeln!(
        o,
        "| Veri | Kesim t | Karşılaştırılan | Farklı | Kademe farklı | Gelecekte değişen |"
    );
    let _ = writeln!(o, "|---|---|---|---|---|---|");
    for d in &s.datasets {
        for c in &d.lookahead {
            let _ = writeln!(
                o,
                "| {} | {} | {} | {} | {} | {} |",
                d.dataset, c.cut_t, c.compared, c.mismatched, c.tier_mismatched, c.future_changed
            );
        }
    }
    let _ = writeln!(
        o,
        "\nHikâye sayısı: {}.",
        s.datasets
            .iter()
            .map(|d| format!("{} {}", d.dataset, d.stories))
            .collect::<Vec<_>>()
            .join(", ")
    );
    o
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// `m3 run`: simulate, write raw, then score from the written raw.
///
/// # Errors
/// Unknown alternative, missing real data, I/O.
pub fn cmd_run(alt: &str, data: DataKind, seeds: &[u64]) -> Result<String, String> {
    let fac = registry::find(&registry::rankers(), alt)
        .ok_or(format!("bilinmeyen M3 alternatifi: {alt}"))?;
    let ds = common::datasets(data, seeds);
    if ds.is_empty() {
        return Err("veri yok (gerçek veri dizini bulunamadı)".to_owned());
    }
    let make = move |_: &[InstrumentInput]| fac();
    let raws = common::par_map(&ds, |d| run_dataset(d, alt, &make));
    let dir = raw_location("M3", alt, data)?;
    for r in &raws {
        write_file(
            &dir.join(common::raw_file_name(data, &r.dataset)),
            &to_json(r),
        )
        .map_err(|e| e.to_string())?;
    }
    cmd_score(alt, data)
}

/// `m3 score`: read raw, write summary JSON + markdown.
///
/// # Errors
/// Missing raw, I/O.
pub fn cmd_score(alt: &str, data: DataKind) -> Result<String, String> {
    let dir = raw_location("M3", alt, data)?;
    let raws: Vec<M3Raw> = read_raws(&dir, data)?;
    let s = score(alt, data.name(), &raws);
    write_summary("M3", alt, data, &to_json(&s), &markdown(&s))
}

/// Raw directory: synthetic → `rounds/R3/experiments/<exp>/raw/<alt>/`; real → the local
/// `fixtures/local/r3-raw/<exp>/<alt>/` (never in the repo).
///
/// # Errors
/// Real data directory absent.
pub fn raw_location(exp: &str, alt: &str, data: DataKind) -> Result<std::path::PathBuf, String> {
    match data {
        DataKind::Synthetic => Ok(common::raw_dir(exp, alt)),
        DataKind::Real => common::real_raw_dir(exp, alt).ok_or("gerçek veri dizini yok".to_owned()),
    }
}

/// Read every raw file (`<prefix>-*.json`, sorted by name) of a data kind.
///
/// # Errors
/// I/O or parse errors, or none found.
pub fn read_raws<T: for<'de> Deserialize<'de>>(
    dir: &std::path::Path,
    data: DataKind,
) -> Result<Vec<T>, String> {
    let prefix = format!("{}-", data.name());
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(Result::ok)
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| n.starts_with(&prefix) && n.ends_with(".json"))
        .collect();
    names.sort();
    if names.is_empty() {
        return Err(format!("ham kayıt yok: {}", dir.display()));
    }
    names
        .iter()
        .map(|n| {
            let text = std::fs::read_to_string(dir.join(n)).map_err(|e| e.to_string())?;
            serde_json::from_str(&text).map_err(|e| format!("{n}: {e}"))
        })
        .collect()
}

/// Write `summary-<data>.json` and `.md` into the repo raw dir (aggregates only, also for
/// real data). Returns the markdown.
///
/// # Errors
/// I/O.
pub fn write_summary(
    exp: &str,
    alt: &str,
    data: DataKind,
    json: &str,
    md: &str,
) -> Result<String, String> {
    let dir = common::raw_dir(exp, alt);
    write_file(&dir.join(format!("summary-{}.json", data.name())), json)
        .map_err(|e| e.to_string())?;
    write_file(&dir.join(format!("summary-{}.md", data.name())), md).map_err(|e| e.to_string())?;
    Ok(md.to_owned())
}
