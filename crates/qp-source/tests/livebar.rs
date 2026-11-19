//! Live bars from price-card updates (R6, M22): alignment (K14), volume, the three
//! methods, the seam with history, late ticks, determinism and §18.4 conformity. All
//! ticks are hand-built (no real data).

use qp_calendar::{builtin, parse_date, ExchangeCalendar, Interval};
use qp_schema::data::{DataChange, DataEvent, FeedKey};
use qp_schema::Bar;
use qp_source::{
    compare_bars, compare_bars_with, Applied, BarMethod, CardTick, DiffTolerance, FeedStore,
    LiveBars, LiveConfig, VolumeUnit,
};

const DAY: &str = "2026-11-18"; // Wednesday, full trading day
const NEXT: &str = "2026-11-19";
const HALF: &str = "2026-10-28"; // half day, close 12:30
const HOLIDAY: &str = "2026-10-29";
const SATURDAY: &str = "2026-11-21";

fn xist() -> ExchangeCalendar {
    ExchangeCalendar::new(&builtin("XIST").expect("XIST")).expect("calendar")
}

/// UTC ms of Istanbul local `date hh:mm:ss` (UTC+3 all year since 2016).
fn at(date: &str, hh: i64, mm: i64, ss: i64) -> i64 {
    parse_date(date).expect("date") * 86_400_000 + ((hh * 60 + mm) * 60 + ss) * 1000 - 3 * 3_600_000
}

fn key(iv: &str) -> FeedKey {
    FeedKey {
        symbol: "TEST".into(),
        interval: iv.into(),
        feed: None,
    }
}

fn cfg(method: BarMethod) -> LiveConfig {
    LiveConfig {
        method,
        ..LiveConfig::default()
    }
}

fn live(iv: &str, method: BarMethod) -> LiveBars {
    LiveBars::new(key(iv), xist(), cfg(method)).expect("live bars")
}

fn trade(t: i64, last: f64, cum: f64) -> CardTick {
    CardTick {
        last: Some(last),
        cum_volume: Some(cum),
        ..CardTick::at(t)
    }
}

/// Pushes ticks, checks local event rules (no empty patch, contiguous seq).
fn run(lb: &mut LiveBars, ticks: &[CardTick]) -> Vec<DataEvent> {
    let mut out = Vec::new();
    for t in ticks {
        out.extend(lb.push(t));
    }
    check_events(&out);
    out
}

fn check_events(evs: &[DataEvent]) {
    for w in evs.windows(2) {
        assert_eq!(w[1].seq, w[0].seq + 1, "seq must be contiguous");
    }
    for e in evs {
        if let DataChange::Patch { fields, .. } = &e.change {
            assert!(!fields.is_empty(), "empty patch");
        }
    }
}

/// Applies events by the §18.4 rules; every event must be a new bar or a live update
/// of the last bar (no correction, no rejection), with monotone h, l, v.
fn apply(store: &mut FeedStore, evs: &[DataEvent]) {
    for e in evs {
        let before = store.bars().last().cloned();
        let applied = store.apply(e).expect("event accepted");
        match applied {
            Applied::NewBar { .. } => {}
            Applied::LiveUpdate { .. } => {
                let (b, a) = (before.expect("bar"), store.bars().last().expect("bar"));
                assert!(a.h >= b.h && a.l <= b.l && a.v >= b.v, "monotone h/l/v");
            }
            other => panic!("unexpected {other:?}"),
        }
    }
    if !store.is_empty() {
        qp_schema::validate_bars(store.bars()).expect("valid bars");
    }
}

fn bars_of(iv: &str, evs: &[DataEvent]) -> Vec<Bar> {
    let mut s = FeedStore::new(key(iv));
    apply(&mut s, evs);
    s.bars().to_vec()
}

