//! The syntax tree of one text file (PR §12.1), before names are resolved.

use qp_schema::protocol::Span;

/// A binary operator of the text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
    And,
    Or,
}

impl BinOp {
    /// The IR `op` name.
    pub(crate) const fn op(self) -> &'static str {
        match self {
            BinOp::Add => "add",
            BinOp::Sub => "sub",
            BinOp::Mul => "mul",
            BinOp::Div => "div",
            BinOp::Lt => "lt",
            BinOp::Le => "le",
            BinOp::Gt => "gt",
            BinOp::Ge => "ge",
            BinOp::Eq => "eq",
            BinOp::Ne => "ne",
            BinOp::And => "and",
            BinOp::Or => "or",
        }
    }
}

/// An expression with its span.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

/// What an expression is.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ExprKind {
    /// A number literal as written; `neg` for a minus sign directly before it.
    Num {
        text: String,
        neg: bool,
    },
    Bool(bool),
    /// A name: parameter or binding (resolved in lowering).
    Name(String),
    /// A bar field, with a higher timeframe for `close@1d`.
    Field {
        field: String,
        tf: Option<String>,
    },
    /// `bar_index` or `time`.
    Builtin(&'static str),
    Call {
        name: String,
        name_span: Span,
        args: Vec<Expr>,
    },
    Neg(Box<Expr>),
    Not(Box<Expr>),
    Bin {
        op: BinOp,
        a: Box<Expr>,
        b: Box<Expr>,
    },
    /// `x[n]`.
    Lag {
        x: Box<Expr>,
        n: Box<Expr>,
    },
    /// `x.output`.
    Select {
        x: Box<Expr>,
        out: String,
    },
    /// `[e1, e2]`: only as the edges of `bucket`.
    List(Vec<Expr>),
    /// `4h`: only as the timeframe of `atr`.
    Interval(String),
}

/// A literal of a parameter line.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ParamLit {
    Num { text: String, neg: bool },
    Bool(bool),
}

/// A spanned value.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Sp<T> {
    pub v: T,
    pub span: Span,
}

/// `param <name> <kind> = <default> [in <min>..<max>] [unit "<u>"] ["<description>"]`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ParamStmt {
    pub name: Sp<String>,
    pub kind: Sp<String>,
    pub default: Sp<ParamLit>,
    pub range: Option<(Sp<ParamLit>, Sp<ParamLit>)>,
    pub unit: Option<String>,
    pub description: Option<String>,
}

/// The clauses of a `signal` line (any order, each at most once).
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct SignalClauses {
    pub side: Option<Sp<String>>,
    pub target: Option<Expr>,
    pub stop: Option<Expr>,
    pub max_bars: Option<Expr>,
    pub source: Option<Sp<(String, String)>>,
}

/// The kind of a one-expression output line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OutKind {
    Series,
    Event,
}

/// One statement (PR §12.1).
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Stmt {
    Param(ParamStmt),
    Bind {
        name: Sp<String>,
        expr: Expr,
    },
    Output {
        kind: OutKind,
        name: Sp<String>,
        expr: Expr,
    },
    Signal {
        name: Sp<String>,
        expr: Expr,
        clauses: SignalClauses,
    },
    State {
        name: Sp<String>,
        dims: Vec<(Sp<String>, Expr)>,
    },
    Reject {
        code: Sp<String>,
        reason: String,
    },
}

/// A statement and the span of its first line.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Located {
    pub stmt: Stmt,
    pub span: Span,
}

/// A span from the start of `a` to the end of `b` (on `a`'s line).
pub(crate) fn join(a: Span, b: Span) -> Span {
    if a.line == b.line && b.col + b.len >= a.col {
        Span {
            line: a.line,
            col: a.col,
            len: b.col + b.len - a.col,
        }
    } else {
        a
    }
}
