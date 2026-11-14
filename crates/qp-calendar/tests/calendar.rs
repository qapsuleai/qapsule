//! BIST and NYSE bar boundaries: intraday, daily, weekly, monthly; daylight-saving
//! transitions, half days, holidays; display time zone does not change alignment.

use qp_calendar::{builtin, days_from_civil, format_display, BarSpan, ExchangeCalendar, Interval};
use qp_schema::Bar;

const H: i64 = 3_600_000;
const M: i64 = 60_000;

/// UTC milliseconds of a civil UTC date-time.
fn utc(y: i64, mo: u32, d: u32, h: i64, mi: i64) -> i64 {
    days_from_civil(y, mo, d) * 86_400_000 + h * H + mi * M
}

fn cal(id: &str) -> ExchangeCalendar {
    ExchangeCalendar::new(&builtin(id).unwrap()).unwrap()
}

fn span(o: i64, c: i64) -> BarSpan {
    BarSpan { open: o, close: c }
}

#[test]
fn nyse_daily_bars_follow_new_york_dst() {
    let c = cal("XNYS");
    // Spring forward on Sunday 2026-03-08: EST (UTC-5) before, EDT (UTC-4) after.
    assert_eq!(
        c.bar_at(Interval::Day, utc(2026, 3, 6, 15, 0)),
        Some(span(utc(2026, 3, 6, 14, 30), utc(2026, 3, 6, 21, 0)))
    );
    assert_eq!(
        c.bar_at(Interval::Day, utc(2026, 3, 9, 15, 0)),
        Some(span(utc(2026, 3, 9, 13, 30), utc(2026, 3, 9, 20, 0)))
    );
    // Fall back on Sunday 2026-11-01.
    assert_eq!(
        c.bar_at(Interval::Day, utc(2026, 10, 30, 14, 0)),
        Some(span(utc(2026, 10, 30, 13, 30), utc(2026, 10, 30, 20, 0)))
    );
    assert_eq!(
        c.bar_at(Interval::Day, utc(2026, 11, 2, 15, 0)),
        Some(span(utc(2026, 11, 2, 14, 30), utc(2026, 11, 2, 21, 0)))
    );
    // Before the open / after the close: no bar.
    assert_eq!(c.bar_at(Interval::Day, utc(2026, 3, 9, 13, 29)), None);
    assert_eq!(c.bar_at(Interval::Day, utc(2026, 3, 9, 20, 0)), None);
}

#[test]
fn nyse_intraday_bars_on_dst_day_and_half_day() {
    let c = cal("XNYS");
    // First trading day after spring forward: 1h bars 13:30..20:00 UTC, last one 30 min.
    let bars = c.bars_between(
        Interval::parse("1h").unwrap(),
        utc(2026, 3, 9, 0, 0),
        utc(2026, 3, 10, 0, 0),
    );
    let opens: Vec<i64> = bars.iter().map(|b| b.open).collect();
    assert_eq!(
        opens,
        (0..7)
            .map(|k| utc(2026, 3, 9, 13, 30) + k * H)
            .collect::<Vec<_>>()
    );
    assert_eq!(bars.last().unwrap().close, utc(2026, 3, 9, 20, 0));
    // Day after Thanksgiving: closes 13:00 local = 18:00 UTC (EST).
    let half = c.bars_between(
        Interval::parse("1h").unwrap(),
        utc(2026, 11, 27, 0, 0),
        utc(2026, 11, 28, 0, 0),
    );
    assert_eq!(half.len(), 4);
    assert_eq!(
        half[0],
        span(utc(2026, 11, 27, 14, 30), utc(2026, 11, 27, 15, 30))
    );
    assert_eq!(
        half[3],
        span(utc(2026, 11, 27, 17, 30), utc(2026, 11, 27, 18, 0))
    );
    assert_eq!(
        c.bar_at(Interval::Day, utc(2026, 11, 27, 15, 0)),
        Some(span(utc(2026, 11, 27, 14, 30), utc(2026, 11, 27, 18, 0)))
    );
    assert_eq!(c.bar_at(Interval::Day, utc(2026, 11, 27, 18, 30)), None);
    // 5-minute bars: 78 on a full day.
    let five = c.bars_between(
        Interval::parse("5m").unwrap(),
        utc(2026, 3, 9, 0, 0),
        utc(2026, 3, 10, 0, 0),
    );
    assert_eq!(five.len(), 78);
}

