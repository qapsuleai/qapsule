//! Series and provenance: bars, studies, instruments, calendars, `Series`, `Author`.

use super::*;

// ---------------------------------------------------------------------------
// Series
// ---------------------------------------------------------------------------

/// One OHLCV bar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Bar {
    /// Bar open time, epoch milliseconds UTC.
    pub t: TimestampMs,
    /// Open price.
    pub o: f64,
    /// High price.
    pub h: f64,
    /// Low price.
    pub l: f64,
    /// Close price.
    pub c: f64,
    /// Volume.
    pub v: f64,
}

/// One point of a single-valued series (e.g. an indicator output).
///
/// Warmup rule (K13): a study output always has one point per input bar, at the same
/// `t`. Points inside the warmup period have `v: null`; nothing is trimmed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Point {
    /// Point time, epoch milliseconds UTC (the time of the bar it belongs to).
    pub t: TimestampMs,
    /// Value; `null` during warmup.
    pub v: Option<f64>,
}

/// The result of running one study over a bar series (K13).
///
/// `t` has one entry per input bar; every vector in `outputs` has exactly the same
/// length as `t`. Warmup values are `null`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StudyOutput {
    /// Study name, e.g. `rsi`, `macd`.
    pub study: String,
    /// Numeric parameters, e.g. `{"period": 14}`.
    #[serde(default)]
    pub params: BTreeMap<String, f64>,
    /// Bar times, epoch milliseconds UTC; same length as every output.
    pub t: Vec<TimestampMs>,
    /// Named outputs, e.g. `macd`, `signal`, `histogram`. `null` during warmup.
    pub outputs: BTreeMap<String, Vec<Option<f64>>>,
    /// Number of leading `null` values per output (warmup length).
    #[serde(default)]
    pub warmup: BTreeMap<String, u32>,
}

/// A traded asset: where it trades and which calendar aligns its bars (K14).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Instrument {
    /// Exchange symbol, e.g. `THYAO`.
    pub symbol: String,
    /// Exchange code, e.g. `BIST`, `NYSE`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exchange: Option<String>,
    /// Calendar id, e.g. `XIST`, `XNYS`. Bars are aligned to this calendar.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calendar: Option<String>,
}

/// Weekday, ISO numbering order.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Weekday {
    /// Monday.
    Mon,
    /// Tuesday.
    Tue,
    /// Wednesday.
    Wed,
    /// Thursday.
    Thu,
    /// Friday.
    Fri,
    /// Saturday.
    Sat,
    /// Sunday.
    Sun,
}

/// One trading session in exchange local time (`HH:MM`, 24h). `close` is exclusive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Session {
    /// Local open time, `HH:MM`.
    #[schemars(regex(pattern = r"^([01][0-9]|2[0-3]):[0-5][0-9]$"))]
    pub open: String,
    /// Local close time, `HH:MM`.
    #[schemars(regex(pattern = r"^([01][0-9]|2[0-3]):[0-5][0-9]$"))]
    pub close: String,
}

/// A trading day that closes early.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HalfDay {
    /// Local date, `YYYY-MM-DD`.
    #[schemars(regex(pattern = r"^[0-9]{4}-[0-9]{2}-[0-9]{2}$"))]
    pub date: String,
    /// Local early close time, `HH:MM`.
    #[schemars(regex(pattern = r"^([01][0-9]|2[0-3]):[0-5][0-9]$"))]
    pub close: String,
}

/// An exchange calendar: time zone, sessions and holidays (K14). Data only; bar
/// boundary logic lives in `qp-calendar`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Calendar {
    /// Calendar id, e.g. `XIST`.
    pub id: String,
    /// IANA time zone of the exchange, e.g. `Europe/Istanbul`.
    pub tz: String,
    /// Regular sessions of a trading day, in local time, ascending.
    pub sessions: Vec<Session>,
    /// Non-trading weekdays.
    #[serde(default)]
    pub weekend: Vec<Weekday>,
    /// Full holidays, local dates `YYYY-MM-DD`.
    #[serde(default)]
    pub holidays: Vec<String>,
    /// Early-close days.
    #[serde(default)]
    pub half_days: Vec<HalfDay>,
}

/// Materialized series data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SeriesData {
    /// OHLCV bars, ascending by `t`.
    Bars {
        /// Bars, ascending by time.
        bars: Vec<Bar>,
    },
    /// Single-valued points, ascending by `t`.
    Points {
        /// Points, ascending by time.
        points: Vec<Point>,
    },
}

