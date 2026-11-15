//! Approach D — evidence envelope + display states (RESEARCH §3.D), with E (data marks)
//! as its support mechanism. Design and measurements: `rounds/RP2/experiments/T7d.md`.
//!
//! - **Authority** runs the shared engine on its feed. Every claim it holds is sent once
//!   as `fact` (provisional) with an evidence envelope (`EvidenceRef` + anchor, rev,
//!   epoch, engine). A claim becomes `fact_final` only when every bar it rests on is
//!   confirmed against the provider's settled bars (T3; the authority refetches each
//!   closed bar `settle_lag` after its close). `fact_retracted` when it stops holding.
//!   Data marks (E) carry digests of settled bars and of 32-bar chunks.
//! - **Replica** keeps its own feed in the shared engine (the chart) and runs a per-claim
//!   state machine: a claim is asserted only while (1) the authority holds it, (2) the
//!   replica's own visible data covers its evidence window and (3) the predicate holds on
//!   the replica's own data — re-checked on every change of either side. `Final` needs a
//!   `fact_final`, a closed local bar and an equal window digest + anchor + values;
//!   otherwise the replica refetches the differing bars from its **own** settled source
//!   (never from the protocol, K23) and, if they still differ, shows `SourceDiffers`
//!   (predicate holds) or `Disputed` (it does not).
//!
//! Variants (ablations) are configurations of the same code ([`Cfg`]); only `d` is in
//! the registry (the registry macro registers one approach per module); the ablations
//! run through the ignored test `ablation_table` (see T7d.md).

mod authority;
mod replica;
mod wire;

use crate::adapter::{Approach, Setup};

/// What the final envelope's digest covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DigestMode {
    /// No digest at all (B-predicate + maturity).
    None,
    /// The evidence window (D).
    Window,
    /// Window plus the full prefix from the anchor (full commitment).
    Prefix,
}

/// Configuration of one D variant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cfg {
    /// Name.
    pub name: &'static str,
    /// Digest in final envelopes.
    pub digest: DigestMode,
    /// E: data marks (per-bar and per-chunk digests of settled bars).
    pub marks: bool,
    /// `fact_final` only after confirmation against the provider's settled bars.
    pub settle: bool,
    /// Replica re-resolves its history from the authority's anchor when they differ.
    pub anchor_align: bool,
    /// Replica backfills closed bars from its own settled source when its feed lags
    /// behind the authority's data marks.
    pub catch_up: bool,
    /// Replica refetches differing / disputed bars from its own settled source.
    pub repair: bool,
    /// Provisional guard: on the live bar the predicate must hold on the replica's data
    /// with this margin (RSI points) before the claim is (re)shown.
    pub prov_margin: f64,
    /// Provisional guard: the (margin) predicate must have held on the replica's live
    /// data continuously this long (µs) before the claim is (re)shown.
    pub prov_dwell: i64,
    /// The authority tells a new provisional claim only when the closed bars it rests on
    /// are confirmed against the provider.
    pub prov_settled_prev: bool,
}

/// The main configuration (`d`).
pub const D: Cfg = Cfg {
    name: "d",
    digest: DigestMode::Window,
    marks: true,
    settle: true,
    anchor_align: true,
    catch_up: true,
    repair: true,
    prov_margin: 0.75,
    prov_dwell: 0,
    prov_settled_prev: true,
};

/// D without E (no data marks).
pub const D_NOE: Cfg = Cfg {
    name: "d_noe",
    marks: false,
    ..D
};

/// D without any digest (predicate + maturity only; no marks).
pub const D_NODIGEST: Cfg = Cfg {
    name: "d_nodigest",
    digest: DigestMode::None,
    marks: false,
    ..D
};

/// D with the full-prefix digest (full commitment).
pub const D_PREFIX: Cfg = Cfg {
    name: "d_prefix",
    digest: DigestMode::Prefix,
    ..D
};

/// D without provider confirmation: final at the authority's own close.
pub const D_NOSETTLE: Cfg = Cfg {
    name: "d_nosettle",
    settle: false,
    ..D
};

/// D without the authority's settled-prev gate on new provisional claims.
pub const D_NOGATE: Cfg = Cfg {
    name: "d_nogate",
    prov_settled_prev: false,
    ..D
};

/// D without anchor alignment.
pub const D_NOANCHOR: Cfg = Cfg {
    name: "d_noanchor",
    anchor_align: false,
    ..D
};