/// One synthetic trading day of card updates: a pre-open snapshot (yesterday's price,
/// no trade), the opening match, continuous trading with day extremes kept by the card,
/// some extreme-only and volume-only updates, the closing session and trades at the
/// closing price.
fn day_ticks(date: &str) -> Vec<CardTick> {
    let mut v = vec![CardTick {
        initial: true,
        last: Some(99.0),
        cum_volume: Some(0.0),
        cum_quantity: Some(0.0),
        phase: Some(4),
        ..CardTick::at(at(date, 9, 41, 0))
    }];
    let (mut hi, mut lo, mut cum, mut qty) = (100.0_f64, 100.0_f64, 5000.0, 50.0);
    v.push(CardTick {
        last: Some(100.0),
        day_open: Some(100.0),
        day_high: Some(100.0),
        day_low: Some(100.0),
        cum_volume: Some(cum),
        cum_quantity: Some(qty),
        phase: Some(2),
        ..CardTick::at(at(date, 9, 59, 30))
    });
    let mut t = at(date, 10, 0, 0);
    let end = at(date, 18, 0, 0);
    let mut k: i64 = 0;
    while t < end {
        k += 1;
        let p = 100.0 + ((k * 37) % 23 - 11) as f64 * 0.05;
        cum += ((k % 7) + 1) as f64 * 1000.0;
        qty += ((k % 5) + 1) as f64;
        let mut tk = CardTick {
            cum_volume: Some(cum),
            cum_quantity: Some(qty),
            ..CardTick::at(t)
        };
        if k % 11 == 0 {
            // A new day extreme arrives without `last` (trade seen only in the card).
            hi += 0.15;
            tk.day_high = Some(hi);
        } else if k % 13 == 0 {
            lo -= 0.15;
            tk.day_low = Some(lo);
        } else if k % 17 != 0 {
            // (every 17th: volume only, trade at the unchanged last price)
            tk.last = Some(p);
            if p > hi {
                hi = p;
                tk.day_high = Some(hi);
            }
            if p < lo {
                lo = p;
                tk.day_low = Some(lo);
            }
        }
        v.push(tk);
        t += 97_000 + (k % 3) * 41_000; // irregular gaps
    }
    // Closing session (18:05) and trades at the closing price (18:12, volume only).
    cum += 50_000.0;
    qty += 500.0;
    v.push(CardTick {
        last: Some(100.4),
        cum_volume: Some(cum),
        cum_quantity: Some(qty),
        phase: Some(5),
        ..CardTick::at(at(date, 18, 5, 0))
    });
    cum += 2000.0;
    qty += 20.0;
    v.push(CardTick {
        cum_volume: Some(cum),
        cum_quantity: Some(qty),
        ..CardTick::at(at(date, 18, 12, 0))
    });
    v
}

/// The card state merged from `ticks` (what a fresh subscription's snapshot shows).
fn snapshot(ticks: &[CardTick]) -> CardTick {
    let mut s = CardTick {
        initial: true,
        ..CardTick::at(ticks.last().expect("tick").t_ms)
    };
    for t in ticks {
        s.last = t.last.or(s.last);
        s.day_open = t.day_open.or(s.day_open);
        s.day_high = t.day_high.or(s.day_high);
        s.day_low = t.day_low.or(s.day_low);
        s.cum_volume = t.cum_volume.or(s.cum_volume);
        s.cum_quantity = t.cum_quantity.or(s.cum_quantity);
        s.phase = t.phase.or(s.phase);
    }
    s
}

const INTERVALS: [&str; 6] = ["1m", "5m", "15m", "1h", "1d", "1w"];

fn methods(iv: &str) -> Vec<BarMethod> {
    if iv == "1d" {
        vec![BarMethod::Ticks, BarMethod::CardDaily, BarMethod::Mixed]
    } else {
        vec![BarMethod::Ticks, BarMethod::Mixed]
    }
}

// --- 1. Alignment ----------------------------------------------------------------

