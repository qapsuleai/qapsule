//! Schema v0 types. Names follow `docs/CONCEPTS.md`.
//!
//! Conventions:
//! - Field names are `snake_case`; enums are tagged with `kind` (or `op` for commands).
//! - Timestamps are `i64` epoch milliseconds, UTC.
//! - Prices and computed values are `f64`.
//! - Maps are `BTreeMap` so serialization order is deterministic.
//! - References between items are by `id` string.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Schema version written into every [`Document`].
pub const SCHEMA_VERSION: &str = "0";

/// Epoch milliseconds, UTC.
pub type TimestampMs = i64;

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
    Inline,
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

    /// The instrument symbol, if any.
    #[must_use]
    pub fn symbol(&self) -> Option<&str> {
        self.instrument.as_ref().map(|i| i.symbol.as_str())
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
}

// ---------------------------------------------------------------------------
// Fact, Finding, Capsule, Glyph
// ---------------------------------------------------------------------------

/// An indisputable numeric event detected by the engine. Carries its evidence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Fact {
    /// Unique id within the document.
    pub id: String,
    /// Event kind, e.g. `cross_above`.
    pub kind: String,
    /// Id of the series the event happened on.
    pub series: String,
    /// Bar index within the series.
    pub bar: u64,
    /// Bar time, epoch milliseconds UTC.
    pub t: TimestampMs,
    /// Short human-readable statement.
    pub text: String,
    /// Evidence values, e.g. `{"level": 70, "prev": 68.9, "value": 71.4}`.
    #[serde(default)]
    pub evidence: BTreeMap<String, f64>,
    /// Who detected it.
    pub provenance: Provenance,
}

/// Comparison operator used in conditions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CompareOp {
    /// Strictly less than.
    Lt,
    /// Less than or equal.
    Le,
    /// Strictly greater than.
    Gt,
    /// Greater than or equal.
    Ge,
    /// Crosses the value upwards.
    CrossAbove,
    /// Crosses the value downwards.
    CrossBelow,
}

/// A condition evaluated over series (invalidation, intent trigger).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Condition {
    /// Last value of a series compared against a constant.
    Threshold {
        /// Series id.
        series: String,
        /// Comparison.
        op: CompareOp,
        /// Constant to compare against.
        value: f64,
    },
    /// Free text; evaluated by a person.
    Text {
        /// Condition statement.
        text: String,
    },
}

/// A claim derived from facts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Finding {
    /// Unique id within the document.
    pub id: String,
    /// The claim, in one sentence.
    pub claim: String,
    /// Ids of the facts the claim rests on.
    pub facts: Vec<String>,
    /// Confidence in `[0, 1]`.
    #[schemars(range(min = 0.0, max = 1.0))]
    pub confidence: f64,
    /// When this condition holds, the finding is invalid.
    pub invalidation: Condition,
    /// Author and basis.
    pub provenance: Provenance,
}

/// Lifecycle state of a capsule:
/// `draft -> live -> (updated)* -> invalidated | expired | approved | rejected`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CapsuleState {
    /// Not yet shown as live.
    Draft,
    /// Updates itself as its series change.
    Live,
    /// The invalidation condition happened.
    Invalidated,
    /// Validity window elapsed.
    Expired,
    /// A person approved it.
    Approved,
    /// A person rejected it.
    Rejected,
}

/// Kind of meaning glyph (initial candidate set; finalized in R3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GlyphKind {
    /// Where is it going?
    Direction,
    /// How strong is the move?
    Strength,
    /// How sure are we?
    Uncertainty,
    /// Is it unusual?
    Extremity,
    /// Has it happened before, and what followed?
    Similarity,
    /// What changed since the last look?
    Change,
}

/// A one-glance visual of meaning, not data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Glyph {
    /// Which glyph.
    pub kind: GlyphKind,
    /// Normalized magnitude; `[-1, 1]` for direction, `[0, 1]` otherwise.
    #[schemars(range(min = -1.0, max = 1.0))]
    pub value: f64,
}

/// Semantic zoom level: 0 sentence, 1 capsule, 2 marked chart, 3 full workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Depth {
    /// Depth 0: one sentence.
    Sentence,
    /// Depth 1: the capsule.
    Capsule,
    /// Depth 2: chart with marks.
    Chart,
    /// Depth 3: full workspace.
    Workspace,
}

/// The live, openable, portable presentation of a single finding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Capsule {
    /// Unique id within the document.
    pub id: String,
    /// Id of the finding it presents.
    pub finding: String,
    /// Lifecycle state.
    pub state: CapsuleState,
    /// Number of live updates so far.
    #[serde(default)]
    pub revision: u32,
    /// Depth-0 sentence.
    pub headline: String,
    /// Glyphs shown at a glance.
    #[serde(default)]
    pub glyphs: Vec<Glyph>,
    /// Id of the view used when the capsule is opened.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view: Option<String>,
    /// Depth the capsule is shown at by default.
    pub depth: Depth,
    /// Expiry time, epoch milliseconds UTC.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<TimestampMs>,
    /// Who made it and when.
    pub provenance: Provenance,
}

// ---------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------

