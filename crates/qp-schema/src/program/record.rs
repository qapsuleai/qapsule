//! The signal record (PR §6; RX5 `LedgerEntry` names, identity by time) and value
//! provenance (PR §9).

use super::*;

use crate::data::FeedKey;
use crate::Author;

/// What produced a record. RL1: signals only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RecordSourceKind {
    /// A program's signal output.
    Signal,
}

/// How a record was born (X26).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RecordMode {
    /// A replay (`try_program`); the only mode in RL1.
    Replay,
    /// Live (RS1).
    Live,
}

/// The label a record is shown with (K77).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RecordLabel {
    /// Simulated (every replay record).
    Simulated,
}

/// Where a signal record comes from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RecordProvenance {
    /// The instance's owner.
    pub owner: String,
    /// Who wrote the program.
    pub author: Author,
    /// Program identity.
    pub program_id: String,
    /// Instance identity.
    pub instance_id: String,
    /// Bound values, by key.
    pub params: BTreeMap<String, ProgramParamValue>,
    /// The signal source's strategy.
    pub strategy: String,
    /// The signal source's version.
    pub program_version: String,
}

/// The state key a signal was born in, if the program has a state definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RecordState {
    /// The state output's name.
    pub output: String,
    /// Dimension name → bin index.
    pub key: BTreeMap<String, u32>,
}

/// How a signal ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OutcomeKind {
    /// Target reached.
    Target,
    /// Stop reached (also when both in one bar).
    Stop,
    /// Horizon reached.
    Expired,
    /// Data ended first.
    Open,
}

/// The outcome of a signal (`exit_*` absent while `open`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SignalOutcome {
    /// How it ended.
    pub kind: OutcomeKind,
    /// Exit bar time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_t: Option<TimestampMs>,
    /// Exit price.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_price: Option<f64>,
}

/// The data a record or a value rests on: the bar series, its anchor and feeds
/// (PR §9; `PROTOCOL.md` §18.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RecordEvidence {
    /// The bound bar series.
    pub series: String,
    /// First bar of the provenance range (`lo`).
    pub from_t: TimestampMs,
    /// The signal bar.
    pub to_t: TimestampMs,
    /// The series' anchor (bar 0 time); absent for inline series.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor_t: Option<TimestampMs>,
    /// The bound feeds it rests on.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub feeds: Vec<FeedKey>,
}

/// One signal record (PR §6).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SignalRecord {
    /// `<instance>.<output>@<born_t>`.
    pub id: String,
    /// What produced it.
    pub source_kind: RecordSourceKind,
    /// Replay or live.
    pub mode: RecordMode,
    /// `simulated` for replay records.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<RecordLabel>,
    /// Where it comes from.
    pub provenance: RecordProvenance,
    /// Instrument symbol.
    pub instrument: String,
    /// Bar interval.
    pub period: String,
    /// Signal bar open time.
    pub born_t: TimestampMs,
    /// Signal bar close time.
    pub known_t: TimestampMs,
    /// Buy or sell.
    pub side: Side,
    /// Entry price (the signal bar's close).
    pub entry: f64,
    /// The outcome rule, values bound.
    pub rule: OutcomeRule,
    /// The state key at birth, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<RecordState>,
    /// How it ended.
    pub outcome: SignalOutcome,
    /// What it rests on.
    pub evidence: RecordEvidence,
}

/// The data part of a value's provenance (PR §9).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProvenanceData {
    /// The bound bar series.
    pub series: String,
    /// Its anchor; absent for inline series.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor_t: Option<TimestampMs>,
    /// Its bound feeds.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub feeds: Vec<FeedKey>,
}

/// Where a published value comes from (PR §9): it depends only on base bars `range`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ValueProvenance {
    /// Output name.
    pub output: String,
    /// The output's node.
    pub node: String,
    /// Bar index.
    pub bar: u64,
    /// Bar time.
    pub t: TimestampMs,
    /// `[lo, i]`, base bar indexes, inclusive.
    pub range: [u64; 2],
    /// Program identity.
    pub program_id: String,
    /// The data.
    pub data: ProvenanceData,
}
