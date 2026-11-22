//! Exchange calendars ("Takvim", K14): bar boundaries follow the exchange's time zone,
//! sessions and holidays — never the viewer's time zone (`display_tz` is display only).
//!
//! - **Data** is `qp_schema::Calendar` (time zone, sessions, weekend, holidays, half days).
//!   [`builtin`] provides calendars for BIST (`XIST`: full 2018–2027 closures and half
//!   days, R4) and NYSE (`XNYS`: a few 2026 sample days only, R1 scope).
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

mod builtin;
mod civil;
mod error;
mod exchange;
mod interval;
mod tzdata;
pub mod zone;

pub use builtin::{builtin, XIST_HALF_DAYS, XIST_HALF_DAY_CLOSE, XIST_HOLIDAYS};
pub use civil::{civil_from_days, days_from_civil, parse_date, weekday_index};
pub use error::CalendarError;
pub use exchange::{format_display, ExchangeCalendar};
pub use interval::{Interval, Span};
pub use zone::{LocalTime, Zone};