#[test]
fn ticks_align_to_the_xist_calendar() {
    let m5 = live("5m", BarMethod::Ticks);
    let h1 = live("1h", BarMethod::Ticks);
    let d1 = live("1d", BarMethod::Ticks);
    let open = |lb: &LiveBars, t| lb.bar_of(t).map(|s| s.open);
    // In session.
    assert_eq!(open(&m5, at(DAY, 10, 7, 12)), Some(at(DAY, 10, 5, 0)));
    assert_eq!(open(&h1, at(DAY, 13, 59, 59)), Some(at(DAY, 13, 0, 0)));
    // 1d bar `t` is the session open, 10:00 Istanbul = 07:00 UTC.
    assert_eq!(open(&d1, at(DAY, 15, 0, 0)), Some(at(DAY, 10, 0, 0)));
    assert_eq!(at(DAY, 10, 0, 0) % 86_400_000, 7 * 3_600_000);
    // Opening auction (09:40–10:00) joins the first bar; earlier is off session.
    for lb in [&m5, &h1, &d1] {
        assert_eq!(open(lb, at(DAY, 9, 45, 0)), Some(at(DAY, 10, 0, 0)));
        assert_eq!(open(lb, at(DAY, 9, 29, 59)), None);
    }
    // Closing session (18:00–18:10, closing-price trades) joins the last bar.
    assert_eq!(open(&m5, at(DAY, 18, 5, 0)), Some(at(DAY, 17, 55, 0)));
    assert_eq!(open(&h1, at(DAY, 18, 12, 0)), Some(at(DAY, 17, 0, 0)));
    assert_eq!(open(&d1, at(DAY, 18, 29, 59)), Some(at(DAY, 10, 0, 0)));
    assert_eq!(open(&d1, at(DAY, 18, 30, 0)), None);
    // Half day: continuous trading ends 12:30; the closing session joins the 12:00 bar.
    assert_eq!(open(&m5, at(HALF, 12, 35, 0)), Some(at(HALF, 12, 25, 0)));
    assert_eq!(open(&h1, at(HALF, 12, 35, 0)), Some(at(HALF, 12, 0, 0)));
    assert_eq!(
        h1.bar_of(at(HALF, 12, 10, 0)).map(|s| s.close),
        Some(at(HALF, 12, 30, 0))
    );
    assert_eq!(open(&h1, at(HALF, 13, 1, 0)), None);
    assert_eq!(open(&h1, at(HALF, 15, 0, 0)), None);
    // Night, weekend, holiday.
    for t in [
        at(DAY, 3, 0, 0),
        at(DAY, 21, 0, 0),
        at(SATURDAY, 12, 0, 0),
        at(HOLIDAY, 12, 0, 0),
    ] {
        assert_eq!(open(&m5, t), None);
        assert_eq!(open(&d1, t), None);
    }
    // Every bar of a full day is calendar-aligned.
    let cal = xist();
    for iv in INTERVALS {
        for m in methods(iv) {
            let mut lb = live(iv, m);
            let bars = bars_of(iv, &run(&mut lb, &day_ticks(DAY)));
            let interval = Interval::parse(iv).expect("interval");
            for b in &bars {
                assert_eq!(cal.bar_at(interval, b.t).map(|s| s.open), Some(b.t), "{iv}");
            }
        }
    }
}

#[test]
fn off_session_ticks_build_nothing_and_are_counted() {
    let mut lb = live("5m", BarMethod::Ticks);
    let evs = run(
        &mut lb,
        &[
            trade(at(SATURDAY, 12, 0, 0), 10.0, 100.0),
            trade(at(HOLIDAY, 11, 0, 0), 10.0, 100.0),
            trade(at(DAY, 2, 0, 0), 10.0, 100.0),
            trade(at(DAY, 9, 0, 0), 10.0, 100.0),
        ],
    );
    assert!(evs.is_empty());
    assert_eq!(lb.stats().off_session, 4);
    assert!(lb.current().is_none());
}

#[test]
fn closing_session_extends_the_last_bar_and_close_until_finalizes_after_it() {
    let mut lb = live("1h", BarMethod::Ticks);
    let mut evs = run(
        &mut lb,
        &[
            trade(at(DAY, 17, 30, 0), 50.0, 1000.0),
            trade(at(DAY, 17, 59, 0), 50.5, 2000.0),
            trade(at(DAY, 18, 5, 0), 50.25, 3000.0),
            CardTick {
                cum_volume: Some(3500.0),
                ..CardTick::at(at(DAY, 18, 12, 0))
            },
        ],
    );
    assert!(lb.close_until(at(DAY, 18, 29, 0)).is_none());
    let closed = lb.close_until(at(DAY, 18, 30, 0)).expect("final");
    assert_eq!(closed.t, at(DAY, 17, 0, 0));
    assert!(lb.close_until(at(DAY, 19, 0, 0)).is_none(), "reported once");
    assert_eq!(lb.last_final_t(), Some(at(DAY, 17, 0, 0)));
    evs.extend(lb.push(&trade(at(DAY, 18, 20, 0), 51.0, 4000.0)));
    assert_eq!(lb.stats().late, 1);
    let bars = bars_of("1h", &evs);
    assert_eq!(
        bars,
        vec![Bar {
            t: at(DAY, 17, 0, 0),
            o: 50.0,
            h: 50.5,
            l: 50.0,
            c: 50.25,
            // First tick of the day (not a snapshot): counter from 0.
            v: 3500.0,
        }]
    );
    assert_eq!(closed, bars[0]);
}

