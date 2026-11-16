//! Elapsed phrase ("Süre ifadesi", R4; `docs/CONCEPTS.md` "Gösterim (R4)", K43): how long
//! ago something happened, in everyday words that follow the bar period and the exchange
//! calendar instead of a bar count ("6 bar önce" → "6 işlem günü önce", "2 saat önce").
//!
//! Deterministic: two bar times (UTC ms) and an [`ExchangeCalendar`]; no clock. Weekends
//! and holidays are not trading days. Times of day are the exchange's local time (the
//! calendar's zone), never the viewer's. A bar is named by its **open** time (`Bar::t`,
//! as the chart labels it).
//!
//! Two steps, like the template realizer: [`elapsed`] reads the two times into a
//! language-independent [`Elapsed`]; [`phrase`] renders it in Turkish or English
//! ([`phrase_vague`]: the same without any digit, for a redacted reader, K33).
//!
//! # Rules
//!
//! Daily bars (`1d`), with `n` = trading days after the event's day up to now's day:
//!
//! | `n` | [`Elapsed`] | TR | EN |
//! |-----|-------------|----|----|
//! | 0 | `Today` | bugün | today |
//! | 1, calendar yesterday | `Yesterday` | dün | yesterday |
//! | 1, across a weekend/holiday | `Weekday` | cuma günü | on Friday |
//! | 2–5 | `TradingDaysAgo` | 3 işlem günü önce | 3 trading days ago |
//! | 6–19, the previous ISO week | `LastWeek` | geçen hafta | last week |
//! | 6–19, otherwise | `TradingDaysAgo` | 8 işlem günü önce | 8 trading days ago |
//! | ≥ 20 | `OnDate` | 12 Mart'ta (2025'te: 12 Mart 2025'te) | on 12 March (2025) |
//!
//! Intraday bars (`Nm`, `Nh`): on now's trading day `JustNow` (same bar time),
//! `MinutesAgo` (< 60 min), `HoursAgo` (whole hours); on the previous trading day
//! `Yesterday` / `Weekday` **with the time** ("dün 15:00'te", "cuma 15:00'te"); then as
//! daily (2–5 trading days, last week, date).
//!
//! Weekly (`1w`): this week / last week / `N` weeks ago. Monthly (`1M`): this month /
//! last month / `N` months ago.
//!
//! Turkish suffixes follow the spoken form (`15:00'te`, `10:00'da`, `15:30'da`,
//! `Mart'ta`, `Eylül'de`, `2025'te`); Turkish keeps the noun singular after a numeral
//! ("3 işlem günü"), English pluralizes except for one ("1 trading day", "an hour").

use qp_calendar::{civil_from_days, weekday_index, ExchangeCalendar, Interval};

use crate::realize::morph::{number_last_word, Word};
use crate::Lang;

const MS_PER_MIN: i64 = 60_000;
const MS_PER_DAY: i64 = 86_400_000;

/// Bar period, as far as the elapsed phrase cares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Period {
    /// Intraday bars of this many minutes.
    Intraday(u32),
    /// One bar per trading day.
    Daily,
    /// One bar per ISO week.
    Weekly,
    /// One bar per calendar month.
    Monthly,
}

impl Period {
    /// From an interval string (`60m`, `1h`, `1d`, `1w`, `1M`; `qp-calendar` syntax), also
    /// the engine's `day` / `1day` series suffixes.
    #[must_use]
    pub fn parse(interval: &str) -> Option<Self> {
        match interval {
            "day" | "1day" | "d" | "D" | "1D" => return Some(Self::Daily),
            _ => {}
        }
        Some(match Interval::parse(interval).ok()? {
            Interval::Minutes(n) => Self::Intraday(n),
            Interval::Day => Self::Daily,
            Interval::Week => Self::Weekly,
            Interval::Month => Self::Monthly,
        })
    }
}

/// A local time of day (exchange zone): hours and minutes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct At {
    /// Hour, 0–23.
    pub h: u32,
    /// Minute, 0–59.
    pub m: u32,
}

