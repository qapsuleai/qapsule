//! Nodes (PR §4): one step of a program, `{"id": ..., "op": ..., <fields>}`.

use super::*;

/// A reference to an earlier node: `"id"`, or `"id.output"` for a multi-output node
/// (PR §4.2; the bare id is the main output).
pub type NodeRef = String;

/// One node: its id and its operation. Id pattern `^[A-Za-z_][A-Za-z0-9_]{0,63}$`, unique
/// in the program; ids starting with `_` are the text language's unnamed nodes.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[schemars(extend("unevaluatedProperties" = false))]
pub struct Node {
    /// Node id.
    pub id: String,
    /// What it computes (`op` and its fields).
    #[serde(flatten)]
    pub op: NodeOp,
}

impl<'de> Deserialize<'de> for Node {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let (id, op) = head::split(d, "id")?;
        Ok(Node { id, op })
    }
}

/// A bar field (PR §4.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BarField {
    /// Open.
    Open,
    /// High.
    High,
    /// Low.
    Low,
    /// Close.
    Close,
    /// Volume.
    Volume,
}

/// `input`: a field of the closed bars of a timeframe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InputNode {
    /// The bar field.
    pub field: BarField,
    /// Higher timeframe (canonical interval, e.g. `4h`, `1d`); the base if absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tf: Option<String>,
}

/// The value of a `const`: a number, a boolean or a parameter (untagged).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ConstValue {
    /// Boolean.
    Bool(bool),
    /// Number (an integer literal reads as a float).
    Num(f64),
    /// A parameter.
    Param(ParamRef),
}

/// `const`: a clockless constant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConstNode {
    /// The value.
    pub value: ConstValue,
}

/// A single-input window kernel: `sma ema rsi highest lowest stdev percentrank`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WindowNode {
    /// Input.
    pub x: NodeRef,
    /// Window, `1 ... program.max_window`.
    pub period: IntArg,
}

/// `macd`: outputs `macd` (main), `signal`, `histogram`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MacdNode {
    /// Input.
    pub x: NodeRef,
    /// Fast period.
    pub fast: IntArg,
    /// Slow period.
    pub slow: IntArg,
    /// Signal period.
    pub signal: IntArg,
}

/// `bollinger`: outputs `upper`, `middle` (main), `lower`, `bandwidth`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BollingerNode {
    /// Input.
    pub x: NodeRef,
    /// Window.
    pub period: IntArg,
    /// Band width in standard deviations (> 0).
    pub k: Num,
}

/// `atr`: average true range of a timeframe's bars.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AtrNode {
    /// Window.
    pub period: IntArg,
    /// Higher timeframe; the base if absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tf: Option<String>,
}

/// One input `x`: `neg abs not defined align`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UnaryNode {
    /// Input.
    pub x: NodeRef,
}

/// Two inputs `a`, `b`: arithmetic, comparison, `and or`, `cross_above cross_below`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BinaryNode {
    /// Left input.
    pub a: NodeRef,
    /// Right input.
    pub b: NodeRef,
}

/// `lag`: the input `n` bars ago.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LagNode {
    /// Input.
    pub x: NodeRef,
    /// Bars back, `0 ... program.max_window` (negative: `E_LOOKAHEAD`).
    pub n: IntArg,
}

/// One condition `c`: `streak bars_since once`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CondNode {
    /// Condition (bool; null counts as false).
    pub c: NodeRef,
}

/// `rearm`: fires on `trigger` while armed, re-arms on `reset`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RearmNode {
    /// Trigger.
    pub trigger: NodeRef,
    /// Re-arming condition.
    pub reset: NodeRef,
}

/// `cooldown`: fires on `trigger` unless it fired within the last `bars` bars.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CooldownNode {
    /// Trigger.
    pub trigger: NodeRef,
    /// Quiet bars, `0 ... program.max_window`.
    pub bars: IntArg,
}

/// An anchor of a `line`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LinePoint {
    /// Time (epoch ms; an `int` parameter allowed).
    pub t: IntArg,
    /// Price (> 0; a `float` parameter allowed).
    pub price: Num,
}

