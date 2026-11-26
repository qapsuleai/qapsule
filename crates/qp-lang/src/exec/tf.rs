//! Higher timeframes (PR §3.2): the alignment source (exchange session or UTC), bar
//! spans and closes, and the building of higher bars from closed base bars. A higher bar
//! exists only once closed; one whose open is before the first base bar is incomplete
//! and never forms; base bars in no session enter none (`dropped`).

use qp_calendar::{
    civil_from_days, days_from_civil, weekday_index, ExchangeCalendar, Interval, Span,
};
use qp_schema::Bar;

const MS_PER_MIN: i64 = 60_000;
const MS_PER_DAY: i64 = 86_400_000;

/// Where bar boundaries come from (PR §7 "hiza kipi").
#[derive(Debug, Clone)]
pub enum Align {
    /// UTC: `Nm`/`Nh` at multiples of the length, `1d` the UTC day, `1w` the ISO week
    /// (Monday 00:00 UTC), `1M` the UTC month. For instruments without a calendar, 24/7
    /// instruments, and RX4 conformance.
    Utc,
    /// The instrument's exchange calendar (`qp_calendar::builtin`, e.g. `XIST`).
    Session(Box<ExchangeCalendar>),
}

impl Align {
    /// The session alignment of a built-in calendar id, or UTC when there is none.
    #[must_use]
    pub fn for_calendar(id: Option<&str>) -> Align {
        id.and_then(qp_calendar::builtin)
            .and_then(|c| ExchangeCalendar::new(&c).ok())
            .map_or(Align::Utc, |c| Align::Session(Box::new(c)))
    }

    /// The bar of `iv` holding instant `t`; `None` outside every session.
    #[must_use]
    pub fn span(&self, iv: Interval, t: i64) -> Option<Span> {
        match self {
            Align::Utc => Some(utc_span(iv, t)),
            Align::Session(cal) => cal.bar_at(iv, t),
        }
    }

    /// When a base bar opened at `t` closes (its `known_t`): the calendar close, or the
    /// UTC bar's end (`t` + the interval; the month's end for `1M`) outside a session.
    #[must_use]
    pub fn close_of(&self, base: Interval, t: i64) -> i64 {
        match self.span(base, t) {
            Some(s) if matches!(self, Align::Session(_)) => s.close,
            _ => match base {
                Interval::Minutes(n) => t + i64::from(n) * MS_PER_MIN,
                Interval::Day => t + MS_PER_DAY,
                Interval::Week => t + 7 * MS_PER_DAY,
                Interval::Month => utc_span(base, t).close,
            },
        }
    }
}

/// The UTC bar of `iv` holding `t`.
fn utc_span(iv: Interval, t: i64) -> Span {
    let day = t.div_euclid(MS_PER_DAY);
    let (open, close) = match iv {
        Interval::Minutes(n) => {
            let len = i64::from(n) * MS_PER_MIN;
            let open = t.div_euclid(len) * len;
            (open, open + len)
        }
        Interval::Day => (day * MS_PER_DAY, (day + 1) * MS_PER_DAY),
        Interval::Week => {
            let monday = day - i64::from(weekday_index(day));
            (monday * MS_PER_DAY, (monday + 7) * MS_PER_DAY)
        }
        Interval::Month => {
            let (y, m, _) = civil_from_days(day);
            let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
            (
                days_from_civil(y, m, 1) * MS_PER_DAY,
                days_from_civil(ny, nm, 1) * MS_PER_DAY,
            )
        }
    };
    Span { open, close }
}

/// A closed higher bar: the aggregated bar (`t` = its open), its index on its clock and
/// its first base bar.
#[derive(Debug, Clone)]
pub(super) struct TfBar {
    pub(super) bar: Bar,
    pub(super) k: u64,
    pub(super) first: u64,
}

/// A higher bar being built.
#[derive(Debug, Clone)]
struct Building {
    span: Span,
    bar: Bar,
    first: u64,
    /// Opened before the first base bar: never forms.
    incomplete: bool,
}

/// The builder of one higher timeframe.
#[derive(Debug, Clone)]
pub(super) struct TfBuilder {
    iv: Interval,
    cur: Option<Building>,
    /// Closed (formed) higher bars so far.
    k: u64,
    /// Base bars in no session.
    pub(super) dropped: u64,
}

impl TfBuilder {
    pub(super) fn new(iv: Interval) -> TfBuilder {
        TfBuilder {
            iv,
            cur: None,
            k: 0,
            dropped: 0,
        }
    }

    /// Feeds closed base bar `i` (closing at `close_i`); returns the higher bars it closes,
    /// in order (at most two: the previous one, and a new one closing at once).
    pub(super) fn feed(
        &mut self,
        align: &Align,
        bar: &Bar,
        i: u64,
        close_i: i64,
        anchor_t: i64,
    ) -> [Option<TfBar>; 2] {
        let mut out = [None, None];
        let Some(span) = align.span(self.iv, bar.t) else {
            self.dropped += 1;
            // A base bar in no session still closes a higher bar that has ended.
            if self.cur.as_ref().is_some_and(|c| c.span.close <= close_i) {
                out[0] = self.close();
            }
            return out;
        };
        match &mut self.cur {
            Some(c) if c.span.open == span.open => {
                c.bar.h = c.bar.h.max(bar.h);
                c.bar.l = c.bar.l.min(bar.l);
                c.bar.c = bar.c;
                c.bar.v += bar.v;
            }
            _ => {
                out[0] = self.close();
                self.cur = Some(Building {
                    span,
                    bar: Bar {
                        t: span.open,
                        ..bar.clone()
                    },
                    first: i,
                    incomplete: span.open < anchor_t,
                });
            }
        }
        if self.cur.as_ref().is_some_and(|c| c.span.close <= close_i) {
            let closed = self.close();
            if out[0].is_none() {
                out[0] = closed;
            } else {
                out[1] = closed;
            }
        }
        out
    }

    /// Closes the bar being built; `None` if there is none or it is incomplete.
    fn close(&mut self) -> Option<TfBar> {
        let c = self.cur.take()?;
        if c.incomplete {
            return None;
        }
        let k = self.k;
        self.k += 1;
        Some(TfBar {
            bar: c.bar,
            k,
            first: c.first,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utc_spans() {
        // 2024-01-03 (Wednesday) 13:37 UTC.
        let t = 1_704_289_020_000;
        let d = utc_span(Interval::Day, t);
        assert_eq!((d.open, d.close), (1_704_240_000_000, 1_704_326_400_000));
        let h4 = utc_span(Interval::Minutes(240), t);
        assert_eq!(h4.open, 1_704_283_200_000);
        let w = utc_span(Interval::Week, t);
        assert_eq!(w.open, 1_704_067_200_000); // Monday 2024-01-01
        let m = utc_span(Interval::Month, t);
        assert_eq!((m.open, m.close), (1_704_067_200_000, 1_706_745_600_000));
    }
}
