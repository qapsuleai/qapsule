//! (RP4) The `rp4` detector set (`qp_facts::rp4_detectors`): the `r3` detections where
//! the season is inactive (daily, below one hour), batch = streaming, no look-ahead,
//! slots, and the seasonal effect on a U-shaped synthetic day. Ported from
//! `rounds/RP4/experiments/M26/mevsim/tests/mevsim.rs` (the bit-identity with that crate
//! itself is checked in `rounds/RP4/experiments/harness/tests/rp4_product.rs`).

use qp_calendar::{builtin, parse_date, ExchangeCalendar, Interval, XIST_HALF_DAYS};
use qp_facts::season::{Season, CLOSE};
use qp_facts::{
    run_batch, BandSqueeze, Detection, Detector, RangeBreakout, SeasonalBreakout,
    SeasonalVolumeSurge, VolumeSurge, SEASON_DAYS, SEASON_MIN_DAYS,
    SEASON_MIN_LEN_MS as MIN_LEN_MS,
};
use qp_schema::Bar;

/// The seasonal volume surge with period floor `min_len_ms`.
fn volume_surge(min_len_ms: i64) -> Box<dyn Detector> {
    Box::new(
        SeasonalVolumeSurge::new(
            VolumeSurge::WINDOW,
            VolumeSurge::MULT,
            SEASON_DAYS,
            SEASON_MIN_DAYS,
            min_len_ms,
        )
        .unwrap()
        .detector(),
    )
}

/// The seasonal range breakout with period floor `min_len_ms`.
fn range_breakout(min_len_ms: i64) -> Box<dyn Detector> {
    Box::new(
        SeasonalBreakout::new(
            RangeBreakout::WINDOW,
            RangeBreakout::ATR,
            SEASON_DAYS,
            SEASON_MIN_DAYS,
            min_len_ms,
        )
        .unwrap()
        .detector(),
    )
}

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

