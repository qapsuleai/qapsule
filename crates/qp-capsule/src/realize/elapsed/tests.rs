//! Tests of the elapsed-time phrases (moved out of `elapsed.rs`).

use super::*;
use qp_calendar::{builtin, days_from_civil};

fn xist() -> ExchangeCalendar {
    ExchangeCalendar::new(&builtin("XIST").unwrap()).unwrap()
}

/// UTC ms of a local Istanbul wall time (UTC+3 all year since 2016).
fn ist(y: i64, m: u32, d: u32, h: i64, min: i64) -> i64 {
    (days_from_civil(y, m, d) * 86_400 + h * 3600 + min * 60 - 3 * 3600) * 1000
}

fn both(e: Elapsed) -> (String, String) {
    (phrase(&e, Lang::Tr), phrase(&e, Lang::En))
}

#[test]
fn period_parse() {
    assert_eq!(Period::parse("1d"), Some(Period::Daily));
    assert_eq!(Period::parse("1day"), Some(Period::Daily));
    assert_eq!(Period::parse("60m"), Some(Period::Intraday(60)));
    assert_eq!(Period::parse("1h"), Some(Period::Intraday(60)));
    assert_eq!(Period::parse("1w"), Some(Period::Weekly));
    assert_eq!(Period::parse("1M"), Some(Period::Monthly));
    assert_eq!(Period::parse("x"), None);
}

/// VERIFY 12: daily bars across a weekend and holidays (XIST 2026: 23 April Thu,
/// 1 May Fri, 19 May Tue).
#[test]
fn daily_weekend_and_holiday() {
    let c = xist();
    let d = |y, m, dd| ist(y, m, dd, 10, 0);
    let p = Period::Daily;
    // Same day; calendar yesterday.
    assert_eq!(
        both(elapsed(&c, p, d(2026, 5, 6), d(2026, 5, 6))),
        ("bugün".into(), "today".into())
    );
    assert_eq!(
        both(elapsed(&c, p, d(2026, 5, 5), d(2026, 5, 6))),
        ("dün".into(), "yesterday".into())
    );
    // Friday → Monday: one trading day, not "yesterday".
    assert_eq!(
        both(elapsed(&c, p, d(2026, 5, 8), d(2026, 5, 11))),
        ("cuma günü".into(), "on Friday".into())
    );
    // Thursday 30 April → Monday 4 May: 1 May (holiday) and the weekend do not count.
    assert_eq!(
        both(elapsed(&c, p, d(2026, 4, 30), d(2026, 5, 4))),
        ("perşembe günü".into(), "on Thursday".into())
    );
    // Wednesday 22 April → Friday 24 April across the 23 April holiday.
    assert_eq!(
        both(elapsed(&c, p, d(2026, 4, 22), d(2026, 4, 24))),
        ("çarşamba günü".into(), "on Wednesday".into())
    );
    // Monday 18 May → Wednesday 20 May: 19 May is a holiday → 1 trading day.
    assert_eq!(
        elapsed(&c, p, d(2026, 5, 18), d(2026, 5, 20)),
        Elapsed::Weekday(0, None)
    );
    // Wednesday 29 April → Tuesday 5 May: 30 Apr, 4 May, 5 May = 3 trading days
    // (calendar 6 days, holiday and weekend excluded).
    assert_eq!(
        both(elapsed(&c, p, d(2026, 4, 29), d(2026, 5, 5))),
        ("3 işlem günü önce".into(), "3 trading days ago".into())
    );
    // Previous ISO week, 6+ trading days.
    assert_eq!(
        both(elapsed(&c, p, d(2026, 6, 4), d(2026, 6, 12))),
        ("geçen hafta".into(), "last week".into())
    );
    // Two weeks back, not the previous week: a count.
    assert_eq!(
        elapsed(&c, p, d(2026, 6, 5), d(2026, 6, 16)),
        Elapsed::TradingDaysAgo(7)
    );
    // Long ago: a date; another year adds the year.
    assert_eq!(
        both(elapsed(&c, p, d(2026, 3, 12), d(2026, 6, 16))),
        ("12 Mart'ta".into(), "on 12 March".into())
    );
    assert_eq!(
        both(elapsed(&c, p, d(2025, 9, 1), d(2026, 6, 16))),
        ("1 Eylül 2025'te".into(), "on 1 September 2025".into())
    );
}

