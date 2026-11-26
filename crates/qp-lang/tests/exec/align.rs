//! Higher timeframes (PR §3.2, VERIFY 12): XIST daily and 4-hour bars on the exchange
//! session (holidays, a half day, a bar outside the session, an incomplete first day) and
//! a 24/7 instrument on UTC. `align` reads only closed higher bars.

use qp_calendar::{builtin, ExchangeCalendar, Interval};
use qp_lang::Align;
use qp_schema::Bar;
use serde_json::json;

use crate::support::*;

/// `align(<field>@<tf>)` for close and high on `1d` and `4h`.
fn aligned() -> serde_json::Value {
    prog(
        json!([
            {"id": "dc", "op": "input", "field": "close", "tf": "1d"},
            {"id": "dh", "op": "input", "field": "high", "tf": "1d"},
            {"id": "hc", "op": "input", "field": "close", "tf": "4h"},
            {"id": "d", "op": "align", "x": "dc"},
            {"id": "h", "op": "align", "x": "dh"},
            {"id": "q", "op": "align", "x": "hc"}
        ]),
        json!([
            {"name": "d", "kind": "series", "node": "d"},
            {"name": "h", "kind": "series", "node": "h"},
            {"name": "q", "kind": "series", "node": "q"}
        ]),
    )
}

/// 2024-04-08 00:00 UTC (Monday). XIST: 09 April is a half day (closes 12:30 local),
/// 10–12 April are holidays.
const APR8: i64 = 1_712_534_400_000;
const DAY: i64 = 24 * HOUR;

/// XIST hourly bars of 8–16 April 2024 from the calendar, plus one bar outside the
/// session (8 April 16:00 UTC), with closes `100 + index`.
fn xist_bars(cal: &ExchangeCalendar) -> Vec<Bar> {
    let mut ts: Vec<i64> = cal
        .bars_between(Interval::Minutes(60), APR8, APR8 + 9 * DAY)
        .iter()
        .map(|s| s.open)
        .collect();
    ts.push(APR8 + 16 * HOUR);
    ts.sort_unstable();
    ts.iter()
        .enumerate()
        .map(|(i, &t)| {
            let c = 100.0 + i as f64;
            Bar {
                t,
                o: c,
                h: c + 0.5,
                l: c - 0.5,
                c,
                v: 1.0,
            }
        })
        .collect()
}

fn at(b: &[Bar], t: i64) -> usize {
    b.iter().position(|x| x.t == t).expect("bar at t")
}

#[test]
fn xist_daily_and_four_hour_follow_the_session() {
    let cal = ExchangeCalendar::new(&builtin("XIST").unwrap()).unwrap();
    let b = xist_bars(&cal);
    // 8 April: 8 bars (07–14 UTC), the 16:00 outside bar, 9 April: 07, 08, 09 (half day,
    // the 09:00 bar closes at 09:30), then 15 and 16 April.
    assert_eq!(b.len(), 8 + 1 + 3 + 8 + 8);
    let c = checked(&aligned());
    let x = run_aligned(&c, &b, Align::for_calendar(Some("XIST")));
    let (d, h, q) = (values(&x, "d"), values(&x, "h"), values(&x, "q"));
    let close = |t: i64| Some(b[at(&b, t)].c);
    let hr = |day: i64, hour: i64| APR8 + day * DAY + hour * HOUR;
    // Daily: nothing before 8 April closes (its last bar, 14:00 UTC = 18:00 local).
    assert_eq!(d[at(&b, hr(0, 13))], None);
    assert_eq!(d[at(&b, hr(0, 14))], close(hr(0, 14)));
    assert_eq!(h[at(&b, hr(0, 14))], close(hr(0, 14)).map(|c| c + 0.5));
    // The bar outside the session enters no daily bar and changes nothing.
    assert_eq!(d[at(&b, hr(0, 16))], close(hr(0, 14)));
    assert_eq!(x.dropped(), 1);
    // Half day: the daily bar closes with the 09:00 bar (closing 09:30 UTC).
    assert_eq!(d[at(&b, hr(1, 8))], close(hr(0, 14)));
    assert_eq!(d[at(&b, hr(1, 9))], close(hr(1, 9)));
    // Holidays: 15 April's bars still read 9 April until 15 April closes.
    assert_eq!(d[at(&b, hr(7, 7))], close(hr(1, 9)));
    assert_eq!(d[at(&b, hr(7, 14))], close(hr(7, 14)));
    // 4h on the session: 07–11 and 11–15 UTC; the half day's only 4h bar ends at 09:30.
    assert_eq!(q[at(&b, hr(0, 9))], None);
    assert_eq!(q[at(&b, hr(0, 10))], close(hr(0, 10)));
    assert_eq!(q[at(&b, hr(0, 13))], close(hr(0, 10)));
    assert_eq!(q[at(&b, hr(0, 14))], close(hr(0, 14)));
    assert_eq!(q[at(&b, hr(1, 8))], close(hr(0, 14)));
    assert_eq!(q[at(&b, hr(1, 9))], close(hr(1, 9)));
    // Provenance: the daily value at 15 April 14:00 rests on that day's first bar.
    let lin = x
        .lineage(x.output("d").unwrap(), at(&b, hr(7, 14)))
        .unwrap();
    assert_eq!(lin.lo, at(&b, hr(7, 7)) as u64);
}

