//! RP4/S: the productized `rp4` record against the measured experiment crates.
//!
//! - Outcome: `qp_capsule::outcome::HighLow` and `Rp4` on intraday bars give, for every
//!   story, the same verdict on the same bar as `m25_yukdus` (reason codes differ only by
//!   their prefix, `highlow.` for `yukdus.`); `Rp4` on daily bars is `Barrier` (`r3`).
//! - Detectors: `qp_facts::rp4_detectors` gives the same detections (ids, versions,
//!   codes, evidence, strength, geometry) as `m26_mevsim::detectors` on 5 min, 15 min,
//!   1 hour and daily bars.
//!
//! Synthetic data only (XIST-aligned random walk, U-shaped volume, occasional wide bars).

use qp_calendar::{builtin, parse_date, ExchangeCalendar, Interval};
use qp_capsule::outcome::{Barrier, HighLow, Rp4};
use qp_capsule::{OpenStory, OutcomeRule, Verdict};
use qp_facts::{run_batch, Detector};
use qp_schema::capsule::Stage;
use qp_schema::{Bar, Fact};

use rp4_harness::registry;
use rp4_harness::sim::{facts_for, run_engine};

const MS_PER_DAY: i64 = 86_400_000;

fn cal() -> ExchangeCalendar {
    ExchangeCalendar::new(&builtin("XIST").unwrap()).unwrap()
}

/// Deterministic LCG in `[0, 1)`.
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        #[allow(clippy::cast_precision_loss)]
        let x = (self.0 >> 11) as f64 / (1u64 << 53) as f64;
        x
    }
}

/// XIST bars of `interval` from `start` for `days` calendar days: random walk with fat
/// wicks now and then (ties of target and stop), U-shaped volume, rare volume surges.
fn bars(interval: Interval, start: &str, days: i64, seed: u64) -> Vec<Bar> {
    let c = cal();
    let from = parse_date(start).unwrap() * MS_PER_DAY;
    let spans = c.bars_between(interval, from, from + days * MS_PER_DAY);
    let intraday = !matches!(interval, Interval::Day);
    let mut r = Lcg(seed);
    let mut p = 100.0;
    let mut out = Vec::with_capacity(spans.len());
    for (i, s) in spans.iter().enumerate() {
        let o = p;
        p *= 1.0 + (r.next() - 0.5) * 0.012;
        let wick = if r.next() < 0.05 { 0.02 } else { 0.003 };
        let (h, l) = (
            o.max(p) * (1.0 + r.next() * wick),
            o.min(p) * (1.0 - r.next() * wick),
        );
        let mut v = 1000.0 * (0.5 + r.next());
        if intraday {
            let span = c.day_span(c.local_day(s.open)).unwrap();
            if s.close >= span.close {
                v *= 8.0;
            } else if s.open == span.open {
                v *= 4.0;
            }
        }
        if i % 97 == 50 {
            v *= 6.0;
        }
        out.push(Bar {
            t: s.open,
            o,
            h,
            l,
            c: p,
            v,
        });
    }
    out
}

/// Per story (capsule id): its transitions as (stage, bar), and its ledger outcome.
type Trace = Vec<(String, Vec<(Stage, u64)>)>;

fn trace(
    bars: &[Bar],
    facts: &[Vec<Fact>],
    rule: Box<dyn OutcomeRule>,
) -> (Trace, Vec<(String, String)>) {
    let e = run_engine("T", "px", bars, facts, rule);
    let mut stories: Vec<&OpenStory> = e
        .closed_stories()
        .iter()
        .chain(e.open_stories().iter())
        .collect();
    stories.sort_by(|a, b| a.id.cmp(&b.id));
    let t = stories
        .iter()
        .map(|s| {
            (
                s.id.clone(),
                s.story.history.iter().map(|h| (h.stage, h.bar)).collect(),
            )
        })
        .collect();
    // Reasons with the rule's prefix dropped (`highlow.` / `yukdus.` / `barrier.`).
    let mut reasons: Vec<(String, String)> = e
        .events()
        .iter()
        .map(|ev| {
            let r = &ev.transition.reason;
            let tail = r.split_once('.').map_or(r.as_str(), |(_, t)| t);
            (ev.capsule.clone(), tail.to_owned())
        })
        .collect();
    reasons.sort();
    (t, reasons)
}

fn closed_by_rule(t: &Trace) -> usize {
    t.iter()
        .filter(|(_, h)| {
            h.last()
                .is_some_and(|(s, _)| matches!(s, Stage::Realized | Stage::Invalidated))
        })
        .count()
}

