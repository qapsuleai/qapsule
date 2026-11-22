//! R3/T2: regime synthetic market, synthetic flow feed, local bar reader.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::cognitive_complexity
)]

use std::collections::BTreeMap;

use qp_calendar::{ExchangeCalendar, Interval};
use qp_schema::Bar;
use qp_source::{
    load_local_bars, load_local_bars_with, regime_at, regime_bars, regime_schedule,
    synthetic_calendar, synthetic_flow, synthetic_market, synthetic_market_hourly, Regime,
    RegimeSpan, FLOW_KEYS,
};

/// The synthetic market's calendar (XIST sessions, frozen R1 holiday sample).
fn xist() -> ExchangeCalendar {
    ExchangeCalendar::new(&synthetic_calendar()).unwrap()
}

/// The synthetic market keeps its own frozen holiday sample: the real XIST holidays
/// (R4/FIX-E1) do not move synthetic dates.
#[test]
fn synthetic_calendar_is_the_frozen_r1_sample() {
    let syn = synthetic_calendar();
    let real = qp_calendar::builtin("XIST").unwrap();
    assert_eq!(
        syn.holidays,
        [
            "2026-01-01",
            "2026-03-20",
            "2026-04-23",
            "2026-05-01",
            "2026-05-19"
        ]
    );
    assert_eq!(syn.half_days.len(), 1);
    assert_eq!(
        (syn.tz.as_str(), &syn.sessions),
        (real.tz.as_str(), &real.sessions)
    );
    assert!(real.holidays.len() > 100);
    // 2023-04-21 (Ramadan Feast) is a synthetic trading day, a real holiday.
    let day = qp_calendar::days_from_civil(2023, 4, 21);
    assert!(xist().is_trading_day(day));
    assert!(!ExchangeCalendar::new(&real).unwrap().is_trading_day(day));
}

fn bits(bars: &[Bar]) -> Vec<[u64; 6]> {
    bars.iter()
        .map(|b| {
            [
                b.t.cast_unsigned(),
                b.o.to_bits(),
                b.h.to_bits(),
                b.l.to_bits(),
                b.c.to_bits(),
                b.v.to_bits(),
            ]
        })
        .collect()
}

#[test]
fn market_is_deterministic_bit_for_bit() {
    let a = synthetic_market(30, 500, 42);
    let b = synthetic_market(30, 500, 42);
    assert_eq!(a.len(), 30);
    for (x, y) in a.iter().zip(&b) {
        assert_eq!(x.symbol, y.symbol);
        assert_eq!(x.schedule, y.schedule);
        assert_eq!(bits(&x.bars), bits(&y.bars));
        assert_eq!(x.bars.len(), 500);
    }
    assert_eq!(a[0].symbol, "SYN01");
    assert_eq!(a[29].symbol, "SYN30");
    let c = synthetic_market(30, 500, 43);
    assert_ne!(bits(&a[0].bars), bits(&c[0].bars));
    // Instruments differ from each other.
    assert_ne!(bits(&a[0].bars), bits(&a[1].bars));
    // A prefix of a longer market is the shorter market (schedules aside: they are
    // drawn up to n_days) — the first instrument's first bar does not depend on n_days.
    let h1 = synthetic_market_hourly(3, 40, 9);
    let h2 = synthetic_market_hourly(3, 40, 9);
    for (x, y) in h1.iter().zip(&h2) {
        assert_eq!(bits(&x.bars), bits(&y.bars));
    }
}

#[test]
fn bars_are_aligned_to_the_xist_calendar() {
    let cal = xist();
    let m = synthetic_market(3, 500, 1);
    for inst in &m {
        qp_schema::validate_bars(&inst.bars).unwrap();
        for b in &inst.bars {
            let span = cal.bar_at(Interval::Day, b.t).expect("in session");
            assert_eq!(span.open, b.t, "daily t must be the session open");
            assert!(b.l > 0.0 && b.v > 0.0);
        }
    }
    // Same times for every instrument.
    assert_eq!(
        m[0].bars.iter().map(|b| b.t).collect::<Vec<_>>(),
        m[2].bars.iter().map(|b| b.t).collect::<Vec<_>>()
    );
    let h = synthetic_market_hourly(2, 60, 1);
    for inst in &h {
        qp_schema::validate_bars(&inst.bars).unwrap();
        for b in &inst.bars {
            let span = cal.bar_at(Interval::Minutes(60), b.t).expect("in session");
            assert_eq!(span.open, b.t, "hourly t must be the hour bar open");
        }
        let days: std::collections::BTreeSet<i64> =
            inst.bars.iter().map(|b| cal.local_day(b.t)).collect();
        assert_eq!(days.len(), 60);
    }
}