// --- 2/7. Events, determinism ----------------------------------------------------

#[test]
fn no_empty_patch_and_first_seq_is_honoured() {
    let mut lb = LiveBars::new(
        key("5m"),
        xist(),
        LiveConfig {
            first_seq: 42,
            ..LiveConfig::default()
        },
    )
    .expect("live");
    let evs = run(
        &mut lb,
        &[
            trade(at(DAY, 10, 1, 0), 10.0, 100.0),
            CardTick {
                phase: Some(2),
                ..CardTick::at(at(DAY, 10, 1, 30))
            },
            trade(at(DAY, 10, 2, 0), 10.0, 100.0), // nothing changed
            CardTick::at(at(DAY, 10, 2, 30)),
            trade(at(DAY, 10, 3, 0), 10.5, 150.0),
        ],
    );
    assert_eq!(evs.len(), 2);
    assert_eq!(evs[0].seq, 42);
    assert_eq!(lb.next_seq(), 44);
    match &evs[1].change {
        DataChange::Patch { t, fields } => {
            assert_eq!(*t, at(DAY, 10, 0, 0));
            assert_eq!(
                (fields.o, fields.l),
                (None, None),
                "unchanged fields absent"
            );
            assert_eq!(
                (fields.h, fields.c, fields.v),
                (Some(10.5), Some(10.5), Some(150.0))
            );
        }
        other => panic!("expected patch, got {other:?}"),
    }
    assert_eq!(lb.stats().last_phase, Some(2));
}

#[test]
fn same_ticks_give_byte_equal_events() {
    let mut ticks = day_ticks(DAY);
    ticks.extend(day_ticks(NEXT));
    for iv in INTERVALS {
        for m in methods(iv) {
            let a = serde_json::to_string(&run(&mut live(iv, m), &ticks)).expect("json");
            let b = serde_json::to_string(&run(&mut live(iv, m), &ticks)).expect("json");
            assert_eq!(a, b, "{iv} {}", m.name());
            assert!(!a.is_empty());
        }
    }
}

// --- 3. Volume -------------------------------------------------------------------

#[test]
fn volume_is_the_counter_increase_and_restarts_each_day() {
    let mut lb = live("1h", BarMethod::Ticks);
    let evs = run(
        &mut lb,
        &[
            trade(at(DAY, 10, 10, 0), 20.0, 1000.0),
            trade(at(DAY, 10, 50, 0), 20.1, 4000.0),
            trade(at(DAY, 11, 5, 0), 20.2, 4500.0),
            // Next day: counter restarted (not a regression).
            trade(at(NEXT, 10, 2, 0), 20.3, 700.0),
            trade(at(NEXT, 10, 3, 0), 20.3, 900.0),
        ],
    );
    let v: Vec<f64> = bars_of("1h", &evs).iter().map(|b| b.v).collect();
    assert_eq!(v, vec![4000.0, 500.0, 900.0]);
    assert_eq!(lb.stats().volume_regressions, 0);
}

#[test]
fn counter_regression_adds_no_negative_volume_and_is_counted() {
    let mut lb = live("5m", BarMethod::Ticks);
    let evs = run(
        &mut lb,
        &[
            trade(at(DAY, 10, 0, 0), 20.0, 10_000.0),
            trade(at(DAY, 10, 1, 0), 20.0, 9_000.0),
            trade(at(DAY, 10, 2, 0), 20.0, 9_500.0),
        ],
    );
    assert_eq!(bars_of("5m", &evs)[0].v, 10_500.0);
    assert_eq!(lb.stats().volume_regressions, 1);
}

