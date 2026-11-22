//! [`Interval`] and the bar [`Span`] (out of `lib.rs`).

use crate::CalendarError;

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
pub struct Span {
    /// Bar open (the bar's `t`).
    pub open: i64,
    /// Bar close (exclusive).
    pub close: i64,
}
