//! (RP4 T3) Live bars from the trade stream (`BarMethod::Trades`, livebar rule 8):
//! alignment (K14) with the opening auction in the first bar and the closing session in
//! the last, volume Σ price·quantity (TL) with lots apart, special trades optional,
//! card ticks ignored, late trades, the seam, determinism. Hand-built trades only.

use qp_calendar::{builtin, parse_date, ExchangeCalendar};
use qp_schema::data::{DataChange, DataEvent, FeedKey};
use qp_schema::Bar;
use qp_source::{
    Applied, BarMethod, CardTick, FeedStore, LiveBars, LiveConfig, SpecialTrades, TradeTick,
    VolumeUnit,
};

const DAY: &str = "2026-11-18"; // Wednesday, full trading day

fn xist() -> ExchangeCalendar {
    ExchangeCalendar::new(&builtin("XIST").expect("XIST")).expect("calendar")
}

fn at(hh: i64, mm: i64, ss: i64) -> i64 {
    parse_date(DAY).expect("date") * 86_400_000 + ((hh * 60 + mm) * 60 + ss) * 1000 - 3 * 3_600_000
}

fn key(iv: &str) -> FeedKey {
    FeedKey {
        symbol: "TEST".into(),
        interval: iv.into(),
        feed: None,
    }
}

fn live(iv: &str, cfg: LiveConfig) -> LiveBars {
    LiveBars::new(key(iv), xist(), cfg).expect("live bars")
}

fn trades_cfg() -> LiveConfig {
    LiveConfig {
        method: BarMethod::Trades,
        ..LiveConfig::default()
    }
}

fn tr(t: i64, price: f64, qty: f64) -> TradeTick {
    TradeTick {
        t_ms: t,
        price,
        quantity: Some(qty),
        special: false,
    }
}

fn run(lb: &mut LiveBars, trades: &[TradeTick]) -> Vec<DataEvent> {
    let mut out = Vec::new();
    for t in trades {
        out.extend(lb.push_trade(t));
    }
    for w in out.windows(2) {
        assert_eq!(w[1].seq, w[0].seq + 1);
    }
    out
}

fn bars_of(iv: &str, evs: &[DataEvent]) -> Vec<Bar> {
    let mut s = FeedStore::new(key(iv));
    for e in evs {
        match s.apply(e).expect("event accepted") {
            Applied::NewBar { .. } | Applied::LiveUpdate { .. } => {}
            other => panic!("unexpected {other:?}"),
        }
    }
    if !s.is_empty() {
        qp_schema::validate_bars(s.bars()).expect("valid bars");
    }
    s.bars().to_vec()
}

/// A day of trades: opening auction match at 09:55:00 (before the 10:00 open), continuous
/// trading, a special trade, the closing auction at 18:05 and a trade at the closing
/// price at 18:08.
fn day() -> Vec<TradeTick> {
    vec![
        tr(at(9, 55, 0), 100.0, 500.0), // opening auction → first bar
        tr(at(10, 0, 10), 100.5, 10.0),
        tr(at(10, 0, 50), 99.75, 20.0),
        TradeTick {
            special: true,
            ..tr(at(10, 2, 0), 150.0, 1000.0)
        },
        tr(at(10, 6, 0), 101.0, 5.0),
        tr(at(17, 58, 0), 102.0, 7.0),
        tr(at(18, 5, 0), 102.5, 300.0), // closing auction → last bar
        tr(at(18, 8, 0), 102.5, 40.0),  // trades at the closing price → last bar
    ]
}

#[test]
fn trades_build_calendar_aligned_bars_with_auctions_in_first_and_last() {
    let mut lb = live("5m", trades_cfg());
    let bars = bars_of("5m", &run(&mut lb, &day()));
    assert_eq!(bars.len(), 3, "{bars:?}");
    // First bar 10:00: opening auction + the 10:00 trades, special trade excluded.
    assert_eq!(bars[0].t, at(10, 0, 0));
    assert_eq!(
        (bars[0].o, bars[0].h, bars[0].l, bars[0].c),
        (100.0, 100.5, 99.75, 99.75)
    );
    assert_eq!(bars[0].v, 100.0 * 500.0 + 100.5 * 10.0 + 99.75 * 20.0);
    assert_eq!(bars[1].t, at(10, 5, 0));
    // Last bar of the day (17:55): 17:58 + closing auction + closing price trades.
    assert_eq!(bars[2].t, at(17, 55, 0));
    assert_eq!(bars[2].c, 102.5);
    assert_eq!(bars[2].v, 102.0 * 7.0 + 102.5 * 300.0 + 102.5 * 40.0);
    assert_eq!(lb.current_quantity(), Some(347.0));
    let st = lb.stats();
    assert_eq!(st.trades, 8);
    assert_eq!(st.special_excluded, 1);
    assert_eq!(st.off_session, 0);

    // Daily bar: one bar, all regular trades.
    let mut d = live("1d", trades_cfg());
    let db = bars_of("1d", &run(&mut d, &day()));
    assert_eq!(db.len(), 1);
    assert_eq!(
        (db[0].o, db[0].h, db[0].l, db[0].c),
        (100.0, 102.5, 99.75, 102.5)
    );
    assert_eq!(d.current_quantity(), Some(882.0));
}