/// `line`: the straight line through two anchors, on base bars.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LineNode {
    /// First anchor.
    pub a: LinePoint,
    /// Second anchor (`a.t < b.t`).
    pub b: LinePoint,
}

/// `bucket`: the bin index of `x` among strictly increasing `edges` (1 ... 8).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BucketNode {
    /// Input.
    pub x: NodeRef,
    /// Bin edges.
    pub edges: Vec<Num>,
}

/// The operation of a node, tagged with `op` (PR §4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum NodeOp {
    /// A bar field (num).
    Input(InputNode),
    /// A constant (num or bool, clockless).
    Const(ConstNode),
    /// The bar index `i` (num, base).
    BarIndex {},
    /// The bar open time `t_i` in ms (num, base).
    Time {},
    /// Simple moving average.
    Sma(WindowNode),
    /// Exponential moving average.
    Ema(WindowNode),
    /// Relative strength index.
    Rsi(WindowNode),
    /// MACD (multi-output).
    Macd(MacdNode),
    /// Bollinger bands (multi-output).
    Bollinger(BollingerNode),
    /// Highest of the window.
    Highest(WindowNode),
    /// Lowest of the window.
    Lowest(WindowNode),
    /// Average true range.
    Atr(AtrNode),
    /// Population standard deviation of the window.
    Stdev(WindowNode),
    /// Percent rank among the previous `period` values.
    Percentrank(WindowNode),
    /// `a + b`.
    Add(BinaryNode),
    /// `a - b`.
    Sub(BinaryNode),
    /// `a * b`.
    Mul(BinaryNode),
    /// `a / b` (null on division by zero).
    Div(BinaryNode),
    /// `b < a ? b : a`.
    Min(BinaryNode),
    /// `b > a ? b : a`.
    Max(BinaryNode),
    /// `-x`.
    Neg(UnaryNode),
    /// `|x|`.
    Abs(UnaryNode),
    /// `x[i - n]`.
    Lag(LagNode),
    /// `a > b`.
    Gt(BinaryNode),
    /// `a >= b`.
    Ge(BinaryNode),
    /// `a < b`.
    Lt(BinaryNode),
    /// `a <= b`.
    Le(BinaryNode),
    /// `a == b`.
    Eq(BinaryNode),
    /// `a != b`.
    Ne(BinaryNode),
    /// Logical and.
    And(BinaryNode),
    /// Logical or.
    Or(BinaryNode),
    /// Logical not.
    Not(UnaryNode),
    /// `a` crosses above `b`.
    CrossAbove(BinaryNode),
    /// `a` crosses below `b`.
    CrossBelow(BinaryNode),
    /// Bars in a row `c` is true (num).
    Streak(CondNode),
    /// Bars since `c` was last true (num).
    BarsSince(CondNode),
    /// True only at the first bar `c` is true.
    Once(CondNode),
    /// Fires once per arming.
    Rearm(RearmNode),
    /// Fires at most once per `bars + 1` bars.
    Cooldown(CooldownNode),
    /// Whether `x` is defined at this bar (never null).
    Defined(UnaryNode),
    /// A higher-timeframe value at the base bar's close (last closed higher bar).
    Align(UnaryNode),
    /// A straight line through two anchors.
    Line(LineNode),
    /// Bin index (cat).
    Bucket(BucketNode),
}

