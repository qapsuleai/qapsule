//! (RP4/T7) Row time stamp ("satırda saat"): the wall time of an event on a capsule or
//! story row, next to the elapsed phrase ([`super::elapsed`]).
//!
//! | Bars | Event on now's day | Earlier day |
//! |------|--------------------|-------------|
//! | intraday (`Nm`, `Nh`) | `14:35` | `12 Kas` / `Nov 12` |
//! | daily, weekly, monthly | `12 Kas` / `Nov 12` | `12 Kas` / `Nov 12` |
//!
//! Another year than now's adds it: `12 Kas 2025` / `Nov 12, 2025`.
//!
//! Deterministic: the event's and now's bar times (UTC ms), the bar period and a display
//! zone; no clock. "Now" is the caller's: the authority's data time (the series' last
//! bar), never the system clock. The display zone defaults to the exchange's (the
//! calendar's zone, K14); a reader's own zone may be given instead. A bar is named by its
//! **open** time (`Bar::t`), as the chart and the elapsed phrase name it.
//!
//! A redacted reader (K33) gets no stamp at all: the caller leaves it out (the stamp is
//! digits).

use qp_calendar::{civil_from_days, Zone};

use super::elapsed::Period;
use crate::Lang;

const MS_PER_MIN: i64 = 60_000;
const MS_PER_DAY: i64 = 86_400_000;

const MONTH_TR: [&str; 12] = [
    "Oca", "Şub", "Mar", "Nis", "May", "Haz", "Tem", "Ağu", "Eyl", "Eki", "Kas", "Ara",
];
const MONTH_EN: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// Local milliseconds since 1970-01-01T00:00 in `zone` of UTC instant `t` (ms).
fn local_ms(zone: Zone, t: i64) -> i64 {
    t + i64::from(zone.offset_at(t.div_euclid(1000))) * 1000
}

/// The stamp of an event at bar time `event_t` as of bar time `now_t` (UTC ms) for bars of
/// `period`, in display zone `zone` and language `lang` (see the module). An event after
/// now counts as now.
#[must_use]
pub fn stamp(zone: Zone, period: Period, event_t: i64, now_t: i64, lang: Lang) -> String {
    let event_t = event_t.min(now_t);
    let (el, nl) = (local_ms(zone, event_t), local_ms(zone, now_t));
    let (ed, nd) = (el.div_euclid(MS_PER_DAY), nl.div_euclid(MS_PER_DAY));
    if matches!(period, Period::Intraday(_)) && ed == nd {
        let mins = el.rem_euclid(MS_PER_DAY) / MS_PER_MIN;
        return format!("{:02}:{:02}", mins / 60, mins % 60);
    }
    let (y, m, d) = civil_from_days(ed);
    let (ny, _, _) = civil_from_days(nd);
    let i = (m.clamp(1, 12) - 1) as usize;
    match (lang, y == ny) {
        (Lang::Tr, true) => format!("{d} {}", MONTH_TR[i]),
        (Lang::Tr, false) => format!("{d} {} {y}", MONTH_TR[i]),
        (Lang::En, true) => format!("{} {d}", MONTH_EN[i]),
        (Lang::En, false) => format!("{} {d}, {y}", MONTH_EN[i]),
    }
}

