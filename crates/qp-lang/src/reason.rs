//! Sub-codes of program errors and warnings (PR §11.2, §11.3) and their closed protocol
//! code (`PROTOCOL.md` §13). The table is the single mapping; nothing else picks a code.

use qp_schema::protocol::{ErrorCode, ProgramError};

/// A program error sub-code (`E_...`) or warning reason (`W_...`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Reason {
    /// Text syntax broken (text language only).
    Parse,
    /// `version` is not `qp-ir/0`.
    Version,
    /// Unknown node operation or function.
    UnknownOp,
    /// Unknown field or bar field.
    UnknownField,
    /// Type mismatch.
    Type,
    /// Missing field or argument.
    MissingArg,
    /// Unknown, below-base or disallowed timeframe.
    UnknownTf,
    /// Inputs on different clocks combined without `align`.
    ClockMismatch,
    /// An output's node is not on the base clock.
    OutputClock,
    /// No output.
    NoOutput,
    /// Unknown or later node, or a missing output of a multi-output node.
    UnknownRef,
    /// Unknown parameter key.
    UnknownParam,
    /// Same node id, output name or parameter key twice.
    DuplicateName,
    /// Invalid constant.
    BadParam,
    /// A parameter definition exceeds its field's range, or a bound value its definition's.
    ParamRange,
    /// A signal without `rule` or `max_bars`.
    NoOutcomeRule,
    /// Invalid rule values.
    BadRule,
    /// `inputs` is not exactly one bar series (session).
    NotBarSeries,
    /// A direct write to a program series (session).
    ProgramSeries,
    /// A signal or state output in `add_program`.
    TryOnly,
    /// An event series as an operand outside `threshold` (session).
    EventOperand,
    /// A window above `program.max_window`.
    WindowLimit,
    /// More nodes than `program.max_nodes`.
    TooManyNodes,
    /// Bytes, outputs or parameters above their limit.
    TooLarge,
    /// Static cost per bar above `program.max_cost_per_bar`.
    CostLimit,
    /// Per-owner live budget exceeded (session).
    Budget,
    /// Negative lag, unclosed bar, future value.
    Lookahead,
    /// Deliberately absent construct.
    Unsupported,
    /// Warning: an event output may fire on consecutive bars.
    ConsecutiveEvents,
    /// Warning: a node reaches no output.
    UnusedNode,
}

impl Reason {
    /// Every reason, errors in PR §11.2 order, then warnings.
    pub const ALL: [Reason; 30] = [
        Reason::Parse,
        Reason::Version,
        Reason::UnknownOp,
        Reason::UnknownField,
        Reason::Type,
        Reason::MissingArg,
        Reason::UnknownTf,
        Reason::ClockMismatch,
        Reason::OutputClock,
        Reason::NoOutput,
        Reason::UnknownRef,
        Reason::UnknownParam,
        Reason::DuplicateName,
        Reason::BadParam,
        Reason::ParamRange,
        Reason::NoOutcomeRule,
        Reason::BadRule,
        Reason::NotBarSeries,
        Reason::ProgramSeries,
        Reason::TryOnly,
        Reason::EventOperand,
        Reason::WindowLimit,
        Reason::TooManyNodes,
        Reason::TooLarge,
        Reason::CostLimit,
        Reason::Budget,
        Reason::Lookahead,
        Reason::Unsupported,
        Reason::ConsecutiveEvents,
        Reason::UnusedNode,
    ];

    /// The sub-code as written (`E_UNKNOWN_OP`, `W_UNUSED_NODE` ...).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Reason::Parse => "E_PARSE",
            Reason::Version => "E_VERSION",
            Reason::UnknownOp => "E_UNKNOWN_OP",
            Reason::UnknownField => "E_UNKNOWN_FIELD",
            Reason::Type => "E_TYPE",
            Reason::MissingArg => "E_MISSING_ARG",
            Reason::UnknownTf => "E_UNKNOWN_TF",
            Reason::ClockMismatch => "E_CLOCK_MISMATCH",
            Reason::OutputClock => "E_OUTPUT_CLOCK",
            Reason::NoOutput => "E_NO_OUTPUT",
            Reason::UnknownRef => "E_UNKNOWN_REF",
            Reason::UnknownParam => "E_UNKNOWN_PARAM",
            Reason::DuplicateName => "E_DUPLICATE_NAME",
            Reason::BadParam => "E_BAD_PARAM",
            Reason::ParamRange => "E_PARAM_RANGE",
            Reason::NoOutcomeRule => "E_NO_OUTCOME_RULE",
            Reason::BadRule => "E_BAD_RULE",
            Reason::NotBarSeries => "E_NOT_BAR_SERIES",
            Reason::ProgramSeries => "E_PROGRAM_SERIES",
            Reason::TryOnly => "E_TRY_ONLY",
            Reason::EventOperand => "E_EVENT_OPERAND",
            Reason::WindowLimit => "E_WINDOW_LIMIT",
            Reason::TooManyNodes => "E_TOO_MANY_NODES",
            Reason::TooLarge => "E_TOO_LARGE",
            Reason::CostLimit => "E_COST_LIMIT",
            Reason::Budget => "E_BUDGET",
            Reason::Lookahead => "E_LOOKAHEAD",
            Reason::Unsupported => "E_UNSUPPORTED",
            Reason::ConsecutiveEvents => "W_CONSECUTIVE_EVENTS",
            Reason::UnusedNode => "W_UNUSED_NODE",
        }
    }

    /// The closed protocol code (PR §11.2); `None` for a warning.
    #[must_use]
    pub const fn code(self) -> Option<ErrorCode> {
        Some(match self {
            Reason::Parse
            | Reason::Version
            | Reason::UnknownOp
            | Reason::UnknownField
            | Reason::Type
            | Reason::MissingArg
            | Reason::UnknownTf
            | Reason::ClockMismatch
            | Reason::OutputClock
            | Reason::NoOutput => ErrorCode::Schema,
            Reason::UnknownRef | Reason::UnknownParam => ErrorCode::NotFound,
            Reason::DuplicateName => ErrorCode::Duplicate,
            Reason::BadParam
            | Reason::ParamRange
            | Reason::NoOutcomeRule
            | Reason::BadRule
            | Reason::NotBarSeries
            | Reason::ProgramSeries => ErrorCode::Invalid,
            Reason::TryOnly | Reason::EventOperand | Reason::Lookahead | Reason::Unsupported => {
                ErrorCode::Unsupported
            }
            Reason::WindowLimit
            | Reason::TooManyNodes
            | Reason::TooLarge
            | Reason::CostLimit
            | Reason::Budget => ErrorCode::Limit,
            Reason::ConsecutiveEvents | Reason::UnusedNode => return None,
        })
    }

    /// Whether this is a warning (`W_`).
    #[must_use]
    pub const fn is_warning(self) -> bool {
        self.code().is_none()
    }

    /// The reason written as `s`, if any.
    #[must_use]
    pub fn parse(s: &str) -> Option<Reason> {
        Reason::ALL.into_iter().find(|r| r.as_str() == s)
    }

    /// A bare program error of this reason (for callers that report session-level
    /// sub-codes such as `E_NOT_BAR_SERIES`, `E_PROGRAM_SERIES`, `E_EVENT_OPERAND`).
    #[must_use]
    pub fn error(self, at: Option<String>, message: String) -> ProgramError {
        ProgramError {
            code: self.code(),
            reason: self.as_str().to_owned(),
            message,
            at,
            span: None,
            expected: Vec::new(),
            did_you_mean: Vec::new(),
            hint: None,
        }
    }
}