#[test]
fn quantity_unit_reads_the_quantity_counter() {
    let mut lb = LiveBars::new(
        key("5m"),
        xist(),
        LiveConfig {
            volume: VolumeUnit::Quantity,
            ..LiveConfig::default()
        },
    )
    .expect("live");
    let tick = |t, q| CardTick {
        last: Some(5.0),
        cum_volume: Some(q * 5.0),
        cum_quantity: Some(q),
        ..CardTick::at(t)
    };
    let evs = run(
        &mut lb,
        &[tick(at(DAY, 10, 0, 0), 10.0), tick(at(DAY, 10, 1, 0), 30.0)],
    );
    assert_eq!(bars_of("5m", &evs)[0].v, 30.0);
}

#[test]
fn mid_session_snapshot_is_the_volume_base() {
    let mut lb = live("5m", BarMethod::Ticks);
    let evs = run(
        &mut lb,
        &[
            CardTick {
                initial: true,
                last: Some(50.0),
                day_open: Some(48.0),
                day_high: Some(52.0),
                day_low: Some(47.0),
                cum_volume: Some(1_000_000.0),
                ..CardTick::at(at(DAY, 13, 0, 10))
            },
            trade(at(DAY, 13, 1, 0), 50.5, 1_002_000.0),
        ],
    );
    assert_eq!(lb.stats().volume_rebases, 1);
    assert_eq!(
        bars_of("5m", &evs),
        vec![Bar {
            t: at(DAY, 13, 0, 0),
            o: 50.5,
            h: 50.5,
            l: 50.5,
            c: 50.5,
            v: 2000.0
        }]
    );
}

// --- 4. Methods ------------------------------------------------------------------

#[test]
fn card_daily_is_only_for_1d() {
    for iv in ["1m", "5m", "1h", "1w"] {
        assert!(LiveBars::new(key(iv), xist(), cfg(BarMethod::CardDaily)).is_err());
        assert!(LiveBars::new(key(iv), xist(), cfg(BarMethod::Mixed)).is_ok());
    }
    assert!(LiveBars::new(key("1d"), xist(), cfg(BarMethod::CardDaily)).is_ok());
}

#[test]
fn card_daily_takes_the_day_fields_ticks_does_not() {
    let ticks = [
        CardTick {
            initial: true,
            last: Some(50.0),
            day_open: Some(48.0),
            day_high: Some(52.0),
            day_low: Some(47.0),
            cum_volume: Some(1_000_000.0),
            ..CardTick::at(at(DAY, 13, 0, 0))
        },
        trade(at(DAY, 14, 0, 0), 53.0, 1_001_000.0),
    ];
    let day = |o, h, l, c, v| Bar {
        t: at(DAY, 10, 0, 0),
        o,
        h,
        l,
        c,
        v,
    };
    for m in [BarMethod::CardDaily, BarMethod::Mixed] {
        let mut lb = live("1d", m);
        let evs = run(&mut lb, &ticks);
        assert!(
            matches!(evs[0].change, DataChange::Bar { .. }),
            "snapshot opens the day bar"
        );
        assert_eq!(
            bars_of("1d", &evs),
            vec![day(48.0, 53.0, 47.0, 53.0, 1_001_000.0)]
        );
    }
    let mut lb = live("1d", BarMethod::Ticks);
    let evs = run(&mut lb, &ticks);
    assert_eq!(
        bars_of("1d", &evs),
        vec![day(53.0, 53.0, 53.0, 53.0, 1000.0)]
    );
}

#[test]
fn mixed_reflects_new_day_extremes_into_the_intraday_bar() {
    let ticks = [
        CardTick {
            last: Some(100.0),
            day_high: Some(100.0),
            day_low: Some(100.0),
            cum_volume: Some(1000.0),
            ..CardTick::at(at(DAY, 10, 0, 0))
        },
        CardTick {
            day_high: Some(100.5),
            cum_volume: Some(1500.0),
            ..CardTick::at(at(DAY, 10, 1, 0))
        },
        CardTick {
            day_low: Some(99.0),
            ..CardTick::at(at(DAY, 10, 2, 0))
        },
        trade(at(DAY, 10, 6, 0), 100.2, 1600.0),
        // Not a new day high (100.5 is): no reflection.
        CardTick {
            day_high: Some(100.4),
            ..CardTick::at(at(DAY, 10, 7, 0))
        },
        // Snapshot after reconnect: its extremes may belong to earlier bars.
        CardTick {
            initial: true,
            day_high: Some(101.0),
            ..CardTick::at(at(DAY, 10, 8, 0))
        },
    ];
    let mut mixed = live("5m", BarMethod::Mixed);
    let b = bars_of("5m", &run(&mut mixed, &ticks));
    // First tick of the day is not a snapshot: its counter counts from 0.
    assert_eq!((b[0].h, b[0].l, b[0].v), (100.5, 99.0, 1500.0));
    assert_eq!((b[1].h, b[1].l), (100.2, 100.2));
    assert_eq!(mixed.stats().extremes_reflected, 2);
    let mut plain = live("5m", BarMethod::Ticks);
    let b = bars_of("5m", &run(&mut plain, &ticks));
    assert_eq!((b[0].h, b[0].l, b[0].v), (100.0, 100.0, 1500.0));
}

