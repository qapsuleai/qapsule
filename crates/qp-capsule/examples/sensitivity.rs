//! R3/S-A sensitivity simulation (GOAL criterion 12, `docs/CAPSULE.md` §6, §9.8, K31),
//! re-run in R4/T3 after K42 (A22: only negative transitions of a position/intent story
//! pass any level; A23: directional ledger key). `admits` judges each event at its own
//! transition stage (the event's story snapshot), so births no longer pass by position.
//!
//! ```text
//! cargo run --release -p qp-capsule --example sensitivity -- synthetic
//! cargo run --release -p qp-capsule --example sensitivity -- real      # local data only
//! ```
//!
//! The chosen engine (`EngineConfig::r3()`, the seven default detectors) runs over the
//! regime synthetic market (`synthetic_market(30, 500, seed)`, seeds 1, 2, 3, synthetic
//! licensed flow) or over the local real bars (`$QAPSULE_LOCAL_BARS` or
//! `fixtures/local/bars`; aggregate statistics only). Then a synthetic audience is
//! filtered with `qp_capsule::admits` at each level:
//!
//! - **Universe** of the audience = watch list ∪ position instruments ∪ intent instruments
//!   (watch-list selection is the subscription layer; `admits` is the level filter).
//! - **Levels**: `important` (tier 1), `balanced` (tiers 1–2), `all`; and the two user
//!   groups with no explicit level (their defaults: `habituated` → `all`, `future` →
//!   `important`).
//! - **Daily capsules** (primary, as M3): stories **born** that trading day on the
//!   universe whose tier at birth passes (or that pass by position / intent). Secondary:
//!   every story event of that day that passes (any stage, the event's tier).
//! - **Override**: passes only because of a position or intent (the tier filter alone
//!   would reject it). Counted per level for births and events, events also by stage.
//! - **Market** (no audience): births on every instrument whose tier passes, for the
//!   comparison with `rounds/R3/experiments/M3/RESULTS.md` (`confirm`, pooled).
//! - Days counted: from the first day of bar 50 (detector warm-up) to the end.
//! - The level is never changed by the system: the audience is compared before and after
//!   every run (assert).
//!
//! Output (R4/T3): `rounds/R4/experiments/T3/sensitivity.{md,json}` (synthetic) or
//! `sensitivity-real.{md,json}` (real; aggregate only); the R3 outputs in
//! `rounds/R3/experiments/S/` are the pre-K42 record and are not rewritten. Deterministic:
//! two runs give the same bytes.

#[path = "support/mod.rs"]
mod support;

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::PathBuf;

use qp_capsule::engine::EngineConfig;
use qp_capsule::{admits, simulate, InstrumentInput, StoryEvent};
use qp_facts::default_detectors;
use qp_schema::capsule::{Audience, Sensitivity, Stage, Story, UserGroup};
use serde::Serialize;

const WARMUP_BARS: usize = 50;
const SEEDS: [u64; 3] = [1, 2, 3];
const LEVELS: [Sensitivity; 3] = [
    Sensitivity::Important,
    Sensitivity::Balanced,
    Sensitivity::All,
];

/// Distribution summary of daily counts (nearest-rank percentiles, as harness-sim).
#[derive(Debug, Clone, Serialize)]
struct Dist {
    days: usize,
    mean: f64,
    median: f64,
    p90: f64,
    max: f64,
    zero_share: f64,
}

#[allow(clippy::cast_precision_loss)]
fn dist(xs: &[u64]) -> Dist {
    let mut v: Vec<u64> = xs.to_vec();
    v.sort_unstable();
    let n = v.len();
    let pct = |p: f64| -> f64 {
        if n == 0 {
            return 0.0;
        }
        let rank = (p * n as f64).ceil() as usize;
        v[rank.clamp(1, n) - 1] as f64
    };
    let mut sum = 0.0;
    for &x in xs {
        sum += x as f64;
    }
    Dist {
        days: n,
        mean: if n == 0 { 0.0 } else { sum / n as f64 },
        median: pct(0.5),
        p90: pct(0.9),
        max: v.last().map_or(0.0, |&x| x as f64),
        zero_share: if n == 0 {
            0.0
        } else {
            xs.iter().filter(|&&x| x == 0).count() as f64 / n as f64
        },
    }
}

