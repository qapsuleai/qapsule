//! The checker (PR §2–§5, §8.1, §11): reads a program as raw JSON, reports every error at
//! once with its JSON pointer, sub-code, expected forms and close names, warns, computes
//! the static cost, and — when there is no error — returns the resolved program for the
//! executor ([`Checked`]).

use std::collections::BTreeMap;

use qp_schema::program::{
    ProgramCheck, ProgramCost, ProgramOutputInfo, ProgramParamDef, ProgramParamKind,
    ProgramParamValue, ProgramReject,
};
use qp_schema::protocol::ProgramError;
use serde_json::Value;

use crate::checked::{Checked, Clock, Op, Tf, Ty};
use crate::diag::Diags;
use crate::limits::{Limit, Limits};
use crate::message::Lang;

mod args;
mod clock;
mod cost;
mod fields;
mod nodes;
mod outputs;
mod params;
mod top;
mod types;
mod warn;

/// The bar series a check is bound to (`inputs`; PS §3.2). Without one, clocks and the
/// budget are checked independently of a binding.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Binding {
    /// The series' interval (`1m` … `1M`), if it has one: the base clock.
    pub interval: Option<String>,
    /// Its bar count, for `cost.bars` and `cost.total`.
    pub bars: Option<u64>,
}

/// What the program is checked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Purpose {
    /// `check_program`, `try_program`, `explain_program`: every output kind.
    #[default]
    Check,
    /// `add_program`: signal and state outputs are refused (`E_TRY_ONLY`, PR §5).
    Document,
}

/// Everything a check reads besides the program.
#[derive(Debug, Clone, Default)]
pub struct CheckCtx<'a> {
    /// Limits in force (from the parameter registry).
    pub limits: Limits,
    /// The bound bar series, if any.
    pub binding: Option<Binding>,
    /// Values to bind (`params` of the message: key → JSON value), if any.
    pub params: Option<&'a Value>,
    /// Message language.
    pub lang: Lang,
    /// What it is checked for.
    pub purpose: Purpose,
}

/// The result of [`check`]: what `check_program` answers, plus the resolved program.
#[derive(Debug, Clone, PartialEq)]
pub struct CheckReport {
    /// Every error, in position order, at most `program.check.max_errors`.
    pub errors: Vec<ProgramError>,
    /// Errors beyond the limit were cut.
    pub truncated: bool,
    /// Warnings (`W_`).
    pub warnings: Vec<ProgramError>,
    /// Static cost, when every node typed.
    pub cost: Option<ProgramCost>,
    /// The outputs that parsed.
    pub outputs: Vec<ProgramOutputInfo>,
    /// The parameter definitions that parsed (normalized).
    pub params: Vec<ProgramParamDef>,
    /// Program identity, when the program parsed.
    pub program_id: Option<String>,
    /// Instance identity, when there is no error.
    pub instance_id: Option<String>,
    /// The refusal, when the input is one (`ok` with no program).
    pub reject: Option<ProgramReject>,
    /// The resolved program, when there is no error and the input is a program.
    pub checked: Option<Checked>,
    /// The limits used.
    pub limits: Limits,
}

impl CheckReport {
    /// No errors (a valid refusal is ok too).
    #[must_use]
    pub fn ok(&self) -> bool {
        self.errors.is_empty()
    }

    /// The `check_program` answer (`ir` is the caller's: it is set when text was lowered).
    #[must_use]
    pub fn to_answer(&self) -> ProgramCheck {
        ProgramCheck {
            ok: self.ok(),
            errors: self.errors.clone(),
            truncated: self.truncated,
            warnings: self.warnings.clone(),
            cost: self.cost.clone(),
            outputs: self.outputs.clone(),
            params: self.params.clone(),
            program_id: self.program_id.clone(),
            limits: self.limits.to_map(),
            ir: None,
            reject: self.reject.clone(),
        }
    }
}

/// Checks `raw` (a program or a refusal, as JSON) in `ctx`. Never stops at the first
/// error; never panics on any JSON.
#[must_use]
pub fn check(raw: &Value, ctx: &CheckCtx<'_>) -> CheckReport {
    top::run(raw, ctx)
}

/// Checks a typed program (e.g. one already in a document, re-checked with new values).
#[must_use]
pub fn check_program(program: &qp_schema::program::Program, ctx: &CheckCtx<'_>) -> CheckReport {
    match serde_json::to_value(program) {
        Ok(v) => check(&v, ctx),
        Err(_) => check(&Value::Null, ctx),
    }
}

/// The base clock as the check sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Base {
    /// No bound series: higher timeframes are only spelled-checked.
    Unbound,
    /// A bound series; `None` when it has no interval.
    Bound(Option<Tf>),
}

/// A parameter as nodes see it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ParamInfo {
    /// Its kind; `None` when the definition is broken (references then stay silent).
    pub(crate) kind: Option<ProgramParamKind>,
    /// Inclusive range, as numbers.
    pub(crate) range: Option<(f64, f64)>,
    /// The bound value (or default), normalized; `None` when invalid.
    pub(crate) value: Option<ProgramParamValue>,
}

/// A node as later nodes see it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct NodeEntry {
    /// Its `op` name, when known (gives the output names of multi-output nodes).
    pub(crate) op: Option<&'static str>,
    /// Typed, when the node has no error.
    pub(crate) typed: Option<Typed>,
}

/// A node that checked.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Typed {
    pub(crate) op: Op,
    pub(crate) outputs: Vec<Ty>,
    pub(crate) clock: Clock,
    pub(crate) w: u64,
}

/// State of one check.
pub(crate) struct Cx<'a> {
    pub(crate) d: Diags,
    pub(crate) limits: Limits,
    pub(crate) base: Base,
    pub(crate) params: BTreeMap<String, ParamInfo>,
    /// First index of each node id in the whole program (for later-node references).
    pub(crate) ids: BTreeMap<String, usize>,
    pub(crate) nodes: Vec<NodeEntry>,
    pub(crate) ctx: &'a CheckCtx<'a>,
}

impl Cx<'_> {
    /// A limit's value.
    pub(crate) fn limit(&self, l: Limit) -> u64 {
        self.limits.get(l)
    }
}

/// Which nodes of `c` the `roots` depend on (themselves included): the executor runs only
/// the cones of the outputs it is asked for (PS §1.3).
pub(crate) fn cone_of(c: &Checked, roots: &[crate::checked::Slot]) -> Vec<bool> {
    let inputs: Vec<Vec<crate::checked::Slot>> = c.nodes.iter().map(|n| n.op.inputs()).collect();
    cost::cone(&inputs, roots)
}
