//! Exchange calendars ("Takvim", K14): bar boundaries follow the exchange's time zone,
//! sessions and holidays — never the viewer's time zone (`display_tz` is display only).
//!
//! - **Data** is `qp_schema::Calendar` (time zone, sessions, weekend, holidays, half days).
//!   [`builtin`] provides sample calendars for BIST (`XIST`) and NYSE (`XNYS`); their
//!   holiday lists contain only a few sample days (full holiday data is out of R1 scope).
//! - **Time zones** are embedded offset-transition tables generated from the IANA tzdb
//!   (`src/tzdata.rs`, see `examples/gen_tzdata.rs`); lookups are integer arithmetic, so
//!   results are identical on every target. Embedded zones: `America/New_York`,
//!   `Europe/Istanbul`, `UTC`. Decision and size measurements: `DECISIONS.md` K16.
//! - **Bars** carry their open time `t` in UTC milliseconds (schema convention). A bar
//!   span is `[open, close)`.
//!
//! Intervals: `Nm`/`Nh` (intraday, aligned to each session's open; the last bar of a
//! session is cut at the session close), `1d` (one bar per trading day: first session open
//! to last session close), `1w` (trading days of an ISO week, Monday to Sunday local),
//! `1M` (trading days of a local calendar month).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use qp_schema::{Bar, Calendar, HalfDay, Session, Weekday};

mod tzdata;
pub mod zone;

pub use zone::{LocalTime, Zone};

const MS_PER_MIN: i64 = 60_000;
const MS_PER_DAY: i64 = 86_400_000;

/// Any calendar error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarError(String);

impl CalendarError {
    fn new(msg: impl Into<String>) -> Self {
        Self(msg.into())
    }
}

impl fmt::Display for CalendarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for CalendarError {}

/// A bar interval.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interval {
    /// Intraday bars of this many minutes.
    Minutes(u32),
    /// One bar per trading day.
    Day,
    /// One bar per ISO week (trading days only).
    Week,
    /// One bar per calendar month (trading days only).
    Month,
}

impl Interval {
    /// Parses `1m`, `5m`, `15m`, `1h`, `4h`, `1d`, `1w`, `1M`.
    ///
    /// # Errors
    /// Unknown unit or a zero/oversized count.
    pub fn parse(s: &str) -> Result<Self, CalendarError> {
        let err = || CalendarError::new(format!("invalid interval `{s}`"));
        let (num, unit) = s.split_at(s.find(|c: char| !c.is_ascii_digit()).ok_or_else(err)?);
        let n: u32 = num.parse().map_err(|_| err())?;
        if n == 0 {
            return Err(err());
        }
        match (unit, n) {
            ("m", n) if n <= 1440 => Ok(Self::Minutes(n)),
            ("h", n) if n <= 24 => Ok(Self::Minutes(n * 60)),
            ("d", 1) => Ok(Self::Day),
            ("w", 1) => Ok(Self::Week),
            ("M", 1) => Ok(Self::Month),
            _ => Err(err()),
        }
    }
}

/// A bar's time span, UTC milliseconds, `[open, close)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BarSpan {
    /// Bar open (the bar's `t`).
    pub open: i64,
    /// Bar close (exclusive).
    pub close: i64,
}

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

