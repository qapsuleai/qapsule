//! `feedh` — R5/T2 M19 feed simulation (see `../README.md`, `../SPEC.md`).
//!
//! ```text
//! feedh run --out <dir> [--seeds 1,2,3] [--days 500] [--n 30] [--real] [--json out/feed.json]
//! ```
//! Writes `<dir>/RESULTS.md` (and with `--json <path>` the full metrics; aggregate statistics only; with
//! `--real` the local real bars are used too — still aggregates only).

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::PathBuf;

use feedh::{
    measure, prepare, real, rule_grid, synthetic, Config, Metrics, Order, Prepared, Rule, ORDERS,
};
use serde::Serialize;

fn arg(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

/// SPEC §4 gate constants.
const MAX_EMPTY: f64 = 0.25;
const MIN_KEPT: f64 = 1.0;
/// SPEC §4: ordering tie margin.
const ORDER_TIE: f64 = 0.02;
/// SPEC §4: bootstrap seed of the skill interval.
const BOOT_SEED: u64 = 1;

#[derive(Debug, Clone, Serialize)]
struct Pooled {
    label: String,
    order: String,
    exempt: bool,
    params: u32,
    min_n: u32,
    per_dataset: Vec<Metrics>,
    kept_per_day: f64,
    rows_per_day: f64,
    hidden_per_day: f64,
    empty_share: f64,
    thin_share: f64,
    kept_hit: (u32, u32),
    hidden_hit: (u32, u32),
    thin_hit: (u32, u32),
    kept_skill: f64,
    kept_skill_range: (f64, f64),
    hidden_skill: f64,
    t1_visible: f64,
    t1_top: f64,
    t2_visible: f64,
    t2_top: f64,
}

fn r(x: (u32, u32)) -> f64 {
    if x.1 == 0 {
        0.0
    } else {
        f64::from(x.0) / f64::from(x.1)
    }
}

fn pool(cfg: &Config, ms: Vec<Metrics>) -> Pooled {
    let n = ms.len().max(1) as f64;
    let mean = |f: fn(&Metrics) -> f64| ms.iter().map(f).sum::<f64>() / n;
    let sum = |f: fn(&Metrics) -> (u32, u32)| {
        ms.iter().map(f).fold((0, 0), |a, b| (a.0 + b.0, a.1 + b.1))
    };
    let pairs: Vec<(f64, f64)> = ms
        .iter()
        .flat_map(|m| m.kept_pairs.iter().copied())
        .collect();
    Pooled {
        label: cfg.rule.label(),
        order: format!("{:?}", cfg.order),
        exempt: cfg.exempt,
        params: cfg.rule.params(),
        min_n: cfg.rule.min_n(),
        kept_per_day: mean(|m| m.kept_per_day),
        rows_per_day: mean(|m| m.rows_per_day),
        hidden_per_day: mean(|m| m.hidden_per_day),
        empty_share: mean(|m| m.empty_share),
        thin_share: mean(|m| m.thin_share),
        kept_hit: sum(|m| m.kept_hit),
        hidden_hit: sum(|m| m.hidden_hit),
        thin_hit: sum(|m| m.thin_hit),
        kept_skill: feedh::skill(&pairs),
        kept_skill_range: feedh::skill_ci(&pairs, 1000, BOOT_SEED),
        hidden_skill: mean(|m| m.hidden_skill),
        t1_visible: r(sum(|m| m.t1_visible)),
        t1_top: r(sum(|m| m.t1_top)),
        t2_visible: r(sum(|m| m.t2_visible)),
        t2_top: r(sum(|m| m.t2_top)),
        per_dataset: ms,
    }
}

fn run_all(ps: &[Prepared]) -> Vec<(Config, Pooled)> {
    let mut out = Vec::new();
    for rule in rule_grid() {
        for order in ORDERS {
            for exempt in [false, true] {
                let cfg = Config {
                    rule,
                    order,
                    exempt,
                };
                let ms: Vec<Metrics> = ps.iter().map(|p| measure(p, &cfg)).collect();
                out.push((cfg, pool(&cfg, ms)));
            }
        }
    }
    out
}

fn hit(x: (u32, u32)) -> String {
    format!("{:.3} ({}/{})", r(x), x.0, x.1)
}

#[allow(clippy::too_many_lines)]
fn report(title: &str, ps: &[Prepared], res: &[(Config, Pooled)], select: bool) -> String {
    let mut t = String::new();
    let _ = writeln!(t, "## {title}\n");
    for p in ps {
        let _ = writeln!(t, "- `{}`: {} gün (sayılan {}), {} kapsül; evren {} enstrüman, elde {} (pozisyon + niyet); seviye `balanced`.", p.name, p.n_days, p.n_days - p.first_day, p.caps.len(), p.spec.universe().len(), p.spec.holdings().len());
    }
    let base: Vec<&(Config, Pooled)> = res
        .iter()
        .filter(|(c, _)| c.order == Order::LiveNew && !c.exempt)
        .collect();
    let _ = writeln!(
        t,
        "\n### Gizleme kuralları (sıralama `LiveNew`, K42 muafiyeti yok)\n"
    );
    let _ = writeln!(t, "| kural | kalan/gün | gizli/gün | boş gün | az örnekli payı | kalanların tutma oranı | gizlenenlerinki | az örneklilerinki | kalan Brier becerisi [önyükleme %95] | gizlenen Brier becerisi | T2 görünür |");
    let _ = writeln!(t, "|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|");
    for (_, p) in &base {
        let _ = writeln!(
            t,
            "| {} | {:.2} | {:.2} | {:.3} | {:.3} | {} | {} | {} | {:.3} [{:.3}, {:.3}] | {:.3} | {:.3} |",
            p.label, p.kept_per_day, p.hidden_per_day, p.empty_share, p.thin_share, hit(p.kept_hit), hit(p.hidden_hit), hit(p.thin_hit), p.kept_skill, p.kept_skill_range.0, p.kept_skill_range.1, p.hidden_skill, p.t2_visible
        );
    }
    // N sensitivity.
    let _ = writeln!(
        t,
        "\n### Eşiğin örnek sayısına (N) duyarlılığı: kalan/gün · kalan tutma · kalan beceri\n"
    );
    let fam = |l: &str| l.split(" N=").next().unwrap_or(l).to_owned();
    let mut fams: BTreeMap<String, BTreeMap<u32, String>> = BTreeMap::new();
    for (c, p) in &base {
        if matches!(c.rule, Rule::None) {
            continue;
        }
        fams.entry(fam(&p.label)).or_default().insert(
            c.rule.min_n(),
            format!(
                "{:.2} · {:.3} · {:.3}",
                p.kept_per_day,
                r(p.kept_hit),
                p.kept_skill
            ),
        );
    }
    let _ = writeln!(
        t,
        "| kural | N=5 | N=10 | N=20 | N=30 |\n|---|---|---|---|---|"
    );
    for (f, row) in &fams {
        let cells: Vec<String> = [5, 10, 20, 30]
            .iter()
            .map(|n| row.get(n).cloned().unwrap_or_default())
            .collect();
        let _ = writeln!(t, "| {f} | {} |", cells.join(" | "));
    }
    if !select {
        return t;
    }
    // Selection (SPEC §4).
    let _ = writeln!(t, "\n### Seçim kuralının uygulanışı (SPEC §4)\n");
    let cands: Vec<&(Config, Pooled)> = base
        .iter()
        .copied()
        .filter(|(c, _)| !matches!(c.rule, Rule::None))
        .collect();
    let feasible: Vec<&(Config, Pooled)> = cands
        .iter()
        .copied()
        .filter(|(_, p)| p.empty_share <= MAX_EMPTY && p.kept_per_day >= MIN_KEPT)
        .collect();
    let _ = writeln!(
        t,
        "1. Kapı (boş gün ≤ {MAX_EMPTY}, kalan/gün ≥ {MIN_KEPT}): {} / {} kural geçti.",
        feasible.len(),
        cands.len()
    );
    let pool_: Vec<&(Config, Pooled)> = if feasible.is_empty() {
        let _ = writeln!(
            t,
            "   Hiçbiri geçmedi → kapı uygulanmadı (SPEC §4, E-kuralı gibi yazılır)."
        );
        cands.clone()
    } else {
        feasible
    };
    let mut ranked = pool_.clone();
    ranked.sort_by(|a, b| {
        b.1.kept_skill
            .total_cmp(&a.1.kept_skill)
            .then(r(b.1.kept_hit).total_cmp(&r(a.1.kept_hit)))
            .then(a.1.params.cmp(&b.1.params))
            .then(b.1.min_n.cmp(&a.1.min_n))
    });
    if ranked.len() >= 2 {
        let (a, b) = (&ranked[0].1, &ranked[1].1);
        let same = a.kept_skill_range.0 <= b.kept_skill_range.1
            && b.kept_skill_range.0 <= a.kept_skill_range.1;
        let _ = writeln!(t, "2. Birincil (kalan Brier becerisi): `{}` {:.3} [{:.3}, {:.3}] ↔ `{}` {:.3} [{:.3}, {:.3}] → {}.", a.label, a.kept_skill, a.kept_skill_range.0, a.kept_skill_range.1, b.label, b.kept_skill, b.kept_skill_range.0, b.kept_skill_range.1, if same { "önyükleme aralıkları örtüşüyor: **ayırt edilemedi** → eşitlik bozucular (kalan tutma oranı, az parametre, büyük N)" } else { "ayrıldı" });
        // Among those indistinguishable from the top: tie-breaks.
        let top = a.clone();
        let mut tied: Vec<&(Config, Pooled)> = ranked
            .iter()
            .copied()
            .filter(|(_, p)| {
                p.kept_skill_range.0 <= top.kept_skill_range.1
                    && top.kept_skill_range.0 <= p.kept_skill_range.1
            })
            .collect();
        tied.sort_by(|x, y| {
            r(y.1.kept_hit)
                .total_cmp(&r(x.1.kept_hit))
                .then(x.1.params.cmp(&y.1.params))
                .then(y.1.min_n.cmp(&x.1.min_n))
        });
        let pick = tied[0];
        let second = tied.get(1).copied().unwrap_or(ranked[1]);
        let _ = writeln!(t, "3. Ayırt edilemeyen {} kural içinde eşitlik bozucu sırası: **`{}`** (kalan tutma {:.3}), ikinci `{}` ({:.3}).", tied.len(), pick.1.label, r(pick.1.kept_hit), second.1.label, r(second.1.kept_hit));
        // Ordering under the pick.
        let _ = writeln!(t, "\n### Sıralama ve K42 (kural `{}`)\n", pick.1.label);
        let _ = writeln!(t, "| sıralama | K42 muaf | satır/gün | kalan/gün | T1 görünür | T1 ilk {} | T2 görünür | T2 ilk {} |", feedh::TOP_K, feedh::TOP_K);
        let _ = writeln!(t, "|---|---|---:|---:|---:|---:|---:|---:|");
        let rows: Vec<&(Config, Pooled)> =
            res.iter().filter(|(c, _)| c.rule == pick.0.rule).collect();
        for (c, p) in &rows {
            let _ = writeln!(
                t,
                "| {:?} | {} | {:.2} | {:.2} | {:.3} | {:.3} | {:.3} | {:.3} |",
                c.order,
                if c.exempt { "evet" } else { "hayır" },
                p.rows_per_day,
                p.kept_per_day,
                p.t1_visible,
                p.t1_top,
                p.t2_visible,
                p.t2_top
            );
        }
        let non = rows
            .iter()
            .find(|(c, _)| c.order == Order::LiveNew && !c.exempt)
            .map(|x| x.1.t2_visible)
            .unwrap_or(1.0);
        let exempt = non < 1.0;
        let _ = writeln!(
            t,
            "\n4. K42: muafiyetsiz T2 görünür = {non:.3} → öneri: {}.",
            if exempt {
                "**olumsuz geçiş gizlemeden muaf** (gizleme eldeki hissenin olumsuz geçişini kaçırıyor)"
            } else {
                "muafiyet gerekmiyor (gizleme bu veride eldeki olumsuz geçişi kaçırmadı); karar Arkin'in"
            }
        );
        let mut ords: Vec<&(Config, Pooled)> = rows
            .iter()
            .copied()
            .filter(|(c, _)| c.exempt == exempt)
            .collect();
        let score = |p: &Pooled| (p.t1_top + p.t2_top) / 2.0;
        ords.sort_by(|a, b| score(&b.1).total_cmp(&score(&a.1)));
        let best = ords[0];
        let live = ords
            .iter()
            .find(|(c, _)| c.order == Order::LiveNew)
            .copied()
            .unwrap_or(best);
        let chosen = if score(&best.1) - score(&live.1) < ORDER_TIE {
            live
        } else {
            best
        };
        let _ = writeln!(t, "5. Sıralama (T1 ve T2 ilk {} ortalaması; fark < {ORDER_TIE} ise `LiveNew`): en iyi `{:?}` {:.3}, `LiveNew` {:.3} → **`{:?}`**.", feedh::TOP_K, best.0.order, score(&best.1), score(&live.1), chosen.0.order);
        let _ = writeln!(
            t,
            "6. İlk iki (gösterimde seçilebilir): `{}` + `{:?}`, `{}`.",
            pick.1.label, chosen.0.order, second.1.label
        );
    }
    t
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) != Some("run") {
        eprintln!("usage: feedh run --out <dir> [--seeds 1,2,3] [--days 500] [--n 30] [--real]");
        std::process::exit(2);
    }
    let out = PathBuf::from(arg(&args, "--out").unwrap_or_else(|| "out/feed".into()));
    let seeds: Vec<u64> = arg(&args, "--seeds")
        .unwrap_or_else(|| "1,2,3".into())
        .split(',')
        .filter_map(|s| s.parse().ok())
        .collect();
    let days: usize = arg(&args, "--days")
        .and_then(|x| x.parse().ok())
        .unwrap_or(500);
    let n: usize = arg(&args, "--n").and_then(|x| x.parse().ok()).unwrap_or(30);
    let syn: Vec<Prepared> = seeds
        .iter()
        .map(|s| prepare(&format!("syn-s{s}"), &synthetic(*s, n, days)))
        .collect();
    let res = run_all(&syn);
    let mut md = String::new();
    let _ = writeln!(md, "# M19 kuru koşu — akış önceliği ve gizleme eşiği\n");
    let _ = writeln!(md, "Üretildi: `feedh run` (deterministik). Benzetim geleceğe bakmaz (SPEC §1; test `no_look_ahead`); gizlenenler karneye işler (test `hidden_still_count_in_ledger`). Tutma oranları kapsülün doğduğu günkü karara göre, sonuç kapanışta (değerlendirme; karar değil).\n");
    let have_real = args.iter().any(|a| a == "--real") && real().is_some();
    md.push_str(&report(
        &format!(
            "Sentetik ({} tohum × {n} enstrüman × {days} gün)",
            seeds.len()
        ),
        &syn,
        &res,
        !have_real,
    ));
    let mut json =
        serde_json::json!({ "synthetic": res.iter().map(|(_, p)| p).collect::<Vec<_>>() });
    if args.iter().any(|a| a == "--real") {
        match real() {
            Some(inputs) => {
                let pr = vec![prepare("real", &inputs)];
                let rr = run_all(&pr);
                md.push('\n');
                md.push_str(&report("Gerçek günlük veri (`fixtures/local/`; yalnız toplu istatistik; seçim burada, SPEC §4)", &pr, &rr, true));
                json["real"] = serde_json::json!(rr.iter().map(|(_, p)| p).collect::<Vec<_>>());
            }
            None => {
                let _ = writeln!(
                    md,
                    "\nGerçek veri yok (`fixtures/local/bars`); yalnız sentetik."
                );
            }
        }
    }
    std::fs::create_dir_all(&out).expect("mkdir");
    std::fs::write(out.join("RESULTS.md"), &md).expect("write");
    // The full metrics (large; aggregates only) go to `--json <path>` when asked.
    if let Some(j) = arg(&args, "--json") {
        std::fs::write(j, serde_json::to_string_pretty(&json).expect("json")).expect("write");
    }
    println!("{md}");
}
