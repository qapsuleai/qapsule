//! Answers of the program queries (PS §3.2): `program_check`, `program_try`,
//! `program_explain` (as `QueryResult` variants, fields flat under `data`).

use super::*;

use crate::protocol::ProgramError;
use crate::Point;

/// Static cost (PR §8.1), in cost units.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProgramCost {
    /// Σ node weight + output count.
    pub per_bar: u64,
    /// Σ over outputs of (their cone's Σ weight + 1).
    pub live_per_bar: u64,
    /// Bars of the bound series, if bound.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bars: Option<u64>,
    /// `per_bar × bars`, if bound.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
    /// `program.max_cost_per_bar`.
    pub limit: u64,
}

/// One output as answers list it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProgramOutputInfo {
    /// Output name.
    pub name: String,
    /// Its kind.
    pub kind: ProgramOutputKind,
    /// Whether `add_program` can put it in a document (series and event).
    pub documentable: bool,
}

/// Answer to `check_program` (`data.kind = "program_check"`). Program errors are inside
/// the answer, not a protocol error.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProgramCheck {
    /// No errors (a valid refusal is `ok` too).
    pub ok: bool,
    /// Every error, in position order (PR §11.1).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<ProgramError>,
    /// More than `program.check.max_errors` errors: the rest were cut.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub truncated: bool,
    /// Warnings (`W_` reasons, no code; PR §11.3).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<ProgramError>,
    /// Static cost, when the program typed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<ProgramCost>,
    /// The outputs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outputs: Vec<ProgramOutputInfo>,
    /// The parameter definitions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<ProgramParamDef>,
    /// Program identity, when the program parsed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub program_id: Option<String>,
    /// The limits in force (registry key → value).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub limits: BTreeMap<String, ProgramParamValue>,
    /// The IR lowered from `source`, when text was given and lowered.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ir: Option<Program>,
    /// The refusal, when the input is one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reject: Option<ProgramReject>,
}

/// A series output in a try.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TrySeries {
    /// Output name.
    pub name: String,
    /// Defined values.
    pub defined: u64,
    /// The last values.
    pub last: Vec<Point>,
}

/// An event's bar: open time and close time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EventAt {
    /// Bar open time.
    pub t: TimestampMs,
    /// Bar close time.
    pub known_t: TimestampMs,
}

/// An event output in a try (PS §1.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TryEvent {
    /// Output name.
    pub name: String,
    /// Event bars.
    pub events: u64,
    /// Runs of consecutive event bars.
    pub runs: u64,
    /// Notifications the live path would give (= `runs`, an upper bound).
    pub notifications: u64,
    /// `runs < events`.
    pub consecutive: bool,
    /// Single-bar runs (may be lost to a batch append).
    pub isolated: u64,
    /// The first events.
    pub first: Vec<EventAt>,
    /// The last events.
    pub last: Vec<EventAt>,
}

/// Outcome counts of a signal output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OutcomeCounts {
    /// `target`.
    pub target: u64,
    /// `stop`.
    pub stop: u64,
    /// `expired`.
    pub expired: u64,
    /// `open`.
    pub open: u64,
}

/// A signal output in a try.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TrySignal {
    /// Output name.
    pub name: String,
    /// Records born.
    pub count: u64,
    /// By outcome.
    pub outcomes: OutcomeCounts,
    /// The first records (`mode: "replay"`, `label: "simulated"`).
    pub first: Vec<SignalRecord>,
}

/// A state key and how many bars had it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StateCount {
    /// Dimension name → bin index.
    pub key: BTreeMap<String, u32>,
    /// Closed bars.
    pub count: u64,
}

/// A state output in a try.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TryState {
    /// Output name.
    pub name: String,
    /// Distinct keys.
    pub distinct: u64,
    /// The most frequent keys.
    pub top: Vec<StateCount>,
}

/// Work used by a try (PR §8.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TryWork {
    /// Work used.
    pub used: u64,
    /// `program.try.max_work`.
    pub limit: u64,
}

/// Why a try stopped early.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    /// The work budget ran out.
    Budget,
}

/// Where and why a try stopped (not an error; the summary runs up to `bar`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TryStopped {
    /// Why.
    pub reason: StopReason,
    /// Bar index.
    pub bar: u64,
    /// Bar time.
    pub t: TimestampMs,
    /// Work used.
    pub work: u64,
    /// The limit.
    pub limit: u64,
}

/// Answer to `try_program` (`data.kind = "program_try"`; replay mode, PR §8.3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProgramTry {
    /// Effective start: the first bar read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_t: Option<TimestampMs>,
    /// The last bar read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_t: Option<TimestampMs>,
    /// Series outputs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub series: Vec<TrySeries>,
    /// Event outputs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub event: Vec<TryEvent>,
    /// Signal outputs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub signal: Vec<TrySignal>,
    /// State outputs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub state: Vec<TryState>,
    /// Base bars in no session (not in any higher bar).
    pub dropped: u64,
    /// Static cost.
    pub cost: ProgramCost,
    /// Work used.
    pub work: TryWork,
    /// Set when the budget stopped it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stopped: Option<TryStopped>,
}

/// The cost line of one node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NodeCost {
    /// Node id.
    pub id: String,
    /// Its `op`.
    pub op: String,
    /// Its clock: `base` or an interval (`4h`, `1d` ...).
    pub clock: String,
    /// Weight per bar.
    pub w: u64,
}

/// Which value `explain_program` traces (PR §9).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExplainAt {
    /// Output name.
    pub output: String,
    /// Bar time.
    pub t: TimestampMs,
}

/// Answer to `explain_program` (`data.kind = "program_explain"`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProgramExplain {
    /// Canonical text.
    pub text: String,
    /// Program identity.
    pub program_id: String,
    /// Instance identity, for an instance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
    /// Bound values.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub params: BTreeMap<String, ProgramParamValue>,
    /// Per node cost.
    pub cost: Vec<NodeCost>,
    /// The outputs.
    pub outputs: Vec<ProgramOutputInfo>,
    /// With `at`: that value's provenance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<ValueProvenance>,
}