fn weekday_of(w: Weekday) -> u32 {
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

fn parse_hm(s: &str) -> Result<i64, CalendarError> {
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

/// A calendar ready for queries.
#[derive(Debug, Clone)]
pub struct ExchangeCalendar {
    id: String,
    zone: Zone,
    /// Regular sessions, local minutes since midnight, `[open, close)`.
    sessions: Vec<(i64, i64)>,
    weekend: [bool; 7],
    holidays: BTreeSet<i64>,
    half_days: BTreeMap<i64, i64>,
}

impl ExchangeCalendar {
    /// Validates calendar data and resolves its time zone.
    ///
    /// # Errors
    /// Unknown (not embedded) time zone, malformed times or dates, empty or overlapping
    /// sessions.
    pub fn new(cal: &Calendar) -> Result<Self, CalendarError> {
        let zone = Zone::get(&cal.tz).ok_or_else(|| {
            CalendarError::new(format!(
                "time zone `{}` is not embedded (available: {})",
                cal.tz,
                Zone::names().join(", ")
            ))
        })?;
        let mut sessions = Vec::with_capacity(cal.sessions.len());
        for Session { open, close } in &cal.sessions {
            let (o, c) = (parse_hm(open)?, parse_hm(close)?);
            if o >= c || sessions.last().is_some_and(|&(_, pc)| o < pc) {
                return Err(CalendarError::new(format!(
                    "{}: sessions must be non-empty, ascending and non-overlapping",
                    cal.id
                )));
            }
            sessions.push((o, c));
        }
        if sessions.is_empty() {
            return Err(CalendarError::new(format!("{}: no sessions", cal.id)));
        }
        let mut weekend = [false; 7];
        for w in &cal.weekend {
            weekend[weekday_of(*w) as usize] = true;
        }
        let holidays = cal
            .holidays
            .iter()
            .map(|d| parse_date(d))
            .collect::<Result<_, _>>()?;
        let half_days = cal
            .half_days
            .iter()
            .map(|HalfDay { date, close }| Ok((parse_date(date)?, parse_hm(close)?)))
            .collect::<Result<_, CalendarError>>()?;
        Ok(Self {
            id: cal.id.clone(),
            zone,
            sessions,
            weekend,
            holidays,
            half_days,
        })
    }

    /// Calendar id.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The exchange time zone.
    #[must_use]
    pub fn zone(&self) -> Zone {
        self.zone
    }

    /// Local day number (exchange time zone) of UTC instant `t_ms`.
    #[must_use]
    pub fn local_day(&self, t_ms: i64) -> i64 {
        let off = i64::from(self.zone.offset_at(t_ms.div_euclid(1000))) * 1000;
        (t_ms + off).div_euclid(MS_PER_DAY)
    }

    /// Whether local day `day` is a trading day.
    #[must_use]
    pub fn is_trading_day(&self, day: i64) -> bool {
        !self.weekend[weekday_index(day) as usize] && !self.holidays.contains(&day)
    }

    /// UTC instant of local `day` at `minute` past local midnight. Gap times move forward
    /// to the first valid instant after the gap; ambiguous times take the earlier instant.
    fn at_local(&self, day: i64, minute: i64) -> i64 {
        let local_s = day * 86_400 + minute * 60;
        let utc_s = match self.zone.local_to_utc(local_s) {
            LocalTime::Unique(t) | LocalTime::Ambiguous(t, _) => t,
            LocalTime::Gap => {
                // Wall times inside a gap: use the offset in force just before the gap,
                // which lands after the transition.
                local_s - i64::from(self.zone.offset_at(local_s - 86_400))
            }
        };
        utc_s * 1000
    }

    /// The sessions of local `day` as UTC spans (half days cut at their early close);
    /// empty for non-trading days.
    #[must_use]
    pub fn sessions_on(&self, day: i64) -> Vec<BarSpan> {
        if !self.is_trading_day(day) {
            return Vec::new();
        }
        let cut = self.half_days.get(&day).copied();
        self.sessions
            .iter()
            .filter_map(|&(o, c)| {
                let c = cut.map_or(c, |cut| c.min(cut));
                (o < c).then(|| BarSpan {
                    open: self.at_local(day, o),
                    close: self.at_local(day, c),
                })
            })
            .collect()
    }

    /// The full trading-day span `[first open, last close)` of local `day`, if trading.
    #[must_use]
    pub fn day_span(&self, day: i64) -> Option<BarSpan> {
        let s = self.sessions_on(day);
        Some(BarSpan {
            open: s.first()?.open,
            close: s.last()?.close,
        })
    }

    /// Whether `t_ms` falls inside a session.
    #[must_use]
    pub fn in_session(&self, t_ms: i64) -> bool {
        self.sessions_on(self.local_day(t_ms))
            .iter()
            .any(|s| s.open <= t_ms && t_ms < s.close)
    }

    /// The bar of `interval` containing UTC instant `t_ms`, or `None` if `t_ms` is outside
    /// every session.
    #[must_use]
    pub fn bar_at(&self, interval: Interval, t_ms: i64) -> Option<BarSpan> {
        let day = self.local_day(t_ms);
        let sessions = self.sessions_on(day);
        let session = *sessions.iter().find(|s| s.open <= t_ms && t_ms < s.close)?;
        match interval {
            Interval::Minutes(n) => {
                let len = i64::from(n) * MS_PER_MIN;
                let open = session.open + (t_ms - session.open) / len * len;
                Some(BarSpan {
                    open,
                    close: (open + len).min(session.close),
                })
            }
            Interval::Day => self.day_span(day),
            Interval::Week => {
                let monday = day - i64::from(weekday_index(day));
                self.span_of_days(monday, monday + 7)
            }
            Interval::Month => {
                let (y, m, _) = civil_from_days(day);
                let first = days_from_civil(y, m, 1);
                let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
                self.span_of_days(first, days_from_civil(ny, nm, 1))
            }
        }
    }

    /// `[first open, last close)` over the trading days in `[from_day, to_day)`.
    fn span_of_days(&self, from_day: i64, to_day: i64) -> Option<BarSpan> {
        let first = (from_day..to_day).find_map(|d| self.day_span(d))?;
        let last = (from_day..to_day).rev().find_map(|d| self.day_span(d))?;
        Some(BarSpan {
            open: first.open,
            close: last.close,
        })
    }

    /// All bars of `interval` whose open lies in `[from_ms, to_ms)`, ascending.
    #[must_use]
    pub fn bars_between(&self, interval: Interval, from_ms: i64, to_ms: i64) -> Vec<BarSpan> {
        let mut out: Vec<BarSpan> = Vec::new();
        if from_ms >= to_ms {
            return out;
        }
        // Local days that can hold a session overlapping the range (±1 day margin).
        let (d0, d1) = (self.local_day(from_ms) - 1, self.local_day(to_ms) + 1);
        for day in d0..=d1 {
            for s in self.sessions_on(day) {
                let spans: Vec<BarSpan> = match interval {
                    Interval::Minutes(n) => {
                        let len = i64::from(n) * MS_PER_MIN;
                        let mut v = Vec::new();
                        let mut o = s.open;
                        while o < s.close {
                            v.push(BarSpan {
                                open: o,
                                close: (o + len).min(s.close),
                            });
                            o += len;
                        }
                        v
                    }
                    _ => self.bar_at(interval, s.open).into_iter().collect(),
                };
                for b in spans {
                    if b.open >= from_ms && b.open < to_ms && out.last() != Some(&b) {
                        out.push(b);
                    }
                }
            }
        }
        out
    }

    /// Resamples bars (e.g. 1-minute bars) into calendar-aligned bars of `interval`.
    /// Input bars are assigned by their open time `t`; bars outside every session are
    /// dropped and counted. Output `t` is the calendar bar's open; `o` = first open,
    /// `h` = max, `l` = min, `c` = last close, `v` = sum (in input order).
    ///
    /// # Errors
    /// Input not strictly ascending by `t`.
    pub fn resample(
        &self,
        interval: Interval,
        bars: &[Bar],
    ) -> Result<(Vec<Bar>, usize), CalendarError> {
        let mut out: Vec<Bar> = Vec::new();
        let mut dropped = 0usize;
        let mut prev_t: Option<i64> = None;
        for b in bars {
            if prev_t.is_some_and(|p| b.t <= p) {
                return Err(CalendarError::new(format!(
                    "input bars not ascending at t={}",
                    b.t
                )));
            }
            prev_t = Some(b.t);
            let Some(span) = self.bar_at(interval, b.t) else {
                dropped += 1;
                continue;
            };
            match out.last_mut() {
                Some(last) if last.t == span.open => {
                    last.h = last.h.max(b.h);
                    last.l = last.l.min(b.l);
                    last.c = b.c;
                    last.v += b.v;
                }
                _ => out.push(Bar {
                    t: span.open,
                    ..b.clone()
                }),
            }
        }
        Ok((out, dropped))
    }
}

/// Formats UTC instant `t_ms` in display time zone `tz` as ISO 8601 with offset, e.g.
/// `2026-03-09T09:30:00-04:00`. Display only: nothing in alignment or computation uses
/// it (K14). Only embedded zones are available; shells may format other zones themselves.
///
/// # Errors
/// Zone not embedded.
pub fn format_display(t_ms: i64, tz: &str) -> Result<String, CalendarError> {
    let zone = Zone::get(tz)
        .ok_or_else(|| CalendarError::new(format!("time zone `{tz}` is not embedded")))?;
    let off = zone.offset_at(t_ms.div_euclid(1000));
    let local_s = t_ms.div_euclid(1000) + i64::from(off);
    let (y, m, d) = civil_from_days(local_s.div_euclid(86_400));
    let sod = local_s.rem_euclid(86_400);
    let sign = if off < 0 { '-' } else { '+' };
    let a = off.unsigned_abs();
    Ok(format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}{sign}{:02}:{:02}",
        sod / 3600,
        sod % 3600 / 60,
        sod % 60,
        a / 3600,
        a % 3600 / 60
    ))
}