#[allow(clippy::too_many_lines)]
/// VERIFY 12 / FIX-E1 B1: real Borsa Istanbul closures outside the R1 sample (feasts,
/// 15 July, 29 October, 2023 earthquake closure, half days), answers written from the
/// official calendar (`qp_calendar::XIST_HOLIDAYS`).
#[test]
fn real_xist_holidays() {
    let c = xist();
    let day = Period::Daily;
    let hour = Period::Intraday(60);
    let cases: &[(Period, i64, i64, &str, &str)] = &[
        // Feast of the Sacrifice 2026: 26 May half day, 27–29 May closed.
        (
            day,
            ist(2026, 5, 26, 10, 0),
            ist(2026, 6, 1, 10, 0),
            "salı günü",
            "on Tuesday",
        ),
        (
            day,
            ist(2026, 5, 25, 10, 0),
            ist(2026, 6, 2, 10, 0),
            "3 işlem günü önce",
            "3 trading days ago",
        ),
        (
            hour,
            ist(2026, 5, 26, 12, 0),
            ist(2026, 6, 1, 10, 0),
            "salı 12:00'de",
            "on Tuesday at 12:00",
        ),
        // 15 July 2026 (Wed).
        (
            day,
            ist(2026, 7, 14, 10, 0),
            ist(2026, 7, 16, 10, 0),
            "salı günü",
            "on Tuesday",
        ),
        // 2025: 1 May (Thu); Ramadan Feast 31 Mar–1 Apr; Sacrifice 5 Jun half, 6 and 9 Jun;
        // 28 Oct half, 29 Oct; 15 Jul (Tue).
        (
            day,
            ist(2025, 4, 30, 10, 0),
            ist(2025, 5, 2, 10, 0),
            "çarşamba günü",
            "on Wednesday",
        ),
        (
            hour,
            ist(2025, 4, 30, 17, 0),
            ist(2025, 5, 2, 10, 0),
            "çarşamba 17:00'de",
            "on Wednesday at 17:00",
        ),
        (
            day,
            ist(2025, 3, 28, 10, 0),
            ist(2025, 4, 2, 10, 0),
            "cuma günü",
            "on Friday",
        ),
        (
            day,
            ist(2025, 6, 4, 10, 0),
            ist(2025, 6, 12, 10, 0),
            "4 işlem günü önce",
            "4 trading days ago",
        ),
        (
            hour,
            ist(2025, 6, 4, 16, 0),
            ist(2025, 6, 10, 10, 0),
            "2 işlem günü önce",
            "2 trading days ago",
        ),
        (
            day,
            ist(2025, 10, 27, 10, 0),
            ist(2025, 11, 4, 10, 0),
            "5 işlem günü önce",
            "5 trading days ago",
        ),
        (
            day,
            ist(2025, 7, 14, 10, 0),
            ist(2025, 7, 23, 10, 0),
            "geçen hafta",
            "last week",
        ),
        // 2019 Sacrifice 12–14 Aug (Mon–Wed): Fri 9 Aug → Thu 15 Aug.
        (
            day,
            ist(2019, 8, 9, 10, 0),
            ist(2019, 8, 15, 10, 0),
            "cuma günü",
            "on Friday",
        ),
        // 2023 earthquake closure 8–14 Feb: Tue 7 Feb → Wed 15 Feb.
        (
            day,
            ist(2023, 2, 7, 10, 0),
            ist(2023, 2, 15, 10, 0),
            "salı günü",
            "on Tuesday",
        ),
        // 2027 (provisional): Ramadan Feast 9–11 Mar, 8 Mar half day → Fri 12 Mar.
        (
            hour,
            ist(2027, 3, 8, 12, 0),
            ist(2027, 3, 12, 11, 0),
            "pazartesi 12:00'de",
            "on Monday at 12:00",
        ),
    ];
    for &(p, e, n, tr, en) in cases {
        assert_eq!(
            both(elapsed(&c, p, e, n)),
            (tr.into(), en.into()),
            "{e} -> {n}"
        );
    }
    // The half day ends at 12:30: the 12:00 bar is its last 60-minute bar.
    let half = days_from_civil(2026, 5, 26);
    let span = c.day_span(half).unwrap();
    assert_eq!(span.close, ist(2026, 5, 26, 12, 30));
}

/// VERIFY 12: 60-minute bars across a weekend and a holiday.
#[test]
fn hourly_weekend_and_holiday() {
    let c = xist();
    let p = Period::Intraday(60);
    let now = ist(2026, 5, 4, 11, 0); // Monday after the 1 May holiday
    assert_eq!(
        both(elapsed(&c, p, now, now)),
        ("az önce".into(), "just now".into())
    );
    assert_eq!(
        both(elapsed(&c, p, ist(2026, 5, 4, 10, 0), now)),
        ("bir saat önce".into(), "an hour ago".into())
    );
    assert_eq!(
        both(elapsed(
            &c,
            p,
            ist(2026, 5, 4, 10, 0),
            ist(2026, 5, 4, 13, 0)
        )),
        ("3 saat önce".into(), "3 hours ago".into())
    );
    // Thursday 30 April 15:00 → Monday: holiday + weekend skipped, with the time.
    assert_eq!(
        both(elapsed(&c, p, ist(2026, 4, 30, 15, 0), now)),
        ("perşembe 15:00'te".into(), "on Thursday at 15:00".into())
    );
    // Calendar yesterday with the time; vowel harmony of the clock time.
    assert_eq!(
        both(elapsed(
            &c,
            p,
            ist(2026, 5, 4, 10, 0),
            ist(2026, 5, 5, 12, 0)
        )),
        ("dün 10:00'da".into(), "yesterday at 10:00".into())
    );
    assert_eq!(
        phrase(
            &elapsed(&c, p, ist(2026, 5, 4, 15, 30), ist(2026, 5, 5, 12, 0)),
            Lang::Tr
        ),
        "dün 15:30'da"
    );
    assert_eq!(
        phrase(
            &elapsed(&c, p, ist(2026, 5, 4, 12, 0), ist(2026, 5, 5, 12, 0)),
            Lang::Tr
        ),
        "dün 12:00'de"
    );
    // Wednesday 22 April 17:00 → Friday 24 April across the holiday.
    assert_eq!(
        phrase(
            &elapsed(&c, p, ist(2026, 4, 22, 17, 0), ist(2026, 4, 24, 10, 0)),
            Lang::Tr
        ),
        "çarşamba 17:00'de"
    );
    // Several trading days: a count, no time.
    assert_eq!(
        both(elapsed(&c, p, ist(2026, 4, 28, 15, 0), now)),
        ("3 işlem günü önce".into(), "3 trading days ago".into())
    );
    // 5-minute bars: minutes.
    assert_eq!(
        both(elapsed(
            &c,
            Period::Intraday(5),
            ist(2026, 5, 4, 10, 35),
            now
        )),
        ("25 dakika önce".into(), "25 minutes ago".into())
    );
}