#[test]
fn nyse_holiday_week_and_month() {
    let c = cal("XNYS");
    // Thanksgiving 2026-11-26 is closed.
    assert_eq!(c.bar_at(Interval::Day, utc(2026, 11, 26, 16, 0)), None);
    // Its week: Monday 23rd open .. Friday 27th early close.
    assert_eq!(
        c.bar_at(Interval::Week, utc(2026, 11, 25, 16, 0)),
        Some(span(utc(2026, 11, 23, 14, 30), utc(2026, 11, 27, 18, 0)))
    );
    // March 2026 spans the DST change: opens 2 Mar EST, closes 31 Mar EDT.
    assert_eq!(
        c.bar_at(Interval::Month, utc(2026, 3, 16, 15, 0)),
        Some(span(utc(2026, 3, 2, 14, 30), utc(2026, 3, 31, 20, 0)))
    );
    let weeks = c.bars_between(
        Interval::Week,
        utc(2026, 11, 1, 0, 0),
        utc(2026, 12, 1, 0, 0),
    );
    // Weeks opening in [1 Nov, 1 Dec): 2, 9, 16, 23 and 30 Nov.
    assert_eq!(weeks.len(), 5);
    assert_eq!(weeks[4].open, utc(2026, 11, 30, 14, 30));
}

#[test]
fn bist_bars_in_istanbul_time_including_historic_dst() {
    let c = cal("XIST");
    // Since 2016 Istanbul is UTC+3 all year: 10:00-18:00 local = 07:00-15:00 UTC.
    assert_eq!(
        c.bar_at(Interval::Day, utc(2026, 11, 13, 9, 0)),
        Some(span(utc(2026, 11, 13, 7, 0), utc(2026, 11, 13, 15, 0)))
    );
    // In 2015 Turkey still had DST, starting Sunday 2015-03-29 (UTC+2 -> UTC+3).
    assert_eq!(
        c.bar_at(Interval::Day, utc(2015, 3, 27, 9, 0)),
        Some(span(utc(2015, 3, 27, 8, 0), utc(2015, 3, 27, 16, 0)))
    );
    assert_eq!(
        c.bar_at(Interval::Day, utc(2015, 3, 30, 9, 0)),
        Some(span(utc(2015, 3, 30, 7, 0), utc(2015, 3, 30, 15, 0)))
    );
    // 2015 DST ended late (Sunday 8 Nov, postponed for the election): Friday 6 Nov still
    // UTC+3, Monday 9 Nov UTC+2.
    assert_eq!(
        c.bar_at(Interval::Day, utc(2015, 11, 6, 9, 0)),
        Some(span(utc(2015, 11, 6, 7, 0), utc(2015, 11, 6, 15, 0)))
    );
    assert_eq!(
        c.bar_at(Interval::Day, utc(2015, 11, 9, 9, 0)),
        Some(span(utc(2015, 11, 9, 8, 0), utc(2015, 11, 9, 16, 0)))
    );
}

#[test]
fn bist_half_day_holiday_week_month() {
    let c = cal("XIST");
    // Eve of the Ramadan Feast 2026-03-19: closes 12:30 local = 09:30 UTC.
    assert_eq!(
        c.bar_at(Interval::Day, utc(2026, 3, 19, 8, 0)),
        Some(span(utc(2026, 3, 19, 7, 0), utc(2026, 3, 19, 9, 30)))
    );
    let hours = c.bars_between(
        Interval::parse("1h").unwrap(),
        utc(2026, 3, 19, 0, 0),
        utc(2026, 3, 20, 0, 0),
    );
    assert_eq!(hours.len(), 3);
    assert_eq!(
        hours[2],
        span(utc(2026, 3, 19, 9, 0), utc(2026, 3, 19, 9, 30))
    );
    // Feast day 2026-03-20 is closed; that week ends with the half day.
    assert_eq!(c.bar_at(Interval::Day, utc(2026, 3, 20, 8, 0)), None);
    assert_eq!(
        c.bar_at(Interval::Week, utc(2026, 3, 17, 8, 0)),
        Some(span(utc(2026, 3, 16, 7, 0), utc(2026, 3, 19, 9, 30)))
    );
    // March 2026: 1 Mar is a Sunday, so the month opens Monday 2 Mar.
    assert_eq!(
        c.bar_at(Interval::Month, utc(2026, 3, 10, 8, 0)),
        Some(span(utc(2026, 3, 2, 7, 0), utc(2026, 3, 31, 15, 0)))
    );
    // 8-hour session in 4h bars: two bars.
    let four = c.bars_between(
        Interval::parse("4h").unwrap(),
        utc(2026, 11, 13, 0, 0),
        utc(2026, 11, 14, 0, 0),
    );
    assert_eq!(
        four,
        vec![
            span(utc(2026, 11, 13, 7, 0), utc(2026, 11, 13, 11, 0)),
            span(utc(2026, 11, 13, 11, 0), utc(2026, 11, 13, 15, 0)),
        ]
    );
}

