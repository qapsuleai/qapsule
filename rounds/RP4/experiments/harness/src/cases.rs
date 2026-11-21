//! (RP4, M25) Arkin's observation, reproduced (`rounds/R6/REPORT.md` §10): on 5 minutes a
//! **down** claim is judged "tutmadı" (invalidated) by `r3` on the **very next bar**, and a
//! later close inside the story's horizon reaches the claim's own target anyway. This
//! command finds every such story in the counted window (optionally one local day) and shows
//! how another rule resolves the same story (same birth: the detectors are the reference set
//! for both). Output: aggregate counts on stdout and in `<out>/cases.<rule>.<period>.json`;
//! per-story detail (symbol, local time, prices in ATR units) only in
//! `<out>/cases.<rule>.<period>.local.jsonl` under `fixtures/local/` (real data stays local).

use std::collections::BTreeMap;
use std::path::Path;

use qp_schema::capsule::Direction;

use crate::common::{par_map, xist};
use crate::data::{load_for, Period, Split, Window};
use crate::registry;
use crate::sim::{day_positions, facts_for, run_symbol, StoryRec, EXP, INV, OPEN, REAL};

fn stage_name(s: u8) -> &'static str {
    match s {
        OPEN => "open",
        REAL => "realized",
        INV => "invalidated",
        EXP => "expired",
        _ => "?",
    }
}

/// Run the case search.
///
/// # Errors
/// Unknown rule, data errors.
#[allow(clippy::too_many_lines)]
pub fn cmd(
    data: &Path,
    rule: &str,
    period: Period,
    split: Split,
    day: Option<&str>,
    out: &Path,
) -> Result<String, String> {
    let alt = registry::rule(rule).ok_or(format!("bilinmeyen kural: {rule}"))?;
    let r3 = registry::reference_rule();
    let series = load_for(data, period, split)?;
    let window = Window::of(period, split);
    let cal = xist();
    let want_day: Option<i64> = match day {
        Some(d) => {
            let ms = crate::common::local_noon_ms(d).ok_or(format!("gün: {d}"))?;
            Some(cal.local_day(ms))
        }
        None => None,
    };
    let det = registry::reference_detectors();
    let per = par_map(&series, |s| {
        let pos = day_positions(&cal, &s.bars);
        let facts = facts_for(&s.symbol, period.name(), &s.bars, det.make, false);
        let a = run_symbol(
            &s.symbol,
            period.name(),
            &s.bars,
            &facts,
            &r3,
            window,
            &pos,
            true,
        );
        let b = run_symbol(
            &s.symbol,
            period.name(),
            &s.bars,
            &facts,
            &alt,
            window,
            &pos,
            true,
        );
        let by_birth: BTreeMap<(u64, String), &StoryRec> =
            b.iter().map(|x| ((x.born_bar, x.key.clone()), x)).collect();
        let mut rows = Vec::new();
        let mut all_down_inv_next = 0u64;
        for x in &a {
            if !window.contains(x.born_t) {
                continue;
            }
            if let Some(d) = want_day {
                if cal.local_day(x.born_t) != d {
                    continue;
                }
            }
            if x.direction != Direction::Down || x.stage != INV || x.close_bar != x.born_bar + 1 {
                continue;
            }
            all_down_inv_next += 1;
            if x.ecr_own != Some(true) {
                continue;
            }
            let other = by_birth.get(&(x.born_bar, x.key.clone()));
            let born = usize::try_from(x.born_bar).unwrap_or(0);
            let atr = qp_capsule::outcome::wilder_atr(&s.bars[..=born], 14).unwrap_or(f64::NAN);
            let p0 = s.bars[born].c;
            let path: Vec<f64> = s
                .bars
                .iter()
                .skip(born + 1)
                .take(x.horizon as usize)
                .map(|bb| ((bb.c - p0) / atr * 100.0).round() / 100.0)
                .collect();
            rows.push(serde_json::json!({
                "symbol": s.symbol,
                "born_local": crate::common::local_hhmm(x.born_t),
                "day": crate::common::local_date(x.born_t),
                "key": x.key,
                "horizon": x.horizon,
                "r3": {"stage": "invalidated", "bars_to_close": 1},
                "alt": other.map(|o| serde_json::json!({
                    "stage": stage_name(o.stage),
                    "bars_to_close": if o.stage == OPEN { serde_json::Value::Null }
                        else { serde_json::json!(o.close_bar - o.born_bar) },
                })),
                "closes_atr_units_from_birth": path,
            }));
        }
        (all_down_inv_next, rows)
    });
    let mut n_next = 0u64;
    let mut rows = Vec::new();
    for (n, r) in per {
        n_next += n;
        rows.extend(r);
    }
    let mut by_stage: BTreeMap<String, u64> = BTreeMap::new();
    for r in &rows {
        let k = r["alt"]["stage"].as_str().unwrap_or("missing").to_owned();
        *by_stage.entry(k).or_default() += 1;
    }
    let summary = serde_json::json!({
        "rule": rule, "period": period.name(), "split": format!("{split:?}"),
        "day": day,
        "r3_down_invalidated_next_bar": n_next,
        "of_which_later_reached_target_in_horizon": rows.len(),
        "same_story_under_rule": by_stage,
    });
    std::fs::create_dir_all(out).map_err(|e| e.to_string())?;
    let tag = format!("{rule}.{}.{}", period.name(), day.unwrap_or("all"));
    std::fs::write(
        out.join(format!("cases.{tag}.json")),
        serde_json::to_string_pretty(&summary).unwrap_or_default(),
    )
    .map_err(|e| e.to_string())?;
    let local: String = rows.iter().map(|r| format!("{r}\n")).collect();
    std::fs::write(out.join(format!("cases.{tag}.local.jsonl")), local)
        .map_err(|e| e.to_string())?;
    Ok(summary.to_string())
}

/// (RP4, M27) Births of every story in the counted window (`symbol`, birth time, ledger
/// key) as local JSON lines under `out` (real data stays local), for comparing two bar
/// sources. Returns the count.
///
/// # Errors
/// Unknown rule, data errors.
pub fn births_cmd(
    data: &Path,
    rule: &str,
    period: Period,
    split: Split,
    out: &Path,
) -> Result<String, String> {
    let entry = registry::rule(rule).ok_or(format!("bilinmeyen kural: {rule}"))?;
    let series = load_for(data, period, split)?;
    let window = Window::of(period, split);
    let cal = xist();
    let det = registry::reference_detectors();
    let per = par_map(&series, |s| {
        let pos = day_positions(&cal, &s.bars);
        let facts = facts_for(&s.symbol, period.name(), &s.bars, det.make, false);
        run_symbol(
            &s.symbol,
            period.name(),
            &s.bars,
            &facts,
            &entry,
            window,
            &pos,
            false,
        )
        .into_iter()
        .filter(|x| window.contains(x.born_t))
        .map(|x| format!("{}\t{}\t{}\n", s.symbol, x.born_t, x.key))
        .collect::<String>()
    });
    std::fs::create_dir_all(out).map_err(|e| e.to_string())?;
    let text: String = per.concat();
    let n = text.lines().count();
    std::fs::write(
        out.join(format!("births.{rule}.{}.local.tsv", period.name())),
        text,
    )
    .map_err(|e| e.to_string())?;
    Ok(format!("{n} doğum"))
}