#[test]
fn incomplete_first_higher_bar_never_forms() {
    let cal = ExchangeCalendar::new(&builtin("XIST").unwrap()).unwrap();
    let all = xist_bars(&cal);
    // Start at 8 April 09:00 UTC: that day opened before the first bar.
    let b: Vec<Bar> = all.into_iter().filter(|x| x.t >= APR8 + 9 * HOUR).collect();
    let c = checked(&aligned());
    let x = run_aligned(&c, &b, Align::for_calendar(Some("XIST")));
    let d = values(&x, "d");
    let q = values(&x, "q");
    let i14 = at(&b, APR8 + 14 * HOUR);
    assert_eq!(d[i14], None, "8 April is incomplete");
    // The 07–11 4h bar is incomplete too; 11–15 is complete.
    assert_eq!(q[at(&b, APR8 + 10 * HOUR)], None);
    assert_eq!(q[i14], Some(b[i14].c));
    let half = at(&b, APR8 + DAY + 9 * HOUR);
    assert_eq!(d[half], Some(b[half].c));
}

#[test]
fn utc_alignment_for_24_7_instruments() {
    let b = bars(24 * 3, 4);
    let c = checked(&aligned());
    for align in [
        Align::Utc,
        Align::for_calendar(None),
        Align::for_calendar(Some("NOPE")),
    ] {
        let x = run_aligned(&c, &b, align);
        let (d, q) = (values(&x, "d"), values(&x, "q"));
        assert_eq!(d[22], None);
        assert_eq!(
            d[23],
            Some(b[23].c),
            "the UTC day closes with its 23:00 bar"
        );
        assert_eq!(d[30], Some(b[23].c));
        assert_eq!(d[47], Some(b[47].c));
        assert_eq!(q[2], None);
        assert_eq!(q[3], Some(b[3].c));
        assert_eq!(q[6], Some(b[3].c));
        assert_eq!(q[7], Some(b[7].c));
        assert_eq!(x.dropped(), 0);
    }
}

#[test]
fn a_gap_closes_the_higher_bar_at_the_next_bar() {
    // Missing 20:00–23:00: the UTC day closes at the first bar of the next day.
    let mut b = bars(48, 8);
    b.retain(|x| !(T0 + 20 * HOUR..T0 + 24 * HOUR).contains(&x.t));
    let c = checked(&aligned());
    let x = run_aligned(&c, &b, Align::Utc);
    let d = values(&x, "d");
    let last_of_day = at(&b, T0 + 19 * HOUR);
    assert_eq!(d[last_of_day], None);
    assert_eq!(d[last_of_day + 1], Some(b[last_of_day].c));
}