/// How long ago, language-independent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Elapsed {
    /// Intraday, the same bar time.
    JustNow,
    /// Intraday, earlier today: minutes (1–59).
    MinutesAgo(u32),
    /// Intraday, earlier today: whole hours.
    HoursAgo(u32),
    /// Daily: today.
    Today,
    /// The previous trading day, which is the calendar's yesterday (intraday: at a time).
    Yesterday(Option<At>),
    /// The previous trading day across a weekend or holiday: its weekday (Monday = 0)
    /// (intraday: at a time).
    Weekday(u32, Option<At>),
    /// This many trading days ago (≥ 2).
    TradingDaysAgo(u32),
    /// The previous ISO week (daily: 6–19 trading days ago; weekly: one bar ago).
    LastWeek,
    /// Weekly: this week.
    ThisWeek,
    /// Weekly: this many weeks ago (≥ 2).
    WeeksAgo(u32),
    /// Monthly: this month.
    ThisMonth,
    /// Monthly: last month.
    LastMonth,
    /// Monthly: this many months ago (≥ 2).
    MonthsAgo(u32),
    /// On a date; `year` is `None` in now's year.
    OnDate {
        /// Day of month.
        day: u32,
        /// Month, 1–12.
        month: u32,
        /// Year, when not now's year.
        year: Option<i64>,
    },
}

fn local_ms(cal: &ExchangeCalendar, t: i64) -> i64 {
    t + i64::from(cal.zone().offset_at(t.div_euclid(1000))) * 1000
}

fn at_of(cal: &ExchangeCalendar, t: i64) -> At {
    let mins = local_ms(cal, t).rem_euclid(MS_PER_DAY) / MS_PER_MIN;
    At {
        h: u32::try_from(mins / 60).unwrap_or(0),
        m: u32::try_from(mins % 60).unwrap_or(0),
    }
}

/// Trading days in `(from, to]` (local day numbers).
fn trading_days_between(cal: &ExchangeCalendar, from: i64, to: i64) -> u32 {
    let n = ((from + 1)..=to).filter(|d| cal.is_trading_day(*d)).count();
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// Monday of the ISO week of local day `d`.
fn week_start(d: i64) -> i64 {
    d - i64::from(weekday_index(d))
}

fn date_of(day: i64, now_day: i64) -> Elapsed {
    let (y, m, d) = civil_from_days(day);
    let (ny, _, _) = civil_from_days(now_day);
    Elapsed::OnDate {
        day: d,
        month: m,
        year: (y != ny).then_some(y),
    }
}

/// Elapsed time from bar time `event_t` to bar time `now_t` (UTC ms) for bars of
/// `period` on `cal` (see the module docs). An event after now counts as now.
#[must_use]
pub fn elapsed(cal: &ExchangeCalendar, period: Period, event_t: i64, now_t: i64) -> Elapsed {
    let event_t = event_t.min(now_t);
    let (ed, nd) = (cal.local_day(event_t), cal.local_day(now_t));
    match period {
        Period::Weekly => match (week_start(nd) - week_start(ed)) / 7 {
            0 => Elapsed::ThisWeek,
            1 => Elapsed::LastWeek,
            w => Elapsed::WeeksAgo(u32::try_from(w).unwrap_or(u32::MAX)),
        },
        Period::Monthly => {
            let (ey, em, _) = civil_from_days(ed);
            let (ny, nm, _) = civil_from_days(nd);
            match (ny - ey) * 12 + i64::from(nm) - i64::from(em) {
                0 => Elapsed::ThisMonth,
                1 => Elapsed::LastMonth,
                k => Elapsed::MonthsAgo(u32::try_from(k).unwrap_or(u32::MAX)),
            }
        }
        Period::Daily | Period::Intraday(_) => {
            let intraday = matches!(period, Period::Intraday(_));
            if ed == nd {
                if !intraday {
                    return Elapsed::Today;
                }
                let mins = (now_t - event_t) / MS_PER_MIN;
                return match mins {
                    0 => Elapsed::JustNow,
                    1..=59 => Elapsed::MinutesAgo(u32::try_from(mins).unwrap_or(1)),
                    _ => Elapsed::HoursAgo(u32::try_from(mins / 60).unwrap_or(u32::MAX)),
                };
            }
            let n = trading_days_between(cal, ed, nd);
            let at = intraday.then(|| at_of(cal, event_t));
            match n {
                0 | 1 if nd - ed == 1 => Elapsed::Yesterday(at),
                0 | 1 => Elapsed::Weekday(weekday_index(ed), at),
                2..=5 => Elapsed::TradingDaysAgo(n),
                6..=19 if week_start(nd) - week_start(ed) == 7 => Elapsed::LastWeek,
                6..=19 => Elapsed::TradingDaysAgo(n),
                _ => date_of(ed, nd),
            }
        }
    }
}

const WEEKDAY_TR: [&str; 7] = [
    "pazartesi",
    "salı",
    "çarşamba",
    "perşembe",
    "cuma",
    "cumartesi",
    "pazar",
];
const WEEKDAY_EN: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];
const MONTH_TR: [&str; 12] = [
    "Ocak", "Şubat", "Mart", "Nisan", "Mayıs", "Haziran", "Temmuz", "Ağustos", "Eylül", "Ekim",
    "Kasım", "Aralık",
];
const MONTH_EN: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