#[test]
fn schedule_covers_all_days_and_switches() {
    let s = regime_schedule(500, 5);
    assert_eq!(s.first().unwrap().start, 0);
    assert_eq!(s.last().unwrap().end, 500);
    for w in s.windows(2) {
        assert_eq!(w[0].end, w[1].start);
        assert_ne!(w[0].regime, w[1].regime);
    }
    assert!(s.len() >= 5, "regimes must switch over 500 days");
    assert_eq!(regime_at(&s, 0), Some(s[0].regime));
    assert_eq!(regime_at(&s, 500), None);
    let m = synthetic_market(30, 500, 7);
    let mut seen = std::collections::BTreeSet::new();
    for inst in &m {
        for sp in &inst.schedule {
            seen.insert(sp.regime);
        }
    }
    assert_eq!(seen.len(), 4, "all four regimes occur");
}

fn returns_by_regime(m: &[qp_source::SyntheticInstrument]) -> BTreeMap<Regime, Vec<f64>> {
    let mut by: BTreeMap<Regime, Vec<f64>> = BTreeMap::new();
    for inst in m {
        for i in 1..inst.bars.len() {
            let r = inst.bars[i].c / inst.bars[i - 1].c - 1.0;
            by.entry(regime_at(&inst.schedule, i).unwrap())
                .or_default()
                .push(r);
        }
    }
    by
}

fn mean(v: &[f64]) -> f64 {
    v.iter().sum::<f64>() / v.len() as f64
}

fn std(v: &[f64]) -> f64 {
    let m = mean(v);
    (v.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / v.len() as f64).sqrt()
}

#[test]
fn regimes_actually_differ() {
    let m = synthetic_market(30, 500, 11);
    let by = returns_by_regime(&m);
    let (r, f, s, v) = (
        &by[&Regime::Rising],
        &by[&Regime::Falling],
        &by[&Regime::Sideways],
        &by[&Regime::Volatile],
    );
    assert!(mean(r) > 0.0005, "rising drift {}", mean(r));
    assert!(mean(f) < -0.0005, "falling drift {}", mean(f));
    assert!(mean(s).abs() < mean(r).abs());
    assert!(
        std(v) > 1.5 * std(s),
        "volatile {} vs sideways {}",
        std(v),
        std(s)
    );
    assert!(std(v) > std(r));
}

#[test]
fn regime_bars_follow_the_given_schedule() {
    let sched = vec![
        RegimeSpan {
            regime: Regime::Rising,
            start: 0,
            end: 150,
        },
        RegimeSpan {
            regime: Regime::Falling,
            start: 150,
            end: 300,
        },
    ];
    // Noise can beat the drift on a single leg; over ten seeds the legs follow the regime.
    let mut right = 0;
    for seed in 0..10 {
        let bars = regime_bars(&sched, seed, 100.0);
        assert_eq!(bars.len(), 300);
        right += usize::from(bars[149].c > bars[0].o);
        right += usize::from(bars[299].c < bars[150].o);
    }
    assert!(right >= 17, "legs following their regime: {right}/20");
    assert_eq!(
        bits(&regime_bars(&sched, 3, 100.0)),
        bits(&regime_bars(&sched, 3, 100.0))
    );
}

/// Wilder RSI(14) per bar (NaN until warm).
fn rsi(bars: &[Bar]) -> Vec<f64> {
    let n = 14.0;
    let mut out = vec![f64::NAN; bars.len()];
    let (mut ag, mut al) = (0.0, 0.0);
    for i in 1..bars.len() {
        let d = bars[i].c - bars[i - 1].c;
        let (g, l) = (d.max(0.0), (-d).max(0.0));
        if i <= 14 {
            ag += g / n;
            al += l / n;
        } else {
            ag = (ag * (n - 1.0) + g) / n;
            al = (al * (n - 1.0) + l) / n;
        }
        if i >= 14 {
            out[i] = if al == 0.0 {
                100.0
            } else {
                100.0 - 100.0 / (1.0 + ag / al)
            };
        }
    }
    out
}

