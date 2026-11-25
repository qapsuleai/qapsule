//! Outputs (PR §5) and the signal's outcome rule (PR §6).

use super::*;

/// One named output: `{"name": ..., "kind": ..., <fields>}`. Name
/// `^[a-z][a-z0-9_]{0,31}$`, unique in the program; its node is on the base clock.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[schemars(extend("unevaluatedProperties" = false))]
pub struct Output {
    /// Output name.
    pub name: String,
    /// Kind and fields.
    #[serde(flatten)]
    pub kind: OutputKind,
}

impl<'de> Deserialize<'de> for Output {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let (name, kind) = head::split(d, "name")?;
        Ok(Output { name, kind })
    }
}

/// The side of a signal. "Hold" makes no record (X30/8): it does not exist here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    /// Buy.
    Al,
    /// Sell.
    Sat,
}

/// The outcome rule a signal is born with (PR §6). Every value may be a parameter.
/// Absent `kind` means the single-stage rule (RS1 may add others as an optional field).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OutcomeRule {
    /// Target in percent of the entry (> 0).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_pct: Option<Num>,
    /// Stop in percent of the entry (> 0).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_pct: Option<Num>,
    /// Horizon in bars, `1 ... program.signal.max_horizon_bars`.
    pub max_bars: IntArg,
}

/// Who says a signal (K77): the strategy's name and version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SignalSource {
    /// Strategy name.
    pub strategy: String,
    /// Strategy version.
    pub version: String,
}

/// One dimension of a state definition: a named `bucket` node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StateDim {
    /// Dimension name.
    pub name: String,
    /// A `cat` node.
    pub node: NodeRef,
}

/// The kind of an output and its fields, tagged with `kind` (PR §5).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum OutputKind {
    /// A number (or null) per bar; a compute series in the document.
    Series {
        /// A `num` node.
        node: NodeRef,
    },
    /// The closed bars where the node is true; an event series (1/0) in the document.
    Event {
        /// A `bool` node.
        node: NodeRef,
    },
    /// A record per closed bar where the node is true (try only in RL1).
    Signal {
        /// A `bool` node.
        node: NodeRef,
        /// Buy or sell.
        side: Side,
        /// The outcome rule (required, X25).
        rule: OutcomeRule,
        /// Who says it.
        source: SignalSource,
    },
    /// A state key per bar (try only in RL1).
    State {
        /// 1 ... 4 dimensions.
        dims: Vec<StateDim>,
    },
}

/// The bare kind of an output, as answers list it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProgramOutputKind {
    /// `series`.
    Series,
    /// `event`.
    Event,
    /// `signal`.
    Signal,
    /// `state`.
    State,
}

impl OutputKind {
    /// The bare kind.
    #[must_use]
    pub const fn tag(&self) -> ProgramOutputKind {
        match self {
            OutputKind::Series { .. } => ProgramOutputKind::Series,
            OutputKind::Event { .. } => ProgramOutputKind::Event,
            OutputKind::Signal { .. } => ProgramOutputKind::Signal,
            OutputKind::State { .. } => ProgramOutputKind::State,
        }
    }

    /// Whether the output may enter a document (series and event; PR §5).
    #[must_use]
    pub const fn documentable(&self) -> bool {
        matches!(self, OutputKind::Series { .. } | OutputKind::Event { .. })
    }
}