/// D without catch-up.
pub const D_NOCATCHUP: Cfg = Cfg {
    name: "d_nocatchup",
    catch_up: false,
    ..D
};

/// D without any own-source refetch (no repair, catch-up or anchor alignment).
pub const D_NOREPAIR: Cfg = Cfg {
    name: "d_norepair",
    repair: false,
    catch_up: false,
    anchor_align: false,
    ..D
};

/// Provisional guard sweep: no guard (closest to RESEARCH §3.D as written).
pub const D_M0: Cfg = Cfg {
    name: "d_m0",
    prov_margin: 0.0,
    ..D
};

/// Guard sweep: margin 0.5.
pub const D_M05: Cfg = Cfg {
    name: "d_m05",
    prov_margin: 0.5,
    ..D
};

/// Guard sweep: margin 1.0.
pub const D_M10: Cfg = Cfg {
    name: "d_m10",
    prov_margin: 1.0,
    ..D
};

/// Guard sweep: margin 1.5.
pub const D_M15: Cfg = Cfg {
    name: "d_m15",
    prov_margin: 1.5,
    ..D
};

/// Guard sweep: no margin, dwell 300 ms instead.
pub const D_W300: Cfg = Cfg {
    name: "d_w300",
    prov_margin: 0.0,
    prov_dwell: 300_000,
    ..D
};

macro_rules! approach {
    ($cfg:expr, $summary:expr) => {
        Approach {
            name: $cfg.name,
            summary: $summary,
            authority: |s: &Setup| Box::new(authority::Authority::new(s, $cfg)),
            replica: |s: &Setup| Box::new(replica::Replica::new(s, $cfg)),
        }
    };
}

/// Registry entry (`d`).
#[must_use]
pub fn register() -> Approach {
    approach!(
        D,
        "D: evidence envelope + display states (+E data marks, settled confirmation)"
    )
}

