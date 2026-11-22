//! Civil-date arithmetic and date / time parsing (out of `lib.rs`).

use qp_schema::Weekday;

use crate::CalendarError;

pub(crate) const MS_PER_MIN: i64 = 60_000;
pub(crate) const MS_PER_DAY: i64 = 86_400_000;

/// Days since 1970-01-01 of a civil date (proleptic Gregorian).
#[must_use]
pub fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let (m, d) = (i64::from(m), i64::from(d));
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Civil date `(year, month, day)` of a day number.
#[must_use]
pub fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    // d in 1..=31 and m in 1..=12 by construction.
    (
        y,
        u32::try_from(m).unwrap_or(1),
        u32::try_from(d).unwrap_or(1),
    )
}

/// ISO weekday index of a day number: Monday = 0 … Sunday = 6.
#[must_use]
pub fn weekday_index(day: i64) -> u32 {
    // 1970-01-01 was a Thursday (index 3).
    u32::try_from((day + 3).rem_euclid(7)).unwrap_or(0)
}

pub(crate) fn weekday_of(w: Weekday) -> u32 {
    match w {
        Weekday::Mon => 0,
        Weekday::Tue => 1,
        Weekday::Wed => 2,
        Weekday::Thu => 3,
        Weekday::Fri => 4,
        Weekday::Sat => 5,
        Weekday::Sun => 6,
    }
}

pub(crate) fn parse_hm(s: &str) -> Result<i64, CalendarError> {
    let err = || CalendarError::new(format!("invalid time `{s}` (expected HH:MM)"));
    let (h, m) = s.split_once(':').ok_or_else(err)?;
    let (h, m): (i64, i64) = (h.parse().map_err(|_| err())?, m.parse().map_err(|_| err())?);
    if !(0..24).contains(&h) || !(0..60).contains(&m) || s.len() != 5 {
        return Err(err());
    }
    Ok(h * 60 + m)
}

/// Parses `YYYY-MM-DD` into a day number.
///
/// # Errors
/// Malformed or impossible date.
pub fn parse_date(s: &str) -> Result<i64, CalendarError> {
    let err = || CalendarError::new(format!("invalid date `{s}` (expected YYYY-MM-DD)"));
    let mut it = s.split('-');
    let (Some(y), Some(m), Some(d), None) = (it.next(), it.next(), it.next(), it.next()) else {
        return Err(err());
    };
    let (y, m, d): (i64, u32, u32) = (
        y.parse().map_err(|_| err())?,
        m.parse().map_err(|_| err())?,
        d.parse().map_err(|_| err())?,
    );
    let day = days_from_civil(y, m, d);
    if civil_from_days(day) != (y, m, d) {
        return Err(err());
    }
    Ok(day)
}