fn weekday_name(i: u32, lang: Lang) -> &'static str {
    let i = (i as usize).min(6);
    match lang {
        Lang::Tr => WEEKDAY_TR[i],
        Lang::En => WEEKDAY_EN[i],
    }
}

fn month_name(m: u32, lang: Lang) -> &'static str {
    let i = (m.clamp(1, 12) - 1) as usize;
    match lang {
        Lang::Tr => MONTH_TR[i],
        Lang::En => MONTH_EN[i],
    }
}

/// `15:00`.
fn clock(a: At) -> String {
    format!("{:02}:{:02}", a.h, a.m)
}

/// Turkish locative of a clock time, by its spoken last word: `15:00'te` ("on beşte"),
/// `10:00'da` ("onda"), `15:30'da` ("otuzda").
fn clock_loc_tr(a: At) -> String {
    let spoken = if a.m == 0 {
        number_last_word(&a.h.to_string())
    } else {
        number_last_word(&a.m.to_string())
    };
    Word {
        written: clock(a),
        spoken: spoken.to_owned(),
        apostrophe: true,
        possessed: false,
    }
    .loc()
}

/// The phrase in `lang` (digits allowed: counts, clock times, dates).
#[must_use]
pub fn phrase(e: &Elapsed, lang: Lang) -> String {
    match lang {
        Lang::Tr => tr(e),
        Lang::En => en(e),
    }
}

fn tr(e: &Elapsed) -> String {
    match *e {
        Elapsed::JustNow => "az önce".into(),
        Elapsed::MinutesAgo(1) => "bir dakika önce".into(),
        Elapsed::MinutesAgo(n) => format!("{n} dakika önce"),
        Elapsed::HoursAgo(1) => "bir saat önce".into(),
        Elapsed::HoursAgo(n) => format!("{n} saat önce"),
        Elapsed::Today => "bugün".into(),
        Elapsed::Yesterday(None) => "dün".into(),
        Elapsed::Yesterday(Some(a)) => format!("dün {}", clock_loc_tr(a)),
        Elapsed::Weekday(w, None) => format!("{} günü", weekday_name(w, Lang::Tr)),
        Elapsed::Weekday(w, Some(a)) => {
            format!("{} {}", weekday_name(w, Lang::Tr), clock_loc_tr(a))
        }
        Elapsed::TradingDaysAgo(n) => format!("{n} işlem günü önce"),
        Elapsed::LastWeek => "geçen hafta".into(),
        Elapsed::ThisWeek => "bu hafta".into(),
        Elapsed::WeeksAgo(n) => format!("{n} hafta önce"),
        Elapsed::ThisMonth => "bu ay".into(),
        Elapsed::LastMonth => "geçen ay".into(),
        Elapsed::MonthsAgo(n) => format!("{n} ay önce"),
        Elapsed::OnDate { day, month, year } => {
            let m = month_name(month, Lang::Tr);
            match year {
                None => format!("{day} {}", Word::proper(m).loc()),
                Some(y) => format!("{day} {m} {}", Word::number(&y.to_string()).loc()),
            }
        }
    }
}