#[test]
fn special_trades_join_only_when_included_and_quantity_unit() {
    let mut inc = live(
        "5m",
        LiveConfig {
            special_trades: SpecialTrades::Include,
            ..trades_cfg()
        },
    );
    let b = bars_of("5m", &run(&mut inc, &day()));
    assert_eq!(b[0].h, 150.0);
    assert_eq!(inc.stats().special_excluded, 0);

    let mut q = live(
        "5m",
        LiveConfig {
            volume: VolumeUnit::Quantity,
            ..trades_cfg()
        },
    );
    let b = bars_of("5m", &run(&mut q, &day()));
    assert_eq!(b[0].v, 530.0);
}

#[test]
fn card_ticks_build_nothing_under_trades_and_trades_nothing_under_cards() {
    let mut lb = live("5m", trades_cfg());
    let card = CardTick {
        last: Some(100.0),
        cum_volume: Some(10.0),
        phase: Some(3),
        ..CardTick::at(at(10, 1, 0))
    };
    assert!(lb.push(&card).is_empty());
    assert_eq!(lb.stats().wrong_source, 1);
    assert_eq!(lb.stats().last_phase, Some(3));
    // The card did not start the builder: a seam is still possible.
    assert!(lb.resume(None, None).is_ok());

    let mut cardbars = live("5m", LiveConfig::default());
    assert!(cardbars.push_trade(&tr(at(10, 1, 0), 1.0, 1.0)).is_empty());
    assert_eq!(cardbars.stats().wrong_source, 1);
}

#[test]
fn trades_late_off_session_missing_quantity_and_bad_price() {
    let mut lb = live("1m", trades_cfg());
    let mut evs = run(&mut lb, &[tr(at(10, 0, 5), 10.0, 1.0)]);
    assert!(lb.close_until(at(10, 1, 0)).is_some());
    // Late (bar final), night, bad price, no quantity.
    evs.extend(run(
        &mut lb,
        &[
            tr(at(10, 0, 30), 11.0, 1.0),
            tr(at(2, 0, 0), 11.0, 1.0),
            tr(at(10, 1, 1), f64::NAN, 1.0),
            TradeTick {
                quantity: None,
                ..tr(at(10, 1, 2), 12.0, 0.0)
            },
        ],
    ));
    let st = lb.stats();
    assert_eq!(
        (st.late, st.off_session, st.invalid_fields, st.no_quantity),
        (1, 1, 1, 1)
    );
    let bars = bars_of("1m", &evs);
    assert_eq!(bars.len(), 2);
    assert_eq!(bars[1].c, 12.0);
    assert_eq!(bars[1].v, 0.0);
}

#[test]
fn seam_bar_is_continued_by_trades() {
    let hist = Bar {
        t: at(10, 0, 0),
        o: 50.0,
        h: 51.0,
        l: 49.0,
        c: 50.5,
        v: 1000.0,
    };
    let mut lb = live("5m", trades_cfg());
    lb.resume(Some(hist.clone()), None).unwrap();
    let evs = run(&mut lb, &[tr(at(10, 3, 0), 52.0, 2.0)]);
    assert_eq!(evs.len(), 1);
    match &evs[0].change {
        DataChange::Patch { t, .. } => assert_eq!(*t, hist.t),
        other => panic!("{other:?}"),
    }
    let (b, fin) = lb.current().unwrap();
    assert!(!fin);
    assert_eq!((b.h, b.c, b.v), (52.0, 52.0, 1104.0));
}

#[test]
fn same_trades_give_byte_equal_events() {
    let a = run(&mut live("1m", trades_cfg()), &day());
    let b = run(&mut live("1m", trades_cfg()), &day());
    assert_eq!(
        serde_json::to_string(&a).unwrap(),
        serde_json::to_string(&b).unwrap()
    );
    assert_eq!(BarMethod::Trades.name(), "trades");
}