/// A row text with its stamp before it: `"<stamp> · <text>"`.
#[must_use]
pub fn with_stamp(stamp: &str, text: &str) -> String {
    format!("{stamp} · {text}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use qp_calendar::days_from_civil;

    fn ist() -> Zone {
        Zone::get("Europe/Istanbul").expect("zone")
    }

    /// UTC ms of local Istanbul (UTC+3) `y-m-d hh:mm`.
    fn at(y: i64, m: u32, d: u32, hh: i64, mm: i64) -> i64 {
        (days_from_civil(y, m, d) * 1440 + hh * 60 + mm - 180) * MS_PER_MIN
    }

    const M5: Period = Period::Intraday(5);

    #[test]
    fn intraday_today_is_the_exchange_clock() {
        // Friday 2026-11-20.
        let now = at(2026, 11, 20, 17, 55);
        assert_eq!(
            stamp(ist(), M5, at(2026, 11, 20, 14, 35), now, Lang::Tr),
            "14:35"
        );
        assert_eq!(
            stamp(ist(), M5, at(2026, 11, 20, 9, 55), now, Lang::En),
            "09:55"
        );
        // Same bar as now.
        assert_eq!(stamp(ist(), M5, now, now, Lang::Tr), "17:55");
        // An hourly series too.
        let h1 = Period::Intraday(60);
        assert_eq!(
            stamp(ist(), h1, at(2026, 11, 20, 10, 0), now, Lang::Tr),
            "10:00"
        );
    }

    #[test]
    fn intraday_yesterday_and_older_are_dates() {
        let now = at(2026, 11, 20, 10, 5);
        assert_eq!(
            stamp(ist(), M5, at(2026, 11, 19, 17, 50), now, Lang::Tr),
            "19 Kas"
        );
        assert_eq!(
            stamp(ist(), M5, at(2026, 11, 19, 17, 50), now, Lang::En),
            "Nov 19"
        );
        assert_eq!(
            stamp(ist(), M5, at(2026, 11, 12, 11, 0), now, Lang::Tr),
            "12 Kas"
        );
        assert_eq!(
            stamp(ist(), M5, at(2026, 8, 3, 11, 0), now, Lang::Tr),
            "3 Ağu"
        );
        assert_eq!(
            stamp(ist(), M5, at(2026, 8, 3, 11, 0), now, Lang::En),
            "Aug 3"
        );
    }

    #[test]
    fn across_a_weekend() {
        // Monday 2026-11-23 morning; the event on Friday's close.
        let now = at(2026, 11, 23, 10, 0);
        let fri = at(2026, 11, 20, 17, 55);
        assert_eq!(stamp(ist(), M5, fri, now, Lang::Tr), "20 Kas");
        assert_eq!(stamp(ist(), M5, fri, now, Lang::En), "Nov 20");
        // Daily: a date whatever the day.
        assert_eq!(stamp(ist(), Period::Daily, fri, now, Lang::Tr), "20 Kas");
    }

    #[test]
    fn half_day() {
        // A half trading day (the session ends 12:30): its last bar is a time that day.
        let now = at(2026, 5, 26, 12, 25);
        assert_eq!(
            stamp(ist(), M5, at(2026, 5, 26, 12, 25), now, Lang::Tr),
            "12:25"
        );
        // The next day, it is a date.
        let next = at(2026, 5, 27, 10, 0);
        assert_eq!(stamp(ist(), M5, now, next, Lang::Tr), "26 May");
        assert_eq!(stamp(ist(), M5, now, next, Lang::En), "May 26");
    }

    #[test]
    fn daily_is_always_a_date() {
        // A daily bar's open time is the session open; today's bar is a date too.
        let now = at(2026, 11, 20, 10, 0);
        assert_eq!(stamp(ist(), Period::Daily, now, now, Lang::Tr), "20 Kas");
        assert_eq!(stamp(ist(), Period::Daily, now, now, Lang::En), "Nov 20");
        assert_eq!(
            stamp(
                ist(),
                Period::Weekly,
                at(2026, 11, 16, 10, 0),
                now,
                Lang::Tr
            ),
            "16 Kas"
        );
        assert_eq!(
            stamp(
                ist(),
                Period::Monthly,
                at(2026, 11, 2, 10, 0),
                now,
                Lang::En
            ),
            "Nov 2"
        );
    }

    #[test]
    fn another_year_names_it() {
        let now = at(2027, 1, 4, 10, 0);
        let ev = at(2026, 12, 31, 12, 0);
        assert_eq!(stamp(ist(), M5, ev, now, Lang::Tr), "31 Ara 2026");
        assert_eq!(stamp(ist(), M5, ev, now, Lang::En), "Dec 31, 2026");
    }

    #[test]
    fn the_display_zone_is_the_callers() {
        // 2026-11-20 14:35 Istanbul is 11:35 UTC; a reader in UTC reads 11:35.
        let now = at(2026, 11, 20, 17, 55);
        let ev = at(2026, 11, 20, 14, 35);
        let utc = Zone::get("UTC").expect("utc");
        assert_eq!(stamp(utc, M5, ev, now, Lang::Tr), "11:35");
        // Near midnight the day follows the display zone: 01:30 Istanbul is the previous
        // day in UTC, still "today" as of a now that is the same UTC day.
        let late = at(2026, 11, 21, 1, 30);
        assert_eq!(stamp(ist(), M5, late, late, Lang::Tr), "01:30");
        assert_eq!(stamp(utc, M5, late, late, Lang::Tr), "22:30");
    }

    #[test]
    fn an_event_after_now_counts_as_now() {
        let now = at(2026, 11, 20, 14, 0);
        assert_eq!(stamp(ist(), M5, now + 3_600_000, now, Lang::Tr), "14:00");
    }

    #[test]
    fn joined() {
        assert_eq!(with_stamp("14:35", "X kırdı"), "14:35 · X kırdı");
    }
}
