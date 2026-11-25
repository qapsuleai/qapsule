//! The checker's output for the executor (PR §7): a valid program, resolved — every
//! reference an index into the node table, every parameter replaced by its bound value,
//! every node typed and clocked — in topological (IR) order.

use std::collections::BTreeMap;

use qp_calendar::Interval;
use qp_schema::program::{BarField, Program, ProgramCost, ProgramParamValue, Side, SignalSource};

/// A value type (PR §3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ty {
    /// `Option<f64>`.
    Num,
    /// `Option<bool>`.
    Bool,
    /// `Option<u32>` (a bin index).
    Cat,
}

impl Ty {
    /// `num`, `bool` or `cat`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Ty::Num => "num",
            Ty::Bool => "bool",
            Ty::Cat => "cat",
        }
    }
}

/// A higher timeframe: its canonical spelling and its interval.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tf {
    /// Canonical interval (`4h`, `1d` ...; `PROTOCOL.md` §18.2).
    pub name: String,
    /// The interval.
    pub interval: Interval,
}

/// The clock of a node (PR §3.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Clock {
    /// Clockless: a constant, or a stateless step over constants only. Its value is the
    /// same at every bar of whatever clock reads it.
    Free,
    /// The bound series' bars.
    Base,
    /// A higher timeframe's closed bars.
    Tf(Tf),
}

impl Clock {
    /// `base`, the interval, or `const` for a clockless node (as `explain` lists it).
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Clock::Free => "const",
            Clock::Base => "base",
            Clock::Tf(tf) => &tf.name,
        }
    }
}

/// An input of a node: node index and output index (0 for single-output nodes; the PR
/// §4.2 position for `macd` and `bollinger`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Slot {
    /// Index into [`Checked::nodes`].
    pub node: usize,
    /// Index into that node's outputs.
    pub out: usize,
}

/// A constant's value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Literal {
    /// A number.
    Num(f64),
    /// A boolean.
    Bool(bool),
}

/// The single-input window kernels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowKind {
    /// `sma`.
    Sma,
    /// `ema`.
    Ema,
    /// `rsi`.
    Rsi,
    /// `highest`.
    Highest,
    /// `lowest`.
    Lowest,
    /// `stdev`.
    Stdev,
    /// `percentrank`.
    Percentrank,
}

/// Two-input steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryKind {
    /// `add`.
    Add,
    /// `sub`.
    Sub,
    /// `mul`.
    Mul,
    /// `div`.
    Div,
    /// `min`.
    Min,
    /// `max`.
    Max,
    /// `gt`.
    Gt,
    /// `ge`.
    Ge,
    /// `lt`.
    Lt,
    /// `le`.
    Le,
    /// `eq`.
    Eq,
    /// `ne`.
    Ne,
    /// `and`.
    And,
    /// `or`.
    Or,
    /// `cross_above`.
    CrossAbove,
    /// `cross_below`.
    CrossBelow,
}

/// One-input stateless steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryKind {
    /// `neg`.
    Neg,
    /// `abs`.
    Abs,
    /// `not`.
    Not,
    /// `defined`.
    Defined,
}

/// One-condition state nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CondKind {
    /// `streak`.
    Streak,
    /// `bars_since`.
    BarsSince,
    /// `once`.
    Once,
}

/// A `line` anchor with bound values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Anchor {
    /// Time, epoch ms.
    pub t: i64,
    /// Price (> 0).
    pub price: f64,
}

/// A node's operation with references resolved and parameters bound.
#[derive(Debug, Clone, PartialEq)]
pub enum Op {
    /// `input`: a bar field on the node's clock.
    Input(BarField),
    /// `const`.
    Const(Literal),
    /// `bar_index`.
    BarIndex,
    /// `time`.
    Time,
    /// A window kernel.
    Window {
        /// Which kernel.
        kind: WindowKind,
        /// Input.
        x: Slot,
        /// Window.
        period: usize,
    },
    /// `macd` (outputs `macd`, `signal`, `histogram`).
    Macd {
        /// Input.
        x: Slot,
        /// Fast period.
        fast: usize,
        /// Slow period (> fast).
        slow: usize,
        /// Signal period.
        signal: usize,
    },
    /// `bollinger` (outputs `upper`, `middle`, `lower`, `bandwidth`).
    Bollinger {
        /// Input.
        x: Slot,
        /// Window.
        period: usize,
        /// Width in standard deviations (> 0).
        k: f64,
    },
    /// `atr` on the node's clock.
    Atr {
        /// Window.
        period: usize,
    },
    /// A two-input step.
    Binary {
        /// Which step.
        kind: BinaryKind,
        /// Left input.
        a: Slot,
        /// Right input.
        b: Slot,
    },
    /// A one-input stateless step.
    Unary {
        /// Which step.
        kind: UnaryKind,
        /// Input.
        x: Slot,
    },
    /// `lag`.
    Lag {
        /// Input.
        x: Slot,
        /// Bars back (≥ 0).
        n: usize,
    },
    /// `streak`, `bars_since`, `once`.
    Cond {
        /// Which node.
        kind: CondKind,
        /// Condition.
        c: Slot,
    },
    /// `rearm`.
    Rearm {
        /// Trigger.
        trigger: Slot,
        /// Re-arming condition.
        reset: Slot,
    },
    /// `cooldown`.
    Cooldown {
        /// Trigger.
        trigger: Slot,
        /// Quiet bars (≥ 0).
        bars: usize,
    },
    /// `align`: a higher-timeframe input read at the base bar's close.
    Align {
        /// Input (on a higher timeframe).
        x: Slot,
    },
    /// `line` (`a.t < b.t`).
    Line {
        /// First anchor.
        a: Anchor,
        /// Second anchor.
        b: Anchor,
    },
    /// `bucket` (strictly increasing edges, 1 … 8).
    Bucket {
        /// Input.
        x: Slot,
        /// Edges.
        edges: Vec<f64>,
    },
}

