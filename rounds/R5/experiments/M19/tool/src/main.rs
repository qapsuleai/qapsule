//! `m19check` — R5/M19: the product's feed rule against the M19 harness.
//!
//! 1. **Same verdicts:** every named product rule (`qp_capsule::feedrule::FEED_RULES`) gives
//!    the harness rule's verdict (hidden / few cases / shown) on every ledger `(n, r)`,
//!    `0 ≤ r ≤ n ≤ 400`.
//! 2. **The product's ledger:** the harness judges a feed item on day `d` by the ledger as
//!    of the close of `d`; the product judges it by the story's ledger at birth (decided
//!    once, never changes; the face's ledger). Over the same admitted items (harness
//!    `feed_on` with no rule, `LiveNew`) this measures how often the two disagree and the
//!    feed numbers with the product's ledger (kept/day, empty days, T2 visible, hit rate of
//!    kept and hidden births), with and without the K42 exemption as the product applies it
//!    (a story about a holding whose current stage is negative is never hidden).
//!
//! ```text
//! m19check [--real]        # prints a Markdown section (aggregates only)
//! ```

use std::collections::BTreeSet;
use std::fmt::Write as _;

use feedh::{feed_on, prepare, real, synthetic, Config, Order, Prepared, Rule, Verdict};
use qp_capsule::feedrule::{FeedVerdict, HideRule, FEED_RULES};
use qp_schema::capsule::{LedgerStat, Stage};

fn harness_rule(name: &str) -> Rule {
    match name {
        "rate55" => Rule::Rate {
            min_n: 20,
            tau: 0.55,
        },
        "rate55n10" => Rule::Rate {
            min_n: 10,
            tau: 0.55,
        },
        "none" => Rule::None,
        other => panic!("no harness rule for {other}"),
    }
}