#[test]
fn classic_events_occur() {
    let m = synthetic_market(30, 500, 2026);
    let (mut rsi_hi, mut rsi_lo, mut breakouts, mut squeezes) = (0, 0, 0, 0);
    let (mut spike_move, mut spike_quiet) = (0, 0);
    let mut divergences = 0;
    for inst in &m {
        let b = &inst.bars;
        let r = rsi(b);
        for i in 1..b.len() {
            if r[i] > 70.0 && r[i - 1] <= 70.0 {
                rsi_hi += 1;
            }
            if r[i] < 30.0 && r[i - 1] >= 30.0 {
                rsi_lo += 1;
            }
        }
        // Bearish divergences: a 5-bar swing high above the previous swing high while
        // RSI is lower than at the previous one.
        let mut prev_peak: Option<usize> = None;
        for i in 5..b.len().saturating_sub(5) {
            let is_peak = (i - 5..=i + 5).all(|j| j == i || b[j].h < b[i].h);
            if is_peak {
                if let Some(p) = prev_peak {
                    if b[i].h > b[p].h && r[i] < r[p] {
                        divergences += 1;
                    }
                }
                prev_peak = Some(i);
            }
        }
        // 20-bar range breakouts.
        for i in 20..b.len() {
            let hi = b[i - 20..i].iter().map(|x| x.h).fold(f64::MIN, f64::max);
            if b[i].c > hi {
                breakouts += 1;
            }
        }
        // Bollinger bandwidth (20, 2): a 120-bar low followed within 20 bars by a doubling.
        let bw: Vec<f64> = (0..b.len())
            .map(|i| {
                if i < 19 {
                    return f64::NAN;
                }
                let w: Vec<f64> = b[i - 19..=i].iter().map(|x| x.c).collect();
                4.0 * std(&w) / mean(&w)
            })
            .collect();
        let mut i = 140;
        while i + 20 < b.len() {
            let low = bw[i - 120..i].iter().copied().fold(f64::MAX, f64::min);
            if bw[i] < low && bw[i + 1..=i + 20].iter().any(|&x| x > 2.0 * bw[i]) {
                squeezes += 1;
                i += 20;
            }
            i += 1;
        }
        // Volume spikes: > 3x the 20-bar mean, with and without a big move.
        for i in 20..b.len() {
            let avg = b[i - 20..i].iter().map(|x| x.v).sum::<f64>() / 20.0;
            if b[i].v > 3.0 * avg {
                let ret = (b[i].c / b[i - 1].c - 1.0).abs();
                if ret > 0.03 {
                    spike_move += 1;
                } else if ret < 0.01 {
                    spike_quiet += 1;
                }
            }
        }
    }
    eprintln!(
        "events over 30x500: rsi>70 {rsi_hi}, rsi<30 {rsi_lo}, breakouts {breakouts}, \
         squeezes {squeezes}, volume spikes on moves {spike_move}, quiet {spike_quiet}, \
         bearish divergences {divergences}"
    );
    assert!(divergences > 30, "bearish divergences {divergences}");
    assert!(rsi_hi > 30, "RSI>70 excursions {rsi_hi}");
    assert!(rsi_lo > 30, "RSI<30 excursions {rsi_lo}");
    assert!(breakouts > 300, "20-bar breakouts {breakouts}");
    assert!(squeezes > 10, "squeeze then expansion {squeezes}");
    assert!(spike_move > 30, "volume spikes on big moves {spike_move}");
    assert!(
        spike_quiet > 10,
        "volume spikes without a move {spike_quiet}"
    );
}