/// Sample calendars: `XIST` (Borsa Istanbul equities) and `XNYS` (NYSE). Holidays and
/// half days are a few 2026 sample days only (R1 scope: mechanism, not full data).
#[must_use]
pub fn builtin(id: &str) -> Option<Calendar> {
    let s = |o: &str, c: &str| Session {
        open: o.to_owned(),
        close: c.to_owned(),
    };
    let strs = |v: &[&str]| v.iter().map(|d| (*d).to_owned()).collect::<Vec<_>>();
    let half = |d: &str, c: &str| HalfDay {
        date: d.to_owned(),
        close: c.to_owned(),
    };
    match id {
        // Continuous trading 10:00-18:00 Istanbul time (opening/closing auctions not
        // modelled as separate sessions).
        "XIST" => Some(Calendar {
            id: "XIST".to_owned(),
            tz: "Europe/Istanbul".to_owned(),
            sessions: vec![s("10:00", "18:00")],
            weekend: vec![Weekday::Sat, Weekday::Sun],
            // 2026 samples: New Year, Ramadan Feast (20 Mar), 23 Apr, 1 May, 19 May.
            holidays: strs(&[
                "2026-01-01",
                "2026-03-20",
                "2026-04-23",
                "2026-05-01",
                "2026-05-19",
            ]),
            // Eve of the Ramadan Feast: half day, closes 12:30.
            half_days: vec![half("2026-03-19", "12:30")],
        }),
        "XNYS" => Some(Calendar {
            id: "XNYS".to_owned(),
            tz: "America/New_York".to_owned(),
            sessions: vec![s("09:30", "16:00")],
            weekend: vec![Weekday::Sat, Weekday::Sun],
            // 2026 samples: New Year, Thanksgiving, Christmas.
            holidays: strs(&["2026-01-01", "2026-11-26", "2026-12-25"]),
            // Day after Thanksgiving and Christmas Eve close at 13:00.
            half_days: vec![half("2026-11-27", "13:00"), half("2026-12-24", "13:00")],
        }),
        _ => None,
    }
}