/// The product's rules, then the harness's runner-up (not in the product: its `min_n` is
/// below the product's few-cases threshold, 20).
fn rules() -> Vec<(&'static str, HideRule)> {
    let mut v: Vec<(&'static str, HideRule)> = FEED_RULES.to_vec();
    v.push((
        "rate55n10",
        HideRule::Rate {
            min_n: 10,
            tau: 0.55,
        },
    ));
    v
}

fn same_verdicts() -> String {
    let mut t = String::new();
    for (name, hide) in rules() {
        let h = harness_rule(name);
        let mut cases = 0u32;
        let mut bad = 0u32;
        for n in 0..=400u32 {
            for r in 0..=n {
                cases += 1;
                let pv = hide.verdict(LedgerStat { n, realized: r });
                let hv = h.judge(n, r);
                let ok = match h {
                    // The harness's reference only labels few cases; neither hides.
                    Rule::None => pv != FeedVerdict::CoinFlip && hv != Verdict::Hidden,
                    _ => {
                        (pv == FeedVerdict::CoinFlip) == (hv == Verdict::Hidden)
                            && (pv == FeedVerdict::FewCases) == (hv == Verdict::Thin)
                    }
                };
                if !ok {
                    bad += 1;
                }
            }
        }
        let _ = writeln!(
            t,
            "| `{name}`{} | `{}` | {cases} | {bad} |",
            if FEED_RULES.iter().any(|(n, _)| *n == name) { "" } else { " (üründe yok)" },
            h.label()
        );
        assert_eq!(bad, 0, "{name}: verdicts differ from the harness");
    }
    t
}

#[derive(Default)]
struct M {
    days: u32,
    kept: u64,
    hidden: u64,
    empty: u32,
    disagree: u64,
    items: u64,
    t2: (u32, u32),
    kept_hit: (u32, u32),
    hidden_hit: (u32, u32),
}

fn rate(x: (u32, u32)) -> String {
    if x.1 == 0 {
        "—".into()
    } else {
        format!("{:.3} ({}/{})", f64::from(x.0) / f64::from(x.1), x.0, x.1)
    }
}

fn measure(p: &Prepared, hide: HideRule, harness: Rule, exempt: bool) -> M {
    let hold: BTreeSet<String> = p.spec.holdings();
    let all = Config {
        rule: Rule::None,
        order: Order::LiveNew,
        exempt: false,
    };
    let mut m = M::default();
    for d in p.first_day..p.n_days {
        m.days += 1;
        let mut vis = 0u64;
        for it in feed_on(p, d, &all) {
            let c = &p.caps[it.cap];
            let Some(st) = c.at(d) else { continue };
            let held = hold.contains(&c.instrument);
            let pv = hide.verdict(st.story.ledger_at_birth);
            let ex = exempt && held && st.story.stage.is_negative();
            let hidden = pv == FeedVerdict::CoinFlip && !ex;
            let hv = harness.judge(it.ledger.0, it.ledger.1) == Verdict::Hidden;
            m.items += 1;
            if (pv == FeedVerdict::CoinFlip) != hv {
                m.disagree += 1;
            }
            if hidden {
                m.hidden += 1;
            } else {
                vis += 1;
            }
            if held && st.moved.iter().any(|s| s.is_negative()) {
                m.t2.1 += 1;
                if !hidden {
                    m.t2.0 += 1;
                }
            }
            if c.born_day == d && st.moved.contains(&Stage::Born) {
                if let Some((_, Some(hit))) = c.close {
                    let h = u32::from(hit);
                    if hidden {
                        m.hidden_hit.0 += h;
                        m.hidden_hit.1 += 1;
                    } else if pv == FeedVerdict::Shown {
                        m.kept_hit.0 += h;
                        m.kept_hit.1 += 1;
                    }
                }
            }
        }
        m.kept += vis;
        if vis == 0 {
            m.empty += 1;
        }
    }
    m
}

fn section(title: &str, ps: &[Prepared]) -> String {
    let mut t = String::new();
    let _ = writeln!(t, "### {title}\n");
    let _ = writeln!(t, "| ürün kuralı | K42 muaf | kalan/gün | gizli/gün | boş gün | kalanların tutma oranı | gizlenenlerinki | T2 görünür | gün sonu karnesiyle ayrışan karar payı |");
    let _ = writeln!(t, "|---|---|---:|---:|---:|---:|---:|---:|---:|");
    for (name, hide) in rules() {
        for exempt in [false, true] {
            let mut a = M::default();
            for p in ps {
                let m = measure(p, hide, harness_rule(name), exempt);
                a.days += m.days;
                a.kept += m.kept;
                a.hidden += m.hidden;
                a.empty += m.empty;
                a.disagree += m.disagree;
                a.items += m.items;
                a.t2.0 += m.t2.0;
                a.t2.1 += m.t2.1;
                a.kept_hit.0 += m.kept_hit.0;
                a.kept_hit.1 += m.kept_hit.1;
                a.hidden_hit.0 += m.hidden_hit.0;
                a.hidden_hit.1 += m.hidden_hit.1;
            }
            let days = f64::from(a.days.max(1));
            #[allow(clippy::cast_precision_loss)]
            let _ = writeln!(
                t,
                "| `{name}` | {} | {:.2} | {:.2} | {:.3} | {} | {} | {} | {:.4} ({}/{}) |",
                if exempt { "evet" } else { "hayır" },
                a.kept as f64 / days,
                a.hidden as f64 / days,
                f64::from(a.empty) / days,
                rate(a.kept_hit),
                rate(a.hidden_hit),
                rate(a.t2),
                a.disagree as f64 / (a.items.max(1)) as f64,
                a.disagree,
                a.items
            );
        }
    }
    t
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut md = String::new();
    let _ = writeln!(md, "## Ürün kuralı ↔ düzenek (`m19check`)\n");
    let _ = writeln!(
        md,
        "### 1. Aynı kararlar (her karne `0 ≤ r ≤ n ≤ 400`)\n\n| ürün adı | düzenek kuralı | karne | ayrışan |\n|---|---|---:|---:|"
    );
    md.push_str(&same_verdicts());
    md.push('\n');
    let syn: Vec<Prepared> = [1u64, 2, 3]
        .iter()
        .map(|s| prepare(&format!("syn-s{s}"), &synthetic(*s, 30, 500)))
        .collect();
    md.push_str(&section(
        "2a. Doğum karnesiyle akış — sentetik (3 tohum × 30 × 500 gün; `LiveNew`, seviye `balanced`)",
        &syn,
    ));
    if args.iter().any(|a| a == "--real") {
        if let Some(inputs) = real() {
            md.push('\n');
            md.push_str(&section(
                "2b. Doğum karnesiyle akış — gerçek günlük veri (yalnız toplu istatistik)",
                &[prepare("real", &inputs)],
            ));
        } else {
            md.push_str("\nGerçek veri yok.\n");
        }
    }
    println!("{md}");
}