#[test]
fn resample_minutes_into_session_aligned_days() {
    let c = cal("XNYS");
    // 1-minute bars from 09:00 to 16:30 New York time on Mon 9 and Tue 10 Mar 2026 (EDT):
    // the pre-market and after-hours minutes are dropped; one daily bar per session.
    let mut bars = Vec::new();
    for day in [9, 10] {
        let start = utc(2026, 3, day, 13, 0);
        for k in 0..450 {
            let x = 100.0 + f64::from(k % 17) * 0.01;
            bars.push(Bar {
                t: start + i64::from(k) * M,
                o: x,
                h: x + 0.05,
                l: x - 0.05,
                c: x,
                v: 1.0,
            });
        }
    }
    let (daily, dropped) = c.resample(Interval::Day, &bars).unwrap();
    assert_eq!(daily.len(), 2);
    assert_eq!(daily[0].t, utc(2026, 3, 9, 13, 30));
    assert_eq!(daily[1].t, utc(2026, 3, 10, 13, 30));
    assert_eq!(daily[0].v, 390.0); // 6.5 h of minutes
    assert_eq!(dropped, 2 * (450 - 390));
    let (hourly, _) = c.resample(Interval::parse("1h").unwrap(), &bars).unwrap();
    assert_eq!(hourly.len(), 14);
    assert!(c
        .resample(Interval::Day, &[bars[1].clone(), bars[0].clone()])
        .is_err());
}

#[test]
fn display_time_zone_does_not_change_alignment() {
    let c = cal("XIST");
    let t = utc(2026, 11, 13, 9, 17);
    let bar = c.bar_at(Interval::Day, t).unwrap();
    // The same bar shown to a user in Istanbul and to one in New York.
    assert_eq!(
        format_display(bar.open, "Europe/Istanbul").unwrap(),
        "2026-11-13T10:00:00+03:00"
    );
    assert_eq!(
        format_display(bar.open, "America/New_York").unwrap(),
        "2026-11-13T02:00:00-05:00"
    );
    assert_eq!(
        format_display(bar.open, "UTC").unwrap(),
        "2026-11-13T07:00:00+00:00"
    );
    // Alignment has no display-zone input at all; the bar is the exchange's session.
    assert_eq!(bar, span(utc(2026, 11, 13, 7, 0), utc(2026, 11, 13, 15, 0)));
    assert!(format_display(bar.open, "Europe/Berlin").is_err());
}

#[test]
fn interval_parsing_and_bad_calendars() {
    assert_eq!(Interval::parse("15m").unwrap(), Interval::Minutes(15));
    assert_eq!(Interval::parse("4h").unwrap(), Interval::Minutes(240));
    assert_eq!(Interval::parse("1M").unwrap(), Interval::Month);
    for bad in ["", "m", "0m", "2d", "1y", "1.5h"] {
        assert!(Interval::parse(bad).is_err(), "{bad}");
    }
    let mut bad = builtin("XNYS").unwrap();
    bad.tz = "Mars/Olympus".into();
    assert!(ExchangeCalendar::new(&bad).is_err());
    let mut bad = builtin("XNYS").unwrap();
    bad.holidays.push("2026-02-30".into());
    assert!(ExchangeCalendar::new(&bad).is_err());
    let mut bad = builtin("XNYS").unwrap();
    bad.sessions[0].close = "09:00".into();
    assert!(ExchangeCalendar::new(&bad).is_err());
}

#[test]
fn builtin_calendars_validate_against_the_schema() {
    for id in ["XIST", "XNYS"] {
        let v = serde_json::to_value(builtin(id).unwrap()).unwrap();
        qp_schema::validate("Calendar", &v).unwrap();
    }
    // The committed example files are the built-in calendars.
    for (id, file) in [("XIST", "xist"), ("XNYS", "xnys")] {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!("../../fixtures/calendar/{file}.json"));
        let v: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap();
        qp_schema::validate("Calendar", &v).unwrap();
        assert_eq!(
            serde_json::from_value::<qp_schema::Calendar>(v).unwrap(),
            builtin(id).unwrap()
        );
    }
}