// --- 5. Provisional / final, late ticks ------------------------------------------

#[test]
fn late_ticks_are_not_written_and_are_counted() {
    let mut lb = live("5m", BarMethod::Ticks);
    let mut evs = run(
        &mut lb,
        &[
            trade(at(DAY, 10, 1, 0), 10.0, 100.0),
            trade(at(DAY, 10, 6, 0), 10.1, 200.0),
        ],
    );
    assert_eq!(lb.last_final_t(), Some(at(DAY, 10, 0, 0)));
    // A tick for the 10:00 bar after 10:05 opened: late.
    evs.extend(lb.push(&trade(at(DAY, 10, 3, 0), 9.0, 300.0)));
    // Clock passes 10:10: the 10:05 bar is final; its later ticks are late too.
    assert_eq!(
        lb.close_until(at(DAY, 10, 10, 0)).map(|b| b.t),
        Some(at(DAY, 10, 5, 0))
    );
    assert_eq!(lb.current().map(|(_, f)| f), Some(true));
    evs.extend(lb.push(&trade(at(DAY, 10, 9, 0), 9.0, 300.0)));
    // A bar whose span the clock already passed is not opened either.
    evs.extend(lb.push(&trade(at(DAY, 10, 8, 0), 9.0, 300.0)));
    assert_eq!(lb.stats().late, 3);
    let bars = bars_of("5m", &evs);
    assert_eq!(bars.len(), 2);
    assert_eq!((bars[0].l, bars[1].l), (10.0, 10.1));
    // The next bar opens normally; the counter continues (late volume lands here).
    let more = lb.push(&trade(at(DAY, 10, 11, 0), 10.2, 400.0));
    match &more[..] {
        [DataEvent {
            change: DataChange::Bar { bar },
            ..
        }] => assert_eq!((bar.t, bar.v), (at(DAY, 10, 10, 0), 200.0)),
        other => panic!("expected one bar event, got {other:?}"),
    }
}

// --- 6. Seam ---------------------------------------------------------------------

#[test]
fn history_plus_live_equals_live_from_the_start() {
    let ticks = day_ticks(DAY);
    for iv in INTERVALS {
        for m in methods(iv) {
            let all = run(&mut live(iv, m), &ticks);
            let whole = bars_of(iv, &all);
            for cut in [1, 2, 3, 7, 40, 111, 200, ticks.len() - 2, ticks.len() - 1] {
                // History: what live produced up to `cut` (stands in for REST bars).
                let head = run(&mut live(iv, m), &ticks[..cut]);
                let mut store = FeedStore::new(key(iv));
                apply(&mut store, &head);
                let hist = store.bars().to_vec();
                let seq = store.seq();
                let cum: f64 = hist.iter().map(|b| b.v).sum::<f64>();
                let cum = if iv == "1d" && m != BarMethod::Ticks {
                    hist.last().map_or(0.0, |b| b.v) // card bar volume is the counter
                } else {
                    cum
                };
                // Live: a fresh subscription at the seam (snapshot), then the rest.
                let mut lb = LiveBars::new(
                    key(iv),
                    xist(),
                    LiveConfig {
                        first_seq: seq + 1,
                        ..cfg(m)
                    },
                )
                .expect("live");
                lb.resume(hist.last().cloned(), Some(cum)).expect("resume");
                let mut tail = vec![snapshot(&ticks[..cut])];
                tail.extend_from_slice(&ticks[cut..]);
                let evs = run(&mut lb, &tail);
                if let Some(first) = evs.first() {
                    assert_eq!(first.seq, seq + 1);
                }
                apply(&mut store, &evs);
                assert_eq!(store.bars(), &whole[..], "{iv} {} cut {cut}", m.name());
            }
        }
    }
}