/// One node of a checked program.
#[derive(Debug, Clone, PartialEq)]
pub struct CheckedNode {
    /// Node id.
    pub id: String,
    /// The operation.
    pub op: Op,
    /// Output types (one for single-output nodes).
    pub outputs: Vec<Ty>,
    /// Clock.
    pub clock: Clock,
    /// Cost weight per bar (PR §8.1), with bound values.
    pub w: u64,
}

/// A signal's outcome rule with bound values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rule {
    /// Target percent (> 0).
    pub target_pct: Option<f64>,
    /// Stop percent (> 0).
    pub stop_pct: Option<f64>,
    /// Horizon in bars (1 … `program.signal.max_horizon_bars`).
    pub max_bars: usize,
}

/// What an output emits.
#[derive(Debug, Clone, PartialEq)]
pub enum OutputSpec {
    /// A `num` per bar.
    Series(Slot),
    /// Closed bars where a `bool` is true.
    Event(Slot),
    /// A record per closed bar where a `bool` is true.
    Signal {
        /// The `bool` input.
        slot: Slot,
        /// Buy or sell.
        side: Side,
        /// Outcome rule.
        rule: Rule,
        /// Who says it.
        source: SignalSource,
    },
    /// A key per bar: named `cat` inputs.
    State(Vec<(String, Slot)>),
}

/// One output of a checked program.
#[derive(Debug, Clone, PartialEq)]
pub struct CheckedOutput {
    /// Output name.
    pub name: String,
    /// What it emits.
    pub spec: OutputSpec,
}

/// A valid program instance ready to run.
#[derive(Debug, Clone, PartialEq)]
pub struct Checked {
    /// The typed IR (as given; canonical form via `Program::canonical_json`).
    pub program: Program,
    /// `sha256:` + hex of the identity JSON.
    pub program_id: String,
    /// `sha256:` + hex of the instance input.
    pub instance_id: String,
    /// Bound values, complete (defaults filled), normalized, by key.
    pub params: BTreeMap<String, ProgramParamValue>,
    /// Nodes in IR (topological) order.
    pub nodes: Vec<CheckedNode>,
    /// Outputs in IR order.
    pub outputs: Vec<CheckedOutput>,
    /// Static cost.
    pub cost: ProgramCost,
    /// The base timeframe, when the check was bound to a series with an interval.
    pub base: Option<Tf>,
}

impl Op {
    /// The inputs it reads, in field order.
    #[must_use]
    pub fn inputs(&self) -> Vec<Slot> {
        match self {
            Op::Input(_)
            | Op::Const(_)
            | Op::BarIndex
            | Op::Time
            | Op::Atr { .. }
            | Op::Line { .. } => {
                vec![]
            }
            Op::Window { x, .. }
            | Op::Macd { x, .. }
            | Op::Bollinger { x, .. }
            | Op::Unary { x, .. }
            | Op::Lag { x, .. }
            | Op::Align { x }
            | Op::Bucket { x, .. } => vec![*x],
            Op::Binary { a, b, .. } => vec![*a, *b],
            Op::Cond { c, .. } => vec![*c],
            Op::Rearm { trigger, reset } => vec![*trigger, *reset],
            Op::Cooldown { trigger, .. } => vec![*trigger],
        }
    }
}

impl OutputSpec {
    /// The inputs it reads (a state's dimensions in order).
    #[must_use]
    pub fn inputs(&self) -> Vec<Slot> {
        match self {
            OutputSpec::Series(s) | OutputSpec::Event(s) | OutputSpec::Signal { slot: s, .. } => {
                vec![*s]
            }
            OutputSpec::State(dims) => dims.iter().map(|(_, s)| *s).collect(),
        }
    }
}