#[test]
fn highlow_and_rp4_equal_yukdus_intraday() {
    for (iv, days, seed) in [
        (Interval::Minutes(5), 40, 1),
        (Interval::Minutes(15), 90, 2),
        (Interval::Minutes(60), 300, 3),
    ] {
        let b = bars(iv, "2025-11-03", days, seed);
        let facts = facts_for("T", "px", &b, qp_facts::default_detectors, false);
        let (y, yr) = trace(&b, &facts, m25_yukdus::outcome());
        let (h, hr) = trace(&b, &facts, Box::new(HighLow::default()));
        let (p, pr) = trace(&b, &facts, Box::new(Rp4::default()));
        assert!(closed_by_rule(&y) > 50, "{iv:?}: {}", closed_by_rule(&y));
        assert_eq!(y, h, "{iv:?}: HighLow");
        assert_eq!(y, p, "{iv:?}: Rp4");
        assert_eq!(yr, hr, "{iv:?}: reasons");
        assert_eq!(yr, pr, "{iv:?}: reasons");
        // The tie rule is exercised.
        assert!(
            yr.iter().any(|(_, r)| r.starts_with("tie_")),
            "{iv:?}: no tie"
        );
        // And it is not the close rule.
        let (r3, _) = trace(&b, &facts, Box::new(Barrier::default()));
        assert_ne!(y, r3, "{iv:?}");
    }
}

#[test]
fn rp4_equals_r3_daily() {
    let b = bars(Interval::Day, "2018-01-02", 2600, 4);
    let facts = facts_for("T", "px", &b, qp_facts::default_detectors, false);
    let (r3, r3r) = trace(&b, &facts, Box::new(Barrier::default()));
    let (p, pr) = trace(&b, &facts, Box::new(Rp4::default()));
    assert!(closed_by_rule(&r3) > 50);
    assert_eq!(r3, p);
    assert_eq!(r3r, pr);
}

/// Every judgement of every story at every bar up to its horizon, one by one (not only
/// the engine's path): the product and the experiment crate agree.
#[test]
fn judge_and_levels_pointwise() {
    let b = bars(Interval::Minutes(5), "2025-11-03", 30, 9);
    let facts = facts_for("T", "px", &b, qp_facts::default_detectors, false);
    let e = run_engine("T", "px", &b, &facts, Box::new(HighLow::default()));
    let (hl, yk, rp) = (HighLow::default(), m25_yukdus::outcome(), Rp4::default());
    let mut n = 0;
    for s in e.closed_stories().iter().chain(e.open_stories().iter()) {
        let born = usize::try_from(s.story.born_bar).unwrap();
        let ly = m25_yukdus::levels(s, &b[..=born]);
        assert_eq!(hl.levels(s, &b[..=born]), ly);
        assert_eq!(
            registry::rp4_levels(s, &b[..=born]).map(|l| (l.p0, l.target, l.stop)),
            ly
        );
        for now in born..(born + 25).min(b.len()) {
            let w = &b[..=now];
            let y = yk.judge(s, w, now as u64);
            let h = hl.judge(s, w, now as u64);
            assert_eq!(y.map(|x| x.0), h.map(|x| x.0));
            assert_eq!(
                y.map(|x| x.1.trim_start_matches("yukdus.")),
                h.map(|x| x.1.trim_start_matches("highlow."))
            );
            assert_eq!(h, rp.judge(s, w, now as u64));
            n += 1;
        }
    }
    assert!(n > 1000, "{n}");
    let _ = Verdict::Realized;
}

fn detections(make: fn() -> Vec<Box<dyn Detector>>, b: &[Bar]) -> Vec<(&'static str, u32, String)> {
    make()
        .iter_mut()
        .map(|d| {
            let (id, v) = (d.id(), d.version());
            (id, v, format!("{:?}", run_batch(d.as_mut(), b, None)))
        })
        .collect()
}

#[test]
fn rp4_detectors_equal_mevsim() {
    for (iv, start, days, seed) in [
        (Interval::Minutes(5), "2025-11-03", 60, 11),
        (Interval::Minutes(15), "2025-11-03", 120, 12),
        (Interval::Minutes(60), "2025-11-03", 300, 13),
        (Interval::Day, "2018-01-02", 2600, 14),
    ] {
        let b = bars(iv, start, days, seed);
        let m = detections(m26_mevsim::detectors, &b);
        let p = detections(qp_facts::rp4_detectors, &b);
        assert!(m.iter().any(|(_, _, d)| d.len() > 2));
        assert_eq!(m, p, "{iv:?}");
        // Through the engine too (registry entries).
        let fm = facts_for(
            "T",
            "px",
            &b,
            registry::detector_set("mevsim").unwrap().make,
            false,
        );
        let fp = facts_for(
            "T",
            "px",
            &b,
            registry::detector_set("rp4").unwrap().make,
            false,
        );
        assert_eq!(fm, fp, "{iv:?}");
    }
}