fn plural(n: u32, one: &str, many: &str) -> String {
    if n == 1 {
        format!("1 {one} ago")
    } else {
        format!("{n} {many} ago")
    }
}

fn en(e: &Elapsed) -> String {
    match *e {
        Elapsed::JustNow => "just now".into(),
        Elapsed::MinutesAgo(1) => "a minute ago".into(),
        Elapsed::MinutesAgo(n) => format!("{n} minutes ago"),
        Elapsed::HoursAgo(1) => "an hour ago".into(),
        Elapsed::HoursAgo(n) => format!("{n} hours ago"),
        Elapsed::Today => "today".into(),
        Elapsed::Yesterday(None) => "yesterday".into(),
        Elapsed::Yesterday(Some(a)) => format!("yesterday at {}", clock(a)),
        Elapsed::Weekday(w, None) => format!("on {}", weekday_name(w, Lang::En)),
        Elapsed::Weekday(w, Some(a)) => {
            format!("on {} at {}", weekday_name(w, Lang::En), clock(a))
        }
        Elapsed::TradingDaysAgo(n) => plural(n, "trading day", "trading days"),
        Elapsed::LastWeek => "last week".into(),
        Elapsed::ThisWeek => "this week".into(),
        Elapsed::WeeksAgo(n) => plural(n, "week", "weeks"),
        Elapsed::ThisMonth => "this month".into(),
        Elapsed::LastMonth => "last month".into(),
        Elapsed::MonthsAgo(n) => plural(n, "month", "months"),
        Elapsed::OnDate { day, month, year } => {
            let m = month_name(month, Lang::En);
            match year {
                None => format!("on {day} {m}"),
                Some(y) => format!("on {day} {m} {y}"),
            }
        }
    }
}

/// The phrase without any digit (a redacted reader, K33): counts become "birkaç / a few",
/// clock times and dates are dropped or bucketed.
#[must_use]
pub fn phrase_vague(e: &Elapsed, lang: Lang) -> String {
    let (tr, en) = match *e {
        Elapsed::JustNow => ("az önce", "just now"),
        Elapsed::MinutesAgo(_) | Elapsed::HoursAgo(_) | Elapsed::Today => ("bugün", "today"),
        Elapsed::Yesterday(_) => ("dün", "yesterday"),
        Elapsed::Weekday(w, _) => {
            return match lang {
                Lang::Tr => format!("{} günü", weekday_name(w, Lang::Tr)),
                Lang::En => format!("on {}", weekday_name(w, Lang::En)),
            }
        }
        Elapsed::TradingDaysAgo(n) if n <= 5 => {
            ("birkaç işlem günü önce", "a few trading days ago")
        }
        Elapsed::TradingDaysAgo(_) => ("birkaç hafta önce", "a few weeks ago"),
        Elapsed::LastWeek => ("geçen hafta", "last week"),
        Elapsed::ThisWeek => ("bu hafta", "this week"),
        Elapsed::WeeksAgo(_) => ("birkaç hafta önce", "a few weeks ago"),
        Elapsed::ThisMonth => ("bu ay", "this month"),
        Elapsed::LastMonth => ("geçen ay", "last month"),
        Elapsed::MonthsAgo(_) | Elapsed::OnDate { .. } => {
            ("bir aydan uzun süre önce", "more than a month ago")
        }
    };
    match lang {
        Lang::Tr => tr.to_owned(),
        Lang::En => en.to_owned(),
    }
}

#[cfg(test)]
mod tests {
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
}