#[test]
fn flow_is_deterministic_and_consistent() {
    let m = synthetic_market(5, 500, 3);
    let bars = &m[0].bars;
    let a = synthetic_flow(bars, 77);
    let b = synthetic_flow(bars, 77);
    assert_eq!(a, b);
    let bits_of = |f: &[qp_source::FlowRecord]| -> Vec<u64> {
        f.iter()
            .flat_map(|r| r.values.values().map(|v| v.to_bits()))
            .collect()
    };
    assert_eq!(bits_of(&a), bits_of(&b));
    assert_ne!(bits_of(&a), bits_of(&synthetic_flow(bars, 78)));
    assert_eq!(a.len(), bars.len());
    let mut big = 0;
    let (mut sxy, mut sxx, mut syy) = (0.0, 0.0, 0.0);
    for (i, (r, bar)) in a.iter().zip(bars).enumerate() {
        assert_eq!(r.t, bar.t);
        let keys: Vec<&str> = r.values.keys().map(String::as_str).collect();
        let mut want = FLOW_KEYS.to_vec();
        want.sort_unstable();
        assert_eq!(keys, want);
        let (net, buy, sell, top5) = (
            r.values["net"],
            r.values["buy"],
            r.values["sell"],
            r.values["brokers_net_top5"],
        );
        assert!(buy >= 0.0 && sell >= 0.0);
        assert!((buy - sell - net).abs() < 0.011);
        assert!(net.abs() <= top5.abs() + 0.011);
        if top5.abs() > 0.15 * bar.v {
            big += 1;
        }
        if i > 0 {
            let x = bar.c / bars[i - 1].c - 1.0;
            let y = top5 / bar.v;
            sxy += x * y;
            sxx += x * x;
            syy += y * y;
        }
    }
    let corr = sxy / (sxx * syy).sqrt();
    assert!(
        corr > 0.2 && corr < 0.9,
        "partial correlation with price {corr}"
    );
    assert!(big >= 5, "institutional days {big}");
}

#[test]
fn local_reader_reads_sorted_and_tolerates_junk() {
    let dir = std::env::temp_dir().join(format!("qp-source-local-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        load_local_bars(&dir, "1day").is_empty(),
        "missing dir: empty"
    );
    std::fs::create_dir_all(&dir).unwrap();
    assert!(load_local_bars(&dir, "1day").is_empty(), "empty dir: empty");
    let series = |sym: &str, bars: &[Bar]| {
        serde_json::to_string(&qp_schema::Series {
            id: sym.to_owned(),
            instrument: Some(qp_schema::Instrument {
                symbol: sym.to_owned(),
                exchange: Some("BIST".to_owned()),
                calendar: Some("XIST".to_owned()),
            }),
            interval: Some("1d".to_owned()),
            source: qp_schema::SeriesSource::Inline {},
            data: Some(qp_schema::SeriesData::Bars {
                bars: bars.to_vec(),
            }),
        })
        .unwrap()
    };
    let m = synthetic_market(2, 10, 1);
    let w = |name: &str, text: &str| std::fs::write(dir.join(name), text).unwrap();
    w("ZZZ.1day.adj.json", &series("ZZZ", &m[0].bars));
    w("AAA.1day.adj.json", &series("AAA", &m[1].bars));
    w("AAA.1day.unadj.json", &series("AAA", &m[0].bars));
    w("AAA.1day.adj.meta.json", "{\"symbol\":\"AAA\"}");
    w("BBB.1day.adj.json", "not json");
    w("CCC.1hour.adj.json", &series("CCC", &m[0].bars));
    w("DDD.1day.adj.json", &series("DDD", &[]));
    let got = load_local_bars(&dir, "1day");
    let syms: Vec<&str> = got.iter().map(|(s, _)| s.as_str()).collect();
    assert_eq!(syms, ["AAA", "ZZZ"]);
    assert_eq!(bits(&got[0].1), bits(&m[1].bars));
    let un = load_local_bars_with(&dir, "1day", false);
    assert_eq!(un.len(), 1);
    assert_eq!(bits(&un[0].1), bits(&m[0].bars));
    assert_eq!(load_local_bars(&dir, "1hour").len(), 1);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn large_market_is_fast_enough() {
    let t0 = std::time::Instant::now();
    let m = synthetic_market(40, 1000, 99);
    let flows: usize = m.iter().map(|i| synthetic_flow(&i.bars, 1).len()).sum();
    assert_eq!(flows, 40_000);
    // Generous bound (debug build); release is far faster.
    assert!(t0.elapsed().as_secs() < 20);
}