/// Visual mark of a layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Mark {
    /// Candlesticks (needs bars).
    Candle,
    /// Polyline.
    Line,
    /// Filled area.
    Area,
    /// Vertical bars.
    Bar,
    /// Horizontal or vertical reference line.
    Rule,
    /// Points.
    Point,
    /// Text label.
    Text,
}

/// Scale type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ScaleKind {
    /// Time axis.
    Time,
    /// Linear numeric.
    Linear,
    /// Logarithmic numeric.
    Log,
}

/// A scale mapping data values to a visual axis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Scale {
    /// Scale type.
    pub kind: ScaleKind,
    /// Fixed domain `[min, max]`; automatic when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain: Option<[f64; 2]>,
}

/// One layer of a view: a mark bound to a series.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Layer {
    /// Visual mark.
    pub mark: Mark,
    /// Series id; absent for constant marks (e.g. a rule).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub series: Option<String>,
    /// Panel index (0 = main price panel).
    #[serde(default)]
    pub panel: u32,
    /// Encoding: visual channel -> field or constant, e.g. `{"y": "v", "color": "#888"}`.
    #[serde(default)]
    pub encoding: BTreeMap<String, String>,
}

/// A presentation recipe written in the visual grammar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct View {
    /// Unique id within the document.
    pub id: String,
    /// Optional title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Display time zone override (IANA); display only, see [`Document::display_tz`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_tz: Option<String>,
    /// Horizontal scale.
    pub x: Scale,
    /// Vertical scale per panel, indexed by `Layer::panel`.
    pub y: Vec<Scale>,
    /// Layers, drawn in order.
    pub layers: Vec<Layer>,
}

// ---------------------------------------------------------------------------
// Intent, Ledger
// ---------------------------------------------------------------------------

/// A persistent request left by the user, e.g. "tell me if the uptrend breaks".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Intent {
    /// Unique id within the document.
    pub id: String,
    /// The request as the user said it.
    pub text: String,
    /// Machine-checkable trigger, if one could be derived.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger: Option<Condition>,
    /// Whether the intent is still active.
    pub active: bool,
    /// Who left it and when.
    pub provenance: Provenance,
}

/// Outcome of a finding compared against what actually happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    /// Not yet decided.
    Pending,
    /// Reality agreed with the claim.
    Confirmed,
    /// Reality contradicted the claim.
    Refuted,
}

/// One record in the ledger.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LedgerEntry {
    /// Finding id.
    pub finding: String,
    /// Final capsule state, if any (e.g. approved / rejected by a person).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<CapsuleState>,
    /// Outcome against reality.
    pub outcome: Outcome,
    /// Record time, epoch milliseconds UTC.
    pub recorded_at: TimestampMs,
    /// Optional note.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// The record of findings compared with reality afterwards.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Ledger {
    /// Entries, in record order.
    #[serde(default)]
    pub entries: Vec<LedgerEntry>,
}

// ---------------------------------------------------------------------------
// Document, Command
// ---------------------------------------------------------------------------

/// Everything: series bindings, views, facts, findings, capsules, intents and the ledger.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Document {
    /// Schema version; `"0"` for this schema.
    pub schema_version: String,
    /// Document id.
    pub id: String,
    /// Optional title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Time zone (IANA) in which times are *shown* to the user. Display only: it never
    /// affects bar alignment or computation (K14); views may override it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_tz: Option<String>,
    /// Series bindings.
    #[serde(default)]
    pub series: Vec<Series>,
    /// Views.
    #[serde(default)]
    pub views: Vec<View>,
    /// Facts.
    #[serde(default)]
    pub facts: Vec<Fact>,
    /// Findings.
    #[serde(default)]
    pub findings: Vec<Finding>,
    /// Capsules.
    #[serde(default)]
    pub capsules: Vec<Capsule>,
    /// Intents.
    #[serde(default)]
    pub intents: Vec<Intent>,
    /// Ledger.
    #[serde(default)]
    pub ledger: Ledger,
}

/// The operation a command performs on a document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Op {
    /// Add or replace a series binding (by id).
    PutSeries {
        /// The series.
        series: Series,
    },
    /// Add or replace a view (by id).
    PutView {
        /// The view.
        view: View,
    },
    /// Add a fact.
    AddFact {
        /// The fact.
        fact: Fact,
    },
    /// Add a finding.
    AddFinding {
        /// The finding.
        finding: Finding,
    },
    /// Add a capsule.
    AddCapsule {
        /// The capsule.
        capsule: Capsule,
    },
    /// Move a capsule to a new lifecycle state.
    SetCapsuleState {
        /// Capsule id.
        capsule: String,
        /// New state.
        state: CapsuleState,
    },
    /// Add an intent.
    AddIntent {
        /// The intent.
        intent: Intent,
    },
    /// Append a ledger entry.
    RecordLedger {
        /// The entry.
        entry: LedgerEntry,
    },
    /// Remove an item by id.
    Remove {
        /// Item id.
        id: String,
    },
}

/// The only way to change a document. Humans and agents use the same commands.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Command {
    /// Who issued the command and when.
    pub provenance: Provenance,
    /// What to do.
    pub op: Op,
}