fn level_name(s: Sensitivity) -> &'static str {
    match s {
        Sensitivity::Important => "important",
        Sensitivity::Balanced => "balanced",
        Sensitivity::All => "all",
    }
}

fn group_name(g: UserGroup) -> &'static str {
    match g {
        UserGroup::Habituated => "habituated",
        UserGroup::Future => "future",
    }
}

fn stage_name(s: Stage) -> &'static str {
    match s {
        Stage::Candidate => "candidate",
        Stage::Born => "born",
        Stage::Strengthened => "strengthened",
        Stage::Weakened => "weakened",
        Stage::Confirmed => "confirmed",
        Stage::Realized => "realized",
        Stage::Invalidated => "invalidated",
        Stage::Expired => "expired",
        Stage::Retracted => "retracted",
    }
}

fn tier_passes(level: Sensitivity, tier: u8) -> bool {
    match level {
        Sensitivity::Important => tier <= 1,
        Sensitivity::Balanced => tier <= 2,
        Sensitivity::All => true,
    }
}

/// The synthetic audience: watch list, positions on three instruments (one outside the
/// watch list), an active intent on one.
#[derive(Debug, Clone, Serialize)]
struct AudienceSpec {
    watchlist: Vec<String>,
    positions: BTreeMap<String, f64>,
    intents_on: Vec<String>,
}

impl AudienceSpec {
    fn universe(&self) -> Vec<String> {
        let mut u: Vec<String> = self
            .watchlist
            .iter()
            .chain(self.positions.keys())
            .chain(&self.intents_on)
            .cloned()
            .collect();
        u.sort();
        u.dedup();
        u
    }

    fn audience(&self, group: UserGroup, level: Option<Sensitivity>) -> Audience {
        Audience {
            group,
            sensitivity: level,
            watchlist: self.watchlist.clone(),
            positions: self.positions.clone(),
        }
    }
}