/// Where a series comes from. The core does not depend on the kind of source.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SeriesSource {
    /// Data is embedded in the series itself (`data`).
    Inline {},
    /// A local file.
    File {
        /// Path to the file.
        path: String,
    },
    /// A Matriks market data feed.
    Matriks {
        /// Exchange symbol, e.g. `THYAO`.
        symbol: String,
        /// Bar interval, e.g. `1d`, `5m`.
        interval: String,
    },
    /// A NATS subject.
    Nats {
        /// Subject name.
        subject: String,
    },
    /// A data binding (RP2, K23): the series' bars come from the data plane, resolved
    /// per principal by the host's source provider; the document carries no bars
    /// (`docs/PROTOCOL.md` §18). Fields as in [`crate::data::DataBinding`].
    Bind {
        /// Exchange symbol, e.g. `THYAO`.
        symbol: String,
        /// Bar interval, e.g. `1d`, `5m`.
        interval: String,
        /// Which bars: `{"last": n}`, `{"from_t": t}` or `{"from_t": a, "to_t": b}`.
        window: crate::data::Window,
        /// Named feed of the host; the host's default if absent.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        feed: Option<String>,
        /// Time of bar 0, fixed by the authority on first resolution (engine entry; §18.3).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        anchor_t: Option<TimestampMs>,
    },
    /// Local computation: a study applied to other series.
    Study {
        /// Study name, e.g. `rsi`.
        study: String,
        /// Input series ids.
        inputs: Vec<String>,
        /// Numeric study parameters, e.g. `{"period": 14}`.
        #[serde(default)]
        params: BTreeMap<String, f64>,
        /// Which output of a multi-output study, if any.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        output: Option<String>,
    },
}

/// Time-based numeric data: bars, an indicator output or a stream.
///
/// In a [`Document`] a series acts as a binding: `source` says where data comes
/// from and `data` may be omitted.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Series {
    /// Unique id within the document.
    pub id: String,
    /// The instrument the series belongs to, if any. Its symbol lives here (there is no
    /// separate `symbol` field), so exchange and calendar travel with it (K14).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instrument: Option<Instrument>,
    /// Bar interval, e.g. `1d`, `5m`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interval: Option<String>,
    /// Where the data comes from.
    pub source: SeriesSource,
    /// Materialized data, if present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<SeriesData>,
}

impl Series {
    /// The embedded bars, if `data` is `bars`.
    #[must_use]
    pub fn bars(&self) -> Option<&[Bar]> {
        match &self.data {
            Some(SeriesData::Bars { bars }) => Some(bars),
            _ => None,
        }
    }

    /// The instrument symbol, if any (the binding's symbol for a bound series).
    #[must_use]
    pub fn symbol(&self) -> Option<&str> {
        match (&self.instrument, &self.source) {
            (Some(i), _) => Some(i.symbol.as_str()),
            (None, SeriesSource::Bind { symbol, .. }) => Some(symbol.as_str()),
            _ => None,
        }
    }

    /// The data binding, for a bound series (`source.kind = "bind"`).
    #[must_use]
    pub fn binding(&self) -> Option<crate::data::DataBinding> {
        match &self.source {
            SeriesSource::Bind {
                symbol,
                interval,
                window,
                feed,
                anchor_t,
            } => Some(crate::data::DataBinding {
                symbol: symbol.clone(),
                interval: interval.clone(),
                window: window.clone(),
                feed: feed.clone(),
                anchor_t: *anchor_t,
            }),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Provenance
// ---------------------------------------------------------------------------

/// Who produced an item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Author {
    /// A person.
    Human {
        /// Display name or user id.
        name: String,
    },
    /// An AI agent.
    Agent {
        /// Agent name.
        name: String,
        /// Model identifier, if known.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        model: Option<String>,
    },
    /// The deterministic engine.
    Engine {
        /// Engine component name.
        name: String,
        /// Engine version.
        version: String,
    },
}

/// Who produced an item, based on what, and when.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    /// Producer.
    pub author: Author,
    /// Production time, epoch milliseconds UTC.
    pub at: TimestampMs,
    /// Ids of the items this one is based on.
    #[serde(default)]
    pub based_on: Vec<String>,
    /// Who approved it, when it entered the document through an accepted proposal
    /// (protocol: `accept_proposal`). Absent for direct writes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approved_by: Option<Author>,
}