#[test]
fn weekly_and_monthly() {
    let c = xist();
    let t = |m, d| ist(2026, m, d, 10, 0);
    assert_eq!(
        elapsed(&c, Period::Weekly, t(6, 15), t(6, 19)),
        Elapsed::ThisWeek
    );
    assert_eq!(
        both(elapsed(&c, Period::Weekly, t(6, 8), t(6, 19))).0,
        "geçen hafta"
    );
    assert_eq!(
        both(elapsed(&c, Period::Weekly, t(5, 25), t(6, 19))),
        ("3 hafta önce".into(), "3 weeks ago".into())
    );
    assert_eq!(
        both(elapsed(&c, Period::Monthly, t(6, 1), t(6, 19))).1,
        "this month"
    );
    assert_eq!(
        both(elapsed(&c, Period::Monthly, t(5, 1), t(6, 19))).0,
        "geçen ay"
    );
    assert_eq!(
        both(elapsed(&c, Period::Monthly, t(1, 2), t(6, 19))),
        ("5 ay önce".into(), "5 months ago".into())
    );
}

#[test]
fn singular_and_plural() {
    assert_eq!(
        phrase(&Elapsed::TradingDaysAgo(1), Lang::En),
        "1 trading day ago"
    );
    assert_eq!(
        phrase(&Elapsed::TradingDaysAgo(2), Lang::En),
        "2 trading days ago"
    );
    assert_eq!(
        phrase(&Elapsed::TradingDaysAgo(2), Lang::Tr),
        "2 işlem günü önce"
    );
    assert_eq!(phrase(&Elapsed::MinutesAgo(1), Lang::En), "a minute ago");
    assert_eq!(phrase(&Elapsed::WeeksAgo(1), Lang::En), "1 week ago");
}

#[test]
fn month_suffixes_follow_harmony() {
    let want = [
        "Ocak'ta",
        "Şubat'ta",
        "Mart'ta",
        "Nisan'da",
        "Mayıs'ta",
        "Haziran'da",
        "Temmuz'da",
        "Ağustos'ta",
        "Eylül'de",
        "Ekim'de",
        "Kasım'da",
        "Aralık'ta",
    ];
    for (i, w) in want.iter().enumerate() {
        let e = Elapsed::OnDate {
            day: 3,
            month: u32::try_from(i + 1).unwrap(),
            year: None,
        };
        assert_eq!(phrase(&e, Lang::Tr), format!("3 {w}"));
    }
}

#[test]
fn vague_has_no_digit() {
    let c = xist();
    for (a, b, p) in [
        (
            ist(2026, 4, 30, 15, 0),
            ist(2026, 5, 4, 11, 0),
            Period::Intraday(60),
        ),
        (
            ist(2026, 3, 12, 10, 0),
            ist(2026, 6, 16, 10, 0),
            Period::Daily,
        ),
        (
            ist(2026, 6, 10, 10, 0),
            ist(2026, 6, 16, 10, 0),
            Period::Daily,
        ),
    ] {
        let e = elapsed(&c, p, a, b);
        for l in [Lang::Tr, Lang::En] {
            assert!(
                !phrase_vague(&e, l).bytes().any(|x| x.is_ascii_digit()),
                "{e:?}"
            );
        }
    }
}

#[test]
fn deterministic() {
    let c = xist();
    let a = elapsed(
        &c,
        Period::Daily,
        ist(2026, 4, 30, 10, 0),
        ist(2026, 5, 4, 10, 0),
    );
    let b = elapsed(
        &c,
        Period::Daily,
        ist(2026, 4, 30, 10, 0),
        ist(2026, 5, 4, 10, 0),
    );
    assert_eq!(a, b);
    assert_eq!(phrase(&a, Lang::Tr), phrase(&b, Lang::Tr));
}
