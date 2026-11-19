//! Exchange clock and BIST equity market openness (`mx/timestamp`, `TimeMessage`).
//!
//! `TimeMessage` is **proto3**: its scalars carry no presence. `isBistPPOpen =
//! false` and "field not sent" decode identically, and so do `timestamp = 0`
//! and "absent". [`TimeTick::fields`] lists what was actually on the wire
//! (a proto3 encoder omits default values, so a missing `isBistPPOpen` most
//! likely means `false`, but the wire cannot prove it).
//!
//! Openness and holiday are meaningful only when `source == "MX"`
//! (`docs/matriks/MESSAGES.md` §2.7, §4.2); other sources (`BISTPP`, `BISTEX`,
//! `BISTVIOP`) only move the last-seen exchange time. [`MarketClock`] follows
//! that rule.

use prost::Message as _;

use crate::fields::{FieldSet, TIME_MESSAGE};
use crate::messages::TimeMessage;
use crate::wire::field_numbers;
use crate::DecodeError;

/// The source value whose openness/holiday flags are authoritative.
pub const MX_SOURCE: &str = "MX";

/// A decoded `TimeMessage`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeTick {
    /// Exchange time, Unix epoch microseconds (UTC).
    pub micros: i64,
    /// Source (`MX`, `BISTPP`, `BISTEX`, `BISTVIOP`).
    pub source: String,
    /// `isBistPPOpen`: BIST equity market open (meaningful only for `MX`).
    pub is_bist_pp_open: bool,
    /// `isBistPPHoliday`: BIST equity market holiday today (meaningful only for `MX`).
    pub is_bist_pp_holiday: bool,
    /// Fields that were on the wire (proto3: absent ≈ default, see module docs).
    pub fields: FieldSet,
}

impl TimeTick {
    /// Exchange time in Unix epoch milliseconds (UTC), floored.
    pub fn millis(&self) -> i64 {
        self.micros.div_euclid(1000)
    }

    /// Whether this tick's openness/holiday flags are authoritative (`source == "MX"`).
    pub fn is_mx(&self) -> bool {
        self.source == MX_SOURCE
    }
}

/// Decodes a `TimeMessage` payload.
pub fn decode_time(payload: &[u8]) -> Result<TimeTick, DecodeError> {
    let numbers = field_numbers(payload)?;
    let m = TimeMessage::decode(payload)?;
    Ok(TimeTick {
        micros: m.timestamp,
        source: m.source,
        is_bist_pp_open: m.is_bist_pp_open,
        is_bist_pp_holiday: m.is_bist_pp_holiday,
        fields: FieldSet::new(TIME_MESSAGE, numbers),
    })
}

/// A change reported by [`MarketClock::apply`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockEvent {
    /// First `MX` tick: openness and holiday are now known.
    Known {
        /// Exchange time of the tick (epoch µs).
        at_micros: i64,
        /// Market open.
        open: bool,
        /// Holiday.
        holiday: bool,
    },
    /// The market went from closed to open.
    Opened {
        /// Exchange time of the tick (epoch µs).
        at_micros: i64,
    },
    /// The market went from open to closed.
    Closed {
        /// Exchange time of the tick (epoch µs).
        at_micros: i64,
    },
    /// The holiday flag changed.
    HolidayChanged {
        /// Exchange time of the tick (epoch µs).
        at_micros: i64,
        /// New holiday flag.
        holiday: bool,
    },
}

/// BIST equity market open/closed state machine, driven only by `MX` ticks.
///
/// Flags are taken as sent; the clock does not infer "closed because holiday".
/// It has no wall clock: it knows only what the ticks tell it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MarketClock {
    open: Option<bool>,
    holiday: Option<bool>,
    last_mx_micros: Option<i64>,
    last_any_micros: Option<i64>,
}

impl MarketClock {
    /// A clock that has seen nothing (openness unknown).
    pub fn new() -> Self {
        Self::default()
    }

    /// Applies one tick and returns the resulting changes (in this order:
    /// `Known` | `Opened`/`Closed`, then `HolidayChanged`).
    pub fn apply(&mut self, tick: &TimeTick) -> Vec<ClockEvent> {
        self.last_any_micros = Some(tick.micros);
        if !tick.is_mx() {
            return Vec::new();
        }
        let at_micros = tick.micros;
        self.last_mx_micros = Some(at_micros);
        let (open, holiday) = (tick.is_bist_pp_open, tick.is_bist_pp_holiday);
        let mut events = Vec::new();
        match (self.open, self.holiday) {
            (Some(prev_open), Some(prev_holiday)) => {
                if open != prev_open {
                    events.push(if open {
                        ClockEvent::Opened { at_micros }
                    } else {
                        ClockEvent::Closed { at_micros }
                    });
                }
                if holiday != prev_holiday {
                    events.push(ClockEvent::HolidayChanged { at_micros, holiday });
                }
            }
            _ => events.push(ClockEvent::Known {
                at_micros,
                open,
                holiday,
            }),
        }
        self.open = Some(open);
        self.holiday = Some(holiday);
        events
    }

    /// Market open per the last `MX` tick; `None` before the first one.
    pub fn is_open(&self) -> Option<bool> {
        self.open
    }

    /// Holiday per the last `MX` tick; `None` before the first one.
    pub fn is_holiday(&self) -> Option<bool> {
        self.holiday
    }

    /// Exchange time of the last `MX` tick (epoch µs).
    pub fn last_mx_micros(&self) -> Option<i64> {
        self.last_mx_micros
    }

    /// Exchange time of the last tick from any source (epoch µs).
    pub fn last_any_micros(&self) -> Option<i64> {
        self.last_any_micros
    }

    /// Forgets everything (e.g. after a disconnect): openness becomes unknown.
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}