fn audience_for(symbols: &[String]) -> AudienceSpec {
    let mut s = symbols.to_vec();
    s.sort();
    if s.len() >= 30 {
        // SYN01..SYN10 watched; positions on SYN01, SYN04 (short) and SYN12 (outside the
        // watch list); intent on SYN07.
        AudienceSpec {
            watchlist: s[..10].to_vec(),
            positions: BTreeMap::from([
                (s[0].clone(), 100.0),
                (s[3].clone(), -50.0),
                (s[11].clone(), 200.0),
            ]),
            intents_on: vec![s[6].clone()],
        }
    } else {
        // Few real symbols: watch all; positions on the first three, intent on the fourth.
        AudienceSpec {
            watchlist: s.clone(),
            positions: s
                .iter()
                .take(3)
                .enumerate()
                .map(|(i, x)| (x.clone(), if i == 1 { -50.0 } else { 100.0 }))
                .collect(),
            intents_on: s.iter().skip(3).take(1).cloned().collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct LevelResult {
    /// Daily births that pass (primary).
    births: Dist,
    /// Daily events that pass (secondary).
    events: Dist,
    /// Births passing only by position / intent.
    override_births: u64,
    /// Events passing only by position / intent.
    override_events: u64,
    /// Override events by stage.
    override_events_by_stage: BTreeMap<String, u64>,
    /// Births that pass, total.
    births_total: u64,
}

#[derive(Debug, Clone, Serialize)]
struct GroupResult {
    default_level: String,
    births: Dist,
    events: Dist,
    override_births: u64,
    override_events: u64,
}

#[derive(Debug, Clone, Serialize)]
struct DatasetResult {
    name: String,
    instruments: usize,
    trading_days: usize,
    counted_days: usize,
    stories: u64,
    events: u64,
    levels: BTreeMap<String, LevelResult>,
    groups: BTreeMap<String, GroupResult>,
    market_births: BTreeMap<String, Dist>,
}

/// One dataset: daily vectors so the pool can be built.
struct Daily {
    result: DatasetResult,
    /// level → (births per day, events per day)
    level_days: BTreeMap<String, (Vec<u64>, Vec<u64>)>,
    group_days: BTreeMap<String, (Vec<u64>, Vec<u64>)>,
    market_days: BTreeMap<String, Vec<u64>>,
    overrides: BTreeMap<String, (u64, u64, BTreeMap<String, u64>, u64)>,
    group_overrides: BTreeMap<String, (u64, u64)>,
}

/// Trading-day index of every bar per instrument: synthetic = bar index; real = rank of
/// the Istanbul local date.
fn day_index(inputs: &[InstrumentInput]) -> (BTreeMap<String, Vec<usize>>, usize, usize) {
    let local = |t: i64| (t + 3 * 3_600_000).div_euclid(86_400_000);
    let mut days: Vec<i64> = inputs
        .iter()
        .flat_map(|x| x.bars.iter().map(|b| local(b.t)))
        .collect();
    days.sort_unstable();
    days.dedup();
    let pos: BTreeMap<i64, usize> = days.iter().enumerate().map(|(i, d)| (*d, i)).collect();
    let mut first = usize::MAX;
    let mut out = BTreeMap::new();
    for x in inputs {
        let v: Vec<usize> = x.bars.iter().map(|b| pos[&local(b.t)]).collect();
        first = first.min(v.get(WARMUP_BARS).copied().unwrap_or(usize::MAX));
        out.insert(x.symbol.clone(), v);
    }
    (out, days.len(), if first == usize::MAX { 0 } else { first })
}

#[allow(clippy::too_many_lines)]
fn run_dataset(name: &str, inputs: &[InstrumentInput], spec: &AudienceSpec) -> Daily {
    let sim = simulate(inputs, &default_detectors, EngineConfig::r3());
    let (dob, n_days, first) = day_index(inputs);
    let universe = spec.universe();
    let day_of = |e: &StoryEvent| -> usize { dob[&e.story.instrument][e.transition.bar as usize] };
    let births: Vec<&StoryEvent> = sim
        .events
        .iter()
        .filter(|e| e.transition.stage == Stage::Born)
        .collect();

    let mut level_days = BTreeMap::new();
    let mut overrides = BTreeMap::new();
    let mut group_days = BTreeMap::new();
    let mut group_overrides = BTreeMap::new();

    // One filter pass for an audience: (births/day, events/day, override births, override
    // events, override events by stage, births total).
    let pass = |aud: &Audience| {
        let before = aud.clone();
        let level = aud.effective();
        let mut b = vec![0u64; n_days];
        let mut ev = vec![0u64; n_days];
        let (mut ob, mut oe, mut total) = (0u64, 0u64, 0u64);
        let mut by_stage: BTreeMap<String, u64> = BTreeMap::new();
        let shown =
            |s: &Story| universe.contains(&s.instrument) && admits(aud, s, &spec.intents_on);
        for e in &births {
            let d = day_of(e);
            if d < first || !shown(&e.story) {
                continue;
            }
            b[d] += 1;
            total += 1;
            if !tier_passes(level, e.story.tier) {
                ob += 1;
            }
        }
        for e in &sim.events {
            let d = day_of(e);
            if d < first || !shown(&e.story) {
                continue;
            }
            ev[d] += 1;
            if !tier_passes(level, e.story.tier) {
                oe += 1;
                *by_stage
                    .entry(stage_name(e.transition.stage).to_owned())
                    .or_insert(0) += 1;
            }
        }
        // K31: the system never changes the level.
        assert_eq!(&before, aud, "the audience changed");
        assert_eq!(before.effective(), aud.effective(), "the level changed");
        (
            b[first..].to_vec(),
            ev[first..].to_vec(),
            ob,
            oe,
            by_stage,
            total,
        )
    };

    for lvl in LEVELS {
        let aud = spec.audience(UserGroup::Habituated, Some(lvl));
        let (b, ev, ob, oe, st, total) = pass(&aud);
        level_days.insert(level_name(lvl).to_owned(), (b, ev));
        overrides.insert(level_name(lvl).to_owned(), (ob, oe, st, total));
    }
    for g in [UserGroup::Habituated, UserGroup::Future] {
        let aud = spec.audience(g, None);
        let (b, ev, ob, oe, _, _) = pass(&aud);
        group_days.insert(group_name(g).to_owned(), (b, ev));
        group_overrides.insert(group_name(g).to_owned(), (ob, oe));
    }
    let mut market_days = BTreeMap::new();
    for lvl in LEVELS {
        let mut b = vec![0u64; n_days];
        for e in &births {
            let d = day_of(e);
            if d >= first && tier_passes(lvl, e.story.tier) {
                b[d] += 1;
            }
        }
        market_days.insert(level_name(lvl).to_owned(), b[first..].to_vec());
    }

    let result = build_result(
        name,
        inputs.len(),
        n_days,
        n_days - first,
        births.len() as u64,
        sim.events.len() as u64,
        &level_days,
        &overrides,
        &group_days,
        &group_overrides,
        &market_days,
    );
    Daily {
        result,
        level_days,
        group_days,
        market_days,
        overrides,
        group_overrides,
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn build_result(
    name: &str,
    instruments: usize,
    trading_days: usize,
    counted_days: usize,
    stories: u64,
    events: u64,
    level_days: &BTreeMap<String, (Vec<u64>, Vec<u64>)>,
    overrides: &BTreeMap<String, (u64, u64, BTreeMap<String, u64>, u64)>,
    group_days: &BTreeMap<String, (Vec<u64>, Vec<u64>)>,
    group_overrides: &BTreeMap<String, (u64, u64)>,
    market_days: &BTreeMap<String, Vec<u64>>,
) -> DatasetResult {
    let levels = level_days
        .iter()
        .map(|(k, (b, ev))| {
            let (ob, oe, st, total) = &overrides[k];
            (
                k.clone(),
                LevelResult {
                    births: dist(b),
                    events: dist(ev),
                    override_births: *ob,
                    override_events: *oe,
                    override_events_by_stage: st.clone(),
                    births_total: *total,
                },
            )
        })
        .collect();
    let groups = group_days
        .iter()
        .map(|(g, (b, ev))| {
            let def = if g == "future" { "important" } else { "all" };
            let (ob, oe) = group_overrides[g];
            (
                g.clone(),
                GroupResult {
                    default_level: def.to_owned(),
                    births: dist(b),
                    events: dist(ev),
                    override_births: ob,
                    override_events: oe,
                },
            )
        })
        .collect();
    let market_births = market_days
        .iter()
        .map(|(k, v)| (k.clone(), dist(v)))
        .collect();
    DatasetResult {
        name: name.to_owned(),
        instruments,
        trading_days,
        counted_days,
        stories,
        events,
        levels,
        groups,
        market_births,
    }
}

fn pool(ds: &[Daily]) -> DatasetResult {
    let mut level_days: BTreeMap<String, (Vec<u64>, Vec<u64>)> = BTreeMap::new();
    let mut group_days: BTreeMap<String, (Vec<u64>, Vec<u64>)> = BTreeMap::new();
    let mut market_days: BTreeMap<String, Vec<u64>> = BTreeMap::new();
    let mut overrides: BTreeMap<String, (u64, u64, BTreeMap<String, u64>, u64)> = BTreeMap::new();
    let mut group_overrides: BTreeMap<String, (u64, u64)> = BTreeMap::new();
    for d in ds {
        for (k, (b, ev)) in &d.level_days {
            let e = level_days.entry(k.clone()).or_default();
            e.0.extend(b);
            e.1.extend(ev);
        }
        for (k, (b, ev)) in &d.group_days {
            let e = group_days.entry(k.clone()).or_default();
            e.0.extend(b);
            e.1.extend(ev);
        }
        for (k, v) in &d.market_days {
            market_days.entry(k.clone()).or_default().extend(v);
        }
        for (k, (ob, oe, st, total)) in &d.overrides {
            let e = overrides.entry(k.clone()).or_default();
            e.0 += ob;
            e.1 += oe;
            for (s, n) in st {
                *e.2.entry(s.clone()).or_insert(0) += n;
            }
            e.3 += total;
        }
        for (k, (ob, oe)) in &d.group_overrides {
            let e = group_overrides.entry(k.clone()).or_default();
            e.0 += ob;
            e.1 += oe;
        }
    }
    build_result(
        "pooled",
        ds.iter().map(|d| d.result.instruments).max().unwrap_or(0),
        ds.iter().map(|d| d.result.trading_days).sum(),
        ds.iter().map(|d| d.result.counted_days).sum(),
        ds.iter().map(|d| d.result.stories).sum(),
        ds.iter().map(|d| d.result.events).sum(),
        &level_days,
        &overrides,
        &group_days,
        &group_overrides,
        &market_days,
    )
}

#[derive(Serialize)]
struct Report {
    command: String,
    rules: [String; 3],
    data: String,
    audience: AudienceSpec,
    universe: Vec<String>,
    warmup_bars: usize,
    level_never_changed: bool,
    datasets: Vec<DatasetResult>,
    pooled: DatasetResult,
}

fn f2(x: f64) -> String {
    format!("{x:.2}")
}

fn dist_row(label: &str, d: &Dist) -> String {
    format!(
        "| {label} | {} | {} | {} | {} | {} |\n",
        f2(d.mean),
        d.median,
        d.p90,
        d.max,
        f2(d.zero_share)
    )
}

const HEAD: &str =
    "| | Mean | Median | p90 | Max | Zero-day share |\n|---|---:|---:|---:|---:|---:|\n";

#[allow(clippy::too_many_lines)]
fn markdown(r: &Report) -> String {
    let mut s = String::new();
    let real = r.data == "real";
    let _ = writeln!(
        s,
        "# R4/T3 (K42 sonrası; R3/S-A aracı) — Hassasiyet benzetimi ({})\n",
        if real {
            "gerçek yerel veri, yalnız toplu"
        } else {
            "sentetik"
        }
    );
    let _ = writeln!(
        s,
        "Üreten: `{}` (belirleyici; iki koşu bayt eşit). Motor `EngineConfig::r3()` \
         (tema `{}`, sonuç `{}`, sıralama `{}`), yedi varsayılan dedektör. Tanımlar \
         `crates/qp-capsule/examples/sensitivity.rs` başında.\n",
        r.command, r.rules[0], r.rules[1], r.rules[2]
    );
    let _ = writeln!(
        s,
        "- Veri: {}.",
        r.datasets
            .iter()
            .map(|d| format!(
                "`{}` ({} enstrüman, {} işlem günü, sayılan {})",
                d.name, d.instruments, d.trading_days, d.counted_days
            ))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let _ = writeln!(
        s,
        "- Kitle: izleme listesi {} enstrüman; pozisyon {} ({}); etkin niyet {}; evren {} \
         enstrüman.",
        r.audience.watchlist.len(),
        r.audience.positions.len(),
        r.audience
            .positions
            .iter()
            .map(|(k, v)| format!("{k} {v}"))
            .collect::<Vec<_>>()
            .join(", "),
        r.audience.intents_on.join(", "),
        r.universe.len()
    );
    let _ = writeln!(
        s,
        "- Günlük kapsül = o işlem günü evrende **doğan** ve seviyeden (ya da pozisyon/niyetten) \
         geçen hikâye; ikincil: o gün geçen her hikâye olayı. İlk {} bar (ısınma) sayılmaz.",
        r.warmup_bars
    );
    let _ = writeln!(
        s,
        "- Sistem seviyeyi hiç değiştirmedi (her koşuda kitle önce/sonra karşılaştırıldı, \
         assert): **{}**.\n",
        if r.level_never_changed {
            "evet"
        } else {
            "HAYIR"
        }
    );

    let p = &r.pooled;
    let _ = writeln!(
        s,
        "## Seviye başına günlük kapsül (kitle; {} gün havuz)\n",
        p.counted_days
    );
    s.push_str(HEAD);
    for l in ["important", "balanced", "all"] {
        s.push_str(&dist_row(&format!("`{l}` doğum"), &p.levels[l].births));
    }
    for l in ["important", "balanced", "all"] {
        s.push_str(&dist_row(
            &format!("`{l}` tüm olaylar"),
            &p.levels[l].events,
        ));
    }
    let _ = writeln!(s, "\n## Kullanıcı grubu varsayılanı (seviye seçilmemiş)\n");
    s.push_str(HEAD);
    for g in ["habituated", "future"] {
        let x = &p.groups[g];
        s.push_str(&dist_row(
            &format!("`{g}` → `{}` doğum", x.default_level),
            &x.births,
        ));
    }
    let _ = writeln!(
        s,
        "\n## Pozisyon ve niyet: seviyeden bağımsız geçenler (havuz)\n\n\
         | Seviye | Geçen doğum | Yalnız pozisyon/niyetle geçen doğum | Yalnız pozisyon/niyetle geçen olay | Bunların aşamaları |\n\
         |---|---:|---:|---:|---|"
    );
    for l in ["important", "balanced", "all"] {
        let x = &p.levels[l];
        let st = x
            .override_events_by_stage
            .iter()
            .map(|(k, v)| format!("{k} {v}"))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(
            s,
            "| `{l}` | {} | {} | {} | {} |",
            x.births_total,
            x.override_births,
            x.override_events,
            if st.is_empty() { "—".to_owned() } else { st }
        );
    }
    let _ = writeln!(
        s,
        "\n## Piyasa geneli (kitle yok; M3 karşılaştırması için, havuz)\n"
    );
    s.push_str(HEAD);
    for l in ["important", "balanced", "all"] {
        s.push_str(&dist_row(&format!("`{l}` doğum"), &p.market_births[l]));
    }
    if !real {
        let _ = writeln!(
            s,
            "\n`important` satırı M3'ün bütünleşik ölçümüyle (`M3/RESULTS.md`, `confirm`: \
             1,79 / 2 / 4 / 8, sıfır gün 0,150) aynı tanımdır; aynı sayıları vermesi ürün \
             modüllerinin ölçülenle eş olduğunun bir başka göstergesidir."
        );
    }
    let _ = writeln!(s, "\n## Veri kümesi başına (kitle, doğum)\n");
    let _ = writeln!(
        s,
        "| Veri | Seviye | Ort. | Medyan | p90 | En çok | Sıfır gün | Pozisyon/niyet geçişi |\n|---|---|---:|---:|---:|---:|---:|---:|"
    );
    for d in &r.datasets {
        for l in ["important", "balanced", "all"] {
            let x = &d.levels[l];
            let _ = writeln!(
                s,
                "| `{}` | `{l}` | {} | {} | {} | {} | {} | {} |",
                d.name,
                f2(x.births.mean),
                x.births.median,
                x.births.p90,
                x.births.max,
                f2(x.births.zero_share),
                x.override_births
            );
        }
    }
    if real {
        let _ = writeln!(
            s,
            "\nGerçek veri `fixtures/local/` altında kalır; burada yalnız toplu sayılar var. \
             Gerçek veride akış ailesi yok (kurumsal akış lisanslı, yerelde yok)."
        );
    }
    s
}

fn out_dir() -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../rounds/R4/experiments/T3")
}

fn main() {
    let arg = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "synthetic".to_owned());
    let (datasets, data): (Vec<(String, Vec<InstrumentInput>)>, &str) = match arg.as_str() {
        "synthetic" => (
            SEEDS
                .iter()
                .map(|&s| {
                    (
                        format!("synthetic-30x500-s{s}"),
                        support::synthetic(s, 30, 500),
                    )
                })
                .collect(),
            "synthetic",
        ),
        "real" => {
            let v: Vec<(String, Vec<InstrumentInput>)> = ["1day", "1hour"]
                .iter()
                .filter_map(|p| support::real(p).map(|x| (format!("real-{p}"), x)))
                .collect();
            if v.is_empty() {
                eprintln!(
                    "no local real bars ({} or fixtures/local/bars); nothing written",
                    qp_source::LOCAL_BARS_ENV
                );
                return;
            }
            (v, "real")
        }
        o => {
            eprintln!("unknown data `{o}` (synthetic, real)");
            std::process::exit(2);
        }
    };
    let symbols: Vec<String> = datasets[0].1.iter().map(|x| x.symbol.clone()).collect();
    let spec = audience_for(&symbols);
    let daily: Vec<Daily> = datasets
        .iter()
        .map(|(n, inp)| run_dataset(n, inp, &spec))
        .collect();
    let pooled = pool(&daily);
    let e = qp_capsule::Engine::new(EngineConfig::r3());
    let (a, b, c) = e.rule_names();
    let report = Report {
        command: format!("cargo run --release -p qp-capsule --example sensitivity -- {data}"),
        rules: [a.to_owned(), b.to_owned(), c.to_owned()],
        data: data.to_owned(),
        universe: spec.universe(),
        audience: spec,
        warmup_bars: WARMUP_BARS,
        level_never_changed: true,
        datasets: daily.into_iter().map(|d| d.result).collect(),
        pooled,
    };
    let stem = if data == "real" {
        "sensitivity-real"
    } else {
        "sensitivity"
    };
    let dir = out_dir();
    std::fs::create_dir_all(&dir).expect("out dir");
    let mut js = serde_json::to_string_pretty(&report).expect("json");
    js.push('\n');
    std::fs::write(dir.join(format!("{stem}.json")), js).expect("write json");
    std::fs::write(dir.join(format!("{stem}.md")), markdown(&report)).expect("write md");
    println!("wrote {}/{stem}.{{md,json}}", dir.display());
}