/// Synthetic bars of `interval` from 2025-11-03 for `days` calendar days: random walk,
/// U-shaped volume (`u_shape`) with occasional real surges.
fn bars(interval: Interval, days: i64, u_shape: bool, seed: u64) -> Vec<Bar> {
    let c = cal();
    let d0 = parse_date("2025-11-03").unwrap();
    let from = d0 * MS_PER_DAY;
    let spans = c.bars_between(interval, from, from + days * MS_PER_DAY);
    let mut r = Lcg(seed);
    let mut p = 100.0;
    let mut out = Vec::with_capacity(spans.len());
    for (i, s) in spans.iter().enumerate() {
        let day = c.local_day(s.open);
        let span = c.day_span(day).unwrap();
        let last = s.close >= span.close;
        let first = s.open == span.open;
        let o = p;
        p *= 1.0 + (r.next() - 0.5) * 0.01;
        let (h, l) = (
            o.max(p) * (1.0 + r.next() * 0.002),
            o.min(p) * (1.0 - r.next() * 0.002),
        );
        let mut v = 1000.0 * (0.5 + r.next());
        if u_shape {
            if last {
                v *= 8.0;
            } else if first {
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

fn daily_bars(n: i64, seed: u64) -> Vec<Bar> {
    let c = cal();
    let from = parse_date("2018-01-02").unwrap() * MS_PER_DAY;
    let spans = c.bars_between(Interval::Day, from, from + n * MS_PER_DAY);
    let mut r = Lcg(seed);
    let mut p = 50.0;
    spans
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let o = p;
            p *= 1.0 + (r.next() - 0.5) * 0.06;
            Bar {
                t: s.open,
                o,
                h: o.max(p) * 1.01,
                l: o.min(p) * 0.99,
                c: p,
                v: 1e6 * (0.5 + r.next()) * if i % 41 == 7 { 5.0 } else { 1.0 },
            }
        })
        .collect()
}

fn run(det: &mut dyn Detector, b: &[Bar]) -> Vec<(u64, Detection)> {
    run_batch(det, b, None)
}

#[test]
fn daily_is_product_bit_identical() {
    let b = daily_bars(2600, 7);
    let vp = run(&mut VolumeSurge::default().detector(), &b);
    let vm = run(volume_surge(MIN_LEN_MS).as_mut(), &b);
    assert!(!vp.is_empty());
    assert_eq!(vp, vm);
    // Even with no period floor (trials): daily bars are never intraday.
    assert_eq!(vp, run(volume_surge(0).as_mut(), &b));
    let bp = run(&mut RangeBreakout::default().detector(), &b);
    assert!(!bp.is_empty());
    assert_eq!(bp, run(range_breakout(MIN_LEN_MS).as_mut(), &b));
    assert_eq!(bp, run(range_breakout(0).as_mut(), &b));
}

#[test]
fn below_floor_is_product_bit_identical() {
    for iv in [Interval::Minutes(5), Interval::Minutes(15)] {
        let b = bars(iv, 60, true, 3);
        let vp = run(&mut VolumeSurge::default().detector(), &b);
        assert!(!vp.is_empty());
        assert_eq!(vp, run(volume_surge(MIN_LEN_MS).as_mut(), &b));
        let bp = run(&mut RangeBreakout::default().detector(), &b);
        assert_eq!(bp, run(range_breakout(MIN_LEN_MS).as_mut(), &b));
    }
}

#[test]
fn active_on_hourly_and_ids_versions() {
    let b = bars(Interval::Minutes(60), 120, true, 5);
    let vp = run(&mut VolumeSurge::default().detector(), &b);
    let vm = run(volume_surge(MIN_LEN_MS).as_mut(), &b);
    assert_ne!(vp, vm, "the season changes hourly detections");
    assert!(vm.iter().any(|(_, d)| d.evidence.contains_key("season")));
    let d = volume_surge(MIN_LEN_MS);
    assert_eq!((d.id(), d.version()), ("flow.volume_surge", 2));
    let d = range_breakout(MIN_LEN_MS);
    assert_eq!((d.id(), d.version()), ("breakout.range", 2));
    let ids: Vec<_> = qp_facts::rp4_detectors().iter().map(|d| d.id()).collect();
    let product: Vec<_> = qp_facts::default_detectors()
        .iter()
        .map(|d| d.id())
        .collect();
    assert_eq!(ids, product);
}

/// U-shaped volume: the product fires on the closing bar most days; the seasonal
/// detector learns the slot and stops (after its warm-up).
#[test]
fn seasonal_volume_drops_closing_births() {
    let c = cal();
    let b = bars(Interval::Minutes(5), 90, true, 11);
    let closing = |v: &[(u64, Detection)]| {
        v.iter()
            .filter(|(i, _)| {
                let t = b[*i as usize].t;
                // after 20 trading days (profile warm)
                t > b[0].t + 30 * MS_PER_DAY
                    && t + 300_000 >= c.day_span(c.local_day(t)).unwrap().close
            })
            .count()
    };
    let vp = run(&mut VolumeSurge::default().detector(), &b);
    let vm = run(volume_surge(0).as_mut(), &b);
    let (np, nm) = (closing(&vp), closing(&vm));
    assert!(np > 30, "product fires at the close: {np}");
    assert!(nm * 10 < np, "seasonal: {nm} vs product {np}");
    // Real surges (every 97th bar ×6) are still found midday.
    assert!(vm.len() > 20);
}

fn streaming_equals_batch(make: &dyn Fn() -> Box<dyn Detector>, b: &[Bar]) {
    let batch = run(make().as_mut(), b);
    let mut det = make();
    let mut out = Vec::new();
    let mut r = Lcg(99);
    for (i, bar) in b.iter().enumerate() {
        // A revision of the pending bar first, then the final bar.
        let mut rev = bar.clone();
        rev.v *= 0.3 + r.next();
        rev.c = rev.c.clamp(rev.l, rev.h);
        det.push(&rev, None);
        for d in det.update_last(bar, None) {
            out.push((i as u64, d));
        }
    }
    assert_eq!(batch, out);
}

#[test]
fn batch_equals_streaming_with_revisions() {
    let b = bars(Interval::Minutes(60), 80, true, 21);
    streaming_equals_batch(&|| volume_surge(MIN_LEN_MS), &b);
    streaming_equals_batch(&|| range_breakout(MIN_LEN_MS), &b);
    let b = bars(Interval::Minutes(5), 30, true, 22);
    streaming_equals_batch(&|| volume_surge(0), &b);
    streaming_equals_batch(&|| range_breakout(0), &b);
}

#[test]
fn no_lookahead_and_deterministic() {
    let b = bars(Interval::Minutes(60), 120, true, 31);
    let cut = b.len() / 2;
    let mut changed = b.clone();
    for x in &mut changed[cut..] {
        x.v *= 7.0;
        x.h *= 1.05;
        x.c *= 1.02;
        x.h = x.h.max(x.c);
    }
    for make in [
        &(|| volume_surge(MIN_LEN_MS)) as &dyn Fn() -> Box<dyn Detector>,
        &|| range_breakout(MIN_LEN_MS),
    ] {
        let a = run(make().as_mut(), &b);
        let z = run(make().as_mut(), &changed);
        let before = |v: &[(u64, Detection)]| {
            v.iter()
                .filter(|(i, _)| (*i as usize) < cut)
                .cloned()
                .collect::<Vec<_>>()
        };
        assert_eq!(before(&a), before(&z));
        assert_eq!(a, run(make().as_mut(), &b));
    }
}

#[test]
fn slots_close_and_half_day() {
    let c = cal();
    let mut s = Season::new(20, 5, 0);
    // Not intraday before two bars of one day.
    let day = parse_date("2025-11-04").unwrap();
    let open = c.day_span(day).unwrap().open;
    let p = s.locate(open);
    assert!(!p.active());
    assert_eq!(p.slot(), None);
    s.commit(&p, 1.0);
    let p = s.locate(open + 300_000);
    assert!(p.active());
    assert_eq!(p.slot(), Some(5));
    s.commit(&p, 1.0);
    // Last 5 min bar (17:55, merged closing auction) is the closing slot.
    let close = c.day_span(day).unwrap().close;
    assert_eq!(s.locate(close - 300_000).slot(), Some(CLOSE));
    assert_eq!(s.locate(close - 600_000).slot(), Some(470));
    // A half day's last bar is the closing slot too.
    let half = XIST_HALF_DAYS
        .iter()
        .map(|d| parse_date(d).unwrap())
        .find(|&d| d > day)
        .unwrap();
    let hc = c.day_span(half).unwrap().close;
    let mut s2 = s.clone();
    let p = s2.locate(hc - 600_000);
    s2.commit(&p, 1.0);
    assert_eq!(s2.locate(hc - 300_000).slot(), Some(CLOSE));
    // Floor: 5 min bars below a one hour floor are inactive, factor exactly 1.
    let s3 = Season::new(20, 5, MIN_LEN_MS);
    assert_eq!(s3.factor(&s3.locate(open)), 1.0);
}

#[test]
fn unchanged_detectors_are_the_products() {
    // The set reuses the product engines for the other five ids.
    let b = bars(Interval::Minutes(60), 100, true, 41);
    let set = qp_facts::rp4_detectors();
    let squeeze = set
        .iter()
        .position(|d| d.id() == "squeeze.bandwidth")
        .unwrap();
    let mut a = qp_facts::rp4_detectors();
    let x = run(a[squeeze].as_mut(), &b);
    assert!(!x.is_empty());
    assert_eq!(x, run(&mut BandSqueeze::default().detector(), &b));
}

/// The whole `rp4` set against the `r3` set: same ids in the same order; version 2 only
/// for the two seasonal detectors; bit-identical detections on daily, 5 and 15 minute
/// bars; on hourly bars only the two seasonal detectors differ.
#[test]
fn rp4_set_against_r3_set() {
    let r3 = qp_facts::default_detectors();
    let rp4 = qp_facts::rp4_detectors();
    let ids = |v: &[Box<dyn Detector>]| v.iter().map(|d| d.id()).collect::<Vec<_>>();
    assert_eq!(ids(&r3), ids(&rp4));
    for (a, b) in r3.iter().zip(&rp4) {
        let want = if matches!(a.id(), "breakout.range" | "flow.volume_surge") {
            2
        } else {
            a.version()
        };
        assert_eq!(b.version(), want, "{}", a.id());
    }
    let all = |make: fn() -> Vec<Box<dyn Detector>>, b: &[Bar]| {
        make()
            .iter_mut()
            .map(|d| run(d.as_mut(), b))
            .collect::<Vec<_>>()
    };
    for b in [
        daily_bars(2600, 7),
        bars(Interval::Minutes(5), 60, true, 3),
        bars(Interval::Minutes(15), 90, true, 4),
    ] {
        let x = all(qp_facts::default_detectors, &b);
        assert!(x.iter().any(|v| !v.is_empty()));
        assert_eq!(x, all(qp_facts::rp4_detectors, &b));
    }
    let b = bars(Interval::Minutes(60), 120, true, 5);
    let (x, y) = (
        all(qp_facts::default_detectors, &b),
        all(qp_facts::rp4_detectors, &b),
    );
    for (i, d) in r3.iter().enumerate() {
        if matches!(d.id(), "breakout.range" | "flow.volume_surge") {
            assert_ne!(x[i], y[i], "{}", d.id());
        } else {
            assert_eq!(x[i], y[i], "{}", d.id());
        }
    }
}

#[test]
fn named_sets() {
    assert_eq!(qp_facts::DETECTOR_SETS, &["r3", "rp4"]);
    for n in qp_facts::DETECTOR_SETS {
        assert_eq!(qp_facts::detectors_named(n).unwrap().len(), 7);
    }
    assert!(qp_facts::detectors_named("r4").is_none());
}