/// All variants (main first), for the ablation runs.
#[must_use]
pub fn variants() -> Vec<Approach> {
    vec![
        register(),
        approach!(D_NOE, "D without E (no data marks)"),
        approach!(D_NODIGEST, "D without digests (predicate + maturity)"),
        approach!(D_PREFIX, "D with full-prefix digest"),
        approach!(D_NOSETTLE, "D without settled confirmation"),
        approach!(D_NOGATE, "D without the settled-prev gate"),
        approach!(D_NOANCHOR, "D without anchor alignment"),
        approach!(D_NOCATCHUP, "D without catch-up"),
        approach!(D_NOREPAIR, "D without any own-source refetch"),
        approach!(D_M0, "D without provisional guard"),
        approach!(D_M05, "D, guard margin 0.5"),
        approach!(D_M10, "D, guard margin 1.0"),
        approach!(D_M15, "D, guard margin 1.5"),
        approach!(D_W300, "D, guard dwell 300 ms, no margin"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenarios;
    use crate::sim::run_one;

    #[test]
    fn deterministic_and_clean_on_ideal() {
        let ap = register();
        for name in ["S1", "S0", "S9"] {
            let spec = scenarios::find(name).unwrap();
            for seed in 0..2 {
                let a = run_one(&spec, &ap, seed);
                let b = run_one(&spec, &ap, seed);
                assert_eq!(a.log, b.log, "{name} seed {seed}");
                assert_eq!(a.metrics.errors(), 0, "{name} seed {seed}: {:?}", a.metrics);
            }
        }
    }

    /// Truth timeline of every claim (`key text → [(vtime, holds)]`), rebuilt from the
    /// world with the shared series/detector code (diagnostics only).
    fn truth_timeline(
        spec: &crate::spec::ScenarioSpec,
        seed: u64,
    ) -> std::collections::BTreeMap<String, Vec<(i64, bool)>> {
        use crate::detector::DetectorCfg;
        use crate::series::SeriesState;
        use crate::world::{generate, StepKind};
        use qp_schema::data::DataChange;
        let world = generate(spec, seed);
        let det = DetectorCfg::new(spec.detector.clone());
        let mut ser: Vec<SeriesState> = world
            .history
            .iter()
            .map(|h| {
                let mut s = SeriesState::new(spec.detector.period, 0);
                s.set_bars(h.clone());
                s
            })
            .collect();
        let mut claims = std::collections::BTreeMap::new();
        let mut out: std::collections::BTreeMap<String, Vec<(i64, bool)>> =
            std::collections::BTreeMap::new();
        let mut rec = |now: i64, ser: &Vec<SeriesState>, b: usize, from: i64, claims: &mut _| {
            let mut deltas = Vec::new();
            let refs: Vec<&SeriesState> = ser.iter().collect();
            det.recompute(&world.ids, &refs, b, from, claims, &mut deltas);
            for d in deltas {
                out.entry(d.key.text())
                    .or_default()
                    .push((now, d.after.is_some()));
            }
        };
        for b in 0..ser.len() {
            rec(0, &ser, b, i64::MIN, &mut claims);
        }
        for st in &world.steps {
            let b = st.binding;
            match &st.kind {
                StepKind::Close { .. } => {}
                StepKind::Snapshot { bars, .. } => {
                    ser[b].set_bars(bars.clone());
                    rec(st.at, &ser, b, i64::MIN, &mut claims);
                }
                StepKind::Data { ev, .. } => {
                    let s = &mut ser[b];
                    let old_last = s.bars().last().map(|x| x.t);
                    let from = match &ev.change {
                        DataChange::Bar { bar } => s.upsert(bar.clone()),
                        DataChange::Patch { t, fields } => match s.index_of(*t) {
                            Some(i) => {
                                let mut bar = s.bars()[i].clone();
                                fields.apply_to(&mut bar);
                                s.upsert(bar)
                            }
                            None => continue,
                        },
                    };
                    let new_last = s.bars().last().map(|x| x.t);
                    let from = match (old_last, new_last) {
                        (Some(o), Some(n)) if n > o => o.min(from),
                        _ => from,
                    };
                    rec(st.at, &ser, b, from, &mut claims);
                }
            }
        }
        out
    }

    /// Lists provisional shows that are false in the truth (diagnostics):
    /// `D_DIAG=S7:0-9 cargo test --release diag_m3p -- --ignored --nocapture`.
    #[test]
    #[ignore = "diagnostics"]
    fn diag_m3p() {
        let arg = std::env::var("D_DIAG").unwrap_or_else(|_| "S8:0-29".into());
        let (sc, range) = arg.split_once(':').unwrap();
        let (a, b) = range.split_once('-').unwrap();
        let spec = scenarios::find(sc).unwrap();
        let ap = variants()
            .into_iter()
            .find(|x| x.name == std::env::var("D_VAR").unwrap_or_else(|_| "d".into()))
            .unwrap();
        for seed in a.parse::<u64>().unwrap()..=b.parse::<u64>().unwrap() {
            let tl = truth_timeline(&spec, seed);
            let out = run_one(&spec, &ap, seed);
            let log = String::from_utf8_lossy(&out.log).to_string();
            for line in log.lines() {
                let f: Vec<&str> = line.split(' ').collect();
                if f.len() < 5 || f[4] != "provisional" || f[2] == "provisional" {
                    continue;
                }
                let now: i64 = f[0].parse().unwrap();
                let hist = tl.get(f[1]).cloned().unwrap_or_default();
                let holds = hist
                    .iter()
                    .rev()
                    .find(|(t, _)| *t <= now)
                    .is_some_and(|x| x.1);
                if !holds {
                    let near: Vec<String> = hist
                        .iter()
                        .filter(|(t, _)| (now - t).abs() < 5_000_000)
                        .map(|(t, h)| format!("{}{}", (t - now) / 1000, if *h { "+" } else { "-" }))
                        .collect();
                    println!("{sc} s{seed} {line} | truth near (ms rel): {}", near.join(" "));
                }
            }
        }
    }

    /// Claims whose last display state disagrees with the truth at the end (diagnostics):
    /// `D_SPEC=file.json D_DIAG=0-9 cargo test --release diag_end -- --ignored --nocapture`.
    #[test]
    #[ignore = "diagnostics"]
    fn diag_end() {
        let spec: crate::spec::ScenarioSpec = match std::env::var("D_SPEC") {
            Ok(p) => serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap(),
            Err(_) => scenarios::find("S9").unwrap(),
        };
        let arg = std::env::var("D_DIAG").unwrap_or_else(|_| "0-9".into());
        let (a, b) = arg.split_once('-').unwrap();
        for seed in a.parse::<u64>().unwrap()..=b.parse::<u64>().unwrap() {
            let tl = truth_timeline(&spec, seed);
            let out = run_one(&spec, &register(), seed);
            let log = String::from_utf8_lossy(&out.log).to_string();
            let mut last: std::collections::BTreeMap<&str, (&str, Vec<&str>)> =
                std::collections::BTreeMap::new();
            for line in log.lines() {
                let f: Vec<&str> = line.split(' ').collect();
                let e = last.entry(f[1]).or_insert(("-", Vec::new()));
                e.0 = f[4];
                e.1.push(line);
            }
            // Long "truth holds on a closed bar, not asserted" spans.
            for (k, h) in &tl {
                let bar_t: i64 = k.rsplit(':').nth(1).and_then(|x| x.parse().ok()).unwrap_or(0);
                let close = crate::spec::LIVE0 + (bar_t - crate::spec::BASE_T + 60_000) * 1000;
                let mut disp: Vec<(i64, bool)> = Vec::new();
                for line in log.lines() {
                    let f: Vec<&str> = line.split(' ').collect();
                    if f[1] == k {
                        disp.push((
                            f[0].parse().unwrap(),
                            matches!(f[4], "final" | "provisional" | "source_differs"),
                        ));
                    }
                }
                let mut ev: Vec<(i64, u8, bool)> = h.iter().map(|&(t, x)| (t, 0, x)).collect();
                ev.extend(disp.iter().map(|&(t, x)| (t, 1, x)));
                ev.push((close, 2, true));
                ev.sort_unstable();
                let (mut tr, mut sh, mut cl) = (false, false, false);
                let mut since: Option<i64> = None;
                for (t, kind, x) in ev {
                    match kind {
                        0 => tr = x,
                        1 => sh = x,
                        _ => cl = x,
                    }
                    let bad = tr && cl && !sh;
                    match (since, bad) {
                        (None, true) => since = Some(t),
                        (Some(s), false) => {
                            if t - s > 5_000_000 {
                                println!("seed {seed} {k} blocked {}..{} ms", s / 1000, t / 1000);
                                for l in log.lines().filter(|l| l.contains(k.as_str())).take(12) {
                                    println!("    {l}");
                                }
                            }
                            since = None;
                        }
                        _ => {}
                    }
                }
            }
            for (k, h) in &tl {
                let holds = h.last().is_some_and(|x| x.1);
                let shown = last
                    .get(k.as_str())
                    .is_some_and(|x| matches!(x.0, "final" | "provisional" | "source_differs"));
                if holds != shown {
                    println!("seed {seed} {k} truth={holds} shown={shown} truth_hist={:?}", h.iter().map(|(t, x)| (t / 1000, *x)).collect::<Vec<_>>());
                    if let Some(x) = last.get(k.as_str()) {
                        for l in x.1.iter().rev().take(6).rev() {
                            println!("    {l}");
                        }
                    }
                }
            }
        }
    }

    /// Prints real authority messages of S1 seed 0 (for the document's examples):
    /// `cargo test --release print_messages -- --ignored --nocapture`.
    #[test]
    #[ignore = "diagnostics"]
    fn print_messages() {
        use crate::adapter::{AuthorityAdapter, Ctx, FetchReply};
        let spec = scenarios::find("S1").unwrap();
        let world = crate::world::generate(&spec, 0);
        let setup = crate::adapter::Setup {
            side: crate::adapter::Side::Authority,
            bindings: vec![crate::adapter::BindingSetup {
                id: world.ids[0].clone(),
                binding: qp_schema::data::DataBinding {
                    symbol: world.feeds[0].symbol.clone(),
                    interval: world.feeds[0].interval.clone(),
                    window: qp_schema::data::Window::last(spec.history_bars),
                    feed: None,
                },
                feed: world.feeds[0].clone(),
                interval_ms: world.interval_ms,
            }],
            detector: crate::detector::DetectorCfg::new(spec.detector.clone()),
            extra_studies: 0,
        };
        let mut a = authority::Authority::new(&setup, D);
        let (mut evs, _) = crate::feed::deliver(
            &world,
            0,
            &spec.authority_feed,
            &mut crate::rng::Rng::new(0),
        );
        evs.sort_by_key(|(t, _)| *t);
        let mut printed = 0;
        let mut pending: Vec<(i64, u64, i64, i64)> = Vec::new();
        let mut timers: Vec<(i64, u64)> = Vec::new();
        let mut fid = 1;
        let show = |ctx: &Ctx, printed: &mut usize| {
            for m in &ctx.msgs {
                let b: serde_json::Value = serde_json::from_slice(m).unwrap();
                println!("batch s={} at={} bytes={}", b["s"], ctx.now(), m.len());
                for op in b["ops"].as_array().unwrap() {
                    let t = op.to_string();
                    if *printed < 40 {
                        println!("  {}", &t[..t.len().min(700)]);
                        *printed += 1;
                    }
                }
            }
        };
        for (at, ev) in evs {
            timers.sort_unstable();
            while let Some(&(t, tok)) = timers.first() {
                if t > at {
                    break;
                }
                timers.remove(0);
                let mut ctx = Ctx::new(t, fid);
                a.on_timer(&mut ctx, tok);
                for f in &ctx.fetches {
                    pending.push((t + 800_000, f.id, f.from_t, f.to_t));
                }
                fid = ctx.next_fetch();
                show(&ctx, &mut printed);
            }
            pending.sort_unstable();
            while let Some(&(t, id, from, to)) = pending.first() {
                if t > at {
                    break;
                }
                pending.remove(0);
                let bars = world.history[0]
                    .iter()
                    .filter(|x| x.t >= from && x.t <= to)
                    .cloned()
                    .collect();
                let mut ctx = Ctx::new(t, fid);
                a.on_fetch(
                    &mut ctx,
                    &FetchReply {
                        id,
                        binding: 0,
                        bars,
                    },
                );
                show(&ctx, &mut printed);
            }
            let mut ctx = Ctx::new(at, fid);
            a.on_source(&mut ctx, &ev);
            timers.extend(ctx.timers.iter().copied());
            fid = ctx.next_fetch();
            show(&ctx, &mut printed);
            if printed >= 40 {
                break;
            }
        }
    }

    /// Prints the ablation table: `cargo test --release ablation_table -- --ignored
    /// --nocapture` (env `D_SEEDS`, `D_SCEN` are read only by this test, not by the
    /// adapters).
    #[test]
    #[ignore = "long; run explicitly"]
    fn ablation_table() {
        let seeds: u64 = std::env::var("D_SEEDS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(100);
        let only: Option<Vec<String>> = std::env::var("D_SCEN")
            .ok()
            .map(|s| s.split(',').map(str::to_owned).collect());
        let vonly: Option<Vec<String>> = std::env::var("D_VAR")
            .ok()
            .map(|s| s.split(',').map(str::to_owned).collect());
        println!(
            "variant scen seeds fail live | M1 M2 M3f M3p | M4 M5 | M6p p50/p99 | M6f p50/p99 | M7ms M8 M8ms | B/s r2a | M10 | M11 p50/un/x | M12 | M13ms"
        );
        for spec in scenarios::presets() {
            if only.as_ref().is_some_and(|o| !o.contains(&spec.name)) {
                continue;
            }
            let n = seeds.min(u64::from(spec.default_seeds));
            for ap in variants() {
                if vonly.as_ref().is_some_and(|o| !o.iter().any(|x| x == ap.name)) {
                    continue;
                }
                let (_, s) = crate::run_scenario(&spec, &ap, 0, n, 16);
                println!(
                    "{} {} {} {} {} | {} {} {} {} | {} {}/{} | {:.0}/{:.0} | {:.0}/{:.0} | {:.0} {} {:.0} | {:.1} {:.1} | {:.1} | {:.0}/{}/{} | {}/{} | {:.0}",
                    ap.name,
                    s.scenario,
                    s.seeds,
                    s.failing_seeds,
                    s.live_errors,
                    s.m1_early,
                    s.m2_contradictory,
                    s.m3_wrong_final,
                    s.m3_wrong_provisional,
                    s.m4_retracted_provisional,
                    s.m5_missed,
                    s.m5_eligible,
                    s.m6_provisional.p50,
                    s.m6_provisional.p99,
                    s.m6_final.p50,
                    s.m6_final.p99,
                    s.m7_stale_ms,
                    s.m8_blocked,
                    s.m8_blocked_ms,
                    s.m9_a2r_bytes_per_sec,
                    s.m9_r2a_bytes_per_sec,
                    s.m10_work_per_event,
                    s.m11_latency.p50,
                    s.m11_unresolved,
                    s.m11_unneeded,
                    s.m12_flips,
                    s.m12_max_flips,
                    s.m13_dispute_ms
                );
            }
        }
    }
}