impl NodeOp {
    /// The `op` name as written in the IR.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            NodeOp::Input(_) => "input",
            NodeOp::Const(_) => "const",
            NodeOp::BarIndex {} => "bar_index",
            NodeOp::Time {} => "time",
            NodeOp::Sma(_) => "sma",
            NodeOp::Ema(_) => "ema",
            NodeOp::Rsi(_) => "rsi",
            NodeOp::Macd(_) => "macd",
            NodeOp::Bollinger(_) => "bollinger",
            NodeOp::Highest(_) => "highest",
            NodeOp::Lowest(_) => "lowest",
            NodeOp::Atr(_) => "atr",
            NodeOp::Stdev(_) => "stdev",
            NodeOp::Percentrank(_) => "percentrank",
            NodeOp::Add(_) => "add",
            NodeOp::Sub(_) => "sub",
            NodeOp::Mul(_) => "mul",
            NodeOp::Div(_) => "div",
            NodeOp::Min(_) => "min",
            NodeOp::Max(_) => "max",
            NodeOp::Neg(_) => "neg",
            NodeOp::Abs(_) => "abs",
            NodeOp::Lag(_) => "lag",
            NodeOp::Gt(_) => "gt",
            NodeOp::Ge(_) => "ge",
            NodeOp::Lt(_) => "lt",
            NodeOp::Le(_) => "le",
            NodeOp::Eq(_) => "eq",
            NodeOp::Ne(_) => "ne",
            NodeOp::And(_) => "and",
            NodeOp::Or(_) => "or",
            NodeOp::Not(_) => "not",
            NodeOp::CrossAbove(_) => "cross_above",
            NodeOp::CrossBelow(_) => "cross_below",
            NodeOp::Streak(_) => "streak",
            NodeOp::BarsSince(_) => "bars_since",
            NodeOp::Once(_) => "once",
            NodeOp::Rearm(_) => "rearm",
            NodeOp::Cooldown(_) => "cooldown",
            NodeOp::Defined(_) => "defined",
            NodeOp::Align(_) => "align",
            NodeOp::Line(_) => "line",
            NodeOp::Bucket(_) => "bucket",
        }
    }

    /// The node references it reads, in field order.
    #[must_use]
    pub fn refs(&self) -> Vec<&str> {
        match self {
            NodeOp::Input(_)
            | NodeOp::Const(_)
            | NodeOp::BarIndex {}
            | NodeOp::Time {}
            | NodeOp::Atr(_)
            | NodeOp::Line(_) => vec![],
            NodeOp::Sma(w)
            | NodeOp::Ema(w)
            | NodeOp::Rsi(w)
            | NodeOp::Highest(w)
            | NodeOp::Lowest(w)
            | NodeOp::Stdev(w)
            | NodeOp::Percentrank(w) => vec![w.x.as_str()],
            NodeOp::Macd(m) => vec![m.x.as_str()],
            NodeOp::Bollinger(b) => vec![b.x.as_str()],
            NodeOp::Lag(l) => vec![l.x.as_str()],
            NodeOp::Bucket(b) => vec![b.x.as_str()],
            NodeOp::Neg(u)
            | NodeOp::Abs(u)
            | NodeOp::Not(u)
            | NodeOp::Defined(u)
            | NodeOp::Align(u) => vec![u.x.as_str()],
            NodeOp::Streak(c) | NodeOp::BarsSince(c) | NodeOp::Once(c) => vec![c.c.as_str()],
            NodeOp::Rearm(r) => vec![r.trigger.as_str(), r.reset.as_str()],
            NodeOp::Cooldown(c) => vec![c.trigger.as_str()],
            NodeOp::Add(b)
            | NodeOp::Sub(b)
            | NodeOp::Mul(b)
            | NodeOp::Div(b)
            | NodeOp::Min(b)
            | NodeOp::Max(b)
            | NodeOp::Gt(b)
            | NodeOp::Ge(b)
            | NodeOp::Lt(b)
            | NodeOp::Le(b)
            | NodeOp::Eq(b)
            | NodeOp::Ne(b)
            | NodeOp::And(b)
            | NodeOp::Or(b)
            | NodeOp::CrossAbove(b)
            | NodeOp::CrossBelow(b) => vec![b.a.as_str(), b.b.as_str()],
        }
    }

    /// Output names of a multi-output node, main output in PR §4.2 position
    /// (`macd`: `macd`, `signal`, `histogram`; `bollinger`: `upper`, `middle`, `lower`,
    /// `bandwidth`); empty for single-output nodes.
    #[must_use]
    pub const fn outputs(&self) -> &'static [&'static str] {
        match self {
            NodeOp::Macd(_) => &["macd", "signal", "histogram"],
            NodeOp::Bollinger(_) => &["upper", "middle", "lower", "bandwidth"],
            _ => &[],
        }
    }

    /// The main output of a multi-output node (what a bare reference reads).
    #[must_use]
    pub const fn main_output(&self) -> Option<&'static str> {
        match self {
            NodeOp::Macd(_) => Some("macd"),
            NodeOp::Bollinger(_) => Some("middle"),
            _ => None,
        }
    }
}
