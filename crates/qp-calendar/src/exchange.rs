//! [`ExchangeCalendar`]: sessions, trading days, bar alignment and resampling, plus
//! [`format_display`] (out of `lib.rs`).

use std::collections::{BTreeMap, BTreeSet};

use qp_schema::{Bar, Calendar, HalfDay, Session};

use crate::civil::{parse_hm, weekday_of, MS_PER_DAY, MS_PER_MIN};
use crate::{
    civil_from_days, days_from_civil, parse_date, weekday_index, CalendarError, Interval,
    LocalTime, Span, Zone,
};

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
    pub fn sessions_on(&self, day: i64) -> Vec<Span> {
        if !self.is_trading_day(day) {
            return Vec::new();
        }
        let cut = self.half_days.get(&day).copied();
        self.sessions
            .iter()
            .filter_map(|&(o, c)| {
                let c = cut.map_or(c, |cut| c.min(cut));
                (o < c).then(|| Span {
                    open: self.at_local(day, o),
                    close: self.at_local(day, c),
                })
            })
            .collect()
    }

    /// The full trading-day span `[first open, last close)` of local `day`, if trading.
    #[must_use]
    pub fn day_span(&self, day: i64) -> Option<Span> {
        let s = self.sessions_on(day);
        Some(Span {
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
    pub fn bar_at(&self, interval: Interval, t_ms: i64) -> Option<Span> {
        let day = self.local_day(t_ms);
        let sessions = self.sessions_on(day);
        let session = *sessions.iter().find(|s| s.open <= t_ms && t_ms < s.close)?;
        match interval {
            Interval::Minutes(n) => {
                let len = i64::from(n) * MS_PER_MIN;
                let open = session.open + (t_ms - session.open) / len * len;
                Some(Span {
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
    fn span_of_days(&self, from_day: i64, to_day: i64) -> Option<Span> {
        let first = (from_day..to_day).find_map(|d| self.day_span(d))?;
        let last = (from_day..to_day).rev().find_map(|d| self.day_span(d))?;
        Some(Span {
            open: first.open,
            close: last.close,
        })
    }

    /// All bars of `interval` whose open lies in `[from_ms, to_ms)`, ascending.
    #[must_use]
    pub fn bars_between(&self, interval: Interval, from_ms: i64, to_ms: i64) -> Vec<Span> {
        let mut out: Vec<Span> = Vec::new();
        if from_ms >= to_ms {
            return out;
        }
        // Local days that can hold a session overlapping the range (±1 day margin).
        let (d0, d1) = (self.local_day(from_ms) - 1, self.local_day(to_ms) + 1);
        for day in d0..=d1 {
            for s in self.sessions_on(day) {
                let spans: Vec<Span> = match interval {
                    Interval::Minutes(n) => {
                        let len = i64::from(n) * MS_PER_MIN;
                        let mut v = Vec::new();
                        let mut o = s.open;
                        while o < s.close {
                            v.push(Span {
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