#[test]
fn seam_bar_is_continued_with_patches_not_a_new_bar() {
    let hist = Bar {
        t: at(DAY, 11, 0, 0),
        o: 30.0,
        h: 31.0,
        l: 29.5,
        c: 30.5,
        v: 12_000.0,
    };
    let mut lb = live("1h", BarMethod::Ticks);
    lb.resume(Some(hist.clone()), Some(80_000.0))
        .expect("resume");
    let evs = run(
        &mut lb,
        &[
            CardTick {
                initial: true,
                last: Some(30.5),
                cum_volume: Some(81_000.0),
                ..CardTick::at(at(DAY, 11, 20, 0))
            },
            trade(at(DAY, 11, 21, 0), 31.5, 82_000.0),
        ],
    );
    assert!(evs
        .iter()
        .all(|e| matches!(e.change, DataChange::Patch { .. })));
    let mut store = FeedStore::with_bars(key("1h"), vec![hist], 0).expect("store");
    apply(&mut store, &evs);
    assert_eq!(
        store.bars(),
        &[Bar {
            t: at(DAY, 11, 0, 0),
            o: 30.0,
            h: 31.5,
            l: 29.5,
            c: 31.5,
            v: 14_000.0
        }]
    );
}

#[test]
fn resume_rejects_misaligned_bars_and_late_calls() {
    let bar = |t| Bar {
        t,
        o: 1.0,
        h: 1.0,
        l: 1.0,
        c: 1.0,
        v: 0.0,
    };
    let mut lb = live("1h", BarMethod::Ticks);
    assert!(lb.resume(Some(bar(at(DAY, 11, 30, 0))), None).is_err());
    assert!(lb.resume(Some(bar(at(SATURDAY, 11, 0, 0))), None).is_err());
    assert!(lb.resume(None, Some(-1.0)).is_err());
    assert!(lb.resume(Some(bar(at(DAY, 11, 0, 0))), None).is_ok());
    lb.push(&trade(at(DAY, 11, 5, 0), 1.0, 1.0));
    assert!(lb.resume(None, None).is_err());
}

// --- 8. Comparison ---------------------------------------------------------------

#[test]
fn compare_bars_counts_matches_and_extreme_differences() {
    let b = |t, o, h, l, c, v| Bar { t, o, h, l, c, v };
    let live = [
        b(1, 10.0, 11.0, 9.0, 10.5, 100.0),
        b(2, 10.5, 11.0, 10.0, 10.8, 200.0),
        b(3, 10.8, 12.0, 10.8, 11.0, 300.0),
    ];
    let rest = [
        b(2, 10.5, 11.0, 10.0, 10.8, 200.0),
        b(3, 10.8, 12.01, 10.8, 11.0, 330.0),
        b(4, 11.0, 11.0, 11.0, 11.0, 10.0),
    ];
    let d = compare_bars(&live, &rest);
    assert_eq!((d.matched, d.live_only, d.rest_only), (2, 1, 1));
    assert_eq!((d.all_exact, d.all_within_tol), (1, 1));
    assert_eq!((d.h.exact, d.v.exact, d.c.exact), (1, 1, 2));
    assert!((d.h.max_abs - 0.01).abs() < 1e-12);
    assert!((d.v.max_abs - 30.0).abs() < 1e-12);
    assert!((d.v.max_rel - 30.0 / 330.0).abs() < 1e-12);
    assert!((d.ratio(d.all_exact) - 0.5).abs() < 1e-12);
    let loose = compare_bars_with(
        &live,
        &rest,
        &DiffTolerance {
            price_abs: 0.005,
            volume_rel: 0.1,
        },
    );
    assert_eq!((loose.h.within_tol, loose.all_within_tol), (1, 1));
    let looser = compare_bars_with(
        &live,
        &rest,
        &DiffTolerance {
            price_abs: 0.011,
            volume_rel: 0.1,
        },
    );
    assert_eq!(looser.all_within_tol, 2);
    assert_eq!(compare_bars(&[], &[]).ratio(0), 0.0);
}
